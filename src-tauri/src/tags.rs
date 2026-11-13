// Tag writing that cannot corrupt a file: copy → write the copy → re-read and verify → atomic rename.
use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::{items::Timestamp, Tag};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Per-field edits. `None` leaves a field alone (a "Mixed" value in a multi-edit),
/// `Some("")` clears it, anything else sets it.
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Edit {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub composer: Option<String>,
    pub year: Option<String>,
    pub track: Option<String>,
    pub disc: Option<String>,
    /// Path of an image to embed as the front cover; "" removes it.
    pub cover: Option<String>,
}

fn text(tag: &mut Tag, v: &Option<String>, set: fn(&mut Tag, String), remove: fn(&mut Tag)) {
    match v.as_deref().map(str::trim) {
        None => {}
        Some("") => remove(tag),
        Some(s) => set(tag, s.to_string()),
    }
}

fn number(v: &Option<String>) -> Result<Option<Option<u32>>, String> {
    match v.as_deref().map(str::trim) {
        None => Ok(None),
        Some("") => Ok(Some(None)),
        Some(s) => s.parse::<u32>().map(|n| Some(Some(n))).map_err(|_| format!("“{s}” is not a number")),
    }
}

fn apply(tag: &mut Tag, e: &Edit) -> Result<(), String> {
    text(tag, &e.title, |t, v| t.set_title(v), |t| t.remove_title());
    text(tag, &e.artist, |t, v| t.set_artist(v), |t| t.remove_artist());
    text(tag, &e.album, |t, v| t.set_album(v), |t| t.remove_album());
    text(tag, &e.genre, |t, v| t.set_genre(v), |t| t.remove_genre());
    text(tag, &e.album_artist, |t, v| { t.insert_text(ItemKey::AlbumArtist, v); }, |t| t.remove_key(ItemKey::AlbumArtist));
    text(tag, &e.composer, |t, v| { t.insert_text(ItemKey::Composer, v); }, |t| t.remove_key(ItemKey::Composer));
    match number(&e.year)? {
        Some(Some(y)) => tag.set_date(Timestamp { year: y as u16, month: None, day: None, hour: None, minute: None, second: None }),
        Some(None) => tag.remove_date(),
        None => {}
    }
    match number(&e.track)? {
        Some(Some(n)) => tag.set_track(n),
        Some(None) => tag.remove_track(),
        None => {}
    }
    match number(&e.disc)? {
        Some(Some(n)) => tag.set_disk(n),
        Some(None) => tag.remove_disk(),
        None => {}
    }
    if let Some(cover) = &e.cover {
        tag.remove_picture_type(PictureType::CoverFront);
        if !cover.is_empty() {
            let bytes = std::fs::read(cover).map_err(|e| format!("cannot read image: {e}"))?;
            let mime = match image::guess_format(&bytes) {
                Ok(image::ImageFormat::Png) => MimeType::Png,
                Ok(image::ImageFormat::Jpeg) => MimeType::Jpeg,
                _ => return Err("artwork must be a JPEG or PNG image".into()),
            };
            tag.push_picture(Picture::unchecked(bytes).pic_type(PictureType::CoverFront).mime_type(mime).build());
        }
    }
    Ok(())
}

fn expect_eq(field: &str, want: &Option<String>, got: Option<String>) -> Result<(), String> {
    match want.as_deref().map(str::trim) {
        Some(w) if got.as_deref().unwrap_or("") != w => Err(format!("{field} did not save (got {got:?})")),
        _ => Ok(()),
    }
}

/// Writes `edit` into `path`. The original stays untouched unless the edited copy verifies.
pub fn write(path: &Path, edit: &Edit) -> Result<(), String> {
    // same folder (so the rename is atomic) and same extension (lofty picks the format by it)
    let stem = path.file_stem().ok_or("not a file")?.to_string_lossy();
    let ext = path.extension().map(|e| e.to_string_lossy()).unwrap_or_default();
    let tmp: PathBuf = path.with_file_name(format!(".{stem}.confluence-tmp.{ext}"));
    let result = (|| {
        let before = lofty::read_from_path(path).map_err(|e| format!("cannot read tags: {e}"))?;
        std::fs::copy(path, &tmp).map_err(|e| format!("cannot copy file: {e}"))?;

        let mut tagged = lofty::read_from_path(&tmp).map_err(|e| e.to_string())?;
        if tagged.primary_tag().is_none() {
            tagged.insert_tag(Tag::new(tagged.primary_tag_type()));
        }
        let tag = tagged.primary_tag_mut().ok_or("this format has no writable tag")?;
        apply(tag, edit)?;
        tag.save_to_path(&tmp, WriteOptions::default()).map_err(|e| format!("cannot write tags: {e}"))?;

        // verify: tags read back as written and the audio itself is unchanged
        let after = lofty::read_from_path(&tmp).map_err(|e| format!("edited copy is unreadable: {e}"))?;
        if after.properties().duration() != before.properties().duration()
            || after.properties().sample_rate() != before.properties().sample_rate()
        {
            return Err("audio changed while writing tags; nothing was saved".into());
        }
        let t = after.primary_tag().ok_or("tag missing after write")?;
        expect_eq("Title", &edit.title, t.title().map(|s| s.to_string()))?;
        expect_eq("Artist", &edit.artist, t.artist().map(|s| s.to_string()))?;
        expect_eq("Album", &edit.album, t.album().map(|s| s.to_string()))?;
        expect_eq("Album Artist", &edit.album_artist, t.get_string(ItemKey::AlbumArtist).map(str::to_string))?;

        std::fs::rename(&tmp, path).map_err(|e| format!("cannot replace file (is it open elsewhere?): {e}"))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(name: &str, dir: &Path) -> PathBuf {
        let to = dir.join(name);
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name), &to).unwrap();
        to
    }

    #[test]
    fn round_trips_flac_mp3_and_m4a() {
        let d = tempfile::tempdir().unwrap();
        for f in ["a.flac", "b.mp3", "c.m4a"] {
            let p = copy(f, d.path());
            let before = std::fs::metadata(&p).unwrap().len();
            let e = Edit {
                title: Some("New Title".into()),
                album_artist: Some("Someone".into()),
                genre: Some("".into()),
                track: Some("7".into()),
                year: Some("1999".into()),
                ..Default::default()
            };
            write(&p, &e).unwrap_or_else(|err| panic!("{f}: {err}"));
            let t = lofty::read_from_path(&p).unwrap();
            let tag = t.primary_tag().unwrap();
            assert_eq!(tag.title().as_deref(), Some("New Title"), "{f}");
            assert_eq!(tag.get_string(ItemKey::AlbumArtist), Some("Someone"), "{f}");
            assert_eq!(tag.genre(), None, "{f}");
            assert_eq!((tag.track(), tag.date().map(|d| d.year)), (Some(7), Some(1999)), "{f}");
            assert!(std::fs::metadata(&p).unwrap().len() > before / 2, "{f} kept its audio");
            assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1, "{f}: no temp file left");
            std::fs::remove_file(&p).unwrap();
        }
    }

    #[test]
    fn embeds_and_removes_cover() {
        let d = tempfile::tempdir().unwrap();
        let p = copy("b.mp3", d.path());
        let png = d.path().join("c.png");
        image::RgbImage::new(8, 8).save(&png).unwrap();
        write(&p, &Edit { cover: Some(png.to_string_lossy().into()), ..Default::default() }).unwrap();
        assert_eq!(lofty::read_from_path(&p).unwrap().primary_tag().unwrap().pictures().len(), 1);
        write(&p, &Edit { cover: Some("".into()), ..Default::default() }).unwrap();
        assert!(lofty::read_from_path(&p).unwrap().primary_tag().unwrap().pictures().is_empty());
    }

    #[test]
    fn a_failed_write_leaves_the_original_untouched() {
        let d = tempfile::tempdir().unwrap();
        let bad = d.path().join("broken.flac");
        std::fs::write(&bad, b"not really a flac file").unwrap();
        assert!(write(&bad, &Edit { title: Some("x".into()), ..Default::default() }).is_err());
        assert_eq!(std::fs::read(&bad).unwrap(), b"not really a flac file");

        let p = copy("a.flac", d.path());
        let original = std::fs::read(&p).unwrap();
        assert!(write(&p, &Edit { track: Some("seven".into()), ..Default::default() }).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), original, "invalid input must not touch the file");
        assert!(!d.path().join(".a.confluence-tmp.flac").exists());
    }
}

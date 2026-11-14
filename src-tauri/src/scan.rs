use crate::db::{self, Track};
use lofty::file::FileType;
use lofty::picture::PictureType;
use lofty::prelude::*;
use rayon::prelude::*;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub const AUDIO: &[&str] = &[
    "flac", "m4a", "alac", "mp3", "aac", "wav", "aif", "aiff", "aifc", "ape", "wv", "opus", "ogg", "oga", "mpc",
    "tta", "tak", "dsf", "dff", "wma", "mka", "ac3", "dts", "caf", "w64", "rf64", "mp2", "spx",
];
pub const VIDEO: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "ts", "m2ts", "mts", "mpg", "mpeg", "vob", "ogv",
    "3gp", "asf", "rmvb", "divx",
];
const LOSSLESS_EXT: &[&str] = &["flac", "wav", "aif", "aiff", "aifc", "ape", "wv", "tta", "tak", "dsf", "dff", "w64", "rf64"];
const FOLDER_ART: &[&str] = &[
    "cover.jpg", "cover.png", "folder.jpg", "folder.png", "front.jpg", "front.png", "album.jpg", "album.png",
];

fn ext(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

pub fn kind_of(p: &Path) -> Option<&'static str> {
    let e = ext(p);
    if AUDIO.contains(&e.as_str()) {
        Some("audio")
    } else if VIDEO.contains(&e.as_str()) {
        Some("video")
    } else {
        None
    }
}

/// The tag editor's in-flight copies (tags.rs); ".sonora-tmp." is what the app wrote under its old name.
fn is_tag_tmp(name: &str) -> bool {
    name.contains(".qistara-tmp.") || name.contains(".confluence-tmp.") || name.contains(".sonora-tmp.")
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Artwork cache: 400px JPEG thumbnails named by content hash, so one album's art is stored once.
pub struct Art {
    pub dir: PathBuf,
    folders: Mutex<HashMap<PathBuf, Option<String>>>,
}

impl Art {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Art { dir, folders: Mutex::new(HashMap::new()) }
    }

    pub fn thumb(&self, bytes: &[u8]) -> Option<String> {
        // shortcut: std hasher is stable within a Rust release; a change only re-creates thumbnails.
        let mut h = std::hash::DefaultHasher::new();
        bytes.hash(&mut h);
        let name = format!("{:016x}.jpg", h.finish());
        let out = self.dir.join(&name);
        if !out.exists() {
            let img = image::load_from_memory(bytes).ok()?.thumbnail(400, 400);
            let tmp = self.dir.join(format!("{name}.{:?}.tmp", std::thread::current().id()));
            let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp).ok()?);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 85)
                .encode_image(&image::DynamicImage::ImageRgb8(img.to_rgb8()))
                .ok()?;
            drop(f);
            if std::fs::rename(&tmp, &out).is_err() {
                let _ = std::fs::remove_file(&tmp); // another thread stored the same image first
            }
        }
        out.exists().then_some(name)
    }

    fn folder(&self, dir: &Path) -> Option<String> {
        if let Some(v) = self.folders.lock().unwrap().get(dir) {
            return v.clone();
        }
        let v = FOLDER_ART
            .iter()
            .map(|n| dir.join(n))
            .find(|p| p.is_file())
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| self.thumb(&b));
        self.folders.lock().unwrap().insert(dir.to_path_buf(), v.clone());
        v
    }
}

fn front(tag: &lofty::tag::Tag) -> Option<&lofty::picture::Picture> {
    let pics = tag.pictures();
    pics.iter().find(|p| p.pic_type() == PictureType::CoverFront).or(pics.first())
}

impl Art {
    /// Full-size artwork for Now Playing, copied into the cache (the webview may only read the cache).
    pub fn large(&self, path: &Path) -> Option<PathBuf> {
        let bytes = lofty::read_from_path(path)
            .ok()
            .and_then(|f| f.primary_tag().or_else(|| f.first_tag()).and_then(front).map(|p| p.data().to_vec()))
            .or_else(|| {
                let d = path.parent()?;
                FOLDER_ART.iter().map(|n| d.join(n)).find(|p| p.is_file()).and_then(|p| std::fs::read(p).ok())
            })?;
        let ext = image::guess_format(&bytes).ok().and_then(|f| f.extensions_str().first().copied()).unwrap_or("jpg");
        let mut h = std::hash::DefaultHasher::new();
        bytes.hash(&mut h);
        let out = self.dir.join("large").join(format!("{:016x}.{ext}", h.finish()));
        if !out.exists() {
            std::fs::create_dir_all(out.parent()?).ok()?;
            std::fs::write(&out, &bytes).ok()?;
        }
        Some(out)
    }
}

fn text(v: Option<std::borrow::Cow<'_, str>>) -> String {
    v.map(|s| s.trim().to_string()).unwrap_or_default()
}

/// Reads tags and stream properties. Files lofty can't parse (DSD, TAK, WMA, video) keep the
/// file name as title; mpv still plays them.
pub fn read(path: &Path, mtime: i64, size: i64, art: &Art) -> Track {
    let e = ext(path);
    let mut t = Track {
        path: path.to_string_lossy().into_owned(),
        mtime,
        size,
        kind: kind_of(path).unwrap_or("audio").into(),
        title: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        codec: e.to_uppercase(),
        lossless: LOSSLESS_EXT.contains(&e.as_str()),
        added_at: now(),
        ..Default::default()
    };
    if t.kind == "audio" {
        if let Ok(f) = lofty::read_from_path(path) {
            let p = f.properties();
            t.duration = p.duration().as_secs_f64();
            t.sample_rate = p.sample_rate();
            t.bit_depth = p.bit_depth();
            t.channels = p.channels();
            t.bitrate = p.audio_bitrate();
            // shortcut: ALAC vs AAC in MP4 is told apart by bit depth (AAC reports none); read the codec box if it misfires.
            let codec = match f.file_type() {
                FileType::Flac => Some(("FLAC", true)),
                FileType::Wav => Some(("WAV", true)),
                FileType::Aiff => Some(("AIFF", true)),
                FileType::Ape => Some(("APE", true)),
                FileType::WavPack => Some(("WavPack", true)),
                FileType::Mp4 if t.bit_depth.is_some() => Some(("ALAC", true)),
                FileType::Mp4 | FileType::Aac => Some(("AAC", false)),
                FileType::Mpeg => Some(("MP3", false)),
                FileType::Opus => Some(("Opus", false)),
                FileType::Vorbis => Some(("Vorbis", false)),
                FileType::Speex => Some(("Speex", false)),
                FileType::Mpc => Some(("Musepack", false)),
                _ => None,
            };
            if let Some((c, l)) = codec {
                t.codec = c.into();
                t.lossless = l;
            }
            if let Some(tag) = f.primary_tag().or_else(|| f.first_tag()) {
                let title = text(tag.title());
                if !title.is_empty() {
                    t.title = title;
                }
                t.artist = text(tag.artist());
                t.album = text(tag.album());
                t.genre = text(tag.genre());
                t.album_artist = tag.get_string(ItemKey::AlbumArtist).unwrap_or("").trim().into();
                t.composer = tag.get_string(ItemKey::Composer).unwrap_or("").trim().into();
                t.year = tag.date().map(|d| d.year as i32);
                t.track = tag.track();
                t.disc = tag.disk();
                t.art = front(tag).and_then(|p| art.thumb(p.data()));
            }
        }
    }
    if t.art.is_none() {
        t.art = path.parent().and_then(|d| art.folder(d));
    }
    t
}

/// True when `path` lies inside library folder `folder` (Windows paths compare case-insensitively).
fn under(path: &str, folder: &str) -> bool {
    let f = folder.trim_end_matches(['\\', '/']).as_bytes();
    let p = path.as_bytes();
    p.len() > f.len() && p[..f.len()].eq_ignore_ascii_case(f) && matches!(p[f.len()], b'\\' | b'/')
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Stats {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

/// Incremental scan: only new or changed files (mtime/size) are parsed, in parallel.
/// The DB is locked only for the reads and the final write, so the UI stays responsive.
pub fn scan(db: &Mutex<Connection>, art: &Art, progress: &(dyn Fn(usize, usize) + Sync)) -> rusqlite::Result<Stats> {
    let (folders, known) = {
        let c = db.lock().unwrap();
        let known: HashMap<String, (i64, i64)> = c
            .prepare("SELECT path, mtime, size FROM tracks")?
            .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
            .collect::<Result<_, _>>()?;
        (db::folders(&c)?, known)
    };

    let mut seen = HashSet::new();
    let mut todo = Vec::new();
    // folders where any part of the walk failed (offline NAS behind a junction, flaky share):
    // nothing under them is treated as deleted this time
    let mut failed: HashSet<&String> = HashSet::new();
    for f in &folders {
        for entry in WalkDir::new(f).follow_links(true) {
            let Ok(e) = entry else {
                failed.insert(f);
                continue;
            };
            // skip non-media files and the tag editor's in-flight copies
            if !e.file_type().is_file() || kind_of(e.path()).is_none() || is_tag_tmp(&e.file_name().to_string_lossy()) {
                continue;
            }
            let p = e.path().to_string_lossy().into_owned();
            let Ok(m) = e.metadata() else {
                seen.insert(p); // unreadable right now: keep its row as it is
                continue;
            };
            let mtime = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
            let size = m.len() as i64;
            if known.get(&p) != Some(&(mtime, size)) {
                todo.push((e.into_path(), mtime, size));
            }
            seen.insert(p);
        }
    }

    let done = AtomicUsize::new(0);
    let total = todo.len();
    let parsed: Vec<Track> = todo
        .par_iter()
        .map(|(p, m, s)| {
            let t = read(p, *m, *s, art);
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 25 == 0 || n == total {
                progress(n, total);
            }
            t
        })
        .collect();

    // A missing file only counts as removed when its library folder is reachable:
    // an unplugged drive must not wipe ratings and play counts.
    let gone: Vec<&String> = known
        .keys()
        .filter(|p| !seen.contains(*p))
        .filter(|p| match folders.iter().find(|f| under(p, f)) {
            Some(f) => !failed.contains(f) && Path::new(f).is_dir(),
            None => true, // its folder was removed from the library
        })
        .collect();

    let mut c = db.lock().unwrap();
    let tx = c.transaction()?;
    for t in &parsed {
        db::upsert(&tx, t)?;
    }
    for p in &gone {
        tx.execute("DELETE FROM tracks WHERE path = ?1", [p])?;
    }
    tx.commit()?;

    let added = parsed.iter().filter(|t| !known.contains_key(&t.path)).count();
    Ok(Stats { added, updated: parsed.len() - added, removed: gone.len() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_lib() -> (tempfile::TempDir, PathBuf, Mutex<Connection>, Art) {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("Music");
        fs::create_dir(&lib).unwrap();
        for f in ["a.flac", "b.mp3", "c.m4a"] {
            fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(f), lib.join(f)).unwrap();
        }
        let c = db::open(&dir.path().join("lib.db")).unwrap();
        c.execute("INSERT INTO folders VALUES (?1)", [lib.to_str().unwrap()]).unwrap();
        let art = Art::new(dir.path().join("art"));
        (dir, lib, Mutex::new(c), art)
    }

    fn by_title<'a>(ts: &'a [Track], title: &str) -> &'a Track {
        ts.iter().find(|t| t.title == title).unwrap_or_else(|| panic!("missing {title}"))
    }

    #[test]
    fn reads_tags_codecs_and_art() {
        let (_d, _lib, c, art) = fixture_lib();
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap(), Stats { added: 3, updated: 0, removed: 0 });
        let ts = db::tracks(&c.lock().unwrap()).unwrap();

        let a = by_title(&ts, "First Light");
        assert_eq!((a.codec.as_str(), a.lossless, a.sample_rate, a.bit_depth), ("FLAC", true, Some(44100), Some(16)));
        assert_eq!((a.artist.as_str(), a.album_artist.as_str(), a.album.as_str()), ("Test Artist", "Test Artist", "Fixtures"));
        assert_eq!((a.genre.as_str(), a.year, a.track, a.disc), ("Ambient", Some(2024), Some(1), Some(1)));
        assert!(art.dir.join(a.art.as_ref().expect("embedded cover")).is_file());

        let b = by_title(&ts, "Lossy One");
        assert_eq!((b.codec.as_str(), b.lossless, b.art.clone()), ("MP3", false, None));

        let c3 = by_title(&ts, "Apple Lossless");
        assert_eq!((c3.codec.as_str(), c3.lossless, c3.sample_rate, c3.bit_depth), ("ALAC", true, Some(96000), Some(24)));
    }

    #[test]
    fn rescans_incrementally_and_never_drops_tracks_of_unreachable_folders() {
        let (dir, lib, c, art) = fixture_lib();
        scan(&c, &art, &|_, _| {}).unwrap();
        c.lock().unwrap().execute("UPDATE tracks SET rating = 5", []).unwrap();
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap(), Stats { added: 0, updated: 0, removed: 0 });

        let away = dir.path().join("unplugged");
        fs::rename(&lib, &away).unwrap();
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap().removed, 0);
        fs::rename(&away, &lib).unwrap();

        fs::remove_file(lib.join("b.mp3")).unwrap();
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap().removed, 1);
        let ts = db::tracks(&c.lock().unwrap()).unwrap();
        assert_eq!(ts.len(), 2);
        assert!(ts.iter().all(|t| t.rating == 5), "user data survives rescans");

        c.lock().unwrap().execute("DELETE FROM folders", []).unwrap();
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap().removed, 2);
    }

    /// An offline NAS behind a junction inside a reachable library folder must keep its tracks.
    #[cfg(windows)]
    #[test]
    fn a_broken_junction_inside_the_library_deletes_nothing() {
        let (dir, lib, c, art) = fixture_lib();
        let nas = dir.path().join("nas");
        fs::create_dir(&nas).unwrap();
        fs::copy(lib.join("a.flac"), nas.join("remote.flac")).unwrap();
        let link = lib.join("NAS");
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&link).arg(&nas).output().unwrap().status.success();
        assert!(ok, "could not create a junction");
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap().added, 4);

        fs::remove_dir_all(&nas).unwrap(); // the NAS goes offline; the junction now points nowhere
        assert_eq!(scan(&c, &art, &|_, _| {}).unwrap().removed, 0);
        assert_eq!(db::tracks(&c.lock().unwrap()).unwrap().len(), 4);
        let _ = fs::remove_dir(&link);
    }

    #[test]
    fn folder_matching_is_case_insensitive_and_boundary_safe() {
        assert!(under(r"D:\Music\a.flac", r"d:\music"));
        assert!(under(r"D:\Music\a.flac", r"D:\Music\"));
        assert!(!under(r"D:\Musicals\a.flac", r"D:\Music"));
        assert!(under(r"D:\a.flac", r"D:\"));
    }
}

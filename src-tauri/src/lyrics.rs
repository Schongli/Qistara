// Lyrics: the song's own (sidecar file, embedded tag) and the cache of online results, one per source.
use lofty::prelude::*;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Found {
    pub kind: &'static str,
    pub text: String,
    pub source: &'static str,
}

/// LRCLIB results keep the file name they always had (the track's path), so existing caches stay valid.
/// `key` is the track's path, or "sub:<id>" for a server song (its stream URL changes every session).
fn cache_file(key: &str, source: &str, cache: &Path) -> PathBuf {
    let mut h = std::hash::DefaultHasher::new();
    key.to_lowercase().hash(&mut h);
    if source != "lrclib" {
        source.hash(&mut h);
    }
    cache.join(format!("{:016x}.lrc", h.finish()))
}

/// Sidecar lyrics files are small; anything bigger isn't one (and TTML parsing would crawl).
const MAX_SIDECAR: u64 = 2 * 1024 * 1024;

/// Text from a file of unknown encoding: a BOM, else UTF-8, else Shift-JIS when it holds kana, else GBK when
/// its high bytes pair up like Chinese text, else Windows-1252 (what Winamp and old taggers wrote).
pub fn decode_text(b: &[u8]) -> String {
    if let Some((enc, bom)) = encoding_rs::Encoding::for_bom(b) {
        return enc.decode_without_bom_handling(&b[bom..]).0.into_owned();
    }
    if let Ok(s) = std::str::from_utf8(b) {
        return s.to_owned();
    }
    let (sjis, bad) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(b);
    if !bad && sjis.chars().any(|c| matches!(c, '\u{3041}'..='\u{30ff}')) {
        return sjis.into_owned();
    }
    // GB2312 text is pairs of bytes >= 0xA1; accented Latin is a high byte followed by plain ASCII
    let (mut pairs, mut cjk, mut i) = (0, 0, 0);
    while i < b.len() {
        if b[i] >= 0x81 {
            pairs += 1;
            if b[i] >= 0xa1 && b.get(i + 1).is_some_and(|&n| n >= 0xa1) {
                cjk += 1;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    let (gbk, bad) = encoding_rs::GBK.decode_without_bom_handling(b);
    if !bad && pairs > 0 && cjk * 10 >= pairs * 9 {
        return gbk.into_owned();
    }
    encoding_rs::WINDOWS_1252.decode_without_bom_handling(b).0.into_owned()
}

pub fn local(path: &Path) -> Option<Found> {
    for (ext, kind) in [("ttml", "ttml"), ("lrc", "lrc"), ("txt", "lrc")] {
        let file = path.with_extension(ext);
        if std::fs::metadata(&file).is_ok_and(|m| m.is_file() && m.len() <= MAX_SIDECAR)
            && let Ok(bytes) = std::fs::read(&file)
        {
            return Some(Found { kind, text: decode_text(&bytes), source: "file" });
        }
    }
    let embedded = lofty::read_from_path(path).ok().and_then(|f| {
        let tag = f.primary_tag().or_else(|| f.first_tag())?;
        tag.get_string(ItemKey::Lyrics).map(str::to_owned)
    });
    embedded.filter(|t| !t.trim().is_empty()).map(|text| Found { kind: "lrc", text, source: "embedded" })
}

/// A cached result: Some("") records that the source had none.
/// shortcut: cached results never expire; delete the lyrics folder to look again
/// A stored answer. "None found" is only trusted for a day: sources add lyrics, and lookups get better.
pub fn cached(key: &str, source: &str, cache: &Path) -> Option<String> {
    let f = cache_file(key, source, cache);
    let text = std::fs::read_to_string(&f).ok()?;
    if text.is_empty() {
        let age = std::fs::metadata(&f).ok()?.modified().ok()?.elapsed().ok()?;
        if age > std::time::Duration::from_secs(24 * 3600) {
            return None;
        }
    }
    Some(text)
}

pub fn store(key: &str, source: &str, cache: &Path, text: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache_file(key, source, cache), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sidecars_and_caches_each_source_apart() {
        let d = tempfile::tempdir().unwrap();
        let song = d.path().join("song.flac");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/a.flac"), &song).unwrap();
        let cache = d.path().join("cache");
        assert_eq!(local(&song), None);

        let key = song.to_string_lossy();
        store(&key, "lrclib", &cache, "[00:01.00]lrclib").unwrap();
        store(&key, "kugou", &cache, "").unwrap();
        assert_eq!(cached(&key, "lrclib", &cache).as_deref(), Some("[00:01.00]lrclib"));
        assert_eq!(cached(&key, "kugou", &cache).as_deref(), Some(""));
        assert_eq!(cached(&key, "netease", &cache), None);

        std::fs::write(d.path().join("song.lrc"), "[00:01.00]sidecar").unwrap();
        let f = local(&song).unwrap();
        assert_eq!((f.kind, f.text.as_str(), f.source), ("lrc", "[00:01.00]sidecar", "file"));
        // a sidecar far too big to be lyrics is ignored
        std::fs::write(d.path().join("song.lrc"), vec![b'a'; MAX_SIDECAR as usize + 1]).unwrap();
        assert_eq!(local(&song), None);
    }

    #[test]
    fn none_found_is_asked_again_after_a_day() {
        let d = tempfile::tempdir().unwrap();
        store("ytdl://x", "lrclib", d.path(), "").unwrap();
        assert_eq!(cached("ytdl://x", "lrclib", d.path()).as_deref(), Some(""));
        let f = std::fs::OpenOptions::new().write(true).open(cache_file("ytdl://x", "lrclib", d.path())).unwrap();
        f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 24 * 3600)).unwrap();
        assert_eq!(cached("ytdl://x", "lrclib", d.path()), None);
        // lyrics found are kept whatever their age
        store("ytdl://y", "lrclib", d.path(), "[00:01.00]a").unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(cache_file("ytdl://y", "lrclib", d.path())).unwrap();
        f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(400 * 24 * 3600)).unwrap();
        assert_eq!(cached("ytdl://y", "lrclib", d.path()).as_deref(), Some("[00:01.00]a"));
    }

    #[test]
    fn decodes_legacy_lyric_and_playlist_encodings() {
        let enc = |e: &'static encoding_rs::Encoding, s: &str| e.encode(s).0.into_owned();
        let jp = "[00:01.00]君の名前を呼んでいた";
        let zh = "[00:01.00]我们一起走过的路";
        let latin = "#EXTINF:1,Mötley Crüe - Café\r\nD:\\Música\\Beyoncé.flac";
        assert_eq!(decode_text(&enc(encoding_rs::SHIFT_JIS, jp)), jp);
        assert_eq!(decode_text(&enc(encoding_rs::GBK, zh)), zh);
        assert_eq!(decode_text(&enc(encoding_rs::WINDOWS_1252, latin)), latin);
        assert_eq!(decode_text("utf8 ✓ ünï".as_bytes()), "utf8 ✓ ünï");
        let mut utf16 = vec![0xff, 0xfe];
        utf16.extend("bom".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(decode_text(&utf16), "bom");
    }
}

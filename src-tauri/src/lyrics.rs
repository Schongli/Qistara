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

pub fn local(path: &Path) -> Option<Found> {
    for (ext, kind) in [("ttml", "ttml"), ("lrc", "lrc"), ("txt", "lrc")] {
        if let Ok(text) = std::fs::read_to_string(path.with_extension(ext)) {
            return Some(Found { kind, text, source: "file" });
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
pub fn cached(key: &str, source: &str, cache: &Path) -> Option<String> {
    std::fs::read_to_string(cache_file(key, source, cache)).ok()
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
    }
}

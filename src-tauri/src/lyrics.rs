// Local lyrics: sidecar file → embedded tag → cached LRCLIB result.
use lofty::prelude::*;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Found {
    pub kind: &'static str,
    pub text: String, // empty = looked up online before and nothing exists
    pub source: &'static str,
}

fn cache_file(path: &Path, cache: &Path) -> PathBuf {
    let mut h = std::hash::DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut h);
    cache.join(format!("{:016x}.lrc", h.finish()))
}

pub fn local(path: &Path, cache: &Path) -> Option<Found> {
    for (ext, kind) in [("ttml", "ttml"), ("lrc", "lrc"), ("txt", "lrc")] {
        if let Ok(text) = std::fs::read_to_string(path.with_extension(ext)) {
            return Some(Found { kind, text, source: "file" });
        }
    }
    let embedded = lofty::read_from_path(path).ok().and_then(|f| {
        let tag = f.primary_tag().or_else(|| f.first_tag())?;
        tag.get_string(ItemKey::Lyrics).map(str::to_owned)
    });
    if let Some(text) = embedded.filter(|t| !t.trim().is_empty()) {
        return Some(Found { kind: "lrc", text, source: "embedded" });
    }
    // shortcut: cached results (including "none found") never expire; delete the lyrics folder to refetch
    std::fs::read_to_string(cache_file(path, cache)).ok().map(|text| Found {
        kind: if text.trim_start().starts_with('<') { "ttml" } else { "lrc" },
        text,
        source: "lrclib",
    })
}

pub fn store(path: &Path, cache: &Path, text: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache_file(path, cache), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_sidecar_then_cache() {
        let d = tempfile::tempdir().unwrap();
        let song = d.path().join("song.flac");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/a.flac"), &song).unwrap();
        let cache = d.path().join("cache");
        assert_eq!(local(&song, &cache), None);

        store(&song, &cache, "[00:01.00]cached").unwrap();
        assert_eq!(local(&song, &cache).unwrap().source, "lrclib");

        std::fs::write(d.path().join("song.lrc"), "[00:01.00]sidecar").unwrap();
        let f = local(&song, &cache).unwrap();
        assert_eq!((f.kind, f.text.as_str(), f.source), ("lrc", "[00:01.00]sidecar", "file"));
    }
}

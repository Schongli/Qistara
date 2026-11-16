// Japanese lyrics for learners: splits lines into words with their dictionary form, reading and part of
// speech, using IPADIC through Lindera. Offline; nothing is sent anywhere.
// The dictionary is optional (most people never open Learn mode on Japanese lyrics): `japanese_install` fetches
// the MeCab IPADIC archive Lindera's own build uses, checks it, and builds it once into app data/dict/ipadic.
// Only *.csv and *.def files from the archive's top folder are unpacked, into one flat folder.
use lindera::dictionary::{load_fs_dictionary, DictionaryBuilder, Metadata};
use lindera::{mode::Mode, segmenter::Segmenter};
use serde::Serialize;
use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

const IPADIC_URL: &str = "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz";
// shortcut: md5 as lindera's build uses, pin SHA-256 once computed from a trusted copy
const IPADIC_MD5: &str = "a95c409f12f1023fce8ef91f991ef042";
/// The archive is about 12 MB; anything far bigger isn't it.
const MAX_ARCHIVE: u64 = 64 << 20;
const MAX_UNPACKED: u64 = 512 << 20;
/// What the page shows as "Learn mode needs the Japanese dictionary".
pub const NOT_INSTALLED: &str = "japanese-dictionary-missing";
/// lindera-ipadic 6.2.0's metadata.json: how the CSV columns map to fields.
const METADATA: &str = r#"{
  "name": "ipadic", "encoding": "UTF-8", "default_word_cost": -10000, "default_left_context_id": 0,
  "default_right_context_id": 0, "default_field_value": "*", "flexible_csv": true, "skip_invalid_cost_or_id": false,
  "normalize_details": true,
  "dictionary_schema": { "fields": ["surface", "left_context_id", "right_context_id", "cost", "part_of_speech",
    "part_of_speech_subcategory_1", "part_of_speech_subcategory_2", "part_of_speech_subcategory_3",
    "conjugation_form", "conjugation_type", "base_form", "reading", "pronunciation"] },
  "user_dictionary_schema": { "fields": ["surface", "part_of_speech", "reading"] }
}"#;

/// One word as IPADIC sees it. `reading` is katakana, empty when the word isn't in the dictionary
/// (English words, names). `pos`, `pos1` and `form` are IPADIC's Japanese tags; the UI turns them into English.
#[derive(Serialize, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub pos: String,
    pub pos1: String,
    pub form: String,
    pub base: String,
    pub reading: String,
}

pub fn dict_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("dict").join("ipadic"))
}

/// The loaded dictionary, dropped again by `japanese_remove` (Windows can't delete a mapped file).
static SEG: Mutex<Option<Arc<Segmenter>>> = Mutex::new(None);

fn segmenter(dir: &Path) -> Result<Arc<Segmenter>, String> {
    let mut held = SEG.lock().unwrap();
    if let Some(s) = held.as_ref() {
        return Ok(s.clone());
    }
    if !dir.join("metadata.json").is_file() {
        return Err(NOT_INSTALLED.into());
    }
    let dict = load_fs_dictionary(dir).map_err(|e| e.to_string())?;
    let seg = Arc::new(Segmenter::new(Mode::Normal, dict, None));
    *held = Some(seg.clone());
    Ok(seg)
}

fn field(d: &[&str], i: usize) -> String {
    d.get(i).filter(|v| **v != "*").map(|v| v.to_string()).unwrap_or_default()
}

/// Words of each line, in order. Spaces are dropped; everything else (punctuation too) is kept.
pub fn words(seg: &Segmenter, lines: &[String]) -> Result<Vec<Vec<Word>>, String> {
    lines
        .iter()
        .map(|line| {
            let mut tokens = seg.segment(Cow::Borrowed(line.as_str())).map_err(|e| e.to_string())?;
            Ok(tokens
                .iter_mut()
                .filter(|t| !t.surface.trim().is_empty())
                .map(|t| {
                    let text = t.surface.to_string();
                    let d = t.details();
                    // IPADIC: pos, pos1, pos2, pos3, conjugation type, conjugation form, base form, reading, pronunciation
                    let base = field(&d, 6);
                    Word {
                        pos: field(&d, 0),
                        pos1: field(&d, 1),
                        form: field(&d, 5),
                        base: if base.is_empty() { text.clone() } else { base },
                        reading: field(&d, 7),
                        text,
                    }
                })
                .collect())
        })
        .collect()
}

fn octal(b: &[u8]) -> Option<u64> {
    let s = std::str::from_utf8(b).ok()?.trim_matches(|c: char| c == '\0' || c == ' ');
    if s.is_empty() { Some(0) } else { u64::from_str_radix(s, 8).ok() }
}

fn cstr(b: &[u8]) -> &str {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    std::str::from_utf8(&b[..end]).unwrap_or("")
}

/// A dictionary source file name: plain, and only the kinds the builder reads.
fn source_name(name: &str) -> bool {
    !name.starts_with('.')
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && (name.ends_with(".csv") || name.ends_with(".def"))
}

/// Unpacks the dictionary sources from a .tar.gz into `to`: regular files directly under the archive's top
/// folder (or at its root), named like a source file. Links, deeper paths and `..` are skipped, never followed.
pub fn unpack(gz: &[u8], to: &Path) -> Result<usize, String> {
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(gz).take(MAX_UNPACKED).read_to_end(&mut tar).map_err(|e| format!("The dictionary archive is damaged: {e}"))?;
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let (mut off, mut n) = (0usize, 0usize);
    while off + 512 <= tar.len() {
        let h = &tar[off..off + 512];
        if h.iter().all(|&b| b == 0) {
            break;
        }
        let size = octal(&h[124..136]).ok_or("The dictionary archive is damaged.")? as usize;
        let start = off + 512;
        let end = start.checked_add(size).filter(|&e| e <= tar.len()).ok_or("The dictionary archive is cut short.")?;
        let (name, prefix) = (cstr(&h[0..100]), cstr(&h[345..500]));
        let path = if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
        let regular = matches!(h[156], b'0' | 0);
        if let Some(file) = parts.last().filter(|f| regular && parts.len() <= 2 && !parts.contains(&"..") && source_name(f)) {
            std::fs::write(to.join(file), &tar[start..end]).map_err(|e| e.to_string())?;
            n += 1;
        }
        off = start + size.div_ceil(512) * 512;
    }
    Ok(n)
}

/// Builds the unpacked sources into a dictionary Lindera can load.
pub fn build(src: &Path, out: &Path) -> Result<(), String> {
    let meta: Metadata = serde_json::from_str(METADATA).map_err(|e| e.to_string())?;
    DictionaryBuilder::new(meta).build_dictionary(src, out).map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
struct Progress {
    stage: &'static str,
    done: u64,
    total: u64,
}

#[tauri::command]
pub async fn japanese_install(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_http::reqwest::{redirect::Policy, Client};
    let dir = dict_dir(&app)?;
    let client = Client::builder()
        .user_agent(crate::UA)
        .redirect(Policy::custom(|a| if a.url().scheme() == "https" { a.follow() } else { a.stop() }))
        .build()
        .map_err(|e| e.to_string())?;
    let mut r = client.get(IPADIC_URL).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    let total = r.content_length().unwrap_or(0);
    if total > MAX_ARCHIVE {
        return Err("The dictionary download is larger than expected, so it was stopped.".into());
    }
    let (mut gz, mut hash, mut last) = (Vec::new(), md5::Context::new(), 0usize);
    while let Some(chunk) = r.chunk().await.map_err(|e| e.to_string())? {
        hash.consume(&chunk);
        gz.extend_from_slice(&chunk);
        if gz.len() as u64 > MAX_ARCHIVE {
            return Err("The dictionary download is larger than expected, so it was stopped.".into());
        }
        if gz.len() - last > 256 * 1024 {
            last = gz.len();
            let _ = app.emit("japanese-progress", Progress { stage: "download", done: gz.len() as u64, total });
        }
    }
    if format!("{:x}", hash.finalize()) != IPADIC_MD5 {
        return Err("The dictionary didn't match its published checksum, so it wasn't installed.".into());
    }
    let _ = app.emit("japanese-progress", Progress { stage: "build", done: 0, total: 0 });
    tauri::async_runtime::spawn_blocking(move || {
        let parent = dir.parent().ok_or("no data folder")?.to_path_buf();
        let (src, tmp) = (parent.join("ipadic-src"), parent.join("ipadic-build"));
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&tmp);
        let res = unpack(&gz, &src).and_then(|n| if n == 0 { Err("The dictionary archive was empty.".into()) } else { build(&src, &tmp) });
        let _ = std::fs::remove_dir_all(&src);
        res.and_then(|_| {
            *SEG.lock().unwrap() = None;
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())
        })
        .inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&tmp);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct Status {
    installed: bool,
    bytes: u64,
}

#[tauri::command]
pub fn japanese_status(app: AppHandle) -> Result<Status, String> {
    let dir = dict_dir(&app)?;
    let bytes = walkdir::WalkDir::new(&dir).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum();
    Ok(Status { installed: dir.join("metadata.json").is_file(), bytes })
}

#[tauri::command]
pub fn japanese_remove(app: AppHandle) -> Result<(), String> {
    let dir = dict_dir(&app)?;
    *SEG.lock().unwrap() = None;
    match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

#[tauri::command]
pub async fn japanese_words(app: AppHandle, lines: Vec<String>) -> Result<Vec<Vec<Word>>, String> {
    let dir = dict_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || words(segmenter(&dir)?.as_ref(), &lines)).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A tar entry: ustar header + data padded to 512 bytes.
    fn entry(name: &str, kind: u8, data: &[u8]) -> Vec<u8> {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[124..135].copy_from_slice(format!("{:011o}", data.len()).as_bytes());
        h[156] = kind;
        h[257..262].copy_from_slice(b"ustar");
        let mut v = h.to_vec();
        v.extend_from_slice(data);
        v.resize(v.len().div_ceil(512) * 512, 0);
        v
    }

    /// A tiny IPADIC-shaped dictionary (lindera's own builder test), packed like the real archive.
    fn archive(extra: &[Vec<u8>]) -> Vec<u8> {
        let root = "mecab-ipadic-test";
        let mut tar = Vec::new();
        tar.extend(entry(&format!("{root}/"), b'5', b""));
        tar.extend(entry(&format!("{root}/char.def"), b'0', "DEFAULT 0 1 0\nKANJI 0 0 2\nHIRAGANA 0 1 2\n0x4E00..0x9FFF KANJI\n0x3041..0x309F HIRAGANA\n".as_bytes()));
        tar.extend(entry(&format!("{root}/unk.def"), b'0', "DEFAULT,0,0,0,記号,一般,*,*,*,*,*\nKANJI,0,0,0,名詞,一般,*,*,*,*,*\nHIRAGANA,0,0,0,名詞,一般,*,*,*,*,*\n".as_bytes()));
        tar.extend(entry(&format!("{root}/Noun.csv"), b'0', "日本,0,0,100,名詞,固有名詞,地域,国,*,*,日本,ニッポン,ニッポン\n本,0,0,200,名詞,一般,*,*,*,*,本,ホン,ホン\n".as_bytes()));
        tar.extend(entry(&format!("{root}/Postp.csv"), b'0', "の,0,0,50,助詞,連体化,*,*,*,*,の,ノ,ノ\n".as_bytes()));
        tar.extend(entry(&format!("{root}/matrix.def"), b'0', b"1 1\n0 0 0\n"));
        for e in extra {
            tar.extend_from_slice(e);
        }
        tar.extend([0u8; 1024]);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn unpacks_builds_and_splits_with_a_downloaded_dictionary() {
        let d = tempfile::tempdir().unwrap();
        let (src, out) = (d.path().join("src"), d.path().join("ipadic"));
        assert_eq!(unpack(&archive(&[]), &src).unwrap(), 5);
        build(&src, &out).unwrap();
        let seg = Segmenter::new(Mode::Normal, load_fs_dictionary(&out).unwrap(), None);
        let got = words(&seg, &["日本の本".into(), "".into()]).unwrap();
        let w = &got[0];
        assert_eq!(w.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), ["日本", "の", "本"]);
        assert_eq!((w[0].base.as_str(), w[0].reading.as_str(), w[0].pos.as_str()), ("日本", "ニッポン", "名詞"));
        assert_eq!((w[1].pos.as_str(), w[1].pos1.as_str()), ("助詞", "連体化"));
        assert!(got[1].is_empty());
    }

    #[test]
    fn unpacking_never_leaves_the_folder_or_follows_links() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("in").join("src");
        let evil = [
            entry("../evil.csv", b'0', b"x"),
            entry("mecab-ipadic-test/../../evil2.csv", b'0', b"x"),
            entry("mecab-ipadic-test/deep/er.csv", b'0', b"x"),
            entry("mecab-ipadic-test/link.csv", b'2', b""),
            entry("mecab-ipadic-test/run.sh", b'0', b"rm -rf /"),
            entry("mecab-ipadic-test/.hidden.def", b'0', b"x"),
        ];
        assert_eq!(unpack(&archive(&evil), &src).unwrap(), 5);
        let names: std::collections::BTreeSet<_> = std::fs::read_dir(&src).unwrap().flatten().map(|e| e.file_name().into_string().unwrap()).collect();
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["Noun.csv", "Postp.csv", "char.def", "matrix.def", "unk.def"]);
        assert!(!d.path().join("in").join("evil.csv").exists() && !d.path().join("evil2.csv").exists());
    }

    #[test]
    fn a_damaged_archive_is_an_error_not_a_panic() {
        let d = tempfile::tempdir().unwrap();
        assert!(unpack(b"not gzip at all", d.path()).is_err());
        let mut cut = archive(&[]);
        cut.truncate(cut.len() / 2);
        assert!(unpack(&cut, d.path()).is_err());
        // fuzz: random bytes after a valid gzip header never panic
        let mut x = 0x2545f491u32;
        for _ in 0..300 {
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            let junk: Vec<u8> = (0..2048).map(|_| { x ^= x << 13; x ^= x >> 17; x ^= x << 5; x as u8 }).collect();
            gz.write_all(&junk).unwrap();
            let _ = unpack(&gz.finish().unwrap(), d.path());
        }
    }

    #[test]
    fn says_when_the_dictionary_isnt_installed() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(segmenter(&d.path().join("missing")).err().as_deref(), Some(NOT_INSTALLED));
    }
}

// Translate on this PC: Firefox Translations' models (Bergamot / Marian), one folder per language pair under app
// data/translate/<from>-<to>. The registry and the files come only from Mozilla's fixed storage bucket, and each model
// must match the SHA-256 the registry publishes for it. The translating itself happens in the page's WebAssembly
// worker (mt.worker.ts), which gets the files from `mt_file`: once a pack is installed, nothing leaves the PC.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

const BASE: &str = "https://storage.googleapis.com/moz-fx-translations-data--303e-prod-translations-data";
const REGISTRY: &str = "https://storage.googleapis.com/moz-fx-translations-data--303e-prod-translations-data/db/models.json";
/// The registry is about 200 KB; models are at most about 60 MB unpacked.
const MAX_REGISTRY: usize = 8 << 20;
const MAX_FILE: u64 = 250 << 20;
/// What the page shows as "download this language pack".
pub const NOT_INSTALLED: &str = "translation-pack-missing";

#[derive(Deserialize)]
pub struct Registry {
    #[serde(rename = "baseUrl")]
    base_url: String,
    models: HashMap<String, Vec<Entry>>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "releaseStatus")]
    release_status: Option<String>,
    #[serde(rename = "sourceLanguage")]
    source: String,
    #[serde(rename = "targetLanguage")]
    target: String,
    files: Files,
}

#[derive(Deserialize)]
struct Files {
    model: FileRef,
    #[serde(rename = "lexicalShortlist")]
    lex: FileRef,
    vocab: Option<FileRef>,
    #[serde(rename = "srcVocab")]
    src_vocab: Option<FileRef>,
    #[serde(rename = "trgVocab")]
    trg_vocab: Option<FileRef>,
}

#[derive(Deserialize, Clone)]
struct FileRef {
    path: String,
    #[serde(rename = "uncompressedSize")]
    size: Option<u64>,
    #[serde(rename = "uncompressedHash")]
    hash: Option<String>,
}

/// A pack the page can offer: a language pair and about how much it takes on disk.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Pack {
    pub from: String,
    pub to: String,
    pub bytes: u64,
}

/// Mozilla's codes: "id", "ja", "zh", "zh_hant".
pub fn valid_lang(s: &str) -> bool {
    (2..=8).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

/// models/<pair>/<run>/exported/<file>.gz, nothing else.
fn valid_path(p: &str) -> bool {
    let parts: Vec<&str> = p.split('/').collect();
    parts.len() == 5
        && parts[0] == "models"
        && parts[3] == "exported"
        && p.ends_with(".gz")
        && parts.iter().all(|s| !s.is_empty() && *s != ".." && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b)))
}

/// The entry Firefox ships on desktop: "Release Desktop", else "Release" (Android-only and Nightly entries are left out).
fn pick(entries: &[Entry]) -> Option<&Entry> {
    let by = |s: &str| entries.iter().find(|e| e.release_status.as_deref() == Some(s));
    by("Release Desktop").or_else(|| by("Release"))
}

/// The files a pack consists of: (file name on disk, where it comes from).
fn files(e: &Entry) -> Option<Vec<(&'static str, &FileRef)>> {
    let f = &e.files;
    let mut v = vec![("model.bin", &f.model), ("lex.bin", &f.lex)];
    match (&f.vocab, &f.src_vocab, &f.trg_vocab) {
        (Some(x), _, _) => v.push(("vocab.spm", x)),
        (None, Some(s), Some(t)) => v.extend([("srcvocab.spm", s), ("trgvocab.spm", t)]),
        _ => return None,
    }
    v.iter().all(|(_, r)| valid_path(&r.path)).then_some(v)
}

/// The packs the registry offers, by pair. A registry pointing somewhere else than Mozilla's bucket is refused.
pub fn catalog(reg: &Registry) -> Result<Vec<Pack>, String> {
    if reg.base_url.trim_end_matches('/') != BASE {
        return Err("The translation model list points somewhere unexpected, so it wasn't used.".into());
    }
    let mut out: Vec<Pack> = reg
        .models
        .values()
        .filter_map(|v| pick(v))
        .filter(|e| valid_lang(&e.source) && valid_lang(&e.target) && files(e).is_some() && e.files.model.hash.is_some())
        .map(|e| Pack { from: e.source.clone(), to: e.target.clone(), bytes: e.files.model.size.unwrap_or(0) + e.files.lex.size.unwrap_or(0) })
        .collect();
    out.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    Ok(out)
}

fn client() -> Result<tauri_plugin_http::reqwest::Client, String> {
    use tauri_plugin_http::reqwest::{redirect::Policy, Client};
    Client::builder()
        .user_agent(crate::UA)
        .redirect(Policy::custom(|a| if a.url().scheme() == "https" { a.follow() } else { a.stop() }))
        .build()
        .map_err(|e| e.to_string())
}

/// The registry, read once per run.
static REG: Mutex<Option<Arc<Registry>>> = Mutex::new(None);

async fn registry() -> Result<Arc<Registry>, String> {
    if let Some(r) = REG.lock().unwrap().clone() {
        return Ok(r);
    }
    let bytes = client()?.get(REGISTRY).send().await.map_err(|e| format!("network: {e}"))?.error_for_status().map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REGISTRY {
        return Err("The translation model list is larger than expected.".into());
    }
    let r: Arc<Registry> = Arc::new(serde_json::from_slice(&bytes).map_err(|e| format!("the translation model list: {e}"))?);
    *REG.lock().unwrap() = Some(r.clone());
    Ok(r)
}

fn root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("translate"))
}

fn pair_dir(app: &AppHandle, from: &str, to: &str) -> Result<PathBuf, String> {
    if !valid_lang(from) || !valid_lang(to) {
        return Err("Not a language code.".into());
    }
    Ok(root(app)?.join(format!("{from}-{to}")))
}

/// Unpacks a .gz download, refusing anything that grows past `cap`.
pub fn gunzip(gz: &[u8], cap: u64) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(gz).take(cap + 1).read_to_end(&mut out).map_err(|e| format!("a damaged download: {e}"))?;
    if out.len() as u64 > cap {
        return Err("A translation file is larger than expected.".into());
    }
    Ok(out)
}

/// The model must be the one the registry describes.
pub fn matches(bytes: &[u8], hash: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)).eq_ignore_ascii_case(hash)
}

#[derive(Serialize, Clone)]
struct Progress {
    pair: String,
    file: usize,
    files: usize,
    done: u64,
    total: u64,
}

#[tauri::command]
pub async fn mt_catalog() -> Result<Vec<Pack>, String> {
    catalog(&*registry().await?)
}

/// Downloads, checks and installs one pack; another install of the same pair replaces it once it's complete.
#[tauri::command]
pub async fn mt_install(app: AppHandle, from: String, to: String) -> Result<(), String> {
    let dir = pair_dir(&app, &from, &to)?;
    let reg = registry().await?;
    let pair = format!("{from}-{to}");
    let entry = reg.models.get(&pair).and_then(|v| pick(v)).filter(|e| e.source == from && e.target == to).ok_or("Mozilla doesn't offer this language pair.")?;
    let list = files(entry).ok_or("This language pack has files Qistara doesn't know.")?;
    let hash = entry.files.model.hash.clone().ok_or("This language pack has no checksum, so it wasn't installed.")?;
    let client = client()?;
    let mut got: Vec<(&'static str, Vec<u8>)> = Vec::new();
    for (i, (name, f)) in list.iter().enumerate() {
        let mut r = client.get(format!("{BASE}/{}", f.path)).send().await.map_err(|e| format!("network: {e}"))?.error_for_status().map_err(|e| e.to_string())?;
        let total = r.content_length().unwrap_or(0);
        let (mut gz, mut last) = (Vec::new(), 0usize);
        while let Some(chunk) = r.chunk().await.map_err(|e| e.to_string())? {
            gz.extend_from_slice(&chunk);
            if gz.len() as u64 > MAX_FILE {
                return Err("A translation file is larger than expected, so the download was stopped.".into());
            }
            if gz.len() - last > 256 * 1024 {
                last = gz.len();
                let _ = app.emit("mt-progress", Progress { pair: pair.clone(), file: i, files: list.len(), done: gz.len() as u64, total });
            }
        }
        got.push((name, gz));
    }
    let _ = app.emit("mt-progress", Progress { pair: pair.clone(), file: list.len(), files: list.len(), done: 0, total: 0 });
    tauri::async_runtime::spawn_blocking(move || {
        let tmp = dir.with_extension("tmp");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let res = (|| {
            for (name, gz) in &got {
                let bytes = gunzip(gz, MAX_FILE)?;
                if *name == "model.bin" && !matches(&bytes, &hash) {
                    return Err("The language pack didn't match its published checksum, so it wasn't installed.".to_string());
                }
                std::fs::write(tmp.join(name), bytes).map_err(|e| e.to_string())?;
            }
            let meta = serde_json::json!({ "from": from, "to": to, "files": got.iter().map(|(n, _)| *n).collect::<Vec<_>>() });
            std::fs::write(tmp.join("meta.json"), meta.to_string()).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())
        })();
        if res.is_err() {
            let _ = std::fs::remove_dir_all(&tmp);
        }
        res
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The installed packs.
#[tauri::command]
pub fn mt_status(app: AppHandle) -> Result<Vec<Pack>, String> {
    let Ok(rd) = std::fs::read_dir(root(&app)?) else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some((from, to)) = name.split_once('-') else { continue };
        if !valid_lang(from) || !valid_lang(to) || !e.path().join("meta.json").is_file() {
            continue;
        }
        let bytes = std::fs::read_dir(e.path()).into_iter().flatten().flatten().filter_map(|f| f.metadata().ok()).map(|m| m.len()).sum();
        out.push(Pack { from: from.into(), to: to.into(), bytes });
    }
    out.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    Ok(out)
}

#[tauri::command]
pub fn mt_remove(app: AppHandle, from: String, to: String) -> Result<(), String> {
    let dir = pair_dir(&app, &from, &to)?;
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// One file of an installed pack, as raw bytes for the worker.
#[tauri::command]
pub fn mt_file(app: AppHandle, from: String, to: String, kind: String) -> Result<tauri::ipc::Response, String> {
    let name = match kind.as_str() {
        "model" => "model.bin",
        "lex" => "lex.bin",
        "vocab" => "vocab.spm",
        "srcvocab" => "srcvocab.spm",
        "trgvocab" => "trgvocab.spm",
        _ => return Err("Not a language pack file.".into()),
    };
    let path = pair_dir(&app, &from, &to)?.join(name);
    match std::fs::read(&path) {
        Ok(b) => Ok(tauri::ipc::Response::new(b)),
        Err(_) => Err(NOT_INSTALLED.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(base: &str) -> Registry {
        let f = |pair: &str, file: &str| format!("models/{pair}/run1/exported/{file}.gz");
        let entry = |pair: &str, status: Option<&str>, vocab: &str| {
            let (s, t) = pair.split_once('-').unwrap();
            serde_json::json!({
                "architecture": "base", "releaseStatus": status, "sourceLanguage": s, "targetLanguage": t,
                "files": {
                    "model": { "path": f(pair, &format!("model.{status:?}.bin")).replace(['"', ' ', '(', ')'], ""), "uncompressedSize": 1000, "uncompressedHash": "ab" },
                    "lexicalShortlist": { "path": f(pair, "lex.bin") },
                    vocab: { "path": f(pair, "vocab.spm") }
                }
            })
        };
        serde_json::from_value(serde_json::json!({
            "baseUrl": base,
            "models": {
                "ja-en": [entry("ja-en", Some("Release Android"), "vocab"), entry("ja-en", Some("Release Desktop"), "vocab"), entry("ja-en", None, "vocab")],
                "en-id": [entry("en-id", Some("Release"), "vocab")],
                "en-xx": [entry("en-xx", Some("Nightly"), "vocab")],
                "en-zz": [entry("en-zz", Some("Release"), "srcVocab")],
            }
        }))
        .unwrap()
    }

    #[test]
    fn offers_the_desktop_release_of_each_pair_only() {
        let c = catalog(&reg(BASE)).unwrap();
        assert_eq!(c, [Pack { from: "en".into(), to: "id".into(), bytes: 1000 }, Pack { from: "ja".into(), to: "en".into(), bytes: 1000 }]);
        let r = reg(BASE);
        assert_eq!(pick(&r.models["ja-en"]).unwrap().release_status.as_deref(), Some("Release Desktop"));
        // Nightly-only is left out, and a pack with only half its vocabularies too
        assert!(pick(&r.models["en-xx"]).is_none());
        assert!(files(pick(&r.models["en-zz"]).unwrap()).is_none());
    }

    #[test]
    fn refuses_a_registry_that_points_elsewhere() {
        assert!(catalog(&reg("https://example.com/models")).is_err());
        assert!(catalog(&reg(&format!("{BASE}/"))).is_ok());
    }

    #[test]
    fn takes_only_plain_model_paths_and_codes() {
        assert!(valid_path("models/ja-en/next26_HIyjLJaN/exported/model.jaen.intgemm.alphas.bin.gz"));
        for bad in ["models/ja-en/../exported/x.gz", "models/ja-en/r/exported/x.bin", "https://x/models/a/b/exported/c.gz", "models/ja-en/r/other/x.gz", "models/ja en/r/exported/x.gz"] {
            assert!(!valid_path(bad), "{bad}");
        }
        assert!(valid_lang("zh_hant") && valid_lang("id"));
        assert!(!valid_lang("") && !valid_lang("../x") && !valid_lang("EN") && !valid_lang("en-id"));
    }

    #[test]
    #[ignore = "network: run on purpose to see Mozilla's registry still reads the same way"]
    fn live_registry_offers_the_languages_qistara_needs() {
        let c = tauri::async_runtime::block_on(mt_catalog()).unwrap();
        for (f, t) in [("en", "id"), ("id", "en"), ("ja", "en"), ("en", "ja"), ("ar", "en"), ("en", "ar"), ("ko", "en"), ("zh", "en")] {
            assert!(c.iter().any(|p| p.from == f && p.to == t), "{f}-{t}");
        }
        assert!(c.len() > 80, "{}", c.len());
    }

    #[test]
    fn unpacks_and_checks_a_model() {
        use std::io::Write;
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(b"marian model").unwrap();
        let gz = e.finish().unwrap();
        let bytes = gunzip(&gz, 100).unwrap();
        assert_eq!(bytes, b"marian model");
        assert!(gunzip(&gz, 5).is_err(), "bigger than the cap");
        assert!(gunzip(b"not gzip", 100).is_err());
        let sha = format!("{:x}", Sha256::digest(b"marian model"));
        assert!(matches(&bytes, &sha) && matches(&bytes, &sha.to_uppercase()));
        assert!(!matches(b"another model", &sha));
    }
}

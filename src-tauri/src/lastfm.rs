// Last.fm scrobbling. Your own API key/secret and the session key live in Windows Credential
// Manager; requests are signed as Last.fm requires (md5 of sorted params + secret).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const API: &str = "https://ws.audioscrobbler.com/2.0/";
const SERVICE: &str = "Qistara Last.fm";

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Creds {
    pub key: String,
    pub secret: String,
    pub session: Option<String>,
    pub user: Option<String>,
}

pub fn load() -> Option<Creds> {
    crate::keys::load(SERVICE)
}

pub fn store(c: &Creds) -> Result<(), String> {
    crate::keys::store(SERVICE, c)
}

pub fn forget() -> Result<(), String> {
    crate::keys::forget(SERVICE)
}

/// api_sig: every parameter except `format`, sorted by name, as name+value, then the secret, md5.
pub fn sign(params: &BTreeMap<&str, String>, secret: &str) -> String {
    let mut s: String = params.iter().filter(|(k, _)| **k != "format").map(|(k, v)| format!("{k}{v}")).collect();
    s.push_str(secret);
    format!("{:x}", md5::compute(s.as_bytes()))
}

pub async fn call(creds: &Creds, method: &str, mut params: BTreeMap<&str, String>) -> Result<serde_json::Value, String> {
    params.insert("method", method.into());
    params.insert("api_key", creds.key.clone());
    if let Some(sk) = &creds.session {
        params.insert("sk", sk.clone());
    }
    let sig = sign(&params, &creds.secret);
    params.insert("api_sig", sig);
    params.insert("format", "json".into());
    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20)) // a hung request must not stall the scrobble queue
        .build()
        .map_err(|e| e.to_string())?;
    let r = client
        .post(API)
        .header("User-Agent", crate::UA)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;
    let body = r.text().await.map_err(|e| e.to_string())?;
    let j: serde_json::Value = serde_json::from_str(&body).map_err(|_| format!("unexpected reply from Last.fm: {body:.120}"))?;
    match j.get("error") {
        Some(code) => Err(format!("Last.fm error {code}: {}", j["message"].as_str().unwrap_or(""))),
        None => Ok(j),
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Scrobble {
    pub artist: String,
    pub track: String,
    pub album: String,
    pub duration: u32,
    pub timestamp: i64,
}

pub fn scrobble_params(s: &Scrobble) -> BTreeMap<&'static str, String> {
    let mut p = BTreeMap::from([("artist", s.artist.clone()), ("track", s.track.clone()), ("timestamp", s.timestamp.to_string())]);
    if !s.album.is_empty() {
        p.insert("album", s.album.clone());
    }
    if s.duration > 0 {
        p.insert("duration", s.duration.to_string());
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_like_the_last_fm_docs() {
        // name+value pairs in alphabetical order, format excluded, secret appended
        let p = BTreeMap::from([("method", "auth.getToken".to_string()), ("api_key", "abc".to_string()), ("format", "json".to_string())]);
        let expect = format!("{:x}", md5::compute(b"api_keyabcmethodauth.getTokensecret"));
        assert_eq!(sign(&p, "secret"), expect);
    }

    #[test]
    fn builds_scrobble_params_without_empty_fields() {
        let s = Scrobble { artist: "A".into(), track: "T".into(), album: "".into(), duration: 0, timestamp: 42 };
        let p = scrobble_params(&s);
        assert_eq!(p.keys().copied().collect::<Vec<_>>(), vec!["artist", "timestamp", "track"]);
    }
}

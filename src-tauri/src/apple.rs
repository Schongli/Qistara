// Apple Music: your library playlists and songs for import, matched to your own files (Apple's streams are
// DRM-protected, so nothing plays from Apple Music here). Needs your own MusicKit key from the Apple
// Developer Program: the developer token is an ES256 JWT signed here with it. You sign in to your Apple ID in
// your browser, on a loopback page that runs Apple's MusicKit JS and hands the Music User Token back.
// Key and token stay in Credential Manager; requests go to fixed api.music.apple.com library paths.
use crate::spotify::{random, Playlist, Playlists, Track};
use base64::{engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_http::reqwest;

const SERVICE: &str = "Confluence Player Apple Music";
const API: &str = "https://api.music.apple.com";
const UA: &str = concat!("ConfluencePlayer/", env!("CARGO_PKG_VERSION"));

#[derive(Serialize, Deserialize, Default, Clone)]
struct Creds {
    team_id: String,
    key_id: String,
    key: String,
    user_token: Option<String>,
}

fn load() -> Option<Creds> {
    crate::keys::load(SERVICE)
}

/// Team and key ids are 10 upper-case letters and digits.
pub fn valid_apple_id(id: &str) -> bool {
    id.len() == 10 && id.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}
/// Library playlist ids: "p." and letters and digits.
pub fn valid_playlist(id: &str) -> bool {
    id.strip_prefix("p.").is_some_and(|r| (1..=64).contains(&r.len()) && r.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn pem_der(pem: &str) -> Result<Vec<u8>, String> {
    let body: String = pem.lines().filter(|l| !l.starts_with("-----")).map(str::trim).collect();
    STANDARD.decode(body).map_err(|_| "That isn't a MusicKit private key (.p8).".to_string())
}

/// The developer token: an ES256 JWT for 12 hours.
pub fn developer_token(team: &str, key_id: &str, pem: &str, now: u64) -> Result<String, String> {
    use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
    let rng = ring::rand::SystemRandom::new();
    let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &pem_der(pem)?, &rng).map_err(|_| "That isn't a MusicKit private key (.p8).")?;
    let part = |v: Value| URL_SAFE_NO_PAD.encode(v.to_string());
    let input = format!("{}.{}", part(json!({ "alg": "ES256", "kid": key_id })), part(json!({ "iss": team, "iat": now, "exp": now + 43_200 })));
    let sig = key.sign(&rng, input.as_bytes()).map_err(|_| "Couldn't sign the developer token.")?;
    Ok(format!("{input}.{}", URL_SAFE_NO_PAD.encode(sig.as_ref())))
}

fn token_now(c: &Creds) -> Result<String, String> {
    developer_token(&c.team_id, &c.key_id, &c.key, crate::scan::now() as u64)
}

#[derive(Serialize)]
pub struct Status {
    configured: bool,
    user: Option<String>,
}

#[tauri::command]
pub fn apple_status() -> Status {
    let c = load();
    Status { configured: c.as_ref().is_some_and(|c| !c.key.is_empty()), user: c.and_then(|c| c.user_token).map(|_| "your Apple ID".to_string()) }
}

/// Stores your MusicKit key once it signs a token; a new key forgets the old sign-in.
#[tauri::command]
pub fn apple_setup(team_id: String, key_id: String, key_path: String) -> Result<(), String> {
    let (team_id, key_id) = (team_id.trim().to_string(), key_id.trim().to_string());
    if !valid_apple_id(&team_id) || !valid_apple_id(&key_id) {
        return Err("The Team ID and Key ID are each 10 capital letters and digits.".into());
    }
    let meta = std::fs::metadata(&key_path).map_err(|e| e.to_string())?;
    if meta.len() > 8 * 1024 {
        return Err("That file is too big to be a .p8 key.".into());
    }
    let key = std::fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
    let c = Creds { team_id, key_id, key, user_token: None };
    token_now(&c)?;
    crate::keys::store(SERVICE, &c)
}

#[tauri::command]
pub fn apple_logout(all: bool) -> Result<(), String> {
    match load() {
        Some(c) if !all => crate::keys::store(SERVICE, &Creds { user_token: None, ..c }),
        _ => crate::keys::forget(SERVICE),
    }
}

/// The sign-in page: the user clicks, MusicKit JS asks Apple, the token comes back to us. The developer
/// token and state are base64url, so they can't break out of the script strings.
fn page(dev: &str, state: &str) -> String {
    let (dev, state) = (serde_json::to_string(dev).unwrap_or_default(), serde_json::to_string(state).unwrap_or_default());
    format!(
        r#"<!doctype html><meta charset=utf-8><title>Confluence Player</title>
<body style="font:16px system-ui;display:grid;place-items:center;height:90vh;margin:0"><div style="text-align:center">
<p id=m>Sign in with your Apple ID to let Confluence Player read your Apple Music library.</p>
<button id=b disabled style="font:inherit;padding:10px 18px;border-radius:8px">Sign In to Apple Music</button></div>
<script src="https://js-cdn.music.apple.com/musickit/v3/musickit.js" async></script>
<script>
const m = document.getElementById('m'), b = document.getElementById('b')
document.addEventListener('musickitloaded', async () => {{
  await MusicKit.configure({{ developerToken: {dev}, app: {{ name: 'Confluence Player', build: '{v}' }} }})
  b.disabled = false
  b.onclick = async () => {{
    try {{
      const token = await MusicKit.getInstance().authorize()
      await fetch('/token?state=' + encodeURIComponent({state}), {{ method: 'POST', body: token }})
      m.textContent = 'Confluence Player is connected to Apple Music. You can close this tab.'
      b.remove()
    }} catch (e) {{ m.textContent = 'Sign-in did not finish. Go back to Confluence Player and try again.' }}
  }}
}})
</script>"#,
        v = env!("CARGO_PKG_VERSION")
    )
}

fn respond(s: &mut std::net::TcpStream, status: &str, kind: &str, body: &str) {
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len());
    let _ = s.write_all(head.as_bytes()).and_then(|_| s.write_all(body.as_bytes()));
}

/// Reads one request: its head (request line and headers) and its body (up to 16 KB).
fn read_request(s: &mut std::net::TcpStream) -> (String, String) {
    let _ = s.set_nonblocking(false);
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let (mut buf, mut chunk) = (Vec::new(), [0u8; 4096]);
    while buf.len() < 16 * 1024 {
        let n = s.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(end) = text.find("\r\n\r\n") {
            let len = text[..end].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())).unwrap_or(0);
            if buf.len() >= end + 4 + len {
                return (text[..end].to_string(), text[end + 4..end + 4 + len.min(text.len() - end - 4)].to_string());
            }
        }
    }
    (String::from_utf8_lossy(&buf).to_string(), String::new())
}

/// Only our own address: a web page that rebinds its domain to 127.0.0.1 sends its own Host, and so can't
/// read the sign-in page (and the developer token in it).
pub fn host_is_ours(head: &str, port: u16) -> bool {
    let want = format!("127.0.0.1:{port}");
    head.lines().skip(1).filter_map(|l| l.split_once(':')).any(|(k, v)| k.trim().eq_ignore_ascii_case("host") && v.trim() == want)
}

/// A Music User Token: printable, no spaces, a sensible length.
pub fn valid_user_token(t: &str) -> bool {
    (20..=4096).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_graphic())
}

fn serve(listener: &TcpListener, port: u16, dev: &str, state: &str) -> Result<String, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let end = Instant::now() + Duration::from_secs(600);
    while Instant::now() < end {
        match listener.accept() {
            Ok((mut s, _)) => {
                let (head, body) = read_request(&mut s);
                let line = head.lines().next().unwrap_or("");
                if !host_is_ours(&head, port) {
                    respond(&mut s, "403 Forbidden", "text/plain", "");
                } else if line.starts_with("GET / ") || line.starts_with("GET /?") {
                    respond(&mut s, "200 OK", "text/html; charset=utf-8", &page(dev, state));
                } else if line.starts_with(&format!("POST /token?state={state} ")) && valid_user_token(body.trim()) {
                    respond(&mut s, "200 OK", "text/plain", "ok");
                    return Ok(body.trim().to_string());
                } else {
                    respond(&mut s, "404 Not Found", "text/plain", "");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(150)),
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Apple Music sign-in timed out. Try again.".into())
}

#[tauri::command]
pub fn apple_begin(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let c = load().filter(|c| !c.key.is_empty()).ok_or("Add your MusicKit key first.")?;
    let dev = token_now(&c)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let state = random(16)?;
    app.opener().open_url(format!("http://127.0.0.1:{port}/"), None::<&str>).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let done = serve(&listener, port, &dev, &state).and_then(|t| crate::keys::store(SERVICE, &Creds { user_token: Some(t), ..c }));
        let _ = app.emit("apple-changed", done.err());
    });
    Ok(())
}

/// One library request; `path` is ours or a `next` link Apple sent, both under /v1/me/library/.
async fn get(path: &str) -> Result<Value, String> {
    if !path.starts_with("/v1/me/library/") {
        return Err("bad path".into());
    }
    let c = load().ok_or("Sign in to Apple Music in Providers first.")?;
    let user = c.user_token.clone().ok_or("Sign in to Apple Music in Providers first.")?;
    let r = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{API}{path}"))
        .bearer_auth(token_now(&c)?)
        .header("Music-User-Token", user)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;
    match r.status().as_u16() {
        200..=299 => serde_json::from_str(&r.text().await.map_err(|e| e.to_string())?).map_err(|_| "Unexpected reply from Apple Music.".into()),
        401 | 403 => Err("Apple Music refused the sign-in. Sign in again in Providers (and check your key is a MusicKit key).".into()),
        429 => Err("Apple Music is limiting requests. Try again in a minute.".into()),
        s => Err(format!("Apple Music error {s}.")),
    }
}

async fn all(path: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut next = Some(path.to_string());
    for _ in 0..500 {
        let Some(p) = next.take() else { break };
        let page = get(&p).await?;
        out.extend(page["data"].as_array().cloned().unwrap_or_default());
        next = page["next"].as_str().map(str::to_string);
    }
    Ok(out)
}

pub fn track_of(t: &Value) -> Option<Track> {
    let a = &t["attributes"];
    Some(Track {
        title: a["name"].as_str()?.to_string(),
        artists: a["artistName"].as_str().map(|n| vec![n.to_string()]).unwrap_or_default(),
        album: a["albumName"].as_str().unwrap_or("").to_string(),
        duration: a["durationInMillis"].as_f64().unwrap_or(0.0) / 1000.0,
        isrc: None,
    })
}

/// Your library playlists; `liked` stands for your whole library of songs (no count up front).
#[tauri::command]
pub async fn apple_playlists() -> Result<Playlists, String> {
    let playlists = all("/v1/me/library/playlists?limit=100")
        .await?
        .iter()
        .filter_map(|p| {
            let id = p["id"].as_str().filter(|id| valid_playlist(id))?;
            let a = &p["attributes"];
            Some(Playlist {
                id: id.to_string(),
                name: a["name"].as_str().unwrap_or("Untitled").to_string(),
                tracks: 0,
                image: a["artwork"]["url"].as_str().filter(|u| u.starts_with("https://")).map(|u| u.replace("{w}", "80").replace("{h}", "80")),
            })
        })
        .collect();
    Ok(Playlists { liked: 0, playlists })
}

/// The songs of a library playlist, or every song in your library when `playlist` is None.
#[tauri::command]
pub async fn apple_tracks(playlist: Option<String>) -> Result<Vec<Track>, String> {
    let path = match playlist {
        Some(id) if valid_playlist(&id) => format!("/v1/me/library/playlists/{id}/tracks?limit=100"),
        Some(_) => return Err("Not an Apple Music library playlist id.".into()),
        None => "/v1/me/library/songs?limit=100".into(),
    };
    Ok(all(&path).await?.iter().filter_map(track_of).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{EcdsaKeyPair, KeyPair, UnparsedPublicKey, ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING};

    #[test]
    fn signs_a_developer_token_that_verifies() {
        let rng = ring::rand::SystemRandom::new();
        let der = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let pem = format!("-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n", STANDARD.encode(der.as_ref()));
        let public = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, der.as_ref(), &rng).unwrap().public_key().as_ref().to_vec();
        let jwt = developer_token("ABCDE12345", "KEY1234567", &pem, 1_000).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!((claims["iss"].as_str(), claims["exp"].as_u64()), (Some("ABCDE12345"), Some(44_200)));
        let header: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!((header["alg"].as_str(), header["kid"].as_str()), (Some("ES256"), Some("KEY1234567")));
        let sig = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, public).verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &sig).unwrap();
        assert!(developer_token("ABCDE12345", "KEY1234567", "-----BEGIN PRIVATE KEY-----\nbm9wZQ==\n-----END PRIVATE KEY-----", 0).is_err());
    }

    #[test]
    fn checks_ids_and_tokens() {
        assert!(valid_apple_id("ABCDE12345") && !valid_apple_id("abcde12345") && !valid_apple_id("ABC"));
        assert!(valid_playlist("p.ZOAXxWKUoXJrP0") && !valid_playlist("p.../../me") && !valid_playlist("pl.abc"));
        assert!(valid_user_token(&"A".repeat(300)) && !valid_user_token("short") && !valid_user_token(&format!("{} x", "A".repeat(30))));
    }

    #[test]
    fn reads_library_songs() {
        let t = track_of(&json!({"attributes": {"name": "Song", "artistName": "Band", "albumName": "Record", "durationInMillis": 200000}})).unwrap();
        assert_eq!((t.title.as_str(), t.artists[0].as_str(), t.album.as_str(), t.duration), ("Song", "Band", "Record", 200.0));
        assert!(track_of(&json!({"attributes": {}})).is_none());
    }

    #[test]
    fn answers_only_requests_for_its_own_address() {
        assert!(host_is_ours("GET / HTTP/1.1\r\nHost: 127.0.0.1:5000\r\nAccept: */*", 5000));
        assert!(!host_is_ours("GET / HTTP/1.1\r\nHost: evil.example:5000", 5000));
        assert!(!host_is_ours("GET / HTTP/1.1\r\nHost: 127.0.0.1:5001", 5000));
        assert!(!host_is_ours("GET / HTTP/1.1\r\nX-Host: 127.0.0.1:5000", 5000));
    }

    #[test]
    fn the_sign_in_page_quotes_what_it_embeds() {
        let p = page("eyJh.eyJi.c2ln", "st_ate-1");
        assert!(p.contains(r#"developerToken: "eyJh.eyJi.c2ln""#) && p.contains(r#"encodeURIComponent("st_ate-1")"#));
        assert!(p.contains("https://js-cdn.music.apple.com/musickit/v3/musickit.js"));
    }
}

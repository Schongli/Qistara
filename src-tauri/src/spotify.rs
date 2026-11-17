// Spotify: sign-in with PKCE through a loopback redirect (your own developer app, no client secret),
// reading your playlists for import, and Spotify Connect remote control. The refresh token lives in
// Windows Credential Manager, the access token only in memory. Every call goes to a fixed endpoint
// and IDs are validated, so nothing from the webview can steer a request elsewhere.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_http::reqwest;

const SERVICE: &str = "Qistara Spotify";
const API: &str = "https://api.spotify.com/v1";
const ACCOUNTS: &str = "https://accounts.spotify.com";
use crate::UA;
pub const SCOPES: &str =
    "playlist-read-private playlist-read-collaborative user-library-read user-read-playback-state user-modify-playback-state";

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Creds {
    pub client_id: String,
    pub refresh: Option<String>,
    pub id: Option<String>,
    pub user: Option<String>,
}

fn load() -> Option<Creds> {
    crate::keys::load(SERVICE)
}
fn store(c: &Creds) -> Result<(), String> {
    crate::keys::store(SERVICE, c)
}

/// The current access token and when it stops working; kept in memory only.
#[derive(Default)]
pub struct Token(tauri::async_runtime::Mutex<Option<(String, Instant)>>);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ---- sign-in (Authorization Code with PKCE, RFC 7636) ----

pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn random(bytes: usize) -> Result<String, String> {
    let mut b = vec![0u8; bytes];
    getrandom::fill(&mut b).map_err(err)?;
    Ok(URL_SAFE_NO_PAD.encode(b))
}

/// Spotify allows any port on a loopback redirect registered as http://127.0.0.1/callback.
pub fn redirect(port: u16) -> String {
    format!("http://127.0.0.1:{port}/callback")
}

pub fn authorize_url(client_id: &str, port: u16, challenge: &str, state: &str) -> String {
    let params = [
        ("client_id", client_id),
        ("response_type", "code"),
        ("redirect_uri", &redirect(port)),
        ("code_challenge_method", "S256"),
        ("code_challenge", challenge),
        ("state", state),
        ("scope", SCOPES),
    ];
    url::Url::parse_with_params(&format!("{ACCOUNTS}/authorize"), params).expect("fixed URL").into()
}

/// The authorization code from the browser's "GET /callback?…" request, if its state matches ours.
/// `name`: the service, for the messages (shared with Tidal).
pub fn parse_callback(request: &str, state: &str, name: &str) -> Result<String, String> {
    let target = request.split_whitespace().nth(1).unwrap_or("");
    let u = url::Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| format!("{name} sent an unreadable reply."))?;
    let q = |k: &str| u.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.into_owned());
    if let Some(e) = q("error") {
        return Err(if e == "access_denied" { format!("{name} sign-in was cancelled.") } else { format!("{name} sign-in failed: {e}") });
    }
    if q("state").as_deref() != Some(state) {
        return Err("The sign-in reply didn't match this request. Try signing in again.".into());
    }
    q("code").filter(|c| !c.is_empty()).ok_or_else(|| format!("{name} sent no sign-in code."))
}

const PAGE: &str = "<!doctype html><meta charset=utf-8><title>Qistara</title>\
<body style=\"font:16px system-ui;display:grid;place-items:center;height:90vh;margin:0\"><p>";

/// Waits for the browser to come back to our loopback port; other requests (favicon, preconnects) are ignored.
pub fn wait_for_code(listener: &TcpListener, state: &str, limit: Duration, name: &str) -> Result<String, String> {
    listener.set_nonblocking(true).map_err(err)?;
    let end = Instant::now() + limit;
    while Instant::now() < end {
        match listener.accept() {
            Ok((mut s, _)) => {
                let _ = s.set_nonblocking(false);
                let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buf = [0u8; 8192];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                if !req.starts_with("GET /callback") {
                    let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                }
                let result = parse_callback(&req, state, name);
                // fixed text only: nothing from the request is echoed into the page
                let text = if result.is_ok() { format!("Qistara is connected to {name}. You can close this tab.") } else { "Sign-in didn't finish. Go back to Qistara for details.".to_string() };
                let body = format!("{PAGE}{text}</p>");
                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(head.as_bytes()).and_then(|_| s.write_all(body.as_bytes()));
                return result;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(150)),
            Err(e) => return Err(err(e)),
        }
    }
    Err(format!("{name} sign-in timed out. Try signing in again."))
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(20)).build().map_err(err)
}

async fn token_request(form: &[(&str, &str)]) -> Result<Value, String> {
    let r = client()?.post(format!("{ACCOUNTS}/api/token")).header("User-Agent", UA).form(form).send().await.map_err(|e| format!("network: {e}"))?;
    let ok = r.status().is_success();
    let body = r.text().await.map_err(err)?;
    let j: Value = serde_json::from_str(&body).map_err(|_| format!("unexpected reply from Spotify: {body:.120}"))?;
    if !ok {
        let why = j["error_description"].as_str().or(j["error"].as_str()).unwrap_or("unknown error");
        return Err(format!("Spotify sign-in failed: {why}"));
    }
    Ok(j)
}

/// The access token and its expiry from a token reply; a rotated refresh token is stored.
fn read_token(c: &mut Creds, j: &Value) -> Result<(String, Instant), String> {
    let token = j["access_token"].as_str().ok_or("Spotify sent no access token.")?.to_string();
    let life = j["expires_in"].as_u64().unwrap_or(3600).saturating_sub(60);
    if let Some(r) = j["refresh_token"].as_str().filter(|r| c.refresh.as_deref() != Some(r)) {
        c.refresh = Some(r.to_string());
        store(c)?;
    }
    Ok((token, Instant::now() + Duration::from_secs(life)))
}

async fn finish(app: &AppHandle, mut c: Creds, code: &str, port: u16, verifier: &str) -> Result<(), String> {
    let j = token_request(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &redirect(port)),
        ("client_id", &c.client_id),
        ("code_verifier", verifier),
    ])
    .await?;
    *app.state::<Token>().0.lock().await = Some(read_token(&mut c, &j)?);
    let me = call(app, reqwest::Method::GET, "/me", &[]).await?.unwrap_or_default();
    c.id = me["id"].as_str().map(str::to_string);
    c.user = me["display_name"].as_str().filter(|n| !n.is_empty()).or(me["id"].as_str()).map(str::to_string);
    store(&c)
}

async fn access(app: &AppHandle, renew: bool) -> Result<String, String> {
    let state = app.state::<Token>();
    // held across the refresh, so two requests never trade the same refresh token at once
    let mut held = state.0.lock().await;
    if let Some((t, until)) = held.as_ref().filter(|_| !renew)
        && Instant::now() < *until {
            return Ok(t.clone());
        }
    let mut c = load().filter(|c| c.refresh.is_some()).ok_or("Connect Spotify in Settings first.")?;
    let refresh = c.refresh.clone().unwrap_or_default();
    let j = token_request(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", &c.client_id)])
        .await
        .map_err(|e| if e.contains("invalid_grant") || e.contains("revoked") { "Spotify access was removed. Connect again in Settings.".into() } else { e })?;
    let (token, until) = read_token(&mut c, &j)?;
    *held = Some((token.clone(), until));
    Ok(token)
}

/// One Web API request. A stale token is renewed once; Spotify's refusals become plain sentences.
async fn call(app: &AppHandle, method: reqwest::Method, path: &str, query: &[(&str, String)]) -> Result<Option<Value>, String> {
    for attempt in 0..2 {
        let token = access(app, attempt == 1).await?;
        let mut req = client()?.request(method.clone(), format!("{API}{path}")).query(query).bearer_auth(&token).header("User-Agent", UA);
        if method != reqwest::Method::GET {
            req = req.body(""); // Spotify wants a Content-Length on PUT/POST
        }
        let r = req.send().await.map_err(|e| format!("network: {e}"))?;
        let status = r.status().as_u16();
        let wait = r.headers().get("retry-after").and_then(|v| v.to_str().ok()).unwrap_or("a few").to_string();
        let body = r.text().await.map_err(err)?;
        if status == 401 && attempt == 0 {
            continue;
        }
        let j: Option<Value> = serde_json::from_str(&body).ok();
        if (200..300).contains(&status) {
            return Ok(j);
        }
        let msg = j.as_ref().and_then(|j| j["error"]["message"].as_str()).unwrap_or("").to_string();
        let reason = j.as_ref().and_then(|j| j["error"]["reason"].as_str()).unwrap_or("").to_string();
        return Err(match status {
            429 => format!("Spotify is limiting requests. Try again in {wait} seconds."),
            404 if reason == "NO_ACTIVE_DEVICE" || path.starts_with("/me/player") => "No active Spotify device. Start playing in the Spotify app first.".into(),
            403 if reason == "PREMIUM_REQUIRED" || msg.contains("Premium") => "This needs Spotify Premium.".into(),
            403 => format!("Spotify refused ({msg}). Check that your Spotify account is listed under User Management in your developer app."),
            _ => format!("Spotify error {status}: {msg}"),
        });
    }
    Err("Spotify keeps rejecting the sign-in. Connect again in Settings.".into())
}

/// Every page of a paged endpoint (50 items each), following `next` by offset.
async fn all_items(app: &AppHandle, path: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut offset = 0u64;
    loop {
        let page = call(app, reqwest::Method::GET, path, &[("limit", "50".into()), ("offset", offset.to_string())]).await?.unwrap_or_default();
        let items = page["items"].as_array().cloned().unwrap_or_default();
        let last = items.is_empty() || page["next"].is_null();
        offset += items.len() as u64;
        out.extend(items);
        if last {
            return Ok(out);
        }
    }
}

/// Spotify IDs are 22 base-62 characters; anything else never reaches a URL path.
pub fn valid_id(id: &str) -> bool {
    id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub fn valid_client_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

// ---- commands ----

#[derive(Serialize)]
pub struct Status {
    configured: bool,
    user: Option<String>,
}

#[tauri::command]
pub fn spotify_status() -> Status {
    let c = load();
    Status { configured: c.as_ref().is_some_and(|c| !c.client_id.is_empty()), user: c.filter(|c| c.refresh.is_some()).and_then(|c| c.user) }
}

#[tauri::command]
pub async fn spotify_set_client(token: State<'_, Token>, client_id: String) -> Result<(), String> {
    let id = client_id.trim().to_lowercase();
    if !valid_client_id(&id) {
        return Err("A Spotify Client ID is 32 letters and digits (0-9, a-f).".into());
    }
    *token.0.lock().await = None;
    store(&Creds { client_id: id, ..Default::default() })
}

#[tauri::command]
pub fn spotify_begin(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let c = load().filter(|c| !c.client_id.is_empty()).ok_or("Add your Client ID first.")?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(err)?;
    let port = listener.local_addr().map_err(err)?.port();
    let verifier = random(64)?;
    let state = random(16)?;
    app.opener().open_url(authorize_url(&c.client_id, port, &challenge(&verifier), &state), None::<&str>).map_err(err)?;
    std::thread::spawn(move || {
        let done = wait_for_code(&listener, &state, Duration::from_secs(300), "Spotify")
            .and_then(|code| tauri::async_runtime::block_on(finish(&app, c, &code, port, &verifier)));
        let _ = app.emit("spotify-changed", done.err());
    });
    Ok(())
}

/// Forgets the account (refresh token, user, in-memory token); `all` forgets the Client ID as well.
#[tauri::command]
pub async fn spotify_logout(token: State<'_, Token>, all: bool) -> Result<(), String> {
    *token.0.lock().await = None;
    match load() {
        Some(c) if !all => store(&Creds { client_id: c.client_id, ..Default::default() }),
        _ => crate::keys::forget(SERVICE),
    }
}

#[derive(Serialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub tracks: u64,
    pub image: Option<String>,
}

#[derive(Serialize)]
pub struct Playlists {
    pub liked: u64,
    pub playlists: Vec<Playlist>,
}

/// Your own and collaborative playlists (Spotify refuses to list the items of anyone else's), plus Liked Songs.
#[tauri::command]
pub async fn spotify_playlists(app: AppHandle) -> Result<Playlists, String> {
    let me = load().and_then(|c| c.id).unwrap_or_default();
    let liked = call(&app, reqwest::Method::GET, "/me/tracks", &[("limit", "1".into())]).await?.unwrap_or_default()["total"].as_u64().unwrap_or(0);
    let playlists = all_items(&app, "/me/playlists")
        .await?
        .into_iter()
        .filter(|p| p["owner"]["id"].as_str() == Some(me.as_str()) || p["collaborative"].as_bool() == Some(true))
        .filter_map(|p| {
            Some(Playlist {
                id: p["id"].as_str().filter(|id| valid_id(id))?.to_string(),
                name: p["name"].as_str().unwrap_or("Untitled").to_string(),
                // the count moved from `tracks` to `items` in 2026; read either
                tracks: p["items"]["total"].as_u64().or(p["tracks"]["total"].as_u64()).unwrap_or(0),
                image: p["images"][0]["url"].as_str().map(str::to_string),
            })
        })
        .collect();
    Ok(Playlists { liked, playlists })
}

#[derive(Serialize)]
pub struct Track {
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration: f64,
    pub isrc: Option<String>,
}

fn track_of(t: &Value) -> Option<Track> {
    if t["type"].as_str() != Some("track") || t["is_local"].as_bool() == Some(true) {
        return None; // podcast episodes and the user's own local files have nothing to match
    }
    Some(Track {
        title: t["name"].as_str()?.to_string(),
        artists: t["artists"].as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str().map(str::to_string)).collect()).unwrap_or_default(),
        album: t["album"]["name"].as_str().unwrap_or("").to_string(),
        duration: t["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0,
        isrc: t["external_ids"]["isrc"].as_str().map(str::to_string),
    })
}

/// The songs of one of your playlists, or of Liked Songs when `playlist` is None.
#[tauri::command]
pub async fn spotify_tracks(app: AppHandle, playlist: Option<String>) -> Result<Vec<Track>, String> {
    let path = match playlist {
        Some(id) if valid_id(&id) => format!("/playlists/{id}/items"),
        Some(_) => return Err("Not a Spotify playlist ID.".into()),
        None => "/me/tracks".into(),
    };
    let items = all_items(&app, &path).await?;
    // playlist items carry the song in `item` (2026) or `track` (before); saved tracks in `track`
    Ok(items.iter().filter_map(|i| track_of(if i["item"].is_object() { &i["item"] } else { &i["track"] })).collect())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    device: String,
    playing: bool,
    progress: f64,
    volume: Option<u64>,
    title: String,
    artists: String,
    album: String,
    image: Option<String>,
    duration: f64,
    url: Option<String>,
}

/// What Spotify Connect is playing right now, on any of your devices; None when nothing is.
#[tauri::command]
pub async fn spotify_player(app: AppHandle) -> Result<Option<Player>, String> {
    let Some(p) = call(&app, reqwest::Method::GET, "/me/player", &[]).await? else { return Ok(None) };
    let item = &p["item"];
    let names = |v: &Value| v.as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    Ok(Some(Player {
        device: p["device"]["name"].as_str().unwrap_or("Spotify").to_string(),
        playing: p["is_playing"].as_bool().unwrap_or(false),
        progress: p["progress_ms"].as_f64().unwrap_or(0.0) / 1000.0,
        volume: p["device"]["volume_percent"].as_u64(),
        title: item["name"].as_str().unwrap_or("").to_string(),
        artists: if item["artists"].is_array() { names(&item["artists"]) } else { item["show"]["name"].as_str().unwrap_or("").to_string() },
        album: item["album"]["name"].as_str().unwrap_or("").to_string(),
        image: item["album"]["images"][0]["url"].as_str().or(item["images"][0]["url"].as_str()).map(str::to_string),
        duration: item["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0,
        url: item["external_urls"]["spotify"].as_str().map(str::to_string),
    }))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Control {
    Play,
    Pause,
    Next,
    Previous,
    Seek { ms: u64 },
    Volume { percent: u8 },
}

#[tauri::command]
pub async fn spotify_control(app: AppHandle, action: Control) -> Result<(), String> {
    use reqwest::Method;
    let (method, path, query) = match action {
        Control::Play => (Method::PUT, "/me/player/play", vec![]),
        Control::Pause => (Method::PUT, "/me/player/pause", vec![]),
        Control::Next => (Method::POST, "/me/player/next", vec![]),
        Control::Previous => (Method::POST, "/me/player/previous", vec![]),
        Control::Seek { ms } => (Method::PUT, "/me/player/seek", vec![("position_ms", ms.to_string())]),
        Control::Volume { percent } => (Method::PUT, "/me/player/volume", vec![("volume_percent", percent.min(100).to_string())]),
    };
    call(&app, method, path, &query).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc_7636() {
        assert_eq!(challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let v = random(64).unwrap();
        assert_eq!(v.len(), 86); // 43–128 characters allowed
        assert!(v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
    }

    #[test]
    fn builds_the_authorize_url() {
        let u = url::Url::parse(&authorize_url("0123456789abcdef0123456789abcdef", 51234, "CH", "ST")).unwrap();
        let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
        assert_eq!(u.host_str(), Some("accounts.spotify.com"));
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:51234/callback");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!((q["code_challenge"].as_str(), q["state"].as_str(), q["response_type"].as_str()), ("CH", "ST", "code"));
        assert_eq!(q["scope"], SCOPES);
    }

    #[test]
    fn reads_the_callback() {
        assert_eq!(parse_callback("GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: x\r\n", "xyz", "Spotify"), Ok("abc".into()));
        assert!(parse_callback("GET /callback?code=abc&state=evil HTTP/1.1", "xyz", "Spotify").unwrap_err().contains("didn't match"));
        assert!(parse_callback("GET /callback?error=access_denied&state=xyz HTTP/1.1", "xyz", "Spotify").unwrap_err().contains("cancelled"));
        assert!(parse_callback("GET /callback?state=xyz HTTP/1.1", "xyz", "Spotify").is_err());
    }

    #[test]
    fn validates_ids() {
        assert!(valid_id("3cEYpjA9oz9GiPac4AsH4n"));
        assert!(!valid_id("3cEYpjA9oz9GiPac4AsH4/"));
        assert!(!valid_id("../../me/player/pause"));
        assert!(valid_client_id("0123456789abcdef0123456789ABCDEF"));
        assert!(!valid_client_id("0123456789abcdef"));
    }

    #[test]
    fn reads_tracks_and_skips_episodes() {
        let t = serde_json::json!({"type": "track", "name": "Song", "artists": [{"name": "A"}, {"name": "B"}], "album": {"name": "LP"}, "duration_ms": 201500, "external_ids": {"isrc": "USX"}});
        let got = track_of(&t).unwrap();
        assert_eq!((got.title.as_str(), got.artists.len(), got.album.as_str(), got.duration, got.isrc.as_deref()), ("Song", 2, "LP", 201.5, Some("USX")));
        assert!(track_of(&serde_json::json!({"type": "episode", "name": "Pod"})).is_none());
        assert!(track_of(&serde_json::json!({"type": "track", "name": "Mine", "is_local": true})).is_none());
    }
}

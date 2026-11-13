// Tidal: sign-in with PKCE through your own developer app (developer.tidal.com), reading your playlists and
// favourite tracks for import, matched to your own files. Tidal's API allows third-party apps no full-length
// playback, so nothing plays from Tidal here. The refresh token lives in Credential Manager, the access
// token in memory; requests go to fixed openapi.tidal.com paths and ids are checked first.
use crate::spotify::{challenge, random, wait_for_code, Playlist, Playlists, Track};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::TcpListener;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_http::reqwest;

const SERVICE: &str = "Confluence Player Tidal";
const API: &str = "https://openapi.tidal.com/v2";
const LOGIN: &str = "https://login.tidal.com/authorize";
const TOKEN: &str = "https://auth.tidal.com/v1/oauth2/token";
const SCOPES: &str = "user.read collection.read playlists.read";
/// Tidal only redirects to a URI registered exactly, port included; Providers shows this one.
pub const PORT: u16 = 47823;
const UA: &str = concat!("ConfluencePlayer/", env!("CARGO_PKG_VERSION"));

#[derive(Serialize, Deserialize, Default, Clone)]
struct Creds {
    client_id: String,
    refresh: Option<String>,
    user: Option<String>,
    country: Option<String>,
}

fn load() -> Option<Creds> {
    crate::keys::load(SERVICE)
}
fn store(c: &Creds) -> Result<(), String> {
    crate::keys::store(SERVICE, c)
}

#[derive(Default)]
pub struct TidalToken(tauri::async_runtime::Mutex<Option<(String, Instant)>>);

pub fn redirect() -> String {
    format!("http://127.0.0.1:{PORT}/callback")
}

pub fn valid_client_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric())
}
/// Playlist ids are UUIDs; track ids are digits.
pub fn valid_playlist(id: &str) -> bool {
    id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}
pub fn valid_track(id: &str) -> bool {
    (1..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
}

pub fn authorize_url(client_id: &str, challenge: &str, state: &str) -> String {
    let params = [
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", &redirect()),
        ("scope", SCOPES),
        ("code_challenge_method", "S256"),
        ("code_challenge", challenge),
        ("state", state),
    ];
    url::Url::parse_with_params(LOGIN, params).expect("fixed URL").into()
}

/// ISO 8601 durations as Tidal sends them ("PT3M20S", "PT1H2M3.5S") in seconds.
pub fn iso_seconds(d: &str) -> f64 {
    let Some(t) = d.strip_prefix("PT") else { return 0.0 };
    let (mut total, mut num) = (0.0, String::new());
    for c in t.chars() {
        match c {
            'H' | 'M' | 'S' => {
                let v: f64 = num.parse().unwrap_or(0.0);
                total += v * match c { 'H' => 3600.0, 'M' => 60.0, _ => 1.0 };
                num.clear();
            }
            _ => num.push(c),
        }
    }
    total
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(20)).build().map_err(|e| e.to_string())
}

async fn token_request(form: &[(&str, &str)]) -> Result<Value, String> {
    let r = client()?.post(TOKEN).header("User-Agent", UA).form(form).send().await.map_err(|e| format!("network: {e}"))?;
    let ok = r.status().is_success();
    let j: Value = serde_json::from_str(&r.text().await.map_err(|e| e.to_string())?).map_err(|_| "Unexpected reply from Tidal.".to_string())?;
    if !ok {
        let why = j["error_description"].as_str().or(j["error"].as_str()).unwrap_or("unknown error");
        return Err(format!("Tidal sign-in failed: {why}"));
    }
    Ok(j)
}

fn read_token(c: &mut Creds, j: &Value) -> Result<(String, Instant), String> {
    let token = j["access_token"].as_str().ok_or("Tidal sent no access token.")?.to_string();
    let life = j["expires_in"].as_u64().unwrap_or(3600).saturating_sub(60);
    if let Some(r) = j["refresh_token"].as_str().filter(|r| c.refresh.as_deref() != Some(r)) {
        c.refresh = Some(r.to_string());
        store(c)?;
    }
    Ok((token, Instant::now() + Duration::from_secs(life)))
}

async fn access(app: &AppHandle, renew: bool) -> Result<String, String> {
    let state = app.state::<TidalToken>();
    let mut held = state.0.lock().await;
    if let Some((t, until)) = held.as_ref().filter(|_| !renew) {
        if Instant::now() < *until {
            return Ok(t.clone());
        }
    }
    let mut c = load().filter(|c| c.refresh.is_some()).ok_or("Sign in to Tidal in Providers first.")?;
    let refresh = c.refresh.clone().unwrap_or_default();
    let j = token_request(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", &c.client_id)]).await?;
    let (token, until) = read_token(&mut c, &j)?;
    *held = Some((token.clone(), until));
    Ok(token)
}

/// One GET under openapi.tidal.com/v2; `path` starts with "/" (ours, or a `links.next` Tidal sent).
async fn get(app: &AppHandle, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err("bad path".into());
    }
    for attempt in 0..2 {
        let token = access(app, attempt == 1).await?;
        let r = client()?
            .get(format!("{API}{path}"))
            .query(query)
            .bearer_auth(&token)
            .header("User-Agent", UA)
            .header("Accept", "application/vnd.api+json")
            .send()
            .await
            .map_err(|e| format!("network: {e}"))?;
        let status = r.status().as_u16();
        if status == 401 && attempt == 0 {
            continue;
        }
        let j: Value = serde_json::from_str(&r.text().await.unwrap_or_default()).unwrap_or_default();
        if (200..300).contains(&status) {
            return Ok(j);
        }
        let why = j["errors"][0]["detail"].as_str().unwrap_or("").to_string();
        return Err(match status {
            429 => "Tidal is limiting requests. Try again in a minute.".into(),
            403 => format!("Tidal refused ({why}). Check the scopes of your developer app."),
            _ => format!("Tidal error {status}: {why}"),
        });
    }
    Err("Tidal keeps rejecting the sign-in. Sign in again in Providers.".into())
}

/// Every page of a JSON:API list: (data, included by "type:id").
async fn all(app: &AppHandle, path: &str, query: Vec<(&str, String)>) -> Result<(Vec<Value>, HashMap<String, Value>), String> {
    let (mut data, mut included) = (Vec::new(), HashMap::new());
    let mut page = get(app, path, &query).await?;
    for _ in 0..200 {
        data.extend(page["data"].as_array().cloned().unwrap_or_default());
        for i in page["included"].as_array().into_iter().flatten() {
            included.insert(format!("{}:{}", i["type"].as_str().unwrap_or(""), i["id"].as_str().unwrap_or("")), i.clone());
        }
        let Some(next) = page["links"]["next"].as_str().map(str::to_string) else { break };
        page = get(app, &next, &[]).await?;
    }
    Ok((data, included))
}

/// (id, track) from a /tracks reply with artists and albums included.
pub fn tracks_of(j: &Value) -> Vec<(String, Track)> {
    let mut inc = HashMap::new();
    for i in j["included"].as_array().into_iter().flatten() {
        inc.insert(format!("{}:{}", i["type"].as_str().unwrap_or(""), i["id"].as_str().unwrap_or("")), i);
    }
    let names = |t: &Value, rel: &str, field: &str| -> Vec<String> {
        t["relationships"][rel]["data"].as_array().into_iter().flatten().filter_map(|r| {
            inc.get(&format!("{}:{}", r["type"].as_str()?, r["id"].as_str()?))?["attributes"][field].as_str().map(str::to_string)
        }).collect()
    };
    j["data"].as_array().into_iter().flatten().filter_map(|t| {
        let a = &t["attributes"];
        let title = a["title"].as_str()?;
        Some((t["id"].as_str()?.to_string(), Track {
            // "Song (Remastered)": Tidal keeps the version apart
            title: a["version"].as_str().filter(|v| !v.is_empty()).map_or(title.to_string(), |v| format!("{title} ({v})")),
            artists: names(t, "artists", "name"),
            album: names(t, "albums", "title").into_iter().next().unwrap_or_default(),
            duration: iso_seconds(a["duration"].as_str().unwrap_or("")),
            isrc: a["isrc"].as_str().map(str::to_string),
        }))
    }).collect()
}

#[derive(Serialize)]
pub struct Status {
    configured: bool,
    user: Option<String>,
    redirect: String,
}

#[tauri::command]
pub fn tidal_status() -> Status {
    let c = load();
    Status { configured: c.as_ref().is_some_and(|c| !c.client_id.is_empty()), user: c.filter(|c| c.refresh.is_some()).and_then(|c| c.user), redirect: redirect() }
}

#[tauri::command]
pub async fn tidal_set_client(token: State<'_, TidalToken>, client_id: String) -> Result<(), String> {
    let id = client_id.trim().to_string();
    if !valid_client_id(&id) {
        return Err("That doesn't look like a Tidal Client ID (letters and digits).".into());
    }
    *token.0.lock().await = None;
    store(&Creds { client_id: id, ..Default::default() })
}

async fn finish(app: &AppHandle, mut c: Creds, code: &str, verifier: &str) -> Result<(), String> {
    let j = token_request(&[("grant_type", "authorization_code"), ("client_id", &c.client_id), ("code", code), ("redirect_uri", &redirect()), ("code_verifier", verifier)]).await?;
    *app.state::<TidalToken>().0.lock().await = Some(read_token(&mut c, &j)?);
    let me = get(app, "/users/me", &[]).await?;
    let a = &me["data"]["attributes"];
    c.user = a["username"].as_str().or(a["firstName"].as_str()).or(me["data"]["id"].as_str()).map(str::to_string);
    c.country = a["country"].as_str().map(str::to_string);
    store(&c)
}

#[tauri::command]
pub fn tidal_begin(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let c = load().filter(|c| !c.client_id.is_empty()).ok_or("Add your Client ID first.")?;
    let listener = TcpListener::bind(("127.0.0.1", PORT)).map_err(|_| format!("Port {PORT} is in use by another program; close it and try again."))?;
    let (verifier, state) = (random(64)?, random(16)?);
    app.opener().open_url(authorize_url(&c.client_id, &challenge(&verifier), &state), None::<&str>).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let done = wait_for_code(&listener, &state, Duration::from_secs(300), "Tidal").and_then(|code| tauri::async_runtime::block_on(finish(&app, c, &code, &verifier)));
        let _ = app.emit("tidal-changed", done.err());
    });
    Ok(())
}

#[tauri::command]
pub async fn tidal_logout(token: State<'_, TidalToken>, all: bool) -> Result<(), String> {
    *token.0.lock().await = None;
    match load() {
        Some(c) if !all => store(&Creds { client_id: c.client_id, ..Default::default() }),
        _ => crate::keys::forget(SERVICE),
    }
}

/// Your playlists (own and saved). `liked` is 0: Tidal doesn't give the favourites count up front.
#[tauri::command]
pub async fn tidal_playlists(app: AppHandle) -> Result<Playlists, String> {
    let (data, inc) = all(&app, "/userCollectionPlaylists/me/relationships/items", vec![("include", "items".into()), ("collectionView", "FLAT".into())]).await?;
    let playlists = data
        .iter()
        .filter(|d| d["type"] == "playlists")
        .filter_map(|d| {
            let id = d["id"].as_str().filter(|id| valid_playlist(id))?;
            let a = &inc.get(&format!("playlists:{id}"))?["attributes"];
            Some(Playlist { id: id.to_string(), name: a["name"].as_str().unwrap_or("Untitled").to_string(), tracks: a["numberOfItems"].as_u64().unwrap_or(0), image: None })
        })
        .collect();
    Ok(Playlists { liked: 0, playlists })
}

/// The tracks of a playlist, or your favourite tracks when `playlist` is None (videos are skipped).
#[tauri::command]
pub async fn tidal_tracks(app: AppHandle, playlist: Option<String>) -> Result<Vec<Track>, String> {
    let path = match playlist {
        Some(id) if valid_playlist(&id) => format!("/playlists/{id}/relationships/items"),
        Some(_) => return Err("Not a Tidal playlist id.".into()),
        None => "/userCollectionTracks/me/relationships/items".into(),
    };
    let country = load().and_then(|c| c.country).unwrap_or_else(|| "US".into());
    let (data, _) = all(&app, &path, vec![("countryCode", country.clone())]).await?;
    let ids: Vec<String> = data.iter().filter(|d| d["type"] == "tracks").filter_map(|d| d["id"].as_str().filter(|i| valid_track(i)).map(str::to_string)).collect();
    let mut out = Vec::new();
    for chunk in ids.chunks(20) {
        let mut q: Vec<(&str, String)> = vec![("countryCode", country.clone()), ("include", "artists".into()), ("include", "albums".into())];
        q.extend(chunk.iter().map(|i| ("filter[id]", i.clone())));
        let page = get(&app, "/tracks", &q).await?;
        // keep the playlist's order: /tracks may answer in any order
        let mut by_id: HashMap<String, Track> = tracks_of(&page).into_iter().collect();
        out.extend(chunk.iter().filter_map(|i| by_id.remove(i)));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_iso_durations() {
        assert_eq!(iso_seconds("PT2M58S"), 178.0);
        assert_eq!(iso_seconds("PT1H2M3.5S"), 3723.5);
        assert_eq!(iso_seconds("PT45S"), 45.0);
        assert_eq!(iso_seconds("nonsense"), 0.0);
    }

    #[test]
    fn checks_ids_and_builds_the_sign_in_url() {
        assert!(valid_playlist("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!valid_playlist("../../users/me"));
        assert!(valid_track("75413016") && !valid_track("7541a016"));
        let u = authorize_url("abcDEF123456", "chal", "st");
        assert!(u.starts_with("https://login.tidal.com/authorize?"));
        assert!(u.contains("code_challenge=chal") && u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A47823%2Fcallback"));
    }

    #[test]
    fn reads_tracks_with_their_artists_and_album() {
        let j: Value = serde_json::from_str(r#"{
            "data": [{"id":"1","type":"tracks","attributes":{"title":"Song","version":"Remastered","duration":"PT3M20S","isrc":"USAB1"},
                      "relationships":{"artists":{"data":[{"id":"9","type":"artists"}]},"albums":{"data":[{"id":"5","type":"albums"}]}}}],
            "included": [{"id":"9","type":"artists","attributes":{"name":"Band"}},{"id":"5","type":"albums","attributes":{"title":"Record"}}]}"#).unwrap();
        let (id, t) = &tracks_of(&j)[0];
        assert_eq!(id, "1");
        assert_eq!((t.title.as_str(), t.artists.as_slice(), t.album.as_str(), t.duration), ("Song (Remastered)", &["Band".to_string()][..], "Record", 200.0));
    }
}

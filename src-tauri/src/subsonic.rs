// Your own music server over the Subsonic API (Navidrome, Gonic, Airsonic, Ampache...). The address, user and
// password stay in Credential Manager; every request signs with a fresh salt (t = md5(password + salt)) as the
// API asks, and goes only to that server. Calls map to a fixed set of endpoints; ids are checked first.
use crate::Lib;
use serde::{Deserialize, Serialize};
use tauri::State;

const SERVICE: &str = "Qistara Subsonic";
const CLIENT: &str = "Qistara";

#[derive(Serialize, Deserialize, Clone)]
struct Creds {
    server: String,
    user: String,
    password: String,
}

#[derive(Serialize)]
pub struct Account {
    server: String,
    user: String,
}

/// The server address as typed: "music.example.com" or "192.168.1.5:4533" get http:// (as most home
/// servers have no TLS); a path is kept for servers behind a reverse proxy.
pub fn normalize_server(s: &str) -> Result<String, String> {
    let s = s.trim();
    let full = if s.contains("://") { s.to_string() } else { format!("http://{s}") };
    let u = url::Url::parse(&full).map_err(|_| "That isn't a server address.".to_string())?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() || u.query().is_some() || u.fragment().is_some() || !u.username().is_empty() {
        return Err("Use an address like https://music.example.com or 192.168.1.5:4533.".into());
    }
    Ok(u.as_str().trim_end_matches('/').to_string())
}

pub fn valid_id(id: &str) -> bool {
    (1..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn salt() -> Result<String, String> {
    let mut b = [0u8; 12];
    getrandom::fill(&mut b).map_err(|e| e.to_string())?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// A signed request URL for `endpoint` on the server (JSON replies).
fn signed(c: &Creds, endpoint: &str, params: &[(&str, String)], salt: &str) -> Result<url::Url, String> {
    let mut u = url::Url::parse(&format!("{}/rest/{endpoint}", c.server)).map_err(|e| e.to_string())?;
    let token = format!("{:x}", md5::compute(format!("{}{salt}", c.password)));
    u.query_pairs_mut()
        .append_pair("u", &c.user)
        .append_pair("t", &token)
        .append_pair("s", salt)
        .append_pair("v", "1.16.1")
        .append_pair("c", CLIENT)
        .append_pair("f", "json")
        .extend_pairs(params.iter().map(|(k, v)| (*k, v.as_str())));
    Ok(u)
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ListKind {
    Newest,
    AlphabeticalByName,
    Frequent,
    Recent,
    Random,
    Starred,
}

/// Everything the Server view asks for.
#[derive(Deserialize, Debug)]
#[serde(tag = "call", rename_all = "camelCase")]
pub enum Call {
    AlbumList { kind: ListKind, offset: u32 },
    Album { id: String },
    Artists,
    Artist { id: String },
    Search { query: String },
    Playlists,
    Playlist { id: String },
}

/// A Subsonic method and its query parameters.
pub type Endpoint = (&'static str, Vec<(&'static str, String)>);

pub fn endpoint(call: &Call) -> Result<Endpoint, String> {
    let id = |id: &String| if valid_id(id) { Ok(vec![("id", id.clone())]) } else { Err("bad id".to_string()) };
    Ok(match call {
        Call::AlbumList { kind, offset } => {
            let kind = serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
            ("getAlbumList2", vec![("type", kind), ("size", "60".into()), ("offset", offset.to_string())])
        }
        Call::Album { id: i } => ("getAlbum", id(i)?),
        Call::Artists => ("getArtists", vec![]),
        Call::Artist { id: i } => ("getArtist", id(i)?),
        Call::Search { query } => {
            let q = query.trim();
            if q.is_empty() || q.len() > 200 {
                return Err("Type what to search for.".into());
            }
            ("search3", vec![("query", q.to_string()), ("artistCount", "20".into()), ("albumCount", "40".into()), ("songCount", "80".into())])
        }
        Call::Playlists => ("getPlaylists", vec![]),
        Call::Playlist { id: i } => ("getPlaylist", id(i)?),
    })
}

fn client() -> Result<tauri_plugin_http::reqwest::Client, String> {
    tauri_plugin_http::reqwest::Client::builder()
        .user_agent(crate::UA)
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

/// The `subsonic-response` body, or the server's own error message.
pub fn unwrap(j: serde_json::Value) -> Result<serde_json::Value, String> {
    let r = j.get("subsonic-response").cloned().ok_or("That address doesn't answer like a Subsonic server.")?;
    if r["status"] == "ok" {
        Ok(r)
    } else {
        Err(r["error"]["message"].as_str().unwrap_or("The server refused the request.").to_string())
    }
}

async fn get(c: &Creds, endpoint: &str, params: &[(&str, String)]) -> Result<serde_json::Value, String> {
    let u = signed(c, endpoint, params, &salt()?)?;
    let r = client()?.get(u).send().await.map_err(|e| format!("Can't reach the server: {e}"))?;
    let status = r.status();
    let body = r.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("The server answered {}.", status.as_u16()));
    }
    unwrap(serde_json::from_str(&body).map_err(|_| "That address doesn't answer like a Subsonic server.")?)
}

fn creds() -> Result<Creds, String> {
    crate::keys::load(SERVICE).ok_or_else(|| "Not signed in to a server.".to_string())
}

#[tauri::command]
pub fn subsonic_status() -> Option<Account> {
    crate::keys::load::<Creds>(SERVICE).map(|c| Account { server: c.server, user: c.user })
}

#[tauri::command]
pub async fn subsonic_connect(server: String, user: String, password: String) -> Result<Account, String> {
    if user.trim().is_empty() || password.is_empty() {
        return Err("Enter your user name and password.".into());
    }
    let c = Creds { server: normalize_server(&server)?, user: user.trim().to_string(), password };
    get(&c, "ping", &[]).await?;
    crate::keys::store(SERVICE, &c)?;
    Ok(Account { server: c.server, user: c.user })
}

#[tauri::command]
pub fn subsonic_logout() -> Result<(), String> {
    crate::keys::forget(SERVICE)
}

#[tauri::command]
pub async fn subsonic_call(call: Call) -> Result<serde_json::Value, String> {
    let (ep, params) = endpoint(&call)?;
    get(&creds()?, ep, &params).await
}

/// Lyrics for a song on your server, as LRC: OpenSubsonic's timed lyrics when the server has them, else the
/// classic getLyrics text ("" when it has none).
pub async fn lyrics(id: &str, artist: &str, title: &str) -> Result<String, String> {
    if !valid_id(id) {
        return Err("bad id".into());
    }
    let c = creds()?;
    match get(&c, "getLyricsBySongId", &[("id", id.to_string())]).await {
        Ok(r) => {
            let text = structured_to_lrc(&r["lyricsList"]["structuredLyrics"]);
            if !text.is_empty() {
                return Ok(text);
            }
        }
        // can't reach the server: try again later rather than record "none"
        Err(e) if e.starts_with("Can't reach") => return Err(e),
        Err(_) => {} // not an OpenSubsonic server: the classic call below
    }
    let r = get(&c, "getLyrics", &[("artist", artist.to_string()), ("title", title.to_string())]).await?;
    Ok(r["lyrics"]["value"].as_str().unwrap_or_default().to_string())
}

/// OpenSubsonic structuredLyrics → LRC (the timed set if there is one; its offset kept as [offset:]).
fn structured_to_lrc(list: &serde_json::Value) -> String {
    let all = list.as_array().map(Vec::as_slice).unwrap_or_default();
    let Some(l) = all.iter().find(|l| l["synced"] == true).or(all.first()) else { return String::new() };
    let lines = l["line"].as_array().map(Vec::as_slice).unwrap_or_default();
    // one line per entry: a break inside a value would start a timed line of its own
    let value = |x: &serde_json::Value| x["value"].as_str().unwrap_or_default().replace(['\r', '\n'], " ");
    let mut out = String::new();
    if l["synced"] == true {
        if let Some(o) = l["offset"].as_i64().filter(|o| *o != 0) {
            out.push_str(&format!("[offset:{o}]\n"));
        }
        for x in lines {
            let ms = x["start"].as_i64().unwrap_or(0).max(0);
            out.push_str(&format!("[{:02}:{:02}.{:03}]{}\n", ms / 60000, ms / 1000 % 60, ms % 1000, value(x)));
        }
    } else {
        for x in lines {
            out.push_str(&value(x));
            out.push('\n');
        }
    }
    if out.trim().is_empty() { String::new() } else { out }
}

/// The song as the server stores it (format=raw: a FLAC stays bit-perfect), for mpv to play.
#[tauri::command]
pub fn subsonic_stream(id: String) -> Result<String, String> {
    if !valid_id(&id) {
        return Err("bad id".into());
    }
    Ok(signed(&creds()?, "stream", &[("id", id), ("format", "raw".into())], &salt()?)?.to_string())
}

/// A cover, saved into the streaming art cache (the page may only show pictures from there); returns its
/// name as artUrl (library.svelte.ts) reads it: "stream:<hash>.jpg".
#[tauri::command]
pub async fn subsonic_cover(lib: State<'_, Lib>, id: String) -> Result<Option<String>, String> {
    if !valid_id(&id) {
        return Err("bad id".into());
    }
    let u = signed(&creds()?, "getCoverArt", &[("id", id), ("size", "600".into())], &salt()?)?;
    let r = client()?.get(u).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Ok(None);
    }
    let bytes = r.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Ok(None);
    }
    Ok(lib.stream_art.thumb(&bytes).map(|n| format!("stream:{n}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzz_structured_lyrics_from_a_server_never_panic() {
        use serde_json::json;
        let mut x = 7u32;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        };
        let pick = |n: u32| [json!(null), json!(true), json!(-5), json!(1e300), json!("x\n[00:01.00]y"), json!([]), json!({}), json!(i64::MAX)][n as usize % 8].clone();
        for _ in 0..5000 {
            let line = json!({ "start": pick(next()), "value": pick(next()) });
            let doc = json!([{ "synced": pick(next()), "offset": pick(next()), "line": [line.clone(), line, pick(next())] }, pick(next())]);
            let lrc = structured_to_lrc(&doc);
            // a value with a line break must not start a line of its own: one line per entry
            let entries = doc[0]["line"].as_array().unwrap().len() + usize::from(doc[0]["synced"] == true && doc[0]["offset"].as_i64().is_some_and(|o| o != 0));
            assert!(lrc.is_empty() || lrc.lines().count() == entries || doc[0]["synced"] != true && doc[1]["line"].is_null(), "{lrc:?}");
            assert!(!lrc.contains("\n[00:01.00]y"), "{lrc:?}");
        }
    }

    #[test]
    fn reads_server_addresses() {
        assert_eq!(normalize_server("192.168.1.5:4533").unwrap(), "http://192.168.1.5:4533");
        assert_eq!(normalize_server(" https://music.example.com/navidrome/ ").unwrap(), "https://music.example.com/navidrome");
        for bad in ["ftp://x.org", "https://user:pw@x.org", "https://x.org/?a=1", "", "http://"] {
            assert!(normalize_server(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn signs_each_request_with_its_salt() {
        let c = Creds { server: "https://m.example.com".into(), user: "me".into(), password: "sesame".into() };
        let u = signed(&c, "ping", &[], "c19b2d").unwrap();
        let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
        // the Subsonic API's own example: md5("sesame" + "c19b2d")
        assert_eq!(q["t"], "26719a1196d2a940705a59634eb18eab");
        assert_eq!((q["u"].as_str(), q["s"].as_str(), q["f"].as_str()), ("me", "c19b2d", "json"));
        assert!(u.as_str().starts_with("https://m.example.com/rest/ping?"));
        assert!(!u.as_str().contains("sesame"));
    }

    #[test]
    fn maps_calls_to_fixed_endpoints_and_checks_ids() {
        let (ep, p) = endpoint(&Call::AlbumList { kind: ListKind::AlphabeticalByName, offset: 60 }).unwrap();
        assert_eq!(ep, "getAlbumList2");
        assert!(p.contains(&("type", "alphabeticalByName".into())) && p.contains(&("offset", "60".into())));
        assert_eq!(endpoint(&Call::Album { id: "al-3f9c".into() }).unwrap().0, "getAlbum");
        assert!(endpoint(&Call::Album { id: "../../x".into() }).is_err());
        assert!(endpoint(&Call::Playlist { id: "a&u=admin".into() }).is_err());
        assert!(endpoint(&Call::Search { query: "  ".into() }).is_err());
        let c: Call = serde_json::from_str(r#"{"call":"albumList","kind":"newest","offset":0}"#).unwrap();
        assert!(matches!(c, Call::AlbumList { kind: ListKind::Newest, offset: 0 }));
    }

    #[test]
    fn turns_structured_lyrics_into_lrc() {
        let list = serde_json::json!([
            {"lang": "eng", "synced": false, "line": [{"value": "plain"}]},
            {"lang": "eng", "synced": true, "offset": -100, "line": [{"start": 1500, "value": "Hello"}, {"start": 62250, "value": "world"}]}
        ]);
        assert_eq!(structured_to_lrc(&list), "[offset:-100]\n[00:01.500]Hello\n[01:02.250]world\n");
        assert_eq!(structured_to_lrc(&serde_json::json!([{"synced": false, "line": [{"value": "a"}, {"value": "b"}]}])), "a\nb\n");
        assert_eq!(structured_to_lrc(&serde_json::json!(null)), "");
    }

    #[test]
    fn reports_the_servers_own_errors() {
        let ok = serde_json::json!({"subsonic-response": {"status": "ok", "version": "1.16.1"}});
        assert!(unwrap(ok).is_ok());
        let bad = serde_json::json!({"subsonic-response": {"status": "failed", "error": {"code": 40, "message": "Wrong username or password"}}});
        assert_eq!(unwrap(bad).unwrap_err(), "Wrong username or password");
        assert!(unwrap(serde_json::json!({"html": 1})).is_err());
    }
}

// Signing in to YouTube in a small window of Qistara's own, like a desktop app's sign-in sheet: Google's sign-in
// page in a private window (nothing it stores outlives it), allowed onto Google's and YouTube's own sites only and
// given no access to the app. Once YouTube's session cookies appear, Qistara keeps them and closes the window.
//
// The cookies are kept encrypted (AES-256-GCM) in streaming/account.bin, with the key in the OS's secret store:
// Windows' Credential Manager can't hold a whole cookie jar. They go only to yt-dlp (a per-run copy, as a
// cookies.txt), mpv's ytdl hook (its copy while signed in) and YT Music's own API (ytmusic.rs, with the
// SAPISIDHASH YouTube's web player sends). The page never sees them: it gets the account's name and picture.
// Signing out deletes the file, the key and the hook's copy.
use base64::Engine as _;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

const SERVICE: &str = "Qistara YouTube";
const FILE: &str = "account.bin";
/// Google's sign-in for YouTube, landing on YT Music (the address YouTube's own Sign In button opens).
#[cfg(desktop)]
const START: &str = "https://accounts.google.com/ServiceLogin?service=youtube&passive=true&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26app%3Ddesktop%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F";
#[cfg(desktop)]
const LABEL: &str = "youtube-sign-in";

/// One cookie, as a cookies.txt line holds it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Row {
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    /// unix seconds; 0 for a cookie that lasts the browser session
    pub expires: i64,
    pub name: String,
    pub value: String,
}

/// Who is signed in, for the page.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Info {
    pub name: String,
    pub handle: Option<String>,
    pub photo: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    cookies: Vec<Row>,
    info: Info,
}

/// A signed-in account, decrypted once per run.
pub struct Signed {
    pub cookies: Vec<Row>,
    pub info: Info,
}

impl Signed {
    /// The Cookie header for youtube.com's own servers.
    pub fn header(&self) -> String {
        self.cookies.iter().filter(|c| on_youtube(&c.domain)).map(|c| format!("{}={}", c.name, c.value)).collect::<Vec<_>>().join("; ")
    }

    /// What YouTube's web player sends to prove the request comes from the signed-in page: SAPISIDHASH <ts>_<sha1>.
    pub fn authorization(&self, origin: &str) -> Option<String> {
        let sapisid = ["SAPISID", "__Secure-3PAPISID"].iter().find_map(|n| self.cookies.iter().find(|c| c.name == *n && on_youtube(&c.domain)))?;
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        Some(sapisid_hash(&sapisid.value, ts, origin))
    }
}

fn on_youtube(domain: &str) -> bool {
    let d = domain.trim_start_matches('.');
    d == "youtube.com" || d.ends_with(".youtube.com")
}

pub fn sapisid_hash(sapisid: &str, ts: u64, origin: &str) -> String {
    let d = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, format!("{ts} {sapisid} {origin}").as_bytes());
    format!("SAPISIDHASH {ts}_{}", d.as_ref().iter().map(|b| format!("{b:02x}")).collect::<String>())
}

/// The cookies as a cookies.txt (Netscape format), for yt-dlp.
pub fn netscape(rows: &[Row]) -> String {
    let mut out = String::from("# Netscape HTTP Cookie File\n# Qistara's YouTube sign-in\n\n");
    for c in rows.iter().filter(|c| on_youtube(&c.domain) || c.domain.trim_start_matches('.').ends_with("google.com")) {
        let tf = |b: bool| if b { "TRUE" } else { "FALSE" };
        let domain = if c.http_only { format!("#HttpOnly_{}", c.domain) } else { c.domain.clone() };
        out += &format!("{domain}\t{}\t{}\t{}\t{}\t{}\t{}\n", tf(c.domain.starts_with('.')), c.path, tf(c.secure), c.expires.max(0), c.name, c.value);
    }
    out
}

/// YouTube's session cookies are there: the sign-in went through.
pub fn signed_in(rows: &[Row]) -> bool {
    let has = |n: &str| rows.iter().any(|c| c.name == n && on_youtube(&c.domain) && !c.value.is_empty());
    (has("SAPISID") || has("__Secure-3PAPISID")) && (has("SID") || has("__Secure-3PSID"))
}

/// The sign-in window may go only to Google's and YouTube's own https sites (frames from about: included).
pub fn allowed(url: &url::Url) -> bool {
    match url.scheme() {
        "about" => true,
        "https" => url.host_str().is_some_and(|h| ["google.com", "youtube.com", "gstatic.com", "googleapis.com"].iter().any(|d| h == *d || h.ends_with(&format!(".{d}")))),
        _ => false,
    }
}

/// Name, handle and picture from YT Music's account menu.
pub fn parse_account_menu(v: &Value) -> Option<Info> {
    let h = find(v, "activeAccountHeaderRenderer")?;
    let runs = |x: &Value| x["runs"].as_array().map(|r| r.iter().filter_map(|t| t["text"].as_str()).collect::<String>()).filter(|s| !s.is_empty());
    let name = runs(&h["accountName"])?;
    let photo = h["accountPhoto"]["thumbnails"].as_array().and_then(|t| t.last()).and_then(|t| t["url"].as_str()).filter(|u| crate::ytmusic::thumb_host(u)).map(str::to_owned);
    Some(Info { name, handle: runs(&h["channelHandle"]), photo })
}

fn find<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => m.get(key).or_else(|| m.values().find_map(|x| find(x, key))),
        Value::Array(a) => a.iter().find_map(|x| find(x, key)),
        _ => None,
    }
}

// ---- kept encrypted ----

fn seal(key: &[u8; 32], plain: &[u8]) -> Result<Vec<u8>, String> {
    let k = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).map_err(|_| "bad key")?);
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
    let mut data = plain.to_vec();
    k.seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::from(FILE.as_bytes()), &mut data).map_err(|_| "couldn't encrypt")?;
    Ok([nonce.as_slice(), &data].concat())
}

fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Vec<u8>> {
    let (nonce, data) = sealed.split_at_checked(12)?;
    let k = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).ok()?);
    let mut data = data.to_vec();
    let plain = k.open_in_place(Nonce::try_assume_unique_for_key(nonce).ok()?, Aad::from(FILE.as_bytes()), &mut data).ok()?;
    Some(plain.to_vec())
}

/// The account read once per run (signing in or out replaces it).
static CACHE: Mutex<Option<(PathBuf, Option<Arc<Signed>>)>> = Mutex::new(None);

fn key() -> Option<[u8; 32]> {
    let k: String = crate::keys::load(SERVICE)?;
    base64::engine::general_purpose::STANDARD.decode(k).ok()?.try_into().ok()
}

/// The signed-in account kept in `streaming` (app data/streaming), if there is one.
pub fn load(streaming: &Path) -> Option<Arc<Signed>> {
    let file = streaming.join(FILE);
    let mut cache = CACHE.lock().unwrap();
    if let Some((_, s)) = cache.as_ref().filter(|(p, _)| *p == file) {
        return s.clone();
    }
    let read = || -> Option<Arc<Signed>> {
        let plain = open(&key()?, &std::fs::read(&file).ok()?)?;
        let s: Saved = serde_json::from_slice(&plain).ok()?;
        Some(Arc::new(Signed { cookies: s.cookies, info: s.info }))
    };
    let s = if file.exists() { read() } else { None };
    *cache = Some((file, s.clone()));
    s
}

pub fn signed(app: &AppHandle) -> Option<Arc<Signed>> {
    load(&crate::ytdlp::streaming_dir(app, "").ok()?)
}

/// The account's cookies as a cookies.txt, for yt-dlp and mpv's hook (ytdlp.rs).
pub fn cookies_txt(streaming: &Path) -> Result<String, String> {
    load(streaming).map(|s| netscape(&s.cookies)).ok_or_else(|| "You're no longer signed in to YouTube. Sign in again in Providers.".into())
}

fn save(streaming: &Path, rows: Vec<Row>, info: Info) -> Result<(), String> {
    let key = match key() {
        Some(k) => k,
        None => {
            let mut k = [0u8; 32];
            getrandom::fill(&mut k).map_err(|e| e.to_string())?;
            crate::keys::store(SERVICE, &base64::engine::general_purpose::STANDARD.encode(k)).map_err(|e| format!("Couldn't keep the sign-in in the system's secret store: {e}"))?;
            k
        }
    };
    let plain = serde_json::to_vec(&Saved { cookies: rows, info }).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(streaming).map_err(|e| e.to_string())?;
    let file = streaming.join(FILE);
    let part = streaming.join("account.part");
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    std::io::Write::write_all(&mut o.open(&part).map_err(|e| e.to_string())?, &seal(&key, &plain)?).map_err(|e| e.to_string())?;
    std::fs::rename(&part, &file).map_err(|e| e.to_string())?;
    *CACHE.lock().unwrap() = None;
    Ok(())
}

fn forget(streaming: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(streaming.join(FILE));
    *CACHE.lock().unwrap() = Some((streaming.join(FILE), None));
    crate::keys::forget(SERVICE)
}

// ---- the sign-in window ----

/// Where `account_sign_in` waits for the window: the account, or why not.
#[derive(Default)]
pub struct Pending(Mutex<Option<tauri::async_runtime::Sender<Result<Info, String>>>>);

#[cfg(desktop)]
fn rows_of(cookies: Vec<tauri::webview::Cookie<'static>>, host: &str) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    for c in cookies {
        let row = Row {
            domain: c.domain().map(str::to_owned).unwrap_or_else(|| host.to_owned()),
            path: c.path().unwrap_or("/").to_owned(),
            secure: c.secure().unwrap_or(false),
            http_only: c.http_only().unwrap_or(false),
            expires: c.expires().and_then(|e| e.datetime()).map_or(0, |d| d.unix_timestamp()),
            name: c.name().to_owned(),
            value: c.value().to_owned(),
        };
        if !rows.iter().any(|r| r.domain == row.domain && r.path == row.path && r.name == row.name) {
            rows.push(row);
        }
    }
    rows
}

/// After a page in the window has loaded: if YouTube's session cookies are there, keep them and close the window.
#[cfg(desktop)]
async fn capture(app: AppHandle, w: tauri::WebviewWindow) {
    use tauri::Manager;
    let mut cookies = Vec::new();
    for host in ["music.youtube.com", "www.youtube.com", "accounts.google.com"] {
        if let Ok(u) = url::Url::parse(&format!("https://{host}/")) {
            cookies.extend(rows_of(w.cookies_for_url(u).unwrap_or_default(), host));
        }
    }
    let mut rows: Vec<Row> = Vec::new();
    for r in cookies {
        if !rows.iter().any(|x| x.domain == r.domain && x.path == r.path && x.name == r.name) {
            rows.push(r);
        }
    }
    // one capture finishes the sign-in (several pages load on the way to YT Music)
    if !signed_in(&rows) {
        return;
    }
    let Some(tx) = app.state::<Pending>().0.lock().unwrap().take() else { return };
    let done = async {
        let streaming = crate::ytdlp::streaming_dir(&app, "")?;
        let signed = Signed { cookies: rows.clone(), info: Info::default() };
        let info = crate::ytmusic::account_info(&signed).await.unwrap_or_else(|e| {
            log::warn!("YouTube account name: {e}");
            Info { name: "your Google account".into(), ..Info::default() }
        });
        save(&streaming, rows, info.clone())?;
        log::info!("signed in to YouTube in Qistara's window");
        Ok(info)
    };
    let result: Result<Info, String> = done.await;
    let _ = tx.try_send(result);
    let _ = w.close();
}

/// Opens Google's sign-in in a small window and waits for it: the account, or why not (the window was closed).
#[cfg(desktop)]
#[tauri::command]
pub async fn account_sign_in(app: AppHandle, pending: tauri::State<'_, Pending>) -> Result<Info, String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Err("busy".into());
    }
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    *pending.0.lock().unwrap() = Some(tx);
    let start = url::Url::parse(START).map_err(|e| e.to_string())?;
    let on_load = app.clone();
    let mut b = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::External(start))
        .title("Sign in to YouTube")
        .inner_size(460.0, 680.0)
        .min_inner_size(380.0, 520.0)
        .center()
        // a private window: nothing the sign-in page stores outlives it
        .incognito(true)
        .on_navigation(|u| {
            let ok = allowed(u);
            if !ok {
                log::info!("sign-in window: kept off {}", u.host_str().unwrap_or("?"));
            }
            ok
        })
        .on_page_load(move |w, p| {
            if matches!(p.event(), tauri::webview::PageLoadEvent::Finished) && p.url().host_str().is_some_and(|h| h.ends_with("youtube.com")) {
                tauri::async_runtime::spawn(capture(on_load.clone(), w));
            }
        });
    // Google turns away browsers it doesn't recognise; WebKit on a Mac names itself as Safari does
    if cfg!(target_os = "macos") {
        b = b.user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.3 Safari/605.1.15");
    }
    let w = b.build().map_err(|e| e.to_string())?;
    let closed = app.clone();
    w.on_window_event(move |e| {
        if let tauri::WindowEvent::Destroyed = e
            && let Some(tx) = closed.state::<Pending>().0.lock().unwrap().take()
        {
            let _ = tx.try_send(Err("closed".into()));
        }
    });
    let result = rx.recv().await.unwrap_or_else(|| Err("closed".into()));
    if result.is_ok() {
        // songs that needed a sign-in are asked for again
        app.state::<crate::ytstream::Engine>().forget_all();
    }
    result
}

#[cfg(mobile)]
#[tauri::command]
pub async fn account_sign_in() -> Result<Info, String> {
    Err("Signing in to YouTube isn't available on phones yet.".into())
}

/// Who is signed in (Providers, the YouTube and YT Music pages), or nothing.
#[tauri::command]
pub async fn account_status(app: AppHandle) -> Option<Info> {
    signed(&app).map(|s| s.info.clone())
}

/// Signs out: the kept cookies, their key and mpv's hook copy go.
#[tauri::command]
pub async fn account_sign_out(app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let _ = crate::ytdlp::hook_cookies(&app, None);
    app.state::<crate::ytstream::Engine>().forget_all();
    forget(&crate::ytdlp::streaming_dir(&app, "")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(domain: &str, name: &str, value: &str) -> Row {
        Row { domain: domain.into(), path: "/".into(), secure: true, http_only: false, expires: 1_900_000_000, name: name.into(), value: value.into() }
    }

    #[test]
    fn hashes_sapisid_the_way_youtube_checks_it() {
        // sha1("1700000000 abcDEF/ghi https://music.youtube.com")
        assert_eq!(sapisid_hash("abcDEF/ghi", 1_700_000_000, "https://music.youtube.com"), "SAPISIDHASH 1700000000_f05a3b9d7e73d0897df18cf793218762a40e8803");
    }

    #[test]
    fn knows_a_finished_sign_in_by_youtubes_session_cookies() {
        assert!(!signed_in(&[row(".youtube.com", "PREF", "x"), row(".youtube.com", "VISITOR_INFO1_LIVE", "y")]));
        // Google's own cookies alone: still on the sign-in page
        assert!(!signed_in(&[row(".google.com", "SAPISID", "a"), row(".google.com", "SID", "b")]));
        assert!(signed_in(&[row(".youtube.com", "SAPISID", "a"), row(".youtube.com", "SID", "b")]));
        assert!(signed_in(&[row(".youtube.com", "__Secure-3PAPISID", "a"), row(".youtube.com", "__Secure-3PSID", "b")]));
    }

    #[test]
    fn writes_a_cookies_txt_yt_dlp_reads() {
        let mut http = row(".youtube.com", "SID", "s1");
        http.http_only = true;
        let host_only = Row { domain: "music.youtube.com".into(), expires: 0, ..row("", "PREF", "f=1") };
        let txt = netscape(&[http, host_only, row(".example.com", "x", "y")]);
        let lines: Vec<&str> = txt.lines().collect();
        assert_eq!(lines[0], "# Netscape HTTP Cookie File");
        assert!(lines.contains(&"#HttpOnly_.youtube.com\tTRUE\t/\tTRUE\t1900000000\tSID\ts1"));
        assert!(lines.contains(&"music.youtube.com\tFALSE\t/\tTRUE\t0\tPREF\tf=1"));
        assert!(!txt.contains("example.com"), "only YouTube's and Google's cookies");
    }

    #[test]
    fn signs_youtube_requests_with_youtube_cookies_only() {
        let s = Signed { cookies: vec![row(".youtube.com", "SAPISID", "abc"), row(".youtube.com", "SID", "s"), row(".google.com", "NID", "n")], info: Info::default() };
        assert_eq!(s.header(), "SAPISID=abc; SID=s");
        assert!(s.authorization("https://music.youtube.com").unwrap().starts_with("SAPISIDHASH "));
        assert!(Signed { cookies: vec![row(".youtube.com", "SID", "s")], info: Info::default() }.authorization("https://music.youtube.com").is_none());
    }

    #[test]
    fn keeps_the_sign_in_window_on_google_and_youtube() {
        let ok = |u: &str| allowed(&url::Url::parse(u).unwrap());
        for u in ["https://accounts.google.com/v3/signin", "https://music.youtube.com/", "https://www.youtube.com/signin", "https://ssl.gstatic.com/x", "https://content-autofill.googleapis.com/x", "about:blank"] {
            assert!(ok(u), "{u}");
        }
        for u in ["http://accounts.google.com/", "https://google.com.evil.example/", "https://evil-youtube.com/", "https://googleapis.com.evil.example/", "file:///etc/passwd", "javascript:alert(1)", "tauri://localhost/"] {
            assert!(!ok(u), "{u}");
        }
    }

    #[test]
    fn keeps_the_account_encrypted_and_refuses_a_wrong_key_or_a_changed_file() {
        let key = [7u8; 32];
        let sealed = seal(&key, b"cookies").unwrap();
        assert!(!sealed.windows(7).any(|w| w == b"cookies"));
        assert_eq!(open(&key, &sealed).as_deref(), Some(&b"cookies"[..]));
        assert_eq!(open(&[8u8; 32], &sealed), None);
        let mut changed = sealed.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert_eq!(open(&key, &changed), None);
        assert_eq!(open(&key, b"short"), None);
    }

    #[test]
    fn reads_the_accounts_name_and_picture() {
        let v = json!({ "actions": [{ "openPopupAction": { "popup": { "multiPageMenuRenderer": { "header": { "activeAccountHeaderRenderer": {
            "accountName": { "runs": [{ "text": "Sam Example" }] },
            "accountPhoto": { "thumbnails": [{ "url": "https://yt3.ggpht.com/a=s48" }, { "url": "https://yt3.ggpht.com/a=s88" }] },
            "channelHandle": { "runs": [{ "text": "@sam" }] } } } } } } }] });
        assert_eq!(parse_account_menu(&v), Some(Info { name: "Sam Example".into(), handle: Some("@sam".into()), photo: Some("https://yt3.ggpht.com/a=s88".into()) }));
        let elsewhere = json!({ "activeAccountHeaderRenderer": { "accountName": { "runs": [{ "text": "x" }] }, "accountPhoto": { "thumbnails": [{ "url": "https://evil.example/p.jpg" }] } } });
        assert_eq!(parse_account_menu(&elsewhere).unwrap().photo, None);
        assert_eq!(parse_account_menu(&json!({})), None);
    }
}

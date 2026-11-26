// Fast YouTube playback, the way Sonora does it (written for Qistara, none of its code): ask YouTube's player API
// for a song's sound from a guest client whose stream addresses need no deciphering, check the address with a
// two-byte read, keep it until shortly before it expires, and serve mpv from a loopback server that fetches it in
// ranges and asks again when YouTube refuses an address mid-song. The next songs and the one under the pointer are
// looked up and their first megabyte fetched ahead, so they start at once. yt-dlp (the user's sign-in included) is
// the fallback when the guest clients can't open a song.
//
// Only the video id and the client's description go to YouTube: no cookies, and the visitor id YouTube hands out
// is kept in memory for this run only. The server listens on 127.0.0.1, answers only Host: 127.0.0.1:<port> and a
// path with this run's random token, and fetches only https addresses on YouTube's video host.
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::async_runtime::block_on as rt;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_http::reqwest;

const PLAYER: &str = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
/// The guest clients asked, in order: their stream addresses come signed, with nothing to decipher.
const CLIENTS: [Client; 2] = [
    Client {
        name: "ANDROID_VR",
        id: "28",
        version: "1.65.10",
        user_agent: "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
        device: &[("deviceMake", "Oculus"), ("deviceModel", "Quest 3"), ("osName", "Android"), ("osVersion", "12L"), ("androidSdkVersion", "32")],
    },
    Client {
        name: "IOS",
        id: "5",
        version: "20.10.4",
        user_agent: "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)",
        device: &[("deviceMake", "Apple"), ("deviceModel", "iPhone16,2"), ("osName", "iPhone"), ("osVersion", "18.3.2.22D82")],
    },
];
/// How much is fetched per request from YouTube's video host (it slows down long single reads).
const CHUNK: u64 = 1 << 20;
/// What is fetched ahead for a song that will probably play next.
const AHEAD: usize = 1 << 20;
/// Streams kept; their read-ahead bytes stay within MAX_AHEAD in all.
const KEEP: usize = 32;
const MAX_AHEAD: usize = 16 << 20;
/// An address is used until this long before YouTube says it expires.
const MARGIN: u64 = 30 * 60;
/// Where an address is tried before it's used: deep in the file, where some clients' addresses stop working
/// after the first megabyte or so (and where a seek goes).
const PROBE_AT: u64 = 3 << 20;

struct Client {
    name: &'static str,
    id: &'static str,
    version: &'static str,
    user_agent: &'static str,
    device: &'static [(&'static str, &'static str)],
}

/// A song's sound: where to fetch it and how.
#[derive(Clone, Debug, PartialEq)]
pub struct Stream {
    url: String,
    user_agent: String,
    mime: &'static str,
    len: Option<u64>,
    /// unix seconds; 0 when the address doesn't say
    expires: u64,
    /// which engine found it, for the log and Settings › Streaming › Test
    engine: &'static str,
    /// the guest client it came from (an index into CLIENTS); None from yt-dlp
    client: Option<usize>,
    /// YouTube's format number: a renewed address continues a song only when it's the same format and length
    itag: Option<u32>,
    ahead: Option<Arc<Vec<u8>>>,
}

#[derive(Default)]
pub struct Engine {
    /// (port, token) once the server runs
    server: OnceLock<(u16, String)>,
    streams: Mutex<HashMap<String, Stream>>,
    order: Mutex<VecDeque<String>>,
    visitor: Mutex<Option<String>>,
    /// the client that worked last
    client: AtomicUsize,
    warming: Mutex<HashSet<String>>,
    /// the sign-in yt-dlp uses and whether Premium quality is wanted: what the page last asked with
    wants: Mutex<(Option<crate::ytdlp::Cookies>, bool)>,
    /// one lookup per song at a time
    busy: Mutex<HashMap<String, Arc<tauri::async_runtime::Mutex<()>>>>,
    /// songs no engine could open lately, and why: asked again at once, they fail at once (mpv then plays them
    /// through its hook without waiting for the same lookups twice)
    failed: Mutex<HashMap<String, (Instant, String)>>,
    /// guest clients whose addresses YouTube stopped serving mid-song this run: not asked again
    distrusted: Mutex<HashSet<usize>>,
}

/// How long a song that couldn't be opened isn't looked up again.
const FAILED_FOR: Duration = Duration::from_secs(30);

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The `expire` a stream address carries (unix seconds).
fn expires(url: &str) -> u64 {
    url::Url::parse(url).ok().and_then(|u| u.query_pairs().find(|(k, _)| k == "expire").and_then(|(_, v)| v.parse().ok())).unwrap_or(0)
}

/// The `itag` (format number) a stream address carries.
fn itag(url: &str) -> Option<u32> {
    url::Url::parse(url).ok().and_then(|u| u.query_pairs().find(|(k, _)| k == "itag").and_then(|(_, v)| v.parse().ok()))
}

/// The same bytes: the same format and length (YouTube serves a format identically from any address).
fn same_file(a: &Stream, b: &Stream) -> bool {
    a.len.is_some() && a.len == b.len && a.itag == b.itag
}

/// The first byte a `Content-Range: bytes a-b/total` reply starts at.
pub fn range_start(content_range: &str) -> Option<u64> {
    content_range.trim().strip_prefix("bytes ")?.split('-').next()?.trim().parse().ok()
}

/// A reply is the bytes asked for from `at`: a 206 starting there, or (only from the start) the whole file.
fn fits(status: u16, content_range: Option<&str>, at: u64) -> bool {
    match status {
        206 => content_range.and_then(range_start) == Some(at),
        200 => at == 0,
        _ => false,
    }
}

fn fresh(s: &Stream) -> bool {
    s.expires == 0 || s.expires > now() + MARGIN
}

/// Only https addresses on YouTube's video host are ever fetched.
pub fn youtube_media(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some_and(|h| h.ends_with(".googlevideo.com")))
}

fn valid_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn body(c: &Client, id: &str, visitor: Option<&str>) -> Value {
    let mut client = json!({ "clientName": c.name, "clientVersion": c.version, "hl": "en", "gl": "US" });
    for (k, v) in c.device {
        client[*k] = if *k == "androidSdkVersion" { json!(v.parse::<u32>().unwrap_or(0)) } else { json!(v) };
    }
    if let Some(v) = visitor {
        client["visitorData"] = json!(v);
    }
    json!({ "context": { "client": client }, "videoId": id, "contentCheckOk": true, "racyCheckOk": true,
        "playbackContext": { "contentPlaybackContext": { "html5Preference": "HTML5_PREF_WANTS" } } })
}

/// The sound to play from a player reply: the best audio format whose address is plain (not ciphered), the
/// original language when a song has dubbed tracks, without dynamic-range compression. Err says why not.
pub fn pick_audio(reply: &Value) -> Result<(String, &'static str, Option<u64>), String> {
    let status = reply["playabilityStatus"]["status"].as_str().unwrap_or("");
    if status != "OK" {
        let why = reply["playabilityStatus"]["reason"].as_str().unwrap_or(status);
        return Err(format!("YouTube: {why}"));
    }
    let formats = reply["streamingData"]["adaptiveFormats"].as_array().ok_or("YouTube sent no streams")?;
    let audio = |f: &&Value| {
        f["mimeType"].as_str().is_some_and(|m| m.starts_with("audio/"))
            && f["url"].as_str().is_some_and(youtube_media)
            && !f["isDrc"].as_bool().unwrap_or(false)
            && f["audioTrack"]["audioIsDefault"].as_bool().unwrap_or(true)
    };
    let best = formats.iter().filter(audio).max_by_key(|f| f["bitrate"].as_u64().unwrap_or(0)).ok_or("no plain audio stream (this client would need deciphering)")?;
    let mime = if best["mimeType"].as_str().unwrap_or("").starts_with("audio/mp4") { "audio/mp4" } else { "audio/webm" };
    Ok((best["url"].as_str().unwrap_or_default().to_string(), mime, best["contentLength"].as_str().and_then(|n| n.parse().ok())))
}

/// The visitor id in YouTube's sw.js_data reply (a base64 "Cg…" string), for a client YouTube wants one from.
pub fn visitor_in(text: &str) -> Option<String> {
    text.split('"').find(|t| t.starts_with("Cg") && (20..=80).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_%=".contains(&b))).map(str::to_owned)
}

/// One client for the whole run, so reads from YouTube's video host reuse their connection. YouTube's video
/// host often sends a range on to another of its servers: redirects are followed there only (https, at most 5).
fn http() -> &'static reqwest::Client {
    static HTTP: OnceLock<reqwest::Client> = OnceLock::new();
    HTTP.get_or_init(|| {
        let policy = reqwest::redirect::Policy::custom(|a| {
            let to = a.url().as_str();
            // (the tests stand in for YouTube on this machine)
            if a.previous().len() < 5 && (youtube_media(to) || (cfg!(test) && to.starts_with("http://127.0.0.1:"))) { a.follow() } else { a.stop() }
        });
        reqwest::Client::builder()
            .redirect(policy)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default()
    })
}

impl Engine {
    async fn visitor(&self) -> Option<String> {
        if let Some(v) = self.visitor.lock().unwrap().clone() {
            return Some(v);
        }
        let r = http().get("https://www.youtube.com/sw.js_data").header("User-Agent", CLIENTS[0].user_agent).send().await.ok()?;
        let v = visitor_in(&r.text().await.ok()?)?;
        *self.visitor.lock().unwrap() = Some(v.clone());
        Some(v)
    }

    async fn ask(&self, c: &Client, id: &str, visitor: Option<&str>) -> Result<Value, String> {
        let mut req = http()
            .post(PLAYER)
            .timeout(Duration::from_secs(10))
            .header("Content-Type", "application/json")
            .header("User-Agent", c.user_agent)
            .header("X-YouTube-Client-Name", c.id)
            .header("X-YouTube-Client-Version", c.version)
            .header("Origin", "https://www.youtube.com");
        if let Some(v) = visitor {
            req = req.header("X-Goog-Visitor-Id", v);
        }
        let r = req.body(body(c, id, visitor).to_string()).send().await.map_err(|e| format!("network: {}", e.without_url()))?;
        if !r.status().is_success() {
            return Err(format!("YouTube answered {}", r.status().as_u16()));
        }
        serde_json::from_str(&r.text().await.map_err(|e| e.to_string())?).map_err(|_| "unexpected reply".to_string())
    }

    /// One guest client: its reply, asked again with a visitor id when YouTube wants one.
    async fn guest(&self, i: usize, id: &str) -> Result<Stream, String> {
        let c = &CLIENTS[i];
        let known = self.visitor.lock().unwrap().clone();
        let mut reply = self.ask(c, id, known.as_deref()).await?;
        if pick_audio(&reply).is_err() && known.is_none()
            && let Some(v) = self.visitor().await {
                reply = self.ask(c, id, Some(&v)).await?;
            }
        if let Some(v) = reply["responseContext"]["visitorData"].as_str().and_then(|v| visitor_in(&format!("\"{v}\""))) {
            self.visitor.lock().unwrap().get_or_insert(v);
        }
        let (url, mime, len) = pick_audio(&reply)?;
        Ok(Stream { expires: expires(&url), itag: itag(&url), url, user_agent: c.user_agent.into(), mime, len, engine: c.name, client: Some(i), ahead: None })
    }

    /// A song's sound: the guest clients (the one that worked last first), each address checked; then yt-dlp with
    /// the user's sign-in. With Premium quality chosen and a sign-in, yt-dlp is asked first.
    async fn resolve(&self, app: &AppHandle, id: &str) -> Result<Stream, String> {
        let started = Instant::now();
        let (cookies, premium) = self.wants.lock().unwrap().clone();
        let premium = premium && cookies.is_some();
        let mut why = Vec::new();
        if premium {
            match with_ytdlp(app, id, &cookies).await {
                Ok(s) => {
                    log::info!("stream {id}: yt-dlp (Premium quality) in {} ms", started.elapsed().as_millis());
                    return Ok(s);
                }
                Err(e) => why.push(format!("yt-dlp: {e}")),
            }
        }
        let first = self.client.load(SeqCst) % CLIENTS.len();
        let distrusted = self.distrusted.lock().unwrap().clone();
        for i in (0..CLIENTS.len()).map(|n| (first + n) % CLIENTS.len()).filter(|i| !distrusted.contains(i)) {
            match self.guest(i, id).await {
                Ok(s) if probe(&s).await => {
                    self.client.store(i, SeqCst);
                    log::info!("stream {id}: {} in {} ms", s.engine, started.elapsed().as_millis());
                    return Ok(s);
                }
                Ok(_) => why.push(format!("{}: address refused deep in the file", CLIENTS[i].name)),
                Err(e) => why.push(format!("{}: {e}", CLIENTS[i].name)),
            }
        }
        if !premium {
            match with_ytdlp(app, id, &cookies).await {
                Ok(s) => {
                    log::info!("stream {id}: yt-dlp in {} ms ({})", started.elapsed().as_millis(), why.join("; "));
                    return Ok(s);
                }
                Err(e) => why.push(format!("yt-dlp: {e}")),
            }
        }
        let why = why.join("; ");
        log::warn!("stream {id} failed: {why}");
        Err(why)
    }

    /// The sign-in and quality the page asks with; a change drops what was found with the old ones.
    fn ask_with(&self, cookies: Option<crate::ytdlp::Cookies>, premium: bool) {
        let mut wants = self.wants.lock().unwrap();
        if *wants != (cookies.clone(), premium) {
            *wants = (cookies, premium);
            drop(wants);
            self.forget_all();
        }
    }

    /// YouTube stopped serving this client's address mid-song even after asking again: its addresses aren't used
    /// for the rest of the run (the other client and yt-dlp are).
    fn distrust(&self, s: &Stream) {
        if let Some(i) = s.client
            && self.distrusted.lock().unwrap().insert(i)
        {
            log::warn!("stream: {} addresses stop mid-song; using the others for this run", CLIENTS[i].name);
        }
    }

    /// Everything found so far, and every failure, is forgotten (a sign-in or a quality change).
    pub fn forget_all(&self) {
        self.streams.lock().unwrap().clear();
        self.order.lock().unwrap().clear();
        self.failed.lock().unwrap().clear();
    }

    fn cached(&self, id: &str) -> Option<Stream> {
        self.streams.lock().unwrap().get(id).filter(|s| fresh(s)).cloned()
    }

    fn keep(&self, id: &str, s: Stream) {
        let mut streams = self.streams.lock().unwrap();
        let mut order = self.order.lock().unwrap();
        order.retain(|x| x != id);
        order.push_back(id.to_string());
        streams.insert(id.to_string(), s);
        while order.len() > KEEP {
            if let Some(old) = order.pop_front() {
                streams.remove(&old);
            }
        }
        // the read-ahead of the oldest streams goes first
        let mut total: usize = streams.values().filter_map(|s| s.ahead.as_ref()).map(|a| a.len()).sum();
        for old in order.iter() {
            if total <= MAX_AHEAD {
                break;
            }
            if let Some(a) = streams.get_mut(old).and_then(|s| s.ahead.take()) {
                total -= a.len();
            }
        }
    }

    fn forget(&self, id: &str) {
        self.streams.lock().unwrap().remove(id);
        self.failed.lock().unwrap().remove(id);
    }

    /// The stream for a song, from what's kept or looked up now. A song is looked up once at a time: a second
    /// ask (mpv opening the song while it's being fetched ahead) waits for the first.
    async fn stream(&self, app: &AppHandle, id: &str) -> Result<Stream, String> {
        if let Some(s) = self.cached(id) {
            return Ok(s);
        }
        let lock = {
            let mut busy = self.busy.lock().unwrap();
            if busy.len() > 256 {
                busy.retain(|_, l| Arc::strong_count(l) > 1);
            }
            busy.entry(id.to_string()).or_default().clone()
        };
        let _one = lock.lock().await;
        if let Some(s) = self.cached(id) {
            return Ok(s);
        }
        if let Some((_, why)) = self.failed.lock().unwrap().get(id).filter(|(at, _)| at.elapsed() < FAILED_FOR) {
            return Err(why.clone());
        }
        match self.resolve(app, id).await {
            Ok(s) => {
                self.keep(id, s.clone());
                Ok(s)
            }
            Err(e) => {
                let mut failed = self.failed.lock().unwrap();
                failed.retain(|_, (at, _)| at.elapsed() < FAILED_FOR);
                failed.insert(id.to_string(), (Instant::now(), e.clone()));
                Err(e)
            }
        }
    }

    /// Looks a song up and fetches its first megabyte, unless that's already done.
    async fn warm(&self, app: &AppHandle, id: &str) {
        if self.cached(id).is_some_and(|s| s.ahead.is_some()) || !self.warming.lock().unwrap().insert(id.to_string()) {
            return;
        }
        if let Ok(s) = self.stream(app, id).await
            && s.ahead.is_none() {
                let end = s.len.map_or(AHEAD as u64, |n| n.min(AHEAD as u64)).saturating_sub(1);
                if let Ok(Ok(bytes)) = fetch(&s, 0, end).await
                    && let Some(k) = self.streams.lock().unwrap().get_mut(id).filter(|k| k.url == s.url) {
                        k.ahead = Some(Arc::new(bytes));
                    }
            }
        self.warming.lock().unwrap().remove(id);
    }

    fn url(&self, id: &str) -> Option<String> {
        self.server.get().map(|(port, token)| format!("http://127.0.0.1:{port}/{token}/yt/{id}"))
    }
}

/// The stream yt-dlp finds, with the user's sign-in.
async fn with_ytdlp(app: &AppHandle, id: &str, cookies: &Option<crate::ytdlp::Cookies>) -> Result<Stream, String> {
    let a = crate::ytdlp::audio_stream(app, id, cookies).await?;
    let user_agent = a.user_agent.unwrap_or_else(|| "Mozilla/5.0".into());
    Ok(Stream { expires: expires(&a.url), itag: itag(&a.url), url: a.url, user_agent, mime: a.mime, len: a.len, engine: "yt-dlp", client: None, ahead: None })
}

fn get(s: &Stream, start: u64, end: u64) -> Result<reqwest::RequestBuilder, String> {
    // (the tests stand in for YouTube on this machine)
    if !youtube_media(&s.url) && !(cfg!(test) && s.url.starts_with("http://127.0.0.1:")) {
        return Err("not YouTube's video host".into());
    }
    Ok(http().get(&s.url).header("User-Agent", &s.user_agent).header("Range", format!("bytes={start}-{end}")))
}

/// A two-byte read deep in the file (where a seek goes, and where some clients' addresses stop working): YouTube
/// serves this address to us all the way through.
async fn probe(s: &Stream) -> bool {
    let at = s.len.map_or(0, |n| n.saturating_sub(2).min(PROBE_AT));
    matches!(fetch(s, at, at + 1).await, Ok(Ok(_)))
}

/// Bytes `start..=end` of a stream, or the status YouTube answered with instead (a refusal, a redirect it wasn't
/// allowed to follow, or a reply that isn't that range).
async fn fetch(s: &Stream, start: u64, end: u64) -> Result<Result<Vec<u8>, u16>, String> {
    let r = get(s, start, end)?.timeout(Duration::from_secs(20)).send().await.map_err(|e| e.to_string())?;
    let status = r.status().as_u16();
    if !fits(status, r.headers().get("content-range").and_then(|v| v.to_str().ok()), start) {
        return Ok(Err(status));
    }
    Ok(Ok(r.bytes().await.map_err(|e| e.to_string())?.to_vec()))
}

/// The total length from a `Content-Range: bytes a-b/total` reply.
async fn total_of(s: &Stream) -> Option<u64> {
    let r = get(s, 0, 0).ok()?.send().await.ok()?;
    r.headers().get("content-range")?.to_str().ok()?.rsplit('/').next()?.parse().ok()
}

/// How a piece of the stream went: the bytes passed on, YouTube's refusal, or a dropped connection.
enum Piece {
    Sent(u64),
    Refused(u16),
    Dropped(String),
}

/// Passes bytes `at..=to` from YouTube on to mpv as they arrive. Err: mpv went away (a seek, or the song ended).
async fn relay(stream: &Stream, mut out: &TcpStream, at: u64, to: u64) -> Result<Piece, String> {
    let mut r = match get(stream, at, to)?.send().await {
        Ok(r) => r,
        Err(e) => return Ok(Piece::Dropped(e.to_string())),
    };
    let status = r.status().as_u16();
    // never pass on bytes from anywhere but `at` (a whole file sent for a range would play as noise)
    if !fits(status, r.headers().get("content-range").and_then(|v| v.to_str().ok()), at) {
        return Ok(Piece::Refused(status));
    }
    let mut sent = 0u64;
    let want = to - at + 1;
    loop {
        match r.chunk().await {
            Ok(Some(c)) => {
                let c = &c[..c.len().min((want - sent) as usize)];
                out.write_all(c).map_err(|e| e.to_string())?;
                sent += c.len() as u64;
                if sent >= want {
                    return Ok(Piece::Sent(sent));
                }
            }
            Ok(None) if sent > 0 => return Ok(Piece::Sent(sent)),
            Ok(None) => return Ok(Piece::Dropped("empty reply".into())),
            Err(e) if sent > 0 => {
                log::debug!("stream read cut short: {e}");
                return Ok(Piece::Sent(sent));
            }
            Err(e) => return Ok(Piece::Dropped(e.to_string())),
        }
    }
}

// ---- the loopback server mpv plays from ----

/// The video id in a request for /<token>/yt/<id> to 127.0.0.1:<port> itself: (id, HEAD?, Range header).
pub fn parse_stream_request(head: &str, port: u16, token: &str) -> Option<(String, bool, Option<String>)> {
    let mut lines = head.lines();
    let mut first = lines.next()?.split(' ');
    let method = first.next()?;
    let id = first.next()?.strip_prefix('/')?.strip_prefix(token)?.strip_prefix("/yt/")?;
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    let get = |k: &str| headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.clone());
    (valid_id(id) && matches!(method, "GET" | "HEAD") && get("host")? == format!("127.0.0.1:{port}")).then(|| (id.to_string(), method == "HEAD", get("range")))
}

fn start_server(app: &AppHandle, engine: &Engine) -> Result<(), String> {
    if engine.server.get().is_some() {
        return Ok(());
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let mut t = [0u8; 16];
    getrandom::fill(&mut t).map_err(|e| e.to_string())?;
    let token: String = t.iter().map(|b| format!("{b:02x}")).collect();
    if engine.server.set((port, token.clone())).is_err() {
        return Ok(()); // another call started it first; this listener just closes
    }
    let app = app.clone();
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let (app, token) = (app.clone(), token.clone());
            std::thread::spawn(move || {
                let _ = s.set_read_timeout(Some(Duration::from_secs(20)));
                let mut buf = Vec::new();
                if let Some((id, head, range)) = crate::session::read_head(&s, &mut buf).and_then(|h| parse_stream_request(&h, port, &token))
                    && let Err(e) = serve(&app, &s, &id, head, range.as_deref()) {
                        log::debug!("stream {id}: {e}");
                    }
                let _ = s.shutdown(std::net::Shutdown::Both);
            });
        }
    });
    Ok(())
}

fn serve(app: &AppHandle, s: &TcpStream, id: &str, head: bool, range: Option<&str>) -> Result<(), String> {
    let engine = app.state::<Engine>();
    let started = Instant::now();
    let stream = match rt(engine.stream(app, id)) {
        Ok(st) => st,
        Err(e) => return fail(s, e),
    };
    if range.is_none_or(|r| r.starts_with("bytes=0-")) {
        log::info!("stream {id}: ready for mpv after {} ms{}", started.elapsed().as_millis(), if stream.ahead.is_some() { " (fetched ahead)" } else { "" });
    }
    // a refused address: first a fresh one from the same client (it may just have expired); refused again, that
    // client is set aside and the others (then yt-dlp) are asked
    let mut asked = 0;
    respond(s, id, stream, head, range, |old| {
        asked += 1;
        if asked > 1 {
            engine.distrust(old);
        }
        engine.forget(id);
        rt(engine.stream(app, id))
    })
}

fn fail(mut s: &TcpStream, why: String) -> Result<(), String> {
    let _ = s.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    Err(why)
}

/// Answers mpv's request for a stream: the range asked for (all of it without one), then the bytes.
fn respond(mut s: &TcpStream, id: &str, stream: Stream, head: bool, range: Option<&str>, renew: impl FnMut(&Stream) -> Result<Stream, String>) -> Result<(), String> {
    let total = match stream.len.or_else(|| rt(total_of(&stream))) {
        Some(n) if n > 0 => n,
        _ => return fail(s, "YouTube didn't say how long the stream is".into()),
    };
    let (start, end) = match crate::session::parse_range(range, total) {
        Ok(r) => r.unwrap_or((0, total - 1)),
        Err(()) => {
            return s.write_all(format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).map_err(|e| e.to_string());
        }
    };
    let status = if range.is_some() { format!("206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}") } else { "200 OK".into() };
    s.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n", stream.mime, end - start + 1).as_bytes())
        .map_err(|e| e.to_string())?;
    if head {
        return Ok(());
    }
    pump(s, id, stream, start, end, renew)
}

/// Sends bytes `at..=end` of a stream to mpv: what was fetched ahead at once, the rest from YouTube a megabyte per
/// request. A refused address is asked for again (`renew`, three times at most) and the song carries on only from
/// the same file; a dropped connection is tried again.
fn pump(out: &TcpStream, id: &str, mut stream: Stream, mut at: u64, end: u64, mut renew: impl FnMut(&Stream) -> Result<Stream, String>) -> Result<(), String> {
    if let Some(ahead) = stream.ahead.clone().filter(|a| (at as usize) < a.len()) {
        let to = (end as usize + 1).min(ahead.len());
        let mut o = out;
        o.write_all(&ahead[at as usize..to]).map_err(|e| e.to_string())?;
        at = to as u64;
    }
    let (mut renewed, mut retried) = (0, 0);
    while at <= end {
        match rt(relay(&stream, out, at, (at + CHUNK - 1).min(end)))? {
            Piece::Sent(n) => {
                at += n;
                retried = 0;
            }
            // YouTube refused the address (expired, or no longer good for us): ask for a new one
            Piece::Refused(status) if renewed < 3 => {
                renewed += 1;
                log::info!("stream {id}: {} address refused ({status}) at byte {at}, asking again", stream.engine);
                let next = renew(&stream)?;
                if !same_file(&stream, &next) {
                    return Err(format!("the new address is another file ({} {:?}/{:?}), so the song can't carry on from byte {at}", next.engine, next.itag, next.len));
                }
                stream = next;
            }
            Piece::Refused(status) => return Err(format!("YouTube refused the stream ({status}) at byte {at}")),
            Piece::Dropped(e) if retried < 3 => {
                retried += 1;
                log::debug!("stream: {e}, trying again");
                std::thread::sleep(Duration::from_millis(300 * retried));
            }
            Piece::Dropped(e) => return Err(e),
        }
    }
    Ok(())
}

// ---- commands ----

/// The address mpv plays a YouTube song from (player.svelte.ts loadTrack). It answers at once: the song is looked
/// up now and the server waits for it when mpv asks, so a song lined up next costs nothing until it plays. A song
/// no engine can open fails in mpv, and the page plays it through mpv's ytdl hook instead (youtube.svelte.ts).
#[tauri::command]
pub fn ytstream_url(app: AppHandle, engine: State<'_, Engine>, id: String, cookies: Option<crate::ytdlp::Cookies>, premium: bool) -> Result<String, String> {
    if !valid_id(&id) {
        return Err("Not a YouTube video.".into());
    }
    start_server(&app, &engine)?;
    engine.ask_with(cookies, premium);
    let url = engine.url(&id).ok_or("The stream server isn't running.")?;
    tauri::async_runtime::spawn(async move { app.state::<Engine>().warm(&app, &id).await });
    Ok(url)
}

/// Songs that will probably play soon (the next in the queue, the one under the pointer): looked up and their
/// first megabyte fetched, two at a time, so they start at once.
#[tauri::command]
pub fn ytstream_warm(app: AppHandle, engine: State<'_, Engine>, ids: Vec<String>, cookies: Option<crate::ytdlp::Cookies>, premium: bool) {
    engine.ask_with(cookies, premium);
    for id in ids.into_iter().filter(|i| valid_id(i)).take(2) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { app.state::<Engine>().warm(&app, &id).await });
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    engine: String,
    resolve_ms: u64,
    first_byte_ms: u64,
    /// the middle of the song was served too, as a seek would ask for it
    seek_ok: bool,
}

/// Settings › Streaming › Test: looks a known song up afresh, reads its start (timing both) and its middle.
#[tauri::command]
pub async fn ytstream_test(app: AppHandle, engine: State<'_, Engine>, cookies: Option<crate::ytdlp::Cookies>, premium: bool) -> Result<TestResult, String> {
    const ID: &str = "dQw4w9WgXcQ"; // public, a few megabytes of sound, there for good
    engine.ask_with(cookies, premium);
    engine.forget(ID);
    let started = Instant::now();
    let s = engine.resolve(&app, ID).await?;
    let resolve_ms = started.elapsed().as_millis() as u64;
    let at = Instant::now();
    if let Err(status) = fetch(&s, 0, 64 * 1024 - 1).await? {
        return Err(format!("YouTube refused the stream ({status})"));
    }
    let first_byte_ms = at.elapsed().as_millis() as u64;
    let middle = s.len.unwrap_or(0) / 2;
    let seek_ok = matches!(fetch(&s, middle, middle + 64 * 1024 - 1).await, Ok(Ok(_)));
    log::info!("streaming test: {} found in {resolve_ms} ms, sound after {first_byte_ms} ms more, seeking {}", s.engine, if seek_ok { "works" } else { "refused" });
    Ok(TestResult { engine: s.engine.into(), resolve_ms, first_byte_ms, seek_ok })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(formats: Value) -> Value {
        json!({ "playabilityStatus": { "status": "OK" }, "streamingData": { "adaptiveFormats": formats } })
    }

    #[test]
    fn picks_the_best_plain_audio_stream() {
        let r = reply(json!([
            { "mimeType": "video/mp4; codecs=\"avc1\"", "bitrate": 900000, "url": "https://r1.googlevideo.com/v?expire=99" },
            { "mimeType": "audio/mp4; codecs=\"mp4a.40.2\"", "bitrate": 130000, "contentLength": "4000000", "url": "https://r1.googlevideo.com/a?expire=99&itag=140" },
            { "mimeType": "audio/webm; codecs=\"opus\"", "bitrate": 160000, "contentLength": "3500000", "url": "https://r1.googlevideo.com/a?expire=99&itag=251" },
            { "mimeType": "audio/webm; codecs=\"opus\"", "bitrate": 170000, "isDrc": true, "url": "https://r1.googlevideo.com/a?itag=251-drc" },
            { "mimeType": "audio/webm; codecs=\"opus\"", "bitrate": 999999, "signatureCipher": "s=x&url=https://r1.googlevideo.com/a" },
            { "mimeType": "audio/webm; codecs=\"opus\"", "bitrate": 500000, "url": "https://r1.googlevideo.com/dub", "audioTrack": { "audioIsDefault": false } },
        ]));
        let (url, mime, len) = pick_audio(&r).unwrap();
        assert_eq!((url.as_str(), mime, len), ("https://r1.googlevideo.com/a?expire=99&itag=251", "audio/webm", Some(3_500_000)));
        assert_eq!(expires(&url), 99);
    }

    #[test]
    fn says_why_when_there_is_nothing_to_play() {
        let blocked = json!({ "playabilityStatus": { "status": "LOGIN_REQUIRED", "reason": "Sign in to confirm your age" } });
        assert_eq!(pick_audio(&blocked).unwrap_err(), "YouTube: Sign in to confirm your age");
        // ciphered addresses only: this client can't be used, the next one is tried
        let ciphered = reply(json!([{ "mimeType": "audio/webm", "bitrate": 1, "signatureCipher": "s=x" }]));
        assert!(pick_audio(&ciphered).is_err());
        // an address off YouTube's video host is never taken
        let elsewhere = reply(json!([{ "mimeType": "audio/webm", "bitrate": 1, "url": "https://evil.example/a" }]));
        assert!(pick_audio(&elsewhere).is_err());
        assert!(pick_audio(&json!({})).is_err());
    }

    #[test]
    fn describes_the_client_and_the_song_only() {
        let b = body(&CLIENTS[0], "dQw4w9WgXcQ", Some("CgtWaXNpdG9y"));
        assert_eq!(b["videoId"], "dQw4w9WgXcQ");
        assert_eq!(b["context"]["client"]["clientName"], "ANDROID_VR");
        assert_eq!(b["context"]["client"]["androidSdkVersion"], 32);
        assert_eq!(b["context"]["client"]["visitorData"], "CgtWaXNpdG9y");
        assert!(body(&CLIENTS[1], "dQw4w9WgXcQ", None)["context"]["client"].get("visitorData").is_none());
    }

    #[test]
    fn finds_the_visitor_id_in_youtubes_reply() {
        let text = r#")]}'
[[["1",[[null,null,null,"x",null,null,null,null,null,null,null,null,null,"CgtSUVFfdWtHbW1kTSiAgqm7BjIKCgJVUxIEGgAgUg%3D%3D"]]]]]"#;
        assert_eq!(visitor_in(text).as_deref(), Some("CgtSUVFfdWtHbW1kTSiAgqm7BjIKCgJVUxIEGgAgUg%3D%3D"));
        assert_eq!(visitor_in(r#"["Cg", "Cg<script>alert(1)</script>xxxxxxxxxx"]"#), None);
    }

    #[test]
    fn serves_only_its_own_requests() {
        let ok = "GET /tok/yt/dQw4w9WgXcQ HTTP/1.1\r\nHost: 127.0.0.1:5000\r\nRange: bytes=0-";
        assert_eq!(parse_stream_request(ok, 5000, "tok"), Some(("dQw4w9WgXcQ".into(), false, Some("bytes=0-".into()))));
        for bad in [
            "GET /tok/yt/dQw4w9WgXcQ HTTP/1.1\r\nHost: evil.example:5000",
            "GET /other/yt/dQw4w9WgXcQ HTTP/1.1\r\nHost: 127.0.0.1:5000",
            "GET /tok/yt/../../etc HTTP/1.1\r\nHost: 127.0.0.1:5000",
            "POST /tok/yt/dQw4w9WgXcQ HTTP/1.1\r\nHost: 127.0.0.1:5000",
            "GET /tok/yt/dQw4w9WgXcQ HTTP/1.1",
        ] {
            assert_eq!(parse_stream_request(bad, 5000, "tok"), None, "{bad}");
        }
        assert!(youtube_media("https://rr3---sn-a.googlevideo.com/videoplayback"));
        assert!(!youtube_media("http://rr3---sn-a.googlevideo.com/x") && !youtube_media("https://googlevideo.com.evil.example/x"));
    }

    #[test]
    fn keeps_recent_streams_and_drops_read_ahead_first() {
        let e = Engine::default();
        let s = |exp: u64, ahead: usize| Stream { ahead: Some(Arc::new(vec![0; ahead])), expires: exp, ..fake("https://r.googlevideo.com/a", None) };
        for i in 0..40 {
            e.keep(&format!("id{i:09}"), s(0, AHEAD));
        }
        let streams = e.streams.lock().unwrap();
        assert_eq!(streams.len(), KEEP);
        assert!(streams.contains_key("id000000039") && !streams.contains_key("id000000000"));
        let total: usize = streams.values().filter_map(|s| s.ahead.as_ref()).map(|a| a.len()).sum();
        assert!(total <= MAX_AHEAD && streams["id000000039"].ahead.is_some());
        drop(streams);
        // an address about to expire is looked up again
        e.keep("soon_expire", s(now() + 60, 0));
        assert!(e.cached("soon_expire").is_none());
    }

    #[test]
    fn takes_only_the_range_it_asked_for() {
        assert_eq!(range_start("bytes 1048576-2097151/3500000"), Some(1_048_576));
        assert_eq!(range_start("bytes */3500000"), None);
        assert!(fits(206, Some("bytes 100-199/1000"), 100));
        assert!(!fits(206, Some("bytes 0-99/1000"), 100), "a range from elsewhere");
        assert!(!fits(206, None, 100));
        assert!(fits(200, None, 0) && !fits(200, None, 100), "the whole file only when the start was asked for");
        for refused in [302, 403, 404, 416, 500] {
            assert!(!fits(refused, Some("bytes 0-1/2"), 0));
        }
        assert_eq!(itag("https://r1.googlevideo.com/videoplayback?expire=9&itag=251&clen=4"), Some(251));
        let a = Stream { itag: Some(251), ..fake("https://r1.googlevideo.com/a", Some(100)) };
        assert!(same_file(&a, &Stream { url: "https://r9.googlevideo.com/b".into(), ..a.clone() }));
        assert!(!same_file(&a, &Stream { itag: Some(140), ..a.clone() }) && !same_file(&a, &Stream { len: Some(99), ..a.clone() }));
    }

    fn fake(url: &str, len: Option<u64>) -> Stream {
        Stream { url: url.into(), user_agent: "test".into(), mime: "audio/webm", len, expires: 0, engine: "test", client: None, itag: Some(251), ahead: None }
    }

    /// Stands in for YouTube's video host, serving `body` by path:
    /// /new in ranges; /old refuses (an expired address); /whole answers every range with the whole file;
    /// /deep refuses ranges past 1 MiB (an address that stops mid-song); /hop sends ranges past 1 MiB on to
    /// `hop_to` (another of YouTube's servers); /flaky refuses the first range past 1 MiB, then serves.
    fn fake_youtube(body: Arc<Vec<u8>>, hop_to: Option<u16>) -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let refused_once = Arc::new(std::sync::atomic::AtomicBool::new(false));
        std::thread::spawn(move || {
            for mut c in l.incoming().flatten() {
                let mut buf = Vec::new();
                let Some(head) = crate::session::read_head(&c, &mut buf) else { continue };
                let range = head.lines().find_map(|l| l.strip_prefix("Range: bytes=").or_else(|| l.strip_prefix("range: bytes="))).unwrap();
                let (a, b) = range.split_once('-').unwrap();
                let (a, b): (usize, usize) = (a.parse().unwrap(), b.parse::<usize>().unwrap().min(body.len() - 1));
                let path = head.split(' ').nth(1).unwrap_or("");
                let deep = a >= 1 << 20;
                let refuse = path == "/old" || (path == "/deep" && deep) || (path == "/flaky" && deep && !refused_once.swap(true, SeqCst));
                if refuse {
                    let _ = c.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                } else if path == "/hop" && deep && hop_to.is_some() {
                    let _ = write!(c, "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{}/new\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", hop_to.unwrap());
                } else if path == "/whole" {
                    let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = c.write_all(&body);
                } else {
                    let _ = write!(c, "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {a}-{b}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len(), b - a + 1);
                    let _ = c.write_all(&body[a..=b]);
                }
            }
        });
        port
    }

    /// What mpv receives from `pump` for bytes at..=end, and how the stream ended.
    fn played(stream: Stream, at: u64, end: u64, renew: impl FnMut(&Stream) -> Result<Stream, String> + Send + 'static) -> (Vec<u8>, Result<(), String>) {
        use std::io::Read;
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut mpv = TcpStream::connect(l.local_addr().unwrap()).unwrap();
        let (out, _) = l.accept().unwrap();
        let sent = std::thread::spawn(move || {
            let r = pump(&out, "dQw4w9WgXcQ", stream, at, end, renew);
            drop(out);
            r
        });
        let mut got = Vec::new();
        let _ = mpv.read_to_end(&mut got);
        (got, sent.join().unwrap())
    }

    fn song() -> Arc<Vec<u8>> {
        Arc::new((0..(CHUNK as usize * 5 / 2)).map(|i| (i * 7 % 251) as u8).collect())
    }

    #[test]
    fn streams_in_ranges_and_asks_again_when_youtube_refuses_the_address() {
        let body = song();
        let port = fake_youtube(body.clone(), None);
        let at = |path: &str, ahead: Option<usize>| Stream { ahead: ahead.map(|n| Arc::new(body[..n].to_vec())), ..fake(&format!("http://127.0.0.1:{port}{path}"), Some(body.len() as u64)) };
        let end = body.len() as u64 - 1;
        // the start fetched ahead goes out first, then the refused address is renewed once and the rest follows
        let renewals = Arc::new(AtomicUsize::new(0));
        let (n, fresh) = (renewals.clone(), at("/new", None));
        let (got, how) = played(at("/old", Some(1000)), 0, end, move |_| {
            n.fetch_add(1, SeqCst);
            Ok(fresh.clone())
        });
        assert!(how.is_ok() && got == body[..], "the whole song arrives in order");
        assert_eq!(renewals.load(SeqCst), 1);
        // a seek: just the range asked for
        let (got, how) = played(at("/new", Some(1000)), 1_500_000, 1_600_000, |_| Err("not needed".into()));
        assert!(how.is_ok() && got == body[1_500_000..=1_600_000]);
        // an address that stays refused ends the stream with the reason
        let still = at("/old", None);
        assert_eq!(played(at("/old", None), 0, end, move |_| Ok(still.clone())).1.unwrap_err(), "YouTube refused the stream (403) at byte 0");
    }

    #[test]
    fn follows_youtube_on_to_another_of_its_servers() {
        let body = song();
        let other = fake_youtube(body.clone(), None);
        let first = fake_youtube(body.clone(), Some(other));
        let hop = fake(&format!("http://127.0.0.1:{first}/hop"), Some(body.len() as u64));
        let end = body.len() as u64 - 1;
        // past the first megabyte every range is redirected: playing through and seeking both carry on there
        let (got, how) = played(hop.clone(), 0, end, |_| Err("a redirect isn't a refusal".into()));
        assert!(how.is_ok() && got == body[..], "{how:?}");
        let (got, how) = played(hop.clone(), 1_500_000, end, |_| Err("a redirect isn't a refusal".into()));
        assert!(how.is_ok() && got == body[1_500_000..]);
        assert!(rt(probe(&hop)), "the deep probe follows it too");
    }

    #[test]
    fn never_passes_on_bytes_from_the_wrong_place() {
        let body = song();
        let port = fake_youtube(body.clone(), None);
        // a server that answers every range with the whole file: the start is fine, a range from the middle isn't
        let whole = fake(&format!("http://127.0.0.1:{port}/whole"), Some(body.len() as u64));
        let w = whole.clone();
        let (got, how) = played(whole.clone(), 0, body.len() as u64 - 1, move |_| Ok(w.clone()));
        assert!(how.unwrap_err().contains("(200) at byte 1048576"));
        assert!(body.starts_with(&got) && got.len() == CHUNK as usize, "what arrived is the song's own start");
        let w = whole.clone();
        let (got, how) = played(whole, 1_500_000, 1_600_000, move |_| Ok(w.clone()));
        assert!(how.is_err() && got.is_empty());
    }

    #[test]
    fn tries_an_address_deep_in_the_file_before_using_it() {
        let body = song();
        let port = fake_youtube(body.clone(), None);
        let len = Some(body.len() as u64);
        assert!(rt(probe(&fake(&format!("http://127.0.0.1:{port}/new"), len))));
        // serves the start, refuses past the first megabyte: rejected before a song plays from it
        assert!(!rt(probe(&fake(&format!("http://127.0.0.1:{port}/deep"), len))));
        assert!(!rt(probe(&fake(&format!("http://127.0.0.1:{port}/whole"), len))));
    }

    #[test]
    fn carries_a_song_on_only_from_the_same_file() {
        let body = song();
        let port = fake_youtube(body.clone(), None);
        let len = Some(body.len() as u64);
        let deep = fake(&format!("http://127.0.0.1:{port}/deep"), len);
        // the new address is another format: the song stops rather than play the wrong bytes
        let other = Stream { itag: Some(140), ..fake(&format!("http://127.0.0.1:{port}/new"), len) };
        let (got, how) = played(deep.clone(), 0, body.len() as u64 - 1, move |_| Ok(other.clone()));
        assert!(how.unwrap_err().contains("another file") && got == body[..CHUNK as usize]);
        // the same format from another address: it carries on
        let same = fake(&format!("http://127.0.0.1:{port}/new"), len);
        let (got, how) = played(deep, 0, body.len() as u64 - 1, move |_| Ok(same.clone()));
        assert!(how.is_ok() && got == body[..]);
    }

    #[test]
    fn sets_aside_a_client_whose_addresses_stop_mid_song() {
        let e = Engine::default();
        e.distrust(&Stream { client: Some(1), ..fake("https://r.googlevideo.com/a", None) });
        e.distrust(&fake("https://r.googlevideo.com/a", None)); // yt-dlp's: nothing to set aside
        assert_eq!(*e.distrusted.lock().unwrap(), HashSet::from([1]));
    }

    /// On a machine that reaches YouTube: `cargo test --lib real_youtube -- --ignored --nocapture`
    #[test]
    #[ignore = "needs YouTube"]
    fn real_youtube_guest_clients_give_a_playable_stream() {
        let e = Engine::default();
        let mut worked = 0;
        for (i, c) in CLIENTS.iter().enumerate() {
            let started = Instant::now();
            match rt(e.guest(i, "dQw4w9WgXcQ")) {
                Ok(s) => {
                    let ok = rt(probe(&s));
                    println!("{}: {} itag {:?}, {:?} bytes, found in {} ms, serves deep in the file: {ok}", c.name, s.mime, s.itag, s.len, started.elapsed().as_millis());
                    worked += ok as usize;
                }
                Err(err) => println!("{}: {err}", c.name),
            }
        }
        assert!(worked > 0, "no guest client gave a stream YouTube serves");
    }

    /// mpv reads streams with ffmpeg's HTTP client: a real song through the server, start to end and from a seek,
    /// and, with the reconnect options mpv.rs gives it, on through a stream the server had to drop midway.
    /// `cargo test --lib ffmpeg_plays -- --ignored`
    #[test]
    #[ignore = "needs ffmpeg"]
    fn ffmpeg_plays_a_song_from_the_server_and_seeks_in_it() {
        let song = Arc::new(std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../test-media/tone_96k24.flac")).unwrap());
        assert!(song.len() > 1 << 20, "the song must reach past the first megabyte");
        // the server mpv plays from, in front of a fake YouTube at `path`; a refused address isn't renewed here
        let server = |path: &'static str| {
            let up = fake_youtube(song.clone(), None);
            let len = song.len() as u64;
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            let me = l.local_addr().unwrap().port();
            std::thread::spawn(move || {
                for c in l.incoming().flatten() {
                    std::thread::spawn(move || {
                        let mut buf = Vec::new();
                        let (_, head, range) = crate::session::read_head(&c, &mut buf).and_then(|h| parse_stream_request(&h, me, "tok")).unwrap();
                        let st = Stream { mime: "audio/flac", ..fake(&format!("http://127.0.0.1:{up}{path}"), Some(len)) };
                        let _ = respond(&c, "dQw4w9WgXcQ", st, head, range.as_deref(), |_| Err("no".into()));
                    });
                }
            });
            format!("http://127.0.0.1:{me}/tok/yt/dQw4w9WgXcQ")
        };
        // seconds of sound ffmpeg decodes (8 kHz mono 16-bit to its output), and what it said
        let ffmpeg = |args: &[&str]| {
            let out = std::process::Command::new("ffmpeg").args(["-v", "error", "-nostdin"]).args(args).args(["-ac", "1", "-ar", "8000", "-f", "s16le", "-"]).output().unwrap();
            (out.status.success(), out.stdout.len() as f64 / 16000.0, String::from_utf8_lossy(&out.stderr).into_owned())
        };
        let url = server("/new");
        let (ok, secs, said) = ffmpeg(&["-i", &url]);
        assert!(ok && (secs - 20.0).abs() < 0.1 && said.is_empty(), "start to end: {secs} s, {said}");
        let (ok, secs, said) = ffmpeg(&["-ss", "12", "-i", &url]);
        assert!(ok && (secs - 8.0).abs() < 0.1 && said.is_empty(), "from a seek to 12 s: {secs} s, {said}");
        // the server drops the stream once, past the first megabyte: without reconnecting, the song ends early…
        let (_, secs, said) = ffmpeg(&["-i", &server("/flaky")]);
        assert!(secs < 15.0 && said.contains("prematurely"), "the drop shows: {secs} s");
        // …and with mpv's reconnect options, ffmpeg asks again from where it stopped and plays to the end
        let (ok, secs, said) = ffmpeg(&["-reconnect", "1", "-reconnect_on_network_error", "1", "-reconnect_delay_max", "4", "-i", &server("/flaky")]);
        assert!(ok && (secs - 20.0).abs() < 0.1, "reconnected: {secs} s, {said}");
    }
}

// Listening together. One Qistara hosts, others join with an invite code, and everyone hears the same song at
// the same moment, with a chat beside it. There's no server in between: the host's PC is the server, reached on
// its network, over a VPN (Tailscale, ZeroTier) or through a forwarded port.
//
// Every connection is encrypted and authenticated with the invite's secret: HKDF-SHA256 over both sides' random
// nonces gives one AES-256-GCM key per direction, each frame carries a counter nonce, and the first frame each way
// proves the other side holds the secret. Anything that fails to open closes the connection.
//
// Guests ask for the host's files by item number (seq) and byte range, never by path: the host only serves the
// few items its page shared, each resolved from a library track id in Rust. A guest's mpv reads them through a
// loopback HTTP proxy (127.0.0.1, a random token in the path, Host checked), which mpv.rs keeps away from yt-dlp.
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::hkdf::{Salt, HKDF_SHA256};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAGIC: &[u8; 4] = b"QSS1";
const CONTROL: u8 = 0;
const DATA: u8 = 1;
const MAX_FRAME: usize = 1 << 20;
const CHUNK: u32 = 512 * 1024;
const MAX_GUESTS: usize = 16;
const MAX_DATA_PER_GUEST: usize = 4;
const HANDSHAKE: Duration = Duration::from_secs(10);
pub const DEFAULT_PORT: u16 = 47800;

type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("the OS random source");
    b
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ---- invite codes ----

/// `qistara://session/<host>:<port>#<secret>`: where to connect, and the key (the part after # never leaves
/// the people it's given to; Qistara doesn't register to open these links, they're pasted).
pub fn invite(host: &str, port: u16, secret: &[u8; 16]) -> String {
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    format!("qistara://session/{host}:{port}#{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret))
}

#[derive(Debug, PartialEq)]
pub struct Invite {
    pub host: String,
    pub port: u16,
    pub secret: [u8; 16],
}

pub fn parse_invite(code: &str) -> Result<Invite, String> {
    let bad = || "That isn't a Qistara session invite.".to_string();
    let rest = code.trim().strip_prefix("qistara://session/").ok_or_else(bad)?;
    let (addr, key) = rest.split_once('#').ok_or_else(bad)?;
    let (host, port) = addr.rsplit_once(':').ok_or_else(bad)?;
    let host = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);
    let port: u16 = port.parse().ok().filter(|p| *p > 0).ok_or_else(bad)?;
    // an IP or a host name, nothing else (no user@, no path)
    let ok_host = host.parse::<IpAddr>().is_ok()
        || (!host.is_empty() && host.len() <= 253 && host.split('.').all(|l| !l.is_empty() && l.len() <= 63 && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')));
    let secret: [u8; 16] = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key).ok().and_then(|k| k.try_into().ok()).ok_or_else(bad)?;
    if !ok_host {
        return Err(bad());
    }
    Ok(Invite { host: host.to_string(), port, secret })
}

// ---- the encrypted channel ----

fn key(secret: &[u8; 16], salt: &[u8], info: &[u8]) -> LessSafeKey {
    let prk = Salt::new(HKDF_SHA256, salt).extract(secret);
    let info = [info];
    let okm = prk.expand(&info, &AES_256_GCM).expect("HKDF length");
    LessSafeKey::new(UnboundKey::from(okm))
}

/// One direction of a connection: its key and its frame counter (the nonce; never reused, never skipped).
pub struct Half {
    key: LessSafeKey,
    n: u64,
}

impl Half {
    fn nonce(&mut self) -> Result<Nonce, String> {
        let mut b = [0u8; 12];
        b[4..].copy_from_slice(&self.n.to_be_bytes());
        self.n = self.n.checked_add(1).ok_or("connection used up")?;
        Ok(Nonce::assume_unique_for_key(b))
    }

    pub fn seal(&mut self, plain: &[u8]) -> Result<Vec<u8>, String> {
        if plain.len() > MAX_FRAME {
            return Err("frame too large".into());
        }
        let mut buf = plain.to_vec();
        let nonce = self.nonce()?;
        self.key.seal_in_place_append_tag(nonce, Aad::empty(), &mut buf).map_err(|_| "seal")?;
        let mut out = (buf.len() as u32).to_be_bytes().to_vec();
        out.extend(buf);
        Ok(out)
    }

    /// Opens one sealed frame body (without its length prefix).
    pub fn open(&mut self, mut body: Vec<u8>) -> Result<Vec<u8>, String> {
        let nonce = self.nonce()?;
        let n = self.key.open_in_place(nonce, Aad::empty(), &mut body).map_err(|_| "a frame failed to open")?.len();
        body.truncate(n);
        Ok(body)
    }
}

pub struct Sender {
    stream: TcpStream,
    half: Half,
}
impl Sender {
    pub fn send(&mut self, plain: &[u8]) -> Result<(), String> {
        let f = self.half.seal(plain)?;
        self.stream.write_all(&f).map_err(|e| e.to_string())
    }
    pub fn send_json(&mut self, v: &Value) -> Result<(), String> {
        self.send(v.to_string().as_bytes())
    }
}

pub struct Receiver {
    stream: TcpStream,
    half: Half,
}
impl Receiver {
    pub fn recv(&mut self) -> Result<Vec<u8>, String> {
        let mut len = [0u8; 4];
        self.stream.read_exact(&mut len).map_err(|e| e.to_string())?;
        let len = u32::from_be_bytes(len) as usize;
        if !(16..=MAX_FRAME + 16).contains(&len) {
            return Err("bad frame length".into());
        }
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body).map_err(|e| e.to_string())?;
        self.half.open(body)
    }
    pub fn recv_json(&mut self) -> Result<Value, String> {
        serde_json::from_slice(&self.recv()?).map_err(|_| "not a message".into())
    }
}

/// The guest's side of the handshake: say hello with a nonce and the connection kind, take the host's nonce.
pub fn connect(addr: SocketAddr, secret: &[u8; 16], kind: u8) -> Result<(Sender, Receiver), String> {
    let s = TcpStream::connect_timeout(&addr, HANDSHAKE).map_err(|e| format!("Couldn't reach the host: {e}"))?;
    s.set_read_timeout(Some(HANDSHAKE)).ok();
    s.set_nodelay(true).ok();
    let mine: [u8; 16] = random();
    let mut hello = MAGIC.to_vec();
    hello.push(kind);
    hello.extend(mine);
    (&s).write_all(&hello).map_err(|e| e.to_string())?;
    let mut back = [0u8; 20];
    (&s).read_exact(&mut back).map_err(|_| "The host didn't answer.".to_string())?;
    if &back[..4] != MAGIC {
        return Err("That isn't a Qistara session.".into());
    }
    let salt = [&mine[..], &back[4..]].concat();
    Ok(halves(s, secret, &salt, b"qistara guest to host", b"qistara host to guest"))
}

/// The host's side: read the hello, answer with a nonce. Returns the connection kind.
fn accept(s: TcpStream, secret: &[u8; 16]) -> Result<(u8, Sender, Receiver), String> {
    s.set_read_timeout(Some(HANDSHAKE)).ok();
    s.set_nodelay(true).ok();
    let mut hello = [0u8; 21];
    (&s).read_exact(&mut hello).map_err(|e| e.to_string())?;
    if &hello[..4] != MAGIC || !matches!(hello[4], CONTROL | DATA) {
        return Err("not a Qistara guest".into());
    }
    let mine: [u8; 16] = random();
    let mut back = MAGIC.to_vec();
    back.extend(mine);
    (&s).write_all(&back).map_err(|e| e.to_string())?;
    let salt = [&hello[5..21], &mine[..]].concat();
    let (tx, rx) = halves(s, secret, &salt, b"qistara host to guest", b"qistara guest to host");
    Ok((hello[4], tx, rx))
}

fn halves(s: TcpStream, secret: &[u8; 16], salt: &[u8], send_info: &[u8], recv_info: &[u8]) -> (Sender, Receiver) {
    let r = s.try_clone().expect("socket clone");
    (Sender { stream: s, half: Half { key: key(secret, salt, send_info), n: 0 } }, Receiver { stream: r, half: Half { key: key(secret, salt, recv_info), n: 0 } })
}

// ---- what the host shares ----

/// An item the host's page shares: a library track (by id, its path resolved in Rust), a YouTube video, or a
/// stream URL. Guests play it under `seq`.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Share {
    File { seq: u64, track: i64 },
    Yt { seq: u64, id: String },
    Url { seq: u64, url: String },
}

/// A stream a guest may open itself: http(s), and no sign-in in it (a Subsonic stream URL carries the account's
/// token and salt: sharing it would hand guests your server).
pub fn shareable_url(u: &str) -> bool {
    url::Url::parse(u).is_ok_and(|p| {
        matches!(p.scheme(), "http" | "https")
            && p.host_str().is_some()
            && p.username().is_empty()
            && p.password().is_none()
            && !p.query_pairs().any(|(k, _)| matches!(k.as_ref(), "t" | "s" | "p" | "u" | "token" | "apiKey" | "api_key" | "key" | "auth" | "sig"))
    })
}

struct Served {
    seq: u64,
    path: PathBuf,
}

// ---- the host ----

struct GuestConn {
    id: u64,
    name: String,
    ip: IpAddr,
    ticket: [u8; 16],
    tx: Arc<Mutex<Sender>>,
    stream: TcpStream,
    last_chat: Instant,
    data: Vec<TcpStream>,
}

pub struct Host {
    name: String,
    port: u16,
    secret: Mutex<[u8; 16]>,
    guests: Mutex<Vec<GuestConn>>,
    served: Mutex<Vec<Served>>,
    state: Mutex<Value>,
    banned: Mutex<HashSet<IpAddr>>,
    stop: AtomicBool,
    next_id: AtomicU64,
    emit: Emit,
}

impl Host {
    /// Starts listening on `port` (all interfaces: guests come from the network).
    pub fn start(name: &str, port: u16, emit: Emit) -> Result<Arc<Host>, String> {
        let listener = TcpListener::bind(("0.0.0.0", port)).map_err(|e| format!("Port {port} can't be used ({e}). Pick another one."))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let host = Arc::new(Host {
            name: clean(name, 32).unwrap_or_else(|| "Host".into()),
            port,
            secret: Mutex::new(random()),
            guests: Mutex::new(Vec::new()),
            served: Mutex::new(Vec::new()),
            state: Mutex::new(Value::Null),
            banned: Mutex::new(HashSet::new()),
            stop: AtomicBool::new(false),
            next_id: AtomicU64::new(1),
            emit,
        });
        let h = host.clone();
        std::thread::spawn(move || {
            while !h.stop.load(SeqCst) {
                match listener.accept() {
                    Ok((s, addr)) => {
                        // on Windows an accepted socket inherits the listener's non-blocking mode, and the
                        // handshake's first read would fail at once
                        let _ = s.set_nonblocking(false);
                        let h = h.clone();
                        std::thread::spawn(move || h.serve(s, addr.ip()));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(80)),
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
        });
        Ok(host)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn secret(&self) -> [u8; 16] {
        *self.secret.lock().unwrap()
    }

    fn serve(self: &Arc<Self>, s: TcpStream, ip: IpAddr) {
        if self.banned.lock().unwrap().contains(&ip) {
            return;
        }
        let peer = s.try_clone();
        let secret = self.secret();
        let Ok((kind, tx, mut rx)) = accept(s, &secret) else { return };
        // the first frame proves the guest holds the secret; anything else ends here
        let Ok(first) = rx.recv_json() else { return };
        match (kind, first["t"].as_str()) {
            (CONTROL, Some("join")) => {
                if let Ok(peer) = peer {
                    self.control(tx, rx, peer, ip, &first);
                }
            }
            (DATA, Some("data")) => {
                if let Ok(peer) = peer {
                    self.data(tx, rx, peer, first["ticket"].as_str().unwrap_or_default());
                }
            }
            _ => {}
        }
    }

    fn roster(&self) -> Vec<Value> {
        self.guests.lock().unwrap().iter().map(|g| json!({ "id": g.id, "name": g.name })).collect()
    }

    fn broadcast(&self, v: &Value) {
        let txs: Vec<_> = self.guests.lock().unwrap().iter().map(|g| (g.id, g.tx.clone())).collect();
        for (id, tx) in txs {
            if tx.lock().unwrap().send_json(v).is_err() {
                self.drop_guest(id);
            }
        }
    }

    fn roster_changed(&self) {
        let r = self.roster();
        self.broadcast(&json!({ "t": "roster", "host": self.name, "guests": r }));
        (self.emit)("session-roster", json!({ "host": self.name, "guests": r }));
    }

    fn drop_guest(&self, id: u64) {
        let gone = {
            let mut g = self.guests.lock().unwrap();
            g.iter().position(|x| x.id == id).map(|i| g.remove(i))
        };
        if let Some(g) = gone {
            let _ = g.stream.shutdown(Shutdown::Both);
            g.data.iter().for_each(|d| drop(d.shutdown(Shutdown::Both)));
            self.roster_changed();
        }
    }

    fn control(self: &Arc<Self>, tx: Sender, mut rx: Receiver, stream: TcpStream, ip: IpAddr, join: &Value) {
        let id = self.next_id.fetch_add(1, SeqCst);
        let name = clean(join["name"].as_str().unwrap_or_default(), 32).unwrap_or_else(|| format!("Guest {id}"));
        let ticket: [u8; 16] = random();
        let tx = Arc::new(Mutex::new(tx));
        {
            let mut g = self.guests.lock().unwrap();
            if g.len() >= MAX_GUESTS {
                let _ = tx.lock().unwrap().send_json(&json!({ "t": "bye", "reason": "The session is full." }));
                return;
            }
            g.push(GuestConn { id, name: name.clone(), ip, ticket, tx: tx.clone(), stream: stream.try_clone().unwrap_or(stream), last_chat: Instant::now() - Duration::from_secs(5), data: Vec::new() });
        }
        let state = self.state.lock().unwrap().clone();
        let welcome = json!({ "t": "welcome", "host": self.name, "you": id, "ticket": hex(&ticket), "state": state, "guests": self.roster() });
        if tx.lock().unwrap().send_json(&welcome).is_err() {
            return self.drop_guest(id);
        }
        self.roster_changed();
        // guests ping every 5 s; a minute of silence means they're gone
        rx.stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
        while let Ok(m) = rx.recv_json() {
            match m["t"].as_str() {
                Some("chat") => {
                    let Some(text) = clean(m["text"].as_str().unwrap_or_default(), 500) else { continue };
                    let ok = {
                        let mut g = self.guests.lock().unwrap();
                        g.iter_mut().find(|x| x.id == id).is_some_and(|x| {
                            let ok = x.last_chat.elapsed() >= Duration::from_secs(1);
                            if ok {
                                x.last_chat = Instant::now();
                            }
                            ok
                        })
                    };
                    if ok {
                        let msg = json!({ "t": "chat", "from": name, "text": text, "at": now_ms() });
                        self.broadcast(&msg);
                        (self.emit)("session-chat", msg);
                    }
                }
                Some("ping") => {
                    let _ = tx.lock().unwrap().send_json(&json!({ "t": "pong", "at": m["at"] }));
                }
                _ => {}
            }
        }
        self.drop_guest(id);
    }

    fn data(&self, mut tx: Sender, mut rx: Receiver, stream: TcpStream, ticket: &str) {
        {
            let mut g = self.guests.lock().unwrap();
            let Some(guest) = g.iter_mut().find(|x| hex(&x.ticket) == ticket) else { return };
            if guest.data.len() >= MAX_DATA_PER_GUEST {
                guest.data.retain(|d| d.peer_addr().is_ok());
                if guest.data.len() >= MAX_DATA_PER_GUEST {
                    return;
                }
            }
            guest.data.push(stream);
        }
        rx.stream.set_read_timeout(Some(Duration::from_secs(120))).ok();
        let mut open: Option<(u64, std::fs::File, u64)> = None;
        while let Ok(req) = rx.recv() {
            let Ok(req): Result<[u8; 20], _> = req.try_into() else { return };
            let seq = u64::from_be_bytes(req[..8].try_into().unwrap());
            let off = u64::from_be_bytes(req[8..16].try_into().unwrap());
            let len = u32::from_be_bytes(req[16..].try_into().unwrap()).min(CHUNK);
            if open.as_ref().is_none_or(|o| o.0 != seq) {
                let path = self.served.lock().unwrap().iter().find(|s| s.seq == seq).map(|s| s.path.clone());
                open = path.and_then(|p| std::fs::File::open(p).ok()).and_then(|f| Some((seq, f.metadata().ok()?.len(), f))).map(|(s, n, f)| (s, f, n));
            }
            let reply = match open.as_mut() {
                None => vec![1u8, 0, 0, 0, 0, 0, 0, 0, 0],
                Some((_, f, total)) => {
                    let mut out = vec![0u8];
                    out.extend(total.to_be_bytes());
                    let want = len.min(total.saturating_sub(off).min(u32::MAX as u64) as u32) as usize;
                    if want > 0 {
                        let at = out.len();
                        out.resize(at + want, 0);
                        if f.seek(SeekFrom::Start(off)).and_then(|_| f.read_exact(&mut out[at..])).is_err() {
                            out = vec![1u8, 0, 0, 0, 0, 0, 0, 0, 0];
                        }
                    }
                    out
                }
            };
            if tx.send(&reply).is_err() {
                return;
            }
        }
    }

    /// What the guests may read: the current and next items, and the one before (still playing for a guest
    /// a moment behind). A path must already be resolved from a library id and checked by the caller.
    pub fn serve_files(&self, files: Vec<(u64, PathBuf)>) {
        let mut s = self.served.lock().unwrap();
        for (seq, path) in files {
            if !s.iter().any(|x| x.seq == seq) {
                s.push(Served { seq, path });
            }
        }
        let keep = s.iter().map(|x| x.seq).max().unwrap_or(0).saturating_sub(2);
        s.retain(|x| x.seq >= keep);
    }

    pub fn set_state(&self, mut state: Value) {
        state["t"] = json!("state");
        state["at"] = json!(now_ms());
        *self.state.lock().unwrap() = state.clone();
        self.broadcast(&state);
    }

    pub fn chat(&self, text: &str) {
        if let Some(text) = clean(text, 500) {
            let msg = json!({ "t": "chat", "from": self.name, "text": text, "at": now_ms() });
            self.broadcast(&msg);
            (self.emit)("session-chat", json!({ "from": self.name, "text": text, "at": now_ms(), "me": true }));
        }
    }

    /// Removes a guest for the rest of the session: their address is turned away, and the others get a new secret
    /// over their encrypted connection, so the old invite (which the removed guest has) stops working.
    pub fn kick(&self, id: u64) -> [u8; 16] {
        let ip = self.guests.lock().unwrap().iter().find(|g| g.id == id).map(|g| g.ip);
        if let Some(ip) = ip {
            self.banned.lock().unwrap().insert(ip);
        }
        if let Some(g) = self.guests.lock().unwrap().iter().find(|g| g.id == id) {
            let _ = g.tx.lock().unwrap().send_json(&json!({ "t": "bye", "reason": "The host removed you from the session." }));
        }
        self.drop_guest(id);
        let fresh: [u8; 16] = random();
        *self.secret.lock().unwrap() = fresh;
        self.broadcast(&json!({ "t": "rekey", "secret": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(fresh) }));
        fresh
    }

    pub fn end(&self) {
        self.stop.store(true, SeqCst);
        self.broadcast(&json!({ "t": "bye", "reason": "The host ended the session." }));
        let ids: Vec<u64> = self.guests.lock().unwrap().iter().map(|g| g.id).collect();
        ids.into_iter().for_each(|id| self.drop_guest(id));
    }
}

/// Text from another person: control characters out, trimmed, capped. None when nothing is left.
pub fn clean(s: &str, max: usize) -> Option<String> {
    let t: String = s.chars().filter(|c| !c.is_control() || *c == ' ').take(max).collect();
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

// ---- the guest ----

pub struct Guest {
    addr: SocketAddr,
    secret: Mutex<[u8; 16]>,
    ticket: String,
    tx: Mutex<Sender>,
    control: TcpStream,
    proxy_port: u16,
    token: String,
    stop: AtomicBool,
}

impl Guest {
    /// Joins: the encrypted control connection, then the loopback proxy mpv reads the host's files through.
    pub fn join(code: &str, name: &str, emit: Emit) -> Result<(Arc<Guest>, Value), String> {
        let inv = parse_invite(code)?;
        let addr = (inv.host.as_str(), inv.port).to_socket_addrs().map_err(|_| "Couldn't find that host.".to_string())?.next().ok_or("Couldn't find that host.")?;
        let (mut tx, mut rx) = connect(addr, &inv.secret, CONTROL)?;
        tx.send_json(&json!({ "t": "join", "name": clean(name, 32).unwrap_or_default() }))?;
        let welcome = rx.recv_json().map_err(|_| "The host turned the invite down: it may be old, or the session has ended.".to_string())?;
        if welcome["t"] == "bye" {
            return Err(welcome["reason"].as_str().unwrap_or("The host said no.").to_string());
        }
        if welcome["t"] != "welcome" {
            return Err("That isn't a Qistara session.".into());
        }
        let ticket = welcome["ticket"].as_str().filter(|t| t.len() == 32).ok_or("That isn't a Qistara session.")?.to_string();
        let proxy = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let proxy_port = proxy.local_addr().map_err(|e| e.to_string())?.port();
        let control = tx.stream.try_clone().map_err(|e| e.to_string())?;
        let guest = Arc::new(Guest { addr, secret: Mutex::new(inv.secret), ticket, tx: Mutex::new(tx), control, proxy_port, token: hex(&random::<16>()), stop: AtomicBool::new(false) });

        // the proxy: one data connection per HTTP connection mpv opens
        let g = guest.clone();
        proxy.set_nonblocking(true).ok();
        std::thread::spawn(move || {
            while !g.stop.load(SeqCst) {
                match proxy.accept() {
                    Ok((s, _)) => {
                        let g = g.clone();
                        s.set_nonblocking(false).ok();
                        std::thread::spawn(move || g.proxy(s));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(50)),
                }
            }
        });
        // pings keep the connection alive and measure the delay
        let g = guest.clone();
        std::thread::spawn(move || {
            while !g.stop.load(SeqCst) {
                if g.tx.lock().unwrap().send_json(&json!({ "t": "ping", "at": now_ms() })).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        });
        // what the host says
        let g = guest.clone();
        rx.stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
        std::thread::spawn(move || {
            let mut lag = 0u64;
            let mut reason = "The connection to the host was lost.".to_string();
            while let Ok(m) = rx.recv_json() {
                match m["t"].as_str() {
                    Some("state") => emit("session-state", json!({ "state": m, "lag": lag })),
                    Some("chat") => emit("session-chat", json!({ "from": m["from"], "text": m["text"], "at": m["at"] })),
                    Some("roster") => emit("session-roster", json!({ "host": m["host"], "guests": m["guests"] })),
                    Some("pong") => lag = now_ms().saturating_sub(m["at"].as_u64().unwrap_or(0)) / 2,
                    Some("rekey") => {
                        if let Some(s) = m["secret"].as_str().and_then(|s| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).ok()).and_then(|k| <[u8; 16]>::try_from(k).ok()) {
                            *g.secret.lock().unwrap() = s;
                        }
                    }
                    Some("bye") => {
                        reason = m["reason"].as_str().unwrap_or("The session ended.").to_string();
                        break;
                    }
                    _ => {}
                }
            }
            if !g.stop.swap(true, SeqCst) {
                emit("session-ended", json!({ "reason": reason }));
            }
        });
        let info = json!({ "host": welcome["host"], "guests": welcome["guests"], "state": welcome["state"], "proxy": guest.base_url() });
        Ok((guest, info))
    }

    /// Where mpv reads item `seq`: base_url + seq.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/{}/", self.proxy_port, self.token)
    }

    pub fn chat(&self, text: &str) -> Result<(), String> {
        let text = clean(text, 500).ok_or("Type a message first.")?;
        self.tx.lock().unwrap().send_json(&json!({ "t": "chat", "text": text }))
    }

    pub fn leave(&self) {
        self.stop.store(true, SeqCst);
        let _ = self.control.shutdown(Shutdown::Both);
    }

    fn data_conn(&self) -> Result<(Sender, Receiver), String> {
        let secret = *self.secret.lock().unwrap();
        let (mut tx, rx) = connect(self.addr, &secret, DATA)?;
        tx.send_json(&json!({ "t": "data", "ticket": self.ticket }))?;
        rx.stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
        Ok((tx, rx))
    }

    fn proxy(&self, s: TcpStream) {
        s.set_read_timeout(Some(Duration::from_secs(120))).ok();
        let mut conn: Option<(Sender, Receiver)> = None;
        let mut buf = Vec::new();
        loop {
            let Some(head) = read_head(&s, &mut buf) else { return };
            let Some(req) = parse_request(&head, self.proxy_port, &self.token) else {
                let _ = (&s).write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                return;
            };
            if conn.is_none() {
                match self.data_conn() {
                    Ok(c) => conn = Some(c),
                    Err(_) => {
                        let _ = (&s).write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        return;
                    }
                }
            }
            let (tx, rx) = conn.as_mut().unwrap();
            if self.respond(&s, tx, rx, &req).is_err() || req.close {
                return;
            }
        }
    }

    fn respond(&self, mut s: &TcpStream, tx: &mut Sender, rx: &mut Receiver, req: &Request) -> Result<(), String> {
        let total = read(tx, rx, req.seq, 0, 0)?.ok_or("gone")?.0;
        let range = match parse_range(req.range.as_deref(), total) {
            Ok(r) => r,
            Err(()) => {
                return s.write_all(format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\n\r\n").as_bytes()).map_err(|e| e.to_string());
            }
        };
        let (start, end) = range.unwrap_or((0, total.saturating_sub(1)));
        let len = if total == 0 { 0 } else { end - start + 1 };
        let status = if range.is_some() { format!("206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}") } else { "200 OK".into() };
        s.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nAccept-Ranges: bytes\r\nContent-Length: {len}\r\nCache-Control: no-store\r\n\r\n").as_bytes()).map_err(|e| e.to_string())?;
        if req.head {
            return Ok(());
        }
        let mut at = start;
        while at < start + len {
            let want = (start + len - at).min(CHUNK as u64) as u32;
            let (_, bytes) = read(tx, rx, req.seq, at, want)?.ok_or("gone")?;
            if bytes.is_empty() {
                return Err("short".into());
            }
            s.write_all(&bytes).map_err(|e| e.to_string())?;
            at += bytes.len() as u64;
        }
        Ok(())
    }
}

/// One read over a data connection: (file size, bytes), or None when the host doesn't serve that item.
fn read(tx: &mut Sender, rx: &mut Receiver, seq: u64, off: u64, len: u32) -> Result<Option<(u64, Vec<u8>)>, String> {
    let mut req = seq.to_be_bytes().to_vec();
    req.extend(off.to_be_bytes());
    req.extend(len.to_be_bytes());
    tx.send(&req)?;
    let r = rx.recv()?;
    if r.len() < 9 || r[0] != 0 {
        return Ok(None);
    }
    Ok(Some((u64::from_be_bytes(r[1..9].try_into().unwrap()), r[9..].to_vec())))
}

pub(crate) fn read_head(mut s: &TcpStream, buf: &mut Vec<u8>) -> Option<String> {
    let mut chunk = [0u8; 2048];
    loop {
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..end]).into_owned();
            buf.drain(..end + 4);
            return Some(head);
        }
        if buf.len() > 8192 {
            return None;
        }
        let n = s.read(&mut chunk).ok().filter(|n| *n > 0)?;
        buf.extend_from_slice(&chunk[..n]);
    }
}

#[derive(Debug, PartialEq)]
pub struct Request {
    seq: u64,
    head: bool,
    range: Option<String>,
    close: bool,
}

/// GET or HEAD /<token>/<seq>, asked of 127.0.0.1:<port> itself (a web page that rebinds its domain to
/// 127.0.0.1 sends its own Host).
pub fn parse_request(head: &str, port: u16, token: &str) -> Option<Request> {
    let mut lines = head.lines();
    let mut first = lines.next()?.split(' ');
    let method = first.next()?;
    let path = first.next()?;
    let seq = path.strip_prefix('/')?.strip_prefix(token)?.strip_prefix('/')?.parse().ok()?;
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    let get = |k: &str| headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.clone());
    if get("host")? != format!("127.0.0.1:{port}") || !matches!(method, "GET" | "HEAD") {
        return None;
    }
    Some(Request { seq, head: method == "HEAD", range: get("range"), close: get("connection").is_some_and(|c| c.eq_ignore_ascii_case("close")) })
}

/// `Range: bytes=a-b | a- | -n` against a file of `total` bytes: Ok(None) for the whole file, Err for one that
/// can't be met (416).
pub fn parse_range(h: Option<&str>, total: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(h) = h else { return Ok(None) };
    let spec = h.trim().strip_prefix("bytes=").ok_or(())?;
    if spec.contains(',') {
        return Ok(None); // several ranges: the whole file instead
    }
    let (a, b) = spec.split_once('-').ok_or(())?;
    let (start, end) = match (a.trim(), b.trim()) {
        ("", n) => {
            let n: u64 = n.parse().map_err(|_| ())?;
            (total.saturating_sub(n), total.saturating_sub(1))
        }
        (a, "") => (a.parse().map_err(|_| ())?, total.saturating_sub(1)),
        (a, b) => (a.parse().map_err(|_| ())?, b.parse::<u64>().map_err(|_| ())?.min(total.saturating_sub(1))),
    };
    if total == 0 || start >= total || start > end {
        return Err(());
    }
    Ok(Some((start, end)))
}

/// This PC's address on its network (the route it would take to the internet; nothing is sent).
pub fn lan_address() -> Option<IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    s.local_addr().ok().map(|a| a.ip()).filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
}

// ---- Tauri commands ----

pub enum Role {
    Host(Arc<Host>),
    Guest(Arc<Guest>),
}

#[derive(Default)]
pub struct Session(pub Mutex<Option<Role>>);

fn emitter(app: &tauri::AppHandle) -> Emit {
    use tauri::Emitter;
    let app = app.clone();
    Arc::new(move |ev, v| drop(app.emit(ev, v)))
}

fn codes(h: &Host) -> Vec<String> {
    let mut hosts: Vec<String> = lan_address().map(|ip| ip.to_string()).into_iter().collect();
    hosts.push("127.0.0.1".into()); // the same PC, for trying it out
    hosts.iter().map(|a| invite(a, h.port(), &h.secret())).collect()
}

#[tauri::command]
pub fn session_host(app: tauri::AppHandle, session: tauri::State<Session>, name: String, port: Option<u16>) -> Result<Value, String> {
    let mut s = session.0.lock().unwrap();
    if s.is_some() {
        return Err("A session is already open. Leave or end it first.".into());
    }
    let host = Host::start(&name, port.unwrap_or(DEFAULT_PORT), emitter(&app))?;
    let out = json!({ "port": host.port(), "invites": codes(&host) });
    *s = Some(Role::Host(host));
    Ok(out)
}

/// The host's page: what plays now and next. Library tracks are looked up by id here, and only files in the
/// library are ever served; a stream URL carrying a sign-in is refused.
#[tauri::command]
pub fn session_share(session: tauri::State<Session>, lib: tauri::State<crate::Lib>, items: Vec<Share>) -> Result<(), String> {
    let Some(Role::Host(h)) = &*session.0.lock().unwrap() else { return Err("Not hosting a session.".into()) };
    let mut files = Vec::new();
    for it in items {
        match it {
            Share::File { seq, track } => {
                let path = lib.path_of(track)?.ok_or("That song isn't in the library.")?;
                if !crate::mpv::local_on(crate::mpv::OS, &path) || !std::path::Path::new(&path).is_file() {
                    return Err("That song's file can't be shared.".into());
                }
                files.push((seq, PathBuf::from(path)));
            }
            Share::Yt { id, .. } if !crate::ytdlp::valid_video_id(&id) => return Err("bad video id".into()),
            Share::Url { url, .. } if !shareable_url(&url) => return Err("This stream can't be shared: it carries a sign-in.".into()),
            _ => {}
        }
    }
    h.serve_files(files);
    Ok(())
}

#[tauri::command]
pub fn session_state(session: tauri::State<Session>, state: Value) -> Result<(), String> {
    match &*session.0.lock().unwrap() {
        Some(Role::Host(h)) => {
            h.set_state(state);
            Ok(())
        }
        _ => Err("Not hosting a session.".into()),
    }
}

#[tauri::command]
pub fn session_chat(session: tauri::State<Session>, text: String) -> Result<(), String> {
    match &*session.0.lock().unwrap() {
        Some(Role::Host(h)) => {
            h.chat(&text);
            Ok(())
        }
        Some(Role::Guest(g)) => g.chat(&text),
        None => Err("No session.".into()),
    }
}

#[tauri::command]
pub fn session_kick(session: tauri::State<Session>, id: u64) -> Result<Vec<String>, String> {
    match &*session.0.lock().unwrap() {
        Some(Role::Host(h)) => {
            h.kick(id);
            Ok(codes(h))
        }
        _ => Err("Not hosting a session.".into()),
    }
}

#[tauri::command]
pub async fn session_join(app: tauri::AppHandle, session: tauri::State<'_, Session>, code: String, name: String) -> Result<Value, String> {
    if session.0.lock().unwrap().is_some() {
        return Err("A session is already open. Leave or end it first.".into());
    }
    let emit = emitter(&app);
    let (guest, info) = tauri::async_runtime::spawn_blocking(move || Guest::join(&code, &name, emit)).await.map_err(|e| e.to_string())??;
    *session.0.lock().unwrap() = Some(Role::Guest(guest));
    Ok(info)
}

/// Leaves (a guest) or ends (the host) the session.
#[tauri::command]
pub fn session_leave(session: tauri::State<Session>) {
    match session.0.lock().unwrap().take() {
        Some(Role::Host(h)) => h.end(),
        Some(Role::Guest(g)) => g.leave(),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_codes_round_trip_and_refuse_anything_else() {
        let s: [u8; 16] = random();
        for host in ["192.168.1.20", "my-pc.tailnet.ts.net", "::1"] {
            let i = parse_invite(&invite(host, 47800, &s)).unwrap();
            assert_eq!(i, Invite { host: host.into(), port: 47800, secret: s });
        }
        for bad in ["", "https://evil.example", "qistara://session/host:0#AAAA", "qistara://session/a@b:1#AAAAAAAAAAAAAAAAAAAAAA", "qistara://session/h:1#short", "qistara://session/h/x:1#AAAAAAAAAAAAAAAAAAAAAA"] {
            assert!(parse_invite(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn frames_open_only_with_the_right_key_in_order() {
        let s: [u8; 16] = random();
        let salt = random::<32>();
        let mut a = Half { key: key(&s, &salt, b"x"), n: 0 };
        let mut b = Half { key: key(&s, &salt, b"x"), n: 0 };
        let f1 = a.seal(b"hello").unwrap();
        let f2 = a.seal(b"world").unwrap();
        assert_eq!(b.open(f1[4..].to_vec()).unwrap(), b"hello");
        // a replayed frame (counter already used) doesn't open
        let mut b2 = Half { key: key(&s, &salt, b"x"), n: 1 };
        assert!(b2.open(f1[4..].to_vec()).is_err());
        assert_eq!(b.open(f2[4..].to_vec()).unwrap(), b"world");
        // a wrong secret, or a flipped bit, doesn't open
        let mut other = Half { key: key(&random(), &salt, b"x"), n: 0 };
        assert!(other.open(a.seal(b"x").unwrap()[4..].to_vec()).is_err());
        let mut t = a.seal(b"tamper").unwrap();
        t[6] ^= 1;
        assert!(b.open(t[4..].to_vec()).is_err());
    }

    #[test]
    fn reads_http_requests_and_ranges() {
        let h = "GET /tok/7 HTTP/1.1\r\nHost: 127.0.0.1:5000\r\nRange: bytes=10-";
        assert_eq!(parse_request(h, 5000, "tok"), Some(Request { seq: 7, head: false, range: Some("bytes=10-".into()), close: false }));
        assert_eq!(parse_request("GET /tok/7 HTTP/1.1\r\nHost: evil.example", 5000, "tok"), None, "another site's page");
        assert_eq!(parse_request("GET /bad/7 HTTP/1.1\r\nHost: 127.0.0.1:5000", 5000, "tok"), None, "a wrong token");
        assert_eq!(parse_request("POST /tok/7 HTTP/1.1\r\nHost: 127.0.0.1:5000", 5000, "tok"), None);
        assert_eq!(parse_range(Some("bytes=0-99"), 1000), Ok(Some((0, 99))));
        assert_eq!(parse_range(Some("bytes=900-"), 1000), Ok(Some((900, 999))));
        assert_eq!(parse_range(Some("bytes=-100"), 1000), Ok(Some((900, 999))));
        assert_eq!(parse_range(Some("bytes=0-5000"), 1000), Ok(Some((0, 999))));
        assert_eq!(parse_range(None, 1000), Ok(None));
        assert_eq!(parse_range(Some("bytes=1000-"), 1000), Err(()));
        assert_eq!(parse_range(Some("bytes=5-1"), 1000), Err(()));
        assert_eq!(parse_range(Some("items=1-2"), 1000), Err(()));
    }

    #[test]
    fn never_shares_a_stream_that_carries_a_sign_in() {
        assert!(shareable_url("https://ice1.somafm.com/groovesalad-128-mp3"));
        assert!(!shareable_url("https://music.example/rest/stream?id=1&u=me&t=abc&s=xyz&format=raw"), "Subsonic");
        assert!(!shareable_url("https://me:pw@radio.example/s"));
        assert!(!shareable_url("file:///etc/passwd") && !shareable_url("rtmp://x/y"));
        assert_eq!(clean("  hi\u{0}\nthere\u{1b}[2J ", 500).as_deref(), Some("hithere[2J"));
        assert_eq!(clean("\n\t", 500), None);
        assert_eq!(clean(&"x".repeat(900), 500).map(|s| s.len()), Some(500));
    }

    #[test]
    fn fuzz_the_handshake_and_frames_never_panic() {
        let mut x = 77u32;
        let s: [u8; 16] = random();
        for _ in 0..3000 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            let body: Vec<u8> = (0..(x % 80)).map(|i| (x >> (i % 24)) as u8).collect();
            let mut h = Half { key: key(&s, &[1, 2, 3], b"x"), n: (x % 3) as u64 };
            assert!(h.open(body).is_err());
            let _ = parse_invite(&String::from_utf8_lossy(&x.to_be_bytes().repeat(5)));
            let _ = parse_range(Some(&format!("bytes={}-{}", x % 7, x % 5)), (x % 9) as u64);
        }
    }

    /// The whole path on loopback: host, guest, chat, the shared file through the guest's HTTP proxy.
    #[test]
    fn a_guest_joins_chats_and_streams_the_hosts_file() {
        let events: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
        let host_ev = events.clone();
        let host = Host::start("Ana", 0, Arc::new(move |e, v| host_ev.lock().unwrap().push((e.to_string(), v)))).unwrap();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/a.flac");
        host.serve_files(vec![(1, fixture.clone())]);
        host.set_state(json!({ "cur": { "seq": 1, "kind": "file", "title": "A" }, "pos": 3.5, "paused": false }));

        let guest_ev: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
        let ge = guest_ev.clone();
        let code = invite("127.0.0.1", host.port(), &host.secret());
        let (guest, info) = Guest::join(&code, "Budi\u{0}", Arc::new(move |e, v| ge.lock().unwrap().push((e.to_string(), v)))).unwrap();
        assert_eq!(info["host"], "Ana");
        assert_eq!(info["state"]["pos"], 3.5, "a late joiner gets what's playing at once");

        // a wrong secret is turned away
        let wrong = invite("127.0.0.1", host.port(), &random());
        assert!(Guest::join(&wrong, "x", Arc::new(|_, _| {})).is_err());

        guest.chat("halo semua").unwrap();
        host.chat("hi Budi");
        std::thread::sleep(Duration::from_millis(400));
        let mut got: Vec<String> = guest_ev.lock().unwrap().iter().filter(|(e, _)| e == "session-chat").map(|(_, v)| v["text"].as_str().unwrap().to_string()).collect();
        got.sort();
        assert_eq!(got, ["halo semua", "hi Budi"], "both messages reach the guest (in either order)");
        let roster = events.lock().unwrap().iter().rev().find(|(e, _)| e == "session-roster").map(|(_, v)| v.clone()).unwrap();
        assert_eq!(roster["guests"][0]["name"], "Budi");

        // the file, byte for byte, through the loopback proxy with a Range request
        let want = std::fs::read(&fixture).unwrap();
        let url = guest.base_url();
        let port: u16 = url.trim_start_matches("http://127.0.0.1:").split('/').next().unwrap().parse().unwrap();
        let path = url.trim_start_matches(&format!("http://127.0.0.1:{port}"));
        let get = |range: &str| {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(s, "GET {path}1 HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nRange: {range}\r\nConnection: close\r\n\r\n").unwrap();
            let mut out = Vec::new();
            s.read_to_end(&mut out).unwrap();
            let end = out.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            (String::from_utf8_lossy(&out[..end]).into_owned(), out[end + 4..].to_vec())
        };
        let (head, body) = get("bytes=0-");
        assert!(head.starts_with("HTTP/1.1 206"), "{head}");
        assert_eq!(body, want);
        let (_, tail) = get("bytes=-100");
        assert_eq!(tail, want[want.len() - 100..]);
        // an item the host never shared, or a page that isn't ours, gets nothing
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET {path}1 HTTP/1.1\r\nHost: evil.example\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 404"));

        // removed: the guest is told, and the old invite stops working
        let id = roster["guests"][0]["id"].as_u64().unwrap();
        host.kick(id);
        std::thread::sleep(Duration::from_millis(300));
        assert!(guest_ev.lock().unwrap().iter().any(|(e, v)| e == "session-ended" && v["reason"].as_str().unwrap().contains("removed")));
        assert!(Guest::join(&code, "again", Arc::new(|_, _| {})).is_err());
        host.end();
    }
}

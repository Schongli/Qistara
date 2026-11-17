// Discord Rich Presence: "Listening to Qistara" and the song on your Discord profile. It goes through the Discord
// app's own local connection (a named pipe on Windows, a Unix socket elsewhere), never the network. Off by
// default (Settings › Integrations). Discord gets the title, artist, album, the song's times and a cover that is
// already a public https picture (YT Music, YouTube, Cover Art Archive) or the Qistara logo: never a file path.
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

/// The Discord application ("Qistara") whose name shows as "Listening to …". Set at build time
/// (QISTARA_DISCORD_ID), from the Discord Developer Portal; without it the setting says so.
const CLIENT_ID: &str = match option_env!("QISTARA_DISCORD_ID") {
    Some(id) => id,
    None => "",
};
const REPO: &str = "https://github.com/Schongli/Qistara";
/// Hosts a cover may come from: Discord fetches it itself, so only public pictures, never anything local.
const COVER_HOSTS: [&str; 6] = ["i.ytimg.com", "lh3.googleusercontent.com", "yt3.ggpht.com", "yt3.googleusercontent.com", "coverartarchive.org", "i.scdn.co"];

/// What the page sends: the song, its place in time, and a listening session's size.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    title: String,
    artist: String,
    album: String,
    /// when the song started and will end, in ms since 1970 (the page works them out from mpv)
    start: Option<u64>,
    end: Option<u64>,
    cover: Option<String>,
    /// (people listening, how many may) during a listening session
    party: Option<(u32, u32)>,
}

/// Discord wants 2–128 characters in each text field (a one-letter title gets a zero-width space).
fn field(s: &str) -> String {
    let t: String = s.trim().chars().filter(|c| !c.is_control()).take(128).collect();
    match t.chars().count() {
        0 => "Untitled".into(),
        1 => format!("{t}\u{200b}"),
        _ => t,
    }
}

fn public_cover(u: &str) -> bool {
    url::Url::parse(u).is_ok_and(|p| p.scheme() == "https" && p.host_str().is_some_and(|h| COVER_HOSTS.contains(&h)) && u.len() <= 256)
}

/// The SET_ACTIVITY arguments for an activity (None clears it).
pub fn activity_json(a: Option<&Activity>) -> Value {
    let Some(a) = a else { return json!({ "pid": std::process::id(), "activity": null }) };
    let mut act = json!({
        "type": 2, // Listening
        "details": field(&a.title),
        "state": field(&if a.artist.is_empty() { "Qistara".into() } else { format!("by {}", a.artist) }),
        "assets": {
            "large_image": a.cover.as_deref().filter(|c| public_cover(c)).unwrap_or("qistara"),
            "large_text": field(if a.album.is_empty() { &a.title } else { &a.album }),
            "small_image": "qistara",
            "small_text": "Qistara",
        },
        "buttons": [{ "label": "Get Qistara", "url": REPO }],
    });
    if let Some(start) = a.start {
        act["timestamps"] = json!({ "start": start });
        if let Some(end) = a.end.filter(|e| *e > start) {
            act["timestamps"]["end"] = json!(end);
        }
    }
    if let Some((n, max)) = a.party.filter(|(n, max)| *n > 0 && max >= n) {
        act["party"] = json!({ "id": "qistara-session", "size": [n, max] });
    }
    json!({ "pid": std::process::id(), "activity": act })
}

/// A frame of Discord's IPC: op and length (little-endian u32 each), then JSON.
pub fn frame(op: u32, v: &Value) -> Vec<u8> {
    let body = v.to_string().into_bytes();
    let mut out = op.to_le_bytes().to_vec();
    out.extend((body.len() as u32).to_le_bytes());
    out.extend(body);
    out
}

pub fn read_frame(r: &mut impl Read) -> std::io::Result<(u32, Value)> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head)?;
    let op = u32::from_le_bytes(head[..4].try_into().unwrap());
    let len = u32::from_le_bytes(head[4..].try_into().unwrap()) as usize;
    if len > 1 << 16 {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

trait Pipe: Read + Write + Send {}
impl<T: Read + Write + Send> Pipe for T {}

/// The Discord app's local connection, if it's running: discord-ipc-0 … 9.
fn open() -> Option<Box<dyn Pipe>> {
    #[cfg(windows)]
    {
        (0..10).find_map(|n| std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{n}")).ok().map(|f| Box::new(f) as Box<dyn Pipe>))
    }
    #[cfg(unix)]
    {
        let dirs: Vec<std::path::PathBuf> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"].iter().filter_map(std::env::var_os).map(Into::into).chain([std::path::PathBuf::from("/tmp")]).collect();
        open_in(&dirs)
    }
}

/// The first socket among `dirs` (and where Flatpak and Snap Discord put theirs) that answers.
#[cfg(unix)]
fn open_in(dirs: &[std::path::PathBuf]) -> Option<Box<dyn Pipe>> {
    for d in dirs {
        for sub in ["", "app/com.discordapp.Discord", "snap.discord"] {
            for n in 0..10 {
                if let Ok(s) = std::os::unix::net::UnixStream::connect(d.join(sub).join(format!("discord-ipc-{n}"))) {
                    s.set_read_timeout(Some(Duration::from_secs(5))).ok();
                    return Some(Box::new(s));
                }
            }
        }
    }
    None
}

/// Says hello as our application; Discord answers with READY.
fn handshake(p: &mut dyn Pipe) -> std::io::Result<()> {
    p.write_all(&frame(0, &json!({ "v": 1, "client_id": CLIENT_ID })))?;
    let (op, v) = read_frame(&mut ReadOf(p))?;
    if op == 1 && v["evt"] == "READY" { Ok(()) } else { Err(std::io::Error::other("Discord said no")) }
}

struct ReadOf<'a>(&'a mut dyn Pipe);
impl Read for ReadOf<'_> {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(b)
    }
}

fn send(p: &mut dyn Pipe, args: &Value, nonce: u64) -> std::io::Result<()> {
    p.write_all(&frame(1, &json!({ "cmd": "SET_ACTIVITY", "args": args, "nonce": nonce.to_string() })))?;
    read_frame(&mut ReadOf(p)).map(drop)
}

/// One thread holds the connection: it sends the latest activity, and while Discord isn't running it tries again
/// every 15 s, quietly.
fn worker(rx: std::sync::mpsc::Receiver<Value>, connect: fn() -> Option<Box<dyn Pipe>>) {
    let (mut pipe, mut pending, mut nonce): (Option<Box<dyn Pipe>>, Option<Value>, u64) = (None, None, 0);
    loop {
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(a) => pending = Some(a),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(a) = rx.try_recv() {
            pending = Some(a); // only the latest matters
        }
        let Some(args) = pending.clone() else { continue };
        if pipe.is_none() {
            pipe = connect().and_then(|mut p| handshake(p.as_mut()).ok().map(|_| p));
        }
        if let Some(p) = pipe.as_mut() {
            nonce += 1;
            match send(p.as_mut(), &args, nonce) {
                Ok(()) => pending = None,
                Err(_) => pipe = None,
            }
        }
    }
}

#[derive(Default)]
pub struct Discord(Mutex<Option<Sender<Value>>>);

/// Whether this build can show anything on Discord (its application id was set at build time).
#[tauri::command]
pub fn discord_ready() -> bool {
    !CLIENT_ID.is_empty()
}

/// Shows (or with None clears) what's playing on Discord.
#[tauri::command]
pub fn discord_set(d: tauri::State<Discord>, activity: Option<Activity>) -> Result<(), String> {
    if CLIENT_ID.is_empty() {
        return Err("Discord isn't set up in this build.".into());
    }
    let mut tx = d.0.lock().unwrap();
    let tx = tx.get_or_insert_with(|| {
        let (tx, rx) = channel();
        std::thread::spawn(move || worker(rx, open));
        tx
    });
    tx.send(activity_json(activity.as_ref())).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song() -> Activity {
        Activity { title: "Chasing the Torrents".into(), artist: "Yu-Peng Chen".into(), album: "HOYO-MiX".into(), start: Some(1_000), end: Some(235_000), cover: None, party: None }
    }

    #[test]
    fn says_listening_with_the_song_and_never_a_path() {
        let v = activity_json(Some(&Activity { cover: Some("C:\\Users\\me\\Music\\cover.jpg".into()), ..song() }));
        let a = &v["activity"];
        assert_eq!((a["type"].as_u64(), a["details"].as_str(), a["state"].as_str()), (Some(2), Some("Chasing the Torrents"), Some("by Yu-Peng Chen")));
        // a local picture is the logo instead: Discord can't see your files, and shouldn't learn their names
        assert_eq!(a["assets"]["large_image"], "qistara");
        assert!(!v.to_string().contains("Users"));
        assert_eq!(a["timestamps"]["end"], 235_000);
        assert_eq!(a["buttons"][0]["label"], "Get Qistara");
        let yt = activity_json(Some(&Activity { cover: Some("https://lh3.googleusercontent.com/x=w544-h544".into()), party: Some((3, 16)), ..song() }));
        assert_eq!(yt["activity"]["assets"]["large_image"], "https://lh3.googleusercontent.com/x=w544-h544");
        assert_eq!(yt["activity"]["party"]["size"], json!([3, 16]));
        assert_eq!(activity_json(None)["activity"], Value::Null);
    }

    #[test]
    fn keeps_fields_within_discords_limits() {
        let long = activity_json(Some(&Activity { title: "x".repeat(400), artist: String::new(), ..song() }));
        assert_eq!(long["activity"]["details"].as_str().unwrap().chars().count(), 128);
        assert_eq!(long["activity"]["state"], "Qistara");
        let short = activity_json(Some(&Activity { title: "A".into(), ..song() }));
        assert!(short["activity"]["details"].as_str().unwrap().chars().count() >= 2);
        assert!(!public_cover("https://evil.example/c.jpg") && !public_cover("http://i.ytimg.com/x.jpg"));
    }

    #[cfg(unix)]
    #[test]
    fn talks_to_a_discord_socket() {
        let d = tempfile::tempdir().unwrap();
        let server = std::os::unix::net::UnixListener::bind(d.path().join("discord-ipc-0")).unwrap();
        let t = std::thread::spawn(move || {
            let (mut s, _) = server.accept().unwrap();
            let (op, hello) = read_frame(&mut s).unwrap();
            assert_eq!((op, hello["v"].as_u64()), (0, Some(1)));
            s.write_all(&frame(1, &json!({ "cmd": "DISPATCH", "evt": "READY" }))).unwrap();
            let (op, set) = read_frame(&mut s).unwrap();
            s.write_all(&frame(1, &json!({ "cmd": "SET_ACTIVITY", "nonce": set["nonce"] }))).unwrap();
            (op, set)
        });
        let mut p = open_in(&[d.path().to_path_buf()]).unwrap();
        handshake(p.as_mut()).unwrap();
        send(p.as_mut(), &activity_json(Some(&song())), 1).unwrap();
        let (op, set) = t.join().unwrap();
        assert_eq!((op, set["cmd"].as_str()), (1, Some("SET_ACTIVITY")));
        assert_eq!(set["args"]["activity"]["details"], "Chasing the Torrents");
    }
}

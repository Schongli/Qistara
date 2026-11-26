//! The page reaches mpv only through these commands: the libmpv plugin's own command, set_property,
//! get_property and init aren't granted to it (capabilities/default.json). Each mpv command, property and
//! start option is checked against what the app sends, so script in the page can't run programs (`run`,
//! `subprocess`, `load-script`), point the ytdl hook at another exe (`script-opts`), hand yt-dlp options
//! (`ytdl-raw-options`) or write files (`stream-record`, `log-file`, lavfi filters). Options that hold a
//! path are set here, never by the page.

use serde_json::{json, Value};
#[cfg(desktop)]
use tauri::Manager;
use tauri::WebviewWindow;
#[cfg(desktop)]
use tauri_plugin_libmpv::{MpvConfig, MpvExt};

/// Phones have no libmpv here yet: the player says so instead of starting (docs/mobile.md).
#[cfg(mobile)]
const NO_PLAYER: &str = "Playback on phones isn't ready yet.";
#[cfg(mobile)]
#[tauri::command]
pub async fn mpv_init(_window: WebviewWindow, _mpv_config: Value) -> Result<String, String> {
    Err(NO_PLAYER.into())
}
#[cfg(mobile)]
#[tauri::command]
pub async fn mpv_command(_window: WebviewWindow, name: String, args: Vec<Value>) -> Result<(), String> {
    check_command(&name, &args).and(Err(NO_PLAYER.into()))
}
#[cfg(mobile)]
#[tauri::command]
pub async fn mpv_set(_window: WebviewWindow, name: String, value: Value) -> Result<(), String> {
    check_prop(&name, &value).and(Err(NO_PLAYER.into()))
}
#[cfg(mobile)]
#[tauri::command]
pub async fn mpv_ytdl_sign_in(_window: WebviewWindow, _cookies: String) -> Result<(), String> {
    Err(NO_PLAYER.into())
}

use crate::ytdlp;

#[cfg(desktop)]
/// The ytdl hook's yt-dlp options: never a config file, the same cache as ytdlp.rs gives it (streaming/), our
/// JavaScript runtime by path, and the sign-in: Firefox's cookies, or the hook's copy of the account's cookies or of
/// a cookies.txt.
fn ytdl_raw(app: &tauri::AppHandle, cookies: &str) -> String {
    let mut o = vec!["ignore-config=".to_string()];
    if let Ok(dir) = ytdlp::streaming_dir(app, "ytdlp-cache") {
        o.push(format!("cache-dir={}", quote(&dir.to_string_lossy())));
    }
    if let Ok(js) = ytdlp::js_runtime(app) {
        o.push(format!("js-runtimes={}", quote(&js)));
    }
    match cookies {
        "firefox" => o.push("cookies-from-browser=firefox".into()),
        "" => {}
        path => match ytdlp::hook_cookies(app, Some(path)) {
            Ok(Some(copy)) => o.push(format!("cookies={}", quote(&copy.to_string_lossy()))),
            // a file that's gone plays as a guest
            _ => log::warn!("YouTube plays as a guest: the cookies file can't be read"),
        },
    }
    if matches!(cookies, "" | "firefox") {
        let _ = ytdlp::hook_cookies(app, None);
    }
    o.join(",")
}

#[derive(Clone, Copy)]
enum Kind {
    /// yes/no or a JSON bool
    Flag,
    Num,
    /// a track or chapter number, or no/auto
    Track,
    One(&'static [&'static str]),
    Aspect,
    Device,
    Af,
    /// the audio output: one per OS, each the one bit-perfect path that OS has
    Ao,
}
use Kind::*;

/// The rules differ per OS (device names, what a local path looks like). `cfg` only picks the OS: the
/// functions take it as a value, so every OS's rules are tested on every machine.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Os {
    Windows,
    Mac,
    Linux,
}
pub const OS: Os = if cfg!(windows) { Os::Windows } else if cfg!(target_os = "macos") { Os::Mac } else { Os::Linux };

/// WASAPI exclusive, CoreAudio hog mode (mpv's coreaudio redirects to coreaudio_exclusive), ALSA hw:.
fn ao(os: Os, s: &str) -> bool {
    s == match os {
        Os::Windows => "wasapi",
        Os::Mac => "coreaudio",
        Os::Linux => "alsa",
    }
}

/// An output device as mpv lists it. ALSA names are held to plain hardware PCMs: an ALSA PCM name can also be a
/// plugin that writes a file (`file:…`), which would undo everything the allowlist keeps out.
pub fn device(os: Os, s: &str) -> bool {
    s == "auto"
        || match os {
            Os::Windows => s.starts_with("wasapi/"),
            Os::Mac => s.strip_prefix("coreaudio/").is_some_and(|uid| !uid.is_empty() && uid.len() <= 256 && !uid.chars().any(char::is_control)),
            Os::Linux => s.strip_prefix("alsa/").is_some_and(|name| {
                let (pcm, args) = name.split_once(':').unwrap_or((name, ""));
                let mut a = args.split(',');
                let card = |c: &str| c.strip_prefix("CARD=").is_some_and(|c| (1..=32).contains(&c.len()) && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
                let dev = |d: &str| d.strip_prefix("DEV=").is_some_and(|n| (1..=2).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()));
                matches!(pcm, "default" | "sysdefault" | "hw" | "plughw" | "front" | "iec958" | "hdmi")
                    && (args.is_empty() || (a.next().is_some_and(card) && a.next().is_none_or(dev) && a.next().is_none()))
            }),
        }
}

/// Properties and start options the page may set (player.svelte.ts and its callers), and what each may hold.
const PROPS: &[(&str, Kind)] = &[
    ("pause", Flag), ("mute", Flag), ("volume", Num), ("volume-max", Num), ("ao-volume", Num), ("speed", Num),
    ("loop-file", One(&["inf", "no"])), ("aid", Track), ("sid", Track), ("chapter", Track),
    ("sub-delay", Num), ("sub-scale", Num), ("sub-margin-y", Num), ("video-aspect-override", Aspect),
    ("panscan", Num), ("audio-device", Device), ("af", Af), ("audio-exclusive", Flag),
    ("replaygain", One(&["no", "track", "album"])), ("audio-spdif", One(&["", "ac3,eac3,dts,dts-hd,truehd"])),
    ("hwdec", One(&["auto-safe", "no"])),
    // a music video over a local song (musicvideo.svelte.ts): its sync, and hiding it again
    ("audio-delay", Num), ("vid", Track),
    // start options (startMpv)
    ("ao", Ao), ("vo", One(&["gpu-next"])), ("target-colorspace-hint", Flag),
    ("video-sync", One(&["audio"])), ("idle", Flag), ("force-window", Flag), ("keep-open", Flag),
    ("gapless-audio", One(&["weak", "yes", "no"])), ("prefetch-playlist", Flag), ("audio-display", One(&["no"])),
    ("cover-art-auto", One(&["no"])), ("input-default-bindings", Flag), ("input-vo-keyboard", Flag), ("osc", Flag),
    ("osd-level", Num), ("sub-auto", One(&["fuzzy"])), ("msg-level", One(&["all=warn"])), ("ytdl", Flag),
    ("ytdl-format", One(&["bestaudio/best"])),
    ("watch-later-options", One(&["start,aid,sid,sub-delay,audio-delay,sub-scale"])),
    ("screenshot-format", One(&["png"])), ("screenshot-template", One(&["Qistara %F %wH.%wM.%wS"])),
];

/// A command argument or property value as mpv reads it.
fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(if *b { "yes" } else { "no" }.into()),
        _ => None,
    }
}

fn num(s: &str) -> bool {
    s.parse::<f64>().is_ok_and(f64::is_finite)
}

fn index(s: &str) -> bool {
    !s.is_empty() && s.len() <= 9 && s.bytes().all(|b| b.is_ascii_digit())
}

/// The labels eq.ts gives its filters: eq0, eq1…
fn eq_label(l: &str) -> bool {
    l.strip_prefix("eq").is_some_and(|n| n.len() <= 2 && index(n))
}

/// The chain eq.ts afString builds: `@pre:lavfi=[volume=-2dB],@eq0:lavfi=[equalizer=f=100:t=q:w=1:g=3],…`
fn af(s: &str) -> bool {
    s.is_empty()
        || s.split(',').all(|f| {
            let Some((label, filter)) = f.strip_prefix('@').and_then(|f| f.split_once(":lavfi=[")) else { return false };
            let Some((name, params)) = filter.strip_suffix(']').and_then(|f| f.split_once('=')) else { return false };
            match (label, name) {
                ("pre", "volume") => params.strip_suffix("dB").is_some_and(num),
                (l, "equalizer" | "lowshelf" | "highshelf" | "highpass" | "lowpass") if eq_label(l) => params
                    .split(':')
                    .all(|p| p == "t=q" || matches!(p.split_once('='), Some(("f" | "w" | "g" | "p", v)) if num(v))),
                _ => false,
            }
        })
}

/// A file on a drive or a share (not a device path like \\.\pipe\…); on Linux and macOS an absolute path
/// outside the kernel's own trees.
pub fn local_on(os: Os, p: &str) -> bool {
    if os != Os::Windows {
        return p.starts_with('/')
            && !p.starts_with("//")
            && !p.contains('\0')
            && !p.split('/').any(|s| s == "..")
            && !["/dev/", "/proc/", "/sys/"].iter().any(|d| p.starts_with(d));
    }
    let b = p.as_bytes();
    let drive = b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/');
    drive || (p.starts_with(r"\\") && !p.starts_with(r"\\?\") && !p.starts_with(r"\\.\"))
}


/// The one video format the page may ask the ytdl hook for (youtube.svelte.ts watchTrack): up to 1080p with its audio.
pub const VIDEO_FORMAT: &str = "bestvideo[height<=?1080]+bestaudio/best[height<=?1080]";

/// loadfile's per-file options: a start time and the video format, each at most once; nothing else.
fn file_options(s: &str) -> bool {
    let (mut start, mut format) = (false, false);
    s.split(',').all(|o| match o.split_once('=') {
        Some(("start", v)) if !start && num(v) => {
            start = true;
            true
        }
        Some(("ytdl-format", v)) if !format && v == VIDEO_FORMAT => {
            format = true;
            true
        }
        _ => false,
    })
}

/// What `video-add` may open over a song: a video file, YouTube's own video host (musicvideo.svelte.ts gets the URL
/// from yt-dlp), or a listening session's loopback proxy.
fn video_source(os: Os, s: &str) -> bool {
    local_on(os, s)
        || url::Url::parse(s).is_ok_and(|u| match u.scheme() {
            "https" => u.host_str().is_some_and(|h| h.ends_with(".googlevideo.com")),
            "http" => u.host_str() == Some("127.0.0.1") && u.port().is_some(),
            _ => false,
        })
}

/// A local file, a stream or a ytdl:// web page: never mpv's other protocols (av://, lavf://, edl://, fd://…).
fn media(os: Os, p: &str) -> bool {
    let l = p.to_ascii_lowercase();
    let web = |u: &str| u.starts_with("http://") || u.starts_with("https://");
    local_on(os, p)
        || web(&l)
        || ["rtsp://", "rtmp://", "mms://", "mmsh://"].iter().any(|s| l.starts_with(s))
        || l.strip_prefix("ytdl://").is_some_and(web)
}

pub fn check_prop(name: &str, value: &Value) -> Result<(), String> {
    check_prop_on(OS, name, value)
}

pub fn check_prop_on(os: Os, name: &str, value: &Value) -> Result<(), String> {
    let kind = PROPS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k);
    let ok = kind.zip(text(value)).is_some_and(|(kind, s)| match kind {
        Flag => s == "yes" || s == "no",
        Num => num(&s),
        Track => s == "no" || s == "auto" || index(&s),
        One(allowed) => allowed.contains(&s.as_str()),
        Aspect => s == "-1" || s.split_once(':').is_some_and(|(w, h)| num(w) && num(h)),
        Device => device(os, &s),
        Af => af(&s),
        Ao => ao(os, &s),
    });
    if ok { Ok(()) } else { Err(format!("mpv: {name} can't be set to {value}")) }
}

pub fn check_command(name: &str, args: &[Value]) -> Result<(), String> {
    check_command_on(OS, name, args)
}

pub fn check_command_on(os: Os, name: &str, args: &[Value]) -> Result<(), String> {
    let refuse = || format!("mpv: command {name} {} isn't allowed", Value::from(args));
    let owned = args.iter().map(text).collect::<Option<Vec<_>>>().ok_or_else(refuse)?;
    let a: Vec<&str> = owned.iter().map(String::as_str).collect();
    let ok = match (name, a.as_slice()) {
        ("set", [p, _]) => return check_prop_on(os, p, &args[1]),
        ("loadfile", [p] | [p, "replace" | "append" | "append-play"]) => media(os, p),
        // resume, reload and YouTube as video (player.svelte.ts loadTrack): a start time and the video format
        ("loadfile", [p, "replace" | "append", "-1", opts]) => media(os, p) && file_options(opts),
        ("video-add", [src, "select"]) => video_source(os, src),
        ("seek", [t, "absolute" | "relative"]) => num(t),
        ("sub-add", [p, "select"]) => local_on(os, p),
        ("af-command", ["pre", "volume", v]) => v.strip_suffix("dB").is_some_and(num),
        ("af-command", [l, "f" | "w" | "g", v]) => eq_label(l) && num(v),
        ("playlist-remove", [i]) => index(i),
        ("cycle", ["sub" | "audio"]) | ("screenshot", ["video"]) => true,
        (
            "stop" | "ab-loop" | "frame-step" | "frame-back-step" | "playlist-clear" | "write-watch-later-config"
            | "delete-watch-later-config",
            [],
        ) => true,
        _ => false,
    };
    if ok { Ok(()) } else { Err(refuse()) }
}

#[cfg(desktop)]
/// mpv's length-prefixed quoting, so a comma or '=' in a path can't split an option list.
fn quote(s: &str) -> String {
    format!("%{}%{s}", s.len())
}

#[cfg(desktop)]
/// Runs off the async runtime, as the plugin's own commands do: an mpv call can block.
async fn on_mpv(
    w: WebviewWindow,
    f: impl FnOnce(&WebviewWindow) -> tauri_plugin_libmpv::Result<()> + Send + 'static,
) -> Result<(), String> {
    match tauri::async_runtime::spawn_blocking(move || f(&w)).await {
        Ok(r) => r.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// A network stream that drops before its end (the streaming engine's server, a radio station) is asked for again
/// from where it stopped, for up to a few seconds, instead of ending the song there.
pub const STREAM_LAVF: &str = "reconnect=1,reconnect_on_network_error=1,reconnect_delay_max=4";

#[cfg(desktop)]
/// Starts mpv with the page's options, once checked, plus the ones that hold a path or are fixed here.
#[tauri::command]
pub async fn mpv_init(window: WebviewWindow, mut mpv_config: MpvConfig) -> Result<String, String> {
    for (name, value) in &mpv_config.initial_options {
        check_prop(name, value)?;
    }
    let data = window.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let ytdlp = ytdlp::hook_ytdlp(window.app_handle())?;
    let raw = ytdl_raw(window.app_handle(), "");
    let o = &mut mpv_config.initial_options;
    o.insert("ytdl-raw-options".into(), json!(raw));
    // ytdl:// tracks (youtube.svelte.ts) play through our own yt-dlp (ytdlp.rs). A listening session's
    // loopback stream (session.rs) is never handed to yt-dlp, even when it fails to open.
    let hook = format!("ytdl_hook-ytdl_path={},ytdl_hook-all_formats=no,ytdl_hook-exclude=^127%.0%.0%.1[:/]", quote(&ytdlp.to_string_lossy()));
    o.insert("script-opts".into(), json!(hook));
    o.insert("watch-later-dir".into(), json!(data.join("resume").to_string_lossy()));
    o.insert("stream-lavf-o".into(), json!(STREAM_LAVF));
    // dev builds keep mpv's own log (format negotiation, device errors) next to the library
    if cfg!(debug_assertions) {
        o.insert("log-file".into(), json!(data.join("mpv.log").to_string_lossy()));
    }
    // optional: a missing Pictures folder must not stop playback from starting
    match crate::screenshot_dir(window.app_handle().clone()) {
        Ok(d) => {
            o.insert("screenshot-directory".into(), json!(d));
        }
        Err(e) => log::warn!("screenshots stay in the working folder: {e}"),
    }
    let label = window.label().to_string();
    // scripts/smoke.sh looks for "mpv started" to know a package finds libmpv
    match window.mpv().init(mpv_config, &label) {
        Ok(l) => {
            log::info!("mpv started");
            Ok(l)
        }
        Err(e) => {
            // a missing libmpv only shows as "Failed to create mpv instance": find out why, so PlaybackSetup can say
            // what to install (playbackhelp.ts). A missing wrapper is Qistara's own part and keeps its message.
            let e = e.to_string();
            let e = match libmpv_problem() {
                Some(why) if !e.contains("libmpv-wrapper") => format!("libmpv library load failed: {why}"),
                _ => e,
            };
            log::error!("mpv didn't start: {e}");
            Err(e)
        }
    }
}

#[cfg(desktop)]
/// libmpv as libmpv-wrapper looks for it: this name beside the wrapper, then by name through the system's loader.
const LIBMPV: &str = if cfg!(windows) {
    "libmpv-2.dll"
} else if cfg!(target_os = "macos") {
    "libmpv.dylib" // scripts/setup-libmpv.sh: re-exports Homebrew's libmpv
} else {
    "libmpv.so.2"
};

#[cfg(desktop)]
/// Why libmpv can't be loaded, if it can't. The wrapper sits in the exe's folder or its lib/ (the plugin looks
/// there) or, from the .deb, in /usr/lib/qistara (build.rs).
fn libmpv_problem() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let beside = [dir.to_path_buf(), dir.join("lib"), dir.join("../lib/qistara")].into_iter().map(|d| d.join(LIBMPV)).find(|p| p.exists());
    // SAFETY: only loads libmpv (its initialisers run, as when the wrapper loads it) and calls nothing in it
    let loaded = match beside {
        Some(p) => unsafe { libloading::Library::new(p) },
        None => unsafe { libloading::Library::new(LIBMPV) },
    };
    loaded.err().map(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
pub async fn mpv_command(window: WebviewWindow, name: String, args: Vec<Value>) -> Result<(), String> {
    check_command(&name, &args)?;
    on_mpv(window, move |w| w.mpv().command(&name, &args, w.label())).await
}

#[cfg(desktop)]
#[tauri::command]
pub async fn mpv_set(window: WebviewWindow, name: String, value: Value) -> Result<(), String> {
    check_prop(&name, &value)?;
    on_mpv(window, move |w| w.mpv().set_property(&name, &value, w.label())).await
}

#[cfg(desktop)]
/// YouTube playback signs in like search and downloads: as a guest, with Firefox's cookies, or with the
/// hook's checked copy of a cookies.txt export (ytdlp.rs hook_cookies). Also called after Deno is installed.
#[tauri::command]
pub async fn mpv_ytdl_sign_in(window: WebviewWindow, cookies: String) -> Result<(), String> {
    on_mpv(window, move |w| {
        let raw = ytdl_raw(w.app_handle(), &cookies);
        w.mpv().set_property("ytdl-raw-options", &json!(raw), w.label())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzz_the_allowlist_never_lets_anything_dangerous_through() {
        let pieces = [
            "loadfile", "run", "subprocess", "script-opts", "ytdl-raw-options", "@eq0:lavfi=[", "@pre:lavfi=[", "equalizer=f=", "volume=",
            "amovie=", "]", ",", ":", "=", "dB", "\\\\.\\pipe\\x", "\\\\?\\C:\\", "D:\\a.flac", "/etc/passwd", "ytdl://", "https://x",
            "av://", "edl://", "lavf://", "replace", "start=", "1", "-1", "yes", "no", "nan", "inf", "1e999", "wasapi/", "alsa/", "file:",
            "hw:CARD=x,DEV=0", " ", "%", "t=q", "eq", "af", "sid", "auto",
        ];
        let names = ["loadfile", "set", "af-command", "seek", "sub-add", "run", "subprocess", "load-script", "stop", "cycle", "playlist-remove"];
        let mut x = 0x1234_5677u32;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as usize
        };
        let mut accepted = 0;
        for _ in 0..20_000 {
            let args: Vec<Value> = (0..next() % 5).map(|_| Value::String((0..1 + next() % 4).map(|_| pieces[next() % pieces.len()]).collect())).collect();
            let name = names[next() % names.len()];
            if check_command_on(Os::Windows, name, &args).is_err() {
                continue;
            }
            accepted += 1;
            let a: Vec<String> = args.iter().filter_map(text).collect();
            assert!(!matches!(name, "run" | "subprocess" | "load-script"), "{name}");
            if name == "loadfile" {
                assert!(media(Os::Windows, &a[0]) && !a[0].to_ascii_lowercase().starts_with("av://"), "{a:?}");
            }
            if name == "sub-add" {
                assert!(local_on(Os::Windows, &a[0]), "{a:?}");
            }
            // a file name may say anything; filters and properties may not
            if matches!(name, "set" | "af-command") {
                assert!(a.iter().all(|v| !v.contains("amovie") && !v.contains("script-opts") && !v.contains("ytdl-raw-options")), "{name} {a:?}");
            }
        }
        assert!(accepted > 50, "the fuzzer should reach accepted commands too ({accepted})");
    }

    fn cmd(name: &str, args: Value) -> Result<(), String> {
        check_command_on(Os::Windows, name, args.as_array().unwrap())
    }
    fn check_prop(name: &str, value: &Value) -> Result<(), String> {
        check_prop_on(Os::Windows, name, value)
    }

    #[test]
    fn each_os_gets_its_own_bit_perfect_output_and_devices() {
        let ok = |os, name: &str, v: &str| check_prop_on(os, name, &json!(v)).is_ok();
        assert!(ok(Os::Windows, "ao", "wasapi") && !ok(Os::Windows, "ao", "coreaudio"));
        assert!(ok(Os::Mac, "ao", "coreaudio") && !ok(Os::Mac, "ao", "wasapi") && !ok(Os::Mac, "ao", "coreaudio_exclusive"));
        assert!(ok(Os::Linux, "ao", "alsa") && !ok(Os::Linux, "ao", "pulse") && !ok(Os::Linux, "ao", "pcm"));
        assert!(ok(Os::Mac, "audio-device", "coreaudio/AppleUSBAudioEngine:Chord:Mojo:14100000:1"));
        assert!(!ok(Os::Mac, "audio-device", "coreaudio/") && !ok(Os::Mac, "audio-device", "wasapi/{x}"));
        for good in ["auto", "alsa/default", "alsa/hw:CARD=PCH,DEV=0", "alsa/plughw:CARD=Mojo,DEV=0", "alsa/iec958:CARD=PCH", "alsa/hdmi:CARD=NVidia,DEV=3", "alsa/sysdefault:CARD=U192k"] {
            assert!(ok(Os::Linux, "audio-device", good), "{good}");
        }
        // ALSA plugins that write files or run things, and anything not a hardware PCM
        for bad in ["alsa/file:'/tmp/x',raw", "alsa/hw:CARD=PCH,DEV=0,x", "alsa/hw:CARD=../x", "alsa/pipe:CMD=sh", "alsa/hw:CARD=", "pulse/x", "alsa/dmix:CARD=PCH", "alsa/hw:CARD=PCH,DEV=100"] {
            assert!(!ok(Os::Linux, "audio-device", bad), "{bad}");
        }
    }

    #[test]
    fn plays_local_files_on_every_os() {
        assert!(local_on(Os::Linux, "/home/me/Music/a.flac") && local_on(Os::Mac, "/Users/me/Music/a b.flac"));
        for bad in ["/dev/snd/pcmC0D0p", "/proc/self/environ", "/sys/x", "/home/../proc/self/environ", "//server/x", "relative/a.flac", r"D:\a.flac"] {
            assert!(!local_on(Os::Linux, bad), "{bad}");
        }
        assert!(!local_on(Os::Windows, "/home/me/a.flac"));
        assert!(check_command_on(Os::Linux, "loadfile", &[json!("/home/me/a.flac"), json!("replace")]).is_ok());
        assert!(check_command_on(Os::Linux, "sub-add", &[json!("/proc/self/fd/0"), json!("select")]).is_err());
    }

    #[test]
    fn starts_sound_only_where_video_isnt_shown() {
        // player.svelte.ts: off Windows mpv starts with vid=no (os.ts SHOWS_VIDEO), so video files play their sound
        for os in [Os::Mac, Os::Linux] {
            assert!(check_prop_on(os, "vid", &json!("no")).is_ok());
        }
        assert!(check_prop("wid", &json!(0)).is_err(), "the window handle is the plugin's to set");
    }

    #[test]
    fn lets_through_what_the_app_sends() {
        // startMpv's options, as player.svelte.ts sends them
        let start = json!({
            "ao": "wasapi", "audio-exclusive": "yes", "af": "", "volume": 100, "volume-max": 100, "replaygain": "no",
            "speed": 1, "vo": "gpu-next", "hwdec": "auto-safe", "target-colorspace-hint": "yes", "video-sync": "audio",
            "idle": "yes", "force-window": "no", "keep-open": "yes", "gapless-audio": "weak", "prefetch-playlist": "yes",
            "audio-display": "no", "cover-art-auto": "no", "input-default-bindings": "no", "input-vo-keyboard": "no",
            "osc": "no", "osd-level": 0, "sub-auto": "fuzzy", "msg-level": "all=warn", "ytdl": "yes",
            "ytdl-format": "bestaudio/best", "watch-later-options": "start,aid,sid,sub-delay,audio-delay,sub-scale",
            "screenshot-format": "png", "screenshot-template": "Qistara %F %wH.%wM.%wS",
        });
        for (name, value) in start.as_object().unwrap() {
            assert_eq!(check_prop(name, value), Ok(()), "{name}");
        }
        for (name, value) in [
            ("pause", json!(true)), ("mute", json!(false)), ("ao-volume", json!(37)), ("speed", json!(1.25)),
            ("loop-file", json!("inf")), ("sub-delay", json!(-0.1)), ("audio-device", json!("wasapi/{0.0.0.00000000}.{ab-12}")),
            ("audio-spdif", json!("ac3,eac3,dts,dts-hd,truehd")), ("video-aspect-override", json!("2.35:1")),
            ("af", json!("@pre:lavfi=[volume=-2.4dB],@eq0:lavfi=[equalizer=f=100:t=q:w=0.7:g=3],@eq9:lavfi=[highpass=f=30:p=2:t=q:w=0.707]")),
        ] {
            assert_eq!(check_prop(name, &value), Ok(()), "{name} {value}");
        }
        for (name, args) in [
            ("loadfile", json!([r"D:\Music\a.flac", "replace"])),
            ("loadfile", json!([r"\\nas\music\b.flac", "append"])),
            ("loadfile", json!(["ytdl://https://www.youtube.com/watch?v=dQw4w9WgXcQ", "append-play"])),
            ("loadfile", json!(["https://stream.example/radio.mp3", "replace", -1, "start=12.5"])),
            // a YouTube song from the streaming engine's loopback server (ytstream.rs), and resumed there
            ("loadfile", json!(["http://127.0.0.1:52345/0f1e2d3c4b5a69788796a5b4c3d2e1f0/yt/dQw4w9WgXcQ", "append"])),
            ("loadfile", json!(["http://127.0.0.1:52345/0f1e2d3c4b5a69788796a5b4c3d2e1f0/yt/dQw4w9WgXcQ", "replace", -1, "start=30"])),
            ("loadfile", json!(["ytdl://https://www.youtube.com/watch?v=dQw4w9WgXcQ", "append", -1, format!("ytdl-format={VIDEO_FORMAT}")])),
            ("loadfile", json!(["ytdl://https://www.youtube.com/watch?v=dQw4w9WgXcQ", "replace", -1, format!("start=61,ytdl-format={VIDEO_FORMAT}")])),
            ("video-add", json!(["https://rr3---sn-a5mekn6s.googlevideo.com/videoplayback?expire=1&itag=137", "select"])),
            ("video-add", json!([r"D:\Videos\Song (Official Video).mkv", "select"])),
            ("video-add", json!(["http://127.0.0.1:52344/9f2c/7", "select"])),
            ("set", json!(["audio-delay", "-1.3"])), ("set", json!(["vid", "no"])),
            ("seek", json!([42.1, "absolute"])), ("set", json!(["sub-margin-y", "96"])), ("set", json!(["sid", "no"])),
            ("set", json!(["aid", "2"])), ("set", json!(["chapter", "3"])), ("sub-add", json!([r"C:\subs\a.srt", "select"])),
            ("screenshot", json!(["video"])), ("cycle", json!(["audio"])), ("af-command", json!(["pre", "volume", "-3.50dB"])),
            ("af-command", json!(["eq3", "g", "4.5"])), ("playlist-remove", json!([0])), ("playlist-clear", json!([])),
            ("stop", json!([])), ("delete-watch-later-config", json!([])),
        ] {
            assert_eq!(cmd(name, args.clone()), Ok(()), "{name} {args}");
        }
    }

    #[test]
    fn refuses_what_could_run_programs_or_write_files() {
        for (name, args) in [
            ("run", json!(["calc.exe"])), ("subprocess", json!([["calc.exe"]])), ("load-script", json!([r"C:\x.lua"])),
            ("keybind", json!(["a", "run calc"])), ("screenshot-to-file", json!([r"C:\x.png"])),
            ("dump-cache", json!([0, "no", r"C:\x.bat"])), ("set", json!(["script-opts", "ytdl_hook-ytdl_path=calc.exe"])),
            ("set", json!(["ytdl-raw-options", "exec=calc"])), ("set", json!(["stream-record", r"C:\x.bat"])),
            ("set", json!(["input-ipc-server", r"\\.\pipe\x"])),
            // ffmpeg's options are fixed in mpv_init (STREAM_LAVF): the page can't add its own
            ("set", json!(["stream-lavf-o", "reconnect=1,http_proxy=http://evil.example"])),
            ("loadfile", json!(["https://stream.example/radio.mp3", "replace", -1, "stream-lavf-o=headers=x"])),
            ("loadfile", json!([r"C:\a.flac", "replace", -1, "script-opts=ytdl_hook-ytdl_path=calc.exe"])),
            ("loadfile", json!([r"C:\a.flac", "replace", -1, "start=1,stream-record=C:\\x.bat"])),
            ("loadfile", json!(["ytdl://https://youtu.be/x", "replace", -1, "ytdl-format=bestvideo+bestaudio"])),
            ("loadfile", json!(["ytdl://https://youtu.be/x", "replace", -1, "start=1,start=2"])),
            ("loadfile", json!(["ytdl://https://youtu.be/x", "append-play", -1, "start=1"])),
            ("video-add", json!(["https://evil.example/v.mp4", "select"])), ("video-add", json!(["https://googlevideo.com.evil.example/v", "select"])),
            ("video-add", json!(["http://192.168.1.20:47800/x", "select"])), ("video-add", json!(["http://127.0.0.1/x", "select"])),
            ("video-add", json!(["edl://C:\\x", "select"])), ("video-add", json!([r"\\.\pipe\x", "select"])),
            ("video-add", json!(["https://rr3.googlevideo.com/v", "auto"])),
            ("loadfile", json!(["av://dshow:audio=Microphone", "replace"])), ("loadfile", json!([r"\\.\pipe\x", "replace"])),
            ("loadfile", json!(["ytdl://file:///C:/x", "replace"])), ("loadfile", json!(["edl://C:\\x", "replace"])),
            ("sub-add", json!(["https://evil.example/x.srt", "select"])), ("cycle", json!(["ytdl"])),
            ("af-command", json!(["pre", "volume", "1dB;x"])), ("af-command", json!(["eq0", "file", "x"])),
        ] {
            assert!(cmd(name, args.clone()).is_err(), "{name} {args}");
        }
        for (name, value) in [
            ("af", json!("lavfi=[amovie=C\\:/x.wav]")), ("af", json!("@eq0:lavfi=[ametadata=mode=print:file=C\\:/x.bat]")),
            ("af", json!("@pre:lavfi=[volume=1dB:eval=frame]")), ("audio-device", json!("pcm/x")), ("ao", json!("pcm")),
            ("vo", json!("image")), ("log-file", json!(r"C:\x.bat")), ("watch-later-dir", json!(r"C:\x")),
            ("screenshot-template", json!(r"..\..\x")), ("wid", json!(1)), ("scripts", json!(r"C:\x.lua")),
            ("pause", json!("maybe")), ("volume", json!("NaN")), ("aid", json!("1;2")), ("speed", json!(null)),
        ] {
            assert!(check_prop(name, &value).is_err(), "{name} {value}");
        }
    }
}

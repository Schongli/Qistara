//! The page reaches mpv only through these commands: the libmpv plugin's own command, set_property,
//! get_property and init aren't granted to it (capabilities/default.json). Each mpv command, property and
//! start option is checked against what the app sends, so script in the page can't run programs (`run`,
//! `subprocess`, `load-script`), point the ytdl hook at another exe (`script-opts`), hand yt-dlp options
//! (`ytdl-raw-options`) or write files (`stream-record`, `log-file`, lavfi filters). Options that hold a
//! path are set here, never by the page.

use serde_json::{json, Value};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_libmpv::{MpvConfig, MpvExt};

use crate::ytdlp;

/// yt-dlp needs a JavaScript runtime for YouTube; `mpv_ytdl_sign_in` appends the sign-in.
const YTDL_RAW: &str = "js-runtimes=node";

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
}
use Kind::*;

/// Properties and start options the page may set (player.svelte.ts and its callers), and what each may hold.
const PROPS: &[(&str, Kind)] = &[
    ("pause", Flag), ("mute", Flag), ("volume", Num), ("volume-max", Num), ("ao-volume", Num), ("speed", Num),
    ("loop-file", One(&["inf", "no"])), ("aid", Track), ("sid", Track), ("chapter", Track),
    ("sub-delay", Num), ("sub-scale", Num), ("sub-margin-y", Num), ("video-aspect-override", Aspect),
    ("panscan", Num), ("audio-device", Device), ("af", Af), ("audio-exclusive", Flag),
    ("replaygain", One(&["no", "track", "album"])), ("audio-spdif", One(&["", "ac3,eac3,dts,dts-hd,truehd"])),
    ("hwdec", One(&["auto-safe", "no"])),
    // start options (startMpv)
    ("ao", One(&["wasapi"])), ("vo", One(&["gpu-next"])), ("target-colorspace-hint", Flag),
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

/// A file on a drive or a share (not a device path like \\.\pipe\…).
fn local(p: &str) -> bool {
    let b = p.as_bytes();
    let drive = b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/');
    drive || (p.starts_with(r"\\") && !p.starts_with(r"\\?\") && !p.starts_with(r"\\.\"))
}

/// A local file, a stream or a ytdl:// web page: never mpv's other protocols (av://, lavf://, edl://, fd://…).
fn media(p: &str) -> bool {
    let l = p.to_ascii_lowercase();
    let web = |u: &str| u.starts_with("http://") || u.starts_with("https://");
    local(p)
        || web(&l)
        || ["rtsp://", "rtmp://", "mms://", "mmsh://"].iter().any(|s| l.starts_with(s))
        || l.strip_prefix("ytdl://").is_some_and(web)
}

pub fn check_prop(name: &str, value: &Value) -> Result<(), String> {
    let kind = PROPS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k);
    let ok = kind.zip(text(value)).is_some_and(|(kind, s)| match kind {
        Flag => s == "yes" || s == "no",
        Num => num(&s),
        Track => s == "no" || s == "auto" || index(&s),
        One(allowed) => allowed.contains(&s.as_str()),
        Aspect => s == "-1" || s.split_once(':').is_some_and(|(w, h)| num(w) && num(h)),
        Device => s == "auto" || s.starts_with("wasapi/"),
        Af => af(&s),
    });
    if ok { Ok(()) } else { Err(format!("mpv: {name} can't be set to {value}")) }
}

pub fn check_command(name: &str, args: &[Value]) -> Result<(), String> {
    let refuse = || format!("mpv: command {name} {} isn't allowed", Value::from(args));
    let owned = args.iter().map(text).collect::<Option<Vec<_>>>().ok_or_else(refuse)?;
    let a: Vec<&str> = owned.iter().map(String::as_str).collect();
    let ok = match (name, a.as_slice()) {
        ("set", [p, _]) => return check_prop(p, &args[1]),
        ("loadfile", [p] | [p, "replace" | "append" | "append-play"]) => media(p),
        // resume and reload (queue.svelte.ts): per-file options are only ever the start time
        ("loadfile", [p, "replace", "-1", start]) => media(p) && start.strip_prefix("start=").is_some_and(num),
        ("seek", [t, "absolute" | "relative"]) => num(t),
        ("sub-add", [p, "select"]) => local(p),
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

/// mpv's length-prefixed quoting, so a comma or '=' in a path can't split an option list.
fn quote(s: &str) -> String {
    format!("%{}%{s}", s.len())
}

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

/// Starts mpv with the page's options, once checked, plus the ones that hold a path.
#[tauri::command]
pub async fn mpv_init(window: WebviewWindow, mut mpv_config: MpvConfig) -> Result<String, String> {
    for (name, value) in &mpv_config.initial_options {
        check_prop(name, value)?;
    }
    let data = window.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let ytdlp = ytdlp::bin_dir(window.app_handle())?.join("yt-dlp.exe");
    let o = &mut mpv_config.initial_options;
    o.insert("ytdl-raw-options".into(), json!(YTDL_RAW));
    // ytdl:// tracks (youtube.svelte.ts) play through our own yt-dlp (ytdlp.rs)
    let hook = format!("ytdl_hook-ytdl_path={},ytdl_hook-all_formats=no", quote(&ytdlp.to_string_lossy()));
    o.insert("script-opts".into(), json!(hook));
    o.insert("watch-later-dir".into(), json!(data.join("resume").to_string_lossy()));
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
    window.mpv().init(mpv_config, &label).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mpv_command(window: WebviewWindow, name: String, args: Vec<Value>) -> Result<(), String> {
    check_command(&name, &args)?;
    on_mpv(window, move |w| w.mpv().command(&name, &args, w.label())).await
}

#[tauri::command]
pub async fn mpv_set(window: WebviewWindow, name: String, value: Value) -> Result<(), String> {
    check_prop(&name, &value)?;
    on_mpv(window, move |w| w.mpv().set_property(&name, &value, w.label())).await
}

/// YouTube playback signs in like search and downloads: as a guest, with Firefox's cookies, or with a
/// checked copy of a cookies.txt export (ytdlp.rs cookie_file). A file that's gone plays as a guest.
#[tauri::command]
pub async fn mpv_ytdl_sign_in(window: WebviewWindow, cookies: String) -> Result<(), String> {
    on_mpv(window, move |w| {
        let raw = match cookies.as_str() {
            "" => YTDL_RAW.to_string(),
            "firefox" => format!("{YTDL_RAW},cookies-from-browser=firefox"),
            path => match ytdlp::youtube_cookies(w.app_handle().clone(), path.into()) {
                Ok(copy) => format!("{YTDL_RAW},cookies={}", quote(&copy)),
                Err(_) => YTDL_RAW.to_string(),
            },
        };
        w.mpv().set_property("ytdl-raw-options", &json!(raw), w.label())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(name: &str, args: Value) -> Result<(), String> {
        check_command(name, args.as_array().unwrap())
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
            ("loadfile", json!([r"C:\a.flac", "replace", -1, "script-opts=ytdl_hook-ytdl_path=calc.exe"])),
            ("loadfile", json!([r"C:\a.flac", "replace", -1, "start=1,stream-record=C:\\x.bat"])),
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

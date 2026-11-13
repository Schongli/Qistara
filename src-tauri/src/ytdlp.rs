// Downloads with yt-dlp. yt-dlp and ffmpeg come from their official GitHub releases, checked against the
// SHA-256 list published with the same release, into app data\bin; an ffmpeg already on PATH is used as is.
// yt-dlp only ever runs with a fixed preset: --ignore-config (a planted config can't add --exec) and "--"
// before the URLs (a URL can't act as an option). Jobs run one at a time and report `download-progress`.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const YTDLP_SUMS: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
const FFMPEG_ZIP: &str = "ffmpeg-master-latest-win64-gpl.zip";
const FFMPEG_URL: &str = "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip";
const FFMPEG_SUMS: &str = "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/checksums.sha256";
/// GitHub serves release files from these hosts; redirects anywhere else are refused.
const RELEASE_HOSTS: [&str; 3] = ["github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com"];

pub fn bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("bin"))
}

/// Runs a tool without flashing a console window.
pub fn command(exe: &Path) -> Command {
    let mut c = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    // yt-dlp is Python: make its pipes UTF-8 whatever the console code page
    c.env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
    c
}

pub fn ytdlp(app: &AppHandle) -> Result<PathBuf, String> {
    let p = bin_dir(app)?.join("yt-dlp.exe");
    if p.is_file() { Ok(p) } else { Err("yt-dlp isn't installed yet. Get it in Downloads.".into()) }
}

/// Our own ffmpeg, else one on PATH (winget, Chocolatey...).
pub fn ffmpeg(app: &AppHandle) -> Option<PathBuf> {
    let own = bin_dir(app).ok()?.join("ffmpeg.exe");
    if own.is_file() {
        return Some(own);
    }
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join("ffmpeg.exe")).find(|p| p.is_file())
}

#[derive(Serialize)]
pub struct Tools {
    ytdlp: Option<String>,
    ffmpeg: Option<String>,
}

#[tauri::command]
pub async fn tools_status(app: AppHandle) -> Tools {
    let version = ytdlp(&app).ok().and_then(|p| command(&p).arg("--version").output().ok()).filter(|o| o.status.success());
    Tools {
        ytdlp: version.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()),
        ffmpeg: ffmpeg(&app).map(|p| p.to_string_lossy().into_owned()),
    }
}

/// The hash listed for `name` in a SHA256SUMS-style file ("<hex>  <name>" per line).
pub fn checksum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, file) = l.trim().split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hash.to_ascii_lowercase())
    })
}

fn release_client() -> Result<tauri_plugin_http::reqwest::Client, String> {
    use tauri_plugin_http::reqwest::{redirect::Policy, Client};
    Client::builder()
        .user_agent(concat!("ConfluencePlayer/", env!("CARGO_PKG_VERSION")))
        .redirect(Policy::custom(|a| if a.url().scheme() == "https" && a.url().host_str().is_some_and(|h| RELEASE_HOSTS.contains(&h)) { a.follow() } else { a.stop() }))
        .build()
        .map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
struct ToolProgress {
    tool: &'static str,
    done: u64,
    total: u64,
}

/// Streams `url` into `to` while hashing it; the file only keeps its name if the hash matches.
async fn fetch_checked(app: &AppHandle, tool: &'static str, url: &str, sums: &str, name: &str, to: &Path) -> Result<(), String> {
    let client = release_client()?;
    let list = client.get(sums).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    let want = checksum_for(&list, name).ok_or("The release has no checksum for this file.")?;
    let mut r = client.get(url).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    let total = r.content_length().unwrap_or(0);
    let tmp = to.with_extension("part");
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let (mut hash, mut done, mut last) = (Sha256::new(), 0u64, 0u64);
    while let Some(chunk) = r.chunk().await.map_err(|e| e.to_string())? {
        hash.update(&chunk);
        f.write_all(&chunk).map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        if done - last > 512 * 1024 {
            last = done;
            let _ = app.emit("tool-progress", ToolProgress { tool, done, total });
        }
    }
    drop(f);
    let got = format!("{:x}", hash.finalize());
    if got != want {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("{name} didn't match its published checksum, so it was deleted."));
    }
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Ytdlp,
    Ffmpeg,
}

#[tauri::command]
pub async fn tool_install(app: AppHandle, tool: Tool) -> Result<(), String> {
    let bin = bin_dir(&app)?;
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    match tool {
        Tool::Ytdlp => fetch_checked(&app, "ytdlp", YTDLP_URL, YTDLP_SUMS, "yt-dlp.exe", &bin.join("yt-dlp.exe")).await,
        Tool::Ffmpeg => {
            let zip = bin.join(FFMPEG_ZIP);
            fetch_checked(&app, "ffmpeg", FFMPEG_URL, FFMPEG_SUMS, FFMPEG_ZIP, &zip).await?;
            let out = bin.join("ffmpeg-unpack");
            let _ = std::fs::remove_dir_all(&out);
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            // shortcut: Windows' own tar (bsdtar, Windows 10 1803+) unpacks the zip; switch to the zip crate if it ever fails
            let ok = command(Path::new("tar")).arg("-xf").arg(&zip).arg("-C").arg(&out).status().map_err(|e| e.to_string())?.success();
            let _ = std::fs::remove_file(&zip);
            let found = ok.then(|| walkdir::WalkDir::new(&out).into_iter().flatten().find(|e| e.file_name() == "ffmpeg.exe")).flatten();
            let res = match found {
                Some(e) => {
                    let dir = e.path().parent().unwrap_or(&out).to_path_buf();
                    ["ffmpeg.exe", "ffprobe.exe"].iter().try_for_each(|n| std::fs::rename(dir.join(n), bin.join(n)).map_err(|e| e.to_string()))
                }
                None => Err("Couldn't unpack ffmpeg.".into()),
            };
            let _ = std::fs::remove_dir_all(&out);
            res
        }
    }
}

/// yt-dlp -U: YouTube changes often and old yt-dlp versions stop working.
#[tauri::command]
pub async fn ytdlp_update(app: AppHandle) -> Result<String, String> {
    let exe = ytdlp(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let o = command(&exe).arg("-U").output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or_default().to_string();
        if o.status.success() { Ok(text) } else { Err(String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("update failed").to_string()) }
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---- downloads ----

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    /// The best audio as published, no re-encode (Opus or AAC).
    AudioOriginal,
    AudioMp3,
    Video1080,
    VideoBest,
}

impl Preset {
    pub fn needs_ffmpeg(self) -> bool {
        matches!(self, Preset::AudioMp3)
    }
}

/// Every URL must be http(s); nothing else reaches yt-dlp.
pub fn valid_url(u: &str) -> bool {
    u.len() <= 2048 && url::Url::parse(u).is_ok_and(|p| matches!(p.scheme(), "http" | "https") && p.host_str().is_some())
}

/// The full argument list for one job. `ffmpeg`: the folder holding ffmpeg.exe, if any.
pub fn args(preset: Preset, dest: &Path, tmp: &Path, ffmpeg: Option<&Path>, urls: &[String]) -> Vec<String> {
    let s = |x: &str| x.to_string();
    let mut a: Vec<String> = [
        "--ignore-config", "--no-cache-dir", "--newline", "--progress", "--quiet", "--no-warnings", "--yes-playlist",
        // YouTube needs a JS runtime; deno is on by default, Node is far more often installed
        "--js-runtimes", "node",
        "--progress-template", "download:DL %(progress._percent_str)s|%(progress._eta_str)s|%(info.title)s",
        "--print", "after_move:FILE %(filepath)s",
        // a playlist gets its own folder
        "-o", "%(playlist_title&{}/|)s%(title).180B.%(ext)s",
    ]
    .map(s)
    .to_vec();
    a.extend([s("-P"), format!("home:{}", dest.display()), s("-P"), format!("temp:{}", tmp.display())]);
    let video = matches!(preset, Preset::Video1080 | Preset::VideoBest);
    a.extend(
        match (preset, ffmpeg.is_some()) {
            (Preset::AudioOriginal, true) => vec!["-f", "bestaudio/best", "-x"],
            (Preset::AudioOriginal, false) => vec!["-f", "bestaudio[ext=m4a]/bestaudio"],
            (Preset::AudioMp3, _) => vec!["-f", "bestaudio/best", "-x", "--audio-format", "mp3", "--audio-quality", "0"],
            (Preset::Video1080, true) => vec!["-f", "bv*[height<=1080]+ba/b[height<=1080]/b", "--merge-output-format", "mkv"],
            (Preset::Video1080, false) => vec!["-f", "b[height<=1080]/b"],
            (Preset::VideoBest, true) => vec!["-f", "bv*+ba/b", "--merge-output-format", "mkv"],
            (Preset::VideoBest, false) => vec!["-f", "b"],
        }
        .into_iter()
        .map(s),
    );
    if let Some(dir) = ffmpeg {
        a.extend([s("--ffmpeg-location"), dir.display().to_string(), s("--embed-metadata"), s("--embed-thumbnail")]);
        if video {
            a.extend(["--embed-subs", "--sub-langs", "all,-live_chat"].map(s));
        }
    }
    a.push(s("--"));
    a.extend(urls.iter().cloned());
    a
}

#[derive(Clone, Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: u64,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One line of yt-dlp output, as printed by the templates in `args`.
pub fn parse_progress(id: u64, line: &str) -> Option<Progress> {
    if let Some(rest) = line.strip_prefix("DL ") {
        let mut p = rest.splitn(3, '|');
        let percent = p.next()?.trim().trim_end_matches('%').parse().ok();
        let eta = p.next().map(|e| e.trim().to_string()).filter(|e| !e.is_empty() && e != "NA" && e != "Unknown");
        let title = p.next().map(|t| t.trim().to_string()).filter(|t| !t.is_empty() && t != "NA");
        return Some(Progress { id, status: "running", percent, eta, title, ..Default::default() });
    }
    let file = line.strip_prefix("FILE ")?.trim();
    Some(Progress { id, status: "running", file: Some(file.to_string()), ..Default::default() })
}

#[derive(Default)]
pub struct Downloads {
    next: AtomicU64,
    /// held while a job runs: jobs go one at a time
    slot: Mutex<()>,
    /// running children (to cancel) and cancelled ids that haven't started
    jobs: Mutex<HashMap<u64, Option<Arc<Mutex<Child>>>>>,
}

#[tauri::command]
pub fn download_add(app: AppHandle, dl: State<Downloads>, urls: Vec<String>, preset: Preset, dest: String, cookies: Option<Cookies>) -> Result<u64, String> {
    let urls: Vec<String> = urls.into_iter().map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).collect();
    if urls.is_empty() || urls.len() > 200 {
        return Err("Paste between 1 and 200 links.".into());
    }
    if let Some(bad) = urls.iter().find(|u| !valid_url(u)) {
        return Err(format!("Not a web link: {bad}"));
    }
    let dest = PathBuf::from(dest);
    if !dest.is_absolute() {
        return Err("Choose a folder to save into.".into());
    }
    std::fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let exe = ytdlp(&app)?;
    let ff = ffmpeg(&app);
    if preset.needs_ffmpeg() && ff.is_none() {
        return Err("MP3 needs ffmpeg. Get it in Downloads first.".into());
    }
    let tmp = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("download-tmp");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut argv = cookie_args(&cookies, &work_dir(&app)?)?;
    argv.extend(args(preset, &dest, &tmp, ff.as_deref().and_then(Path::parent), &urls));

    let id = dl.next.fetch_add(1, SeqCst) + 1;
    dl.jobs.lock().unwrap().insert(id, None);
    let _ = app.emit("download-progress", Progress { id, status: "queued", ..Default::default() });
    std::thread::spawn(move || {
        let dl = app.state::<Downloads>();
        let _slot = dl.slot.lock().unwrap_or_else(|e| e.into_inner());
        if !dl.jobs.lock().unwrap().contains_key(&id) {
            return; // cancelled while queued
        }
        let end = run(&app, &dl, id, &exe, &argv);
        dl.jobs.lock().unwrap().remove(&id);
        let _ = app.emit("download-progress", end);
    });
    Ok(id)
}

/// Reads one of yt-dlp's pipes line by line, reporting progress; returns its last ERROR line.
fn pump(app: AppHandle, id: u64, pipe: impl Read) -> Option<String> {
    let (mut reader, mut buf, mut error) = (BufReader::new(pipe), Vec::new(), None);
    while reader.read_until(b'\n', &mut buf).is_ok_and(|n| n > 0) {
        let line = String::from_utf8_lossy(&buf).trim_end().to_string();
        match parse_progress(id, &line) {
            Some(p) => drop(app.emit("download-progress", p)),
            None if line.starts_with("ERROR") => error = Some(line),
            None => {}
        }
        buf.clear();
    }
    error
}

/// Once per day per run of the app, before the first download: yt-dlp -U.
fn update_daily(exe: &Path) {
    static LAST: Mutex<u64> = Mutex::new(0);
    let today = crate::scan::now() as u64 / 86_400;
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if *last != today {
        *last = today;
        let _ = command(exe).arg("-U").output();
    }
}

fn run(app: &AppHandle, dl: &Downloads, id: u64, exe: &Path, argv: &[String]) -> Progress {
    let fail = |e: String| Progress { id, status: "failed", error: Some(e), ..Default::default() };
    update_daily(exe);
    let mut child = match command(exe).args(argv).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return fail(e.to_string()),
    };
    let (out, err) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let child = Arc::new(Mutex::new(child));
    dl.jobs.lock().unwrap().insert(id, Some(child.clone()));
    let _ = app.emit("download-progress", Progress { id, status: "running", ..Default::default() });
    let errors = std::thread::spawn({
        let app = app.clone();
        move || pump(app, id, err)
    });
    let out_error = pump(app.clone(), id, out);
    let status = child.lock().unwrap().wait();
    let error = errors.join().ok().flatten().or(out_error);
    let cancelled = !dl.jobs.lock().unwrap().get(&id).is_some_and(|c| c.is_some());
    match status {
        _ if cancelled => Progress { id, status: "cancelled", ..Default::default() },
        Ok(s) if s.success() => Progress { id, status: "done", ..Default::default() },
        Ok(_) => fail(error.map(|e| e.trim_start_matches("ERROR:").trim().to_string()).unwrap_or_else(|| "yt-dlp stopped with an error.".into())),
        Err(e) => fail(e.to_string()),
    }
}

#[tauri::command]
pub fn download_cancel(app: AppHandle, dl: State<Downloads>, id: u64) {
    // a running job loses its entry first, so `run` reports it as cancelled
    let entry = dl.jobs.lock().unwrap().insert(id, None);
    match entry {
        Some(Some(child)) => {
            let _ = child.lock().unwrap().kill();
        }
        _ => {
            dl.jobs.lock().unwrap().remove(&id);
            let _ = app.emit("download-progress", Progress { id, status: "cancelled", ..Default::default() });
        }
    }
}

// ---- YouTube: search and lists for the YouTube view; mpv's ytdl hook plays the results ----

/// How yt-dlp signs in to YouTube: guest (None), Firefox's cookies, or a cookies.txt export.
/// (Chrome and Edge encrypt their cookies to the browser itself on Windows, so yt-dlp can't read them.)
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Cookies {
    Firefox,
    File { path: String },
}

/// yt-dlp writes its cookie jar back into the --cookies file when it ends, so it only ever gets a copy in
/// `work` (app data), and only of a real cookies export: never a file of yours it could overwrite.
pub fn cookie_file(path: &str, work: &Path) -> Result<PathBuf, String> {
    let gone = || "The cookies file isn't there anymore. Choose it again in Providers.".to_string();
    let p = Path::new(path);
    if !path.to_ascii_lowercase().ends_with(".txt") || std::fs::metadata(p).map_err(|_| gone())?.len() > 4 * 1024 * 1024 {
        return Err(gone());
    }
    let text = std::fs::read_to_string(p).map_err(|_| gone())?;
    let first = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if !(first.starts_with("# Netscape HTTP Cookie File") || first.starts_with("# HTTP Cookie File")) {
        return Err("That file isn't a cookies.txt export (it should start with \"# Netscape HTTP Cookie File\").".into());
    }
    std::fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let copy = work.join("youtube-cookies.txt");
    std::fs::write(&copy, text).map_err(|e| e.to_string())?;
    Ok(copy)
}

pub fn cookie_args(c: &Option<Cookies>, work: &Path) -> Result<Vec<String>, String> {
    Ok(match c {
        None => vec![],
        Some(Cookies::Firefox) => vec!["--cookies-from-browser".into(), "firefox".into()],
        Some(Cookies::File { path }) => vec!["--cookies".into(), cookie_file(path, work)?.display().to_string()],
    })
}

fn work_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("download-tmp"))
}

/// The copy mpv's ytdl hook should read, for playback signed in with a cookies file.
#[tauri::command]
pub fn youtube_cookies(app: AppHandle, path: String) -> Result<String, String> {
    Ok(cookie_file(&path, &work_dir(&app)?)?.display().to_string())
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Video {
    id: String,
    title: String,
    channel: String,
    duration: Option<f64>,
}

pub fn valid_video_id(id: &str) -> bool {
    id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Videos from yt-dlp's flat JSON; channels and playlists in search results are left out.
pub fn videos(json: &serde_json::Value) -> Vec<Video> {
    let text = |v: &serde_json::Value| v.as_str().unwrap_or_default().trim().to_string();
    json["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["id"].as_str().is_some_and(valid_video_id))
        .map(|e| Video {
            id: text(&e["id"]),
            title: text(&e["title"]),
            channel: [&e["channel"], &e["uploader"]].into_iter().map(text).find(|s| !s.is_empty()).unwrap_or_default(),
            duration: e["duration"].as_f64(),
        })
        .collect()
}

async fn flat(app: AppHandle, target: String, cookies: Option<Cookies>) -> Result<Vec<Video>, String> {
    let exe = ytdlp(&app)?;
    let mut argv: Vec<String> = ["--ignore-config", "--no-cache-dir", "--no-warnings", "--js-runtimes", "node", "-J", "--flat-playlist", "--playlist-end", "500"].map(String::from).to_vec();
    argv.extend(cookie_args(&cookies, &work_dir(&app)?)?);
    argv.extend(["--".into(), target]);
    let o = tauri::async_runtime::spawn_blocking(move || command(&exe).args(&argv).stdin(Stdio::null()).output()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    if !o.status.success() {
        let err = String::from_utf8_lossy(&o.stderr);
        return Err(err.lines().rev().find(|l| l.starts_with("ERROR")).map_or("YouTube didn't answer.", |l| l).trim_start_matches("ERROR:").trim().to_string());
    }
    Ok(videos(&serde_json::from_slice(&o.stdout).map_err(|_| "Unexpected reply from yt-dlp.")?))
}

#[tauri::command]
pub async fn youtube_search(app: AppHandle, query: String, cookies: Option<Cookies>) -> Result<Vec<Video>, String> {
    let q: String = query.chars().filter(|c| !c.is_control()).collect();
    let q = q.trim();
    if q.is_empty() || q.len() > 200 {
        return Err("Type what to search for.".into());
    }
    flat(app, format!("ytsearch30:{q}"), cookies).await
}

/// A playlist, album or channel link, or "liked" for your Liked Music (needs sign-in).
#[tauri::command]
pub async fn youtube_list(app: AppHandle, url: String, cookies: Option<Cookies>) -> Result<Vec<Video>, String> {
    let url = if url == "liked" { "https://music.youtube.com/playlist?list=LM".to_string() } else { url.trim().to_string() };
    let host = url::Url::parse(&url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default();
    if !valid_url(&url) || !["youtube.com", "www.youtube.com", "music.youtube.com", "m.youtube.com", "youtu.be"].contains(&host.as_str()) {
        return Err("That isn't a YouTube link.".into());
    }
    flat(app, url, cookies).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_web_links_only() {
        assert!(valid_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(valid_url("http://example.com/a.mp3"));
        for bad in ["--exec calc", "-o x", "file:///C:/Windows/win.ini", "ftp://x.org/a", "javascript:alert(1)", "C:\\a.mp3", ""] {
            assert!(!valid_url(bad), "{bad}");
        }
    }

    #[test]
    fn builds_fixed_arguments_with_urls_after_the_separator() {
        let urls = vec!["https://a.example/x".to_string()];
        let a = args(Preset::AudioMp3, Path::new("D:\\Music"), Path::new("C:\\tmp"), Some(Path::new("C:\\bin")), &urls);
        assert_eq!(a[0], "--ignore-config");
        let sep = a.iter().position(|x| x == "--").unwrap();
        assert_eq!(&a[sep + 1..], &urls[..]);
        assert!(a.contains(&"--audio-format".to_string()) && a.contains(&"home:D:\\Music".to_string()));
        assert!(a.contains(&"--embed-thumbnail".to_string()));
        // without ffmpeg: nothing that needs it
        let b = args(Preset::Video1080, Path::new("D:\\V"), Path::new("C:\\t"), None, &urls);
        assert!(!b.iter().any(|x| x == "--merge-output-format" || x == "--embed-metadata" || x == "-x"));
    }

    #[test]
    fn reads_progress_and_finished_files() {
        let p = parse_progress(3, "DL  42.5%|00:12|Some Song").unwrap();
        assert_eq!((p.percent, p.eta.as_deref(), p.title.as_deref()), (Some(42.5), Some("00:12"), Some("Some Song")));
        let f = parse_progress(3, "FILE D:\\Music\\Some Song.opus").unwrap();
        assert_eq!(f.file.as_deref(), Some("D:\\Music\\Some Song.opus"));
        assert_eq!(parse_progress(3, "DL NA|NA|NA").unwrap().percent, None);
        assert!(parse_progress(3, "[youtube] abc: Downloading webpage").is_none());
    }

    #[test]
    fn finds_the_published_checksum() {
        let sums = "e51e69e89489f199530f13d32a4342da3ddb2cf34842f364603a066db05afbf3  yt-dlp\nF5389D3698ECDE4EF06F4E6B1CE0294FE9AC1396228C8990B8D3416D6D177A7D  yt-dlp.exe\n";
        assert_eq!(checksum_for(sums, "yt-dlp.exe").as_deref(), Some("f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d"));
        assert_eq!(checksum_for(sums, "yt-dlp.zip"), None);
        assert_eq!(checksum_for("nothex  yt-dlp.exe", "yt-dlp.exe"), None);
    }

    #[test]
    fn reads_videos_and_skips_channels() {
        let j: serde_json::Value = serde_json::from_str(r#"{"entries":[
            {"id":"jNQXAC9IVRw","title":"Me at the zoo","channel":"jawed","duration":19.0},
            {"id":"UCabcdefghijklmnopqrstuv","title":"A channel"},
            {"id":"x;rm -rf /","title":"bad"},
            {"id":"KN4kvIhbLmQ","title":"Al-Fatiha","uploader":"Alafasy"}]}"#).unwrap();
        let v = videos(&j);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], Video { id: "jNQXAC9IVRw".into(), title: "Me at the zoo".into(), channel: "jawed".into(), duration: Some(19.0) });
        assert_eq!((v[1].channel.as_str(), v[1].duration), ("Alafasy", None));
    }

    #[test]
    fn signs_in_with_firefox_or_a_copy_of_a_real_cookies_file() {
        let d = tempfile::tempdir().unwrap();
        let work = d.path().join("work");
        assert!(cookie_args(&None, &work).unwrap().is_empty());
        assert_eq!(cookie_args(&Some(Cookies::Firefox), &work).unwrap(), ["--cookies-from-browser", "firefox"]);
        assert!(cookie_args(&Some(Cookies::File { path: r"C:\nope\cookies.txt".into() }), &work).is_err());
        let f = d.path().join("cookies.txt");
        std::fs::write(&f, "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t0\tSID\tx\n").unwrap();
        let a = cookie_args(&Some(Cookies::File { path: f.to_string_lossy().into() }), &work).unwrap();
        // yt-dlp gets the copy, never the user's own file
        assert_eq!((a[0].as_str(), Path::new(&a[1])), ("--cookies", work.join("youtube-cookies.txt").as_path()));
        let notes = d.path().join("notes.txt");
        std::fs::write(&notes, "my diary").unwrap();
        assert!(cookie_args(&Some(Cookies::File { path: notes.to_string_lossy().into() }), &work).is_err());
        let exe = d.path().join("evil.exe");
        std::fs::write(&exe, "# Netscape HTTP Cookie File").unwrap();
        assert!(cookie_args(&Some(Cookies::File { path: exe.to_string_lossy().into() }), &work).is_err());
    }
}

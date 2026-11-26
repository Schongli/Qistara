// Downloads with yt-dlp. yt-dlp, ffmpeg and Deno come from their official GitHub releases, checked against the
// SHA-256 published with the same release, into app data/bin; an ffmpeg, Node or Deno already installed is used as is.
// yt-dlp only ever runs with fixed arguments: --ignore-config (a planted config can't add --exec) and "--"
// before the URLs (a URL can't act as an option). Jobs run one at a time and report `download-progress`.
// A cookies.txt sign-in reaches yt-dlp only as a copy made for that one run, deleted when the run ends.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const RELEASES: &str = "https://github.com";
/// yt-dlp's unpacked build for this OS: a folder with the program and its _internal/ libraries. It starts in a
/// fraction of the time the single-file build takes, which unpacks itself on every run (seconds on a Mac).
const YTDLP_ZIP: &str = if cfg!(windows) {
    "yt-dlp_win.zip"
} else if cfg!(target_os = "macos") {
    "yt-dlp_macos.zip"
} else if cfg!(target_arch = "aarch64") {
    "yt-dlp_linux_aarch64.zip"
} else {
    "yt-dlp_linux.zip"
};
/// The program inside that folder.
const YTDLP_PROGRAM: &str = if cfg!(windows) {
    "yt-dlp.exe"
} else if cfg!(target_os = "macos") {
    "yt-dlp_macos"
} else if cfg!(target_arch = "aarch64") {
    "yt-dlp_linux_aarch64"
} else {
    "yt-dlp_linux"
};
/// yt-dlp's FFmpeg builds: Windows and Linux. macOS uses an ffmpeg from Homebrew.
const FFMPEG_ASSET: Option<&str> = if cfg!(windows) {
    Some("ffmpeg-master-latest-win64-gpl.zip")
} else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
    Some("ffmpeg-master-latest-linuxarm64-gpl.tar.xz")
} else if cfg!(target_os = "linux") {
    Some("ffmpeg-master-latest-linux64-gpl.tar.xz")
} else {
    None
};
/// Deno, the JavaScript runtime yt-dlp needs for YouTube (its default one).
const DENO_ASSET: &str = if cfg!(windows) {
    "deno-x86_64-pc-windows-msvc.zip"
} else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    "deno-aarch64-apple-darwin.zip"
} else if cfg!(target_os = "macos") {
    "deno-x86_64-apple-darwin.zip"
} else if cfg!(target_arch = "aarch64") {
    "deno-aarch64-unknown-linux-gnu.zip"
} else {
    "deno-x86_64-unknown-linux-gnu.zip"
};
/// GitHub serves release files from these hosts; redirects anywhere else are refused.
const RELEASE_HOSTS: [&str; 3] = ["github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com"];
pub const NO_JS: &str = "YouTube needs a JavaScript runtime. Click Get Deno in Downloads, or install Node.js 20 or newer.";

/// A program's file name on this OS.
pub fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

pub fn bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("bin"))
}

/// Streaming's own corner of app data (session: sign-in copies; tmp: downloads in progress; ytdlp-cache: what
/// yt-dlp keeps between runs), kept apart from the library so it can be cleared on its own.
pub fn streaming_dir(app: &AppHandle, sub: &str) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("streaming").join(sub))
}

/// yt-dlp keeps YouTube's player code and what it worked out from it between runs, so a song or search after the
/// first doesn't fetch and solve it again: in streaming/, cleared with the streaming cache.
pub fn cache_args(app: &AppHandle) -> Result<[String; 2], String> {
    Ok(["--cache-dir".into(), streaming_dir(app, "ytdlp-cache")?.to_string_lossy().into_owned()])
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

/// A program on PATH. An app started from Finder gets launchd's PATH, which leaves out Homebrew: look there too.
pub fn on_path(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if !cfg!(windows) {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    }
    dirs.into_iter().map(|d| d.join(exe(name))).find(|p| p.is_file())
}

/// The unpacked build's folder: app data/bin/ytdlp.
fn ytdlp_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(bin_dir(app)?.join("ytdlp"))
}

/// The yt-dlp to run: the unpacked build, else a single-file one from before 0.6 (replaced at the next start).
pub fn ytdlp(app: &AppHandle) -> Result<PathBuf, String> {
    let unpacked = ytdlp_dir(app)?.join(YTDLP_PROGRAM);
    let single = bin_dir(app)?.join(exe("yt-dlp"));
    [unpacked, single].into_iter().find(|p| p.is_file()).ok_or_else(|| "yt-dlp isn't installed yet. Get it in Downloads.".into())
}

/// Where mpv's ytdl hook finds yt-dlp: the one installed, or where it will be once it is.
pub fn hook_ytdlp(app: &AppHandle) -> Result<PathBuf, String> {
    ytdlp(app).or_else(|_| Ok(ytdlp_dir(app)?.join(YTDLP_PROGRAM)))
}

/// Whether this is the unpacked build (which updates by downloading the release again; it has no -U).
fn is_unpacked(exe: &Path) -> bool {
    exe.parent().and_then(Path::file_name).is_some_and(|d| d == "ytdlp")
}

/// Our own ffmpeg, else one on PATH (winget, Chocolatey, Homebrew, the distribution's).
pub fn ffmpeg(app: &AppHandle) -> Option<PathBuf> {
    let own = bin_dir(app).ok()?.join(exe("ffmpeg"));
    if own.is_file() { Some(own) } else { on_path("ffmpeg") }
}

/// The JavaScript runtime yt-dlp gets, as `--js-runtimes` takes it: our own Deno, else Node 20+ or Deno
/// already installed. A path is always given, so a runtime outside the app's PATH is still found.
static RUNTIME: Mutex<Option<String>> = Mutex::new(None);

pub fn js_runtime(app: &AppHandle) -> Result<String, String> {
    let mut held = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = held.as_ref() {
        return Ok(r.clone());
    }
    let own = Some(bin_dir(app)?.join(exe("deno"))).filter(|p| p.is_file());
    let node = || on_path("node").filter(|n| node_major(n).is_some_and(|v| v >= 20));
    let found = own.map(|p| format!("deno:{}", p.display()))
        .or_else(|| node().map(|p| format!("node:{}", p.display())))
        .or_else(|| on_path("deno").map(|p| format!("deno:{}", p.display())))
        .ok_or_else(|| NO_JS.to_string())?;
    *held = Some(found.clone());
    Ok(found)
}

/// Forget the runtime found earlier (after Deno is installed).
fn forget_runtime() {
    *RUNTIME.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn node_major(node: &Path) -> Option<u32> {
    let o = command(node).arg("--version").output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().trim_start_matches('v').split('.').next()?.parse().ok()
}

#[derive(Serialize)]
pub struct Tools {
    ytdlp: Option<String>,
    ffmpeg: Option<String>,
    /// the JavaScript runtime YouTube uses, or None
    js: Option<String>,
    /// ffmpeg can be installed here (not on macOS: use Homebrew)
    ffmpeg_download: bool,
}

#[tauri::command]
pub async fn tools_status(app: AppHandle) -> Tools {
    let version = ytdlp(&app).ok().and_then(|p| command(&p).args(["--ignore-config", "--version"]).output().ok()).filter(|o| o.status.success());
    Tools {
        ytdlp: version.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()),
        ffmpeg: ffmpeg(&app).map(|p| p.to_string_lossy().into_owned()),
        js: js_runtime(&app).ok(),
        ffmpeg_download: FFMPEG_ASSET.is_some(),
    }
}

/// The hash listed for `name` in a SHA256SUMS-style file ("<hex>  <name>" per line).
pub fn checksum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, file) = l.trim().split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hash.to_ascii_lowercase())
    })
}

/// A checksum file for a single download: its one SHA-256, whatever the layout (Deno's Windows ones are
/// PowerShell's "Hash : …" listing). None when there isn't exactly one.
pub fn single_checksum(text: &str) -> Option<String> {
    let mut hashes = text.split(|c: char| !c.is_ascii_hexdigit()).filter(|t| t.len() == 64);
    let h = hashes.next()?;
    hashes.next().is_none().then(|| h.to_ascii_lowercase())
}

fn release_client() -> Result<tauri_plugin_http::reqwest::Client, String> {
    use tauri_plugin_http::reqwest::{redirect::Policy, Client};
    Client::builder()
        .user_agent(crate::UA)
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

/// Streams `url` into `to` while hashing it; the file only keeps its name if the hash matches the one published
/// for `name` in `sums` (a list), or the only one in it (a single-file checksum).
async fn fetch_checked(app: &AppHandle, tool: &'static str, url: &str, sums: &str, name: &str, to: &Path) -> Result<(), String> {
    let client = release_client()?;
    let list = client.get(sums).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    let want = checksum_for(&list, name).or_else(|| single_checksum(&list)).ok_or("The release has no checksum for this file.")?;
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

fn le16(b: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
}
fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// A file listed in a zip's central directory (local headers can carry zero sizes when the archive was
/// streamed, so the central directory is what counts).
struct ZipEntry {
    name: String,
    method: usize,
    crc: u32,
    packed: usize,
    size: usize,
    local: usize,
    /// Unix permission bits, when the archive was made on Unix (only applied on Unix)
    #[cfg_attr(not(unix), allow(dead_code))]
    mode: u32,
}

fn zip_entries(zip: &[u8]) -> Result<Vec<ZipEntry>, String> {
    let bad = || "The archive is damaged.".to_string();
    let eocd = (0..zip.len().saturating_sub(21)).rev().take(65_557).find(|&i| zip[i..].starts_with(b"PK\x05\x06")).ok_or_else(bad)?;
    let (count, mut at) = (le16(zip, eocd + 10).ok_or_else(bad)?, le32(zip, eocd + 16).ok_or_else(bad)? as usize);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if !zip.get(at..).is_some_and(|z| z.starts_with(b"PK\x01\x02")) {
            return Err(bad());
        }
        let (method, crc) = (le16(zip, at + 10).ok_or_else(bad)?, le32(zip, at + 16).ok_or_else(bad)?);
        let (packed, size) = (le32(zip, at + 20).ok_or_else(bad)? as usize, le32(zip, at + 24).ok_or_else(bad)? as usize);
        let (n, extra, comment) = (le16(zip, at + 28).ok_or_else(bad)?, le16(zip, at + 30).ok_or_else(bad)?, le16(zip, at + 32).ok_or_else(bad)?);
        let made_on_unix = le16(zip, at + 4).ok_or_else(bad)? >> 8 == 3;
        let mode = if made_on_unix { le32(zip, at + 38).ok_or_else(bad)? >> 16 } else { 0 };
        let local = le32(zip, at + 42).ok_or_else(bad)? as usize;
        let name = String::from_utf8_lossy(zip.get(at + 46..at + 46 + n).ok_or_else(bad)?).into_owned();
        at += 46 + n + extra + comment;
        out.push(ZipEntry { name, method, crc, packed, size, local, mode });
    }
    Ok(out)
}

/// One entry's bytes: stored or deflated, CRC-checked.
fn zip_read(zip: &[u8], e: &ZipEntry) -> Result<Vec<u8>, String> {
    let bad = || "The archive is damaged.".to_string();
    if e.size > 512 << 20 {
        return Err("The archive holds a file far bigger than expected.".into());
    }
    if !zip.get(e.local..).is_some_and(|z| z.starts_with(b"PK\x03\x04")) {
        return Err(bad());
    }
    let start = e.local + 30 + le16(zip, e.local + 26).ok_or_else(bad)? + le16(zip, e.local + 28).ok_or_else(bad)?;
    let data = zip.get(start..start.checked_add(e.packed).ok_or_else(bad)?).ok_or_else(bad)?;
    let out = match e.method {
        0 => data.to_vec(),
        8 => {
            let mut v = Vec::with_capacity(e.size);
            flate2::read::DeflateDecoder::new(data).take(e.size as u64 + 1).read_to_end(&mut v).map_err(|_| bad())?;
            v
        }
        _ => return Err("The archive uses a compression this can't read.".into()),
    };
    let mut c = flate2::Crc::new();
    c.update(&out);
    if out.len() == e.size && c.sum() == e.crc { Ok(out) } else { Err(bad()) }
}

/// One file out of a zip archive, by name (at any depth). Enough for Deno's single-file release zips.
pub fn unzip_one(zip: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let e = zip_entries(zip)?.into_iter().find(|e| e.name == name || e.name.ends_with(&format!("/{name}"))).ok_or(format!("{name} isn't in the archive."))?;
    zip_read(zip, &e)
}

/// A zip entry's path as plain relative parts: no absolute paths, drive letters, "..", "." or backslashes, so
/// nothing lands outside the folder it's unpacked into.
fn zip_path(name: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = name.split('/').filter(|p| !p.is_empty()).collect();
    let ok = !name.starts_with('/') && !parts.is_empty() && parts.iter().all(|p| *p != ".." && *p != "." && !p.contains(['\\', ':', '\0']));
    ok.then(|| parts.iter().collect())
}

/// A whole archive into `dest` (yt-dlp's unpacked builds: the program and its _internal folder). Unix permission
/// bits come from the archive; a damaged entry or an unsafe path stops it.
fn unzip_all(zip: &[u8], dest: &Path) -> Result<(), String> {
    let entries = zip_entries(zip)?;
    if entries.iter().map(|e| e.size as u64).sum::<u64>() > 1 << 30 {
        return Err("The archive is far bigger than expected.".into());
    }
    for e in &entries {
        let rel = zip_path(&e.name).ok_or("The archive holds a file with an unsafe name.")?;
        let to = dest.join(rel);
        if e.name.ends_with('/') {
            std::fs::create_dir_all(&to).map_err(|x| x.to_string())?;
            continue;
        }
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|x| x.to_string())?;
        }
        std::fs::write(&to, zip_read(zip, e)?).map_err(|x| x.to_string())?;
        #[cfg(unix)]
        if e.mode & 0o777 != 0 {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&to, std::fs::Permissions::from_mode(e.mode & 0o755)).map_err(|x| x.to_string())?;
        }
    }
    Ok(())
}

/// Unix: a downloaded program must be executable.
fn make_executable(p: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let _ = p;
    Ok(())
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Ytdlp,
    Ffmpeg,
    Deno,
}

#[tauri::command]
pub async fn tool_install(app: AppHandle, tool: Tool) -> Result<(), String> {
    let bin = bin_dir(&app)?;
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let latest = |repo: &str, asset: &str| format!("{RELEASES}/{repo}/releases/latest/download/{asset}");
    match tool {
        Tool::Ytdlp => {
            let zip = bin.join(YTDLP_ZIP);
            fetch_checked(&app, "ytdlp", &latest("yt-dlp/yt-dlp", YTDLP_ZIP), &latest("yt-dlp/yt-dlp", "SHA2-256SUMS"), YTDLP_ZIP, &zip).await?;
            let bytes = std::fs::read(&zip).map_err(|e| e.to_string());
            let _ = std::fs::remove_file(&zip);
            let bytes = bytes?;
            tauri::async_runtime::spawn_blocking(move || install_unpacked(&bin, &bytes)).await.map_err(|e| e.to_string())?
        }
        Tool::Deno => {
            let zip = bin.join(DENO_ASSET);
            let sums = latest("denoland/deno", &format!("{DENO_ASSET}.sha256sum"));
            fetch_checked(&app, "deno", &latest("denoland/deno", DENO_ASSET), &sums, DENO_ASSET, &zip).await?;
            let bytes = std::fs::read(&zip).map_err(|e| e.to_string());
            let _ = std::fs::remove_file(&zip);
            let to = bin.join(exe("deno"));
            std::fs::write(&to, unzip_one(&bytes?, &exe("deno"))?).map_err(|e| e.to_string())?;
            make_executable(&to)?;
            forget_runtime();
            Ok(())
        }
        Tool::Ffmpeg => {
            let asset = FFMPEG_ASSET.ok_or("On macOS, install ffmpeg with Homebrew: brew install ffmpeg")?;
            let archive = bin.join(asset);
            let repo = "yt-dlp/FFmpeg-Builds";
            fetch_checked(&app, "ffmpeg", &latest(repo, asset), &latest(repo, "checksums.sha256"), asset, &archive).await?;
            let out = bin.join("ffmpeg-unpack");
            let _ = std::fs::remove_dir_all(&out);
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            // shortcut: the system's tar unpacks it (Windows 10 1803+ has bsdtar, which reads zip; GNU tar reads .tar.xz);
            // switch to a zip/xz crate if it ever fails
            let ok = command(Path::new("tar")).arg("-xf").arg(&archive).arg("-C").arg(&out).status().map_err(|e| e.to_string())?.success();
            let _ = std::fs::remove_file(&archive);
            let found = ok.then(|| walkdir::WalkDir::new(&out).into_iter().flatten().find(|e| e.file_name().to_string_lossy() == exe("ffmpeg"))).flatten();
            let res = match found {
                Some(e) => {
                    let dir = e.path().parent().unwrap_or(&out).to_path_buf();
                    [exe("ffmpeg"), exe("ffprobe")].iter().try_for_each(|n| {
                        std::fs::rename(dir.join(n), bin.join(n)).map_err(|e| e.to_string())?;
                        make_executable(&bin.join(n))
                    })
                }
                None => Err("Couldn't unpack ffmpeg.".into()),
            };
            let _ = std::fs::remove_dir_all(&out);
            res
        }
    }
}

/// The unpacked build into bin/ytdlp, swapped in whole: unpacked beside it first, and the single-file build it
/// replaces removed. The program must be in it.
fn install_unpacked(bin: &Path, zip: &[u8]) -> Result<(), String> {
    let (dir, part, old) = (bin.join("ytdlp"), bin.join("ytdlp.part"), bin.join("ytdlp.old"));
    let _ = std::fs::remove_dir_all(&part);
    if let Err(e) = unzip_all(zip, &part).and_then(|_| {
        if part.join(YTDLP_PROGRAM).is_file() { make_executable(&part.join(YTDLP_PROGRAM)) } else { Err("The download didn't hold yt-dlp.".into()) }
    }) {
        let _ = std::fs::remove_dir_all(&part);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&old);
    if dir.exists() {
        std::fs::rename(&dir, &old).map_err(|_| "yt-dlp is busy (a download or a song is using it). Try again in a moment.".to_string())?;
    }
    std::fs::rename(&part, &dir).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&old);
    let _ = std::fs::remove_file(bin.join(exe("yt-dlp")));
    let _ = std::fs::write(dir.join(".checked"), b"");
    Ok(())
}

fn version(exe: &Path) -> Option<String> {
    let o = command(exe).args(["--ignore-config", "--version"]).output().ok().filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// The newest yt-dlp release's tag ("2026.11.20"), from where GitHub's "latest" link points.
async fn latest_ytdlp() -> Result<String, String> {
    use tauri_plugin_http::reqwest::{redirect::Policy, Client};
    let client = Client::builder().user_agent(crate::UA).redirect(Policy::none()).timeout(Duration::from_secs(15)).build().map_err(|e| e.to_string())?;
    let r = client.get(format!("{RELEASES}/yt-dlp/yt-dlp/releases/latest")).send().await.map_err(|e| e.to_string())?;
    let to = r.headers().get("location").and_then(|l| l.to_str().ok()).unwrap_or_default();
    release_tag(to).ok_or_else(|| "Couldn't find yt-dlp's latest release.".into())
}

/// The tag at the end of a GitHub release link, when it looks like one of yt-dlp's (digits and dots).
fn release_tag(link: &str) -> Option<String> {
    let tag = link.strip_prefix("https://github.com/yt-dlp/yt-dlp/releases/tag/")?;
    (!tag.is_empty() && tag.len() <= 20 && tag.bytes().all(|b| b.is_ascii_digit() || b == b'.')).then(|| tag.to_string())
}

/// Downloads › Check for Update: YouTube changes often and old yt-dlp versions stop working. The newest release is
/// downloaded and checked again, as at install (the unpacked build has no -U).
#[tauri::command]
pub async fn ytdlp_update(app: AppHandle) -> Result<String, String> {
    let exe = ytdlp(&app)?;
    let latest = latest_ytdlp().await?;
    let current = tauri::async_runtime::spawn_blocking({
        let exe = exe.clone();
        move || version(&exe)
    })
    .await
    .map_err(|e| e.to_string())?;
    if is_unpacked(&exe) && current.as_deref() == Some(latest.as_str()) {
        return Ok(format!("yt-dlp is up to date ({latest})."));
    }
    tool_install(app, Tool::Ytdlp).await?;
    Ok(format!("Updated yt-dlp to {latest}."))
}

/// At start: a single-file yt-dlp from before 0.6 is replaced by the unpacked build, and the unpacked build is
/// brought up to date once a day. Quietly: if it fails, the yt-dlp there keeps working.
pub fn maintain(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(exe) = ytdlp(&app) else { return };
        let marker = exe.with_file_name(".checked");
        let stale = !is_unpacked(&exe) || marker.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_none_or(|age| age > Duration::from_secs(86_400));
        if !stale {
            return;
        }
        match ytdlp_update(app).await {
            Ok(m) => log::info!("{m}"),
            Err(e) => log::warn!("yt-dlp update: {e}"),
        }
        let _ = std::fs::write(&marker, b"");
    });
}

// ---- downloads ----

/// What a download keeps: video up to a height, or audio as published or converted. Every choice maps to
/// fixed yt-dlp arguments in `args`, so nothing the page sends becomes an option of its own.
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(tag = "kind", content = "quality", rename_all = "lowercase")]
pub enum Choice {
    Video(VideoQ),
    Audio(AudioQ),
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
pub enum VideoQ {
    #[serde(rename = "best")]
    Best,
    #[serde(rename = "2160")]
    P2160,
    #[serde(rename = "1440")]
    P1440,
    #[serde(rename = "1080")]
    P1080,
    #[serde(rename = "720")]
    P720,
    #[serde(rename = "480")]
    P480,
    #[serde(rename = "360")]
    P360,
    #[serde(rename = "240")]
    P240,
}

impl VideoQ {
    pub const ALL: [VideoQ; 8] = [VideoQ::Best, VideoQ::P2160, VideoQ::P1440, VideoQ::P1080, VideoQ::P720, VideoQ::P480, VideoQ::P360, VideoQ::P240];
    pub fn height(self) -> Option<u32> {
        Some(match self {
            VideoQ::Best => return None,
            VideoQ::P2160 => 2160,
            VideoQ::P1440 => 1440,
            VideoQ::P1080 => 1080,
            VideoQ::P720 => 720,
            VideoQ::P480 => 480,
            VideoQ::P360 => 360,
            VideoQ::P240 => 240,
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
pub enum AudioQ {
    /// The best audio as published, no re-encode (Opus or AAC).
    #[serde(rename = "original")]
    Original,
    /// AAC as published, for players that don't read Opus.
    #[serde(rename = "m4a")]
    M4a,
    #[serde(rename = "mp3-320")]
    Mp3_320,
    #[serde(rename = "mp3-192")]
    Mp3_192,
    #[serde(rename = "mp3-128")]
    Mp3_128,
}

impl AudioQ {
    pub const ALL: [AudioQ; 5] = [AudioQ::Original, AudioQ::M4a, AudioQ::Mp3_320, AudioQ::Mp3_192, AudioQ::Mp3_128];
    fn mp3_kbps(self) -> Option<u32> {
        match self {
            AudioQ::Mp3_320 => Some(320),
            AudioQ::Mp3_192 => Some(192),
            AudioQ::Mp3_128 => Some(128),
            _ => None,
        }
    }
}

impl Choice {
    pub fn needs_ffmpeg(self) -> bool {
        matches!(self, Choice::Audio(q) if q.mp3_kbps().is_some())
    }
}

/// Every URL must be http(s); nothing else reaches yt-dlp.
pub fn valid_url(u: &str) -> bool {
    u.len() <= 2048 && url::Url::parse(u).is_ok_and(|p| matches!(p.scheme(), "http" | "https") && p.host_str().is_some())
}

/// The full argument list for one job. `ffmpeg`: the folder holding ffmpeg, if any; `js`: as js_runtime gives it.
pub fn args(choice: Choice, dest: &Path, tmp: &Path, cache: &Path, ffmpeg: Option<&Path>, js: Option<&str>, urls: &[String]) -> Vec<String> {
    let s = |x: &str| x.to_string();
    let mut a: Vec<String> = [
        "--ignore-config", "--newline", "--progress", "--quiet", "--no-warnings", "--yes-playlist",
        "--progress-template", "download:DL %(progress._percent_str)s|%(progress._eta_str)s|%(info.title)s",
        "--print", "after_move:FILE %(filepath)s",
        // a playlist gets its own folder
        "-o", "%(playlist_title&{}/|)s%(title).180B.%(ext)s",
    ]
    .map(s)
    .to_vec();
    a.splice(1..1, [s("--cache-dir"), cache.display().to_string()]);
    if let Some(js) = js {
        a.extend([s("--js-runtimes"), s(js)]);
    }
    a.extend([s("-P"), format!("home:{}", dest.display()), s("-P"), format!("temp:{}", tmp.display())]);
    let ff = ffmpeg.is_some();
    match choice {
        Choice::Video(q) => {
            let cap = q.height().map(|h| format!("[height<={h}]")).unwrap_or_default();
            if ff {
                a.extend([s("-f"), format!("bv*{cap}+ba/b{cap}/b"), s("--merge-output-format"), s("mkv")]);
            } else {
                // without ffmpeg only files that already hold sound
                a.extend([s("-f"), format!("b{cap}/b")]);
            }
        }
        Choice::Audio(AudioQ::Original) if ff => a.extend(["-f", "bestaudio/best", "-x"].map(s)),
        Choice::Audio(AudioQ::Original | AudioQ::M4a) => a.extend(["-f", "bestaudio[ext=m4a]/bestaudio"].map(s)),
        Choice::Audio(q) => {
            let kbps = q.mp3_kbps().unwrap_or(320);
            a.extend([s("-f"), s("bestaudio/best"), s("-x"), s("--audio-format"), s("mp3"), s("--audio-quality"), format!("{kbps}K")]);
        }
    }
    if let Some(dir) = ffmpeg {
        a.extend([s("--ffmpeg-location"), dir.display().to_string(), s("--embed-metadata"), s("--embed-thumbnail")]);
        if matches!(choice, Choice::Video(_)) {
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

impl Downloads {
    /// A job is queued or running (its partial files are in streaming/tmp).
    pub fn busy(&self) -> bool {
        !self.jobs.lock().unwrap().is_empty()
    }
}

#[tauri::command]
pub fn download_add(app: AppHandle, dl: State<Downloads>, urls: Vec<String>, choice: Choice, dest: String, cookies: Option<Cookies>) -> Result<u64, String> {
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
    if choice.needs_ffmpeg() && ff.is_none() {
        return Err("MP3 needs ffmpeg. Get it in Downloads first.".into());
    }
    let tmp = streaming_dir(&app, "tmp")?;
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    // the sign-in copy lives exactly as long as this job (moved into its thread below)
    let (mut argv, copy) = cookie_args(&cookies, &streaming_dir(&app, "session")?)?;
    // without a runtime yt-dlp still handles other sites, and says so for YouTube
    let js = js_runtime(&app).ok();
    let cache = streaming_dir(&app, "ytdlp-cache")?;
    argv.extend(args(choice, &dest, &tmp, &cache, ff.as_deref().and_then(Path::parent), js.as_deref(), &urls));

    let id = dl.next.fetch_add(1, SeqCst) + 1;
    dl.jobs.lock().unwrap().insert(id, None);
    let _ = app.emit("download-progress", Progress { id, status: "queued", ..Default::default() });
    std::thread::spawn(move || {
        let _copy = copy;
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

/// Once per day per run of the app, before the first download: yt-dlp -U, for a single-file build from before 0.6
/// (the unpacked one is kept up to date by `maintain`).
fn update_daily(exe: &Path) {
    if is_unpacked(exe) {
        return;
    }
    static LAST: Mutex<u64> = Mutex::new(0);
    let today = crate::scan::now() as u64 / 86_400;
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if *last != today {
        *last = today;
        let _ = command(exe).args(["--ignore-config", "-U"]).output();
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

// ---- what a link offers: the quality chips' sizes in Downloads ----

#[derive(Serialize, Debug, PartialEq, Default)]
pub struct Probe {
    title: String,
    uploader: String,
    duration: Option<f64>,
    /// the tallest video height on offer
    top: Option<u32>,
    thumbnail: Option<String>,
    /// bytes per choice, as far as YouTube says (None: unknown)
    video: Vec<(VideoQ, Option<u64>)>,
    audio: Vec<(AudioQ, Option<u64>)>,
}

/// Thumbnail hosts the page may show (they're in the CSP); anything else is left out.
fn shown_thumbnail(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some_and(|h| h == "i.ytimg.com" || h.ends_with(".ytimg.com") && h.len() < 40))
}

/// Sizes per choice from yt-dlp's -J for one video: video up to each height is the best stream at or below it,
/// plus the best audio when that stream is video-only (what `bv*[height<=N]+ba` downloads).
pub fn probe(j: &serde_json::Value) -> Probe {
    let formats = j["formats"].as_array().map(Vec::as_slice).unwrap_or_default();
    let size = |f: &serde_json::Value| f["filesize"].as_u64().or_else(|| f["filesize_approx"].as_u64());
    let has = |f: &serde_json::Value, k: &str| f[k].as_str().is_some_and(|c| c != "none");
    let rate = |f: &serde_json::Value| f["tbr"].as_f64().or_else(|| f["abr"].as_f64()).unwrap_or(0.0);
    let audio_only: Vec<_> = formats.iter().filter(|f| has(f, "acodec") && !has(f, "vcodec")).collect();
    let best_audio = audio_only.iter().copied().max_by(|a, b| rate(a).total_cmp(&rate(b)));
    let best_m4a = audio_only.iter().copied().filter(|f| f["ext"] == "m4a").max_by(|a, b| rate(a).total_cmp(&rate(b)));
    let videos: Vec<_> = formats.iter().filter(|f| has(f, "vcodec") && f["height"].as_u64().is_some()).collect();
    let top = videos.iter().filter_map(|f| f["height"].as_u64()).max().map(|h| h as u32);
    let duration = j["duration"].as_f64();
    let video = VideoQ::ALL
        .iter()
        .filter(|q| q.height().is_none_or(|h| top.is_some_and(|t| h <= t)))
        .filter_map(|&q| {
            let cap = q.height().or(top)?;
            let v = videos.iter().filter(|f| f["height"].as_u64().is_some_and(|h| h as u32 <= cap)).max_by(|a, b| (a["height"].as_u64(), rate(a)).partial_cmp(&(b["height"].as_u64(), rate(b))).unwrap_or(std::cmp::Ordering::Equal))?;
            let with_audio = if has(v, "acodec") { Some(0) } else { best_audio.and_then(size) };
            Some((q, size(v).zip(with_audio).map(|(a, b)| a + b)))
        })
        .collect();
    let audio = AudioQ::ALL
        .iter()
        .map(|&q| {
            let bytes = match q.mp3_kbps() {
                Some(k) => duration.map(|d| (d * k as f64 * 1000.0 / 8.0) as u64),
                None if q == AudioQ::M4a => best_m4a.and_then(size),
                None => best_audio.and_then(size),
            };
            (q, bytes)
        })
        .collect();
    let text = |v: &serde_json::Value| v.as_str().unwrap_or_default().trim().chars().take(300).collect::<String>();
    Probe {
        title: text(&j["title"]),
        uploader: [&j["channel"], &j["uploader"]].into_iter().map(text).find(|s| !s.is_empty()).unwrap_or_default(),
        duration,
        top,
        thumbnail: j["thumbnail"].as_str().filter(|u| shown_thumbnail(u)).map(str::to_owned),
        video,
        audio,
    }
}

#[tauri::command]
pub async fn download_probe(app: AppHandle, url: String, cookies: Option<Cookies>) -> Result<Probe, String> {
    let url = url.trim().to_string();
    if !valid_url(&url) {
        return Err("That isn't a web link.".into());
    }
    let exe = ytdlp(&app)?;
    let (mut argv, copy) = cookie_args(&cookies, &streaming_dir(&app, "session")?)?;
    argv.extend(["--ignore-config", "--no-warnings", "--no-playlist", "-J"].map(String::from));
    argv.extend(cache_args(&app)?);
    if let Ok(js) = js_runtime(&app) {
        argv.extend(["--js-runtimes".into(), js]);
    }
    argv.extend(["--".into(), url]);
    let o = tauri::async_runtime::spawn_blocking(move || command(&exe).args(&argv).stdin(Stdio::null()).output()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    drop(copy);
    if !o.status.success() {
        return Err(last_error(&o.stderr, "Couldn't read that link."));
    }
    Ok(probe(&serde_json::from_slice(&o.stdout).map_err(|_| "Unexpected reply from yt-dlp.")?))
}

/// The picture a music video adds over a local song (musicvideo.svelte.ts): video only, up to 1080p, H.264 when YouTube
/// has it (every GPU decodes it). mpv opens the address yt-dlp prints with video-add (mpv.rs).
const MUSIC_VIDEO_FORMAT: &str = "bestvideo[height<=?1080][vcodec^=avc1]/bestvideo[height<=?1080]";

/// yt-dlp's arguments for a video's stream address: fixed, with the watch page after `--`.
pub fn video_url_args(id: &str, cache: &Path, js: Option<String>) -> Vec<String> {
    let mut a: Vec<String> = ["--ignore-config", "--cache-dir", &cache.to_string_lossy(), "--no-warnings", "--no-playlist", "-g", "-f", MUSIC_VIDEO_FORMAT].map(String::from).into();
    if let Some(js) = js {
        a.extend(["--js-runtimes".into(), js]);
    }
    a.extend(["--".into(), format!("https://www.youtube.com/watch?v={id}")]);
    a
}

/// The stream address of a YouTube video's picture, for a music video over a local song. Only an https address on
/// YouTube's own video host is passed on.
#[tauri::command]
pub async fn youtube_video_url(app: AppHandle, id: String, cookies: Option<Cookies>) -> Result<String, String> {
    if !valid_video_id(&id) {
        return Err("Not a YouTube video.".into());
    }
    let exe = ytdlp(&app)?;
    let (mut argv, copy) = cookie_args(&cookies, &streaming_dir(&app, "session")?)?;
    argv.extend(video_url_args(&id, &streaming_dir(&app, "ytdlp-cache")?, js_runtime(&app).ok()));
    let o = tauri::async_runtime::spawn_blocking(move || command(&exe).args(&argv).stdin(Stdio::null()).output()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    drop(copy);
    if !o.status.success() {
        return Err(last_error(&o.stderr, "Couldn't open that video."));
    }
    let url = String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or_default().trim().to_string();
    match url::Url::parse(&url) {
        Ok(u) if u.scheme() == "https" && u.host_str().is_some_and(|h| h.ends_with(".googlevideo.com")) => Ok(url),
        _ => Err("YouTube didn't give a video address Qistara can play.".into()),
    }
}

/// yt-dlp's arguments for a song's sound (the streaming engine's fallback, ytstream.rs): fixed, one JSON reply.
pub fn audio_stream_args(id: &str, cache: &Path, js: Option<String>) -> Vec<String> {
    let mut a: Vec<String> = ["--ignore-config", "--cache-dir", &cache.to_string_lossy(), "--no-warnings", "--no-playlist", "-f", "bestaudio/best", "-j"].map(String::from).into();
    if let Some(js) = js {
        a.extend(["--js-runtimes".into(), js]);
    }
    a.extend(["--".into(), format!("https://www.youtube.com/watch?v={id}")]);
    a
}

/// A song's sound as yt-dlp found it: the address (on YouTube's own video host only), the User-Agent it must be
/// fetched with, its type and size when known.
#[derive(Debug, PartialEq)]
pub struct AudioStream {
    pub url: String,
    pub user_agent: Option<String>,
    pub mime: &'static str,
    pub len: Option<u64>,
}

pub fn parse_audio_stream(j: &Value) -> Option<AudioStream> {
    let url = j["url"].as_str()?;
    let u = url::Url::parse(url).ok()?;
    if u.scheme() != "https" || !u.host_str().is_some_and(|h| h.ends_with(".googlevideo.com")) {
        return None;
    }
    let mime = match j["ext"].as_str() {
        Some("m4a" | "mp4") => "audio/mp4",
        _ => "audio/webm",
    };
    let user_agent = j["http_headers"]["User-Agent"].as_str().filter(|ua| ua.len() <= 512 && ua.chars().all(|c| (' '..='~').contains(&c))).map(String::from);
    Some(AudioStream { url: url.to_string(), user_agent, mime, len: j["filesize"].as_u64() })
}

/// A song's sound through yt-dlp, with the sign-in when there is one (age-restricted songs, Premium quality).
pub async fn audio_stream(app: &AppHandle, id: &str, cookies: &Option<Cookies>) -> Result<AudioStream, String> {
    if !valid_video_id(id) {
        return Err("Not a YouTube video.".into());
    }
    let exe = ytdlp(app)?;
    let (mut argv, copy) = cookie_args(cookies, &streaming_dir(app, "session")?)?;
    argv.extend(audio_stream_args(id, &streaming_dir(app, "ytdlp-cache")?, js_runtime(app).ok()));
    let o = tauri::async_runtime::spawn_blocking(move || command(&exe).args(&argv).stdin(Stdio::null()).output()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    drop(copy);
    if !o.status.success() {
        return Err(last_error(&o.stderr, "yt-dlp couldn't open this song."));
    }
    let j: Value = serde_json::from_slice(&o.stdout).map_err(|_| "Unexpected reply from yt-dlp.")?;
    parse_audio_stream(&j).ok_or_else(|| "yt-dlp gave an address Qistara won't fetch.".into())
}

fn last_error(stderr: &[u8], fallback: &str) -> String {
    let err = String::from_utf8_lossy(stderr);
    err.lines().rev().find(|l| l.starts_with("ERROR")).map_or(fallback, |l| l).trim_start_matches("ERROR:").trim().to_string()
}

// ---- YouTube: search and lists for the YouTube view; mpv's ytdl hook plays the results ----

/// How yt-dlp signs in to YouTube: guest (None), the account signed in in Qistara's window (account.rs),
/// Firefox's cookies, or a cookies.txt export.
/// (Chrome and Edge encrypt their cookies to the browser itself on Windows, so yt-dlp can't read them.)
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Cookies {
    Account,
    Firefox,
    File { path: String },
}

/// The text of a sign-in that is a cookies.txt: the account's (kept in streaming/, `session`'s parent), or a file.
fn sign_in_text(source: &str, session: &Path) -> Result<String, String> {
    if source == ACCOUNT {
        return crate::account::cookies_txt(session.parent().ok_or("no data folder")?);
    }
    cookie_text(source)
}

/// What the page passes mpv.rs for the account (a cookies file's path always ends in .txt).
pub const ACCOUNT: &str = "account";

/// A copy of a cookies file, deleted when dropped. yt-dlp writes its cookie jar back into the --cookies file
/// when it ends, so it only ever gets a copy of its own (owner-only on Unix), gone once the run is over.
#[derive(Debug)]
pub struct CookieCopy(PathBuf);

impl CookieCopy {
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for CookieCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Reads `path` if it's a real cookies export (never a file of yours yt-dlp could overwrite).
fn cookie_text(path: &str) -> Result<String, String> {
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
    Ok(text)
}

/// Writes a file only this user can read (on Unix; app data is per-user on Windows already).
fn write_private(to: &Path, text: &str) -> Result<(), String> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    o.open(to).and_then(|mut f| f.write_all(text.as_bytes())).map_err(|e| e.to_string())
}

/// A one-run copy of a cookies export, or of the account's cookies, in `session` (streaming/session).
pub fn cookie_file(path: &str, session: &Path) -> Result<CookieCopy, String> {
    let text = sign_in_text(path, session)?;
    std::fs::create_dir_all(session).map_err(|e| e.to_string())?;
    let mut r = [0u8; 8];
    getrandom::fill(&mut r).map_err(|e| e.to_string())?;
    let copy = CookieCopy(session.join(format!("cookies-{}.txt", r.iter().map(|b| format!("{b:02x}")).collect::<String>())));
    write_private(copy.path(), &text)?;
    Ok(copy)
}

/// yt-dlp's sign-in arguments, and the copy they point at (keep it alive until yt-dlp is done).
pub fn cookie_args(c: &Option<Cookies>, session: &Path) -> Result<(Vec<String>, Option<CookieCopy>), String> {
    Ok(match c {
        None => (vec![], None),
        Some(Cookies::Firefox) => (vec!["--cookies-from-browser".into(), "firefox".into()], None),
        Some(Cookies::Account) => {
            let copy = cookie_file(ACCOUNT, session)?;
            (vec!["--cookies".into(), copy.path().display().to_string()], Some(copy))
        }
        Some(Cookies::File { path }) => {
            let copy = cookie_file(path, session)?;
            (vec!["--cookies".into(), copy.path().display().to_string()], Some(copy))
        }
    })
}

/// The one copy that outlives a single run: mpv's ytdl hook reads it on every YouTube track while you're
/// signed in. Replaced on each sign-in; removed on signing out, on quitting and at the next start.
pub const HOOK_COOKIES: &str = "ytdl-hook-cookies.txt";

pub fn hook_cookies(app: &AppHandle, path: Option<&str>) -> Result<Option<PathBuf>, String> {
    let session = streaming_dir(app, "session")?;
    let to = session.join(HOOK_COOKIES);
    let Some(path) = path else {
        let _ = std::fs::remove_file(&to);
        return Ok(None);
    };
    let text = sign_in_text(path, &session)?;
    std::fs::create_dir_all(&session).map_err(|e| e.to_string())?;
    write_private(&to, &text)?;
    Ok(Some(to))
}

/// At start and on quit: no sign-in copy outlives the app (download-tmp/ held one before 0.4).
pub fn sweep_cookies(app: &AppHandle) {
    if let Ok(session) = streaming_dir(app, "session") {
        for e in std::fs::read_dir(&session).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if (n.starts_with("cookies-") || n == HOOK_COOKIES) && n.ends_with(".txt") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    if let Ok(dir) = app.path().app_local_data_dir() {
        let _ = std::fs::remove_file(dir.join("download-tmp").join("youtube-cookies.txt"));
    }
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
    let js = js_runtime(&app)?;
    let mut argv: Vec<String> = ["--ignore-config", "--no-warnings", "--js-runtimes", &js, "-J", "--flat-playlist", "--playlist-end", "500"].map(String::from).to_vec();
    argv.extend(cache_args(&app)?);
    let (sign_in, copy) = cookie_args(&cookies, &streaming_dir(&app, "session")?)?;
    argv.extend(sign_in);
    argv.extend(["--".into(), target]);
    let o = tauri::async_runtime::spawn_blocking(move || command(&exe).args(&argv).stdin(Stdio::null()).output()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    drop(copy);
    if !o.status.success() {
        return Err(last_error(&o.stderr, "YouTube didn't answer."));
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

/// Before a YouTube track plays: Err with the fix when YouTube can't work here yet.
#[tauri::command]
pub fn youtube_ready(app: AppHandle) -> Result<(), String> {
    ytdlp(&app)?;
    js_runtime(&app).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_for_a_music_videos_picture_with_fixed_arguments() {
        let a = video_url_args("dQw4w9WgXcQ", Path::new("/c"), Some("deno:/x/deno".into()));
        assert_eq!(&a[..6], ["--ignore-config", "--cache-dir", "/c", "--no-warnings", "--no-playlist", "-g"]);
        assert_eq!(&a[6..8], ["-f", MUSIC_VIDEO_FORMAT]);
        assert_eq!(&a[a.len() - 2..], ["--", "https://www.youtube.com/watch?v=dQw4w9WgXcQ"]);
        assert!(a.windows(2).any(|w| w == ["--js-runtimes", "deno:/x/deno"]));
        assert!(!video_url_args("dQw4w9WgXcQ", Path::new("/c"), None).contains(&"--js-runtimes".to_string()));
    }

    #[test]
    fn fuzz_progress_lines_never_panic() {
        let mut x = 99u32;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as usize
        };
        let pieces = ["DL ", "FILE ", "|", "%", "NA", "12.5", "nan", "é", "\u{0}", " ", "Unknown"];
        for _ in 0..20_000 {
            let line: String = (0..next() % 10).map(|_| pieces[next() % pieces.len()]).collect();
            if let Some(p) = parse_progress(1, &line) {
                assert!(p.percent.is_none_or(|v| !v.is_nan() || line.contains("nan")));
            }
        }
    }

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
        let mp3 = Choice::Audio(AudioQ::Mp3_192);
        let a = args(mp3, Path::new("D:\\Music"), Path::new("C:\\tmp"), Path::new("C:\\cache"), Some(Path::new("C:\\bin")), Some("deno:C:\\bin\\deno.exe"), &urls);
        assert_eq!(a[0], "--ignore-config");
        let sep = a.iter().position(|x| x == "--").unwrap();
        assert_eq!(&a[sep + 1..], &urls[..]);
        assert!(a.windows(2).any(|w| w == ["--audio-quality", "192K"]) && a.contains(&"home:D:\\Music".to_string()));
        assert!(a.windows(2).any(|w| w == ["--js-runtimes", "deno:C:\\bin\\deno.exe"]));
        assert!(a.contains(&"--embed-thumbnail".to_string()));
        // without ffmpeg: nothing that needs it
        let b = args(Choice::Video(VideoQ::P1080), Path::new("D:\\V"), Path::new("C:\\t"), Path::new("C:\\c"), None, None, &urls);
        assert!(!b.iter().any(|x| x == "--merge-output-format" || x == "--embed-metadata" || x == "-x" || x == "--js-runtimes"));
        assert!(b.windows(2).any(|w| w == ["-f", "b[height<=1080]/b"]));
    }

    #[test]
    fn every_choice_maps_to_fixed_formats() {
        let f = |c: Choice, ff: bool| {
            let a = args(c, Path::new("/d"), Path::new("/t"), Path::new("/c"), ff.then_some(Path::new("/bin")), None, &[]);
            a[a.iter().position(|x| x == "-f").unwrap() + 1].clone()
        };
        assert_eq!(f(Choice::Video(VideoQ::Best), true), "bv*+ba/b/b");
        assert_eq!(f(Choice::Video(VideoQ::P1440), true), "bv*[height<=1440]+ba/b[height<=1440]/b");
        assert_eq!(f(Choice::Video(VideoQ::P240), false), "b[height<=240]/b");
        assert_eq!(f(Choice::Audio(AudioQ::Original), true), "bestaudio/best");
        assert_eq!(f(Choice::Audio(AudioQ::Original), false), "bestaudio[ext=m4a]/bestaudio");
        assert_eq!(f(Choice::Audio(AudioQ::M4a), true), "bestaudio[ext=m4a]/bestaudio");
        assert!(Choice::Audio(AudioQ::Mp3_128).needs_ffmpeg() && !Choice::Audio(AudioQ::M4a).needs_ffmpeg());
        // what the page sends
        let c: Choice = serde_json::from_str(r#"{"kind":"video","quality":"720"}"#).unwrap();
        assert_eq!(c, Choice::Video(VideoQ::P720));
        let c: Choice = serde_json::from_str(r#"{"kind":"audio","quality":"mp3-320"}"#).unwrap();
        assert_eq!(c, Choice::Audio(AudioQ::Mp3_320));
        assert!(serde_json::from_str::<Choice>(r#"{"kind":"video","quality":"--exec calc"}"#).is_err());
    }

    #[test]
    fn sizes_each_choice_from_the_formats_on_offer() {
        let j: serde_json::Value = serde_json::from_str(r#"{"title":"A","channel":"C","duration":200,"thumbnail":"https://i.ytimg.com/vi/x/maxresdefault.jpg","formats":[
            {"format_id":"140","ext":"m4a","acodec":"mp4a","vcodec":"none","abr":128,"filesize":3000000},
            {"format_id":"251","ext":"webm","acodec":"opus","vcodec":"none","abr":160,"filesize":3500000},
            {"format_id":"137","ext":"mp4","acodec":"none","vcodec":"avc1","height":1080,"tbr":4000,"filesize":80000000},
            {"format_id":"136","ext":"mp4","acodec":"none","vcodec":"avc1","height":720,"tbr":2000,"filesize_approx":40000000},
            {"format_id":"18","ext":"mp4","acodec":"mp4a","vcodec":"avc1","height":360,"tbr":600,"filesize":15000000}]}"#).unwrap();
        let p = probe(&j);
        assert_eq!((p.title.as_str(), p.uploader.as_str(), p.top), ("A", "C", Some(1080)));
        assert!(p.thumbnail.is_some());
        // 2160 and 1440 aren't on offer, nor anything at or below 240 (it would quietly download 360p)
        assert_eq!(p.video.iter().map(|v| v.0).collect::<Vec<_>>(), [VideoQ::Best, VideoQ::P1080, VideoQ::P720, VideoQ::P480, VideoQ::P360]);
        assert_eq!(p.video[0].1, Some(83_500_000));
        assert_eq!(p.video[2].1, Some(43_500_000));
        // 360p is a stream with its own sound
        assert_eq!(p.video[4].1, Some(15_000_000));
        assert_eq!(p.audio[0], (AudioQ::Original, Some(3_500_000)));
        assert_eq!(p.audio[1], (AudioQ::M4a, Some(3_000_000)));
        assert_eq!(p.audio[2], (AudioQ::Mp3_320, Some(8_000_000)));
        // a thumbnail from anywhere else is left out (the CSP wouldn't show it anyway)
        assert_eq!(probe(&serde_json::json!({"thumbnail": "https://evil.example/x.jpg"})).thumbnail, None);
    }

    #[test]
    fn reads_single_file_checksums_in_either_layout() {
        let h = "5d41402abc4b2a76b9719d911017c592ae0b6b1d8d3c1e6f0f0f0f0f0f0f0f0f";
        assert_eq!(single_checksum(&format!("{h}  deno-x86_64-unknown-linux-gnu.zip\n")).as_deref(), Some(h));
        assert_eq!(single_checksum(&format!("\r\nAlgorithm : SHA256\r\nHash      : {}\r\nPath      : D:\\a\\deno.zip\r\n", h.to_uppercase())).as_deref(), Some(h));
        assert_eq!(single_checksum(&format!("{h}  a\n{h}  b\n")), None, "a list is not a single checksum");
        assert_eq!(single_checksum("nothing here"), None);
    }

    /// A zip as streaming zippers write it: data descriptor flag, sizes only in the central directory.
    fn zip(entries: &[(&str, &[u8], bool)]) -> Vec<u8> {
        use std::io::Write;
        let (mut out, mut central) = (Vec::new(), Vec::new());
        for (name, data, deflate) in entries {
            let packed = if *deflate {
                let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(data).unwrap();
                e.finish().unwrap()
            } else {
                data.to_vec()
            };
            let mut crc = flate2::Crc::new();
            crc.update(data);
            let method: u16 = if *deflate { 8 } else { 0 };
            let at = out.len() as u32;
            out.extend(b"PK\x03\x04");
            out.extend(20u16.to_le_bytes());
            out.extend(8u16.to_le_bytes()); // sizes follow the data
            out.extend(method.to_le_bytes());
            out.extend([0u8; 8]);
            out.extend([0u8; 8]); // zero sizes in the local header
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(name.as_bytes());
            out.extend(&packed);
            central.extend(b"PK\x01\x02");
            central.extend([20, 0, 20, 0]);
            central.extend(8u16.to_le_bytes());
            central.extend(method.to_le_bytes());
            central.extend([0u8; 4]);
            central.extend(crc.sum().to_le_bytes());
            central.extend((packed.len() as u32).to_le_bytes());
            central.extend((data.len() as u32).to_le_bytes());
            central.extend((name.len() as u16).to_le_bytes());
            central.extend([0u8; 12]);
            central.extend(at.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let cd = out.len() as u32;
        out.extend(&central);
        out.extend(b"PK\x05\x06");
        out.extend([0u8; 4]);
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((central.len() as u32).to_le_bytes());
        out.extend(cd.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out
    }

    #[test]
    fn unpacks_a_whole_build_and_refuses_unsafe_paths() {
        let d = tempfile::tempdir().unwrap();
        let z = zip(&[(YTDLP_PROGRAM, b"program", true), ("_internal/python.so", &b"lib".repeat(300), true), ("_internal/base_library.zip", b"x", false)]);
        unzip_all(&z, d.path()).unwrap();
        assert_eq!(std::fs::read(d.path().join(YTDLP_PROGRAM)).unwrap(), b"program");
        assert_eq!(std::fs::read(d.path().join("_internal").join("python.so")).unwrap(), b"lib".repeat(300));
        for bad in ["../evil", "/etc/evil", "_internal/../../evil", "C:\\evil", "a\\..\\evil", "./x"] {
            let e = tempfile::tempdir().unwrap();
            assert!(unzip_all(&zip(&[(bad, b"x", false)]), e.path()).is_err(), "{bad}");
            assert_eq!(std::fs::read_dir(e.path()).unwrap().count(), 0, "{bad}");
        }
    }

    #[test]
    fn swaps_in_the_unpacked_build_and_drops_the_single_file() {
        let bin = tempfile::tempdir().unwrap();
        let b = bin.path();
        std::fs::write(b.join(exe("yt-dlp")), b"old single file").unwrap();
        std::fs::create_dir_all(b.join("ytdlp")).unwrap();
        std::fs::write(b.join("ytdlp").join(YTDLP_PROGRAM), b"older build").unwrap();
        // a download without the program changes nothing
        assert!(install_unpacked(b, &zip(&[("_internal/x", b"x", false)])).is_err());
        assert_eq!(std::fs::read(b.join("ytdlp").join(YTDLP_PROGRAM)).unwrap(), b"older build");
        assert!(!b.join("ytdlp.part").exists());
        install_unpacked(b, &zip(&[(YTDLP_PROGRAM, b"new build", true), ("_internal/x", b"x", false)])).unwrap();
        assert_eq!(std::fs::read(b.join("ytdlp").join(YTDLP_PROGRAM)).unwrap(), b"new build");
        assert!(!b.join(exe("yt-dlp")).exists() && !b.join("ytdlp.old").exists() && b.join("ytdlp").join(".checked").exists());
        assert!(is_unpacked(&b.join("ytdlp").join(YTDLP_PROGRAM)) && !is_unpacked(&b.join(exe("yt-dlp"))));
        #[cfg(unix)]
        assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(b.join("ytdlp").join(YTDLP_PROGRAM)).unwrap().permissions()) & 0o111, 0o111);
    }

    /// QISTARA_YTDLP_ZIP=<a downloaded yt-dlp_*.zip> cargo test --lib real_unpacked_build -- --ignored
    #[test]
    #[ignore = "needs a downloaded yt-dlp release zip"]
    fn real_unpacked_build_installs_and_runs() {
        let zip = std::fs::read(std::env::var("QISTARA_YTDLP_ZIP").unwrap()).unwrap();
        let bin = tempfile::tempdir().unwrap();
        install_unpacked(bin.path(), &zip).unwrap();
        let exe = bin.path().join("ytdlp").join(YTDLP_PROGRAM);
        let started = std::time::Instant::now();
        let v = version(&exe).expect("the unpacked build runs");
        println!("yt-dlp {v}: --version in {} ms", started.elapsed().as_millis());
    }

    #[test]
    fn reads_a_songs_stream_from_yt_dlp_and_keeps_to_youtubes_host() {
        let a = audio_stream_args("dQw4w9WgXcQ", Path::new("/c"), None);
        assert_eq!(&a[a.len() - 2..], ["--", "https://www.youtube.com/watch?v=dQw4w9WgXcQ"]);
        assert!(a.windows(2).any(|w| w == ["-f", "bestaudio/best"]) && a.contains(&"-j".to_string()));
        let j = serde_json::json!({ "url": "https://rr1---sn-x.googlevideo.com/videoplayback?expire=1", "ext": "m4a", "filesize": 4000,
            "http_headers": { "User-Agent": "Mozilla/5.0 (X11)" } });
        assert_eq!(parse_audio_stream(&j), Some(AudioStream { url: j["url"].as_str().unwrap().into(), user_agent: Some("Mozilla/5.0 (X11)".into()), mime: "audio/mp4", len: Some(4000) }));
        for bad in ["http://rr1---sn-x.googlevideo.com/v", "https://evil.example/googlevideo.com", "https://googlevideo.com.evil.example/v", "file:///etc/passwd"] {
            assert_eq!(parse_audio_stream(&serde_json::json!({ "url": bad })), None, "{bad}");
        }
        // a header that could break the request line is dropped
        let j = serde_json::json!({ "url": "https://r.googlevideo.com/v", "http_headers": { "User-Agent": "a\r\nHost: evil" } });
        assert_eq!(parse_audio_stream(&j).unwrap().user_agent, None);
    }

    #[test]
    fn reads_the_latest_release_tag_from_githubs_link() {
        assert_eq!(release_tag("https://github.com/yt-dlp/yt-dlp/releases/tag/2026.11.20").as_deref(), Some("2026.11.20"));
        for bad in ["https://github.com/other/repo/releases/tag/2026.11.20", "https://github.com/yt-dlp/yt-dlp/releases/tag/v1;rm", "https://github.com/yt-dlp/yt-dlp/releases/tag/", ""] {
            assert_eq!(release_tag(bad), None, "{bad}");
        }
    }

    #[test]
    fn unzips_one_file_by_the_central_directory() {
        let deno = b"\x7fELF fake deno binary ".repeat(500);
        let z = zip(&[("README.md", b"hi", false), ("deno", &deno, true)]);
        assert_eq!(unzip_one(&z, "deno").unwrap(), deno);
        assert_eq!(unzip_one(&z, "README.md").unwrap(), b"hi");
        assert!(unzip_one(&z, "deno.exe").is_err());
        // a damaged byte fails the CRC instead of installing garbage
        let mut bad = z.clone();
        let at = bad.windows(4).position(|w| w == b"PK\x03\x04").unwrap() + 40;
        bad[at] ^= 0xff;
        assert!(unzip_one(&bad, "README.md").is_err());
        // fuzz: random corruption never panics
        let mut x = 5u32;
        for _ in 0..3000 {
            let mut c = z.clone();
            for _ in 0..4 {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                let i = x as usize % c.len();
                c[i] = (x >> 8) as u8;
            }
            let _ = unzip_one(&c, "deno");
        }
        assert!(unzip_one(b"PK\x05\x06", "deno").is_err());
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
    fn signs_in_with_firefox_or_a_copy_that_lasts_one_run() {
        let d = tempfile::tempdir().unwrap();
        let session = d.path().join("session");
        assert!(cookie_args(&None, &session).unwrap().0.is_empty());
        assert_eq!(cookie_args(&Some(Cookies::Firefox), &session).unwrap().0, ["--cookies-from-browser", "firefox"]);
        // the account signed in in Qistara's window, once it's gone (signed out elsewhere): says so, never a guest run
        assert!(cookie_args(&Some(Cookies::Account), &session).unwrap_err().contains("no longer signed in"));
        assert_eq!(serde_json::from_str::<Cookies>(r#"{"kind":"account"}"#).unwrap(), Cookies::Account);
        assert!(cookie_args(&Some(Cookies::File { path: r"C:\nope\cookies.txt".into() }), &session).is_err());
        let f = d.path().join("cookies.txt");
        std::fs::write(&f, "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t0\tSID\tx\n").unwrap();
        let (a, copy) = cookie_args(&Some(Cookies::File { path: f.to_string_lossy().into() }), &session).unwrap();
        // yt-dlp gets a copy, never the user's own file, and the copy is gone with the run
        let copied = PathBuf::from(&a[1]);
        assert_eq!(a[0], "--cookies");
        assert!(copied.starts_with(&session) && copied.is_file() && copied != f);
        #[cfg(unix)]
        assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&copied).unwrap().permissions()) & 0o777, 0o600);
        drop(copy);
        assert!(!copied.exists() && f.exists());
        let notes = d.path().join("notes.txt");
        std::fs::write(&notes, "my diary").unwrap();
        assert!(cookie_args(&Some(Cookies::File { path: notes.to_string_lossy().into() }), &session).is_err());
        let exe = d.path().join("evil.exe");
        std::fs::write(&exe, "# Netscape HTTP Cookie File").unwrap();
        assert!(cookie_args(&Some(Cookies::File { path: exe.to_string_lossy().into() }), &session).is_err());
    }
}

mod apple;
mod db;
mod discord;
mod japanese;
mod keys;
mod lastfm;
mod lyrics;
mod lyrics_online;
mod media_keys;
mod mpv;
mod quran;
mod scan;
mod session;
mod spotify;
mod subsonic;
mod tags;
mod tidal;
mod translate;
mod translate_local;
mod update;
mod user;
mod ytdlp;
mod ytmusic;

use notify_debouncer_mini::{new_debouncer, notify::RecommendedWatcher, notify::RecursiveMode, Debouncer};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// Every request identifies itself with this and nothing else (no personal data).
pub const UA: &str = concat!("Qistara/", env!("CARGO_PKG_VERSION"));
/// Data folders of the app's earlier names, newest first.
const OLD_DATA: [&str; 2] = ["com.confluenceplayer.app", "com.sonora.player"];

pub struct Lib {
  db: Mutex<rusqlite::Connection>,
  pub art: scan::Art,
  /// covers of streamed things (your server, YT Music), kept apart from the library's: app data/streaming/art
  pub stream_art: scan::Art,
  /// YT Music and YouTube covers already in stream_art, by URL (remote_art)
  remote_art: Mutex<std::collections::HashMap<String, String>>,
  scanning: AtomicBool,
  again: AtomicBool,
  watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

impl Lib {
  /// A library track's path by id (a listening session shares files only this way).
  pub fn path_of(&self, id: i64) -> Res<Option<String>> {
    db::path_of(&self.db.lock().unwrap(), id).map_err(err)
  }
}

type Res<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
  e.to_string()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Library {
  folders: Vec<String>,
  tracks: Vec<db::Track>,
  playlists: Vec<user::Playlist>,
  art_dir: String,
  stream_art_dir: String,
}

#[tauri::command]
fn library(lib: State<Lib>) -> Res<Library> {
  let c = lib.db.lock().unwrap();
  Ok(Library {
    folders: db::folders(&c).map_err(err)?,
    tracks: db::tracks(&c).map_err(err)?,
    playlists: user::playlists(&c).map_err(err)?,
    art_dir: lib.art.dir.to_string_lossy().into_owned(),
    stream_art_dir: lib.stream_art.dir.to_string_lossy().into_owned(),
  })
}

#[tauri::command]
fn add_folder(app: AppHandle, lib: State<Lib>, path: String) -> Res<()> {
  lib.db.lock().unwrap().execute("INSERT OR IGNORE INTO folders VALUES (?1)", [&path]).map_err(err)?;
  watch(&app);
  rescan(app);
  Ok(())
}

#[tauri::command]
fn remove_folder(app: AppHandle, lib: State<Lib>, path: String) -> Res<()> {
  lib.db.lock().unwrap().execute("DELETE FROM folders WHERE path = ?1", [&path]).map_err(err)?;
  watch(&app);
  rescan(app);
  Ok(())
}

#[derive(serde::Serialize)]
struct Extensions {
  audio: &'static [&'static str],
  video: &'static [&'static str],
}

#[tauri::command]
fn extensions() -> Extensions {
  Extensions { audio: scan::AUDIO, video: scan::VIDEO }
}

/// Pictures\Qistara, created on first use (mpv won't create a missing screenshot folder).
#[tauri::command]
fn screenshot_dir(app: AppHandle) -> Res<String> {
  let d = app.path().picture_dir().map_err(err)?.join("Qistara");
  std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
  Ok(d.to_string_lossy().into_owned())
}

/// Settings › Show Log: Qistara's log file (tauri-plugin-log writes `<name>.log` in the OS's log folder), or that
/// folder before anything was written.
#[tauri::command]
fn log_file(app: AppHandle) -> Res<String> {
  let dir = app.path().app_log_dir().map_err(err)?;
  let file = dir.join(format!("{}.log", app.package_info().name));
  Ok(if file.exists() { file } else { dir }.to_string_lossy().into_owned())
}

/// The playback card's Restart: a libmpv that failed to load is only looked for again by a new process.
#[tauri::command]
fn app_restart(app: AppHandle) {
  app.restart()
}

/// Lyrics found online for a library song stay with the library; those for a server song ("sub:<id>"), a
/// YouTube video or a stream go to streaming/lyrics, which Settings › Storage can clear on its own.
fn lyrics_dir(lib: &Lib, key: &str) -> std::path::PathBuf {
  if ["sub:", "session:", "ytdl://", "http://", "https://"].iter().any(|p| key.starts_with(p)) {
    lib.stream_art.dir.with_file_name("lyrics")
  } else {
    lib.art.dir.with_file_name("lyrics")
  }
}

#[tauri::command]
fn lyrics(path: String) -> Option<lyrics::Found> {
  lyrics::local(Path::new(&path))
}

/// The playing track's lyrics for notch, a separate app that reads `<app data>/notch/now-lyrics.json`.
/// Local only; written atomically so notch never reads half a file.
#[tauri::command]
fn notch_lyrics(app: AppHandle, json: String) -> Res<()> {
  share_with_notch(&app.path().app_local_data_dir().map_err(err)?.join("notch"), &json)
}

fn share_with_notch(dir: &Path, json: &str) -> Res<()> {
  // a whole long surah with word times is a few hundred KB
  if json.len() > 4 << 20 {
    return Err("lyrics for notch are too large".into());
  }
  std::fs::create_dir_all(dir).map_err(err)?;
  let tmp = dir.join("now-lyrics.json.tmp");
  std::fs::write(&tmp, json).map_err(err)?;
  std::fs::rename(&tmp, dir.join("now-lyrics.json")).map_err(err)
}

/// One online source's lyrics for a track ("" = it has none), cached in app data, never in the music folder.
/// Network trouble is an error and isn't cached, so the next play tries again.
#[tauri::command]
async fn lyrics_fetch(lib: State<'_, Lib>, source: lyrics_online::Source, key: String, song: lyrics_online::Song) -> Res<String> {
  let dir = lyrics_dir(&lib, &key);
  if let Some(text) = lyrics::cached(&key, source.name(), &dir) {
    return Ok(text);
  }
  let text = lyrics_online::fetch(source, &song).await?;
  if let Err(e) = lyrics::store(&key, source.name(), &dir, &text) {
    log::warn!("couldn't cache lyrics: {e}");
  }
  Ok(text)
}

#[tauri::command]
fn playlist_create(lib: State<Lib>, name: String, rules: Option<String>, items: Vec<i64>) -> Res<i64> {
  let mut c = lib.db.lock().unwrap();
  let id = user::create(&c, &name, rules.as_deref(), scan::now()).map_err(err)?;
  user::set_items(&mut c, id, &items).map_err(err)?;
  Ok(id)
}

#[tauri::command]
fn playlist_update(lib: State<Lib>, id: i64, name: String, rules: Option<String>) -> Res<()> {
  user::update(&lib.db.lock().unwrap(), id, &name, rules.as_deref()).map_err(err)
}

#[tauri::command]
fn playlist_items(lib: State<Lib>, id: i64, items: Vec<i64>) -> Res<()> {
  user::set_items(&mut lib.db.lock().unwrap(), id, &items).map_err(err)
}

#[tauri::command]
fn playlist_delete(lib: State<Lib>, id: i64) -> Res<()> {
  user::delete(&lib.db.lock().unwrap(), id).map_err(err)
}

#[tauri::command]
fn rate(lib: State<Lib>, id: i64, rating: u8) -> Res<()> {
  user::rate(&lib.db.lock().unwrap(), id, rating).map_err(err)
}

#[tauri::command]
fn love(lib: State<Lib>, id: i64, loved: bool) -> Res<()> {
  user::love(&lib.db.lock().unwrap(), id, loved).map_err(err)
}

#[tauri::command]
fn played(lib: State<Lib>, id: i64) -> Res<i64> {
  let at = scan::now();
  user::played(&lib.db.lock().unwrap(), id, at).map_err(err)?;
  Ok(at)
}

#[tauri::command]
fn skipped(lib: State<Lib>, id: i64) -> Res<()> {
  user::skipped(&lib.db.lock().unwrap(), id).map_err(err)
}

#[tauri::command]
fn top(lib: State<Lib>, since: i64) -> Res<Vec<(i64, i64)>> {
  user::top(&lib.db.lock().unwrap(), since).map_err(err)
}

#[derive(serde::Deserialize)]
struct FileEdit {
  path: String,
  edit: tags::Edit,
}

/// Writes tag edits file by file; returns one message per file that could not be saved.
/// Each write copies the whole file, so it runs on a worker thread, not the UI thread.
#[tauri::command]
async fn tag_write(app: AppHandle, edits: Vec<FileEdit>) -> Res<Vec<String>> {
  tauri::async_runtime::spawn_blocking(move || write_tags(&app.state::<Lib>(), &edits)).await.map_err(err)
}

fn write_tags(lib: &Lib, edits: &[FileEdit]) -> Vec<String> {
  let mut errors = Vec::new();
  for fe in edits {
    let p = Path::new(&fe.path);
    match tags::write(p, &fe.edit) {
      Ok(()) => {
        // refresh the library row now instead of waiting for the folder watcher
        if let Ok(m) = std::fs::metadata(p) {
          let mtime = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
          let t = scan::read(p, mtime, m.len() as i64, &lib.art);
          if let Err(e) = db::upsert(&lib.db.lock().unwrap(), &t) {
            errors.push(format!("{}: saved, but the library didn't update: {e}", fe.path));
          }
        }
      }
      Err(e) => errors.push(format!("{}: {e}", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())),
    }
  }
  errors
}

/// Copies bytes into the cache's downloads folder (where the webview may preview them).
fn stage(lib: &Lib, bytes: &[u8]) -> Res<String> {
  let dir = lib.art.dir.join("downloads");
  std::fs::create_dir_all(&dir).map_err(err)?;
  let mut h = std::hash::DefaultHasher::new();
  std::hash::Hash::hash(bytes, &mut h);
  let out = dir.join(format!("{:016x}.img", std::hash::Hasher::finish(&h)));
  std::fs::write(&out, bytes).map_err(err)?;
  Ok(out.to_string_lossy().into_owned())
}

#[tauri::command]
fn kv_get(lib: State<Lib>, key: String) -> Res<Option<String>> {
  db::kv_get(&lib.db.lock().unwrap(), &key).map_err(err)
}

#[tauri::command]
fn kv_set(lib: State<Lib>, key: String, value: String) -> Res<()> {
  db::kv_set(&lib.db.lock().unwrap(), &key, &value).map_err(err)
}

#[derive(Default)]
struct LastfmPending(Mutex<Option<String>>);

#[derive(serde::Serialize)]
struct LastfmStatus {
  configured: bool,
  user: Option<String>,
}

#[tauri::command]
fn lastfm_status() -> LastfmStatus {
  let c = lastfm::load();
  LastfmStatus { configured: c.as_ref().is_some_and(|c| !c.key.is_empty()), user: c.and_then(|c| c.user) }
}

#[tauri::command]
fn lastfm_set_keys(key: String, secret: String) -> Res<()> {
  if key.trim().len() != 32 || secret.trim().len() != 32 {
    return Err("A Last.fm API key and shared secret are 32 characters each.".into());
  }
  lastfm::store(&lastfm::Creds { key: key.trim().into(), secret: secret.trim().into(), ..Default::default() })
}

/// Step 1: get a token and open Last.fm in the browser for the user to approve.
#[tauri::command]
async fn lastfm_begin(app: AppHandle, pending: State<'_, LastfmPending>) -> Res<()> {
  use tauri_plugin_opener::OpenerExt;
  let c = lastfm::load().ok_or("Add your API key first.")?;
  let j = lastfm::call(&lastfm::Creds { session: None, ..c.clone() }, "auth.getToken", Default::default()).await?;
  let token = j["token"].as_str().ok_or("no token from Last.fm")?.to_string();
  let url = format!("https://www.last.fm/api/auth/?api_key={}&token={}", c.key, token);
  *pending.0.lock().unwrap() = Some(token);
  app.opener().open_url(url, None::<&str>).map_err(err)
}

/// Step 2: after approving in the browser, trade the token for a session.
#[tauri::command]
async fn lastfm_finish(pending: State<'_, LastfmPending>) -> Res<String> {
  let token = pending.0.lock().unwrap().take().ok_or("Start the connection first.")?;
  let mut c = lastfm::load().ok_or("Add your API key first.")?;
  c.session = None;
  let j = lastfm::call(&c, "auth.getSession", std::collections::BTreeMap::from([("token", token)])).await?;
  c.session = j["session"]["key"].as_str().map(str::to_string);
  c.user = j["session"]["name"].as_str().map(str::to_string);
  lastfm::store(&c)?;
  c.user.ok_or("Last.fm returned no user".into())
}

#[tauri::command]
fn lastfm_logout() -> Res<()> {
  lastfm::forget()
}

#[tauri::command]
async fn lastfm_now_playing(s: lastfm::Scrobble) -> Res<()> {
  let Some(c) = lastfm::load().filter(|c| c.session.is_some()) else { return Ok(()) };
  let mut p = lastfm::scrobble_params(&s);
  p.remove("timestamp");
  lastfm::call(&c, "track.updateNowPlaying", p).await.map(|_| ())
}

/// Scrobbles now, after first sending anything that failed earlier (offline, Last.fm down).
/// Serialises scrobbles so two calls never read and rewrite the backlog at the same time.
#[derive(Default)]
struct ScrobbleLock(tauri::async_runtime::Mutex<()>);

#[tauri::command]
async fn lastfm_scrobble(lib: State<'_, Lib>, lock: State<'_, ScrobbleLock>, s: lastfm::Scrobble) -> Res<usize> {
  let _one_at_a_time = lock.0.lock().await;
  let Some(c) = lastfm::load().filter(|c| c.session.is_some()) else { return Ok(0) };
  let mut pending: Vec<lastfm::Scrobble> = {
    let db = lib.db.lock().unwrap();
    db::kv_get(&db, "lastfm.queue").ok().flatten().and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
  };
  pending.push(s);
  let mut left = Vec::new();
  for item in pending {
    if !left.is_empty() || lastfm::call(&c, "track.scrobble", lastfm::scrobble_params(&item)).await.is_err() {
      left.push(item); // keep order: once one fails, the rest wait too
    }
  }
  // keep the newest 2800: Last.fm ignores scrobbles older than two weeks anyway
  let excess = left.len().saturating_sub(2800);
  left.drain(..excess);
  let db = lib.db.lock().unwrap();
  db::kv_set(&db, "lastfm.queue", &serde_json::to_string(&left).map_err(err)?).map_err(err)?;
  Ok(left.len())
}

/// Saves an exported playlist where the user chose. Only .m3u/.m3u8 files can be written this way.
#[tauri::command]
fn write_playlist(path: String, text: String) -> Res<()> {
  let ext = Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
  if ext != "m3u" && ext != "m3u8" {
    return Err("playlists are saved as .m3u8 files".into());
  }
  std::fs::write(&path, text.as_bytes()).map_err(err)
}

/// Reads a text file the user picked: an Equalizer APO / AutoEQ preset, or an M3U playlist (any encoding).
#[tauri::command]
fn read_text(path: String, playlist: Option<bool>) -> Res<String> {
  let (max, what) = if playlist == Some(true) { (16 << 20, "a playlist") } else { (1 << 20, "a preset") };
  let m = std::fs::metadata(&path).map_err(err)?;
  if m.len() > max {
    return Err(format!("This file is too large to be {what}."));
  }
  Ok(lyrics::decode_text(&std::fs::read(&path).map_err(err)?))
}

/// A picked image file, staged so the tag editor can preview it before embedding.
#[tauri::command]
fn stage_image(lib: State<Lib>, path: String) -> Res<String> {
  let bytes = std::fs::read(&path).map_err(err)?;
  if !matches!(image::guess_format(&bytes), Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg)) {
    return Err("choose a JPEG or PNG image".into());
  }
  stage(&lib, &bytes)
}

/// Downloads release artwork from the Cover Art Archive into the cache, for embedding.
#[tauri::command]
async fn fetch_cover(lib: State<'_, Lib>, url: String) -> Res<String> {
  let u = tauri::Url::parse(&url).map_err(err)?;
  if u.scheme() != "https" || u.host_str() != Some("coverartarchive.org") {
    return Err("only Cover Art Archive images can be downloaded".into());
  }
  let client = tauri_plugin_http::reqwest::Client::builder()
    .user_agent(UA)
    .redirect(tauri_plugin_http::reqwest::redirect::Policy::custom(|a| {
      // CAA redirects to archive.org; nothing else
      if a.url().host_str().is_some_and(|h| h == "archive.org" || h.ends_with(".archive.org")) { a.follow() } else { a.stop() }
    }))
    .build()
    .map_err(err)?;
  let r = client.get(u).send().await.map_err(err)?.error_for_status().map_err(err)?;
  let bytes = r.bytes().await.map_err(err)?;
  if bytes.len() > 20 * 1024 * 1024 {
    return Err("image is too large".into());
  }
  stage(&lib, &bytes)
}

/// A cached cover by the name the page knows it by: "<hash>.jpg" (the library's) or "stream:<hash>.jpg"
/// (streaming/art). A name only: never a path.
pub(crate) fn art_file(lib: &Lib, art: &str) -> Option<std::path::PathBuf> {
  let (stream, name) = art_name(art)?;
  Some(if stream { lib.stream_art.dir.join(name) } else { lib.art.dir.join(name) })
}

/// (is it a streaming cover, its file name), or None for anything that isn't a plain cache name.
fn art_name(art: &str) -> Option<(bool, &str)> {
  let (stream, name) = art.strip_prefix("stream:").map_or((false, art), |n| (true, n));
  (!name.is_empty() && !name.contains(['/', '\\', ':']) && !name.contains("..")).then_some((stream, name))
}

/// A YT Music or YouTube cover (https, from the thumbnail hosts only) copied into streaming/art, for the adaptive
/// theme and the OS media overlay, which can't use a URL the page loads: its "stream:<name>". Fetched once per URL.
pub(crate) async fn remote_art(lib: &Lib, url: &str) -> Option<String> {
  if !ytmusic::thumb_host(url) {
    return None;
  }
  let known = lib.remote_art.lock().unwrap().get(url).cloned();
  if let Some(name) = known.filter(|n| art_file(lib, n).is_some_and(|f| f.exists())) {
    return Some(name);
  }
  const MAX: usize = 2 * 1024 * 1024;
  let client = tauri_plugin_http::reqwest::Client::builder()
    .user_agent(UA)
    .redirect(tauri_plugin_http::reqwest::redirect::Policy::none())
    .timeout(Duration::from_secs(10))
    .build()
    .ok()?;
  let r = client.get(url).send().await.ok()?.error_for_status().ok()?;
  if r.content_length().is_some_and(|n| n as usize > MAX) {
    return None;
  }
  let bytes = r.bytes().await.ok()?;
  if bytes.len() > MAX {
    return None;
  }
  let dir = lib.stream_art.dir.clone();
  let name = tauri::async_runtime::spawn_blocking(move || scan::Art::new(dir).thumb(&bytes)).await.ok()??;
  let name = format!("stream:{name}");
  let mut m = lib.remote_art.lock().unwrap();
  if m.len() > 500 {
    m.clear();
  }
  m.insert(url.to_string(), name.clone());
  Some(name)
}

/// The cached file for a cover the page knows by its name or, for a streaming song, by its https URL.
pub(crate) async fn cover_file(lib: &Lib, art: &str) -> Option<std::path::PathBuf> {
  let name = if art.starts_with("https://") { remote_art(lib, art).await? } else { art.to_string() };
  art_file(lib, &name).filter(|f| f.exists())
}

/// Appearance › Match artwork and the adaptive theme: the vivid colour of a cover (a cached one, or a streaming
/// song's, fetched once).
#[tauri::command]
async fn art_color(lib: State<'_, Lib>, art: String) -> Res<Option<String>> {
  let Some(file) = cover_file(&lib, &art).await else { return Ok(None) };
  Ok(tauri::async_runtime::spawn_blocking(move || image::open(file).ok().and_then(|i| vivid(&i))).await.ok().flatten())
}

/// A listening session shows guests the host's cover of a library song: a 160 px JPEG small enough to send
/// with the song's details (64 KB at most).
#[tauri::command]
fn session_art(lib: State<Lib>, art: String) -> Option<String> {
  use base64::Engine;
  let img = image::open(art_file(&lib, &art)?).ok()?.thumbnail(160, 160).to_rgb8();
  let mut out = Vec::new();
  image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 75).encode_image(&img).ok()?;
  (out.len() <= 64 * 1024).then(|| format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(out)))
}

/// Settings › Storage: what each part of app data takes, in bytes.
#[derive(serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct Storage {
  library_art: u64,
  lyrics: u64,
  streaming: u64,
  tools: u64,
  dictionary: u64,
  language_packs: u64,
}

fn dir_size(p: &Path) -> u64 {
  walkdir::WalkDir::new(p).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}

#[tauri::command]
async fn storage_usage(app: AppHandle) -> Res<Storage> {
  let d = app.path().app_local_data_dir().map_err(err)?;
  tauri::async_runtime::spawn_blocking(move || Storage {
    library_art: dir_size(&d.join("art")),
    lyrics: dir_size(&d.join("lyrics")),
    streaming: dir_size(&d.join("streaming")),
    tools: dir_size(&d.join("bin")),
    dictionary: dir_size(&d.join("dict")),
    language_packs: dir_size(&d.join("translate")),
  })
  .await
  .map_err(err)
}

/// Empties streaming/: server and YT Music covers and their lyrics. The sign-in copy of a session in use and
/// a download in progress (streaming/tmp) are left alone; the library is never touched.
#[tauri::command]
fn clear_streaming_cache(app: AppHandle, lib: State<Lib>, dl: State<ytdlp::Downloads>) -> Res<()> {
  let root = lib.stream_art.dir.parent().ok_or("no data folder")?.to_path_buf();
  for sub in ["art", "lyrics"] {
    clear_dir(&root.join(sub));
  }
  if !dl.busy() {
    clear_dir(&ytdlp::streaming_dir(&app, "tmp")?);
  }
  Ok(())
}

/// Lyrics found online for library songs: the next play looks them up again.
#[tauri::command]
fn clear_lyrics_cache(lib: State<Lib>) {
  clear_dir(&lib.art.dir.with_file_name("lyrics"));
}

/// Removes what's in `dir`, keeping the folder (the asset scope and the art cache point at it).
fn clear_dir(dir: &Path) {
  for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
    let p = e.path();
    let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
  }
}

/// The cover's main colour: its colourful, mid-bright pixels are sorted into 12 hue bands and the
/// strongest band is averaged (averaging every hue would turn navy and gold into grey). None for grey art.
fn vivid(img: &image::DynamicImage) -> Option<String> {
  let small = img.thumbnail(32, 32).to_rgb8();
  let mut bands = [([0f64; 3], 0f64); 12];
  for p in small.pixels() {
    let [r, g, b] = p.0.map(f64::from);
    let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
    let (s, v) = (if hi > 0.0 { (hi - lo) / hi } else { 0.0 }, hi / 255.0);
    if s <= 0.25 || !(0.2..=0.97).contains(&v) {
      continue;
    }
    let d = hi - lo;
    let hue = if hi == r { ((g - b) / d).rem_euclid(6.0) } else if hi == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    let band = &mut bands[((hue * 2.0) as usize).min(11)];
    band.0 = [band.0[0] + r * s, band.0[1] + g * s, band.0[2] + b * s];
    band.1 += s;
  }
  let (sum, w) = bands.into_iter().max_by(|a, b| a.1.total_cmp(&b.1))?;
  // a few coloured pixels in a grey cover are noise, not its colour
  (w > 0.05 * (small.width() * small.height()) as f64)
    .then(|| format!("#{:02x}{:02x}{:02x}", (sum[0] / w) as u8, (sum[1] / w) as u8, (sum[2] / w) as u8))
}

#[tauri::command]
fn cover(lib: State<Lib>, path: String) -> Option<String> {
  lib.art.large(Path::new(&path)).map(|p| p.to_string_lossy().into_owned())
}

/// Runs a scan on a worker thread; calls made meanwhile queue exactly one more pass.
#[tauri::command]
fn rescan(app: AppHandle) {
  let lib = app.state::<Lib>();
  // announce the request first, so a worker finishing right now still sees it
  lib.again.store(true, SeqCst);
  if lib.scanning.swap(true, SeqCst) {
    return;
  }
  std::thread::spawn(move || {
    let lib = app.state::<Lib>();
    /// If anything below panics, later rescans must still run.
    struct Unstick<'a>(&'a AtomicBool);
    impl Drop for Unstick<'_> {
      fn drop(&mut self) {
        if std::thread::panicking() {
          self.0.store(false, SeqCst);
        }
      }
    }
    let _unstick = Unstick(&lib.scanning);
    loop {
      lib.again.store(false, SeqCst);
      match scan::scan(&lib.db, &lib.art, &|done, total| {
        let _ = app.emit("scan-progress", (done, total));
      }) {
        Ok(stats) => {
          let _ = app.emit("library-changed", stats);
        }
        Err(e) => {
          log::error!("scan failed: {e}");
          let _ = app.emit("scan-error", e.to_string());
        }
      }
      if !lib.again.load(SeqCst) {
        lib.scanning.store(false, SeqCst);
        // a request that landed between the check and the store would otherwise be lost
        if !(lib.again.load(SeqCst) && !lib.scanning.swap(true, SeqCst)) {
          break;
        }
      }
    }
  });
}

fn watch(app: &AppHandle) {
  let lib = app.state::<Lib>();
  let folders = db::folders(&lib.db.lock().unwrap()).unwrap_or_default();
  let h = app.clone();
  match new_debouncer(Duration::from_secs(3), move |res: notify_debouncer_mini::DebounceEventResult| {
    if res.is_ok() {
      rescan(h.clone());
    }
  }) {
    Ok(mut d) => {
      for f in &folders {
        if let Err(e) = d.watcher().watch(Path::new(f), RecursiveMode::Recursive) {
          log::warn!("cannot watch {f}: {e}");
        }
      }
      *lib.watcher.lock().unwrap() = Some(d);
    }
    Err(e) => log::warn!("folder watcher unavailable: {e}"),
  }
}

/// The app was called Confluence Player (com.confluenceplayer.app) and before that Sonora (com.sonora.player):
/// copy the newest old library, settings (kv) and caches over once. Copies, never moves, so the old install keeps working. The database goes last: once it is
/// there, the copy is done.
fn migrate(old: &Path, new: &Path) -> std::io::Result<bool> {
  if new.join("library.db").exists() || !old.join("library.db").exists() {
    return Ok(false);
  }
  for d in ["art", "lyrics", "resume", "quran-sync"] {
    copy_dir(&old.join(d), &new.join(d))?;
  }
  for f in ["library.db-wal", "library.db-shm", "library.db"] {
    if old.join(f).exists() {
      std::fs::copy(old.join(f), new.join(f))?;
    }
  }
  Ok(true)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
  if !from.is_dir() {
    return Ok(());
  }
  std::fs::create_dir_all(to)?;
  for e in std::fs::read_dir(from)? {
    let e = e?;
    if e.file_type()?.is_dir() {
      copy_dir(&e.path(), &to.join(e.file_name()))?;
    } else {
      std::fs::copy(e.path(), to.join(e.file_name()))?;
    }
  }
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let builder = tauri::Builder::default();
  // playback: libmpv on desktop; phones don't have it yet (mpv.rs answers for it). Updates: desktop too (update.rs)
  #[cfg(desktop)]
  let builder = builder.plugin(tauri_plugin_libmpv::init()).plugin(tauri_plugin_updater::Builder::new().build());
  builder
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_opener::init())
    .manage(LastfmPending::default())
    .manage(ScrobbleLock::default())
    .manage(spotify::Token::default())
    .manage(ytdlp::Downloads::default())
    .manage(tidal::TidalToken::default())
    .manage(session::Session::default())
    .manage(discord::Discord::default())
    .manage(update::Pending::default())
    .setup(|app| {
      // a small log file in the OS's log folder (Settings › General › Updates › Show Log), for bug reports and
      // the packaging smoke test (scripts/smoke.sh). It stays on this computer.
      app.handle().plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
      let dir = app.path().app_local_data_dir()?;
      // only the newest old library: if copying it fails, the next start tries it again (never an older one)
      if let Some(old) = OLD_DATA.iter().filter_map(|n| dir.parent().map(|p| p.join(n))).find(|o| o.join("library.db").exists()) {
        match migrate(&old, &dir) {
          Ok(true) => log::info!("copied the library from {}", old.display()),
          Ok(false) => {}
          Err(e) => log::warn!("couldn't copy the library from {}: {e}", old.display()),
        }
      }
      std::fs::create_dir_all(&dir)?;
      // the webview may load artwork from the cache only (the $APPLOCALDATA glob didn't match on Windows)
      let (art, stream_art) = (dir.join("art"), dir.join("streaming").join("art"));
      for d in [&art, &stream_art] {
        std::fs::create_dir_all(d)?;
        app.asset_protocol_scope().allow_directory(d, true)?;
        // Tauri checks canonical paths, and under folder redirection (MSIX app containers) the files we
        // write can resolve somewhere other than the folder itself: also allow where a written file lands
        let probe = d.join(".scope");
        if let Some(real) = std::fs::write(&probe, b"").ok().and_then(|_| std::fs::canonicalize(&probe).ok()).and_then(|p| p.parent().map(Path::to_path_buf)) {
          app.asset_protocol_scope().allow_directory(real, true)?;
        }
      }
      app.manage(Lib {
        db: Mutex::new(db::open(&dir.join("library.db"))?),
        art: scan::Art::new(art),
        stream_art: scan::Art::new(stream_art),
        remote_art: Mutex::default(),
        scanning: AtomicBool::new(false),
        again: AtomicBool::new(false),
        watcher: Mutex::new(None),
      });
      ytdlp::sweep_cookies(app.handle());
      watch(app.handle());
      rescan(app.handle().clone());
      let controls = app.get_webview_window("main").and_then(|w| media_keys::init(&w));
      app.manage(media_keys::Media(Mutex::new(controls)));
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      library, add_folder, remove_folder, cover, art_color, storage_usage, log_file, app_restart, clear_streaming_cache, clear_lyrics_cache, rescan, extensions, screenshot_dir, lyrics, lyrics_fetch, notch_lyrics,
      playlist_create, playlist_update, playlist_items, playlist_delete, rate, love, played, skipped, top,
      tag_write, fetch_cover, stage_image, read_text, write_playlist, kv_get, kv_set,
      lastfm_status, lastfm_set_keys, lastfm_begin, lastfm_finish, lastfm_logout, lastfm_now_playing, lastfm_scrobble, media_keys::media_track, media_keys::media_state,
      spotify::spotify_status, spotify::spotify_set_client, spotify::spotify_begin, spotify::spotify_logout, spotify::spotify_playlists,
      spotify::spotify_tracks, spotify::spotify_player, spotify::spotify_control, japanese::japanese_words, japanese::japanese_install, japanese::japanese_remove,
      translate::google_translate,
      quran::quran_fetch, quran::quran_pauses, ytdlp::tools_status, ytdlp::tool_install, ytdlp::ytdlp_update,
      ytdlp::download_add, ytdlp::download_cancel, ytdlp::youtube_search, ytdlp::youtube_list, ytdlp::youtube_ready, ytdlp::download_probe, ytdlp::youtube_video_url,
      session::session_host, session::session_share, session::session_state, session::session_chat, session::session_kick,
      session::session_join, session::session_leave, session_art, discord::discord_set, discord::discord_ready,
      ytmusic::ytm_home, ytmusic::ytm_search, ytmusic::ytm_album, ytmusic::ytm_artist, ytmusic::ytm_playlist, ytmusic::ytm_radio,
      japanese::japanese_status,
      subsonic::subsonic_status, subsonic::subsonic_connect, subsonic::subsonic_logout, subsonic::subsonic_call,
      subsonic::subsonic_stream, subsonic::subsonic_cover, tidal::tidal_status, tidal::tidal_set_client, tidal::tidal_begin,
      tidal::tidal_logout, tidal::tidal_playlists, tidal::tidal_tracks, apple::apple_status, apple::apple_setup, apple::apple_begin,
      apple::apple_logout, apple::apple_playlists, apple::apple_tracks, mpv::mpv_init, mpv::mpv_command, mpv::mpv_set,
      mpv::mpv_ytdl_sign_in, update::update_check, update::update_install, translate_local::mt_catalog, translate_local::mt_install,
      translate_local::mt_status, translate_local::mt_remove, translate_local::mt_file
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|app, event| {
      // no sign-in copy outlives the app (ytdlp.rs)
      if let tauri::RunEvent::Exit = event {
        ytdlp::sweep_cookies(app);
      }
    });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shares_lyrics_with_notch_in_one_piece() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("notch");
    share_with_notch(&dir, r#"{"v":1}"#).unwrap();
    share_with_notch(&dir, r#"{"v":1,"lines":[]}"#).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("now-lyrics.json")).unwrap(), r#"{"v":1,"lines":[]}"#);
    assert!(!dir.join("now-lyrics.json.tmp").exists(), "the temp file was renamed into place");
    assert!(share_with_notch(&dir, &"x".repeat(5 << 20)).is_err(), "too large is refused");
  }

  #[test]
  fn copies_the_old_library_once_and_leaves_it_in_place() {
    let d = tempfile::tempdir().unwrap();
    let (old, new) = (d.path().join("com.sonora.player"), d.path().join("com.qistara.app"));
    std::fs::create_dir_all(old.join("art/sub")).unwrap();
    std::fs::create_dir_all(old.join("EBWebView")).unwrap();
    std::fs::write(old.join("library.db"), b"db").unwrap();
    std::fs::write(old.join("art/sub/a.jpg"), b"jpg").unwrap();
    assert!(migrate(&old, &new).unwrap());
    assert_eq!(std::fs::read(new.join("library.db")).unwrap(), b"db");
    assert_eq!(std::fs::read(new.join("art/sub/a.jpg")).unwrap(), b"jpg");
    assert!(!new.join("EBWebView").exists());
    assert!(old.join("library.db").exists());
    // a library of its own is never overwritten
    std::fs::write(new.join("library.db"), b"mine").unwrap();
    assert!(!migrate(&old, &new).unwrap());
    assert_eq!(std::fs::read(new.join("library.db")).unwrap(), b"mine");
    // nothing to copy: a fresh install
    assert!(!migrate(&d.path().join("none"), &d.path().join("fresh")).unwrap());
  }

  #[test]
  fn names_covers_never_paths() {
    assert_eq!(art_name("0123abcd.jpg"), Some((false, "0123abcd.jpg")));
    assert_eq!(art_name("stream:0123abcd.jpg"), Some((true, "0123abcd.jpg")));
    for bad in ["", "stream:", "../library.db", "stream:../../x", "a/b.jpg", r"a\b.jpg", "C:x.jpg", "stream:stream:x"] {
      assert_eq!(art_name(bad), None, "{bad}");
    }
  }

  #[test]
  fn takes_the_newest_old_library() {
    let d = tempfile::tempdir().unwrap();
    for (n, db) in [("com.confluenceplayer.app", "confluence"), ("com.sonora.player", "sonora")] {
      std::fs::create_dir_all(d.path().join(n)).unwrap();
      std::fs::write(d.path().join(n).join("library.db"), db).unwrap();
    }
    let new = d.path().join("com.qistara.app");
    std::fs::create_dir_all(&new).unwrap();
    let copied: Vec<bool> = OLD_DATA.iter().map(|n| migrate(&d.path().join(n), &new).unwrap()).collect();
    assert_eq!(copied, [true, false]);
    assert_eq!(std::fs::read(new.join("library.db")).unwrap(), b"confluence");
  }

  #[test]
  fn finds_a_covers_colour_and_none_in_grey_art() {
    let half = image::RgbImage::from_fn(64, 64, |x, _| if x < 32 { image::Rgb([200, 30, 40]) } else { image::Rgb([128, 128, 128]) });
    let c = vivid(&image::DynamicImage::ImageRgb8(half)).unwrap();
    let [r, g, b] = [1, 3, 5].map(|i| u8::from_str_radix(&c[i..i + 2], 16).unwrap());
    assert!(r > 170 && g < 70 && b < 80, "{c}");
    // two strong hues: one of them, never the grey between them
    let split = image::RgbImage::from_fn(64, 64, |x, _| if x < 40 { image::Rgb([20, 40, 120]) } else { image::Rgb([220, 170, 40]) });
    let c = vivid(&image::DynamicImage::ImageRgb8(split)).unwrap();
    let [r, g, b] = [1, 3, 5].map(|i| u8::from_str_radix(&c[i..i + 2], 16).unwrap());
    assert!(b > 90 && r < 60 && g < 70, "{c}");
    let grey = image::RgbImage::from_pixel(64, 64, image::Rgb([90, 90, 90]));
    assert_eq!(vivid(&image::DynamicImage::ImageRgb8(grey)), None);
  }
}

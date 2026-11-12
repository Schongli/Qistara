mod db;
mod lastfm;
mod lyrics;
mod media_keys;
mod scan;
mod tags;
mod user;

use notify_debouncer_mini::{new_debouncer, notify::RecommendedWatcher, notify::RecursiveMode, Debouncer};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct Lib {
  db: Mutex<rusqlite::Connection>,
  pub art: scan::Art,
  scanning: AtomicBool,
  again: AtomicBool,
  watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
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
}

#[tauri::command]
fn library(lib: State<Lib>) -> Res<Library> {
  let c = lib.db.lock().unwrap();
  Ok(Library {
    folders: db::folders(&c).map_err(err)?,
    tracks: db::tracks(&c).map_err(err)?,
    playlists: user::playlists(&c).map_err(err)?,
    art_dir: lib.art.dir.to_string_lossy().into_owned(),
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

/// Pictures\Sonora, created on first use (mpv won't create a missing screenshot folder).
#[tauri::command]
fn screenshot_dir(app: AppHandle) -> Res<String> {
  let d = app.path().picture_dir().map_err(err)?.join("Sonora");
  std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
  Ok(d.to_string_lossy().into_owned())
}

fn lyrics_dir(lib: &Lib) -> std::path::PathBuf {
  lib.art.dir.with_file_name("lyrics")
}

#[tauri::command]
fn lyrics(lib: State<Lib>, path: String) -> Option<lyrics::Found> {
  lyrics::local(Path::new(&path), &lyrics_dir(&lib))
}

/// Caches an online result in app data, never in the music folder; "" records that none exists.
#[tauri::command]
fn lyrics_store(lib: State<Lib>, path: String, text: String) -> Res<()> {
  lyrics::store(Path::new(&path), &lyrics_dir(&lib), &text).map_err(err)
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

/// Reads a small text file the user picked (an Equalizer APO / AutoEQ preset).
#[tauri::command]
fn read_text(path: String) -> Res<String> {
  let m = std::fs::metadata(&path).map_err(err)?;
  if m.len() > 1024 * 1024 {
    return Err("file is too large for a preset".into());
  }
  std::fs::read_to_string(&path).map_err(err)
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
    .user_agent("Sonora/0.1.0")
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_libmpv::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_opener::init())
    .manage(LastfmPending::default())
    .manage(ScrobbleLock::default())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      let dir = app.path().app_local_data_dir()?;
      std::fs::create_dir_all(&dir)?;
      // the webview may load artwork from the cache only (the $APPLOCALDATA glob didn't match on Windows)
      app.asset_protocol_scope().allow_directory(dir.join("art"), true)?;
      app.manage(Lib {
        db: Mutex::new(db::open(&dir.join("library.db"))?),
        art: scan::Art::new(dir.join("art")),
        scanning: AtomicBool::new(false),
        again: AtomicBool::new(false),
        watcher: Mutex::new(None),
      });
      watch(app.handle());
      rescan(app.handle().clone());
      let controls = app.get_webview_window("main").and_then(|w| media_keys::init(&w));
      app.manage(media_keys::Media(Mutex::new(controls)));
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      library, add_folder, remove_folder, cover, rescan, extensions, screenshot_dir, lyrics, lyrics_store,
      playlist_create, playlist_update, playlist_items, playlist_delete, rate, love, played, skipped, top,
      tag_write, fetch_cover, stage_image, read_text, write_playlist, kv_get, kv_set,
      lastfm_status, lastfm_set_keys, lastfm_begin, lastfm_finish, lastfm_logout, lastfm_now_playing, lastfm_scrobble, media_keys::media_track, media_keys::media_state
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

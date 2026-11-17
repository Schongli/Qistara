// Updates (Settings › General › Updates): Qistara asks its GitHub releases for a newer version and installs it
// when you say so. The plugin checks the download against the public key in tauri.conf.json and against the
// version it was signed for (requireSignedVersion), so an altered latest.json can't install an older release.
// The page gets only these two commands, not the plugin's own; the endpoint is fixed in tauri.conf.json.

use serde::Serialize;

/// A build without the updater's public key (tauri.conf.json › plugins › updater › pubkey) can't update itself.
pub const NOT_SET_UP: &str = "updates-not-set-up";

#[derive(Serialize)]
pub struct Info {
  version: String,
  notes: String,
  /// seconds since 1970, when the release says
  date: Option<i64>,
}

#[cfg(desktop)]
pub use desktop::*;

#[cfg(desktop)]
mod desktop {
  use super::{Info, NOT_SET_UP};
  use std::sync::Mutex;
  use tauri::{AppHandle, Emitter, Runtime, State};
  use tauri_plugin_updater::{Update, UpdaterExt};

  /// The update the last check found, kept for Install.
  #[derive(Default)]
  pub struct Pending(Mutex<Option<Update>>);

  fn configured<R: Runtime>(app: &AppHandle<R>) -> bool {
    let key = app.config().plugins.0.get("updater").and_then(|u| u.get("pubkey")).and_then(|k| k.as_str());
    key.is_some_and(|k| !k.trim().is_empty())
  }

  fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
  }

  #[tauri::command]
  pub async fn update_check(app: AppHandle, pending: State<'_, Pending>) -> Result<Option<Info>, String> {
    if !configured(&app) {
      return Err(NOT_SET_UP.into());
    }
    let update = app.updater_builder().header("User-Agent", crate::UA).map_err(err)?.build().map_err(err)?.check().await.map_err(err)?;
    let info = update.as_ref().map(|u| Info {
      version: u.version.clone(),
      notes: u.body.clone().unwrap_or_default(),
      date: u.date.map(|d| d.unix_timestamp()),
    });
    *pending.0.lock().unwrap() = update;
    Ok(info)
  }

  /// Downloads, checks and installs the update the last check found, then restarts (on Windows the installer
  /// closes Qistara itself and opens the new version).
  #[tauri::command]
  pub async fn update_install(app: AppHandle, pending: State<'_, Pending>) -> Result<(), String> {
    let update = pending.0.lock().unwrap().take().ok_or("Check for updates first.")?;
    let (mut done, mut told) = (0u64, 0u64);
    update
      .download_and_install(
        |chunk, total| {
          done += chunk as u64;
          // about every half megabyte, not every network read
          if done - told >= 512 * 1024 || Some(done) == total {
            told = done;
            let _ = app.emit("update-progress", (done, total));
          }
        },
        || {},
      )
      .await
      .map_err(err)?;
    app.restart()
  }
}

/// Phones update through their store.
#[cfg(mobile)]
#[derive(Default)]
pub struct Pending;

#[cfg(mobile)]
#[tauri::command]
pub async fn update_check() -> Result<Option<Info>, String> {
  Err(NOT_SET_UP.into())
}

#[cfg(mobile)]
#[tauri::command]
pub async fn update_install() -> Result<(), String> {
  Err(NOT_SET_UP.into())
}

#[cfg(all(test, desktop))]
mod tests {
  /// The plugin reads this at startup: a typo would stop the app, an http endpoint or a missing version check
  /// would weaken updates.
  #[test]
  fn config_is_what_the_plugin_expects() {
    let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let c: tauri_plugin_updater::Config = serde_json::from_value(conf["plugins"]["updater"].clone()).unwrap();
    assert!(c.require_signed_version && !c.allow_downgrades && !c.dangerous_insecure_transport_protocol);
    assert_eq!(c.endpoints.len(), 1);
    assert!(c.endpoints[0].as_str().starts_with("https://github.com/Schongli/Qistara/releases/"));
    assert!(c.windows.is_some());
  }

  /// The key in the app is a minisign *public* key (never the secret key from the same folder).
  #[test]
  fn pubkey_is_a_public_key() {
    use base64::Engine;
    let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let key = conf["plugins"]["updater"]["pubkey"].as_str().unwrap();
    let text = String::from_utf8(base64::engine::general_purpose::STANDARD.decode(key).unwrap()).unwrap();
    let mut lines = text.lines();
    assert!(lines.next().unwrap().starts_with("untrusted comment: minisign public key: "));
    let raw = base64::engine::general_purpose::STANDARD.decode(lines.next().unwrap()).unwrap();
    assert_eq!((raw.len(), &raw[..2]), (42, &b"Ed"[..]));
  }
}

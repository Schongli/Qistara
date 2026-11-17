// Media keys and the system media overlay (SMTC on Windows, MPRIS on Linux, Now Playing on macOS) via souvlaki.
// Phones: none yet (the type is a placeholder and the commands do nothing).
#[cfg(desktop)]
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
#[cfg(desktop)]
use std::path::Path;
use std::sync::Mutex;
#[cfg(desktop)]
use std::time::Duration;
use tauri::{State, WebviewWindow};
#[cfg(desktop)]
use tauri::{Emitter, Manager};

#[cfg(mobile)]
pub type MediaControls = ();
pub struct Media(pub Mutex<Option<MediaControls>>);

#[cfg(mobile)]
pub fn init(_window: &WebviewWindow) -> Option<MediaControls> {
    None
}
#[cfg(mobile)]
#[tauri::command]
pub fn media_track(_media: State<Media>, _path: String, _title: String, _artist: String, _album: String, _duration: f64) {}
#[cfg(mobile)]
#[tauri::command]
pub fn media_state(_media: State<Media>, _playing: bool, _position: f64) {}

#[cfg(desktop)]
pub fn init(window: &WebviewWindow) -> Option<MediaControls> {
    #[cfg(windows)]
    let hwnd = Some(window.hwnd().ok()?.0 as *mut std::ffi::c_void);
    #[cfg(not(windows))]
    let hwnd = None;
    let mut c = MediaControls::new(PlatformConfig { display_name: "Qistara", dbus_name: "qistara", hwnd })
        .map_err(|e| log::warn!("media controls unavailable: {e:?}"))
        .ok()?;
    let app = window.app_handle().clone();
    c.attach(move |e| {
        let key = match e {
            MediaControlEvent::Play => "play",
            MediaControlEvent::Pause => "pause",
            MediaControlEvent::Toggle => "toggle",
            MediaControlEvent::Next => "next",
            MediaControlEvent::Previous => "previous",
            MediaControlEvent::Stop => "stop",
            MediaControlEvent::SetPosition(MediaPosition(d)) => {
                let _ = app.emit("media-seek", d.as_secs_f64());
                return;
            }
            _ => return,
        };
        let _ = app.emit("media-key", key);
    })
    .ok()?;
    Some(c)
}

#[cfg(desktop)]
#[tauri::command]
pub fn media_track(media: State<Media>, lib: State<crate::Lib>, path: String, title: String, artist: String, album: String, duration: f64) {
    let cover = lib.art.large(Path::new(&path)).map(|p| format!("file://{}", p.to_string_lossy()));
    if let Some(c) = media.0.lock().unwrap().as_mut() {
        let _ = c.set_metadata(MediaMetadata {
            title: Some(&title),
            artist: Some(&artist),
            album: Some(&album),
            cover_url: cover.as_deref(),
            duration: (duration > 0.0).then(|| Duration::from_secs_f64(duration)),
        });
    }
}

#[cfg(desktop)]
#[tauri::command]
pub fn media_state(media: State<Media>, playing: bool, position: f64) {
    if let Some(c) = media.0.lock().unwrap().as_mut() {
        let progress = Some(MediaPosition(Duration::from_secs_f64(position.max(0.0))));
        let _ = c.set_playback(if playing { MediaPlayback::Playing { progress } } else { MediaPlayback::Paused { progress } });
    }
}

// Windows media keys + the system media overlay (SMTC) via souvlaki.
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use std::ffi::c_void;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State, WebviewWindow};

pub struct Media(pub Mutex<Option<MediaControls>>);

pub fn init(window: &WebviewWindow) -> Option<MediaControls> {
    let hwnd = window.hwnd().ok()?.0 as *mut c_void;
    let mut c = MediaControls::new(PlatformConfig { display_name: "Sonora", dbus_name: "sonora", hwnd: Some(hwnd) })
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

#[tauri::command]
pub fn media_state(media: State<Media>, playing: bool, position: f64) {
    if let Some(c) = media.0.lock().unwrap().as_mut() {
        let progress = Some(MediaPosition(Duration::from_secs_f64(position.max(0.0))));
        let _ = c.set_playback(if playing { MediaPlayback::Playing { progress } } else { MediaPlayback::Paused { progress } });
    }
}

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

/// What the OS media overlay shows for the playing song (media.svelte.ts).
#[derive(serde::Deserialize)]
pub struct NowPlaying {
    path: String,
    title: String,
    artist: String,
    album: String,
    duration: f64,
    /// a streaming song's cover: a cached name ("stream:…") or a YT Music URL
    art: Option<String>,
}
pub struct Media(pub Mutex<Option<MediaControls>>);

#[cfg(mobile)]
pub fn init(_window: &WebviewWindow) -> Option<MediaControls> {
    None
}
#[cfg(mobile)]
#[tauri::command]
pub fn media_track(_media: State<Media>, _track: NowPlaying) {}
#[cfg(mobile)]
#[tauri::command]
pub fn media_state(_media: State<Media>, _playing: bool, _position: f64) {}

#[cfg(desktop)]
pub fn init(window: &WebviewWindow) -> Option<MediaControls> {
    #[cfg(windows)]
    let hwnd = Some(window.hwnd().ok()?.0);
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
/// The song the overlay shows now: a cover fetched for an earlier one doesn't replace it.
static SHOWN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(desktop)]
fn show(media: &Media, t: &NowPlaying, cover: Option<&Path>) {
    let cover = cover.map(|p| format!("file://{}", p.to_string_lossy()));
    if let Some(c) = media.0.lock().unwrap().as_mut() {
        let _ = c.set_metadata(MediaMetadata {
            title: Some(&t.title),
            artist: Some(&t.artist),
            album: Some(&t.album),
            cover_url: cover.as_deref(),
            duration: (t.duration > 0.0).then(|| Duration::from_secs_f64(t.duration)),
        });
    }
}

/// A library song's cover comes from its file; a streaming song's (art: a cached name or a YT Music URL) is fetched
/// once, then the overlay is updated if that song is still the one playing.
#[cfg(desktop)]
#[tauri::command]
pub async fn media_track(media: State<'_, Media>, lib: State<'_, crate::Lib>, track: NowPlaying) -> Result<(), String> {
    use std::sync::atomic::Ordering::SeqCst;
    let me = SHOWN.fetch_add(1, SeqCst) + 1;
    let local = (!track.path.contains("://")).then(|| lib.art.large(Path::new(&track.path))).flatten();
    show(&media, &track, local.as_deref());
    let remote = match (&local, &track.art) {
        (None, Some(a)) => crate::cover_file(&lib, a).await,
        _ => None,
    };
    if let Some(file) = remote.filter(|_| SHOWN.load(SeqCst) == me) {
        show(&media, &track, Some(&file));
    }
    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
pub fn media_state(media: State<Media>, playing: bool, position: f64) {
    if let Some(c) = media.0.lock().unwrap().as_mut() {
        let progress = Some(MediaPosition(Duration::from_secs_f64(position.max(0.0))));
        let _ = c.set_playback(if playing { MediaPlayback::Playing { progress } } else { MediaPlayback::Paused { progress } });
    }
}

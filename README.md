# Confluence Player

A lossless music and media player for Windows, where your files, your music server and YouTube come together.
Built with [Tauri 2](https://tauri.app), [Svelte 5](https://svelte.dev) and [libmpv](https://mpv.io).

## Features

- **Bit-perfect audio.** WASAPI exclusive output at each file's own sample rate and bit depth, on by default.
  A Signal Path view shows every step from file to DAC and says when something changes the sound.
- **Plays almost anything.** FLAC, ALAC, WAV, AIFF, APE, WavPack, DSD (as PCM), Opus, MP3, AAC and more.
  Video works like VLC: subtitles, audio tracks, chapters, speed, A-B loop, frame step, screenshots, resume.
- **Library.** Watched folders, plus Home, Albums, Artists, Genres, Composers, Songs, Videos, History and search.
  Albums pinned to the sidebar, Lossless and Hi-Res badges, and a duplicate finder.
- **Playback.** Gapless queue with Up Next, shuffle, repeat and Autoplay. A player bar, a lyrics and Up Next pane
  beside the library, full-screen Now Playing, MiniPlayer, sleep timer, media keys, and scroll-wheel volume.
- **Lyrics.**
  - Synced lyrics from TTML, LRC, embedded tags or [LRCLIB](https://lrclib.net), with word-by-word highlight.
  - Translation under each line.
  - Learn mode: tap a word for its meaning. Japanese also gets furigana, romaji and a vocabulary review.
  - Qur'an: recitations are recognised by their titles (surahs, Juz 'Amma, Hafs or Shu'bah). Uthmani text with
    word meanings, transliteration and the Kemenag, Saheeh International and Clear Quran translations, the ayah
    being recited kept in view, and words saved to Vocabulary.
- **Organise.** Smart playlists, ratings, Loved, play stats (Your Top), M3U8 import and export.
- **Tag editor.** Safe writes (copy, write, verify, then replace), with MusicBrainz and Cover Art Archive lookup.
- **Parametric EQ.** A live response curve, plus AutoEQ / Equalizer APO preset import.
- **Appearance.** Light, dark or automatic; nine accent colours or your own, or one taken from the playing cover;
  text size, compact rows, Inter or Segoe UI, Mica or a solid window, and which sidebar items show.
- **Sources (Providers), all optional.**
  - **Navidrome / Subsonic:** browse and stream your own server; FLAC streams as stored, so it stays bit-perfect.
  - **YouTube:** search, play and download through [yt-dlp](https://github.com/yt-dlp/yt-dlp), as a guest or
    signed in with Firefox's cookies or a cookies.txt file (for Liked Music and Premium audio).
  - **Spotify, Apple Music, Tidal:** import your playlists as playlists of your own files, plus a Spotify Connect
    remote. Their streams are DRM-protected, so they never play or download here.
  - Last.fm scrobbling and internet radio ([Radio Browser](https://www.radio-browser.info)).
- **Downloads.** Paste links (videos, songs, playlists, channels) and save them as audio (as published, or MP3) or
  video (1080p or best) into a folder that joins your library. yt-dlp comes from its official GitHub release and is
  checked against the release's SHA-256 list; an ffmpeg on your PATH is used, or fetched the same way.
  Download only what you have the right to keep.

## Install

Download `Confluence Player_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it.
- **Requirements:** 64-bit Windows 10 or 11.
- **Windows warning:** the installer isn't signed yet, so Windows shows "Windows protected your PC". Choose **More info → Run anyway**.

## Build from source

You need:
- Windows 10/11 x64
- [Node.js](https://nodejs.org) 22 or newer
- [Rust](https://rustup.rs) (stable, MSVC)
- Visual Studio Build Tools with the "Desktop development with C++" workload

```bash
npm install
npx tauri-plugin-libmpv-api setup-lib   # puts libmpv-2.dll and libmpv-wrapper.dll in src-tauri/lib
npm run tauri dev                       # run the app
npm run tauri build                     # installer in src-tauri/target/release/bundle/nsis
```

The first build downloads the IPADIC dictionary for Japanese word analysis and embeds it in the app.

Tests: `npx vitest run` and `cargo test --lib --manifest-path src-tauri/Cargo.toml`.
`bash scripts/make-test-media.sh` (needs ffmpeg) makes test files: hi-res FLAC tones, an HEVC 10-bit video with subtitles,
and a small tagged library.

## Privacy

Confluence Player has no account, no telemetry and no servers of its own. It only contacts these services, and only for the features that need them:

| Service | When | What is sent |
|---|---|---|
| LRCLIB | a song has no local lyrics | title, artist, album, length |
| MusicBrainz, Cover Art Archive | you look up tags or artwork | album and artist |
| Google Translate | you turn on translation or Learn mode | the lyric lines, or the one word you tap |
| Radio Browser | you search for stations | your search text |
| quran.com, GitHub (fawazahmed0 quran-api) | a Qur'an recitation plays | the surah number |
| YouTube (through yt-dlp) | you search, play or download | your search text or link; your cookies only if you sign in |
| GitHub releases | you install or update yt-dlp or ffmpeg | nothing but the download |
| Your Subsonic server | you sign in to it | your user name and a salted password hash, only to that server |
| Last.fm, Spotify, Apple Music, Tidal | you connect your own account | what that feature needs |

Requests identify themselves only as `ConfluencePlayer/<version>`. Last.fm keys, your server password and the
Spotify, Apple Music and Tidal sign-ins are kept in Windows Credential Manager.

### Coming from Sonora

This app was called Sonora. On first start, Confluence Player copies the old library, settings and caches
(`%LOCALAPPDATA%\com.sonora.player`) and the saved sign-ins; the old folder is left as it was.

## Credits

- Inspired by [Molow](https://github.com/Dlcuy22/molow).
- Playback by [mpv](https://mpv.io) / FFmpeg.
- Tags by [lofty](https://github.com/Serial-ATA/lofty-rs).
- Japanese analysis by [Lindera](https://github.com/lindera/lindera) with the IPADIC dictionary.
- Downloads and YouTube by [yt-dlp](https://github.com/yt-dlp/yt-dlp); Qur'an text from [quran.com](https://quran.com)
  and [fawazahmed0/quran-api](https://github.com/fawazahmed0/quran-api); Arabic font: [Amiri Quran](https://www.amirifont.org).
- Icons by [Phosphor](https://phosphoricons.com); font: [Inter](https://rsms.me/inter/).

Confluence Player is not affiliated with Apple, Spotify, Tidal, Google or YouTube.

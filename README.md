# Sonora

A lossless music and media player for Windows with an Apple-style interface.
Built with [Tauri 2](https://tauri.app), [Svelte 5](https://svelte.dev) and [libmpv](https://mpv.io).

## Features

- **Bit-perfect audio.** WASAPI exclusive output at each file's own sample rate and bit depth, on by default.
  A Signal Path view shows every step from file to DAC and says when something changes the sound.
- **Plays almost anything.** FLAC, ALAC, WAV, AIFF, APE, WavPack, DSD (as PCM), Opus, MP3, AAC and more.
  Video works like VLC: subtitles, audio tracks, chapters, speed, A-B loop, frame step, screenshots, resume.
- **Library.** Watched folders, plus Albums, Artists, Genres, Composers, Songs, Videos and search.
  Lossless and Hi-Res badges, and a duplicate finder.
- **Playback.** Gapless queue with Up Next, shuffle, repeat and Autoplay. Full-screen Now Playing, MiniPlayer,
  sleep timer, media keys, and scroll-wheel volume.
- **Lyrics.**
  - Synced lyrics from TTML, LRC, embedded tags or [LRCLIB](https://lrclib.net), with word-by-word highlight.
  - Translation under each line.
  - Learn mode: tap a word for its meaning. Japanese also gets furigana, romaji and a vocabulary review.
- **Organise.** Smart playlists, ratings, Loved, play stats (Your Top), M3U8 import and export.
- **Tag editor.** Safe writes (copy, write, verify, then replace), with MusicBrainz and Cover Art Archive lookup.
- **Parametric EQ.** A live response curve, plus AutoEQ / Equalizer APO preset import.
- **Online, all optional.**
  - Last.fm scrobbling and internet radio ([Radio Browser](https://www.radio-browser.info)).
  - Spotify playlist import, matched to your own files, and a Spotify Connect remote.

## Install

Download `Sonora_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it.
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

Sonora has no account, no telemetry and no servers of its own. It only contacts these services, and only for the features that need them:

| Service | When | What is sent |
|---|---|---|
| LRCLIB | a song has no local lyrics | title, artist, album, length |
| MusicBrainz, Cover Art Archive | you look up tags or artwork | album and artist |
| Google Translate | you turn on translation or Learn mode | the lyric lines, or the one word you tap |
| Radio Browser | you search for stations | your search text |
| Last.fm, Spotify | you connect your own account | what that feature needs |

Requests identify themselves only as `Sonora/<version>`. Last.fm keys and the Spotify sign-in are kept in Windows Credential Manager.

## Credits

- Inspired by [Molow](https://github.com/Dlcuy22/molow).
- Playback by [mpv](https://mpv.io) / FFmpeg.
- Tags by [lofty](https://github.com/Serial-ATA/lofty-rs).
- Japanese analysis by [Lindera](https://github.com/lindera/lindera) with the IPADIC dictionary.
- Icons by [Phosphor](https://phosphoricons.com); font: [Inter](https://rsms.me/inter/).

Sonora is not affiliated with Apple or Spotify.

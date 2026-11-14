# Qistara

A lossless music and media player for Windows that keeps your music in true balance: what reaches your DAC is
what is in the file. Your files, your music server and YouTube in one place.
Built with [Tauri 2](https://tauri.app), [Svelte 5](https://svelte.dev) and [libmpv](https://mpv.io).

![Qistara's album library, with a hi-res song playing bit-perfect](docs/screenshots/albums.png)

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
  - Synced lyrics from TTML, LRC, embedded tags, your Navidrome/Subsonic server, [LRCLIB](https://lrclib.net),
    NetEase or Kugou, asked in the order you set, with word-by-word highlight (and an option to prefer word timing).
  - Translation under each line.
  - Learn mode: tap a word for its meaning. Japanese also gets furigana, romaji and a vocabulary review.
  - Qur'an: recitations are recognised by their titles (surahs, Juz 'Amma, Hafs or Shu'bah). Uthmani text with
    word meanings, transliteration and the Kemenag, Saheeh International and Clear Quran translations, the ayah
    being recited kept in view, and words saved to Vocabulary.
- **Organise.** Smart playlists, ratings, Loved, play stats (Your Top), M3U8 import and export.
- **Tag editor.** Safe writes (copy, write, verify, then replace), with MusicBrainz and Cover Art Archive lookup.
- **Parametric EQ.** A live response curve, plus AutoEQ / Equalizer APO preset import.
- **Appearance.** Light, Qistara Obsidian or automatic, plus Catppuccin Frappé, Rosé Pine, Gruvbox Dark and Solarized
  Dark; nine accent colours or your own, or one taken from the playing cover; five icon packs; corners from square
  to round; text size, compact rows, Inter or Segoe UI, Mica and blur on or off, and which sidebar items show.
  Settings has tabs and a search (Ctrl+F).
- **Sources (Providers), all optional.**
  - **Navidrome / Subsonic:** browse and stream your own server; FLAC streams as stored, so it stays bit-perfect.
  - **YouTube:** search, play and download through [yt-dlp](https://github.com/yt-dlp/yt-dlp), as a guest or
    signed in with Firefox's cookies or a cookies.txt file (for Liked Music and Premium audio).
  - **Spotify, Apple Music, Tidal:** import your playlists as playlists of your own files, plus a Spotify Connect
    remote. Their streams are DRM-protected, so they never play or download here.
  - Last.fm scrobbling and internet radio ([Radio Browser](https://www.radio-browser.info)).
- **Downloads.** Paste links (videos, songs, playlists, channels) and save them as audio (as published, or MP3) or
  video (1080p or best) into a folder that joins your library (the folder and format are remembered). yt-dlp comes from its official GitHub release and is
  checked against the release's SHA-256 list; an ffmpeg on your PATH is used, or fetched the same way.
  Download only what you have the right to keep.

## Screenshots

| | |
|---|---|
| ![An album with its synced lyrics beside it, highlighted word by word](docs/screenshots/album-lyrics.png) | ![Now Playing with synced lyrics](docs/screenshots/now-playing.png) |
| An album, with lyrics highlighted word by word | Now Playing |
| ![The Signal Path, showing a 24-bit/96 kHz FLAC reaching the DAC untouched](docs/screenshots/signal-path.png) | ![The parametric equalizer with five bands](docs/screenshots/equalizer.png) |
| The Signal Path: every step from file to DAC | Parametric EQ, with AutoEQ import |
| ![Appearance settings: themes, accent colours, corners and icon packs](docs/screenshots/settings.png) | ![The album library in the light theme](docs/screenshots/albums-light.png) |
| Settings, with tabs and search | The light theme |

The screenshots show the demo library that `scripts/make-test-media.sh` makes: the artists are made up.

## Documentation

- [User guide](docs/user-guide.md): every feature, the Signal Path, lyrics sources, accounts, shortcuts and troubleshooting.
- [How Qistara is built](docs/architecture.md): the parts, how playback and data work, and the rules that keep it safe.
- [Contributing](CONTRIBUTING.md), [Security policy](SECURITY.md) and [Changelog](CHANGELOG.md).

## Install

Download `Qistara_x.y.z_x64-setup.exe` from [Releases](https://github.com/Schongli/Qistara/releases) and run it.
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

Tests: `npx vitest run` and `cargo test --lib --manifest-path src-tauri/Cargo.toml`. More in [CONTRIBUTING.md](CONTRIBUTING.md).
`bash scripts/make-test-media.sh` (needs ffmpeg) makes test files: hi-res FLAC tones, an HEVC 10-bit video with subtitles,
and a small tagged library.

## Privacy

Qistara has no account, no telemetry and no servers of its own. It only contacts these services, and only for the features that need them:

| Service | When | What is sent |
|---|---|---|
| LRCLIB | a song has no local lyrics | title, artist, album, length |
| NetEase Cloud Music, Kugou | you turn them on in Settings › Lyrics and a song has no lyrics higher in the list | title, artist, length |
| MusicBrainz, Cover Art Archive | you look up tags or artwork | album and artist |
| Google Translate | you turn on translation or Learn mode | the lyric lines, or the one word you tap |
| Radio Browser | you search for stations | your search text |
| quran.com, GitHub (fawazahmed0 quran-api) | a Qur'an recitation plays | the surah number |
| YouTube (through yt-dlp) | you search, play or download | your search text or link; your cookies only if you sign in |
| GitHub releases | you install or update yt-dlp or ffmpeg | nothing but the download |
| Your Subsonic server | you sign in to it | your user name and a salted password hash, only to that server; a song's id when its lyrics are looked up |
| Last.fm, Spotify, Apple Music, Tidal | you connect your own account | what that feature needs |

Requests identify themselves only as `Qistara/<version>`. Last.fm keys, your server password and the
Spotify, Apple Music and Tidal sign-ins are kept in Windows Credential Manager.

### Coming from Confluence Player or Sonora

This app was called Confluence Player, and before that Sonora. On first start, Qistara copies the newest old library,
settings and caches (`%LOCALAPPDATA%\com.confluenceplayer.app` or `com.sonora.player`) and the saved sign-ins; the old
folders are left as they were, and the old app can be uninstalled.

## Credits

- Inspired by [Molow](https://github.com/Dlcuy22/molow).
- Playback by [mpv](https://mpv.io) / FFmpeg.
- Tags by [lofty](https://github.com/Serial-ATA/lofty-rs).
- Japanese analysis by [Lindera](https://github.com/lindera/lindera) with the IPADIC dictionary.
- Downloads and YouTube by [yt-dlp](https://github.com/yt-dlp/yt-dlp); Qur'an text from [quran.com](https://quran.com)
  and [fawazahmed0/quran-api](https://github.com/fawazahmed0/quran-api); Arabic font: [Amiri Quran](https://www.amirifont.org).
- Icons by [Phosphor](https://phosphoricons.com) (MIT), [Lucide](https://lucide.dev) (ISC), [Iconoir](https://iconoir.com) (MIT),
  [Remix Icon](https://remixicon.com) (Apache 2.0) and [Solar](https://www.figma.com/community/file/1166831539721848736) by 480 Design (CC BY 4.0),
  through [Iconify](https://iconify.design); font: [Inter](https://rsms.me/inter/).
- Theme palettes: [Catppuccin](https://catppuccin.com), [Rosé Pine](https://rosepinetheme.com), [Gruvbox](https://github.com/morhetz/gruvbox)
  and [Solarized](https://ethanschoonover.com/solarized/).

Qistara is not affiliated with Apple, Spotify, Tidal, Google, YouTube, NetEase or Kugou.

## License

[MIT](LICENSE) © 2026 Schongli.
The installer also ships libmpv and FFmpeg from [zhongfly/mpv-winbuild](https://github.com/zhongfly/mpv-winbuild), which are
licensed under the GPL; their source is at [mpv-player/mpv](https://github.com/mpv-player/mpv) and [ffmpeg.org](https://ffmpeg.org/download.html).

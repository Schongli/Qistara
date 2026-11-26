# Changelog

## 0.6.0 (beta)

- **YouTube and YT Music songs start much faster.** Songs now play from Qistara's own streaming engine instead of
  running yt-dlp for each one. It asks YouTube for the song's sound directly and starts playing as the first bytes
  arrive. It also fetches the next songs ahead, as well as the song your pointer rests on and the first song of an
  album or playlist you open, so they start at once. If YouTube refuses mid-song, the engine asks again and carries
  on. If the engine can't open a song, yt-dlp plays it as before. YT Music no longer needs yt-dlp at all.
- **Sign in to YouTube in Qistara's own window.** Providers › YouTube › Sign In with Google opens Google's sign-in
  page in a small window, the way desktop apps sign in.
  - **What you get:** YT Music shows your own Home and a Library tab (Liked Music, your playlists, saved albums,
    followed artists), and age-restricted songs play.
  - **Where it's kept:** encrypted on your computer, with the key in Credential Manager, the Keychain or the Secret
    Service, and it is sent only to YouTube.
  - Firefox and cookies.txt are still there, under "Other ways to sign in".
- **Audio quality:** Standard starts fastest. Premium uses yt-dlp with your account for YouTube Premium's 256 kbps
  audio when it's offered to you.
- **yt-dlp is quicker too** (search, downloads, videos):
  - It's installed unpacked rather than as a single file that unpacks itself on every run.
  - It keeps YouTube's player code between runs, in the streaming cache.
  - Updates fetch the new release, checked against its SHA-256.
- **Lyrics for YouTube songs.**
  - Qistara now asks with the song's own title (without "(Official Video)", 【MV】 and the like) and without the
    "YT Music" placeholder album.
  - LRCLIB falls back to its search.
  - A new YT Music lyrics source, on by default, gives YouTube songs the lyrics YouTube itself shows.
- **The adaptive theme follows YouTube and YT Music covers**, and the system's media controls show the song's cover
  instead of the app icon.
- **A spinning ring on Play** while a song or a seek is opening.
- Settings › Playback › **Streaming › Test** times how long YouTube takes to find a song and to start sounding.
- macOS: Now Playing's close button sits below the window buttons. Settings › Appearance › Corners says that it
  shapes buttons, cards and artwork (the window's corners follow the system).
- Betas include the Windows installer (unsigned), so a beta can be tried on Windows too.
- Linux with mpv 0.37 or older (Ubuntu 24.04, Debian 12): Qistara opens again where you left off, and switching
  exclusive mode or a YouTube song's video keeps your place. These failed there before.

## 0.5.0 (beta)

- **Qistara on macOS and Linux.** A `.dmg` for Macs with Apple Silicon (macOS 13 or newer), and a `.deb` and an
  `.AppImage` for Linux. Your library, bit-perfect playback (CoreAudio hog mode on a Mac, an ALSA `hw:` device on
  Linux), lyrics and translation, YT Music, listening sessions, Discord and in-app updates all come along.
  - **macOS** plays through mpv from Homebrew (`brew install mpv`). Qistara isn't signed by Apple yet: the first
    time, choose System Settings › Privacy & Security › Open Anyway.
  - **Linux:** the `.deb` (Ubuntu 24.04+, Debian 12+, Mint) brings libmpv with it. The AppImage uses your
    system's (Fedora `mpv-libs`, Arch `mpv`).
  - **Music first:** on macOS and Linux, video files play their sound for now. Watch Video and music videos stay
    Windows-only until video can show inside the window there.
- **"Install mpv to play music".** When the player can't start, Qistara says why and how to fix it, with the command
  to copy and a Restart button, instead of staying silent.
- **A log file** for bug reports: Settings › General › Updates › Show Log. It stays on your computer.
- **notch:** the lyrics you see (word timing and translation included), or the ayahs of a recitation with their
  translation, are shared with notch, a separate top-of-screen app, through a local file in Qistara's data folder
  (`notch/now-lyrics.json`). Nothing is sent anywhere.

## 0.4.1 (2026-11-23)

- **Translate on this PC.** Settings › Lyrics › Translation › Translate with: Google, then this PC when Google is busy
  (default); Google only; or this PC only. "This PC" is Firefox's own translator with Mozilla's language packs, about
  100 pairs to and from English (Indonesian, Japanese, Arabic, Korean, Chinese, Malay…), downloaded once and checked;
  then nothing is sent anywhere. The lyrics' language is recognised offline.
- **Google Translate no longer "busy".** A song was sometimes sent line by line in a burst, which Google refuses.
  Now each batch is one request, requests are spaced out, and when Google says it's busy Qistara waits and tries
  again by itself.
- **Watch YouTube songs with their video** (Watch Video, the Video button, Ctrl+Shift+V), and back to sound only.
- **Music videos for your own songs:** link a YouTube video (pasted or found) or one from your library. It shows
  over the song while you still hear your lossless file, bit-perfect, with a Sync control for intros.
- **Watch together:** guests in a session see the YouTube video or music video the host shows.
- **Right-click menus:** Play, Shuffle, Play Next and Play Later on albums, pinned albums, playlists, artists,
  composers, genres, YT Music cards and songs; playlists also get Open, Export and Delete.
- **Keyboard shortcuts:** Ctrl+/ lists them all. New: Ctrl+S shuffle, Ctrl+R repeat, Ctrl+↑/↓ volume, Shift+←/→
  seek, Ctrl+L love, Ctrl+Shift+L / Q lyrics and Up Next, Ctrl+, settings, Ctrl+N new playlist, Alt+→ forward.
- **Readability:** the theme variant list (and every dropdown) is readable in dark themes; Arabic and Hebrew titles
  read in their own direction.
- **Fixes:** hosting a listen-together session on Windows turned every guest away.

## 0.4.0 (2026-11-17)

- **YT Music without an account:** Home, search by songs, albums, artists or playlists, album, artist and playlist
  pages, Start Radio, and endless autoplay from the radio. Songs play through yt-dlp; nothing is signed in or stored.
- **Listen together:** host a session from your PC and share the invite (LAN, Tailscale, ZeroTier or a forwarded
  port). Guests hear the same song at the same moment, your own files included, and chat beside it. The connection
  is end-to-end encrypted; removing a guest changes the invite.
- **Downloads:** pick Video or Audio, then a quality (Best to 240p; Original, M4A, MP3 320/192/128), with the size
  of each shown for a single link.
- **YouTube without Node.js:** Downloads can fetch Deno (checksum-verified) when no JavaScript runtime is
  installed, and says so plainly when one is missing. A YouTube sign-in's cookies are copied per run and deleted
  afterwards.
- **Parametric EQ:** a table of every band (type, frequency, gain, Q, on/off) under the graph; double-click adds a
  band, Shift+drag sets Q, and values are kept in range. The curve uses the real sample rate.
- **Themes:** 17 families with 29 palettes under Qistara's own names (Pastel, Moonbloom, Timber, Sundial, Fjord,
  Nocturne, Neon City, Evergreen, Ukiyo, Slate, Andalus, Madinah, Sakura, Matcha, Pitch Black, WibuFest, plus Light
  and Obsidian). **Adaptive theme** colours the whole app from the cover of what's playing. Every palette is
  contrast-tested.
- **Discord:** show what you're listening to on your Discord profile (off by default; local files never send a
  path).
- **Qur'an mode in Indonesian:** Kemenag surah names, Indonesian titles (`QS`, `murotal`, `asy-`, `adz-`…)
  recognised, Indonesian labels and word meanings.
- **macOS and Linux:** bit-perfect output through CoreAudio hog mode and ALSA `hw:` devices, Unix paths, the
  system's own window buttons on macOS. Built and tested on all three in CI.
- **Smaller windows and touch:** the layout works down to 360 px wide (icon rail, drawer, compact player bar), one
  tap plays on a touch screen and a long press opens the menu. The code also builds for phones; playback there
  needs a native player first (docs/mobile.md).
- **Updates:** Settings › General › Updates checks once a day (can be turned off) and installs signed updates when
  you choose to.
- **Japanese dictionary is optional:** the installer is smaller; Learn mode offers the download when you first use
  it.
- **Faster long lists:** song tables and Up Next render only the rows on screen; icon packs load when chosen.
- **Storage:** streaming covers and lyrics are kept apart from your library, with sizes and Clear buttons in
  Settings › General › Storage.
- **Sturdier parsing:** lyrics metadata tags (`[ti:]`, `[ar:]`, `[by:]`…) never show as lines, TTML background
  vocals stay out of the main line, non-UTF-8 lyrics and playlists are decoded, and M3U import handles `file://`
  links, POSIX paths and bad escapes. Parsers are fuzz-tested.

## 0.3.0 (2026-11-14)

Confluence Player is now **Qistara**.

- **New name and look.** A new icon, and the Obsidian and Brass colour theme. The signal badge turns cyan when playback
  is verified bit-perfect. On first start, the library, settings and sign-ins are copied from Confluence Player or Sonora.
- **Lyrics sources** in the order you choose: local files, Navidrome/Subsonic, LRCLIB, NetEase and Kugou (with
  word-by-word timing), plus **Prefer word-synced lyrics**. Results are cached per source.
- **Settings** has tabs and a search (Ctrl+F).
- **Appearance:**
  - Theme presets: Catppuccin Frappé, Rosé Pine, Gruvbox Dark and Solarized Dark.
  - Corner styles, from square to round.
  - Blur on or off, for the window and the interface.
  - Five icon packs: Phosphor, Lucide, Iconoir, Remix and Solar.
- **Back and Forward** return you to where you were scrolled, with the same artist, sort and period selected.
- **Downloads** remember the folder, the format and "add to library", and quick downloads use them too.
- **EQ bands** are coloured by the part of the spectrum they sit in.
- **Safer settings.** A settings read that fails no longer lets defaults overwrite what was stored.
- **Safer playback control.** The interface reaches mpv only through an allowlist of commands and properties in Rust.

## 0.2.0 (2026-11-14): Confluence Player

- **Renamed** from Sonora, with the library and sign-ins carried over.
- **Streaming-app layout:** Home, sidebar groups and pinned albums, a bottom player bar, a docked lyrics and Up Next
  pane, Back and Forward.
- **Appearance settings:** theme, accent colour (a preset, your own, or taken from the cover), text size, density,
  font, solid window and sidebar items.
- **Downloads with yt-dlp,** checksum-verified, with presets, playlists, progress and cancel.
- **YouTube:** search, links and Liked Music, as a guest or signed in with cookies.
- **Navidrome / Subsonic:** browse, search and stream, with FLAC streamed as stored.
- **Playlist import** from Apple Music and Tidal, alongside Spotify.
- **Qur'an Learn mode:** word meanings, transliteration and translations, with ayahs synced to the recitation.

## 0.1.0 (2026-11-12): Sonora

The first release.
- **Bit-perfect WASAPI exclusive playback,** with the Signal Path view.
- **Gapless queue.**
- **Video:** subtitles, tracks, chapters, A-B loop and resume.
- **Synced lyrics** with word highlighting, translation and Learn mode.
- **Playlists:** smart playlists, ratings and play stats.
- **Safe tag editor** with MusicBrainz lookup.
- **Parametric EQ** with AutoEQ import.
- **Services:** Last.fm, radio, and Spotify playlist import.

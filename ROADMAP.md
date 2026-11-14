# Qistara roadmap

## Phase 1: first build
- [x] M0 Setup: toolchain, Tauri + Svelte scaffold, libmpv
- [x] M1 Engine spike: embedded video under HTML, WASAPI exclusive params match, Mica
- [x] M2 Library: folders, scan + watch, art cache, browse views, search, lossless badges
- [x] M3 Playback: player bar, Up Next, shuffle/repeat, gapless, Now Playing sheet, media keys, devices, shortcuts
- [x] M4 Bit-perfect + Signal Path popover, device volume, Sound Check, passthrough
- [x] M5 Video: subs, tracks, speed, chapters, A-B loop, frame step, screenshot, aspect, resume, info, fullscreen
- [x] M6 Synced lyrics (TTML/LRC/embedded/LRCLIB), word sweep, tap to seek
- [x] M7 Playlists, smart playlists, ratings, Loved, play/skip counts, Your Top
- [x] M8 Tag editor (safe writes), MusicBrainz, Cover Art Archive
- [x] M9 Parametric EQ (curve UI, af-command, presets)
- [x] M10 Last.fm scrobbling, Open URL streams, radio stations
- [x] M11 Review fixes, strict CSP, NSIS installer (lyrics view under CSP and light mode not yet checked by hand)

## Phase 2
- [x] MiniPlayer (Ctrl+Shift+M), Autoplay ∞, sleep timer (minutes or end of song)
- [x] Radio Browser directory search, M3U/M3U8 import + export
- [x] AutoEQ / Equalizer APO preset import (in the EQ), Composers view, duplicate finder
- [x] Lyrics translation (Google Translate, cached), scroll-wheel volume on sliders and video
- [x] Learn mode in lyrics: tap any word for its meaning; Japanese gets furigana, romaji, dictionary forms and
  grammar notes (offline IPADIC via Lindera); saved words → Vocabulary page with line replay and flashcard review
  (not yet checked in the running app)
- [x] Qur'an Learn mode: recitations are recognised by title/album (surah, Juz 'Amma/Tabarak, Hafs or Shu'bah,
  partial selections; "Choose Surah…" to override). Uthmani text with word meanings, transliteration, Kemenag,
  Saheeh and Clear Quran; ayahs synced from the pauses in the recording (quran.com timings for its own Alafasy
  murattal), Fix sync by tapping; words save to Vocabulary
- [ ] Learn mode next: romanization for Korean/Chinese/other scripts, a furigana on/off switch
- [ ] Video thumbnails + Movies/TV grouping, crossfade, CUE-sheet tracks, file organiser, Discord presence

## Phase 3
- [x] Spotify: PKCE sign-in with your own developer app (owner needs Premium), playlist + Liked Songs import
  matched to local files, Connect remote (sign-in and remote not yet tried with a real account)
- [ ] DLNA/UPnP, device sync, macOS/Linux

## Phase 4: Confluence Player
- [x] Rename from Sonora (name, icon, data folder and Credential Manager migration)
- [x] Streaming-client shell: Home, sidebar groups and Pinned albums, bottom player bar, docked lyrics/Up Next pane,
  back/forward, album header
- [x] Appearance: theme, accent (presets, custom, match artwork), text size, density, font, solid window, sidebar items
- [x] Downloads with yt-dlp (checksum-verified install, presets, playlists, progress, cancel, into the library)
- [x] YouTube: search, links and Liked Music, played through mpv's ytdl hook, guest or cookies sign-in
- [x] Navidrome / Subsonic: browse, search and stream (FLAC as stored)
- [x] Providers page; Apple Music (own MusicKit key) and Tidal (own developer app) playlist import
  (Apple and Tidal not yet tried with real accounts)
- [ ] Offline copies of server songs, Deezer, Discord presence, UI translations

## Phase 5: Qistara
- [x] Rename from Confluence Player ("Confluence" is a registered name): pivot-Q mark, Obsidian and brass palette,
  signal colours (bit-perfect cyan), data folder and Credential Manager carried over from either old name
- [x] Back and Forward return to the scroll position, the selected artist or composer, Songs sort and Top period
- [x] Downloads remember the folder, format and "add to library" choice (quick downloads use them too)
- [x] Lyrics sources in priority order: local files, Navidrome/Subsonic, LRCLIB, NetEase, Kugou (word-synced KRC),
  with "prefer word-synced"
- [x] Settings tabs and search; theme presets (Catppuccin Frappé, Rosé Pine, Gruvbox Dark, Solarized Dark); corners;
  blur window / blur UI; icon packs (Phosphor, Lucide, Iconoir, Remix, Solar)
  (all of Phase 5 not yet checked by hand in the installed app)

Not planned: fake-lossless checker, native DSD/DoP (mpv converts DSD to PCM), playing or downloading DRM streams.

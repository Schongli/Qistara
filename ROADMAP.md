# Sonora roadmap

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
- [ ] Learn mode next: romanization for Korean/Chinese/other scripts, a furigana on/off switch
- [ ] Video thumbnails + Movies/TV grouping, crossfade, CUE-sheet tracks, file organiser, Discord presence

## Phase 3
- [x] Spotify: PKCE sign-in with your own developer app (owner needs Premium), playlist + Liked Songs import
  matched to local files, Connect remote (sign-in and remote not yet tried with a real account)
- [ ] DLNA/UPnP, device sync, macOS/Linux

Not planned: fake-lossless checker, native DSD/DoP (mpv converts DSD to PCM).

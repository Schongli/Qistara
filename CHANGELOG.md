# Changelog

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

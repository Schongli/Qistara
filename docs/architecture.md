# How Qistara is built

Qistara is a [Tauri 2](https://tauri.app) app. The interface is [Svelte 5](https://svelte.dev) in the system webview (WebView2 on Windows,
WKWebView on macOS, WebKitGTK on Linux).
Playback is [libmpv](https://mpv.io), and everything that touches files, the network or secrets is Rust.

```
┌──────────────────────────── Qistara window ────────────────────────────┐
│  Svelte 5 page (src/)                                                   │
│   views/  components/  lib/*.svelte.ts (state)  lib/*.ts (pure logic)   │
│        │ invoke('command')                    │ cmd / set (mpv)          │
├────────┼──────────────────────────────────────┼──────────────────────────┤
│  Rust (src-tauri/src/)                        ▼                          │
│   lib.rs: commands, setup, migration     mpv.rs: allowlist ──► libmpv ──► WASAPI / CoreAudio / ALSA ──► DAC
│   db.rs  scan.rs  tags.rs  lyrics*.rs  ytdlp.rs  ytmusic.rs  session.rs  │
│   discord.rs  update.rs  providers  keys.rs                              │
└─────────────────────────────────────────────────────────────────────────┘
```

## Layout

| Path | What lives there |
|---|---|
| `src/views/` | One Svelte component per page (Albums, Settings, Equalizer, …) |
| `src/components/` | Shared pieces: player bar, Now Playing, lyrics, tag editor, icons |
| `src/lib/*.svelte.ts` | App state as Svelte runes: queue, player, settings, navigation, lyrics |
| `src/lib/*.ts` | Pure logic with Vitest tests next to it (`*.test.ts`): parsers, the bit-perfect verdict, EQ maths, smart playlists |
| `src-tauri/src/` | Rust commands, one module per concern |
| `scripts/` | Test media generator and the icon generator |

## Rust modules

| Module | Job |
|---|---|
| `lib.rs` | Tauri setup, the command list, data-folder migration, the `UA` constant used by every HTTP request |
| `db.rs` | SQLite library (`library.db`): tracks, playlists, stats and the `kv` table for app state |
| `scan.rs` | Folder scan and file watching |
| `tags.rs` | Tag reading and safe tag writes ([lofty](https://github.com/Serial-ATA/lofty-rs)) |
| `mpv.rs` | The only way the page reaches mpv: an allowlist of commands, properties and values |
| `lyrics.rs`, `lyrics_online.rs` | Local lyrics, online sources (LRCLIB, NetEase, Kugou) and their cache |
| `japanese.rs` | Offline Japanese word analysis ([Lindera](https://github.com/lindera/lindera)); the IPADIC dictionary is an optional download, md5-checked and built on the PC |
| `translate.rs`, `quran.rs` | Lyrics translation and Qur'an text, through fixed endpoints |
| `ytdlp.rs` | yt-dlp, ffmpeg and Deno install (checksum-verified), the JavaScript runtime check, downloads with fixed Video/Audio choices, per-run cookie copies |
| `ytmusic.rs` | YT Music as a guest (InnerTube `WEB_REMIX`): Home, search, albums, artists, playlists, radio; ids validated, thumbnails only from Google's image hosts |
| `session.rs` | Listen together: the host's server and the guest's loopback proxy (see below) |
| `discord.rs` | Discord Rich Presence over Discord's local IPC (named pipe / Unix socket), never the network |
| `update.rs` | Signed updates from GitHub releases (tauri-plugin-updater), check and install only |
| `subsonic.rs`, `spotify.rs`, `apple.rs`, `tidal.rs`, `lastfm.rs` | Providers |
| `keys.rs` | Secrets in the OS store: Windows Credential Manager, the macOS Keychain, Secret Service on Linux |
| `media_keys.rs` | Media keys and the system media controls (SMTC, MPNowPlayingInfo, MPRIS) |

## Playback

- **The queue lives in TypeScript** (`queue.svelte.ts`). mpv's own playlist only ever holds the current and the next
  file, which gives gapless playback without mpv owning the order.
- **State flows one way.** The page sets mpv properties through `mpv.rs` and reads state only from observed
  properties (`player.svelte.ts`). It never polls with `getProperty`.
- **Bit-perfect** is the default (`BIT_PERFECT` in `player.svelte.ts`): exclusive output, no resampling, no filters,
  volume sent to the device (`ao-volume`). Exclusive means WASAPI exclusive on Windows, CoreAudio hog mode on macOS
  (`coreaudio_exclusive`) and an ALSA `hw:`, `iec958:` or `hdmi:` device on Linux. `src/lib/signal.ts` compares what
  the decoder produced with what the output receives, and is the one place that decides whether playback is
  bit-perfect.
- **The EQ** is a chain of labelled FFmpeg biquad filters (`eq.ts`). Moving a band retunes its filter with
  `af-command`, so playback doesn't restart.
- **Remote tracks** (YT Music, YouTube, your server, radio, a session) have negative ids, so they never reach play
  stats, playlists or the restored queue. YouTube plays as `ytdl://` URLs through mpv's ytdl hook and Qistara's own
  yt-dlp, which needs a JavaScript runtime: Deno from Downloads, or Node.js 20+.

## Listen together

`session.rs`, plain `std::net` threads with `ring` for the crypto:

- **Invite** `qistara://session/<host>:<port>#<secret>`. Both sides send a nonce; keys for each direction come from
  HKDF-SHA256 over the secret and both nonces. Every frame is AES-256-GCM with a counter nonce, at most 1 MiB; any
  failure closes the connection.
- **The host shares track ids, never paths.** Rust looks each id up (`db::path_of`) and serves the file in ranges over
  the encrypted data connection. YouTube ids and radio URLs pass through; server (Subsonic) URLs are refused, because
  they carry the account's token. Removing a guest bans their address and changes the secret.
- **The guest plays from a loopback proxy** (`127.0.0.1`, Range requests, Host checked). mpv's ytdl hook skips
  `127.0.0.1`. The queue follows the host (`queue.follow`), drift over a second is corrected (`session.ts`), and
  the guest's own seek and pause are locked while following.

## Data

- The app data folder (`%LOCALAPPDATA%\com.qistara.app` on Windows) has `library.db`, which holds the library and, in its `kv` table, every piece of app state
  (settings, EQ, queue, radio). The page reaches it through `src/lib/persist.ts`, never through `localStorage`, which
  WebView2 can lose when the process is killed. State is loaded before it is saved, and a failed load stops saving.
- Artwork, lyrics found online and video resume points sit in folders next to it. `bin/` holds yt-dlp, ffmpeg and
  Deno, and `dict/ipadic/` the optional Japanese dictionary.
- **Streaming is kept apart** in `streaming/`: covers from your server and YT Music (`art/`, shown as `stream:` names),
  lyrics for remote tracks (`lyrics/`), per-run cookie copies (`session/`) and download scratch space (`tmp/`).
  Settings › General › Storage clears it without touching the library.
- Secrets (Last.fm keys, your server password, provider tokens) are in the OS secret store, read only by Rust.

## Rules that keep it safe

- **Fixed endpoints.** Every network call goes to a fixed URL, with inputs capped and ids validated before they reach
  a URL. Requests identify themselves only as `Qistara/<version>`.
- **Only what's needed leaves the PC.** Lyrics sources get title, artist, album and length. Translation gets the lyric
  lines. Learn mode gets the one word you tap.
- **Spotify, Apple Music and Tidal are import-only.** Their DRM streams are never played, recorded or downloaded.
- **yt-dlp** runs with fixed choices, `--ignore-config` and `--` before URLs, and only for http(s) links (mpv's hook
  gets `ignore-config` too). It, ffmpeg and Deno are installed only from their GitHub releases and must match the
  release's SHA-256 list. A YouTube sign-in's cookies reach yt-dlp only as a per-run copy that's deleted afterwards.
- **Discord** gets title, artist, album, times and an https cover, never a path, and only while it's switched on.
- **Updates** install only with a valid signature from the release key, bound to the version announced.
- **Tag writes** go to a copy: copy, write, read back, then replace with an atomic rename.
- **SQL** takes column names only from fixed enums, and always binds values.
- **The page can't drive mpv freely.** `mpv.rs` allowlists each command, property and value. Options that hold a
  path are set in Rust.
- **Strict CSP.** The page loads no remote code.

## Renames

Qistara was Sonora (0.1), then Confluence Player (0.2). On first start, `lib.rs` copies the newest old data folder
(`OLD_DATA`, newest first) once, and never deletes it. `keys.rs` carries the old Credential Manager entries over.

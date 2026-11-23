# Qistara

Lossless music + media player (formerly Confluence Player, before that Sonora). Tauri 2 (Rust) + Svelte 5 + libmpv. Windows-first;
macOS and Linux build and run from source (system libmpv), phones have a build scaffold only (`docs/mobile.md`).
Phases and status: `ROADMAP.md`.

## Commands
- `npm run tauri dev`: run the app (Windows needs `src-tauri/lib/*.dll`: `npx tauri-plugin-libmpv-api setup-lib`;
  macOS/Linux need any file in `src-tauri/lib/` for the resource glob, e.g. `.placeholder`)
- `npx vitest run` (TS logic) and `cargo test --lib --manifest-path src-tauri/Cargo.toml` (Rust).
  While `tauri dev` runs, add `--target-dir src-tauri/target-test`: the build step re-copies the libmpv DLL the app holds open.
  A Cargo.toml change rebuilds while the app runs and fails the same way: close the app, then restart `tauri dev`.
- IPADIC isn't embedded: `japanese_install` downloads it at runtime (fixed URL, md5 + size checked) and builds `dict/ipadic/`.
- Releases: tag `vX.Y.Z` → `.github/workflows/release.yml` (SignPath + updater signature); setup in `docs/code-signing.md`.
- `bash scripts/make-test-media.sh`: regenerate `test-media/` (FLAC 16/44.1 + 24/96, HEVC10 MKV w/ subs, chapters)
- `npm run tauri build`: NSIS installer (unsigned; CI: `.github/workflows/ci.yml` on all three OSes)

## Debugging
- Webview `console.*` and unhandled rejections are forwarded into the `tauri dev` output.
- Dev builds write mpv's own log to `%LOCALAPPDATA%\com.qistara.app\mpv.log` (WASAPI format negotiation, filter chain).
- Don't call `getProperty` from tauri-plugin-libmpv-api: it crashes libmpv-wrapper. State comes from observed properties.
- Integer mpv properties (sid, aid, chapter, sub-margin-y) must be set as text via `cmd('set', name, String(v))`.
- Tests from Claude's shell run inside an MSIX container: `%LOCALAPPDATA%` writes are redirected, Pictures and Music
  aren't writable (screenshots, the default Downloads folder fail there, not in the installed app).

## Invariants
- Bit-perfect mode is the default: `BIT_PERFECT` in `src/lib/player.svelte.ts`. Never set `audio-samplerate`,
  `audio-format`, filters, volume != 100 or speed != 1 while it is on. In this mode volume goes to `ao-volume`.
  `ao` is per OS (`AO` in signal.ts): wasapi, coreaudio (exclusive → `coreaudio_exclusive`, hog mode), alsa.
- `src/lib/signal.ts` is the single source of the bit-perfect verdict; extend it (with tests), don't duplicate it.
  Exclusive = WASAPI exclusive, `coreaudio_exclusive`, or an ALSA `hw:`/`iec958:`/`hdmi:` device.
- OS rules in `mpv.rs` are pure functions of an `Os` value (`check_prop_on`, `device`, `local_on`), so every CI runner
  tests all three. ALSA devices: `ALSA_DEVICE` (signal.ts) and `device()` (mpv.rs) accept the same names; change both, with tests.
- Queue lives in TS; mpv's playlist only ever holds [current, next] (gapless).
- The page reaches mpv only through `src-tauri/src/mpv.rs` (`cmd`/`set` in player.svelte.ts), which allowlists each
  command, property and value: a new one must be added there, with a test. Never grant `libmpv:default` or the plugin's
  command/set_property/init; options holding a path (script-opts, ytdl-raw-options, log/resume/screenshot dirs) are set in Rust.
  The hook's script-opts keep `ytdl_hook-exclude` for `127.0.0.1` (session URLs) and its raw options `ignore-config`.
  loadfile's per-file options are only `start=` and `ytdl-format=` + `VIDEO_FORMAT` (the same constant in mpv.rs and
  player.svelte.ts `loadTrack`, the one way the queue loads files). `video-add` only takes a local path, https on
  `*.googlevideo.com` or the session's `127.0.0.1:<port>`. A music video never replaces the song: the song stays the
  main file (bit-perfect, stats, lyrics) and the video is a picture-only track (`musicvideo.svelte.ts`).
- Tag writes: copy → write → re-read → atomic rename. Never write tags in place.
- SQL: column names from fixed enums, values always bound.
- No personal data in HTTP requests (User-Agent is `UA` in lib.rs / `ua.ts`: `Qistara/<version>`). Lyrics translation sends only the lyric lines;
  Learn-mode lookups send only the one word. Japanese analysis (`japanese.rs`) is offline.
- Providers (`spotify.rs`, `tidal.rs`, `apple.rs`, `subsonic.rs`): secrets stay in Rust (Credential Manager), commands
  map to fixed endpoints, IDs are validated before they reach a URL, and followed `next` links must stay on the
  service's own API path. Sign-out clears that service's kv import map (`spotifyImports`, `appleImports`,
  `tidalImports`). Apple's loopback page answers only `Host: 127.0.0.1:<port>`. Spotify, Apple Music and Tidal are
  import-only: never play, record or download their DRM streams.
- yt-dlp (`ytdlp.rs`): fixed `Choice`s only, always `--ignore-config` (also `-U`/`--version`) and `--` before URLs,
  URLs must be http(s), `--js-runtimes` from `js_runtime()` (bin/ Deno, Node 20+, Deno on PATH; else `NO_JS`).
  Cookies reach yt-dlp only as a per-run `CookieCopy` in `streaming/session/` (0600, deleted on Drop): yt-dlp writes
  back to it. mpv's hook gets its own copy only while signed in, swept at start and on exit.
  yt-dlp/ffmpeg/Deno are fetched only from their GitHub releases and must match the release's SHA-256 list.
- Streaming data lives in `streaming/` (art as `stream:<name>`, lyrics for `sub:`/`ytdl://`/`http`/`session:` keys,
  cookies, tmp), never in the library's `art/`/`lyrics/`; it must stay clearable without touching the library.
- YT Music (`ytmusic.rs`): fixed InnerTube endpoints, no cookies, no stored visitor id, ids validated (`valid_*`),
  thumbnails only from `THUMB_HOSTS` (same hosts in the CSP).
- Sessions (`session.rs`): the host shares track ids, never paths (`db::path_of` + `mpv::local`), and never Subsonic
  URLs (they carry the token). Frames are AES-256-GCM per direction with counter nonces, ≤ 1 MiB; guests ≤ 16, chat
  ≤ 500 chars. Kick bans the address and rekeys. The guest proxy listens on 127.0.0.1 only and checks Host.
- Discord (`discord.rs`): local IPC only; only title, artist, album, times and an https cover, only while the setting
  is on; never a path.
- Updates (`update.rs`): the page gets only `update_check`/`update_install`, never the plugin's commands
  (`updater:default`); the endpoint is fixed in tauri.conf.json and `requireSignedVersion` stays on.
- YouTube tracks are `ytdl://<watch url>` played by mpv's ytdl hook (`ytdl_hook-all_formats=no`, bestaudio); remote
  tracks (YouTube, server, radio) have negative ids, so they never reach stats, playlists or the restored queue.
- Rename: the data folder was `com.confluenceplayer.app`, before that `com.sonora.player` (`OLD_DATA` in lib.rs, newest
  first); `migrate()` copies the newest once and never deletes it, and `keys.rs` carries "Confluence Player …" and
  "Sonora …" Credential Manager entries over (and forgets all of them on sign-out).
- Translation: Google calls go through `gtx()` (translate.svelte.ts): one at a time, a second apart, a batch per
  request (`alignSegments`), backing off on `google-busy:<s>`; never line-by-line bursts. Offline translation
  (`translate_local.rs`, `mt.worker.ts`): models only from Mozilla's fixed bucket (registry `baseUrl` must match, paths
  checked), the model's SHA-256 checked, files to the worker as raw IPC bytes; CSP adds only `'wasm-unsafe-eval'`.
- Lyrics (`lyrics_online.rs`): each source is a fixed endpoint and gets only title/artist/album/length (a server song's
  id goes only to your own server). Results are cached per (track, source); LRCLIB keeps its old cache key.
- Icons go through `Icon.svelte` (Phosphor names); a new icon needs an entry in `scripts/icon-map.json`, then
  `node scripts/gen-icons.mjs`. Colours with a meaning use the signal tokens in `app.css` (`--signal-pure` = verified
  bit-perfect only); radii use `--r-*` (scaled by Settings › Corners) and blur uses `--blur`.
- App state (settings, EQ, queue, radio) lives in the SQLite `kv` table via `src/lib/persist.ts`, never localStorage
  (WebView2 loses recent localStorage writes when the process is killed). Load before saving.
- Switching exclusive mode reopens the current file (`queue.reload()`): mpv may not reopen the device on its own.
- Theme tokens use `light-dark()` in `app.css`; Appearance sets `color-scheme`/inline `--accent` (`appearance.svelte.ts`).
  A custom or artwork accent always goes through `readableAccent` (`color.ts`). Lyrics/Qur'an views are white on the
  dark blurred-cover backdrop in both Now Playing and the docked pane (`PanelContent.svelte`).
- Theme palettes (`themes.ts`) carry Qistara's own names (upstream schemes credited in the README) and must pass
  `themes.test.ts` contrast checks; `RENAMED` maps old ids. Adaptive theme = `adaptivePalette`, tested over all hues.
- Layout: ≤900 px icon rail, ≤600 px drawer + sheet; rows play via `activate()` (`touch.ts`), not `ondblclick`.
- Keys live in `shortcuts.ts` (one table: App dispatches it, the Ctrl+/ sheet lists it, the user guide copies it).
  Menus that stand for songs start with `playItems` (Play, Shuffle, Play Next, Play Later) in `menu.svelte.ts`.
  The root's `has-caps` class means Caps.svelte draws the window buttons (not macOS, not phones).

## Skills (IWNOTS map)
- Always: `/ponytail`. Big feature: `/feature-dev`. Logic: `/test-driven-development`.
- UI: `/interface-design`, `/apple-design` (springs, sheets, materials), polish `/emil-design-eng`, `/impeccable polish`.
- Review: `/impeccable critique` or `@design-critic`, `/break-ui`, `/ponytail-review`, rust/typescript reviewer agents.
- Bugs: `/systematic-debugging`. Security (tags, Last.fm, HTTP): `/security-review`, `/supply-chain-risk-auditor`.
- Done: `/verification-before-completion`.

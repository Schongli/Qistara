# Qistara

Lossless music + media player (formerly Confluence Player, before that Sonora). Tauri 2 (Rust) + Svelte 5 + libmpv. Windows-first.
Phases and status: `ROADMAP.md`.

## Commands
- `npm run tauri dev`: run the app (needs `src-tauri/lib/*.dll`: `npx tauri-plugin-libmpv-api setup-lib`)
- `npx vitest run` (TS logic) and `cargo test --lib --manifest-path src-tauri/Cargo.toml` (Rust).
  While `tauri dev` runs, add `--target-dir src-tauri/target-test`: the build step re-copies the libmpv DLL the app holds open.
  A Cargo.toml change rebuilds while the app runs and fails the same way: close the app, then restart `tauri dev`.
- The first build downloads the IPADIC dictionary for Lindera (`lindera-ipadic` build script, md5-checked) and embeds it.
- `bash scripts/make-test-media.sh`: regenerate `test-media/` (FLAC 16/44.1 + 24/96, HEVC10 MKV w/ subs, chapters)
- `npm run tauri build`: NSIS installer

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
- `src/lib/signal.ts` is the single source of the bit-perfect verdict; extend it (with tests), don't duplicate it.
- Queue lives in TS; mpv's playlist only ever holds [current, next] (gapless).
- The page reaches mpv only through `src-tauri/src/mpv.rs` (`cmd`/`set` in player.svelte.ts), which allowlists each
  command, property and value: a new one must be added there, with a test. Never grant `libmpv:default` or the plugin's
  command/set_property/init; options holding a path (script-opts, ytdl-raw-options, log/resume/screenshot dirs) are set in Rust.
- Tag writes: copy → write → re-read → atomic rename. Never write tags in place.
- SQL: column names from fixed enums, values always bound.
- No personal data in HTTP requests (User-Agent is `UA` in lib.rs / `ua.ts`: `Qistara/<version>`). Lyrics translation sends only the lyric lines;
  Learn-mode lookups send only the one word. Japanese analysis (`japanese.rs`) is offline.
- Providers (`spotify.rs`, `tidal.rs`, `apple.rs`, `subsonic.rs`): secrets stay in Rust (Credential Manager), commands
  map to fixed endpoints, IDs are validated before they reach a URL, and followed `next` links must stay on the
  service's own API path. Sign-out clears that service's kv import map (`spotifyImports`, `appleImports`,
  `tidalImports`). Apple's loopback page answers only `Host: 127.0.0.1:<port>`. Spotify, Apple Music and Tidal are
  import-only: never play, record or download their DRM streams.
- yt-dlp (`ytdlp.rs`): fixed presets only, always `--ignore-config` and `--` before URLs, URLs must be http(s).
  A cookies file reaches yt-dlp (and mpv's ytdl hook) only as a checked copy in app data: yt-dlp writes back to it.
  yt-dlp/ffmpeg are fetched only from their GitHub releases and must match the release's SHA-256 list.
- YouTube tracks are `ytdl://<watch url>` played by mpv's ytdl hook (`ytdl_hook-all_formats=no`, bestaudio); remote
  tracks (YouTube, server, radio) have negative ids, so they never reach stats, playlists or the restored queue.
- Rename: the data folder was `com.confluenceplayer.app`, before that `com.sonora.player` (`OLD_DATA` in lib.rs, newest
  first); `migrate()` copies the newest once and never deletes it, and `keys.rs` carries "Confluence Player …" and
  "Sonora …" Credential Manager entries over (and forgets all of them on sign-out).
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

## Skills (IWNOTS map)
- Always: `/ponytail`. Big feature: `/feature-dev`. Logic: `/test-driven-development`.
- UI: `/interface-design`, `/apple-design` (springs, sheets, materials), polish `/emil-design-eng`, `/impeccable polish`.
- Review: `/impeccable critique` or `@design-critic`, `/break-ui`, `/ponytail-review`, rust/typescript reviewer agents.
- Bugs: `/systematic-debugging`. Security (tags, Last.fm, HTTP): `/security-review`, `/supply-chain-risk-auditor`.
- Done: `/verification-before-completion`.

# Sonora

Apple-style lossless music + media player. Tauri 2 (Rust) + Svelte 5 + libmpv. Windows-first.
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
- Dev builds write mpv's own log to `%LOCALAPPDATA%\com.sonora.player\mpv.log` (WASAPI format negotiation, filter chain).
- Don't call `getProperty` from tauri-plugin-libmpv-api: it crashes libmpv-wrapper. State comes from observed properties.
- Integer mpv properties (sid, aid, chapter, sub-margin-y) must be set as text via `cmd('set', name, String(v))`.

## Invariants
- Bit-perfect mode is the default: `BIT_PERFECT` in `src/lib/player.svelte.ts`. Never set `audio-samplerate`,
  `audio-format`, filters, volume != 100 or speed != 1 while it is on. In this mode volume goes to `ao-volume`.
- `src/lib/signal.ts` is the single source of the bit-perfect verdict; extend it (with tests), don't duplicate it.
- Queue lives in TS; mpv's playlist only ever holds [current, next] (gapless).
- Tag writes: copy → write → re-read → atomic rename. Never write tags in place.
- SQL: column names from fixed enums, values always bound.
- No personal data in HTTP requests (User-Agent is `Sonora/<version>`). Lyrics translation sends only the lyric lines;
  Learn-mode lookups send only the one word. Japanese analysis (`japanese.rs`) is offline.
- Spotify (`spotify.rs`): tokens stay in Rust (refresh token in Credential Manager), commands map to fixed
  endpoints, IDs are validated before they reach a URL. Disconnect must also clear kv `spotifyImports`.
- App state (settings, EQ, queue, radio) lives in the SQLite `kv` table via `src/lib/persist.ts`, never localStorage
  (WebView2 loses recent localStorage writes when the process is killed). Load before saving.
- Switching exclusive mode reopens the current file (`queue.reload()`): mpv may not reopen the device on its own.

## Skills (IWNOTS map)
- Always: `/ponytail`. Big feature: `/feature-dev`. Logic: `/test-driven-development`.
- UI: `/interface-design`, `/apple-design` (springs, sheets, materials), polish `/emil-design-eng`, `/impeccable polish`.
- Review: `/impeccable critique` or `@design-critic`, `/break-ui`, `/ponytail-review`, rust/typescript reviewer agents.
- Bugs: `/systematic-debugging`. Security (tags, Last.fm, HTTP): `/security-review`, `/supply-chain-risk-auditor`.
- Done: `/verification-before-completion`.

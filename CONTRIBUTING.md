# Contributing to Qistara

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

## Reporting a bug

[Open an issue](https://github.com/Schongli/Qistara/issues) with:
- what you did, what you expected and what happened instead,
- your Windows version, your audio device or DAC, and the file format if it's about playback,
- for playback problems, a screenshot of the **Signal Path** (click the format badge in the player bar).

Security problems go privately instead: see [SECURITY.md](SECURITY.md).

## Building

You need Windows 10/11 x64, [Node.js](https://nodejs.org) 22+, [Rust](https://rustup.rs) (stable, MSVC) and Visual
Studio Build Tools with "Desktop development with C++".

```bash
npm install
npx tauri-plugin-libmpv-api setup-lib   # libmpv-2.dll and libmpv-wrapper.dll into src-tauri/lib
npm run tauri dev                       # run the app with hot reload
```

`bash scripts/make-test-media.sh` (needs ffmpeg) makes a demo library in `test-media/`: hi-res FLAC, ALAC and MP3
albums with covers and synced lyrics, and an HEVC 10-bit video with subtitles and chapters. The screenshots in the
README use it.

## Tests

```bash
npx vitest run                                        # TypeScript logic
cargo test --lib --manifest-path src-tauri/Cargo.toml # Rust
npm run check                                         # Svelte and TypeScript types
```

While `tauri dev` is running, add `--target-dir src-tauri/target-test` to the cargo command: the normal build
re-copies the libmpv DLL that the running app holds open. A change to `Cargo.toml` while the app runs fails the same
way, so close the app and restart `tauri dev`.

New logic (a parser, a rule, anything with branches) comes with a test next to it.

## Debugging

- The page's `console.*` output and unhandled errors show up in the `tauri dev` terminal.
- Dev builds write mpv's own log to `%LOCALAPPDATA%\com.qistara.app\mpv.log`. It shows WASAPI format negotiation and
  the filter chain.

## Rules the code keeps

[docs/architecture.md](docs/architecture.md) explains how the parts fit. A pull request must keep these true:

- **Bit-perfect stays bit-perfect.** While it is on, never set `audio-samplerate`, `audio-format`, filters, a volume
  other than 100 or a speed other than 1. `src/lib/signal.ts` is the only place that decides the verdict. Extend it,
  with tests, rather than adding a second check.
- **mpv goes through `mpv.rs`.** A new mpv command, property or value is added to its allowlist, with a test.
- **No personal data in requests.** Each service gets only what its feature needs, from a fixed endpoint, with ids
  validated. Secrets stay in Rust and Windows Credential Manager.
- **No DRM streams.** Spotify, Apple Music and Tidal are import-only.
- **Safe tag writes:** copy, write, read back, then rename. Never write tags in place.
- **State in `kv`, not `localStorage`**, through `src/lib/persist.ts`. Load before saving.
- **Icons** go through `Icon.svelte` with Phosphor names. A new icon needs an entry in `scripts/icon-map.json`, then
  `node scripts/gen-icons.mjs`.
- **Colours with a meaning** use the signal tokens in `src/app.css`: `--signal-pure` (cyan) only for verified
  bit-perfect playback.
- **Dependencies:** pin exact versions, and add one only when the code would otherwise be more than a few lines.

## Pull requests

- Keep each pull request to one change, and say what it fixes or adds and how you checked it.
- Run the tests and `npm run check` before you open it.
- For anything you can see, add a before/after screenshot, in both light and dark mode.
- By sending a pull request you agree that your work is released under the [MIT License](LICENSE).

# Qistara on phones (Tauri mobile)

Qistara is a desktop app first. The code is ready to be built for Android and iOS, and the interface already works
at phone width with touch, but **playback on phones isn't there yet**. This page says what's done, what's missing
and how to try it.

## What's ready

- **Layout and touch.** Below 600 px wide the sidebar becomes a drawer (the menu button, top left), the player bar
  keeps play, next and the pane buttons (tap the cover for Now Playing), the side pane becomes a sheet, and song
  lists drop the album and rating columns. On a touch screen one tap plays a song (a mouse double-clicks), a long
  press opens the menu a right-click does, and buttons are at least 44 px.
- **The Rust side builds without desktop-only parts.** libmpv, media keys and the OS secret store are desktop-only
  dependencies (`src-tauri/Cargo.toml`). On a phone:
  - the player answers "Playback on phones isn't ready yet." (`mpv.rs`),
  - media keys do nothing (`media_keys.rs`),
  - sign-ins aren't kept (`keys.rs`), so providers that need one can't be connected.
- **Permissions** for phones are in `src-tauri/capabilities/mobile.json` (no libmpv, no window controls).

## What's missing

**A player.** Playback goes through libmpv via `tauri-plugin-libmpv`, which is desktop-only. A phone needs a native
player plugin (ExoPlayer/Media3 on Android, AVPlayer on iOS) behind the same small surface `player.svelte.ts` uses:
load, append (gapless), pause, seek, volume, and the observed properties (time, duration, pause, audio params).
Android can't play bit-perfect through the system mixer anyway; a USB DAC with an exclusive path would be its own
project.

Also not on phones yet: yt-dlp downloads (no bundled Python binary), the folder watcher, and Credential-Manager-style
sign-ins.

## Trying it

You need the Android SDK and NDK (Android Studio) or Xcode on a Mac, then:

```bash
npm install
npx tauri android init
npx tauri android dev
```

or `npx tauri ios init` and `npx tauri ios dev`. The generated `src-tauri/gen/android` and `src-tauri/gen/apple`
projects aren't committed. None of this has been built yet: expect to fix small things the first time.

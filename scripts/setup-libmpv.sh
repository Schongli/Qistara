#!/usr/bin/env bash
# libmpv for macOS and Linux builds (on Windows: npx tauri-plugin-libmpv-api setup-lib).
#
# Puts the mpv plugin's libmpv-wrapper in src-tauri/lib/, from one pinned release of nini22P/libmpv-wrapper, checked
# against the SHA-256 that release publishes. The wrapper loads libmpv at run time from its own folder, else by name:
# - Linux: the system's libmpv.so.2 (Debian/Ubuntu libmpv2, Fedora mpv-libs, Arch mpv).
# - macOS: Homebrew's, through libmpv.dylib built here next to the wrapper: an empty library that re-exports
#   $(brew --prefix mpv)/lib/libmpv.2.dylib. Homebrew on Apple Silicon is in /opt/homebrew, where a bare name isn't
#   looked for. Without mpv installed the app says how to install it (PlaybackSetup.svelte).
#
# A new wrapper version: change VERSION and the hashes below to that release's sha256.txt.
set -euo pipefail

VERSION=v0.1.1
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) asset=linux-x86_64 sha=1583564042f10be25166b52b6fe02db6d87cf9cef34985ba517048c044b7eee0 lib=libmpv-wrapper.so ;;
  Linux-aarch64) asset=linux-aarch64 sha=94ff6465ae05a86b44e818089270af857657251166bd693caf4a87a53821d943 lib=libmpv-wrapper.so ;;
  Darwin-arm64) asset=macos-aarch64 sha=086dbf977fe5b51785048cbd190f1809186513688be0ecea1ad27bd766c55de8 lib=libmpv-wrapper.dylib ;;
  Darwin-x86_64) asset=macos-x86_64 sha=99d65d7e368a456c883b4b3cf5d6384c9af64ec29dccaf4923548c2e3784b0e4 lib=libmpv-wrapper.dylib ;;
  *)
    echo "setup-libmpv: no wrapper for $(uname -s) $(uname -m). On Windows run: npx tauri-plugin-libmpv-api setup-lib" >&2
    exit 1
    ;;
esac

dir="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/lib"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$dir"

curl -fsSL --retry 3 -o "$tmp/wrapper.zip" \
  "https://github.com/nini22P/libmpv-wrapper/releases/download/$VERSION/libmpv-wrapper-$asset.zip"
got="$( (sha256sum "$tmp/wrapper.zip" 2>/dev/null || shasum -a 256 "$tmp/wrapper.zip") | cut -d' ' -f1)"
if [ "$got" != "$sha" ]; then
  echo "setup-libmpv: libmpv-wrapper-$asset.zip has SHA-256 $got, expected $sha" >&2
  exit 1
fi
unzip -p "$tmp/wrapper.zip" "bin/$lib" > "$tmp/$lib"
mv "$tmp/$lib" "$dir/$lib"
echo "setup-libmpv: $dir/$lib ($VERSION)"

if [ "$(uname -s)" = Darwin ]; then
  if ! prefix="$(brew --prefix mpv 2>/dev/null)" || [ ! -e "$prefix/lib/libmpv.2.dylib" ]; then
    echo "setup-libmpv: Homebrew's mpv isn't installed. Run: brew install mpv" >&2
    exit 1
  fi
  # the re-export keeps Homebrew's own path (opt/mpv, the same across mpv versions), so the app needs no copy of mpv
  printf '' | clang -x c -dynamiclib -o "$dir/libmpv.dylib" - \
    -Wl,-reexport_library,"$prefix/lib/libmpv.2.dylib" -install_name @rpath/libmpv.dylib -mmacosx-version-min=13.0
  echo "setup-libmpv: $dir/libmpv.dylib → $(otool -L "$dir/libmpv.dylib" | awk '/libmpv\.2/ {print $1}')"
elif ! { ldconfig -p 2>/dev/null || true; } | grep -q 'libmpv\.so\.2 '; then
  echo "setup-libmpv: note: libmpv.so.2 isn't installed; for playback install libmpv2 (Debian/Ubuntu), mpv-libs (Fedora) or mpv (Arch)" >&2
fi

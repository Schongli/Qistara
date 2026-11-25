#!/usr/bin/env bash
# Starts a packaged Qistara for a while and passes when its log says "mpv started" (mpv.rs mpv_init): the package
# found libmpv-wrapper and the wrapper found libmpv, the parts a build can't check. CI runs it on macOS and Linux.
#
#   scripts/smoke.sh <program> [args…]
#     Linux .deb:  xvfb-run -a scripts/smoke.sh qistara
#     AppImage:    xvfb-run -a scripts/smoke.sh ./Qistara.AppImage --appimage-extract-and-run
#     macOS:       scripts/smoke.sh /Applications/Qistara.app/Contents/MacOS/qistara
#
# Leaves smoke.png (a screenshot, when there's a tool for it) and smoke-out.log (the app's output) here.
set -euo pipefail

case "$(uname -s)" in
  Darwin) log="$HOME/Library/Logs/com.qistara.app/Qistara.log" ;;
  *) log="${XDG_DATA_HOME:-$HOME/.local/share}/com.qistara.app/logs/Qistara.log" ;;
esac
rm -f "$log"

"$@" > smoke-out.log 2>&1 &
pid=$!
result=timeout
for _ in $(seq 90); do
  sleep 1
  if grep -q "mpv started" "$log" 2>/dev/null; then result=started; break; fi
  if grep -q "mpv didn't start" "$log" 2>/dev/null; then result=failed; break; fi
  if ! kill -0 "$pid" 2>/dev/null; then result=exited; break; fi
done

# a few seconds more so the window has drawn, then a picture of the screen for the job's artifacts
sleep 4
if [ "$(uname -s)" = Darwin ]; then
  screencapture -x smoke.png 2>/dev/null || true
elif command -v import > /dev/null; then
  import -window root smoke.png 2>/dev/null || true
fi
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

echo "--- Qistara.log ($log)"
cat "$log" 2>/dev/null || echo "(no log)"
if [ "$result" != started ]; then
  echo "--- output"
  cat smoke-out.log
  echo "smoke: mpv didn't start ($result)" >&2
  exit 1
fi
echo "smoke: mpv started"

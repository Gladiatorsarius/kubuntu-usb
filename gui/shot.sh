#!/bin/bash
# Usage: shot.sh <output-name> [app args...]
# Runs the app under Xvfb, screenshots the screen, kills the app.
set -u
name="$1"; shift
cd "$HOME/kubuntu-usb/gui"
xvfb-run -a -s "-screen 0 1280x800x24" bash -c '
  ./target/debug/kubuntu-usb-sync "$@" &
  PID=$!
  sleep 7
  scrot -o "/downloads/'"$name"'.png"
  kill $PID 2>/dev/null
' _ "$@"
echo "saved /downloads/$name.png"

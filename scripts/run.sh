#!/usr/bin/env bash
# Starts Xvfb, a window manager (egui needs input focus to receive typing)
# and the launcher on display :99.
#   bash scripts/run.sh          start
#   bash scripts/run.sh stop     kill everything
set -u

export DISPLAY=:99

if [ "${1:-start}" = stop ]; then
    pkill -x rus-launcher 2>/dev/null
    pkill -x matchbox-window-manager 2>/dev/null
    pkill -x Xvfb 2>/dev/null
    echo stopped
    exit 0
fi

pgrep -x Xvfb >/dev/null || (Xvfb :99 -screen 0 1440x900x24 &)
sleep 1
pgrep -x matchbox-window-manager >/dev/null || (matchbox-window-manager -use_titlebar no &)
sleep 1

pkill -x rus-launcher 2>/dev/null
sleep 1

cd "$(dirname "$0")/../rus-launcher"
DISPLAY=:99 LIBGL_ALWAYS_SOFTWARE=1 nohup ./target/release/rus-launcher >/tmp/rus-launcher.log 2>&1 &
sleep 6
DISPLAY=:99 xdotool search --onlyvisible --name . | head -1

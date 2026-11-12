#!/usr/bin/env bash
# Remakes every Help screenshot in assets/help/ from the real program.
# Needs: a built target/debug/ilex (or ILEX=path; MINIMAX=path still works), Xvfb, (no window manager needed)
# ImageMagick (`import`, `convert`). Usage: tools/help_shots.sh [id ...]
set -euo pipefail
cd "$(dirname "$0")/.."
BIN=${ILEX:-${MINIMAX:-target/debug/ilex}}
OUT=assets/help
WAIT=${WAIT:-14}
IDS=("$@")
if [ ${#IDS[@]} -eq 0 ]; then
  IDS=(layout create transform editpoly ribbon modifiers splines materials library procedural paint sky render grading ghosttown mech cad prefs crash characters wardrobe)
fi
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"; kill "${XPID:-0}" 2>/dev/null || true' EXIT
if [ -z "${DISPLAY:-}" ] || [ -n "${HELP_SHOTS_XVFB:-}" ]; then
  Xvfb :97 -screen 0 1600x920x24 >/dev/null 2>&1 & XPID=$!
  export DISPLAY=:97
  sleep 2
fi
mkdir -p "$TMP/cfg" "$TMP/data"
for id in "${IDS[@]}"; do
  XDG_CONFIG_HOME="$TMP/cfg" XDG_DATA_HOME="$TMP/data" "$BIN" --help-shot "$id" >/dev/null 2>&1 & APP=$!
  sleep "$WAIT"
  import -window root "$TMP/$id.png"
  kill $APP; wait $APP 2>/dev/null || true
  convert "$TMP/$id.png" -resize 1200x -quality 82 "$OUT/$id.jpg"
  echo "$OUT/$id.jpg"
done

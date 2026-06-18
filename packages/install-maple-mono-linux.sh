#!/bin/sh
set -eu

FONT_NAME="Maple Mono NF CN"
FONT_DIR="${HOME}/.local/share/fonts/MapleMono-NF-CN"
FONT_URL="https://github.com/subframe7536/maple-font/releases/latest/download/MapleMono-NF-CN.zip"

if [ "$(uname -s)" != "Linux" ]; then
  echo "skip: Maple Mono NF CN Linux font install only runs on Linux"
  exit 0
fi

for command_name in curl unzip fc-cache fc-list find; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing dependency: $command_name" >&2
    exit 1
  fi
done

if fc-list | grep -qi "$FONT_NAME"; then
  echo "font already installed: $FONT_NAME"
  exit 0
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT HUP INT TERM

mkdir -p "$FONT_DIR"
curl -L "$FONT_URL" -o "$tmpdir/MapleMono-NF-CN.zip"
unzip -q "$tmpdir/MapleMono-NF-CN.zip" -d "$tmpdir/MapleMono-NF-CN"
find "$tmpdir/MapleMono-NF-CN" -type f \( -name '*.ttf' -o -name '*.otf' \) \
  -exec cp {} "$FONT_DIR"/ \;
fc-cache -f "$HOME/.local/share/fonts"

if fc-list | grep -qi "$FONT_NAME"; then
  echo "installed font: $FONT_NAME"
else
  echo "installed font files, but fontconfig did not report: $FONT_NAME" >&2
  exit 1
fi

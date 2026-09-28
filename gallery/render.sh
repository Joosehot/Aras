#!/usr/bin/env bash
# Regenerates the gallery: an SVG and a PNG preview for every line of
# sentences.txt ("name|sentence"), plus a cutting layout and a paper pattern
# for one garment. PNGs are rendered with headless Chrome.
#
#   bash gallery/render.sh
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -q
ARAS=target/release/aras
CHROME=${CHROME:-"/c/Program Files/Google/Chrome/Application/chrome.exe"}
command -v "$CHROME" >/dev/null 2>&1 || CHROME=$(command -v google-chrome || command -v chromium)
OUT=gallery

png() { # svg png scale
  local svg=$1 png=$2 k=$3
  read -r w h < <(grep -o 'viewBox="0 0 [0-9.]* [0-9.]*"' "$svg" | head -1 | awk '{gsub(/"/,""); print $3, $4}')
  local pw ph
  pw=$(awk -v a="$w" -v k="$k" 'BEGIN{printf "%d", a*k}')
  ph=$(awk -v a="$h" -v k="$k" 'BEGIN{printf "%d", a*k}')
  local tmp="${svg%.svg}.px.svg"
  sed "0,/width=\"[0-9.]*mm\" height=\"[0-9.]*mm\"/s//width=\"$pw\" height=\"$ph\"/" "$svg" > "$tmp"
  local abs
  abs=$(cd "$(dirname "$tmp")" && pwd -W 2>/dev/null || pwd)
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --screenshot="$abs/$(basename "$png")" \
    --window-size="$pw,$ph" "file:///$abs/$(basename "$tmp")" >/dev/null 2>&1
  rm -f "$tmp"
}

while IFS='|' read -r name sentence; do
  [ -z "$name" ] && continue
  "$ARAS" "$sentence" -o "$OUT/$name.svg"
  png "$OUT/$name.svg" "$OUT/$name.png" 2.2
  echo "$name"
done < "$OUT/sentences.txt"

s="make an oversized black hoodie with white sleeves, a grey hood and rib cuffs"
"$ARAS" "$s" --marker -o "$OUT/cutting_layout.svg"
png "$OUT/cutting_layout.svg" "$OUT/cutting_layout.png" 0.45
"$ARAS" "make a red gingham shirt" --pieces -o "$OUT/paper_pattern.svg"
png "$OUT/paper_pattern.svg" "$OUT/paper_pattern.png" 0.8
echo done

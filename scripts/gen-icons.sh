#!/bin/sh
# Regenerate the committed icon derivatives from assets/icon/crew.svg.
# Dev-machine only — outputs are committed so CI/builds need no SVG tooling.
# Requires: rsvg-convert (brew install librsvg), ImageMagick `magick`
# (brew install imagemagick); iconutil is macOS built-in.
set -e
cd "$(dirname "$0")/.."
SRC=assets/icon/crew.svg
OUT=assets/icon

for s in 16 32 64 128 256 512 1024; do
    rsvg-convert -w "$s" -h "$s" "$SRC" -o "$OUT/tmp-$s.png"
done

# Linux hicolor sizes + runtime-embedded PNGs (committed).
for s in 32 128 256 512; do
    cp "$OUT/tmp-$s.png" "$OUT/crew-$s.png"
done

# Windows multi-size .ico (committed; embedded by build.rs).
magick "$OUT/tmp-16.png" "$OUT/tmp-32.png" "$OUT/tmp-64.png" \
    "$OUT/tmp-128.png" "$OUT/tmp-256.png" "$OUT/crew.ico"

# macOS .icns (committed; written into Crew.app/Contents/Resources).
if command -v iconutil >/dev/null 2>&1; then
    ISET="$OUT/crew.iconset"
    rm -rf "$ISET" && mkdir "$ISET"
    cp "$OUT/tmp-16.png"   "$ISET/icon_16x16.png"
    cp "$OUT/tmp-32.png"   "$ISET/icon_16x16@2x.png"
    cp "$OUT/tmp-32.png"   "$ISET/icon_32x32.png"
    cp "$OUT/tmp-64.png"   "$ISET/icon_32x32@2x.png"
    cp "$OUT/tmp-128.png"  "$ISET/icon_128x128.png"
    cp "$OUT/tmp-256.png"  "$ISET/icon_128x128@2x.png"
    cp "$OUT/tmp-256.png"  "$ISET/icon_256x256.png"
    cp "$OUT/tmp-512.png"  "$ISET/icon_256x256@2x.png"
    cp "$OUT/tmp-512.png"  "$ISET/icon_512x512.png"
    cp "$OUT/tmp-1024.png" "$ISET/icon_512x512@2x.png"
    iconutil -c icns "$ISET" -o "$OUT/crew.icns"
    rm -rf "$ISET"
else
    echo "warning: iconutil not found — crew.icns NOT regenerated" >&2
fi

rm -f "$OUT"/tmp-*.png
echo "Icons regenerated in $OUT"

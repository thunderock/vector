#!/usr/bin/env bash
# Deterministic DMG background renderer. Run from repo root:
#   bash xtask/scripts/render-dmg-bg.sh > crates/vector-app/resources/dmg-background.png
# Required: ImageMagick `convert` on PATH (brew install imagemagick).
# Source strings (load-bearing — grep checks):
#   - "Vector"
#   - "If macOS blocks the app, run this in Terminal:"
#   - "xattr -dr com.apple.quarantine /Applications/Vector.app"
#   - "https://github.com/<owner>/vector"
# Font paths point at the macOS system .ttc files because Homebrew's
# ImageMagick ships without a fontconfig database; name-based lookups
# ("Helvetica-Bold", "Courier") fail. .ttc collections resolve cleanly.
# Expected SHA-256 of the produced PNG: see CI artifact + 01-04-SUMMARY.md.
set -euo pipefail

HELVETICA="${HELVETICA_TTC:-/System/Library/Fonts/Helvetica.ttc}"
COURIER="${COURIER_TTC:-/System/Library/Fonts/Courier.ttc}"

convert -size 1280x800 xc:'#1A1A1A' \
  -gravity NorthWest -fill '#FFFFFF' -font "$HELVETICA" -pointsize 28 \
  -annotate +64+48 'Vector' \
  -gravity South -fill '#FFFFFF' -font "$HELVETICA" -pointsize 28 \
  -annotate +0+160 'If macOS blocks the app, run this in Terminal:' \
  -gravity South -fill '#FFFFFF' -font "$COURIER" -pointsize 22 \
  -annotate +0+96 'xattr -dr com.apple.quarantine /Applications/Vector.app' \
  -gravity South -fill '#9A9A9A' -font "$HELVETICA" -pointsize 20 \
  -annotate +0+32 'https://github.com/<owner>/vector' \
  -colorspace sRGB -type TrueColor -depth 8 \
  -define png:color-type=2 -define png:bit-depth=8 \
  png:-

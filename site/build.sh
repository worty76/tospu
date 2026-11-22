#!/usr/bin/env bash
# Assembles the landing page into _site/, copying companion sheets from sprites/.
set -euo pipefail
cd "$(dirname "$0")/.."
COMPANIONS=(meow-okay pip watermelon-dog)
rm -rf _site
mkdir -p _site/assets/companions
cp site/index.html _site/
cp -R site/assets/. _site/assets/
for p in "${COMPANIONS[@]}"; do
  mkdir -p "_site/assets/companions/$p"
  cp "sprites/$p/"{idle,wave,cheer,sleep}.png "_site/assets/companions/$p/"
done
echo "Built _site/. Preview with: python3 -m http.server -d _site 8000"

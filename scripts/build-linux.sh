#!/usr/bin/env bash
# Linux build (.deb + .AppImage). Linuxon vagy WSL-ben futtasd:
#   bash scripts/build-linux.sh [projekt-mappa]
# A Windows-os node_modules és target nem használható Linuxon, ezért a forrást
# egy Linux-oldali munkamappába másoljuk, és ott fordítunk.
set -euo pipefail

SRC="$(cd "${1:-$(dirname "$0")/..}" && pwd)"
WORK="${KRIPTO_WORK:-$HOME/build/kriptomerleg}"
OUT="$SRC/telepito/linux"

export PATH="$HOME/.local/node/bin:$HOME/.cargo/bin:$PATH"
export APPIMAGE_EXTRACT_AND_RUN=1
export NO_STRIP=true

for tool in node npm cargo pkg-config; do
  command -v "$tool" >/dev/null || { echo "Hiányzik: $tool" >&2; exit 1; }
done
pkg-config --exists webkit2gtk-4.1 || {
  echo "Hiányzik a webkit2gtk-4.1. Telepítsd:" >&2
  echo "  sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev" >&2
  exit 1
}

echo "==> Forrás másolása: $SRC -> $WORK"
mkdir -p "$WORK"
tar -C "$SRC" \
  --exclude=./node_modules --exclude=./dist --exclude=./telepito \
  --exclude=./src-tauri/target --exclude=./src-tauri/gen \
  -cf - . | tar -C "$WORK" -xf -

cd "$WORK"
echo "==> npm ci"
npm ci --no-audit --no-fund
echo "==> tauri build (deb, appimage)"
npm run tauri build -- --bundles deb,appimage

VERSION="$(node -p "require('./package.json').version")"
mkdir -p "$OUT"
rm -f "$OUT"/*.deb "$OUT"/*.AppImage
cp "$(ls -t src-tauri/target/release/bundle/deb/*.deb | head -1)" "$OUT/kriptomerleg_${VERSION}_linux_amd64.deb"
cp "$(ls -t src-tauri/target/release/bundle/appimage/*.AppImage | head -1)" "$OUT/kriptomerleg_${VERSION}_linux_amd64.AppImage"
echo "==> Kész:"
ls -la "$OUT"

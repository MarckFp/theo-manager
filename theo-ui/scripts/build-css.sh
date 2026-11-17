#!/usr/bin/env sh
# Builds assets/theo-ui.css from css/input.css with the Tailwind CSS standalone CLI.
# No Node.js or npm (REQUIREMENTS.md 3.2, 8.3). Usage: ./scripts/build-css.sh [--watch]
set -eu
VERSION="v4.3.5"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOLS="$ROOT/tools"

# Checksums from the official sha256sums.txt of the release (pinned: a changed binary fails).
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  ASSET="tailwindcss-linux-x64";   SHA256="e47695cee3f0d90623a91f3939b08bdee610eb5ad99340adff9d4145b284d6cc" ;;
  Linux-aarch64) ASSET="tailwindcss-linux-arm64"; SHA256="ffcd0d21dbd7c605dc12743ff0c0b47debc963c86b399674084c1c8f2c96ecad" ;;
  Darwin-arm64)  ASSET="tailwindcss-macos-arm64"; SHA256="f3f4f5d1727e2b693b8f1c9a1492b5dd87008c86ea286bee1589cb458e6235fc" ;;
  Darwin-x86_64) ASSET="tailwindcss-macos-x64";   SHA256="a63702686a3ed048b3135d84a73be49ea9d47f880c9b20e54b3ef60934ad8cdf" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)"; exit 1 ;;
esac

BIN="$TOOLS/$ASSET-$VERSION"
if [ ! -x "$BIN" ]; then
  mkdir -p "$TOOLS"
  echo "Downloading Tailwind CSS $VERSION ..."
  curl -sSL -o "$BIN" "https://github.com/tailwindlabs/tailwindcss/releases/download/$VERSION/$ASSET"
  chmod +x "$BIN"
fi
if command -v sha256sum >/dev/null 2>&1; then ACTUAL="$(sha256sum "$BIN" | cut -d' ' -f1)"; else ACTUAL="$(shasum -a 256 "$BIN" | cut -d' ' -f1)"; fi
if [ "$ACTUAL" != "$SHA256" ]; then rm -f "$BIN"; echo "Checksum mismatch for $ASSET"; exit 1; fi

"$BIN" -i "$ROOT/css/input.css" -o "$ROOT/assets/theo-ui.css" --minify "$@"

#!/usr/bin/env bash
# Generates every platform icon from the one source, theo-app/icons/icon.svg, into
# theo-app/icons/gen/ (not committed: only the source is kept in the repository).
#
# Output: Windows .ico, macOS .icns, PNGs for Linux, Android mipmaps (adaptive icon) and iOS
# AppIcon PNGs. theo-app/Dioxus.toml points at the files the bundler reads; the release workflow
# copies the Android and iOS ones into the generated projects.
#
# Needs the Tauri CLI (only its `icon` command is used):
#   cargo install tauri-cli --version "^2" --locked
#
# Usage: scripts/icons.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
source="$root/theo-app/icons/icon.svg"
out="$root/theo-app/icons/gen"

rm -rf "$out"
cargo tauri icon "$source" --output "$out"

# Android adaptive icon: the background behind the (full-bleed) foreground, in the brand colour.
sed -i.bak 's/#FFFFFF/#3756D6/' "$out/android/values/ic_launcher_background.xml"
rm "$out/android/values/ic_launcher_background.xml.bak"

echo "Icons written to $out"

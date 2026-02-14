#!/usr/bin/env bash
# Build PBR Studio as a Linux AppImage.
# Run from project root: ./scripts/build-appimage.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
UI_DIR="$PROJECT_ROOT/pbr-studio-ui"

cd "$UI_DIR"

# Ensure deps are installed
if ! command -v npm &>/dev/null; then
  echo "Error: npm is required. Install Node.js first."
  exit 1
fi

if ! command -v cargo &>/dev/null; then
  echo "Error: Rust/cargo is required. Install from https://rustup.rs"
  exit 1
fi

# Linux system deps (informational; script does not install them)
echo "Prerequisites: install Tauri Linux deps if not already present:"
echo "  Ubuntu/Debian: sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2"
echo ""

# Install frontend deps
npm ci

# Build AppImage only (skips deb, rpm)
echo "Building AppImage..."
npm run tauri build -- --bundles appimage --ci

# Output location
BUNDLE_DIR="$UI_DIR/src-tauri/target/release/bundle/appimage"

if [[ -d "$BUNDLE_DIR" ]]; then
  APPIMAGE=$(find "$BUNDLE_DIR" -name "*.AppImage" 2>/dev/null | head -1)
  if [[ -n "$APPIMAGE" ]]; then
    echo ""
    echo "✓ AppImage built: $APPIMAGE"
    echo "  Run with: chmod +x \"$APPIMAGE\" && \"$APPIMAGE\""
  fi
fi

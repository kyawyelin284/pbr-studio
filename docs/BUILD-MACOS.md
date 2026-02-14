# Building PBR Studio for macOS (DMG)

The macOS installer is built as a DMG (Apple Disk Image). **DMG can only be built on macOS** because it requires Apple's build tools.

## Prerequisites

### 1. Node.js and npm

Install Node.js LTS (v18 or newer). Using [Homebrew](https://brew.sh):

```bash
brew install node
```

### 2. Rust

Install Rust via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 3. Xcode Command Line Tools

Install the Xcode Command Line Tools (required for building):

```bash
xcode-select --install
```

For full Xcode (optional, for advanced development):

```bash
xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

## Build

### Option A: DMG only

From `pbr-studio-ui`:

```bash
npm ci
npm run dmg
```

Output: `src-tauri/target/release/bundle/dmg/pbr-studio-ui_0.1.0_aarch64.dmg` (Apple Silicon) or `pbr-studio-ui_0.1.0_x64.dmg` (Intel).

### Option B: Full build (all formats)

```bash
npm run tauri:build
```

This produces both `.app` and `.dmg` bundles.

### Option C: Build for specific architecture

```bash
# Apple Silicon (M1/M2/M3)
npm run tauri build -- --target aarch64-apple-darwin

# Intel Mac
npm run tauri build -- --target x86_64-apple-darwin

# Universal binary (both architectures)
npm run tauri build -- --target universal-apple-darwin
```

## Configuration

DMG settings are in `tauri.conf.json` under `bundle.macOS.dmg`:

- **windowSize**: 660×400 (default installer window size)
- **appPosition**: Icon position for the app
- **applicationFolderPosition**: Icon position for the Applications folder
- **background**: Custom background image (optional)

## Output location

DMG files are written to:

```
pbr-studio-ui/src-tauri/target/release/bundle/dmg/
```

## Code signing and notarization

For distribution outside the App Store, Apple recommends signing and notarizing your app. See [Tauri's macOS signing guide](https://tauri.app/distribute/sign/macos/) for details.

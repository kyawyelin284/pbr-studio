# Build Testing

Verification steps for Linux, Windows, and macOS builds. All builds run **offline**—no network during analysis.

## Prerequisites by Platform

### Linux

- **Rust**: `rustup`
- **Tauri/Desktop**: `pkg-config`, `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `libappindicator3-dev`, `libfuse2` (see [BUILD-APPIMAGE.md](BUILD-APPIMAGE.md))
- **CLI only**: No system deps beyond Rust

### macOS

- **Xcode Command Line Tools** or full Xcode
- **WebKit** (system)
- See [BUILD-MACOS.md](BUILD-MACOS.md)

### Windows

- **Visual Studio Build Tools** (C++)
- **WebView2** (Edge WebView2 runtime)
- See [BUILD-WINDOWS.md](BUILD-WINDOWS.md)

## Build Commands

### pbr-core (library)

```bash
# Minimal (no pdf, no ai)
cargo build -p pbr-core --no-default-features

# With PDF export
cargo build -p pbr-core --features pdf

# With AI/ONNX
cargo build -p pbr-core --features ai
```

### pbr-cli

```bash
# Full CLI (includes pdf for report --export pdf)
cargo build -p pbr-cli --release
```

**Note:** `pbr-cli` uses `pbr-core` with `pdf` feature. If genpdf has compatibility issues, build without: temporarily remove `pdf` from pbr-cli's pbr-core dependency.

### pbr-studio-ui (Tauri)

```bash
cd pbr-studio-ui
npm install
npm run tauri:build
```

For CI (non-interactive):

```bash
CI=false npm run tauri:build
```

## Offline Verification

1. **No network in analysis** – Validation, optimization, report generation use only local files
2. **Audit log** – `~/.config/pbr-studio/audit.json` or `PBR_STUDIO_AUDIT_PATH`
3. **Plugins** – Loaded from `./.pbr-studio/plugins`, `~/.config/pbr-studio/plugins`, `PBR_STUDIO_PLUGINS`
4. **AI** – Heuristics use pixel data only; ONNX models from local path

## Quick Test (CLI)

```bash
# Create a minimal material folder
mkdir -p /tmp/test-mat
# Add at least one texture (albedo, etc.)

# Validate
pbr-cli check /tmp/test-mat

# Report
pbr-cli report /tmp/test-mat --json
```

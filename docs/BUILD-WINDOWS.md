# Building PBR Studio for Windows (MSI)

The Windows installer is built as a Microsoft Installer (`.msi`) using the WiX Toolset. **MSI can only be built on Windows** because WiX runs natively on Windows only.

## Prerequisites

### 1. Node.js and npm

Install Node.js LTS (v18 or newer).

### 2. Rust

Install Rust via [rustup](https://rustup.rs):

```powershell
# In PowerShell
winget install Rustlang.Rustup
# Or: https://rustup.rs
```

### 3. Visual Studio Build Tools

Windows development requires the **Microsoft C++ build tools**. Install either:

- [Visual Studio 2022](https://visualstudio.microsoft.com/downloads/) (Community or higher) with the "Desktop development with C++" workload
- Or [Build Tools for Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the same workload

### 4. VBScript (for MSI)

MSI builds require the VBScript optional feature. It is usually enabled by default. If you see `failed to run light.exe`:

1. Open **Settings** → **Apps** → **Optional features**
2. Click **More Windows features**
3. Enable **Microsoft Visual Basic Script Edition (VBScript)**

## Build

### Option A: MSI only

From `pbr-studio-ui`:

```powershell
npm ci
npm run msi
```

Output: `src-tauri/target/release/bundle/msi/pbr-studio-ui_1.0.0_x64_en-US.msi`

### Option B: Full build (all formats)

```powershell
npm run tauri:build
```

This produces both MSI and NSIS (`.exe`) installers.

### Option C: GitHub Actions (tag-based)

Push a version tag (e.g. `v1.0.0`) to trigger cross-platform builds including Windows MSI. See [CI-RELEASE.md](CI-RELEASE.md).

## Configuration

MSI behavior is configured in `tauri.conf.json` under `bundle.windows`:

- **webviewInstallMode**: `embedBootstrapper` – embeds the WebView2 bootstrapper (~1.8 MB) for better compatibility, including Windows 7.
- **wix.language**: `en-US` – installer UI language.

To add more languages, use an array:

```json
"wix": {
  "language": ["en-US", "de-DE", "fr-FR"]
}
```

## Cross-compilation

Building Windows MSI from Linux or macOS is **not supported** (WiX is Windows-only). Options:

- Use a Windows machine or VM
- Use [GitHub Actions](https://tauri.app/distribute/pipelines/github/) with `windows-latest` runners

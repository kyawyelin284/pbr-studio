# PBR Studio UI

Desktop app for analyzing PBR texture sets. Built with Tauri + React.

## Layout

- **Left panel** – Texture inputs: material folder selector, texture slot inputs
- **Center** – 3D preview viewport (rotating sphere with PBR materials)
- **Right panel** – Validation results (issues from pbr-core)

## Run

### Web (dev)
```bash
npm run dev
```

### Desktop (Tauri)
Requires [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (e.g. on Ubuntu: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, etc.)

```bash
npm run tauri:dev
```

### Build
```bash
npm run build       # Web assets
npm run tauri:build # Desktop app (all formats)
npm run appimage    # Linux AppImage only
```

### AppImage (Linux)
To build a portable AppImage for Linux, see [docs/BUILD-APPIMAGE.md](../docs/BUILD-APPIMAGE.md). From project root:
```bash
./scripts/build-appimage.sh
```

### MSI (Windows)
To build a Windows `.msi` installer (Windows only), see [docs/BUILD-WINDOWS.md](../docs/BUILD-WINDOWS.md):
```powershell
cd pbr-studio-ui && npm run msi
```

### DMG (macOS)
To build a macOS `.dmg` installer (macOS only), see [docs/BUILD-MACOS.md](../docs/BUILD-MACOS.md):
```bash
cd pbr-studio-ui && npm run dmg
```

## Features

- **Folder selection** – Open dialog to pick a material folder (Tauri)
- **Analyze** – Runs pbr-core validation and shows issues in the right panel
- **3D preview** – Sphere with PBR material (placeholder; can be wired to loaded textures)

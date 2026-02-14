# PBR Studio v1.0.0

> **Ready to paste into GitHub Release.** Copy the content below (excluding this line) when creating the v1.0.0 release.

**Release date:** 2025-02-14

First stable production release of PBR Studio—a fully offline PBR texture set analyzer and exporter for game-engine workflows.

---

## Highlights

- **Validation engine** – 9 rules: required maps, resolution, power-of-two, albedo brightness, roughness uniformity, metallic mid-gray, normal strength, tileability
- **Scoring** – 0–100 material score with Critical/Major/Minor severities
- **Optimization** – Resize to 1K/2K/4K, RMA channel packing, LOD chain generation
- **Export presets** – 4K, Unreal Engine, Unity, Mobile + custom plugin presets
- **Advanced analysis** – Duplicate detection, cross-material consistency, tileability analysis
- **Fix tileability** – CLI and UI: apply edge blending for seamless tiling
- **EXR input support** – Load OpenEXR textures (HDR tone-mapped to 8-bit)
- **Plugin system** – Custom validation rules and export presets via JSON/TOML
- **AI heuristics** – Material classification, smart suggestions, anomaly detection (optional ONNX)
- **Audit logging** – Track validation, optimization, report actions locally
- **Desktop UI** – Blender-style layout: materials, 3D viewport, validation, console, audit log
- **Offline-first** – No backend, cloud, or database; all data stays local

---

## Installation

### Linux (AppImage)

1. Download `pbr-studio-ui_1.0.0_amd64.AppImage`
2. Make executable: `chmod +x pbr-studio-ui_1.0.0_amd64.AppImage`
3. Run: `./pbr-studio-ui_1.0.0_amd64.AppImage`

**Requirements:** glibc 2.35+, libfuse2, WebKitGTK 4.1. See [BUILD-APPIMAGE.md](BUILD-APPIMAGE.md).

### macOS (DMG)

1. Download `pbr-studio-ui_1.0.0_aarch64.dmg` (Apple Silicon) or `pbr-studio-ui_1.0.0_x64.dmg` (Intel)
2. Open DMG, drag PBR Studio to Applications
3. If Gatekeeper blocks: right-click → Open, then Open

**Note:** DMG is unsigned. See [BUILD-MACOS.md](BUILD-MACOS.md) for code signing.

### Windows (MSI)

1. Download `pbr-studio-ui_1.0.0_x64_en-US.msi`
2. Run installer
3. Launch from Start Menu

**Requirements:** WebView2 (included on Windows 11; may need install on older Windows 10).

### CLI only

```bash
# From source
git clone <repo>
cd pbr-studio
cargo build -p pbr-cli --release
# Binary: target/release/pbr-cli
```

---

## Known limitations

- **macOS DMG** – Unsigned; Gatekeeper may show a warning. Right-click → Open to run.
- **Linux AppImage** – Requires glibc 2.35+ (Ubuntu 22.04+). Older distros may need a rebuild.
- **PDF export** – Requires system fonts or bundled DejaVu Sans.
- **AI module** – Heuristic-only by default; ONNX model optional for ML classification.

---

## Checksums / signatures

*(Add after build; example format)*

```
# SHA-256 checksums (replace with actual values after build)
pbr-studio-ui_1.0.0_amd64.AppImage    <sha256>
pbr-studio-ui_1.0.0_aarch64.dmg       <sha256>
pbr-studio-ui_1.0.0_x64.dmg           <sha256>
pbr-studio-ui_1.0.0_x64_en-US.msi     <sha256>
```

To generate checksums:
```bash
sha256sum pbr-studio-ui_1.0.0_amd64.AppImage
shasum -a 256 pbr-studio-ui_1.0.0_aarch64.dmg
```

---

## Documentation

- [CLI Quickstart](CLI-QUICKSTART.md)
- [Desktop UI Guide](UI-GUIDE.md)
- [Plugin Development](plugins/README.md)
- [AI Heuristics Module](ai-module.md)
- [Release Checklist](RELEASE-CHECKLIST.md)

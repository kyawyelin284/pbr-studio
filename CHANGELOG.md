# Changelog

All notable changes to PBR Studio are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-02-14

### Major features

- **Validation engine** – 9 rules: required maps, resolution, power-of-two, albedo brightness, roughness uniformity, metallic mid-gray, normal strength, tileability
- **Scoring** – 0–100 material score with Critical/Major/Minor severities
- **Optimization** – Resize to 1K/2K/4K, RMA channel packing, LOD chain generation
- **Export presets** – 4K, Unreal Engine, Unity, Mobile + custom plugin presets
- **Batch operations** – `batch-check`, `batch-optimize`, `export-report` for multiple materials
- **Advanced analysis** – Duplicate detection, cross-material consistency, tileability analysis
- **Fix tileability** – CLI and UI: apply edge blending for seamless tiling
- **EXR input support** – Load OpenEXR textures (HDR tone-mapped to 8-bit for analysis)
- **Plugin system** – Custom validation rules and export presets via JSON/TOML
- **AI heuristics** – Material classification, smart suggestions, anomaly detection (optional ONNX)
- **Audit logging** – Track validation, optimization, report actions locally
- **Desktop UI** – Blender-style layout: materials, 3D viewport, validation, console, audit log
- **Offline-first** – No backend, cloud, or database; all data stays local

### Breaking changes

None. This is the first stable release.

### Known limitations

- **macOS DMG** – Unsigned; Gatekeeper may show a warning. Users can right-click → Open, or use Developer ID signing (see [BUILD-MACOS.md](docs/BUILD-MACOS.md)).
- **Linux AppImage** – Built on Ubuntu 22.04; requires glibc 2.35+. Older distros (e.g. Ubuntu 20.04) may need a rebuild on an older base.
- **PDF export** – Requires system fonts (LiberationSans, DejaVu, Arial) or bundled DejaVu Sans. Works on Linux, Windows, macOS.
- **AI module** – Heuristic-only by default; ONNX model optional for ML-based classification.

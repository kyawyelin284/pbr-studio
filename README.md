# PBR Studio

A fully offline, open-source PBR texture set analyzer and exporter. Optimized for game-engine workflows (Unreal Engine, Unity, mobile) with validation, scoring, and texture optimization.

## Features

- **Validation**: Required maps, resolution, power-of-two, albedo brightness, roughness uniformity, metallic mid-gray, normal strength, tileability
- **Scoring**: 0–100 material score with Critical/Major/Minor severities
- **Optimization**: Resize to 1K/2K/4K, channel packing (R=AO, G=Roughness, B=Metallic)
- **Export presets**: 4K, Unreal Engine, Unity, Mobile
- **Offline**: No backend, cloud, or database—runs entirely locally

## Architecture

| Crate        | Purpose                                                                 |
|-------------|-------------------------------------------------------------------------|
| **pbr-core**   | Image I/O (PNG, JPG, TGA; EXR planned), material analysis, validation rules, report, optimization |
| **pbr-cli**    | Command-line tools for CI and batch processing                         |
| **pbr-studio-ui** | Tauri desktop app with Blender-style layout                         |

## CLI (pbr-cli)

```bash
cargo build -p pbr-cli --release
# Binary: target/release/pbr-cli
```

### Commands

| Command       | Description                                              |
|---------------|----------------------------------------------------------|
| `check <folder>` | Validate a material folder; exit 1 if score < min        |
| `batch-check <root>` | Recursively scan for material folders and summarize     |
| `pre-commit` | Validate materials with staged files (for Git hooks)    |
| `analyze <root>` | Advanced analysis (duplicates, cross-material, tileability) |
| `fix-tileability <path> --output <path>` | Apply tileability fix to texture |
| `optimize <folder> --output <path> --target <preset>` | Export optimized textures; `--lod` for LOD chain |
| `batch-optimize <root> --output <path> --target <preset>` | Batch export all materials under root |
| `report <folder> --json` | Generate text or JSON report; `--vram` for VRAM estimate |
| `export-report <folders> --format html|pdf --output <path>` | Export HTML or PDF reports for one or more material folders |
| `audit-log [--limit N] [--json]` | Show audit log (validation, optimization, report actions) |

The CLI exits with code 1 if any material score is below the configured `--min-score` threshold.

### Examples

```bash
# Validate material folder (min score 60)
pbr-cli check ./Materials/Wood --min-score 60

# Batch check all materials under a root
pbr-cli batch-check ./Assets/Materials --min-score 60

# Optimize for Unreal Engine (2K, packed RMA)
pbr-cli optimize ./Materials/Brick --output ./Optimized --target unreal

# Optimize for mobile (1K)
pbr-cli optimize ./Materials/Metal --output ./Mobile --target mobile

# Optimize for 4K
pbr-cli optimize ./Materials/Hero --output ./Hero4K --target 4k

# Export with LOD chain (LOD0, LOD1, LOD2 subdirs)
pbr-cli optimize ./Materials/Wood --output ./WoodLOD --target unreal --lod

# Batch export all materials under root
pbr-cli batch-optimize ./Materials --output ./Optimized --target unreal

# Report with VRAM estimate
pbr-cli report ./Materials/Wood --vram

# JSON report
pbr-cli report ./Materials/Wood --json

# CI/CD: structured JSON for pipelines
pbr-cli check ./Materials/Wood --ci --min-score 60
pbr-cli batch-check ./Assets/Materials --ci --min-score 60

# Pre-commit: validate only staged material folders
pbr-cli pre-commit --min-score 60

# Advanced analysis (duplicates, cross-material trends, tileability)
pbr-cli analyze ./Materials --tileability

# Fix tileability (blend edges for seamless tiling)
pbr-cli fix-tileability ./Materials/Wood --output ./Fixed
```

### Advanced analysis

The `analyze` command outputs structured JSON with:

- **Duplicate detection**: Finds identical or highly similar textures across materials (perceptual hash).
- **Cross-material consistency**: Resolution distribution, map coverage, recommendations.
- **Tileability**: Optional report on textures that would benefit from edge blending.

The `fix-tileability` command applies edge blending to make textures tile seamlessly (top↔bottom, left↔right).

### VRAM estimation

Reports include VRAM estimates (RGBA8, with mipmaps). Use `--vram` for text output:

```bash
pbr-cli report ./Materials/Wood --vram
```

JSON reports always include `vram_estimate` (bytes, formatted, per-texture breakdown).

### LOD generation

Export with `--lod` to generate LOD0 (full), LOD1 (512), LOD2 (256), LOD3 (128) subdirs for streaming.

### Pre-commit hook

Install the hook to validate materials before each commit:

```bash
cp scripts/pre-commit-pbr .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

The hook runs `pbr-cli pre-commit`, which validates only material folders that have staged texture files. Use `--min-score` to adjust the threshold.

### CI/CD integration

Use `--ci` to output structured JSON for automated pipelines:

```json
{
  "success": false,
  "min_score": 60,
  "total_materials": 3,
  "passed": 2,
  "failed": 1,
  "results": [
    {
      "path": "Materials/Wood",
      "score": 75,
      "passed": true,
      "critical_count": 0,
      "major_count": 1,
      "minor_count": 0,
      "issues": [...]
    }
  ]
}
```

### Targets

- `4k` – 4K resolution, packed RMA texture
- `unreal` – 2K, packed RMA
- `unity` – 2K, packed RMA
- `mobile` – 1K, packed RMA

## Desktop App (pbr-studio-ui)

Blender-style layout with:

- Left panel: asset list, drag-and-drop
- Center: 3D viewport
- Right panel: validation and reports
- Bottom: console / log panel

### Development

```bash
cd pbr-studio-ui
npm install
npm run tauri:dev
```

### Build

```bash
cd pbr-studio-ui
npm run tauri:build
```

See [docs/](docs/) for platform-specific builds:

- [Linux AppImage](docs/BUILD-APPIMAGE.md)
- [macOS DMG](docs/BUILD-MACOS.md)
- [Windows MSI](docs/BUILD-WINDOWS.md)

## Release builds (GitHub Actions)

Push a version tag to trigger cross-platform builds:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds:

- Linux: AppImage (x64)
- Windows: MSI (x64)
- macOS: DMG (x64 and Apple Silicon)

Artifacts are uploaded to a draft GitHub release. See [.github/workflows/ci-release.yml](.github/workflows/ci-release.yml).

## Supported formats

- **Input**: PNG, JPG, TGA (EXR support planned)
- **Output**: PNG, JPG, TGA
- **Channel packing**: R=AO, G=Roughness, B=Metallic (ORM/RMA texture)

## License

See the repository for license information.
# pbr-studio

# PBR Studio CLI Usage Guide

Complete reference for `pbr-cli` commands with examples. All operations run **offline**—no network calls.

## Quick Start

```bash
# Build the CLI
cargo build -p pbr-cli --release

# Validate a material folder
pbr-cli check ./Materials/Wood --min-score 60

# Generate a report
pbr-cli report ./Materials/Wood --json
```

## Command Reference

| Command | Description |
|---------|-------------|
| `check` | Validate a single material folder |
| `batch-check` | Recursively scan and validate all materials under root |
| `pre-commit` | Validate only materials with staged files (Git hooks) |
| `optimize` | Export optimized textures for target engine |
| `batch-optimize` | Batch export all materials under root |
| `report` | Generate text or JSON report |
| `export-report` | Export HTML, PDF, or batch JSON reports |
| `analyze` | Advanced analysis (duplicates, cross-material, tileability) |
| `fix-tileability` | Apply edge blending for seamless tiling |
| `audit-log` | Show validation/optimization/report history |
| `plugin-list` | List loaded plugins (rules and presets) |
| `ai-analyze` | AI-assisted classification and suggestions |

---

## Validation

### Single material

```bash
# Basic validation (min score 60)
pbr-cli check ./Materials/Wood

# Stricter threshold
pbr-cli check ./Materials/Wood --min-score 80

# With custom plugin rules
pbr-cli check ./Materials/Wood --plugins

# Custom plugin directory
pbr-cli check ./Materials/Wood --plugins-dir ./studio-plugins

# CI/CD: structured JSON, exit 1 on fail
pbr-cli check ./Materials/Wood --ci --min-score 60
```

### Batch validation

```bash
# Scan all materials under root
pbr-cli batch-check ./Assets/Materials --min-score 60

# Write aggregated JSON report to file
pbr-cli batch-check ./Assets/Materials --output batch-report.json --ci

# With plugins
pbr-cli batch-check ./Assets/Materials --plugins --min-score 70
```

### Pre-commit hook

```bash
# Validate only staged material folders
pbr-cli pre-commit --min-score 60

# From specific repo root
pbr-cli pre-commit --root /path/to/repo --min-score 70
```

---

## Optimization & Export

### Optimization presets

| Target | Resolution | Use case |
|--------|------------|----------|
| `4k` | 4096 | Hero/cinematic assets |
| `unreal` | 2048 | Unreal Engine (packed RMA) |
| `unity` | 2048 | Unity (packed RMA) |
| `mobile` | 1024 | Mobile-optimized |

### Single material export

```bash
# Unreal Engine preset (default)
pbr-cli optimize ./Materials/Brick --output ./Optimized --target unreal

# 4K for hero assets
pbr-cli optimize ./Materials/Hero --output ./Hero4K --target 4k

# Mobile (1K)
pbr-cli optimize ./Materials/Prop --output ./Mobile --target mobile

# With LOD chain (LOD0, LOD1, LOD2 subdirs)
pbr-cli optimize ./Materials/Wood --output ./WoodLOD --target unreal --lod
```

### Batch export

```bash
# Export all materials under root
pbr-cli batch-optimize ./Materials --output ./Optimized --target unreal

# With LOD for each material
pbr-cli batch-optimize ./Materials --output ./Optimized --target unreal --lod
```

---

## Reports

### Text/JSON report

```bash
# Text report to stdout
pbr-cli report ./Materials/Wood

# JSON report
pbr-cli report ./Materials/Wood --json

# With VRAM estimate
pbr-cli report ./Materials/Wood --vram

# Export to file (json, html, or pdf)
pbr-cli report ./Materials/Wood --export html --output report.html
pbr-cli report ./Materials/Wood --export pdf --output report.pdf
```

### Batch report export

Output paths are local-only (no network). Formats: `html`, `pdf`, or `json`.

```bash
# HTML report for multiple materials
pbr-cli export-report ./Mat1 ./Mat2 ./Mat3 --format html --output batch-report.html

# PDF report
pbr-cli export-report ./Materials/* --format pdf --output report.pdf

# Batch JSON (matches report <folder> --json schema; array of { path, report })
pbr-cli export-report ./Mat1 ./Mat2 --format json --output batch-report.json

# Track versions in .pbr-studio/versions.json
pbr-cli export-report ./Materials --format html --output report.html --track
```

---

## Batch Analysis

Advanced analysis for duplicate detection, cross-material consistency, and tileability:

```bash
# Full analysis (duplicates + cross-material)
pbr-cli analyze ./Materials

# Include tileability report
pbr-cli analyze ./Materials --tileability

# Custom similarity thresholds
pbr-cli analyze ./Materials --duplicate-threshold 0.99 --similar-threshold 0.80

# Write JSON report to file
pbr-cli analyze ./Materials --tileability --output analysis.json
```

### Tileability fix

```bash
# Apply edge blending for seamless tiling
pbr-cli fix-tileability ./Materials/Wood --output ./Fixed

# Blend width (default 4 pixels)
pbr-cli fix-tileability ./Materials/Wood --output ./Fixed --blend-width 8
```

---

## Plugin System

### List plugins

```bash
# Human-readable
pbr-cli plugin-list

# JSON output
pbr-cli plugin-list --json
```

### Use plugins in validation

```bash
# Load from default paths (./.pbr-studio/plugins, ~/.config/pbr-studio/plugins)
pbr-cli check ./material --plugins

# Custom directory
pbr-cli check ./material --plugins-dir ./my-plugins

# Config file
pbr-cli check ./material --config .pbr-studio/config.toml
```

See [plugins/README.md](plugins/README.md) for plugin format and rule types.

---

## AI Analysis

```bash
# Heuristic classification (default build)
pbr-cli ai-analyze ./Materials/Wood

# With ONNX model (requires cargo build --features ai)
pbr-cli ai-analyze ./Materials/Wood --model ./models/material_classifier.onnx
```

Output includes: material classification, smart optimization suggestions, anomaly detection. See [ai-module.md](ai-module.md).

---

## Audit Log

```bash
# Last 50 entries (text)
pbr-cli audit-log

# JSON output
pbr-cli audit-log --json

# More entries
pbr-cli audit-log --limit 200

# Write to file
pbr-cli audit-log --output audit.txt --format text
pbr-cli audit-log -o audit.json --format json
```

---

## Global Options

| Option | Description |
|--------|-------------|
| `--plugins-dir <path>` | Add plugin directory |
| `--config <path>` | Config file (TOML, can set `plugins_dir`) |

---

## CI/CD Integration

```bash
# Exit 1 if any material fails
pbr-cli check ./Materials --ci --min-score 60
pbr-cli batch-check ./Assets --ci --min-score 60 --output results.json
```

CI output format:

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

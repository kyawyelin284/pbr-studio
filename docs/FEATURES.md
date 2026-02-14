# PBR Studio Features

Overview of batch analysis, plugin system, AI modules, and optimization presets.

---

## Batch Analysis

Recursively analyze multiple material folders for duplicates, consistency, and tileability.

### Commands

```bash
# Batch validation
pbr-cli batch-check ./Materials --min-score 60 --output report.json

# Advanced analysis (duplicates, cross-material, tileability)
pbr-cli analyze ./Materials --tileability --output analysis.json

# Batch export
pbr-cli batch-optimize ./Materials --output ./Optimized --target unreal
```

### Duplicate detection

- Perceptual hash comparison across materials
- `--duplicate-threshold 0.99` – near-identical (default)
- `--similar-threshold 0.80` – similar textures

### Cross-material consistency

- Resolution distribution
- Map coverage (albedo, normal, etc.)
- Recommendations for standardization

### Tileability

- Identifies textures that would benefit from edge blending
- `fix-tileability` applies blend for seamless tiling

---

## Plugin System

Custom validation rules and export presets without code changes. See [plugins/README.md](plugins/README.md).

### Discovery paths

1. `./.pbr-studio/plugins/` (project-local)
2. `~/.config/pbr-studio/plugins/` (user)
3. `PBR_STUDIO_PLUGINS` env var (colon-separated)

### Rule types

| Type | Description |
|------|-------------|
| `required_maps` | Require albedo, normal, height, etc. |
| `max_resolution` | Fail if dimensions exceed limit |
| `min_resolution` | Fail if dimensions below limit |
| `power_of_two` | Require power-of-two dimensions |
| `max_texture_count` | Limit texture count |
| `script` | External script (Python, Lua) via stdin/stdout |

### Custom presets

Define export presets with `target_resolution` (4k, 2k, 1k, etc.) and `include_lod`.

---

## AI Modules

Optional offline AI for classification, suggestions, and anomaly detection. See [ai-module.md](ai-module.md).

### Material classification

- **Heuristic** (default): Color variance, edge density, saturation, warm ratio
- **ONNX** (optional): Build with `--features ai`, pass model path
- Classes: metal, wood, skin, fabric, stone, plastic, unknown

### Smart optimization suggestions

- Low complexity → suggest downscale to 2K/1K
- High res + moderate complexity → suggest 4K
- Flat albedo → note limited height map impact

### Anomaly detection

- Dimension mismatch between textures
- Unusually flat normal maps

---

## Optimization Presets

| Preset | Resolution | Packed RMA | Use case |
|--------|------------|------------|----------|
| **4K** | 4096 | Yes | Hero/cinematic |
| **Unreal** | 2048 | Yes | Unreal Engine |
| **Unity** | 2048 | Yes | Unity |
| **Mobile** | 1024 | Yes | Mobile |

### LOD chain

`--lod` generates LOD0 (full), LOD1 (512), LOD2 (256), LOD3 (128) subdirs for streaming.

### Channel packing

R=AO, G=Roughness, B=Metallic (ORM/RMA texture) for reduced draw calls.

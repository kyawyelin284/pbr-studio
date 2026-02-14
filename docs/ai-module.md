# AI-Assisted Analysis Module

Optional offline AI analysis for PBR materials. All processing runs locally with no network calls.

## Features

### 1. Material Classification

Classifies albedo texture into categories: **metal**, **wood**, **skin**, **fabric**, **stone**, **plastic**, or **unknown**.

- **Heuristic mode** (default): Uses texture statistics (color variance, edge density, saturation, warm/cool ratio)
- **ONNX mode** (optional): Build with `--features ai` and pass `--model path` for ML-based classification

### 2. Smart Optimization Suggestions

Analyzes texture complexity to suggest safe downscaling:

- Low variance + low edge density → "Consider downscaling to 2K/1K without noticeable quality loss"
- High resolution + moderate complexity → "4K or 2K may suffice"
- Flat albedo → "Height map may have limited impact"

Suggestions are merged into the main optimization list and appear in the report.

### 3. Anomaly Detection

Flags unusual or inconsistent textures within a material set:

- **Dimension mismatch**: Textures with significantly different resolutions (e.g. 2x difference)
- **Flat normal map**: Normal map appears unusually uniform vs albedo

Anomalies appear as minor issues in the validation report.

## Integration

### JSON Report

```json
{
  "ai_insights": {
    "classification": "wood",
    "classification_confidence": 0.6,
    "smart_suggestions": [
      {
        "category": "resolution",
        "message": "Low visual complexity...",
        "confidence": 0.75,
        "target_resolution": "2K"
      }
    ],
    "anomalies": [
      {
        "slot": "normal",
        "message": "Normal map appears unusually flat",
        "score": 0.6
      }
    ]
  }
}
```

### HTML/PDF Export

AI insights are included in report exports (classification, anomalies).

### UI

The Validation Panel shows:
- **AI Insights** section with classification and confidence
- **Anomalies detected** list when applicable
- Smart suggestions appear in the main "Suggested Optimizations" list

## ONNX Support (Optional)

Build with `--features ai` to enable ONNX-based classification:

```bash
cargo build -p pbr-core --features ai
cargo build -p pbr-cli --features ai  # if using CLI
```

### Model Format

- **Input**: `[1, 3, 224, 224]` NCHW, ImageNet normalization `(pixel/255 - mean) / std`
- **Output**: `[1, N]` logits. Indices 0–5 map to: metal, wood, skin, fabric, stone, plastic; index 6+ = unknown

### CLI

```bash
# Heuristic-only (default build)
pbr-cli ai-analyze ./material

# With ONNX model (requires --features ai)
pbr-cli ai-analyze ./material --model ./models/material_classifier.onnx
```

### Tauri / UI

```typescript
const json = await invoke('ai_analyze', { path: materialPath, modelPath: optionalModelPath });
```

### Report Integration

Use `MaterialReport::from_material_set_with_ai(set, issues, onnx_path)` to include ML classification in reports.

## Offline Guarantee

- No network requests
- All heuristics use only pixel data from loaded textures
- ONNX models loaded from local path only

# PBR Studio UI Guide

Desktop application with Blender-style layout. All operations run **offline**—no cloud or network.

## Desktop app only

PBR Studio requires the **Tauri desktop build** for filesystem access (material folders, export, validation). If you open the app in a web browser (e.g. via `npm run dev` without Tauri), you will see:

> **PBR Studio is a desktop application. Please download the desktop app for Linux, Windows, or macOS.**

In that case, all UI actions that need filesystem access (add materials, export, report generation) are hidden. Use `npm run tauri:dev` or build the desktop app to access the full interface.

## Layout

| Panel | Purpose |
|-------|---------|
| **Left** | Material list, folder selection, batch actions, export presets |
| **Center** | 3D viewport with material preview |
| **Right** | Validation results, score, issues, optimization suggestions |
| **Bottom** | Console (logs) and Audit Log tabs |

## Getting Started

### Add materials

1. **Drag and drop** – Drop material folders or parent folders onto the left panel
2. **Add folders** – Click **Add folders** to select one or more material directories (multi-select supported)
3. Materials are auto-detected by texture slots (albedo, normal, roughness, etc.)

### Analyze

- Materials are analyzed automatically when added
- **Refresh** (↻) – Re-analyze selected material after editing textures
- **Live scoring** – Selected material re-analyzes when texture files change (2s poll)

## Validation Panel

- **Score** – 0–100; green (pass), yellow (warning), red (fail)
- **Issues** – Critical, major, minor with rule IDs and messages
- **Optimization suggestions** – Resolution, format, workflow tips
- **AI insights** – Material classification, smart suggestions, anomalies (when available)
- **VRAM estimate** – Memory usage for textures

## Export

### Single material

1. Select a material
2. Choose preset: **4K**, **Unreal Engine**, **Unity**, **Mobile**, or custom from plugins
3. Optionally enable **Include LOD**
4. Click export, choose output folder

### Batch export

1. Add multiple materials (Add folders or drag-drop)
2. Use **Batch 4K**, **Batch Unreal Engine**, etc. to export all (or selected) materials
3. Choose output folder; each material exports to a subfolder
4. **Export report (all)** – HTML/PDF report for all materials from the validation panel

### Report export

1. Select materials (or use all)
2. Use **Export Report** from validation panel
3. Choose **HTML** or **PDF**
4. Pick output path

## Compare Mode

- Select two materials and click **Compare**
- Side-by-side validation and 3D preview
- Useful for A/B testing or consistency checks

## Settings

Open via gear icon (top-right):

- **Theme** – Dark / Light
- **Layout** – Compact / Normal / Spacious
- **Undo history** – 10–100 steps
- **Validation colors** – Customize critical/warning/pass colors
- **Plugins directory** – Custom path for validation rules and export presets

## Audit Log

- **Bottom tab** – Switch between Console and Audit Log
- **Refresh** (↻) – Reload audit entries
- **Export** – Save audit log as JSON or text file
- Tracks: validation, optimization, report actions with timestamps and scores

## Keyboard & Actions

- **Undo / Redo** – Revert material list changes
- **Remove** – Remove selected material from list
- **Clear selection** – Deselect all

## Plugins

1. **Settings → Plugins directory** – Set path (e.g. `./.pbr-studio/plugins`)
2. Custom validation rules apply to all analyses
3. Custom export presets appear in the preset dropdown
4. Leave empty to use defaults: `./.pbr-studio/plugins`, `~/.config/pbr-studio/plugins`

## Offline Guarantee

- No network requests
- All data stored locally (preferences in localStorage, audit in `~/.config/pbr-studio/audit.json`)
- Textures loaded from disk only
- AI analysis uses heuristics or local ONNX model (no cloud APIs)

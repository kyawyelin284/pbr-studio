# PBR Studio Plugin System

Plugins allow studios to define custom validation rules and export presets without modifying core code.

## Plugin Discovery

Plugins are loaded from (in order):

1. `./.pbr-studio/plugins/` (project-local)
2. `~/.config/pbr-studio/plugins/` (user config)
3. `PBR_STUDIO_PLUGINS` env var (colon-separated paths)

Each plugin is a directory containing `plugin.json` or `plugin.toml`.

## Manifest Format

### JSON (plugin.json)

```json
{
  "name": "my-studio",
  "version": "1.0.0",
  "rules": [
    {
      "id": "max_8k",
      "description": "Max resolution 8K",
      "severity": "major",
      "condition": {
        "type": "max_resolution",
        "max_width": 8192,
        "max_height": 8192
      }
    }
  ],
  "presets": [
    {
      "id": "cinematic",
      "name": "Cinematic 4K",
      "target_resolution": "4k",
      "include_lod": true
    }
  ]
}
```

### TOML (plugin.toml)

```toml
name = "my-studio"
version = "1.0.0"

[[rules]]
id = "max_8k"
description = "Max resolution 8K"
severity = "major"
[rules.condition]
type = "max_resolution"
max_width = 8192
max_height = 8192

[[presets]]
id = "cinematic"
name = "Cinematic 4K"
target_resolution = "4k"
include_lod = true
```

## Rule Condition Types

| Type | Parameters | Description |
|------|------------|-------------|
| `required_maps` | `maps: [string]` | Require specific texture slots (albedo, normal, roughness, metallic, ao, height) |
| `max_resolution` | `max_width`, `max_height` | Fail if any dimension exceeds |
| `min_resolution` | `min_width`, `min_height` | Fail if any dimension below |
| `power_of_two` | - | All textures must be power-of-two |
| `max_texture_count` | `max: int` | Limit texture count |
| `script` | `command`, `args` | External script (Python, Lua, etc.) |

## Scripting API

For `type: "script"` rules, the executable receives JSON on stdin and must return JSON on stdout.

### Input (stdin)

```json
{
  "path": null,
  "name": "MyMaterial",
  "texture_count": 5,
  "dimensions": { "width": 2048, "height": 2048 },
  "maps": {
    "albedo": true,
    "normal": true,
    "roughness": true,
    "metallic": true,
    "ao": true,
    "height": false
  },
  "dimensions_consistent": true
}
```

### Output (stdout)

```json
{
  "issues": [
    {
      "rule_id": "custom_script",
      "severity": "major",
      "message": "Custom validation failed: ..."
    }
  ]
}
```

### Example Python Script

```python
#!/usr/bin/env python3
import json, sys
data = json.load(sys.stdin)
issues = []
if data.get("texture_count", 0) > 6:
    issues.append({
        "rule_id": "custom_script",
        "severity": "major",
        "message": "Too many textures"
    })
print(json.dumps({"issues": issues}))
```

## CLI Usage

```bash
# Use default plugin paths
pbr-cli check ./material --plugins

# Custom plugin directory
pbr-cli check ./material --plugins-dir ./my-plugins

# Config file (can specify plugins-dir)
pbr-cli check ./material --config .pbr-studio/config.toml

# List loaded plugins (rules and presets)
pbr-cli plugin-list
pbr-cli plugin-list --json
```

## UI Usage

- **Settings → Plugins directory**: Set a custom path for validation rules and export presets. Leave empty to use defaults.
- Validation and export use your custom plugins when a directory is configured.

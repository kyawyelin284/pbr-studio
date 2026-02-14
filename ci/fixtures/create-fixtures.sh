#!/bin/sh
# Creates minimal PBR material fixtures for CI (batch-check, report generation).
# Uses base64-encoded 1x1 PNG (smallest valid PNG ~71 bytes).

set -e
ROOT="${1:-.}"
MATERIAL_DIR="$ROOT/sample-material"
mkdir -p "$MATERIAL_DIR"

# Minimal 1x1 red PNG (base64) - use Python for portability
MATERIAL_DIR="$MATERIAL_DIR" python3 -c "
import base64, os
d = os.environ['MATERIAL_DIR']
os.makedirs(d, exist_ok=True)
png = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQzwAEAgMBwY3wBwQAAAABJRU5ErkJggg==')
for n in ['albedo.png', 'normal.png', 'roughness.png', 'metallic.png']:
    open(os.path.join(d, n), 'wb').write(png)
print('Created fixtures in', d)
"

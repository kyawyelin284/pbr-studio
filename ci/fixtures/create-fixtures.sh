#!/bin/sh
# Creates minimal PBR material fixtures for CI (batch-check, report generation).
# Uses Python Pillow to generate valid PNGs (avoids base64 CRC issues).

set -e
ROOT="${1:-.}"
MATERIAL_DIR="$ROOT/sample-material"
mkdir -p "$MATERIAL_DIR"

MATERIAL_DIR="$MATERIAL_DIR" python3 -c "
import os
try:
    from PIL import Image
except ImportError:
    raise SystemExit('Pillow required: pip install Pillow or apt install python3-pillow')

d = os.environ['MATERIAL_DIR']
os.makedirs(d, exist_ok=True)
# 4x4 RGBA (minimal valid size for PBR tooling)
img = Image.new('RGBA', (4, 4), (128, 128, 128, 255))
for n in ['albedo.png', 'normal.png', 'roughness.png', 'metallic.png']:
    img.save(os.path.join(d, n))
print('Created fixtures in', d)
"

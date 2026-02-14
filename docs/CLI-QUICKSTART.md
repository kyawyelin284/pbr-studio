# CLI Quickstart

Common workflows for `pbr-cli`. For full reference, see [CLI-USAGE.md](CLI-USAGE.md).

## Build

```bash
cargo build -p pbr-cli --release
# Binary: target/release/pbr-cli
```

## Validate

```bash
# Single material
pbr-cli check ./Materials/Wood --min-score 60

# All materials under root
pbr-cli batch-check ./Assets/Materials --min-score 60

# CI mode (JSON output, exit 1 on fail)
pbr-cli batch-check ./Materials --ci --min-score 60
```

## Optimize & export

```bash
# Export for Unreal Engine (2K, packed RMA)
pbr-cli optimize ./Materials/Brick --output ./Optimized --target unreal

# Batch export all materials
pbr-cli batch-optimize ./Materials --output ./Optimized --target unreal

# With LOD chain
pbr-cli optimize ./Materials/Wood --output ./WoodLOD --target unreal --lod
```

## Reports

```bash
# JSON report
pbr-cli report ./Materials/Wood --json

# Batch HTML/PDF/JSON
pbr-cli export-report ./Mat1 ./Mat2 --format html --output report.html
pbr-cli export-report ./Mat1 ./Mat2 --format json --output report.json
```

## Pre-commit hook

```bash
cp scripts/pre-commit-pbr .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

Runs `pbr-cli pre-commit` on staged material folders. Requires a git repository.

## Advanced analysis

```bash
# Duplicates, cross-material, tileability
pbr-cli analyze ./Materials

# Fix non-tileable texture
pbr-cli fix-tileability ./Materials/Wood --output ./Fixed
```

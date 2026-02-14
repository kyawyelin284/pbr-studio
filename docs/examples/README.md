# Report Examples

Sample outputs from PBR Studio report generation.

## Files

| File | Description |
|------|-------------|
| `sample-report.json` | JSON report structure (from `pbr-cli report --json`) |
| `sample-report.html` | HTML report layout (from `pbr-cli report --export html` or `export-report`) |

## Generating reports

```bash
# JSON report
pbr-cli report ./Materials/Wood --json

# HTML report
pbr-cli report ./Materials/Wood --export html --output report.html

# PDF report (requires build with --features pdf)
pbr-cli report ./Materials/Wood --export pdf --output report.pdf

# Batch HTML/PDF
pbr-cli export-report ./Mat1 ./Mat2 --format html --output batch-report.html
pbr-cli export-report ./Materials --format pdf --output report.pdf
```

## Report structure

- **JSON**: `name`, `score`, `summary`, `issues`, `optimization_suggestions`, `vram_estimate`, `ai_insights`
- **HTML**: Styled layout with sections for each
- **PDF**: Same content as HTML, rendered as PDF

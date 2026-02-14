# pbr-core

Engine for analyzing PBR (Physically Based Rendering) texture sets. Designed for use by CLI tools and desktop applications.

## Architecture

The library is organized into four modules:

| Module | Purpose |
|--------|---------|
| `image_loading` | Load images, detect texture slots from filenames, extract metadata |
| `material` | Represent texture sets, analyze completeness and consistency |
| `validation` | Pluggable validation rules (albedo required, dimension consistency, etc.) |
| `report` | Generate human-readable and JSON reports from analysis results |

## Usage

```rust
use pbr_core::{ImageLoader, MaterialAnalyzer, Report, Validator};

// Build a texture set (e.g., from a directory scan)
let mut set = pbr_core::TextureSet::new();
// ... add textures ...

// Analyze and validate
let analysis = MaterialAnalyzer::analyze(&set);
let results = Validator::default().validate(&set);

// Generate report
let report = Report::from_texture_set(&set, results, Some("MyMaterial".into()));
println!("{}", report.to_text());
```

## Adding Custom Validation Rules

Implement the `ValidationRule` trait:

```rust
use pbr_core::validation::{ValidationRule, ValidationResult, Severity};

struct MyCustomRule;

impl ValidationRule for MyCustomRule {
    fn id(&self) -> &str { "my_rule" }
    fn description(&self) -> &str { "Custom validation" }
    fn validate(&self, set: &pbr_core::TextureSet) -> ValidationResult {
        ValidationResult {
            rule_id: self.id().to_string(),
            severity: Severity::Warning,
            message: "...".into(),
            passed: true,
        }
    }
}

let validator = Validator::new().with_rule(MyCustomRule);
```

## License

MIT or Apache-2.0

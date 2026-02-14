//! Validation rules and checks for PBR texture sets.
//!
//! Defines pluggable validation rules that can be composed
//! for different validation strategies.

use crate::material::{MaterialSet, TextureMap};
use serde::{Deserialize, Serialize};

/// Severity of a validation finding.
/// Maps to scoring: Critical -20, Major -10, Minor -5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Major,
    Minor,
}

impl Severity {
    pub fn score_penalty(&self) -> i32 {
        match self {
            Severity::Critical => 20,
            Severity::Major => 10,
            Severity::Minor => 5,
        }
    }
}

/// Compute material score from issues. Start at 100, subtract penalties.
pub fn compute_score(issues: &[Issue]) -> i32 {
    let total: i32 = issues.iter().map(|i| i.severity.score_penalty()).sum();
    (100 - total).max(0)
}

/// Legacy type for Report compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub passed: bool,
}

impl From<Issue> for ValidationResult {
    fn from(issue: Issue) -> Self {
        ValidationResult {
            rule_id: issue.rule_id,
            severity: issue.severity,
            message: issue.message,
            passed: false,
        }
    }
}

/// A validation issue found by a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
}

impl Issue {
    pub fn new(rule_id: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
        }
    }
}

/// Pluggable validation rule
pub trait ValidationRule: Send + Sync {
    /// Unique identifier for this rule
    fn id(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Check the material set. Returns `Some(Issue)` if a problem is found, `None` if valid.
    fn check(&self, set: &MaterialSet) -> Option<Issue>;

    /// Check and return all issues (default: 0 or 1 from `check`).
    /// Override for rules that can emit multiple issues (e.g. script plugins).
    fn check_all(&self, set: &MaterialSet) -> Vec<Issue> {
        self.check(set).into_iter().collect()
    }
}

/// Runs validation rules against material sets
pub struct Validator {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn with_rule<R: ValidationRule + 'static>(mut self, rule: R) -> Self {
        self.rules.push(Box::new(rule));
        self
    }

    pub fn add_rule<R: ValidationRule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    /// Build validator with default rules + plugin rules from loader.
    pub fn with_plugins(mut self, loader: &crate::plugin::PluginLoader) -> Self {
        let (plugin_rules, _presets) = loader.load();
        for r in plugin_rules {
            self.rules.push(Box::new(r));
        }
        self
    }

    pub fn check(&self, set: &MaterialSet) -> Vec<Issue> {
        self.rules
            .iter()
            .flat_map(|r| r.check_all(set))
            .collect()
    }

    pub fn has_issues(&self, set: &MaterialSet) -> bool {
        !self.check(set).is_empty()
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
            .with_rule(RequiredMapsRule)
            .with_rule(ResolutionMismatchRule)
            .with_rule(NonPowerOfTwoRule)
            .with_rule(TextureResolutionRule)
            .with_rule(AlbedoBrightnessRule)
            .with_rule(RoughnessUniformityRule)
            .with_rule(MetallicMidGrayRule)
            .with_rule(NormalMapStrengthRule)
            .with_rule(TileabilityRule)
    }
}

/// Rule: Albedo + normal required minimum
pub struct RequiredMapsRule;

impl ValidationRule for RequiredMapsRule {
    fn id(&self) -> &str {
        "required_maps"
    }

    fn description(&self) -> &str {
        "Albedo and normal maps are required minimum for PBR"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        if set.albedo.is_none() {
            return Some(Issue::new(
                self.id(),
                Severity::Critical,
                "Missing albedo/base color map. Required for PBR.",
            ));
        }
        if set.normal.is_none() {
            return Some(Issue::new(
                self.id(),
                Severity::Critical,
                "Missing normal map. Required for PBR.",
            ));
        }
        None
    }
}

/// Rule: All textures must have same resolution
pub struct ResolutionMismatchRule;

impl ValidationRule for ResolutionMismatchRule {
    fn id(&self) -> &str {
        "resolution_mismatch"
    }

    fn description(&self) -> &str {
        "Texture resolution mismatch across maps"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        if !set.dimensions_consistent() {
            return Some(Issue::new(
                self.id(),
                Severity::Major,
                "Texture resolution mismatch. All maps should have the same dimensions.",
            ));
        }
        None
    }
}

fn is_power_of_two(n: u32) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// Rule: Non-power-of-two dimensions
pub struct NonPowerOfTwoRule;

impl ValidationRule for NonPowerOfTwoRule {
    fn id(&self) -> &str {
        "non_power_of_two"
    }

    fn description(&self) -> &str {
        "Texture dimensions should be power of two (e.g. 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048)"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let maps = [
            ("albedo", set.albedo.as_ref()),
            ("normal", set.normal.as_ref()),
            ("roughness", set.roughness.as_ref()),
            ("metallic", set.metallic.as_ref()),
            ("ao", set.ao.as_ref()),
            ("height", set.height.as_ref()),
        ];

        let bad: Vec<_> = maps
            .into_iter()
            .filter_map(|(name, map)| {
                let m = map?;
                if !is_power_of_two(m.width) || !is_power_of_two(m.height) {
                    Some((name, m.width, m.height))
                } else {
                    None
                }
            })
            .collect();

        if bad.is_empty() {
            return None;
        }

        let list = bad
            .iter()
            .map(|(n, w, h)| format!("{} ({}x{})", n, w, h))
            .collect::<Vec<_>>()
            .join(", ");

        Some(Issue::new(
            self.id(),
            Severity::Minor,
            format!("Non-power-of-two dimensions: {}. May cause GPU issues.", list),
        ))
    }
}

/// Rule: Albedo brightness and clipped colors
pub struct AlbedoBrightnessRule;

impl ValidationRule for AlbedoBrightnessRule {
    fn id(&self) -> &str {
        "albedo_brightness_range"
    }

    fn description(&self) -> &str {
        "Albedo brightness should be in valid PBR range (not fully black or excessively bright)"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let albedo = set.albedo.as_ref()?;

        let (mean_lum, _min_lum, max_lum) = luminance_stats(albedo);
        let clipped = count_clipped_pixels(albedo);

        if mean_lum < 5.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Major,
                format!(
                    "Albedo appears nearly black (mean luminance {:.1}/255).",
                    mean_lum
                ),
            ));
        }

        if max_lum > 250.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Minor,
                format!(
                    "Albedo has very bright pixels (max {:.1}/255). May indicate non-PBR or HDR.",
                    max_lum
                ),
            ));
        }

        if clipped > 0 {
            let total = (albedo.width as usize) * (albedo.height as usize);
            let pct = 100.0 * clipped as f64 / total as f64;
            if pct > 5.0 {
                return Some(Issue::new(
                    self.id(),
                    Severity::Minor,
                    format!("Albedo has {:.1}% clipped pixels (255 or 0).", pct),
                ));
            }
        }

        None
    }
}

/// Rule: Roughness uniformity / black check
pub struct RoughnessUniformityRule;

impl ValidationRule for RoughnessUniformityRule {
    fn id(&self) -> &str {
        "roughness_uniformity"
    }

    fn description(&self) -> &str {
        "Roughness map should have variation; uniformly constant or black may indicate placeholder"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let roughness = set.roughness.as_ref()?;

        let mean = channel_mean(roughness, 0);
        if mean < 5.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Major,
                "Roughness map is nearly black. May indicate missing or incorrect texture.",
            ));
        }

        let stddev = channel_stddev(roughness, 0);
        if stddev < 2.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Minor,
                format!(
                    "Roughness map is nearly uniform (stddev {:.2}, mean {:.1}).",
                    stddev, mean
                ),
            ));
        }

        None
    }
}

/// Resolution threshold for 4K warning (4096)
const RESOLUTION_4K: u32 = 4096;

/// Rule: Warn if texture resolution exceeds 4K
pub struct TextureResolutionRule;

impl ValidationRule for TextureResolutionRule {
    fn id(&self) -> &str {
        "texture_resolution"
    }

    fn description(&self) -> &str {
        "Warns when texture resolution exceeds 4K (4096px)"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let maps = [
            ("albedo", set.albedo.as_ref()),
            ("normal", set.normal.as_ref()),
            ("roughness", set.roughness.as_ref()),
            ("metallic", set.metallic.as_ref()),
            ("ao", set.ao.as_ref()),
            ("height", set.height.as_ref()),
        ];

        let over_4k: Vec<_> = maps
            .into_iter()
            .filter_map(|(name, map)| {
                let m = map?;
                if m.width > RESOLUTION_4K || m.height > RESOLUTION_4K {
                    Some((name, m.width, m.height))
                } else {
                    None
                }
            })
            .collect();

        if over_4k.is_empty() {
            return None;
        }

        let list = over_4k
            .iter()
            .map(|(n, w, h)| format!("{} ({}x{})", n, w, h))
            .collect::<Vec<_>>()
            .join(", ");

        Some(Issue::new(
            self.id(),
            Severity::Major,
            format!(
                "Texture resolution exceeds 4K: {}. Large textures may impact performance.",
                list
            ),
        ))
    }
}

/// Rule: Metallic mid-gray detection (uniformly 128 may indicate placeholder)
pub struct MetallicMidGrayRule;

impl ValidationRule for MetallicMidGrayRule {
    fn id(&self) -> &str {
        "metallic_mid_gray"
    }

    fn description(&self) -> &str {
        "Metallic map uniformly mid-gray may indicate non-metallic or placeholder"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let metallic = set.metallic.as_ref()?;

        let mean = channel_mean(metallic, 0);
        let stddev = channel_stddev(metallic, 0);

        if (mean - 128.0).abs() < 5.0 && stddev < 2.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Minor,
                "Metallic map is uniformly mid-gray. May indicate uniform or placeholder.",
            ));
        }
        None
    }
}

/// Rule: Normal map strength / blue channel check
pub struct NormalMapStrengthRule;

impl ValidationRule for NormalMapStrengthRule {
    fn id(&self) -> &str {
        "normal_map_strength"
    }

    fn description(&self) -> &str {
        "Normal map blue channel should be dominant (tangent-space normals point up)"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let normal = set.normal.as_ref()?;

        let mean_b = channel_mean(normal, 2);
        if mean_b < 100.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Minor,
                format!(
                    "Normal map blue channel low (mean {:.1}). Tangent-space normals typically have dominant blue.",
                    mean_b
                ),
            ));
        }
        None
    }
}

/// Rule: Tileability / edge difference detection
pub struct TileabilityRule;

impl ValidationRule for TileabilityRule {
    fn id(&self) -> &str {
        "tileability"
    }

    fn description(&self) -> &str {
        "Detect obvious seams at texture edges (simple edge difference)"
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let albedo = set.albedo.as_ref()?;
        let w = albedo.width as usize;
        let h = albedo.height as usize;
        if w < 4 || h < 4 {
            return None;
        }

        let edge_diff = edge_difference(albedo);
        if edge_diff > 40.0 {
            return Some(Issue::new(
                self.id(),
                Severity::Minor,
                format!(
                    "High edge difference ({:.1}). Texture may not tile seamlessly.",
                    edge_diff
                ),
            ));
        }
        None
    }
}

fn count_clipped_pixels(map: &TextureMap) -> usize {
    map.data
        .chunks_exact(4)
        .filter(|p| p[0] == 0 || p[0] == 255 || p[1] == 0 || p[1] == 255 || p[2] == 0 || p[2] == 255)
        .count()
}

fn edge_difference(map: &TextureMap) -> f64 {
    let w = map.width as usize;
    let h = map.height as usize;
    let mut sum = 0.0f64;
    let mut count = 0usize;

    for x in 0..w {
        let top = (0 * w + x) * 4;
        let bottom = ((h - 1) * w + x) * 4;
        if top + 3 < map.data.len() && bottom + 3 < map.data.len() {
            let d = (map.data[top] as i32 - map.data[bottom] as i32).abs()
                + (map.data[top + 1] as i32 - map.data[bottom + 1] as i32).abs()
                + (map.data[top + 2] as i32 - map.data[bottom + 2] as i32).abs();
            sum += d as f64;
            count += 1;
        }
    }
    for y in 0..h {
        let left = (y * w + 0) * 4;
        let right = (y * w + (w - 1)) * 4;
        if left + 3 < map.data.len() && right + 3 < map.data.len() {
            let d = (map.data[left] as i32 - map.data[right] as i32).abs()
                + (map.data[left + 1] as i32 - map.data[right + 1] as i32).abs()
                + (map.data[left + 2] as i32 - map.data[right + 2] as i32).abs();
            sum += d as f64;
            count += 1;
        }
    }

    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

/// Compute luminance stats (0-255 scale) for RGB
fn luminance_stats(map: &TextureMap) -> (f64, f64, f64) {
    let mut sum = 0.0f64;
    let mut min_val = 255.0f64;
    let mut max_val = 0.0f64;
    let mut count = 0usize;

    for i in (0..map.data.len()).step_by(4) {
        if i + 3 > map.data.len() {
            break;
        }
        let r = map.data[i] as f64;
        let g = map.data[i + 1] as f64;
        let b = map.data[i + 2] as f64;
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;

        sum += lum;
        min_val = min_val.min(lum);
        max_val = max_val.max(lum);
        count += 1;
    }

    let mean = if count > 0 { sum / count as f64 } else { 0.0 };
    (mean, min_val, max_val)
}

fn channel_mean(map: &TextureMap, channel: usize) -> f64 {
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for i in (channel..map.data.len()).step_by(4) {
        sum += map.data[i] as f64;
        count += 1;
    }
    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

fn channel_stddev(map: &TextureMap, channel: usize) -> f64 {
    let mean = channel_mean(map, channel);
    let mut sum_sq = 0.0f64;
    let mut count = 0usize;
    for i in (channel..map.data.len()).step_by(4) {
        let v = map.data[i] as f64 - mean;
        sum_sq += v * v;
        count += 1;
    }
    if count > 1 {
        (sum_sq / (count - 1) as f64).sqrt()
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{MaterialSet, TextureMap};

    fn make_texture_map(width: u32, height: u32, data: Vec<u8>) -> TextureMap {
        TextureMap {
            width,
            height,
            data,
            path: None,
        }
    }

    #[test]
    fn required_maps_critical_when_missing() {
        let set = MaterialSet::new();
        let issue = RequiredMapsRule.check(&set);
        assert!(issue.is_some());
        assert!(issue.unwrap().message.contains("albedo"));
    }

    #[test]
    fn albedo_brightness_major_on_black() {
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture_map(
            2,
            2,
            vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255],
        ));
        set.normal = Some(make_texture_map(2, 2, vec![128u8; 16]));
        let issue = AlbedoBrightnessRule.check(&set);
        assert!(issue.is_some());
        assert!(issue.unwrap().message.contains("black"));
    }

    #[test]
    fn albedo_brightness_passes_on_valid() {
        let mut set = MaterialSet::new();
        let data: Vec<u8> = (0..4).flat_map(|_| [128u8, 128, 128, 255]).collect();
        set.albedo = Some(make_texture_map(2, 2, data));
        set.normal = Some(make_texture_map(2, 2, vec![128u8; 16]));
        let issue = AlbedoBrightnessRule.check(&set);
        assert!(issue.is_none());
    }

    #[test]
    fn roughness_uniformity_minor_on_constant() {
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture_map(4, 4, vec![128u8; 256]));
        set.normal = Some(make_texture_map(4, 4, vec![128u8; 256]));
        set.roughness = Some(make_texture_map(4, 4, vec![128u8; 256]));
        let issue = RoughnessUniformityRule.check(&set);
        assert!(issue.is_some());
        assert!(issue.unwrap().message.contains("uniform"));
    }

    #[test]
    fn texture_resolution_major_over_4k() {
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture_map(4097, 2, vec![128; 4097 * 2 * 4]));
        set.normal = Some(make_texture_map(4097, 2, vec![128; 4097 * 2 * 4]));
        let issue = TextureResolutionRule.check(&set);
        assert!(issue.is_some());
        assert!(issue.unwrap().message.contains("4K"));
    }

    #[test]
    fn validator_returns_issues() {
        let validator = Validator::default();
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture_map(2, 2, vec![0u8; 16]));
        set.normal = Some(make_texture_map(2, 2, vec![128u8; 16]));
        set.roughness = Some(make_texture_map(4, 4, vec![128u8; 256]));
        let issues = validator.check(&set);
        assert!(issues.len() >= 2);
    }

    #[test]
    fn compute_score() {
        use crate::validation::{compute_score, Issue};
        let issues = vec![
            Issue::new("r1", Severity::Critical, "c"),
            Issue::new("r2", Severity::Major, "m"),
        ];
        assert_eq!(compute_score(&issues), 70); // 100 - 20 - 10
    }
}

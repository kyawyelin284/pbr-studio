//! JSON report generation for PBR material analysis.
//!
//! Exports structured reports as JSON using serde.

use crate::estimation::{estimate_vram, VramEstimate};
use crate::material::{MaterialSet, TextureSet};
use crate::validation::Issue;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Severity level for issues (JSON output)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Major,
    Minor,
}

impl From<crate::validation::Severity> for Severity {
    fn from(s: crate::validation::Severity) -> Self {
        match s {
            crate::validation::Severity::Critical => Severity::Critical,
            crate::validation::Severity::Major => Severity::Major,
            crate::validation::Severity::Minor => Severity::Minor,
        }
    }
}

/// A validation issue in the JSON report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportIssue {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
}

impl From<Issue> for ReportIssue {
    fn from(issue: Issue) -> Self {
        ReportIssue {
            rule_id: issue.rule_id,
            severity: issue.severity.into(),
            message: issue.message,
        }
    }
}

/// A suggestion for optimizing the material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// Category of the suggestion (e.g., "resolution", "format", "workflow")
    pub category: String,
    /// Human-readable suggestion message
    pub message: String,
    /// Optional priority hint (higher = more impactful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    /// Additional context or details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl OptimizationSuggestion {
    pub fn new(category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            message: message.into(),
            priority: None,
            details: None,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Complete material report for JSON export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialReport {
    /// Material or folder name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Material score (0-100)
    pub score: i32,
    /// Summary of texture maps present
    pub summary: MaterialSummary,
    /// Validation issues found
    pub issues: Vec<ReportIssue>,
    /// Optimization suggestions
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
    /// Overall validation passed (no critical issues)
    pub passed: bool,
    /// Count of critical-level issues
    pub error_count: usize,
    /// Count of major-level issues
    pub warning_count: usize,
    /// VRAM estimate (when available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_estimate: Option<VramEstimate>,
    /// AI-assisted insights (classification, smart suggestions, anomalies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_insights: Option<crate::ai::AiInsights>,
}

/// Summary of material texture set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSummary {
    pub texture_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<TextureDimensions>,
    pub maps: MapSummary,
    pub dimensions_consistent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSummary {
    pub albedo: bool,
    pub normal: bool,
    pub roughness: bool,
    pub metallic: bool,
    pub ao: bool,
    pub height: bool,
}

impl MaterialReport {
    /// Build a report from a material set and validation issues
    pub fn from_material_set(set: &MaterialSet, issues: Vec<Issue>) -> Self {
        Self::from_material_set_with_ai(set, issues, None)
    }

    /// Build a report with optional ONNX model path for ML-based material classification
    pub fn from_material_set_with_ai(
        set: &MaterialSet,
        issues: Vec<Issue>,
        onnx_path: Option<&Path>,
    ) -> Self {
        let texture_set = TextureSet::from(set);
        let analysis = crate::material::MaterialAnalyzer::analyze(&texture_set);

        let ai_insights = crate::ai::analyze_material(set, onnx_path);

        // Append AI anomalies as minor issues
        let mut issues = issues;
        if let Some(ref anomalies) = ai_insights.anomalies {
            for a in anomalies {
                issues.push(Issue::new(
                    "ai_anomaly",
                    crate::validation::Severity::Minor,
                    format!("[{}] {}", a.slot, a.message),
                ));
            }
        }

        let error_count = issues.iter().filter(|i| i.severity == crate::validation::Severity::Critical).count();
        let warning_count = issues.iter().filter(|i| i.severity == crate::validation::Severity::Major).count();
        let passed = error_count == 0;
        let score = crate::validation::compute_score(&issues);

        let mut optimization_suggestions = Self::derive_suggestions(set, &issues);

        // Merge AI smart suggestions into optimization_suggestions
        if let Some(ref smart) = ai_insights.smart_suggestions {
            for s in smart {
                optimization_suggestions.push(
                    OptimizationSuggestion::new(&s.category, &s.message)
                        .with_priority((s.confidence * 10.0) as u8)
                        .with_details(&format!("AI confidence: {:.0}%", s.confidence * 100.0)),
                );
            }
        }

        let vram_estimate = estimate_vram(set, true, Self::can_pack_orm(set));

        MaterialReport {
            name: set.name.clone(),
            score,
            summary: MaterialSummary {
                texture_count: analysis.texture_count,
                dimensions: set.dimensions().map(|(w, h)| TextureDimensions {
                    width: w,
                    height: h,
                }),
                maps: MapSummary {
                    albedo: analysis.has_albedo,
                    normal: analysis.has_normal,
                    roughness: analysis.has_roughness,
                    metallic: analysis.has_metallic,
                    ao: analysis.has_ao,
                    height: set.has_height(),
                },
                dimensions_consistent: analysis.dimensions_consistent,
            },
            issues: issues.into_iter().map(ReportIssue::from).collect(),
            optimization_suggestions,
            passed,
            error_count,
            warning_count,
            vram_estimate: Some(vram_estimate),
            ai_insights: Some(ai_insights),
        }
    }

    fn can_pack_orm(set: &MaterialSet) -> bool {
        set.roughness.is_some() && set.metallic.is_some() && set.ao.is_some()
    }

    fn derive_suggestions(set: &MaterialSet, issues: &[Issue]) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        for issue in issues {
            match issue.rule_id.as_str() {
                "texture_resolution" => {
                    suggestions.push(
                        OptimizationSuggestion::new(
                            "resolution",
                            "Consider downscaling textures over 4K to reduce memory and improve load times",
                        )
                        .with_priority(2)
                        .with_details(&issue.message),
                    );
                }
                "albedo_brightness_range" => {
                    suggestions.push(
                        OptimizationSuggestion::new(
                            "pbr_correctness",
                            "Verify albedo texture values are in valid PBR range",
                        )
                        .with_details(&issue.message),
                    );
                }
                "roughness_uniformity" => {
                    suggestions.push(
                        OptimizationSuggestion::new(
                            "workflow",
                            "Use a proper roughness texture for more realistic surface variation",
                        )
                        .with_details(&issue.message),
                    );
                }
                _ => {}
            }
        }

        if set.dimensions().map_or(false, |(w, h)| w > 2048 || h > 2048) && suggestions.is_empty() {
            suggestions.push(
                OptimizationSuggestion::new(
                    "resolution",
                    "Textures above 2K may be larger than needed for many use cases",
                )
                .with_priority(1),
            );
        }

        suggestions
    }

    /// Serialize to formatted JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to compact JSON string
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{MaterialSet, TextureMap};

    #[test]
    fn material_report_serializes_to_json() {
        let mut set = MaterialSet::new();
        set.name = Some("TestMaterial".into());
        set.albedo = Some(TextureMap {
            width: 4,
            height: 4,
            data: vec![128; 4 * 4 * 4],
            path: None,
        });

        let issues = vec![
            crate::validation::Issue::new("test_rule", crate::validation::Severity::Major, "Test issue message"),
        ];

        let report = MaterialReport::from_material_set(&set, issues);
        let json = report.to_json().unwrap();

        assert!(json.contains("TestMaterial"));
        assert!(json.contains("test_rule"));
        assert!(json.contains("Test issue message"));
        assert!(json.contains("summary"));
        assert!(json.contains("issues"));
        assert!(json.contains("optimization_suggestions"));

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("name").is_some());
        assert!(parsed.get("summary").is_some());
        assert!(parsed.get("issues").is_some());
        assert!(parsed.get("optimization_suggestions").is_some());
    }
}

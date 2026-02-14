//! Report generation from analysis and validation results.
//!
//! Produces structured reports suitable for CLI output,
//! JSON export, or UI display.

use crate::material::{MaterialAnalysis, MaterialSet, TextureSet};
use crate::validation::{Issue, Severity, ValidationResult};
use serde::Serialize;

/// Complete analysis report for a PBR texture set
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub name: Option<String>,
    pub analysis: MaterialAnalysis,
    pub validation_results: Vec<ValidationResult>,
    pub passed: bool,
    pub error_count: usize,
    pub warning_count: usize,
}

/// Builder for constructing reports
pub struct ReportBuilder {
    name: Option<String>,
    analysis: Option<MaterialAnalysis>,
    validation_results: Vec<ValidationResult>,
}

impl ReportBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            analysis: None,
            validation_results: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_analysis(mut self, analysis: MaterialAnalysis) -> Self {
        self.analysis = Some(analysis);
        self
    }

    pub fn with_validation_results(mut self, results: Vec<ValidationResult>) -> Self {
        self.validation_results = results;
        self
    }

    pub fn add_validation_result(mut self, result: ValidationResult) -> Self {
        self.validation_results.push(result);
        self
    }

    pub fn build(self) -> Report {
        let analysis = self.analysis.unwrap_or_default();
        let error_count = self
            .validation_results
            .iter()
            .filter(|r| r.severity == Severity::Critical)
            .count();
        let warning_count = self
            .validation_results
            .iter()
            .filter(|r| r.severity == Severity::Major)
            .count();
        let passed = error_count == 0;

        Report {
            name: self.name,
            analysis,
            validation_results: self.validation_results,
            passed,
            error_count,
            warning_count,
        }
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MaterialAnalysis {
    fn default() -> Self {
        Self {
            has_albedo: false,
            has_normal: false,
            has_metallic: false,
            has_roughness: false,
            has_ao: false,
            dimensions_consistent: true,
            texture_count: 0,
        }
    }
}

impl Report {
    /// Create a report from a texture set and validation results
    pub fn from_texture_set(
        set: &TextureSet,
        validation_results: Vec<ValidationResult>,
        name: Option<String>,
    ) -> Self {
        ReportBuilder::new()
            .with_name(name.unwrap_or_else(|| "Unnamed".to_string()))
            .with_analysis(crate::material::MaterialAnalyzer::analyze(set))
            .with_validation_results(validation_results)
            .build()
    }

    /// Create a report from a material set and validation issues
    pub fn from_material_set(set: &MaterialSet, issues: Vec<Issue>) -> Self {
        let texture_set = TextureSet::from(set);
        let validation_results: Vec<ValidationResult> =
            issues.into_iter().map(ValidationResult::from).collect();
        ReportBuilder::new()
            .with_name(set.name.clone().unwrap_or_else(|| "Unnamed".to_string()))
            .with_analysis(crate::material::MaterialAnalyzer::analyze(&texture_set))
            .with_validation_results(validation_results)
            .build()
    }

    /// Format as human-readable text
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();

        if let Some(name) = &self.name {
            lines.push(format!("Report: {}", name));
            lines.push(String::new());
        }

        lines.push("Analysis".to_string());
        lines.push(format!("  Textures: {}", self.analysis.texture_count));
        lines.push(format!("  Albedo: {}", self.analysis.has_albedo));
        lines.push(format!("  Normal: {}", self.analysis.has_normal));
        lines.push(format!("  Metallic: {}", self.analysis.has_metallic));
        lines.push(format!("  Roughness: {}", self.analysis.has_roughness));
        lines.push(format!("  AO: {}", self.analysis.has_ao));
        lines.push(format!(
            "  Dimensions consistent: {}",
            self.analysis.dimensions_consistent
        ));
        lines.push(String::new());

        lines.push("Validation".to_string());
        for result in &self.validation_results {
            let status = if result.passed { "✓" } else { "✗" };
            let severity = format!("{:?}", result.severity).to_lowercase();
            lines.push(format!("  {} [{}] {}: {}", status, severity, result.rule_id, result.message));
        }
        lines.push(String::new());

        lines.push(format!(
            "Result: {} ({} errors, {} warnings)",
            if self.passed { "PASSED" } else { "FAILED" },
            self.error_count,
            self.warning_count
        ));

        lines.join("\n")
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

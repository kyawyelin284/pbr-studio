//! Local audit logs for PBR Studio.
//!
//! Tracks every validation, optimization, and report generation action.
//! Supports "Material Certified for Pipeline" badge for approved materials.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const AUDIT_FILENAME: &str = "audit.json";
const BADGE_FILENAME: &str = "certified.svg";
const MAX_ENTRIES: usize = 1000;

/// Action type for audit entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Validation,
    Optimization,
    ReportGeneration,
}

/// A single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: AuditAction,
    pub material_path: Option<String>,
    pub score: Option<i32>,
    pub passed: Option<bool>,
    pub min_score: Option<i32>,
    pub issue_count: Option<usize>,
    pub error_count: Option<usize>,
    pub warning_count: Option<usize>,
    pub output_path: Option<String>,
    pub preset: Option<String>,
    pub format: Option<String>,
    pub texture_count: Option<usize>,
    pub certified: bool,
}

/// In-memory audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: AuditEntry) {
        self.entries.insert(0, entry);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Default audit log path: ~/.config/pbr-studio/audit.json or PBR_STUDIO_AUDIT_PATH
pub fn default_audit_path() -> PathBuf {
    if let Ok(p) = std::env::var("PBR_STUDIO_AUDIT_PATH") {
        return PathBuf::from(p);
    }
    let config = std::env::var("XDG_CONFIG_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{}/.config", h)))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(config).join("pbr-studio").join(AUDIT_FILENAME)
}

fn ensure_config_dir(path: &Path) -> Result<(), crate::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Load audit log from path
pub fn load_audit_log(path: Option<&Path>) -> Result<AuditLog, crate::Error> {
    let default = default_audit_path();
    let path = path.unwrap_or(&default);
    if !path.exists() {
        return Ok(AuditLog::new());
    }
    let bytes = std::fs::read(path)?;
    let log: AuditLog = serde_json::from_slice(&bytes)
        .map_err(|e| crate::Error::Other(format!("Invalid audit log: {}", e)))?;
    Ok(log)
}

/// Save audit log to path (JSON format)
pub fn save_audit_log(path: Option<&Path>, log: &AuditLog) -> Result<(), crate::Error> {
    let default = default_audit_path();
    let path = path.unwrap_or(&default);
    ensure_config_dir(path)?;
    let json = serde_json::to_string_pretty(log)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Export audit log as human-readable text (for file output or display)
pub fn export_audit_log_text(log: &AuditLog, limit: Option<usize>) -> String {
    let slice = match limit {
        Some(n) => &log.entries[..log.entries.len().min(n)],
        None => &log.entries[..],
    };
    let mut lines = Vec::new();
    for e in slice {
        let action = match &e.action {
            AuditAction::Validation => "validation",
            AuditAction::Optimization => "optimization",
            AuditAction::ReportGeneration => "report",
        };
        let path = e.material_path.as_deref().unwrap_or("-");
        let score = e
            .score
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "-".to_string());
        let min = e
            .min_score
            .map(|m| format!("/{}", m))
            .unwrap_or_default();
        let certified = if e.certified { " [certified]" } else { "" };
        let mut line = format!("{} [{}] path={} score={}{}{}", e.timestamp, action, path, score, min, certified);
        if let Some(c) = e.issue_count {
            line.push_str(&format!(" issues={}", c));
        }
        if let Some(o) = &e.output_path {
            line.push_str(&format!(" output={}", o));
        }
        if let Some(p) = &e.preset {
            line.push_str(&format!(" preset={}", p));
        }
        if let Some(f) = &e.format {
            line.push_str(&format!(" format={}", f));
        }
        lines.push(line);
    }
    lines.join("\n")
}

/// Save audit log to path as text file
pub fn save_audit_log_text(path: &Path, log: &AuditLog, limit: Option<usize>) -> Result<(), crate::Error> {
    ensure_config_dir(path)?;
    let text = export_audit_log_text(log, limit);
    std::fs::write(path, text)?;
    Ok(())
}

/// Record a validation action
pub fn record_validation(
    material_path: &Path,
    score: i32,
    passed: bool,
    min_score: i32,
    issue_count: usize,
    error_count: usize,
    warning_count: usize,
    audit_path: Option<&Path>,
) -> Result<(), crate::Error> {
    let certified = passed && score >= min_score;
    let mut log = load_audit_log(audit_path)?;
    log.add(AuditEntry {
        timestamp: Utc::now().to_rfc3339(),
        action: AuditAction::Validation,
        material_path: Some(material_path.to_string_lossy().to_string()),
        score: Some(score),
        passed: Some(passed),
        min_score: Some(min_score),
        issue_count: Some(issue_count),
        error_count: Some(error_count),
        warning_count: Some(warning_count),
        output_path: None,
        preset: None,
        format: None,
        texture_count: None,
        certified,
    });
    save_audit_log(audit_path, &log)?;
    if certified {
        let _ = write_certified_badge(material_path);
    }
    Ok(())
}

/// Record an optimization/export action
pub fn record_optimization(
    material_path: &Path,
    output_path: &Path,
    preset: &str,
    texture_count: usize,
    audit_path: Option<&Path>,
) -> Result<(), crate::Error> {
    let mut log = load_audit_log(audit_path)?;
    log.add(AuditEntry {
        timestamp: Utc::now().to_rfc3339(),
        action: AuditAction::Optimization,
        material_path: Some(material_path.to_string_lossy().to_string()),
        score: None,
        passed: None,
        min_score: None,
        issue_count: None,
        error_count: None,
        warning_count: None,
        output_path: Some(output_path.to_string_lossy().to_string()),
        preset: Some(preset.to_string()),
        format: None,
        texture_count: Some(texture_count),
        certified: false,
    });
    save_audit_log(audit_path, &log)
}

/// Record a report generation action
pub fn record_report(
    material_path: Option<&Path>,
    format: &str,
    output_path: &Path,
    score: Option<i32>,
    passed: Option<bool>,
    audit_path: Option<&Path>,
) -> Result<(), crate::Error> {
    let mut log = load_audit_log(audit_path)?;
    log.add(AuditEntry {
        timestamp: Utc::now().to_rfc3339(),
        action: AuditAction::ReportGeneration,
        material_path: material_path.map(|p| p.to_string_lossy().to_string()),
        score,
        passed,
        min_score: None,
        issue_count: None,
        error_count: None,
        warning_count: None,
        output_path: Some(output_path.to_string_lossy().to_string()),
        preset: None,
        format: Some(format.to_string()),
        texture_count: None,
        certified: false,
    });
    save_audit_log(audit_path, &log)
}

/// Write "Material Certified for Pipeline" badge SVG to material folder
pub fn write_certified_badge(material_folder: &Path) -> Result<PathBuf, crate::Error> {
    let dir = material_folder.join(".pbr-studio");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(BADGE_FILENAME);
    let svg = certified_badge_svg();
    std::fs::write(&path, svg)?;
    Ok(path)
}

/// Check if material has a certified badge
pub fn has_certified_badge(material_folder: &Path) -> bool {
    material_folder.join(".pbr-studio").join(BADGE_FILENAME).exists()
}

/// Generate SVG badge content
fn certified_badge_svg() -> String {
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="80" viewBox="0 0 200 80">
  <defs>
    <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#198754"/>
      <stop offset="100%" style="stop-color:#20c997"/>
    </linearGradient>
  </defs>
  <rect width="200" height="80" rx="8" fill="url(#grad)"/>
  <text x="100" y="32" font-family="system-ui,sans-serif" font-size="14" font-weight="bold" fill="white" text-anchor="middle">✓ Certified</text>
  <text x="100" y="52" font-family="system-ui,sans-serif" font-size="10" fill="rgba(255,255,255,0.9)" text-anchor="middle">Material Ready for Pipeline</text>
</svg>"#.to_string()
}

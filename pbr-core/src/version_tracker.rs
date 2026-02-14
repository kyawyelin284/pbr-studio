//! Local version tracking for material folders.
//!
//! Stores a changelog per material folder in `.pbr-studio/versions.json`.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use chrono::Utc;

const VERSIONS_FILE: &str = ".pbr-studio/versions.json";
const MAX_ENTRIES: usize = 50;

/// A single version entry in the changelog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub timestamp: String,
    pub score: i32,
    pub passed: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub issue_count: usize,
}

/// Changelog for a material folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionLog {
    pub folder: String,
    pub entries: Vec<VersionEntry>,
}

impl VersionLog {
    pub fn new(folder: impl Into<String>) -> Self {
        Self {
            folder: folder.into(),
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, score: i32, passed: bool, error_count: usize, warning_count: usize, issue_count: usize) {
        let entry = VersionEntry {
            timestamp: Utc::now().to_rfc3339(),
            score,
            passed,
            error_count,
            warning_count,
            issue_count,
        };
        self.entries.insert(0, entry);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
    }
}

/// Load version log for a material folder
pub fn load_version_log(material_folder: &Path) -> Result<VersionLog, crate::Error> {
    let log_path = material_folder.join(VERSIONS_FILE);
    if !log_path.exists() {
        return Ok(VersionLog::new(
            material_folder.to_string_lossy().to_string(),
        ));
    }
    let bytes = fs::read(&log_path)?;
    let mut log: VersionLog = serde_json::from_slice(&bytes)
        .map_err(|e| crate::Error::Other(format!("Invalid versions.json: {}", e)))?;
    log.folder = material_folder.to_string_lossy().to_string();
    Ok(log)
}

/// Save version log for a material folder
pub fn save_version_log(material_folder: &Path, log: &VersionLog) -> Result<(), crate::Error> {
    let dir = material_folder.join(".pbr-studio");
    fs::create_dir_all(&dir)?;
    let path = dir.join("versions.json");
    let json = serde_json::to_string_pretty(log)?;
    fs::write(path, json)?;
    Ok(())
}

/// Record a new analysis result for a material folder
pub fn record_analysis(
    material_folder: &Path,
    score: i32,
    passed: bool,
    error_count: usize,
    warning_count: usize,
    issue_count: usize,
) -> Result<(), crate::Error> {
    let mut log = load_version_log(material_folder)?;
    log.add_entry(score, passed, error_count, warning_count, issue_count);
    save_version_log(material_folder, &log)
}

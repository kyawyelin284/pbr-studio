//! In-memory undo stack for tracking validation and optimization changes per material.
//!
//! Stores the last N operations locally. No backend or cloud storage.
//! Used by CLI/UI to support revert of recent actions.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// Operation type for undo tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoAction {
    Validation,
    Optimization,
    ReportGeneration,
}

/// A single undo entry (metadata only; state is held by the caller)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    pub action: UndoAction,
    pub material_path: Option<String>,
    pub timestamp: String,
    pub score: Option<i32>,
    pub preset: Option<String>,
}

/// Thread-safe in-memory undo stack. Max size configured at creation.
pub struct UndoStack {
    pub max_size: usize,
    entries: Mutex<Vec<UndoEntry>>,
}

impl UndoStack {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size: max_size.max(1),
            entries: Mutex::new(Vec::new()),
        }
    }

    /// Push an entry. Drops oldest if over max_size.
    pub fn push(&self, entry: UndoEntry) {
        let mut guard = self.entries.lock().expect("undo stack lock");
        guard.insert(0, entry);
        if guard.len() > self.max_size {
            guard.truncate(self.max_size);
        }
    }

    /// Record a validation action
    pub fn record_validation(&self, material_path: &Path, score: i32) {
        self.push(UndoEntry {
            action: UndoAction::Validation,
            material_path: Some(material_path.to_string_lossy().to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            score: Some(score),
            preset: None,
        });
    }

    /// Record an optimization action
    pub fn record_optimization(&self, material_path: &Path, preset: &str) {
        self.push(UndoEntry {
            action: UndoAction::Optimization,
            material_path: Some(material_path.to_string_lossy().to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            score: None,
            preset: Some(preset.to_string()),
        });
    }

    /// Record a report generation action
    pub fn record_report(&self, material_path: Option<&Path>) {
        self.push(UndoEntry {
            action: UndoAction::ReportGeneration,
            material_path: material_path.map(|p| p.to_string_lossy().to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            score: None,
            preset: None,
        });
    }

    /// Get the last N entries (newest first)
    pub fn entries(&self, limit: usize) -> Vec<UndoEntry> {
        let guard = self.entries.lock().expect("undo stack lock");
        guard.iter().take(limit).cloned().collect()
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.entries.lock().expect("undo stack lock").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut guard = self.entries.lock().expect("undo stack lock");
        guard.clear();
    }
}

//! # PBR Core
//!
//! Engine for analyzing PBR (Physically Based Rendering) texture sets.
//! Designed for use by CLI tools and desktop applications.
//!
//! ## Architecture
//!
//! - [`image_loading`] - Image loading and texture metadata
//! - [`material`] - Material and texture set analysis
//! - [`validation`] - Validation rules and checks
//! - [`report`] - Report generation from analysis results
//! - [`analysis`] - Advanced analysis (duplicates, cross-material, tileability)
//! - [`estimation`] - GPU/CPU VRAM estimation

pub mod ai;
pub mod analysis;
pub mod audit_log;
pub mod estimation;
pub mod image_loading;
pub mod json_report;
pub mod material;
pub mod optimization;
pub mod plugin;
pub mod report;
pub mod report_export;
pub mod validation;
pub mod undo_stack;
pub mod version_tracker;

// Re-export main types for convenient access
pub use image_loading::{ExrValidationReport, ImageLoader, LoadedImage, TextureSlot};
pub use json_report::{MaterialReport, OptimizationSuggestion, ReportIssue};
pub use report_export::{export_html_batch, export_html_single, export_pdf_batch, export_pdf_single};
pub use version_tracker::{record_analysis, load_version_log, VersionEntry, VersionLog};
pub use undo_stack::{UndoAction, UndoEntry, UndoStack};
pub use audit_log::{
    default_audit_path, export_audit_log_text, has_certified_badge, load_audit_log,
    record_optimization, record_report, record_validation, save_audit_log_text, write_certified_badge,
    AuditAction, AuditEntry, AuditLog,
};
pub use material::{MaterialAnalyzer, MaterialSet, TextureMap, TextureSet};
pub use report::{Report, ReportBuilder};
pub use optimization::{
    batch_export_with_optimization_preset, batch_export_with_preset, export_with_lod,
    export_with_optimization_preset, export_with_preset, export_with_target,
    export_with_target_and_lod, generate_lod_chain,
    pack_rma, pack_rma_from_material, resize_and_save_texture, resize_material_set,
    resize_texture, save_texture, ExportPreset, OptimizationPreset, TargetResolution,
};
pub use estimation::{estimate_vram, VramEstimate};
pub use validation::{compute_score, Issue, ValidationResult, ValidationRule, Validator};
pub use ai::{
    ai_analyze_json, analyze_material, classify_material, detect_anomalies, suggest_optimizations,
    AiInsights, AiSuggestion, Anomaly, MaterialClass, AI_ONNX_ENABLED,
};
pub use plugin::{
    PluginInfo, PluginLoader, PluginManifest, PresetConfig, RuleConfig, RuleCondition,
};
pub use analysis::{
    analyze_tileability, detect_duplicates, analyze_cross_material, edge_difference,
    fix_tileability, fix_tileability_with_report,
    run_advanced_analysis, run_advanced_analysis_and_write,
    AdvancedAnalysisReport, CrossMaterialResult, DuplicateAnalysisResult, DuplicatePair,
    TileabilityAnalysisEntry, TileabilityFixResult,
    TILEABILITY_THRESHOLD,
};


/// Common result type for PBR operations
pub type Result<T> = std::result::Result<T, Error>;

/// Library-wide error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

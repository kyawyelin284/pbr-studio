use pbr_core::{
    ai_analyze_json, export_audit_log_text, export_html_batch, export_html_single,
    export_pdf_batch, export_pdf_single, export_with_lod, export_with_preset, export_with_target,
    load_audit_log, record_analysis, record_optimization as audit_record_optimization,
    record_report as audit_record_report, record_validation as audit_record_validation,
    save_audit_log_text, ExportPreset, MaterialReport, MaterialSet, PluginInfo, PluginLoader,
    Validator,
};
use pbr_core::optimization::TargetResolution;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct AnalyzeFolderPayload {
    pub path: String,
}

fn build_loader(plugins_dir: Option<&str>) -> PluginLoader {
    let mut loader = PluginLoader::new().with_default_paths();
    if let Some(dir) = plugins_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            loader = loader.add_dir(trimmed);
        }
    }
    loader
}

fn get_validator(plugins_dir: Option<&str>) -> Validator {
    let loader = build_loader(plugins_dir);
    Validator::default().with_plugins(&loader)
}

#[tauri::command]
fn analyze_folder(path: String, plugins_dir: Option<String>) -> Result<String, String> {
    let set = MaterialSet::load_from_folder(&path).map_err(|e| e.to_string())?;
    let validator = get_validator(plugins_dir.as_deref());
    let issues = validator.check(&set);
    let report = MaterialReport::from_material_set(&set, issues);
    let score = report.score;
    let passed = report.passed;
    let min_score = 70;
    let critical = issues.iter().filter(|i| i.severity == pbr_core::validation::Severity::Critical).count();
    let major = issues.iter().filter(|i| i.severity == pbr_core::validation::Severity::Major).count();
    let _ = audit_record_validation(
        std::path::Path::new(&path),
        score,
        passed,
        min_score,
        issues.len(),
        critical,
        major,
        None,
    );
    report.to_json().map_err(|e| e.to_string())
}

#[tauri::command]
fn analyze_folders(paths: Vec<String>, plugins_dir: Option<String>) -> Result<Vec<String>, String> {
    let validator = get_validator(plugins_dir.as_deref());
    let mut results = Vec::with_capacity(paths.len());
    for path in paths {
        match MaterialSet::load_from_folder(&path) {
            Ok(set) => {
                let issues = validator.check(&set);
                let report = MaterialReport::from_material_set(&set, issues);
                let min_score = 70;
                let critical = issues.iter().filter(|i| i.severity == pbr_core::validation::Severity::Critical).count();
                let major = issues.iter().filter(|i| i.severity == pbr_core::validation::Severity::Major).count();
                let _ = audit_record_validation(
                    std::path::Path::new(&path),
                    report.score,
                    report.passed,
                    min_score,
                    issues.len(),
                    critical,
                    major,
                    None,
                );
                match report.to_json() {
                    Ok(json) => results.push(json),
                    Err(e) => results.push(serde_json::json!({
                        "error": e.to_string(),
                        "path": path,
                        "score": null
                    }).to_string()),
                }
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "error": e.to_string(),
                    "path": path,
                    "score": null
                }).to_string());
            }
        }
    }
    Ok(results)
}

#[tauri::command]
fn export_preset(
    source_path: String,
    output_path: String,
    preset: String,
    include_lod: Option<bool>,
    plugins_dir: Option<String>,
) -> Result<Vec<String>, String> {
    let material = MaterialSet::load_from_folder(&source_path).map_err(|e| e.to_string())?;

    let written = if let Some(preset_enum) = match preset.to_lowercase().as_str() {
        "4k" | "4k_high" => Some(ExportPreset::Res4K),
        "unreal" | "unreal_engine" => Some(ExportPreset::UnrealEngine),
        "unity" => Some(ExportPreset::Unity),
        "mobile" | "mobile_optimized" => Some(ExportPreset::MobileOptimized),
        _ => None,
    } {
        if include_lod == Some(true) {
            let levels = TargetResolution::default_lod_levels();
            export_with_lod(&material, &output_path, preset_enum, levels).map_err(|e| e.to_string())?
        } else {
            export_with_preset(&material, &output_path, preset_enum).map_err(|e| e.to_string())?
        }
    } else {
        // Custom preset from plugin
        let loader = build_loader(plugins_dir.as_deref());
        let (_, presets) = loader.load();
        let custom = presets.iter().find(|p| p.id == preset);
        let target = custom
            .map(|p| TargetResolution::Custom(p.max_dimension()))
            .ok_or_else(|| format!("Unknown preset: {}", preset))?;
        if include_lod == Some(true) {
            let levels = TargetResolution::default_lod_levels();
            let mut written = Vec::new();
            let lod0 = export_with_target(&material, &output_path, target).map_err(|e| e.to_string())?;
            written.extend(lod0);
            for (i, &level) in levels.iter().enumerate() {
                let lod_dir = std::path::Path::new(&output_path).join(format!("LOD{}", i + 1));
                std::fs::create_dir_all(&lod_dir).map_err(|e| e.to_string())?;
                let lod_written = export_with_target(&material, &lod_dir, level).map_err(|e| e.to_string())?;
                written.extend(lod_written);
            }
            written
        } else {
            export_with_target(&material, &output_path, target).map_err(|e| e.to_string())?
        }
    };

    let count = written.len();
    let _ = audit_record_optimization(
        std::path::Path::new(&source_path),
        std::path::Path::new(&output_path),
        &preset,
        count,
        None,
    );

    Ok(written
        .into_iter()
        .filter_map(|p| p.to_str().map(String::from))
        .collect())
}

#[tauri::command]
fn get_plugin_presets(plugins_dir: Option<String>) -> Result<String, String> {
    let loader = build_loader(plugins_dir.as_deref());
    let (_, presets) = loader.load();
    serde_json::to_string(&presets).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_plugins(plugins_dir: Option<String>) -> Result<Vec<PluginInfo>, String> {
    let loader = build_loader(plugins_dir.as_deref());
    Ok(loader.list_loaded())
}

#[tauri::command]
fn ai_analyze(path: String, model_path: Option<String>) -> Result<String, String> {
    let set = MaterialSet::load_from_folder(&path).map_err(|e| e.to_string())?;
    let onnx = model_path.as_deref().map(std::path::Path::new);
    ai_analyze_json(&set, onnx).map_err(|e| e.to_string())
}

#[tauri::command]
fn resolve_material_folder(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("Path does not exist".into());
    }
    let folder = if p.is_dir() {
        p.to_path_buf()
    } else {
        p.parent()
            .ok_or("Could not get parent directory")?
            .to_path_buf()
    };
    folder
        .to_str()
        .map(String::from)
        .ok_or_else(|| "Invalid path".into())
}

fn is_material_folder(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    const EXTS: &[&str] = &["png", "jpg", "jpeg", "tga"];
    const SLOTS: &[&str] = &[
        "albedo", "basecolor", "diffuse", "color",
        "normal", "norm",
        "roughness", "rough",
        "metallic", "metal",
        "ao", "ambientocclusion", "ambient_occlusion",
        "height", "displacement", "bump",
    ];

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if EXTS.contains(&ext.as_str()) && SLOTS.iter().any(|s| stem.contains(s)) {
            return true;
        }
    }
    false
}

fn find_material_folders(root: &Path, dir: &Path, results: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if is_material_folder(&path) {
                results.push(path.clone());
            }
            find_material_folders(root, &path, results)?;
        }
    }
    Ok(())
}

/// Expands dropped paths: if a path is a material folder, add it; if a directory, recursively find all material subfolders.
#[tauri::command]
fn expand_material_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for path_str in paths {
        let p = Path::new(&path_str);
        if !p.exists() {
            continue;
        }
        if p.is_file() {
            if let Some(parent) = p.parent() {
                if is_material_folder(parent) {
                    result.push(parent.to_string_lossy().into_owned());
                }
            }
        } else if p.is_dir() {
            if is_material_folder(p) {
                result.push(path_str);
            } else {
                let mut sub = Vec::new();
                find_material_folders(p, p, &mut sub).map_err(|e| e.to_string())?;
                for fp in sub {
                    if let Some(s) = fp.to_str() {
                        result.push(s.to_string());
                    }
                }
            }
        }
    }
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    result.retain(|p| seen.insert(p.clone()));
    Ok(result)
}

#[tauri::command]
fn export_report(
    paths: Vec<String>,
    format: String,
    output_path: String,
    track: Option<bool>,
) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No paths provided".into());
    }
    let validator = Validator::default();
    let mut reports: Vec<(String, MaterialReport)> = Vec::new();

    for path in &paths {
        let set = MaterialSet::load_from_folder(path).map_err(|e| e.to_string())?;
        let issues = validator.check(&set);
        let report = MaterialReport::from_material_set(&set, issues);
        if track == Some(true) {
            let _ = record_analysis(
                std::path::Path::new(path),
                report.score,
                report.passed,
                report.error_count,
                report.warning_count,
                report.issues.len(),
            );
        }
        reports.push((path.clone(), report));
    }

    let out = std::path::Path::new(&output_path);
    match format.to_lowercase().as_str() {
        "html" => {
            if reports.len() == 1 {
                export_html_single(&reports[0].1, out).map_err(|e| e.to_string())?;
            } else {
                export_html_batch(&reports, out).map_err(|e| e.to_string())?;
            }
        }
        "pdf" => {
            if reports.len() == 1 {
                export_pdf_single(&reports[0].1, out).map_err(|e| e.to_string())?;
            } else {
                export_pdf_batch(&reports, out).map_err(|e| e.to_string())?;
            }
        }
        _ => return Err(format!("Unknown format: {}. Use html or pdf.", format).into()),
    }

    for (path_str, report) in &reports {
        let _ = audit_record_report(
            Some(std::path::Path::new(path_str)),
            &format,
            out,
            Some(report.score),
            Some(report.passed),
            None,
        );
    }

    Ok(())
}

#[tauri::command]
fn get_audit_log(limit: Option<usize>) -> Result<String, String> {
    let log = load_audit_log(None).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);
    let entries: Vec<_> = log.entries.iter().take(limit).cloned().collect();
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_audit_log(output_path: String, format: String, limit: Option<usize>) -> Result<(), String> {
    let log = load_audit_log(None).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(1000);
    let path = std::path::Path::new(&output_path);
    if format.eq_ignore_ascii_case("text") {
        save_audit_log_text(path, &log, Some(limit)).map_err(|e| e.to_string())?;
    } else {
        let entries: Vec<_> = log.entries.iter().take(limit).cloned().collect();
        let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Returns the latest modification time (as Unix timestamp ms) of texture files in the folder.
/// Used for live scoring: when mtime changes, re-analyze and refresh previews.
#[tauri::command]
fn get_material_folder_mtime(path: String) -> Result<Option<i64>, String> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Ok(None);
    }
    let mut latest: Option<std::time::SystemTime> = None;
    let entries = match std::fs::read_dir(p) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
            if matches!(ext.as_deref(), Some("png") | Some("jpg") | Some("jpeg") | Some("tga")) {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        latest = Some(latest.map_or(mtime, |l| mtime.max(l)));
                    }
                }
            }
        }
    }
    Ok(latest.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as i64)))
}

#[tauri::command]
fn get_texture_paths(path: String) -> Result<String, String> {
    let set = MaterialSet::load_from_folder(&path).map_err(|e| e.to_string())?;
    let mut paths = serde_json::Map::new();

    if let Some(ref t) = set.albedo {
        if let Some(ref p) = t.path {
            paths.insert("albedo".into(), p.to_string_lossy().into_owned());
        }
    }
    if let Some(ref t) = set.normal {
        if let Some(ref p) = t.path {
            paths.insert("normal".into(), p.to_string_lossy().into_owned());
        }
    }
    if let Some(ref t) = set.roughness {
        if let Some(ref p) = t.path {
            paths.insert("roughness".into(), p.to_string_lossy().into_owned());
        }
    }
    if let Some(ref t) = set.metallic {
        if let Some(ref p) = t.path {
            paths.insert("metallic".into(), p.to_string_lossy().into_owned());
        }
    }
    if let Some(ref t) = set.ao {
        if let Some(ref p) = t.path {
            paths.insert("ao".into(), p.to_string_lossy().into_owned());
        }
    }
    if let Some(ref t) = set.height {
        if let Some(ref p) = t.path {
            paths.insert("height".into(), p.to_string_lossy().into_owned());
        }
    }

    serde_json::to_string(&paths).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            ai_analyze,
            analyze_folder,
            export_audit_log,
            analyze_folders,
            export_preset,
            export_report,
            expand_material_paths,
            get_audit_log,
            get_material_folder_mtime,
            get_plugin_presets,
            get_texture_paths,
            list_plugins,
            resolve_material_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use pbr_core::{
    ai_analyze_json, batch_export_with_preset, export_html_batch,
    export_html_single, export_pdf_batch, export_pdf_single, export_with_lod, export_with_preset,
    export_with_target, fix_tileability_with_report, load_audit_log, record_analysis,
    record_optimization as audit_record_optimization, record_report as audit_record_report,
    record_validation as audit_record_validation, run_advanced_analysis, save_audit_log_text,
    save_texture, ExportPreset, MaterialReport, MaterialSet, PluginInfo, PluginLoader,
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
    let report = MaterialReport::from_material_set(&set, issues.clone());
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
                let report = MaterialReport::from_material_set(&set, issues.clone());
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
fn batch_export_preset(
    source_paths: Vec<String>,
    output_root: String,
    preset: String,
    include_lod: Option<bool>,
    plugins_dir: Option<String>,
) -> Result<Vec<String>, String> {
    if source_paths.is_empty() {
        return Err("No source paths provided".into());
    }

    let mut materials: Vec<(PathBuf, MaterialSet)> = Vec::new();
    for path_str in &source_paths {
        let material = MaterialSet::load_from_folder(path_str).map_err(|e| e.to_string())?;
        materials.push((PathBuf::from(path_str), material));
    }

    let preset_enum = match preset.to_lowercase().as_str() {
        "4k" | "4k_high" => Some(ExportPreset::Res4K),
        "unreal" | "unreal_engine" => Some(ExportPreset::UnrealEngine),
        "unity" => Some(ExportPreset::Unity),
        "mobile" | "mobile_optimized" => Some(ExportPreset::MobileOptimized),
        _ => None,
    };

    let written: Vec<String> = if let Some(preset_enum) = preset_enum {
        if include_lod == Some(true) {
            use pbr_core::optimization::export_with_lod;
            let output_root = std::path::Path::new(&output_root);
            std::fs::create_dir_all(output_root).map_err(|e| e.to_string())?;
            let levels = pbr_core::optimization::TargetResolution::default_lod_levels();
            let mut all_written = Vec::new();
            for (folder, material) in &materials {
                let name = material
                    .name
                    .clone()
                    .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "material".to_string());
                let material_dir = output_root.join(&name);
                let w = export_with_lod(material, &material_dir, preset_enum, levels).map_err(|e| e.to_string())?;
                let count = w.len();
                all_written.extend(w.into_iter().filter_map(|p| p.to_str().map(String::from)));
                let _ = audit_record_optimization(
                    folder.as_path(),
                    material_dir.as_path(),
                    &preset,
                    count,
                    None,
                );
            }
            all_written
        } else {
            let written = batch_export_with_preset(&materials, &output_root, preset_enum).map_err(|e| e.to_string())?;
            let output_root = std::path::Path::new(&output_root);
            for (folder, material) in &materials {
                let name = material
                    .name
                    .clone()
                    .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "material".to_string());
                let material_dir = output_root.join(&name);
                let _prefix = material_dir.to_string_lossy();
                let count = written.iter().filter(|p| p.starts_with(&material_dir)).count();
                let _ = audit_record_optimization(
                    folder.as_path(),
                    &material_dir,
                    &preset,
                    count,
                    None,
                );
            }
            written.into_iter().filter_map(|p| p.to_str().map(String::from)).collect()
        }
    } else {
        // Custom preset from plugin
        let loader = build_loader(plugins_dir.as_deref());
        let (_, presets) = loader.load();
        let custom = presets.iter().find(|p| p.id == preset);
        let target = custom
            .map(|p| TargetResolution::Custom(p.max_dimension()))
            .ok_or_else(|| format!("Unknown preset: {}", preset))?;
        use pbr_core::optimization::export_with_target;
        let output_root = std::path::Path::new(&output_root);
        std::fs::create_dir_all(output_root).map_err(|e| e.to_string())?;
        let mut all_written = Vec::new();
        for (folder, material) in &materials {
            let name = material
                .name
                .clone()
                .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "material".to_string());
            let material_dir = output_root.join(&name);
            std::fs::create_dir_all(&material_dir).map_err(|e| e.to_string())?;
            let w = export_with_target(material, &material_dir, target).map_err(|e| e.to_string())?;
            let count = w.len();
            all_written.extend(w.into_iter().filter_map(|p| p.to_str().map(String::from)));
            let _ = audit_record_optimization(
                folder.as_path(),
                &material_dir,
                &preset,
                count,
                None,
            );
        }
        all_written
    };

    Ok(written)
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
    const EXTS: &[&str] = &["png", "jpg", "jpeg", "tga", "exr"];
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
            if matches!(ext.as_deref(), Some("png") | Some("jpg") | Some("jpeg") | Some("tga") | Some("exr")) {
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
fn run_advanced_analysis_cmd(
    paths: Vec<String>,
    duplicate_threshold: Option<f32>,
    similar_threshold: Option<f32>,
    tileability_threshold: Option<f32>,
) -> Result<String, String> {
    if paths.is_empty() {
        return Err("No material paths provided".into());
    }
    let mut materials: Vec<(PathBuf, MaterialSet)> = Vec::new();
    for path_str in &paths {
        let material = MaterialSet::load_from_folder(path_str).map_err(|e| e.to_string())?;
        materials.push((PathBuf::from(path_str), material));
    }
    let dup = duplicate_threshold.unwrap_or(0.99);
    let sim = similar_threshold.unwrap_or(0.8);
    let report = run_advanced_analysis(&materials, dup, sim, false).map_err(|e| e.to_string())?;
    let json = report.to_json().map_err(|e| e.to_string())?;
    Ok(json)
}

#[derive(serde::Serialize)]
struct FixTileabilityResult {
    output_path: String,
    original_edge_difference: f32,
    fixed_edge_difference: f32,
    improved: bool,
}

#[tauri::command]
fn fix_tileability_texture(
    path: String,
    output_path: String,
    blend_width: Option<u32>,
) -> Result<FixTileabilityResult, String> {
    use std::ffi::OsStr;
    let path_buf = PathBuf::from(&path);
    let path_buf = path_buf.canonicalize().unwrap_or(path_buf);
    let output_buf = PathBuf::from(&output_path);

    let (texture, out_path) = if path_buf.is_dir() {
        let set = MaterialSet::load_from_folder(&path_buf).map_err(|e| e.to_string())?;
        let albedo = set.albedo.ok_or("No albedo texture found in material folder")?;
        let out = if output_buf.is_dir() {
            output_buf.join("albedo.png")
        } else {
            output_buf
        };
        (albedo, out)
    } else {
        let img = pbr_core::ImageLoader::load(&path_buf).map_err(|e| e.to_string())?;
        let texture = pbr_core::material::TextureMap::from_loaded(img, Some(path_buf.clone()));
        let out = if output_buf.is_dir() {
            output_buf.join(
                path_buf
                    .file_name()
                    .unwrap_or(OsStr::new("albedo.png")),
            )
        } else {
            output_buf
        };
        (texture, out)
    };

    let blend = blend_width.unwrap_or(4);
    let (fixed, result) = fix_tileability_with_report(&texture, blend).map_err(|e| e.to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    save_texture(&fixed, &out_path).map_err(|e| e.to_string())?;

    let output_str = out_path
        .to_str()
        .map(String::from)
        .unwrap_or_else(|| output_path);
    Ok(FixTileabilityResult {
        output_path: output_str,
        original_edge_difference: result.original_edge_difference,
        fixed_edge_difference: result.fixed_edge_difference,
        improved: result.improved,
    })
}

#[tauri::command]
fn get_texture_paths(path: String) -> Result<String, String> {
    let set = MaterialSet::load_from_folder(&path).map_err(|e| e.to_string())?;
    let mut paths = serde_json::Map::new();

    if let Some(ref t) = set.albedo {
        if let Some(ref p) = t.path {
            paths.insert("albedo".into(), serde_json::Value::String(p.to_string_lossy().into_owned()));
        }
    }
    if let Some(ref t) = set.normal {
        if let Some(ref p) = t.path {
            paths.insert("normal".into(), serde_json::Value::String(p.to_string_lossy().into_owned()));
        }
    }
    if let Some(ref t) = set.roughness {
        if let Some(ref p) = t.path {
            paths.insert("roughness".into(), serde_json::Value::String(p.to_string_lossy().into_owned()));
        }
    }
    if let Some(ref t) = set.metallic {
        if let Some(ref p) = t.path {
            paths.insert("metallic".into(), serde_json::Value::String(p.to_string_lossy().into_owned()));
        }
    }
    if let Some(ref t) = set.ao {
        if let Some(ref p) = t.path {
            paths.insert("ao".into(), serde_json::Value::String(p.to_string_lossy().into_owned()));
        }
    }
    if let Some(ref t) = set.height {
        if let Some(ref p) = t.path {
            paths.insert("height".into(), serde_json::Value::String(p.to_string_lossy().into_owned()));
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
            analyze_folders,
            batch_export_preset,
            export_audit_log,
            export_preset,
            export_report,
            expand_material_paths,
            fix_tileability_texture,
            get_audit_log,
            get_material_folder_mtime,
            get_plugin_presets,
            get_texture_paths,
            list_plugins,
            resolve_material_folder,
            run_advanced_analysis_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

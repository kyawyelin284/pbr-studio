//! PBR texture set analyzer CLI

use clap::{Parser, Subcommand};
use pbr_core::{
    batch_export_with_preset, estimate_vram, export_with_lod, export_with_preset,
    fix_tileability_with_report, record_analysis, run_advanced_analysis,
    run_advanced_analysis_and_write,
    export_html_batch, export_html_single, export_pdf_batch, export_pdf_single,
    export_audit_log_text, load_audit_log, record_optimization as audit_record_optimization,
    save_audit_log_text,
    record_report as audit_record_report, record_validation as audit_record_validation,
    ai_analyze_json, ExportPreset, MaterialReport, MaterialSet, PluginInfo, PluginLoader, Validator,
};
use pbr_core::optimization::{save_texture, TargetResolution};
use pbr_core::validation::{Issue, Severity};
use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// CI/CD output format for automated pipelines
#[derive(Debug, Serialize)]
struct CiOutput {
    success: bool,
    min_score: i32,
    total_materials: usize,
    passed: usize,
    failed: usize,
    results: Vec<CiMaterialResult>,
}

#[derive(Debug, Serialize)]
struct CiMaterialResult {
    path: String,
    score: i32,
    passed: bool,
    critical_count: usize,
    major_count: usize,
    minor_count: usize,
    issues: Vec<CiIssue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    optimization_suggestions: Vec<CiOptimizationSuggestion>,
}

#[derive(Debug, Serialize)]
struct CiIssue {
    rule_id: String,
    severity: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct CiOptimizationSuggestion {
    category: String,
    message: String,
}

/// Batch JSON export entry: path + MaterialReport (matches report <folder> --json schema)
#[derive(Debug, Serialize)]
struct BatchJsonEntry {
    path: String,
    report: MaterialReport,
}

#[derive(Parser)]
#[command(name = "pbr-cli")]
#[command(about = "Offline PBR texture set analyzer. CI-integratable.")]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Load plugins from directory (or set PBR_STUDIO_PLUGINS)
    #[arg(long, global = true)]
    plugins_dir: Option<PathBuf>,

    /// Config file (TOML). Can set plugins_dir.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Debug, serde::Deserialize)]
struct CliConfig {
    plugins_dir: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run validation checks on a material folder
    Check {
        /// Path to the material folder
        folder: PathBuf,
        /// Minimum score to pass (0-100). Default 60
        #[arg(long, default_value = "60")]
        min_score: i32,
        /// Output structured JSON for CI/CD pipelines
        #[arg(long)]
        ci: bool,
        /// Load custom rules from plugins
        #[arg(long)]
        plugins: bool,
    },
    /// Recursively scan for material folders and print validation summary
    BatchCheck {
        /// Root folder to scan recursively
        #[arg(value_name = "ROOT-FOLDER")]
        root_folder: PathBuf,
        /// Minimum score to pass (0-100). Default 60
        #[arg(long, default_value = "60")]
        min_score: i32,
        /// Output structured JSON for CI/CD pipelines
        #[arg(long)]
        ci: bool,
        /// Write aggregated JSON report to file (local only, no network)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Load custom rules from plugins
        #[arg(long)]
        plugins: bool,
    },
    /// Validate materials affected by staged files (for Git pre-commit hooks)
    PreCommit {
        /// Minimum score to pass (0-100). Default 60
        #[arg(long, default_value = "60")]
        min_score: i32,
        /// Repository root (default: current directory or git root)
        #[arg(long)]
        root: Option<PathBuf>,
        /// Output structured JSON for CI/CD pipelines
        #[arg(long)]
        ci: bool,
        /// Load custom rules from plugins
        #[arg(long)]
        plugins: bool,
    },
    /// Export optimized textures for target engine
    Optimize {
        /// Path to the material folder
        folder: PathBuf,
        /// Output folder for optimized textures
        #[arg(short, long)]
        output: PathBuf,
        /// Target: 4k, unreal, unity, or mobile
        #[arg(long, default_value = "unreal")]
        target: String,
        /// Generate LOD chain (LOD0, LOD1, LOD2 subdirs)
        #[arg(long)]
        lod: bool,
    },
    /// Batch export all materials under root with preset
    BatchOptimize {
        /// Root folder containing material subfolders
        root_folder: PathBuf,
        /// Output root folder
        #[arg(short, long)]
        output: PathBuf,
        /// Target: 4k, unreal, unity, or mobile
        #[arg(long, default_value = "unreal")]
        target: String,
        /// Generate LOD chain for each material
        #[arg(long)]
        lod: bool,
    },
    /// Generate a report (text or JSON)
    Report {
        /// Path to the material folder
        folder: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Include VRAM estimate
        #[arg(long)]
        vram: bool,
        /// Export to file (json, html, or pdf)
        #[arg(long)]
        export: Option<String>,
        /// Output path for export (required with --export)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Export reports for one or more material folders
    ExportReport {
        /// Path(s) to material folder(s)
        #[arg(value_name = "FOLDER", num_args = 1..)]
        folders: Vec<PathBuf>,
        /// Output format: html, pdf, or json
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
        /// Write version changelog to .pbr-studio/versions.json
        #[arg(long)]
        track: bool,
    },
    /// Run advanced analysis (duplicates, cross-material, tileability)
    Analyze {
        /// Root folder containing material subfolders
        root_folder: PathBuf,
        /// Run tileability fix analysis (report only, use fix-tileability to apply)
        #[arg(long)]
        tileability: bool,
        /// Duplicate threshold (0–1). Default 0.99
        #[arg(long, default_value = "0.99")]
        duplicate_threshold: f32,
        /// Similar threshold (0–1). Default 0.80
        #[arg(long, default_value = "0.80")]
        similar_threshold: f32,
        /// Write JSON report to file (local only, no network)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show audit log (validation, optimization, report actions)
    AuditLog {
        /// Maximum number of entries to show. Default 50
        #[arg(long, default_value = "50")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Write to file (JSON or text based on --format)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format: json or text
        #[arg(long, default_value = "json", value_name = "FORMAT")]
        format: String,
    },
    /// List loaded plugins (rules and presets)
    PluginList {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// AI-assisted analysis (classification, optimization suggestions, anomaly detection)
    AiAnalyze {
        /// Path to the material folder
        folder: PathBuf,
        /// ONNX model path for ML classification (requires build with --features ai)
        #[arg(long)]
        model: Option<PathBuf>,
    },
    /// Apply tileability fix to albedo texture and save
    FixTileability {
        /// Path to material folder or texture file
        path: PathBuf,
        /// Output path (file or folder)
        #[arg(short, long)]
        output: PathBuf,
        /// Blend width in pixels. Default 4
        #[arg(long, default_value = "4")]
        blend_width: u32,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { folder, min_score, ci, plugins } => {
            let validator = build_validator(cli.plugins_dir.as_ref(), cli.config.as_ref(), plugins);
            cmd_check(&folder, min_score, ci, validator)
        }
        Commands::BatchCheck { root_folder, min_score, ci, plugins, output } => {
            let validator = build_validator(cli.plugins_dir.as_ref(), cli.config.as_ref(), plugins);
            cmd_batch_check(&root_folder, min_score, ci, output.as_ref().map(|p| p.as_path()), validator)
        }
        Commands::PreCommit { min_score, root, ci, plugins } => {
            let validator = build_validator(cli.plugins_dir.as_ref(), cli.config.as_ref(), plugins);
            cmd_pre_commit(min_score, root.as_deref(), ci, validator)
        }
        Commands::Optimize { folder, output, target, lod } => cmd_optimize(&folder, &output, &target, lod),
        Commands::BatchOptimize { root_folder, output, target, lod } => cmd_batch_optimize(&root_folder, &output, &target, lod),
        Commands::Report { folder, json, vram, export, output } => cmd_report(&folder, json, vram, export.as_deref(), output.as_ref()),
        Commands::ExportReport { folders, format, output, track } => cmd_export_report(&folders, &format, &output, track),
        Commands::Analyze {
            root_folder,
            tileability,
            duplicate_threshold,
            similar_threshold,
            output,
        } => cmd_analyze(&root_folder, tileability, duplicate_threshold, similar_threshold, output.as_deref()),
        Commands::FixTileability {
            path,
            output,
            blend_width,
        } => cmd_fix_tileability(&path, &output, blend_width),
        Commands::AuditLog { limit, json, output, format } => cmd_audit_log(limit, json, output.as_deref(), &format),
        Commands::PluginList { json } => cmd_plugin_list(&cli, json),
        Commands::AiAnalyze { folder, model } => cmd_ai_analyze(&folder, model.as_deref()),
    }
}

fn build_validator(
    plugins_dir: Option<&PathBuf>,
    config_path: Option<&PathBuf>,
    use_plugins: bool,
) -> Validator {
    if !use_plugins {
        return Validator::default();
    }
    let loader = build_plugin_loader(plugins_dir, config_path);
    Validator::default().with_plugins(&loader)
}

fn cmd_check(folder: &PathBuf, min_score: i32, ci: bool, validator: Validator) -> Result<(), Box<dyn std::error::Error>> {
    let set = MaterialSet::load_from_folder(folder)?;
    let issues = validator.check(&set);
    let score = pbr_core::validation::compute_score(&issues);
    let passed = score >= min_score;

    if ci {
        let output = CiOutput {
            success: passed,
            min_score,
            total_materials: 1,
            passed: if passed { 1 } else { 0 },
            failed: if passed { 0 } else { 1 },
            results: vec![to_ci_result(folder, &issues, score, min_score)],
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        for issue in &issues {
            let prefix = match issue.severity {
                Severity::Critical => "✗ [CRITICAL]",
                Severity::Major => "⚠ [MAJOR]",
                Severity::Minor => "ℹ [MINOR]",
            };
            println!("{} {}: {}", prefix, issue.rule_id, issue.message);
        }

        let critical = issues.iter().filter(|i| i.severity == Severity::Critical).count();
        let major = issues.iter().filter(|i| i.severity == Severity::Major).count();
        let minor = issues.iter().filter(|i| i.severity == Severity::Minor).count();
        println!("\nScore: {} (min: {})", score, min_score);
        println!("{} issue(s) ({} critical, {} major, {} minor)", issues.len(), critical, major, minor);
    }

    let critical = issues.iter().filter(|i| i.severity == Severity::Critical).count();
    let major = issues.iter().filter(|i| i.severity == Severity::Major).count();
    let _ = audit_record_validation(
        folder,
        score,
        passed,
        min_score,
        issues.len(),
        critical,
        major,
        None,
    );

    if !passed {
        std::process::exit(1);
    }
    Ok(())
}

fn to_ci_result(path: &Path, issues: &[Issue], score: i32, min_score: i32) -> CiMaterialResult {
    to_ci_result_with_suggestions(path, issues, score, min_score, &[])
}

fn to_ci_result_with_suggestions(
    path: &Path,
    issues: &[Issue],
    score: i32,
    min_score: i32,
    optimization_suggestions: &[pbr_core::OptimizationSuggestion],
) -> CiMaterialResult {
    let critical = issues.iter().filter(|i| i.severity == Severity::Critical).count();
    let major = issues.iter().filter(|i| i.severity == Severity::Major).count();
    let minor = issues.iter().filter(|i| i.severity == Severity::Minor).count();
    CiMaterialResult {
        path: path.display().to_string(),
        score,
        passed: score >= min_score,
        critical_count: critical,
        major_count: major,
        minor_count: minor,
        issues: issues.iter().map(|i| CiIssue {
            rule_id: i.rule_id.clone(),
            severity: format!("{:?}", i.severity).to_lowercase(),
            message: i.message.clone(),
        }).collect(),
        optimization_suggestions: optimization_suggestions.iter().map(|s| CiOptimizationSuggestion {
            category: s.category.clone(),
            message: s.message.clone(),
        }).collect(),
    }
}

fn cmd_optimize(
    folder: &PathBuf,
    output: &PathBuf,
    target: &str,
    lod: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let preset = match target.to_lowercase().as_str() {
        "4k" | "4k_high" => ExportPreset::Res4K,
        "unreal" | "unreal_engine" => ExportPreset::UnrealEngine,
        "unity" => ExportPreset::Unity,
        "mobile" | "mobile_optimized" => ExportPreset::MobileOptimized,
        _ => return Err(format!("Unknown target: {}. Use 4k, unreal, unity, or mobile.", target).into()),
    };

    let material = MaterialSet::load_from_folder(folder)?;
    let written = if lod {
        let levels = TargetResolution::default_lod_levels();
        export_with_lod(&material, output, preset, levels)?
    } else {
        export_with_preset(&material, output, preset)?
    };
    let _ = audit_record_optimization(folder, output, &target, written.len(), None);
    println!("Exported {} texture(s) to {}", written.len(), output.display());
    Ok(())
}

fn cmd_batch_optimize(
    root_folder: &PathBuf,
    output: &PathBuf,
    target: &str,
    lod: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let preset = match target.to_lowercase().as_str() {
        "4k" | "4k_high" => ExportPreset::Res4K,
        "unreal" | "unreal_engine" => ExportPreset::UnrealEngine,
        "unity" => ExportPreset::Unity,
        "mobile" | "mobile_optimized" => ExportPreset::MobileOptimized,
        _ => return Err(format!("Unknown target: {}. Use 4k, unreal, unity, or mobile.", target).into()),
    };

    let root = root_folder.canonicalize().unwrap_or_else(|_| root_folder.clone());
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()).into());
    }

    let mut material_folders = Vec::new();
    find_material_folders(&root, &root, &mut material_folders)?;

    if material_folders.is_empty() {
        return Err(format!("No material folders found under \"{}\"", root.display()).into());
    }

    let mut materials: Vec<(std::path::PathBuf, MaterialSet)> = Vec::new();
    for folder in &material_folders {
        match MaterialSet::load_from_folder(folder) {
            Ok(set) => materials.push((folder.clone(), set)),
            Err(e) => eprintln!("⚠ Skipping {}: {}", folder.display(), e),
        }
    }

    let written = if lod {
        let mut all = Vec::new();
        for (folder, material) in &materials {
            let name = material
                .name
                .clone()
                .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "material".to_string());
            let out_dir = output.join(&name);
            let levels = TargetResolution::default_lod_levels();
            all.extend(export_with_lod(material, &out_dir, preset, levels)?);
        }
        all
    } else {
        batch_export_with_preset(&materials, output, preset)?
    };

    let per_material = written.len() / materials.len().max(1);
    for (folder, material) in &materials {
        let name = material
            .name
            .clone()
            .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "material".to_string());
        let out_dir = output.join(&name);
        let _ = audit_record_optimization(folder, &out_dir, &target, per_material, None);
    }

    println!("Exported {} texture(s) from {} material(s) to {}", written.len(), materials.len(), output.display());
    Ok(())
}

fn cmd_batch_check(root: &PathBuf, min_score: i32, ci: bool, output_path: Option<&Path>, validator: Validator) -> Result<(), Box<dyn std::error::Error>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.clone());
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()).into());
    }

    let mut material_folders = Vec::new();
    find_material_folders(&root, &root, &mut material_folders)?;

    if material_folders.is_empty() {
        let output = CiOutput {
            success: true,
            min_score,
            total_materials: 0,
            passed: 0,
            failed: 0,
            results: vec![],
        };
        if let Some(p) = output_path {
            std::fs::write(p, serde_json::to_string_pretty(&output)?)?;
        }
        if ci {
            println!("{}", serde_json::to_string(&output)?);
        } else {
            println!("No material folders found under \"{}\"", root.display());
        }
        return Ok(());
    }

    let mut results: Vec<CiMaterialResult> = Vec::new();
    let mut failed_count = 0;

    for folder in &material_folders {
        let set = match MaterialSet::load_from_folder(folder) {
            Ok(s) => s,
            Err(e) => {
                if !ci {
                    eprintln!("⚠ Skipping {}: {}", folder.display(), e);
                }
                continue;
            }
        };

        let issues = validator.check(&set);
        let score = pbr_core::validation::compute_score(&issues);
        let passed = score >= min_score;
        if !passed {
            failed_count += 1;
        }

        let critical = issues.iter().filter(|i| i.severity == Severity::Critical).count();
        let major = issues.iter().filter(|i| i.severity == Severity::Major).count();
        let _ = audit_record_validation(
            folder,
            score,
            passed,
            min_score,
            issues.len(),
            critical,
            major,
            None,
        );

        let rel = folder.strip_prefix(&root).unwrap_or(folder);
        let report = MaterialReport::from_material_set(&set, issues.clone());
        let result = to_ci_result_with_suggestions(
            rel,
            &issues,
            score,
            min_score,
            &report.optimization_suggestions,
        );
        results.push(result);

        if !ci {
            if !passed || !issues.is_empty() {
                let status = if critical > 0 || !passed { "✗" } else { "⚠" };
                println!("{} {} (score: {}, {} critical, {} major)", status, rel.display(), score, critical, major);
                for issue in &issues {
                    let prefix = match issue.severity {
                        Severity::Critical => "    ✗",
                        Severity::Major => "    ⚠",
                        Severity::Minor => "    ℹ",
                    };
                    println!("{} {}: {}", prefix, issue.rule_id, issue.message);
                }
            }
        }
    }

    let passed_count = results.len() - failed_count;
    let output = CiOutput {
        success: failed_count == 0,
        min_score,
        total_materials: results.len(),
        passed: passed_count,
        failed: failed_count,
        results,
    };
    if let Some(p) = output_path {
        std::fs::write(p, serde_json::to_string_pretty(&output)?)?;
    }
    if ci {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        let total_critical: usize = output.results.iter().map(|r| r.critical_count).sum();
        let total_major: usize = output.results.iter().map(|r| r.major_count).sum();
        println!("\n--- Summary ---");
        println!("Scanned {} material folder(s)", material_folders.len());
        println!("{} folder(s) below threshold", failed_count);
        println!("{} total critical, {} total major", total_critical, total_major);
    }

    // Exit non-zero if any material score is below threshold
    if failed_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_pre_commit(min_score: i32, root: Option<&Path>, ci: bool, validator: Validator) -> Result<(), Box<dyn std::error::Error>> {
    let root = match root {
        Some(p) => p.canonicalize().unwrap_or_else(|_| p.to_path_buf()),
        None => {
            // Try git root, fallback to current dir
            let out = std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output();
            match out {
                Ok(o) if o.status.success() => {
                    let s = String::from_utf8_lossy(&o.stdout);
                    PathBuf::from(s.trim().trim_end_matches('\n'))
                }
                _ => std::env::current_dir()?,
            }
        }
    };

    let staged = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(&root)
        .output()?;

    if !staged.status.success() {
        return Err("Not a git repository or git command failed".into());
    }

    let paths: Vec<PathBuf> = String::from_utf8_lossy(&staged.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| root.join(l))
        .collect();

    // Collect unique material folders that contain staged files
    let mut material_folders: Vec<PathBuf> = Vec::new();
    for path in &paths {
        if let Some(parent) = path.parent() {
            if is_material_folder(parent) && !material_folders.iter().any(|f| f == parent) {
                material_folders.push(parent.to_path_buf());
            }
        }
    }

    if material_folders.is_empty() {
        if ci {
            let output = CiOutput {
                success: true,
                min_score,
                total_materials: 0,
                passed: 0,
                failed: 0,
                results: vec![],
            };
            println!("{}", serde_json::to_string(&output)?);
        } else {
            println!("No material folders with staged changes.");
        }
        return Ok(());
    }

    // Run batch validation on the affected folders only
    let mut results: Vec<CiMaterialResult> = Vec::new();
    let mut failed_count = 0;

    for folder in &material_folders {
        let set = match MaterialSet::load_from_folder(folder) {
            Ok(s) => s,
            Err(e) => {
                if !ci {
                    eprintln!("⚠ Skipping {}: {}", folder.display(), e);
                }
                continue;
            }
        };

        let issues = validator.check(&set);
        let score = pbr_core::validation::compute_score(&issues);
        let passed = score >= min_score;
        if !passed {
            failed_count += 1;
        }

        let critical = issues.iter().filter(|i| i.severity == Severity::Critical).count();
        let major = issues.iter().filter(|i| i.severity == Severity::Major).count();
        let _ = audit_record_validation(
            folder,
            score,
            passed,
            min_score,
            issues.len(),
            critical,
            major,
            None,
        );

        let rel = folder.strip_prefix(&root).unwrap_or(folder);
        let result = to_ci_result(rel, &issues, score, min_score);
        results.push(result);

        if !ci {
            let status = if critical > 0 || !passed { "✗" } else { "⚠" };
            println!("{} {} (score: {}, {} critical, {} major)", status, rel.display(), score, critical, major);
            for issue in &issues {
                let prefix = match issue.severity {
                    Severity::Critical => "    ✗",
                    Severity::Major => "    ⚠",
                    Severity::Minor => "    ℹ",
                };
                println!("{} {}: {}", prefix, issue.rule_id, issue.message);
            }
        }
    }

    if ci {
        let passed_count = results.len() - failed_count;
        let output = CiOutput {
            success: failed_count == 0,
            min_score,
            total_materials: results.len(),
            passed: passed_count,
            failed: failed_count,
            results,
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("\n--- Pre-commit ---");
        println!("Validated {} material folder(s) with staged changes", material_folders.len());
        println!("{} folder(s) below threshold (min: {})", failed_count, min_score);
    }

    if failed_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_analyze(
    root: &PathBuf,
    tileability: bool,
    duplicate_threshold: f32,
    similar_threshold: f32,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.clone());
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()).into());
    }

    let mut material_folders = Vec::new();
    find_material_folders(&root, &root, &mut material_folders)?;

    if material_folders.is_empty() {
        return Err(format!("No material folders found under \"{}\"", root.display()).into());
    }

    let mut materials: Vec<(PathBuf, MaterialSet)> = Vec::new();
    for folder in &material_folders {
        match MaterialSet::load_from_folder(folder) {
            Ok(set) => materials.push((folder.clone(), set)),
            Err(e) => eprintln!("⚠ Skipping {}: {}", folder.display(), e),
        }
    }

    if let Some(out) = output {
        run_advanced_analysis_and_write(&materials, out, duplicate_threshold, similar_threshold, None, tileability)?;
        println!("Wrote analysis to {}", out.display());
    } else {
        let report = run_advanced_analysis(&materials, duplicate_threshold, similar_threshold, tileability)?;
        println!("{}", report.to_json()?);
    }
    Ok(())
}

fn cmd_fix_tileability(
    path: &PathBuf,
    output: &PathBuf,
    blend_width: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.canonicalize().unwrap_or_else(|_| path.clone());

    let (texture, output_path) = if path.is_dir() {
        let set = MaterialSet::load_from_folder(&path)?;
        let albedo = set.albedo.ok_or("No albedo texture found in material folder")?;
        let out = if output.is_dir() {
            output.join("albedo.png")
        } else {
            output.clone()
        };
        (albedo, out)
    } else {
        let img = pbr_core::ImageLoader::load(&path)?;
        let texture = pbr_core::material::TextureMap::from_loaded(img, Some(path.clone()));
        let out = if output.is_dir() {
            output.join(path.file_name().unwrap_or(OsStr::new("albedo.png")))
        } else {
            output.clone()
        };
        (texture, out)
    };

    let (fixed, result) = fix_tileability_with_report(&texture, blend_width)?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    save_texture(&fixed, &output_path)?;
    println!("Fixed tileability: {} -> {}", result.path, output_path.display());
    println!("  Edge difference: {:.1} -> {:.1} (improved: {})",
        result.original_edge_difference, result.fixed_edge_difference, result.improved);
    Ok(())
}

fn find_material_folders(
    root: &Path,
    dir: &Path,
    results: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

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
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if EXTS.contains(&ext.as_str()) && SLOTS.iter().any(|s| stem.contains(s)) {
            return true;
        }
    }
    false
}

fn cmd_report(
    folder: &PathBuf,
    json: bool,
    vram: bool,
    export: Option<&str>,
    output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let set = MaterialSet::load_from_folder(folder)?;
    let validator = Validator::default();
    let issues = validator.check(&set);

    if let (Some(format), Some(out)) = (export, output) {
        let report = MaterialReport::from_material_set(&set, issues);
        match format.to_lowercase().as_str() {
            "html" => export_html_single(&report, out)?,
            "pdf" => export_pdf_single(&report, out)?,
            "json" => std::fs::write(out, report.to_json()?)?,
            _ => return Err(format!("Unknown format: {}. Use html, pdf, or json.", format).into()),
        }
        if let Err(e) = record_analysis(folder, report.score, report.passed, report.error_count, report.warning_count, report.issues.len()) {
            eprintln!("Warning: could not record version: {}", e);
        }
        let _ = audit_record_report(
            Some(folder.as_path()),
            format,
            out.as_path(),
            Some(report.score),
            Some(report.passed),
            None,
        );
        println!("Exported to {}", out.display());
        return Ok(());
    }

    if json {
        let report = MaterialReport::from_material_set(&set, issues);
        println!("{}", report.to_json()?);
    } else {
        let text_report = pbr_core::Report::from_material_set(&set, issues);
        println!("{}", text_report.to_text());
        if vram {
            let can_pack = set.roughness.is_some() && set.metallic.is_some() && set.ao.is_some();
            let est = estimate_vram(&set, true, can_pack);
            println!("\nVRAM estimate (mipmaps): {}", est.formatted);
        }
    }

    Ok(())
}

fn cmd_export_report(
    folders: &[PathBuf],
    format: &str,
    output: &PathBuf,
    track: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if folders.is_empty() {
        return Err("At least one folder required".into());
    }

    let validator = Validator::default();
    let mut reports: Vec<(String, MaterialReport)> = Vec::new();

    for folder in folders {
        let path_str = folder.display().to_string();
        let set = match MaterialSet::load_from_folder(folder) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: skipping {}: {}", path_str, e);
                continue;
            }
        };
        let issues = validator.check(&set);
        let report = MaterialReport::from_material_set(&set, issues);
        if track {
            if let Err(e) = record_analysis(folder, report.score, report.passed, report.error_count, report.warning_count, report.issues.len()) {
                eprintln!("Warning: could not record version for {}: {}", path_str, e);
            }
        }
        reports.push((path_str, report));
    }

    if reports.is_empty() {
        return Err("No valid material folders found".into());
    }

    match format.to_lowercase().as_str() {
        "html" => {
            if reports.len() == 1 {
                export_html_single(&reports[0].1, output)?;
            } else {
                export_html_batch(&reports, output)?;
            }
        }
        "pdf" => {
            if reports.len() == 1 {
                export_pdf_single(&reports[0].1, output)?;
            } else {
                export_pdf_batch(&reports, output)?;
            }
        }
        "json" => {
            // Batch JSON: array of { path, report } matching report <folder> --json schema
            let batch: Vec<BatchJsonEntry> = reports
                .iter()
                .map(|(path, report)| BatchJsonEntry {
                    path: path.clone(),
                    report: report.clone(),
                })
                .collect();
            let json = serde_json::to_string_pretty(&batch)
                .map_err(|e| format!("JSON serialization failed: {}", e))?;
            std::fs::write(output, json)?;
        }
        _ => return Err(format!("Unknown format: {}. Use html, pdf, or json.", format).into()),
    }

    for (path_str, report) in &reports {
        let path = Path::new(path_str);
        let _ = audit_record_report(
            Some(path),
            format,
            output,
            Some(report.score),
            Some(report.passed),
            None,
        );
    }

    println!("Exported {} material(s) to {}", reports.len(), output.display());
    Ok(())
}

fn cmd_plugin_list(cli: &Cli, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let loader = build_plugin_loader(cli.plugins_dir.as_ref(), cli.config.as_ref());
    let plugins: Vec<PluginInfo> = loader.list_loaded();

    if json {
        println!("{}", serde_json::to_string_pretty(&plugins)?);
    } else {
        if plugins.is_empty() {
            println!("No plugins loaded.");
            println!("Search paths: ./.pbr-studio/plugins, ~/.config/pbr-studio/plugins, PBR_STUDIO_PLUGINS");
            println!("Use --plugins-dir DIR or --config FILE to add custom paths.");
        } else {
            for p in &plugins {
                println!("{} {} @ {}", p.name, p.version, p.path.display());
                for id in &p.rule_ids {
                    println!("  rule: {}", id);
                }
                for id in &p.preset_ids {
                    println!("  preset: {}", id);
                }
            }
        }
    }
    Ok(())
}

fn cmd_ai_analyze(folder: &PathBuf, model: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    if model.is_some() && !pbr_core::AI_ONNX_ENABLED {
        eprintln!("Warning: --model ignored (build without --features ai). Using heuristics.");
    }
    let set = MaterialSet::load_from_folder(folder)?;
    let json = ai_analyze_json(&set, model).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn build_plugin_loader(
    plugins_dir: Option<&PathBuf>,
    config_path: Option<&PathBuf>,
) -> PluginLoader {
    let mut loader = PluginLoader::new().with_default_paths();
    if let Some(dir) = plugins_dir {
        loader = loader.add_dir(dir);
    }
    if let Some(config) = config_path {
        if let Ok(s) = std::fs::read_to_string(config) {
            if let Ok(cfg) = toml::from_str::<CliConfig>(&s) {
                if let Some(dir) = cfg.plugins_dir {
                    loader = loader.add_dir(&dir);
                }
            }
        }
    }
    loader
}

fn cmd_audit_log(
    limit: usize,
    json: bool,
    output: Option<&Path>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let log = load_audit_log(None)?;
    let entries: Vec<_> = log.entries.iter().take(limit).cloned().collect();
    let use_json = json || format.eq_ignore_ascii_case("json");

    if let Some(path) = output {
        if use_json {
            let content = serde_json::to_string_pretty(&entries)?;
            std::fs::write(path, content)?;
        } else {
            save_audit_log_text(path, &log, Some(limit))?;
        }
        println!("Audit log written to {}", path.display());
    } else {
        if use_json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        } else {
            println!("{}", export_audit_log_text(&log, Some(limit)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_material_dir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let mat1 = tmp.path().join("mat1");
        std::fs::create_dir_all(&mat1).unwrap();
        let img = image::RgbaImage::from_raw(4, 4, vec![128u8; 4 * 4 * 4]).unwrap();
        img.save(mat1.join("albedo.png")).unwrap();
        img.save(mat1.join("normal.png")).unwrap();
        img.save(mat1.join("roughness.png")).unwrap();
        (tmp, mat1)
    }

    #[test]
    fn export_report_format_json_batch() {
        let (tmp, mat1) = create_test_material_dir();
        let mat2_path = tmp.path().join("mat2");
        std::fs::create_dir_all(&mat2_path).unwrap();
        let img = image::RgbaImage::from_raw(4, 4, vec![64u8; 4 * 4 * 4]).unwrap();
        img.save(mat2_path.join("albedo.png")).unwrap();
        img.save(mat2_path.join("roughness.png")).unwrap();

        let out = tmp.path().join("batch-report.json");
        let folders = vec![mat1.clone(), mat2_path];
        let result = cmd_export_report(&folders, "json", &out, false);

        assert!(result.is_ok(), "export-report json failed: {:?}", result.err());
        assert!(out.exists(), "JSON file was not created");

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let arr = parsed.as_array().expect("expected JSON array");
        assert_eq!(arr.len(), 2, "expected 2 material reports");

        for (i, entry) in arr.iter().enumerate() {
            assert!(entry.get("path").is_some(), "entry {} missing path", i);
            let report = entry.get("report").expect("entry missing report");
            assert!(report.get("score").is_some(), "report missing score");
            assert!(report.get("summary").is_some(), "report missing summary");
            assert!(report.get("issues").is_some(), "report missing issues");
        }
    }
}

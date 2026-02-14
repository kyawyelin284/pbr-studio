//! Plugin system for custom validation rules and presets.
//!
//! Supports:
//! - JSON/TOML config-driven rules (no code changes)
//! - External script plugins (Python, Lua, etc.) via stdin/stdout
//! - Dynamic plugin discovery from config directories

use crate::material::MaterialSet;
use crate::validation::{Issue, Severity, ValidationRule};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Plugin manifest (plugin.json or plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin identifier
    pub name: String,
    /// Plugin version (e.g. "1.0.0")
    #[serde(default)]
    pub version: String,
    /// Custom validation rules
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
    /// Custom export presets
    #[serde(default)]
    pub presets: Vec<PresetConfig>,
}

/// Rule definition from config (JSON/TOML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    pub condition: RuleCondition,
}

fn default_severity() -> String {
    "major".to_string()
}

/// Rule condition types for config-driven validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleCondition {
    /// Require specific texture maps
    RequiredMaps { maps: Vec<String> },
    /// Max resolution (width or height must not exceed)
    MaxResolution { max_width: u32, max_height: u32 },
    /// Min resolution
    MinResolution { min_width: u32, min_height: u32 },
    /// Require power-of-two dimensions
    PowerOfTwo,
    /// Max texture count
    MaxTextureCount { max: usize },
    /// External script: receives material JSON on stdin, returns issues JSON on stdout
    Script {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

/// Custom export preset from config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConfig {
    pub id: String,
    pub name: String,
    /// Target resolution: 4k, 2k, 1k, 512, 256, 128
    pub target_resolution: String,
    #[serde(default)]
    pub include_lod: bool,
}

impl PresetConfig {
    /// Resolve target resolution string to max dimension (for export).
    pub fn max_dimension(&self) -> u32 {
        match self.target_resolution.to_lowercase().as_str() {
            "4k" | "4096" => 4096,
            "2k" | "2048" => 2048,
            "1k" | "1024" => 1024,
            "512" => 512,
            "256" => 256,
            "128" => 128,
            _ => 2048,
        }
    }
}

/// Material summary sent to external scripts (scripting API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSummaryForScript {
    pub path: Option<String>,
    pub name: Option<String>,
    pub texture_count: usize,
    pub dimensions: Option<DimensionsForScript>,
    pub maps: MapsForScript,
    pub dimensions_consistent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionsForScript {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapsForScript {
    pub albedo: bool,
    pub normal: bool,
    pub roughness: bool,
    pub metallic: bool,
    pub ao: bool,
    pub height: bool,
}

/// Response from external script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPluginResponse {
    #[serde(default)]
    pub issues: Vec<ScriptIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptIssue {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
}

/// Validation rule backed by config
#[derive(Debug, Clone)]
pub struct ConfigRule {
    pub config: RuleConfig,
}

impl ValidationRule for ConfigRule {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn description(&self) -> &str {
        if self.config.description.is_empty() {
            "Custom rule from plugin config"
        } else {
            &self.config.description
        }
    }

    fn check(&self, set: &MaterialSet) -> Option<Issue> {
        let severity = parse_severity(&self.config.severity).unwrap_or(Severity::Major);
        check_condition(&self.config.condition, set, &self.config.id, severity)
    }

    fn check_all(&self, set: &MaterialSet) -> Vec<Issue> {
        if let RuleCondition::Script { command, args } = &self.config.condition {
            return run_script_plugin_all(command, args, set, &self.config.id);
        }
        self.check(set).into_iter().collect()
    }
}

fn parse_severity(s: &str) -> Option<Severity> {
    match s.to_lowercase().as_str() {
        "critical" | "error" => Some(Severity::Critical),
        "major" | "warning" => Some(Severity::Major),
        "minor" | "info" => Some(Severity::Minor),
        _ => None,
    }
}

fn check_condition(
    cond: &RuleCondition,
    set: &MaterialSet,
    rule_id: &str,
    severity: Severity,
) -> Option<Issue> {
    match cond {
        RuleCondition::RequiredMaps { maps } => {
            let missing: Vec<_> = maps
                .iter()
                .filter(|m| !has_map(set, m))
                .cloned()
                .collect();
            if missing.is_empty() {
                None
            } else {
                Some(Issue::new(
                    rule_id,
                    severity,
                    format!("Missing required maps: {}", missing.join(", ")),
                ))
            }
        }
        RuleCondition::MaxResolution {
            max_width,
            max_height,
        } => {
            if let Some((w, h)) = set.dimensions() {
                if w > *max_width || h > *max_height {
                    return Some(Issue::new(
                        rule_id,
                        severity,
                        format!(
                            "Resolution {}x{} exceeds max {}x{}",
                            w, h, max_width, max_height
                        ),
                    ));
                }
            }
            None
        }
        RuleCondition::MinResolution {
            min_width,
            min_height,
        } => {
            if let Some((w, h)) = set.dimensions() {
                if w < *min_width || h < *min_height {
                    return Some(Issue::new(
                        rule_id,
                        severity,
                        format!(
                            "Resolution {}x{} below min {}x{}",
                            w, h, min_width, min_height
                        ),
                    ));
                }
            }
            None
        }
        RuleCondition::PowerOfTwo => {
            let bad: Vec<_> = [
                ("albedo", set.albedo.as_ref()),
                ("normal", set.normal.as_ref()),
                ("roughness", set.roughness.as_ref()),
                ("metallic", set.metallic.as_ref()),
                ("ao", set.ao.as_ref()),
                ("height", set.height.as_ref()),
            ]
            .into_iter()
            .filter_map(|(name, map)| {
                let m = map?;
                if !is_power_of_two(m.width) || !is_power_of_two(m.height) {
                    Some(format!("{} ({}x{})", name, m.width, m.height))
                } else {
                    None
                }
            })
            .collect();
            if bad.is_empty() {
                None
            } else {
                Some(Issue::new(
                    rule_id,
                    severity,
                    format!("Non-power-of-two: {}", bad.join(", ")),
                ))
            }
        }
        RuleCondition::MaxTextureCount { max } => {
            let count = set.texture_count();
            if count > *max {
                Some(Issue::new(
                    rule_id,
                    severity,
                    format!("Texture count {} exceeds max {}", count, max),
                ))
            } else {
                None
            }
        }
        RuleCondition::Script { command, args } => {
            run_script_plugin(command, args, set, rule_id)
        }
    }
}

fn has_map(set: &MaterialSet, slot: &str) -> bool {
    match slot.to_lowercase().as_str() {
        "albedo" | "basecolor" | "diffuse" | "color" => set.albedo.is_some(),
        "normal" | "norm" => set.normal.is_some(),
        "roughness" | "rough" => set.roughness.is_some(),
        "metallic" | "metal" => set.metallic.is_some(),
        "ao" | "ambientocclusion" | "ambient_occlusion" => set.ao.is_some(),
        "height" | "displacement" | "bump" => set.height.is_some(),
        _ => false,
    }
}

fn is_power_of_two(n: u32) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

fn run_script_plugin(
    command: &str,
    args: &[String],
    set: &MaterialSet,
    rule_id: &str,
) -> Option<Issue> {
    let issues = run_script_plugin_all(command, args, set, rule_id);
    issues.into_iter().next()
}

fn run_script_plugin_all(
    command: &str,
    args: &[String],
    set: &MaterialSet,
    rule_id: &str,
) -> Vec<Issue> {
    let summary = material_summary_for_script(set);
    let input_json = match serde_json::to_string(&summary) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return vec![Issue::new(
                rule_id,
                Severity::Minor,
                format!("Plugin script {} failed to run: {}", command, e),
            )];
        }
    };
    {
        let mut stdin = match child.stdin.take() {
            Some(s) => s,
            None => return vec![],
        };
        use std::io::Write;
        if stdin.write_all(input_json.as_bytes()).is_err() {
            return vec![];
        }
    }
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    if !output.status.success() {
        return vec![Issue::new(
            rule_id,
            Severity::Minor,
            format!("Plugin script {} failed (exit {})", command, output.status),
        )];
    }
    let out_str = String::from_utf8_lossy(&output.stdout);
    let response: ScriptPluginResponse = match serde_json::from_str(&out_str) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    response
        .issues
        .into_iter()
        .map(|si| {
            Issue::new(
                rule_id,
                parse_severity(&si.severity).unwrap_or(Severity::Major),
                si.message,
            )
        })
        .collect()
}

fn material_summary_for_script(set: &MaterialSet) -> MaterialSummaryForScript {
    let (dims, dims_opt) = match set.dimensions() {
        Some((w, h)) => (
            true,
            Some(DimensionsForScript {
                width: w,
                height: h,
            }),
        ),
        None => (false, None),
    };
    let dims_consistent = if dims { set.dimensions_consistent() } else { true };
    MaterialSummaryForScript {
        path: set.name.as_ref().map(|_| "".to_string()),
        name: set.name.clone(),
        texture_count: set.texture_count(),
        dimensions: dims_opt,
        maps: MapsForScript {
            albedo: set.albedo.is_some(),
            normal: set.normal.is_some(),
            roughness: set.roughness.is_some(),
            metallic: set.metallic.is_some(),
            ao: set.ao.is_some(),
            height: set.height.is_some(),
        },
        dimensions_consistent: dims_consistent,
    }
}

/// Plugin metadata for listing loaded plugins (CLI/UI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub rule_ids: Vec<String>,
    pub preset_ids: Vec<String>,
}

/// Plugin loader: discovers and loads plugins from directories
pub struct PluginLoader {
    plugin_dirs: Vec<PathBuf>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            plugin_dirs: Vec::new(),
        }
    }

    /// Add a directory to search for plugins
    pub fn add_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.plugin_dirs.push(path.as_ref().to_path_buf());
        self
    }

    /// Standard discovery paths: ./.pbr-studio, ~/.config/pbr-studio, PBR_STUDIO_PLUGINS
    pub fn with_default_paths(self) -> Self {
        let mut loader = self;

        if let Ok(cwd) = std::env::current_dir() {
            loader.plugin_dirs.push(cwd.join(".pbr-studio").join("plugins"));
        }
        if let Ok(home) = std::env::var("HOME") {
            let config = std::env::var("XDG_CONFIG_HOME")
                .unwrap_or_else(|_| format!("{}/.config", home));
            loader.plugin_dirs.push(PathBuf::from(config).join("pbr-studio").join("plugins"));
        }
        if let Ok(env_path) = std::env::var("PBR_STUDIO_PLUGINS") {
            for p in env_path.split(path_separator()) {
                let trimmed = p.trim();
                if !trimmed.is_empty() {
                    loader.plugin_dirs.push(PathBuf::from(trimmed));
                }
            }
        }
        loader
    }

    /// Load all manifests and return config rules + preset configs
    pub fn load(&self) -> (Vec<ConfigRule>, Vec<PresetConfig>) {
        let mut rules = Vec::new();
        let mut presets = Vec::new();

        for dir in &self.plugin_dirs {
            if !dir.is_dir() {
                continue;
            }
            // Load from subdirs (each plugin = one folder)
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some((r, p)) = load_manifest_from_dir(&path) {
                            rules.extend(r);
                            presets.extend(p);
                        }
                    }
                }
            }
            // Also load plugin.json/toml directly in dir
            if let Some((r, p)) = load_manifest_from_dir(dir) {
                rules.extend(r);
                presets.extend(p);
            }
        }
        (rules, presets)
    }

    /// List loaded plugins (metadata only). Uses same discovery as load().
    pub fn list_loaded(&self) -> Vec<PluginInfo> {
        let mut out = Vec::new();
        for dir in &self.plugin_dirs {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(info) = load_plugin_info_from_dir(&path) {
                            out.push(info);
                        }
                    }
                }
            }
            if let Some(info) = load_plugin_info_from_dir(dir) {
                out.push(info);
            }
        }
        out
    }
}

#[cfg(not(windows))]
fn path_separator() -> char {
    ':'
}

#[cfg(windows)]
fn path_separator() -> char {
    ';'
}

fn load_manifest_from_dir(dir: &Path) -> Option<(Vec<ConfigRule>, Vec<PresetConfig>)> {
    let manifest = read_manifest_from_dir(dir)?;
    let rules: Vec<ConfigRule> = manifest
        .rules
        .into_iter()
        .map(|c| ConfigRule { config: c })
        .collect();
    let presets = manifest.presets;
    Some((rules, presets))
}

fn read_manifest_from_dir(dir: &Path) -> Option<PluginManifest> {
    let json_path = dir.join("plugin.json");
    let toml_path = dir.join("plugin.toml");

    if json_path.exists() {
        let s = std::fs::read_to_string(&json_path).ok()?;
        serde_json::from_str::<PluginManifest>(&s).ok()
    } else if toml_path.exists() {
        let s = std::fs::read_to_string(&toml_path).ok()?;
        toml::from_str::<PluginManifest>(&s).ok()
    } else {
        None
    }
}

fn load_plugin_info_from_dir(dir: &Path) -> Option<PluginInfo> {
    let manifest = read_manifest_from_dir(dir)?;
    let rule_ids = manifest.rules.iter().map(|r| r.id.clone()).collect();
    let preset_ids = manifest.presets.iter().map(|p| p.id.clone()).collect();
    Some(PluginInfo {
        name: manifest.name,
        version: manifest.version,
        path: dir.to_path_buf(),
        rule_ids,
        preset_ids,
    })
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

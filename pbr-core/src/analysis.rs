//! Advanced analysis modules.
//!
//! Provides duplicate/similar texture detection, cross-material consistency
//! analysis, and automatic tileability fixes. All analyses are fully offline
//! and output structured JSON results.

use crate::material::{MaterialSet, TextureMap};
use crate::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Perceptual hash size for similarity comparison (8x8 = 64 values)
const PHASH_SIZE: u32 = 8;

/// Compute a simple perceptual hash: downsample to PHASH_SIZE x PHASH_SIZE grayscale,
/// return mean per block. Used for duplicate/similar detection.
fn perceptual_hash(map: &TextureMap) -> Vec<f32> {
    let w = map.width as usize;
    let h = map.height as usize;
    if w == 0 || h == 0 {
        return vec![];
    }

    let block_w = (w as f32 / PHASH_SIZE as f32).max(1.0);
    let block_h = (h as f32 / PHASH_SIZE as f32).max(1.0);
    let mut hash = Vec::with_capacity((PHASH_SIZE * PHASH_SIZE) as usize);

    for by in 0..PHASH_SIZE {
        for bx in 0..PHASH_SIZE {
            let x0 = (bx as f32 * block_w) as usize;
            let y0 = (by as f32 * block_h) as usize;
            let x1 = ((bx as f32 + 1.0) * block_w).min(w as f32) as usize;
            let y1 = ((by as f32 + 1.0) * block_h).min(h as f32) as usize;

            let mut sum = 0.0f64;
            let mut count = 0usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    if i + 2 < map.data.len() {
                        let g = 0.299 * map.data[i] as f64
                            + 0.587 * map.data[i + 1] as f64
                            + 0.114 * map.data[i + 2] as f64;
                        sum += g;
                        count += 1;
                    }
                }
            }
            let mean = if count > 0 { sum / count as f64 } else { 0.0 };
            hash.push(mean as f32);
        }
    }
    hash
}

/// Compute similarity (0.0 = different, 1.0 = identical) from perceptual hashes.
fn hash_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let sq_diff: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
    let max_diff = a.len() as f32 * 255.0 * 255.0;
    (1.0 - (sq_diff / max_diff).min(1.0)).max(0.0)
}

/// Texture descriptor for duplicate detection
#[derive(Debug, Clone)]
struct TextureRef {
    path: Option<PathBuf>,
    slot: String,
    material_name: Option<String>,
    hash: Vec<f32>,
}

fn collect_texture_refs(materials: &[(PathBuf, MaterialSet)]) -> Vec<TextureRef> {
    let mut refs = Vec::new();
    for (folder, set) in materials {
        let name = set.name.clone().or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()));
        for (opt, slot) in [
            (set.albedo.as_ref(), "albedo"),
            (set.normal.as_ref(), "normal"),
            (set.roughness.as_ref(), "roughness"),
            (set.metallic.as_ref(), "metallic"),
            (set.ao.as_ref(), "ao"),
            (set.height.as_ref(), "height"),
        ] {
            if let Some(t) = opt {
                refs.push(TextureRef {
                    path: t.path.clone(),
                    slot: slot.to_string(),
                    material_name: name.clone(),
                    hash: perceptual_hash(t),
                });
            }
        }
    }
    refs
}

// --- JSON output types ---

#[derive(Debug, Clone, Serialize)]
pub struct DuplicatePair {
    pub path_a: String,
    pub path_b: String,
    pub slot: String,
    pub material_a: Option<String>,
    pub material_b: Option<String>,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateAnalysisResult {
    pub duplicate_pairs: Vec<DuplicatePair>,
    pub similar_pairs: Vec<DuplicatePair>,
    pub duplicate_threshold: f32,
    pub similar_threshold: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolutionDistribution {
    pub width: u32,
    pub height: u32,
    pub count: usize,
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapCoverage {
    pub slot: String,
    pub present_count: usize,
    pub total_count: usize,
    pub coverage_percent: f32,
    pub missing_in: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossMaterialResult {
    pub material_count: usize,
    pub resolution_distributions: Vec<ResolutionDistribution>,
    pub resolution_inconsistent: bool,
    pub map_coverage: Vec<MapCoverage>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TileabilityFixResult {
    pub path: String,
    pub original_edge_difference: f32,
    pub fixed_edge_difference: f32,
    pub improved: bool,
}

/// Per-texture tileability analysis (which textures would benefit from edge blending).
#[derive(Debug, Clone, Serialize)]
pub struct TileabilityAnalysisEntry {
    pub path: String,
    pub slot: String,
    pub material_name: Option<String>,
    pub edge_difference: f32,
    pub needs_fix: bool,
}

/// Default edge-difference threshold above which a texture is considered non-tileable.
pub const TILEABILITY_THRESHOLD: f32 = 10.0;

/// Detect duplicate or highly similar textures within a set of materials.
/// Compares textures of the same slot (albedo to albedo, etc.) across materials.
pub fn detect_duplicates(
    materials: &[(PathBuf, MaterialSet)],
    duplicate_threshold: f32,
    similar_threshold: f32,
) -> DuplicateAnalysisResult {
    let refs = collect_texture_refs(materials);
    let mut duplicate_pairs = Vec::new();
    let mut similar_pairs = Vec::new();

    for i in 0..refs.len() {
        for j in (i + 1)..refs.len() {
            if refs[i].slot != refs[j].slot {
                continue;
            }
            let sim = hash_similarity(&refs[i].hash, &refs[j].hash);
            let path_a = refs[i].path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".into());
            let path_b = refs[j].path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".into());

            let pair = DuplicatePair {
                path_a: path_a.clone(),
                path_b: path_b.clone(),
                slot: refs[i].slot.clone(),
                material_a: refs[i].material_name.clone(),
                material_b: refs[j].material_name.clone(),
                similarity: sim,
            };

            if sim >= duplicate_threshold {
                duplicate_pairs.push(pair);
            } else if sim >= similar_threshold {
                similar_pairs.push(pair);
            }
        }
    }

    DuplicateAnalysisResult {
        duplicate_pairs,
        similar_pairs,
        duplicate_threshold,
        similar_threshold,
    }
}

/// Analyze consistency across multiple materials.
pub fn analyze_cross_material(materials: &[(PathBuf, MaterialSet)]) -> CrossMaterialResult {
    let mut resolution_groups: HashMap<(u32, u32), Vec<String>> = HashMap::new();
    let mut map_counts: HashMap<String, usize> = HashMap::new();
    let mut missing: HashMap<String, Vec<String>> = HashMap::new();

    let slots = ["albedo", "normal", "roughness", "metallic", "ao", "height"];

    for (folder, set) in materials {
        let name = set.name.clone().or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| folder.display().to_string());

        if let Some((w, h)) = set.dimensions() {
            resolution_groups.entry((w, h)).or_default().push(name.clone());
        }

        for slot in slots {
            let has = match slot {
                "albedo" => set.albedo.is_some(),
                "normal" => set.normal.is_some(),
                "roughness" => set.roughness.is_some(),
                "metallic" => set.metallic.is_some(),
                "ao" => set.ao.is_some(),
                "height" => set.height.is_some(),
                _ => false,
            };
            if has {
                *map_counts.entry(slot.to_string()).or_insert(0) += 1;
            } else {
                missing.entry(slot.to_string()).or_default().push(name.clone());
            }
        }
    }

    let total = materials.len();
    let resolution_distributions: Vec<ResolutionDistribution> = resolution_groups
        .into_iter()
        .map(|((w, h), mats)| ResolutionDistribution {
            width: w,
            height: h,
            count: mats.len(),
            materials: mats,
        })
        .collect();

    let resolution_inconsistent = resolution_distributions.len() > 1;

    let map_coverage: Vec<MapCoverage> = slots
        .iter()
        .map(|slot| {
            let present = map_counts.get(*slot).copied().unwrap_or(0);
            let missing_in = missing.get(*slot).cloned().unwrap_or_default();
            MapCoverage {
                slot: slot.to_string(),
                present_count: present,
                total_count: total,
                coverage_percent: if total > 0 { 100.0 * present as f32 / total as f32 } else { 0.0 },
                missing_in,
            }
        })
        .collect();

    let mut recommendations = Vec::new();
    if resolution_inconsistent {
        recommendations.push("Materials use different resolutions. Consider standardizing to a target (e.g. 2K) for consistency.".into());
    }
    for cov in &map_coverage {
        if cov.coverage_percent < 100.0 && cov.coverage_percent > 0.0 {
            recommendations.push(format!(
                "Map '{}' missing in {} material(s). Consider adding for consistency.",
                cov.slot, cov.missing_in.len()
            ));
        }
    }

    CrossMaterialResult {
        material_count: total,
        resolution_distributions,
        resolution_inconsistent,
        map_coverage,
        recommendations,
    }
}

/// Compute mean edge difference (top↔bottom, left↔right). Higher = less tileable.
pub fn edge_difference(map: &TextureMap) -> f64 {
    let w = map.width as usize;
    let h = map.height as usize;
    let mut sum = 0.0f64;
    let mut count = 0usize;

    for x in 0..w {
        let top = (0 * w + x) * 4;
        let bottom = ((h - 1) * w + x) * 4;
        if top + 3 < map.data.len() && bottom + 3 < map.data.len() {
            let d = (map.data[top] as i32 - map.data[bottom] as i32).abs()
                + (map.data[top + 1] as i32 - map.data[bottom + 1] as i32).abs()
                + (map.data[top + 2] as i32 - map.data[bottom + 2] as i32).abs();
            sum += d as f64;
            count += 1;
        }
    }
    for y in 0..h {
        let left = (y * w + 0) * 4;
        let right = (y * w + (w - 1)) * 4;
        if left + 3 < map.data.len() && right + 3 < map.data.len() {
            let d = (map.data[left] as i32 - map.data[right] as i32).abs()
                + (map.data[left + 1] as i32 - map.data[right + 1] as i32).abs()
                + (map.data[left + 2] as i32 - map.data[right + 2] as i32).abs();
            sum += d as f64;
            count += 1;
        }
    }

    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

/// Analyze which textures have high edge difference (would benefit from tileability fix).
pub fn analyze_tileability(
    materials: &[(PathBuf, MaterialSet)],
    threshold: f32,
) -> Vec<TileabilityAnalysisEntry> {
    let mut entries = Vec::new();
    for (folder, set) in materials {
        let name = set.name.clone().or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()));
        for (opt, slot) in [
            (set.albedo.as_ref(), "albedo"),
            (set.normal.as_ref(), "normal"),
            (set.roughness.as_ref(), "roughness"),
            (set.metallic.as_ref(), "metallic"),
            (set.ao.as_ref(), "ao"),
            (set.height.as_ref(), "height"),
        ] {
            if let Some(t) = opt {
                let ed = edge_difference(t);
                let path = t.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".into());
                entries.push(TileabilityAnalysisEntry {
                    path,
                    slot: slot.to_string(),
                    material_name: name.clone(),
                    edge_difference: ed as f32,
                    needs_fix: ed > threshold as f64,
                });
            }
        }
    }
    entries
}

/// Apply automatic tileability fix by blending opposite edges.
/// Blends top↔bottom and left↔right so opposite edges match for seamless tiling.
/// `blend_width` controls how many pixel rows/columns from each edge are blended.
pub fn fix_tileability(texture: &TextureMap, blend_width: u32) -> Result<TextureMap> {
    let w = texture.width as usize;
    let h = texture.height as usize;
    if w < 4 || h < 4 {
        return Ok(texture.clone());
    }

    let blend = blend_width.min((w.min(h) / 4) as u32).max(1) as usize;
    let mut data = texture.data.clone();

    // Top ↔ Bottom: set opposite rows to their average so they match when tiled
    for dy in 0..blend {
        for x in 0..w {
            for c in 0..4 {
                let top_i = (dy * w + x) * 4 + c;
                let bottom_i = ((h - 1 - dy) * w + x) * 4 + c;
                if top_i < data.len() && bottom_i < data.len() {
                    let avg = ((data[top_i] as f32 + data[bottom_i] as f32) / 2.0).round() as u8;
                    data[top_i] = avg;
                    data[bottom_i] = avg;
                }
            }
        }
    }

    // Left ↔ Right: set opposite columns to their average
    for dx in 0..blend {
        for y in 0..h {
            for c in 0..4 {
                let left_i = (y * w + dx) * 4 + c;
                let right_i = (y * w + (w - 1 - dx)) * 4 + c;
                if left_i < data.len() && right_i < data.len() {
                    let avg = ((data[left_i] as f32 + data[right_i] as f32) / 2.0).round() as u8;
                    data[left_i] = avg;
                    data[right_i] = avg;
                }
            }
        }
    }

    Ok(TextureMap {
        width: texture.width,
        height: texture.height,
        data,
        path: texture.path.clone(),
    })
}

/// Run tileability fix and return before/after metrics.
pub fn fix_tileability_with_report(
    texture: &TextureMap,
    blend_width: u32,
) -> Result<(TextureMap, TileabilityFixResult)> {
    let path = texture.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".into());
    let original_ed = edge_difference(texture);
    let fixed = fix_tileability(texture, blend_width)?;
    let fixed_ed = edge_difference(&fixed);

    let result = TileabilityFixResult {
        path,
        original_edge_difference: original_ed as f32,
        fixed_edge_difference: fixed_ed as f32,
        improved: fixed_ed < original_ed,
    };

    Ok((fixed, result))
}

/// Combined advanced analysis output for JSON export.
#[derive(Debug, Clone, Serialize)]
pub struct AdvancedAnalysisReport {
    pub duplicates: DuplicateAnalysisResult,
    pub cross_material: CrossMaterialResult,
    /// Textures that would benefit from tileability fix (edge difference above threshold).
    pub tileability_analysis: Vec<TileabilityAnalysisEntry>,
    /// Results from applying tileability fix (when run with fix_tileability_maps=true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tileability_fixes: Option<Vec<TileabilityFixResult>>,
}

impl AdvancedAnalysisReport {
    /// Serialize to formatted JSON string.
    pub fn to_json(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to compact JSON string.
    pub fn to_json_compact(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Write report to a local JSON file. Fully offline; no network.
    pub fn write_to_file(&self, path: &std::path::Path) -> crate::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialSet;
    use std::path::PathBuf;

    fn make_texture(w: u32, h: u32, value: u8) -> TextureMap {
        let len = (w as usize) * (h as usize) * 4;
        TextureMap {
            width: w,
            height: h,
            data: vec![value; len],
            path: None,
        }
    }

    #[test]
    fn detect_duplicate_identical() {
        let tex = make_texture(8, 8, 128);
        let mut set1 = MaterialSet::new();
        set1.albedo = Some(tex.clone());
        let mut set2 = MaterialSet::new();
        set2.albedo = Some(tex);

        let materials = vec![
            (PathBuf::from("mat1"), set1),
            (PathBuf::from("mat2"), set2),
        ];
        let result = detect_duplicates(&materials, 0.99, 0.8);
        assert_eq!(result.duplicate_pairs.len(), 1);
        assert!(result.duplicate_pairs[0].similarity >= 0.99);
    }

    #[test]
    fn fix_tileability_reduces_edge_difference() {
        let mut data = vec![0u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..16 {
                let i = (y * 16 + x) * 4;
                data[i] = (x * 16) as u8;
                data[i + 1] = (y * 16) as u8;
                data[i + 2] = 128;
                data[i + 3] = 255;
            }
        }
        let tex = TextureMap { width: 16, height: 16, data: data.clone(), path: None };
        let ed_before = edge_difference(&tex);
        let fixed = fix_tileability(&tex, 4).unwrap();
        let ed_after = edge_difference(&fixed);
        assert!(ed_after < ed_before || ed_before < 1.0);
    }

    #[test]
    fn run_advanced_analysis_produces_json() {
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture(64, 64, 128));
        set.normal = Some(make_texture(64, 64, 128));
        let materials = vec![(PathBuf::from("test"), set)];
        let report = run_advanced_analysis(&materials, 0.99, 0.8, false).unwrap();
        let json = report.to_json().unwrap();
        assert!(json.contains("duplicates"));
        assert!(json.contains("cross_material"));
    }
}

/// Run all advanced analyses and return a combined report.
pub fn run_advanced_analysis(
    materials: &[(PathBuf, MaterialSet)],
    duplicate_threshold: f32,
    similar_threshold: f32,
    fix_tileability_maps: bool,
) -> Result<AdvancedAnalysisReport> {
    run_advanced_analysis_with_tileability_threshold(
        materials,
        duplicate_threshold,
        similar_threshold,
        TILEABILITY_THRESHOLD,
        fix_tileability_maps,
    )
}

/// Run advanced analysis with configurable tileability threshold.
pub fn run_advanced_analysis_with_tileability_threshold(
    materials: &[(PathBuf, MaterialSet)],
    duplicate_threshold: f32,
    similar_threshold: f32,
    tileability_threshold: f32,
    fix_tileability_maps: bool,
) -> Result<AdvancedAnalysisReport> {
    let duplicates = detect_duplicates(materials, duplicate_threshold, similar_threshold);
    let cross_material = analyze_cross_material(materials);
    let tileability_analysis = analyze_tileability(materials, tileability_threshold);

    let mut tileability_fixes: Vec<TileabilityFixResult> = Vec::new();
    if fix_tileability_maps {
        for (_folder, set) in materials {
            if let Some(ref albedo) = set.albedo {
                if let Ok((_, result)) = fix_tileability_with_report(albedo, 4) {
                    if result.improved {
                        tileability_fixes.push(result);
                    }
                }
            }
        }
    }

    Ok(AdvancedAnalysisReport {
        duplicates,
        cross_material,
        tileability_analysis,
        tileability_fixes: if tileability_fixes.is_empty() {
            None
        } else {
            Some(tileability_fixes)
        },
    })
}

/// Run advanced analysis and write results to a local JSON file. Fully offline.
pub fn run_advanced_analysis_and_write(
    materials: &[(PathBuf, MaterialSet)],
    output_path: &std::path::Path,
    duplicate_threshold: f32,
    similar_threshold: f32,
    tileability_threshold: Option<f32>,
    fix_tileability_maps: bool,
) -> Result<AdvancedAnalysisReport> {
    let threshold = tileability_threshold.unwrap_or(TILEABILITY_THRESHOLD);
    let report = run_advanced_analysis_with_tileability_threshold(
        materials,
        duplicate_threshold,
        similar_threshold,
        threshold,
        fix_tileability_maps,
    )?;
    report.write_to_file(output_path)?;
    Ok(report)
}

//! Optional AI-assisted analysis (offline, local inference).
//!
//! When built with `--features ai`, supports ONNX model path for ML-based classification.
//!
//! Provides:
//! - Material classification (metal, wood, skin, fabric)
//! - Smart optimization suggestions (resolution vs quality)
//! - Anomaly detection for inconsistent textures
//!
//! Uses heuristic analysis by default. Enable `ai` feature and provide an ONNX
//! model path for ML-based classification.

use crate::material::{MaterialSet, TextureMap};
use serde::{Deserialize, Serialize};

/// True when built with `--features ai` (ONNX support)
pub const AI_ONNX_ENABLED: bool = cfg!(feature = "ai");

/// Material type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaterialClass {
    Metal,
    Wood,
    Skin,
    Fabric,
    Stone,
    Plastic,
    Unknown,
}

impl MaterialClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialClass::Metal => "metal",
            MaterialClass::Wood => "wood",
            MaterialClass::Skin => "skin",
            MaterialClass::Fabric => "fabric",
            MaterialClass::Stone => "stone",
            MaterialClass::Plastic => "plastic",
            MaterialClass::Unknown => "unknown",
        }
    }
}

/// Extracted texture features for analysis
#[derive(Debug, Clone)]
pub struct TextureFeatures {
    pub mean_r: f32,
    pub mean_g: f32,
    pub mean_b: f32,
    pub std_r: f32,
    pub std_g: f32,
    pub std_b: f32,
    pub variance: f32,
    pub edge_density: f32,
    pub saturation_mean: f32,
    pub warm_ratio: f32, // R/(R+G+B) for warm vs cool
}

/// AI-powered optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSuggestion {
    pub category: String,
    pub message: String,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_resolution: Option<String>,
}

/// Detected anomaly in texture set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub slot: String,
    pub message: String,
    pub score: f32,
}

/// AI insights block for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInsights {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_suggestions: Option<Vec<AiSuggestion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anomalies: Option<Vec<Anomaly>>,
}

/// Extract features from a texture for analysis
pub fn extract_features(tex: &TextureMap) -> TextureFeatures {
    let (w, h) = (tex.width as usize, tex.height as usize);
    let n = w * h;
    if n == 0 {
        return TextureFeatures {
            mean_r: 0.0, mean_g: 0.0, mean_b: 0.0,
            std_r: 0.0, std_g: 0.0, std_b: 0.0,
            variance: 0.0, edge_density: 0.0,
            saturation_mean: 0.0, warm_ratio: 0.33,
        };
    }

    let mut sum_r: f64 = 0.0;
    let mut sum_g: f64 = 0.0;
    let mut sum_b: f64 = 0.0;
    let mut sum_r2: f64 = 0.0;
    let mut sum_g2: f64 = 0.0;
    let mut sum_b2: f64 = 0.0;
    let mut sum_sat: f64 = 0.0;
    let mut sum_warm: f64 = 0.0;
    let mut edge_count: u32 = 0;

    let data = &tex.data;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            if i + 4 > data.len() {
                break;
            }
            let r = data[i] as f64;
            let g = data[i + 1] as f64;
            let b = data[i + 2] as f64;
            sum_r += r;
            sum_g += g;
            sum_b += b;
            sum_r2 += r * r;
            sum_g2 += g * g;
            sum_b2 += b * b;
            let maxc = r.max(g).max(b);
            let minc = r.min(g).min(b);
            let sat = if maxc > 0.0 { (maxc - minc) / maxc } else { 0.0 };
            sum_sat += sat;
            let total = r + g + b;
            sum_warm += if total > 0.0 { r / total } else { 0.33 };
        }
    }

    let nf = n as f64;
    let mean_r = (sum_r / nf) as f32;
    let mean_g = (sum_g / nf) as f32;
    let mean_b = (sum_b / nf) as f32;
    let std_r = ((sum_r2 / nf - (sum_r / nf).powi(2)).max(0.0).sqrt()) as f32;
    let std_g = ((sum_g2 / nf - (sum_g / nf).powi(2)).max(0.0).sqrt()) as f32;
    let std_b = ((sum_b2 / nf - (sum_b / nf).powi(2)).max(0.0).sqrt()) as f32;
    let variance = (std_r * std_r + std_g * std_g + std_b * std_b) / 3.0;
    let saturation_mean = (sum_sat / nf) as f32;
    let warm_ratio = (sum_warm / nf) as f32;

    // Simple edge detection: count pixels where neighbor difference > threshold
    let threshold = 30.0;
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = (y * w + x) * 4;
            let c = data[i] as f32 + data[i + 1] as f32 + data[i + 2] as f32;
            let right = (data[i + 4] as f32 + data[i + 5] as f32 + data[i + 6] as f32).abs();
            let down = (data[(y + 1) * w * 4 + x * 4] as f32
                + data[(y + 1) * w * 4 + x * 4 + 1] as f32
                + data[(y + 1) * w * 4 + x * 4 + 2] as f32)
                .abs();
            if (c - right).abs() > threshold || (c - down).abs() > threshold {
                edge_count += 1;
            }
        }
    }
    let edge_density = edge_count as f32 / n as f32;

    TextureFeatures {
        mean_r,
        mean_g,
        mean_b,
        std_r,
        std_g,
        std_b,
        variance,
        edge_density,
        saturation_mean,
        warm_ratio,
    }
}

/// Classify material from albedo texture using heuristics (fully offline)
pub fn classify_material(set: &MaterialSet, _onnx_path: Option<&std::path::Path>) -> (MaterialClass, f32) {
    let albedo = match &set.albedo {
        Some(a) => a,
        None => return (MaterialClass::Unknown, 0.0),
    };

    #[cfg(feature = "ai")]
    if let Some(path) = _onnx_path {
        if let Ok((class, conf)) = classify_with_onnx(albedo, path) {
            return (class, conf);
        }
    }

    let f = extract_features(albedo);

    // Heuristic rules (tuned for common PBR textures)
    // Metal: often desaturated, high contrast, metallic map present
    let has_metallic = set.metallic.is_some();
    if has_metallic && f.saturation_mean < 0.15 && f.variance > 200.0 {
        return (MaterialClass::Metal, 0.65);
    }
    if f.saturation_mean < 0.2 && f.std_r + f.std_g + f.std_b > 80.0 {
        return (MaterialClass::Metal, 0.5);
    }

    // Wood: warm tones, moderate variance, grain (higher edge density)
    if f.warm_ratio > 0.38 && f.warm_ratio < 0.5 && f.edge_density > 0.02 && f.edge_density < 0.08 {
        return (MaterialClass::Wood, 0.6);
    }
    if f.warm_ratio > 0.36 && f.variance < 1500.0 && f.variance > 200.0 {
        return (MaterialClass::Wood, 0.45);
    }

    // Skin: warm, low saturation, low variance, soft
    if f.warm_ratio > 0.38 && f.saturation_mean < 0.25 && f.variance < 500.0 {
        return (MaterialClass::Skin, 0.6);
    }

    // Fabric: can have patterns, moderate saturation
    if f.edge_density > 0.03 && f.saturation_mean > 0.2 && f.saturation_mean < 0.6 {
        return (MaterialClass::Fabric, 0.5);
    }

    // Stone: often cool, medium variance
    if f.warm_ratio < 0.34 && f.variance > 100.0 && f.variance < 2000.0 {
        return (MaterialClass::Stone, 0.45);
    }

    // Plastic: high saturation, uniform
    if f.saturation_mean > 0.4 && f.variance < 300.0 {
        return (MaterialClass::Plastic, 0.5);
    }

    (MaterialClass::Unknown, 0.3)
}

#[cfg(feature = "ai")]
fn classify_with_onnx(tex: &TextureMap, path: &std::path::Path) -> Result<(MaterialClass, f32), crate::Error> {
    use tract_onnx::prelude::*;

    let model = tract_onnx::onnx()
        .model_for_path(path)
        .map_err(|e| crate::Error::Other(format!("Failed to load ONNX model: {}", e)))?
        .into_optimized()
        .map_err(|e| crate::Error::Other(format!("Failed to optimize model: {}", e)))?
        .into_runnable()
        .map_err(|e| crate::Error::Other(format!("Failed to build runnable model: {}", e)))?;

    // Default ImageNet-style input size; model may override
    let (in_h, in_w) = (224, 224);
    let data = &tex.data;
    let (w, h) = (tex.width as usize, tex.height as usize);
    if w == 0 || h == 0 || data.len() < w * h * 4 {
        return Err(crate::Error::Other("Invalid texture dimensions".into()));
    }

    // Resize and normalize: create RGB, resize to in_h x in_w, normalize with ImageNet stats
    let mut resized = vec![0u8; in_h * in_w * 3];
    for y in 0..in_h {
        for x in 0..in_w {
            let src_x = (x * w) / in_w;
            let src_y = (y * h) / in_h;
            let i = (src_y * w + src_x) * 4;
            let o = (y * in_w + x) * 3;
            resized[o] = data[i];
            resized[o + 1] = data[i + 1];
            resized[o + 2] = data[i + 2];
        }
    }

    let mean = [0.485f32, 0.456, 0.406];
    let std = [0.229f32, 0.224, 0.225];
    let input: Tensor = tract_onnx::prelude::tract_ndarray::Array4::from_shape_fn((1, 3, in_h, in_w), |(_, c, y, x)| {
        let p = resized[(y * in_w + x) * 3 + c] as f32 / 255.0;
        (p - mean[c]) / std[c]
    })
    .into();

    let result = model
        .run(tvec!(input.into()))
        .map_err(|e| crate::Error::Other(format!("ONNX inference failed: {}", e)))?;
    let logits = result[0]
        .to_array_view::<f32>()
        .map_err(|e| crate::Error::Other(format!("Invalid model output: {}", e)))?;

    let (idx, &max_val) = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .ok_or_else(|| crate::Error::Other("Empty model output".into()))?;

    let class = match idx {
        0 => MaterialClass::Metal,
        1 => MaterialClass::Wood,
        2 => MaterialClass::Skin,
        3 => MaterialClass::Fabric,
        4 => MaterialClass::Stone,
        5 => MaterialClass::Plastic,
        _ => MaterialClass::Unknown,
    };
    let confidence = (max_val.exp() / (logits.iter().map(|x| x.exp()).sum::<f32>() + 1e-8)).min(1.0).max(0.0);
    Ok((class, confidence))
}

/// Generate smart optimization suggestions based on texture analysis
pub fn suggest_optimizations(set: &MaterialSet) -> Vec<AiSuggestion> {
    let mut suggestions = Vec::new();

    let albedo = match &set.albedo {
        Some(a) => a,
        None => return suggestions,
    };

    let f = extract_features(albedo);
    let (w, h) = (albedo.width, albedo.height);
    let max_dim = w.max(h) as f32;

    // Low complexity → safe to downscale
    if f.variance < 300.0 && f.edge_density < 0.02 {
        let target = if max_dim > 2048.0 { "2K" } else { "1K" };
        suggestions.push(AiSuggestion {
            category: "resolution".to_string(),
            message: format!(
                "Low visual complexity detected. Consider downscaling to {} without noticeable quality loss.",
                target
            ),
            confidence: 0.75,
            target_resolution: Some(target.to_string()),
        });
    }

    // High res + low complexity
    if max_dim > 4096.0 && f.variance < 800.0 {
        suggestions.push(AiSuggestion {
            category: "resolution".to_string(),
            message: "Texture exceeds 4K with moderate complexity. 4K or 2K may suffice for most use cases.".to_string(),
            confidence: 0.7,
            target_resolution: Some("4K".to_string()),
        });
    }

    // Uniform albedo + no height → height map may not add much
    if set.height.is_none() && f.variance < 200.0 && set.albedo.is_some() {
        suggestions.push(AiSuggestion {
            category: "workflow".to_string(),
            message: "Flat albedo texture. Height map may have limited impact.".to_string(),
            confidence: 0.6,
            target_resolution: None,
        });
    }

    suggestions
}

/// Detect anomalies (inconsistent textures) within a material set
pub fn detect_anomalies(set: &MaterialSet) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    let textures: Vec<(&str, &TextureMap)> = [
        ("albedo", set.albedo.as_ref()),
        ("normal", set.normal.as_ref()),
        ("roughness", set.roughness.as_ref()),
        ("metallic", set.metallic.as_ref()),
        ("ao", set.ao.as_ref()),
        ("height", set.height.as_ref()),
    ]
    .into_iter()
    .filter_map(|(name, opt)| opt.map(|t| (name, t)))
    .collect();

    if textures.len() < 2 {
        return anomalies;
    }

    // Check dimension consistency
    let dims: Vec<(u32, u32)> = textures.iter().map(|(_, t)| (t.width, t.height)).collect();
    let (ref_w, ref_h) = dims[0];
    for (i, ((name, _), (w, h))) in textures.iter().zip(dims.iter()).enumerate() {
        if i > 0 && (*w != ref_w || *h != ref_h) {
            let ratio = (*w as f32 / ref_w as f32).max(*h as f32 / ref_h as f32);
            if ratio > 1.5 || ratio < 0.67 {
                anomalies.push(Anomaly {
                    slot: (*name).to_string(),
                    message: format!(
                        "Resolution {}x{} differs significantly from reference {}x{}",
                        w, h, ref_w, ref_h
                    ),
                    score: 0.8,
                });
            }
        }
    }

    // Check albedo vs other maps for outlier statistics
    if let Some(albedo) = &set.albedo {
        let albedo_f = extract_features(albedo);
        for (name, tex) in &textures {
            if *name == "albedo" {
                continue;
            }
            let f = extract_features(tex);
            // Roughness/metallic often have different characteristics - skip strict comparison
            if *name == "normal" {
                let albedo_bright = (albedo_f.mean_r + albedo_f.mean_g + albedo_f.mean_b) / 3.0;
                let norm_bright = (f.mean_r + f.mean_g + f.mean_b) / 3.0;
                if (albedo_bright - norm_bright).abs() > 100.0 && f.variance < 50.0 {
                    anomalies.push(Anomaly {
                        slot: (*name).to_string(),
                        message: "Normal map appears unusually flat or uniform".to_string(),
                        score: 0.6,
                    });
                }
            }
        }
    }

    anomalies
}

/// Run full AI analysis and return insights for report integration
pub fn analyze_material(set: &MaterialSet, onnx_path: Option<&std::path::Path>) -> AiInsights {
    let (classification, conf) = classify_material(set, onnx_path);
    let suggestions = suggest_optimizations(set);
    let anomalies = detect_anomalies(set);

    AiInsights {
        classification: Some(classification.as_str().to_string()),
        classification_confidence: Some(conf),
        smart_suggestions: if suggestions.is_empty() {
            None
        } else {
            Some(suggestions)
        },
        anomalies: if anomalies.is_empty() {
            None
        } else {
            Some(anomalies)
        },
    }
}

/// Run AI analysis and return JSON string (offline, no cloud). For CLI/UI integration.
pub fn ai_analyze_json(set: &MaterialSet, onnx_path: Option<&std::path::Path>) -> Result<String, serde_json::Error> {
    let insights = analyze_material(set, onnx_path);
    serde_json::to_string_pretty(&insights)
}

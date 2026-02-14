//! GPU/CPU estimation for PBR texture sets.
//!
//! Estimates VRAM usage for material sets. Assumes uncompressed RGBA8
//! format for GPU upload; mipmap overhead (~33%) is optional.

use crate::material::{MaterialSet, TextureMap};
use serde::{Deserialize, Serialize};

/// Bytes per pixel for RGBA8 (uncompressed)
const BYTES_PER_PIXEL_RGBA8: u64 = 4;

/// Mipmap chain adds ~33% to base texture size
const MIPMAP_OVERHEAD: f64 = 4.0 / 3.0;

/// VRAM/CPU usage estimate for a material set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramEstimate {
    /// Total estimated bytes
    pub bytes: u64,
    /// Human-readable size (e.g. "12.5 MB")
    pub formatted: String,
    /// Whether mipmaps were included
    pub include_mipmaps: bool,
    /// Whether RMA packing was assumed (reduces textures)
    pub packed_orm: bool,
    /// Per-texture breakdown
    pub textures: Vec<TextureVramEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureVramEntry {
    pub slot: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

/// Estimate VRAM for a single texture (uncompressed RGBA8)
fn estimate_texture_bytes(width: u32, height: u32, include_mipmaps: bool) -> u64 {
    let base = (width as u64) * (height as u64) * BYTES_PER_PIXEL_RGBA8;
    if include_mipmaps {
        (base as f64 * MIPMAP_OVERHEAD).round() as u64
    } else {
        base
    }
}

/// Estimate VRAM for a material set.
/// If `packed_orm` is true, roughness/metallic/ao are counted as one ORM texture.
fn add_texture(
    textures: &mut Vec<TextureVramEntry>,
    total: &mut u64,
    slot: &str,
    opt: Option<&TextureMap>,
    include_mipmaps: bool,
) {
    if let Some(t) = opt {
        let bytes = estimate_texture_bytes(t.width, t.height, include_mipmaps);
        *total += bytes;
        textures.push(TextureVramEntry {
            slot: slot.to_string(),
            width: t.width,
            height: t.height,
            bytes,
        });
    }
}

pub fn estimate_vram(
    material: &MaterialSet,
    include_mipmaps: bool,
    packed_orm: bool,
) -> VramEstimate {
    let mut textures = Vec::new();
    let mut total: u64 = 0;

    add_texture(&mut textures, &mut total, "albedo", material.albedo.as_ref(), include_mipmaps);
    add_texture(&mut textures, &mut total, "normal", material.normal.as_ref(), include_mipmaps);

    if packed_orm && material.roughness.is_some() && material.metallic.is_some() && material.ao.is_some() {
        let r = material.roughness.as_ref().unwrap();
        let bytes = estimate_texture_bytes(r.width, r.height, include_mipmaps);
        total += bytes;
        textures.push(TextureVramEntry {
            slot: "orm".to_string(),
            width: r.width,
            height: r.height,
            bytes,
        });
    } else {
        add_texture(&mut textures, &mut total, "roughness", material.roughness.as_ref(), include_mipmaps);
        add_texture(&mut textures, &mut total, "metallic", material.metallic.as_ref(), include_mipmaps);
        add_texture(&mut textures, &mut total, "ao", material.ao.as_ref(), include_mipmaps);
    }

    add_texture(&mut textures, &mut total, "height", material.height.as_ref(), include_mipmaps);

    let formatted = format_bytes(total);

    VramEstimate {
        bytes: total,
        formatted,
        include_mipmaps,
        packed_orm,
        textures,
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_texture(w: u32, h: u32) -> TextureMap {
        TextureMap {
            width: w,
            height: h,
            data: vec![0; (w as usize) * (h as usize) * 4],
            path: None,
        }
    }

    #[test]
    fn estimate_vram_single_texture() {
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture(1024, 1024));
        let est = estimate_vram(&set, false, false);
        assert_eq!(est.bytes, 1024 * 1024 * 4);
        assert!(est.formatted.contains("MB"));
    }

    #[test]
    fn estimate_vram_with_mipmaps() {
        let mut set = MaterialSet::new();
        set.albedo = Some(make_texture(1024, 1024));
        let est = estimate_vram(&set, true, false);
        assert!(est.bytes > 1024 * 1024 * 4);
    }
}

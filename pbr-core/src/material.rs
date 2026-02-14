//! Material and texture set analysis.
//!
//! Analyzes PBR texture sets for consistency, completeness,
//! and physical correctness.

use crate::image_loading::{ImageLoader, LoadedImage, TextureSlot};
use crate::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Supported image extensions for folder scanning
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "tga", "exr"];

/// A texture map with resolution and pixel data
#[derive(Debug, Clone)]
pub struct TextureMap {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// RGBA pixel data (4 bytes per pixel, row-major)
    pub data: Vec<u8>,
    /// Source path when loaded from file
    pub path: Option<PathBuf>,
}

impl TextureMap {
    pub fn from_loaded(image: LoadedImage, path: Option<PathBuf>) -> Self {
        Self {
            width: image.width,
            height: image.height,
            data: image.data,
            path,
        }
    }

    /// Get pixel at (x, y) as [R, G, B, A]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        if i + 4 > self.data.len() {
            return None;
        }
        Some([
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ])
    }
}

/// A material set with optional PBR texture maps.
/// Each map stores resolution and pixel data when present.
#[derive(Debug, Clone, Default)]
pub struct MaterialSet {
    pub albedo: Option<TextureMap>,
    pub normal: Option<TextureMap>,
    pub roughness: Option<TextureMap>,
    pub metallic: Option<TextureMap>,
    pub ao: Option<TextureMap>,
    pub height: Option<TextureMap>,
    /// Optional name (e.g., folder name)
    pub name: Option<String>,
}

impl MaterialSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_albedo(&mut self, map: TextureMap) {
        self.albedo = Some(map);
    }
    pub fn add_normal(&mut self, map: TextureMap) {
        self.normal = Some(map);
    }
    pub fn add_roughness(&mut self, map: TextureMap) {
        self.roughness = Some(map);
    }
    pub fn add_metallic(&mut self, map: TextureMap) {
        self.metallic = Some(map);
    }
    pub fn add_ao(&mut self, map: TextureMap) {
        self.ao = Some(map);
    }
    pub fn add_height(&mut self, map: TextureMap) {
        self.height = Some(map);
    }

    pub fn has_albedo(&self) -> bool {
        self.albedo.is_some()
    }
    pub fn has_normal(&self) -> bool {
        self.normal.is_some()
    }
    pub fn has_roughness(&self) -> bool {
        self.roughness.is_some()
    }
    pub fn has_metallic(&self) -> bool {
        self.metallic.is_some()
    }
    pub fn has_ao(&self) -> bool {
        self.ao.is_some()
    }
    pub fn has_height(&self) -> bool {
        self.height.is_some()
    }

    pub fn get(&self, slot: TextureSlot) -> Option<&TextureMap> {
        match slot {
            TextureSlot::Albedo => self.albedo.as_ref(),
            TextureSlot::Normal => self.normal.as_ref(),
            TextureSlot::Roughness => self.roughness.as_ref(),
            TextureSlot::Metallic => self.metallic.as_ref(),
            TextureSlot::AmbientOcclusion => self.ao.as_ref(),
            TextureSlot::Height => self.height.as_ref(),
            _ => None,
        }
    }

    pub fn texture_count(&self) -> usize {
        [
            self.albedo.as_ref(),
            self.normal.as_ref(),
            self.roughness.as_ref(),
            self.metallic.as_ref(),
            self.ao.as_ref(),
            self.height.as_ref(),
        ]
        .into_iter()
        .filter(Option::is_some)
        .count()
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        [
            self.albedo.as_ref(),
            self.normal.as_ref(),
            self.roughness.as_ref(),
            self.metallic.as_ref(),
            self.ao.as_ref(),
            self.height.as_ref(),
        ]
        .into_iter()
        .find_map(|m| m.map(|t| (t.width, t.height)))
    }

    pub fn dimensions_consistent(&self) -> bool {
        let Some((w, h)) = self.dimensions() else {
            return true;
        };
        [
            self.albedo.as_ref(),
            self.normal.as_ref(),
            self.roughness.as_ref(),
            self.metallic.as_ref(),
            self.ao.as_ref(),
            self.height.as_ref(),
        ]
        .into_iter()
        .filter_map(|m| m)
        .all(|t| t.width == w && t.height == h)
    }

    /// Load a material set from a folder by scanning for image files
    /// and detecting PBR map type from filenames (albedo, basecolor, normal, etc.).
    pub fn load_from_folder<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let folder_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from);

        let mut set = MaterialSet {
            name: folder_name,
            ..Default::default()
        };

        let entries = std::fs::read_dir(path)?;
        let mut candidates: Vec<(PathBuf, TextureSlot)> = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());

            let Some(ext) = ext else {
                continue;
            };

            if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            let Some(slot) = ImageLoader::detect_slot_from_path(&path) else {
                continue;
            };

            // Only include slots we care about
            if matches!(
                slot,
                TextureSlot::Albedo | TextureSlot::Normal | TextureSlot::Roughness
                    | TextureSlot::Metallic | TextureSlot::AmbientOcclusion | TextureSlot::Height
            ) {
                candidates.push((path, slot));
            }
        }

        // Sort for deterministic ordering (first match wins per slot)
        candidates.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));

        for (file_path, slot) in candidates {
            match slot {
                TextureSlot::Albedo if set.albedo.is_none() => {
                    let img = ImageLoader::load(&file_path)?;
                    set.albedo = Some(TextureMap::from_loaded(img, Some(file_path)));
                }
                TextureSlot::Normal if set.normal.is_none() => {
                    let img = ImageLoader::load(&file_path)?;
                    set.normal = Some(TextureMap::from_loaded(img, Some(file_path)));
                }
                TextureSlot::Roughness if set.roughness.is_none() => {
                    let img = ImageLoader::load(&file_path)?;
                    set.roughness = Some(TextureMap::from_loaded(img, Some(file_path)));
                }
                TextureSlot::Metallic if set.metallic.is_none() => {
                    let img = ImageLoader::load(&file_path)?;
                    set.metallic = Some(TextureMap::from_loaded(img, Some(file_path)));
                }
                TextureSlot::AmbientOcclusion if set.ao.is_none() => {
                    let img = ImageLoader::load(&file_path)?;
                    set.ao = Some(TextureMap::from_loaded(img, Some(file_path)));
                }
                TextureSlot::Height if set.height.is_none() => {
                    let img = ImageLoader::load(&file_path)?;
                    set.height = Some(TextureMap::from_loaded(img, Some(file_path)));
                }
                _ => {}
            }
        }

        Ok(set)
    }
}

/// A single texture in a PBR set (metadata only, for validation)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextureInfo {
    pub slot: TextureSlot,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

/// A complete PBR texture set (metadata view for validation)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TextureSet {
    pub textures: HashMap<TextureSlot, TextureInfo>,
}

impl TextureSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_texture(&mut self, info: TextureInfo) {
        self.textures.insert(info.slot, info);
    }

    pub fn get(&self, slot: TextureSlot) -> Option<&TextureInfo> {
        self.textures.get(&slot)
    }

    pub fn has_slot(&self, slot: TextureSlot) -> bool {
        self.textures.contains_key(&slot)
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.textures.values().next().map(|t| (t.width, t.height))
    }

    pub fn dimensions_consistent(&self) -> bool {
        let Some((w, h)) = self.dimensions() else {
            return true;
        };
        self.textures.values().all(|t| t.width == w && t.height == h)
    }
}

impl From<&MaterialSet> for TextureSet {
    fn from(set: &MaterialSet) -> Self {
        let mut textures = HashMap::new();

        if let Some(ref t) = set.albedo {
            textures.insert(
                TextureSlot::Albedo,
                TextureInfo {
                    slot: TextureSlot::Albedo,
                    path: t.path.clone().unwrap_or_default(),
                    width: t.width,
                    height: t.height,
                },
            );
        }
        if let Some(ref t) = set.normal {
            textures.insert(
                TextureSlot::Normal,
                TextureInfo {
                    slot: TextureSlot::Normal,
                    path: t.path.clone().unwrap_or_default(),
                    width: t.width,
                    height: t.height,
                },
            );
        }
        if let Some(ref t) = set.roughness {
            textures.insert(
                TextureSlot::Roughness,
                TextureInfo {
                    slot: TextureSlot::Roughness,
                    path: t.path.clone().unwrap_or_default(),
                    width: t.width,
                    height: t.height,
                },
            );
        }
        if let Some(ref t) = set.metallic {
            textures.insert(
                TextureSlot::Metallic,
                TextureInfo {
                    slot: TextureSlot::Metallic,
                    path: t.path.clone().unwrap_or_default(),
                    width: t.width,
                    height: t.height,
                },
            );
        }
        if let Some(ref t) = set.ao {
            textures.insert(
                TextureSlot::AmbientOcclusion,
                TextureInfo {
                    slot: TextureSlot::AmbientOcclusion,
                    path: t.path.clone().unwrap_or_default(),
                    width: t.width,
                    height: t.height,
                },
            );
        }
        if let Some(ref t) = set.height {
            textures.insert(
                TextureSlot::Height,
                TextureInfo {
                    slot: TextureSlot::Height,
                    path: t.path.clone().unwrap_or_default(),
                    width: t.width,
                    height: t.height,
                },
            );
        }

        TextureSet { textures }
    }
}

/// Analyzes PBR texture sets
pub struct MaterialAnalyzer;

impl MaterialAnalyzer {
    /// Analyze a texture set and return findings
    pub fn analyze(set: &TextureSet) -> MaterialAnalysis {
        MaterialAnalysis {
            has_albedo: set.has_slot(TextureSlot::Albedo),
            has_normal: set.has_slot(TextureSlot::Normal),
            has_metallic: set.has_slot(TextureSlot::Metallic),
            has_roughness: set.has_slot(TextureSlot::Roughness),
            has_ao: set.has_slot(TextureSlot::AmbientOcclusion),
            dimensions_consistent: set.dimensions_consistent(),
            texture_count: set.textures.len(),
        }
    }
}

/// Results of material analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialAnalysis {
    pub has_albedo: bool,
    pub has_normal: bool,
    pub has_metallic: bool,
    pub has_roughness: bool,
    pub has_ao: bool,
    pub dimensions_consistent: bool,
    pub texture_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_folder_detects_maps() {
        let img = image::RgbaImage::from_raw(
            4,
            4,
            vec![128u8; 4 * 4 * 4],
        )
        .unwrap();

        let tmp = std::env::temp_dir().join("pbr_material_test");
        std::fs::create_dir_all(&tmp).unwrap();

        img.save(tmp.join("albedo.png")).unwrap();
        img.save(tmp.join("normal.png")).unwrap();
        img.save(tmp.join("roughness.png")).unwrap();

        let set = MaterialSet::load_from_folder(&tmp).unwrap();

        std::fs::remove_file(tmp.join("albedo.png")).ok();
        std::fs::remove_file(tmp.join("normal.png")).ok();
        std::fs::remove_file(tmp.join("roughness.png")).ok();
        std::fs::remove_dir(&tmp).ok();

        assert!(set.has_albedo());
        assert!(set.has_normal());
        assert!(set.has_roughness());
        assert!(!set.has_metallic());
        assert!(!set.has_ao());
        assert!(!set.has_height());

        assert_eq!(set.texture_count(), 3);
        assert_eq!(set.dimensions(), Some((4, 4)));
        assert!(set.dimensions_consistent());

        let albedo = set.albedo.as_ref().unwrap();
        assert_eq!(albedo.width, 4);
        assert_eq!(albedo.height, 4);
        assert_eq!(albedo.data.len(), 4 * 4 * 4);
    }

    #[test]
    fn load_from_folder_with_exr() {
        let tmp = std::env::temp_dir().join("pbr_material_exr_test");
        std::fs::create_dir_all(&tmp).unwrap();

        exr::image::write::write_rgba_file(
            tmp.join("albedo.exr"),
            8,
            8,
            |_, _| (0.5_f32, 0.5, 0.5, 1.0),
        )
        .unwrap();

        let set = MaterialSet::load_from_folder(&tmp).unwrap();

        std::fs::remove_file(tmp.join("albedo.exr")).ok();
        std::fs::remove_dir(&tmp).ok();

        assert!(set.has_albedo());
        assert_eq!(set.albedo.as_ref().unwrap().width, 8);
        assert_eq!(set.albedo.as_ref().unwrap().height, 8);
    }
}

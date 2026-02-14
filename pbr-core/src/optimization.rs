//! Texture optimization module.
//!
//! Provides optimization presets for different targets (Unreal, Unity, Mobile):
//!
//! - **Resize textures**: 1K, 2K, 4K (longest edge) using Lanczos3 resampling
//! - **Channel packing**: R=AO, G=Roughness, B=Metallic (ORM/RMA texture)
//! - **LOD generation**: Low-res textures (512, 256, 128) for streaming
//!
//! All outputs are saved locally; no cloud or backend.

use crate::material::TextureMap;
use crate::Result;
use image::imageops::FilterType;
use image::{ImageBuffer, RgbaImage};

/// Target resolution presets for texture optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetResolution {
    /// 4096 pixels on the longest edge
    Res4K,
    /// 2048 pixels on the longest edge
    Res2K,
    /// 1024 pixels on the longest edge
    Res1K,
    /// 512 pixels (LOD 1)
    Res512,
    /// 256 pixels (LOD 2)
    Res256,
    /// 128 pixels (LOD 3)
    Res128,
    /// Custom maximum dimension (e.g. from plugin preset)
    Custom(u32),
}

impl TargetResolution {
    /// Maximum dimension (longest edge) in pixels.
    pub fn max_dimension(&self) -> u32 {
        match self {
            TargetResolution::Res4K => 4096,
            TargetResolution::Res2K => 2048,
            TargetResolution::Res1K => 1024,
            TargetResolution::Res512 => 512,
            TargetResolution::Res256 => 256,
            TargetResolution::Res128 => 128,
            TargetResolution::Custom(d) => *d,
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> String {
        match self {
            TargetResolution::Res4K => "4K".to_string(),
            TargetResolution::Res2K => "2K".to_string(),
            TargetResolution::Res1K => "1K".to_string(),
            TargetResolution::Res512 => "512".to_string(),
            TargetResolution::Res256 => "256".to_string(),
            TargetResolution::Res128 => "128".to_string(),
            TargetResolution::Custom(d) => format!("{}px", d),
        }
    }

    /// Default LOD chain (512, 256, 128) for streaming
    pub fn default_lod_levels() -> &'static [TargetResolution] {
        &[TargetResolution::Res512, TargetResolution::Res256, TargetResolution::Res128]
    }
}

/// Computes new dimensions for an image when scaling the longest edge to the target.
/// Preserves aspect ratio.
fn compute_target_dimensions(
    width: u32,
    height: u32,
    max_dim: u32,
) -> (u32, u32) {
    if width <= max_dim && height <= max_dim {
        return (width, height);
    }

    let scale = if width >= height {
        max_dim as f64 / width as f64
    } else {
        max_dim as f64 / height as f64
    };

    let new_width = (width as f64 * scale).round().max(1.0) as u32;
    let new_height = (height as f64 * scale).round().max(1.0) as u32;

    (new_width, new_height)
}

/// Resizes a TextureMap to a target resolution using Lanczos3 resampling.
/// Returns a new TextureMap; does not modify the original.
pub fn resize_texture(
    texture: &TextureMap,
    target: TargetResolution,
) -> Result<TextureMap> {
    let max_dim = target.max_dimension();
    let (new_width, new_height) =
        compute_target_dimensions(texture.width, texture.height, max_dim);

    if new_width == texture.width && new_height == texture.height {
        return Ok(texture.clone());
    }

    let img: RgbaImage = ImageBuffer::from_raw(
        texture.width,
        texture.height,
        texture.data.clone(),
    )
    .ok_or_else(|| crate::Error::Other("Invalid texture dimensions".into()))?;

    let resized = image::imageops::resize(
        &img,
        new_width,
        new_height,
        FilterType::Lanczos3,
    );

    let data = resized.into_raw();

    Ok(TextureMap {
        width: new_width,
        height: new_height,
        data,
        path: texture.path.clone(),
    })
}

/// Resizes all textures in a material set to the target resolution.
/// Only resizes textures that exceed the target; smaller textures are left unchanged.
pub fn resize_material_set(
    material: &crate::material::MaterialSet,
    target: TargetResolution,
) -> Result<crate::material::MaterialSet> {
    let mut result = material.clone();

    if let Some(ref t) = material.albedo {
        result.albedo = Some(resize_texture(t, target)?);
    }
    if let Some(ref t) = material.normal {
        result.normal = Some(resize_texture(t, target)?);
    }
    if let Some(ref t) = material.roughness {
        result.roughness = Some(resize_texture(t, target)?);
    }
    if let Some(ref t) = material.metallic {
        result.metallic = Some(resize_texture(t, target)?);
    }
    if let Some(ref t) = material.ao {
        result.ao = Some(resize_texture(t, target)?);
    }
    if let Some(ref t) = material.height {
        result.height = Some(resize_texture(t, target)?);
    }

    Ok(result)
}

/// Saves a TextureMap to the given path.
/// Format is inferred from the file extension (PNG, JPG, TGA).
pub fn save_texture<P: AsRef<std::path::Path>>(
    texture: &TextureMap,
    output_path: P,
) -> Result<()> {
    let path = output_path.as_ref();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    let img: RgbaImage = ImageBuffer::from_raw(
        texture.width,
        texture.height,
        texture.data.clone(),
    )
    .ok_or_else(|| crate::Error::Other("Invalid texture dimensions".into()))?;

    match ext.as_deref() {
        Some("png") => img.save(path)?,
        Some("jpg") | Some("jpeg") => img.save(path)?,
        Some("tga") => img.save(path)?,
        _ => {
            return Err(crate::Error::Other(format!(
                "Unsupported output format: {:?}. Use .png, .jpg, or .tga.",
                ext
            )))
        }
    }

    Ok(())
}

/// Saves a resized texture to the given output path.
/// Format is inferred from the file extension (PNG, JPG, TGA).
pub fn resize_and_save_texture<P: AsRef<std::path::Path>>(
    texture: &TextureMap,
    target: TargetResolution,
    output_path: P,
) -> Result<TextureMap> {
    let resized = resize_texture(texture, target)?;

    let path = output_path.as_ref();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    let img: RgbaImage = ImageBuffer::from_raw(
        resized.width,
        resized.height,
        resized.data.clone(),
    )
    .ok_or_else(|| crate::Error::Other("Invalid texture dimensions".into()))?;

    match ext.as_deref() {
        Some("png") => img.save(path)?,
        Some("jpg") | Some("jpeg") => img.save(path)?,
        Some("tga") => img.save(path)?,
        _ => {
            return Err(crate::Error::Other(format!(
                "Unsupported output format: {:?}. Use .png, .jpg, or .tga.",
                ext
            )))
        }
    }

    Ok(resized)
}

/// Extracts grayscale value from an RGBA pixel (uses R channel; for grayscale maps R=G=B).
#[inline]
fn sample_grayscale(data: &[u8], width: u32, _height: u32, x: u32, y: u32) -> u8 {
    let i = (y as usize * width as usize + x as usize) * 4;
    if i < data.len() {
        data[i]
    } else {
        0
    }
}

/// Resizes a texture to exact dimensions using Lanczos3.
fn resize_to_exact(texture: &TextureMap, width: u32, height: u32) -> Result<TextureMap> {
    if texture.width == width && texture.height == height {
        return Ok(texture.clone());
    }

    let img: RgbaImage = ImageBuffer::from_raw(
        texture.width,
        texture.height,
        texture.data.clone(),
    )
    .ok_or_else(|| crate::Error::Other("Invalid texture dimensions".into()))?;

    let resized =
        image::imageops::resize(&img, width, height, FilterType::Lanczos3);

    Ok(TextureMap {
        width,
        height,
        data: resized.into_raw(),
        path: texture.path.clone(),
    })
}

/// Packs roughness, metallic, and ambient occlusion maps into a single RGBA texture.
///
/// - **R channel** = Ambient Occlusion
/// - **G channel** = Roughness
/// - **B channel** = Metallic
/// - **A channel** = 255 (opaque)
///
/// This is a common game engine optimization (ORM/RMA texture) that reduces texture
/// samplers and memory bandwidth. All input maps are treated as grayscale (R channel used).
/// Output dimensions match the roughness map; metallic and AO are resized if they differ.
pub fn pack_rma(
    roughness: &TextureMap,
    metallic: &TextureMap,
    ao: &TextureMap,
) -> Result<TextureMap> {
    let width = roughness.width;
    let height = roughness.height;

    let metallic = if metallic.width != width || metallic.height != height {
        resize_to_exact(metallic, width, height)?
    } else {
        metallic.clone()
    };

    let ao = if ao.width != width || ao.height != height {
        resize_to_exact(ao, width, height)?
    } else {
        ao.clone()
    };

    let pixel_count = (width as usize) * (height as usize);
    let mut data = Vec::with_capacity(pixel_count * 4);

    for y in 0..height {
        for x in 0..width {
            let ao_val = sample_grayscale(&ao.data, width, height, x, y);
            let r_val = sample_grayscale(&roughness.data, width, height, x, y);
            let m_val = sample_grayscale(&metallic.data, width, height, x, y);
            data.extend_from_slice(&[ao_val, r_val, m_val, 255]);
        }
    }

    Ok(TextureMap {
        width,
        height,
        data,
        path: None,
    })
}

/// Export preset identifiers for game engine optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPreset {
    /// High quality: 4K resolution, packed RMA texture
    Res4K,
    /// Unreal Engine: 2K resolution, packed RMA texture
    UnrealEngine,
    /// Unity: 2K resolution, packed RMA texture
    Unity,
    /// Mobile: 1K resolution, packed RMA texture
    MobileOptimized,
}

impl ExportPreset {
    /// Base resolution for this preset (1K, 2K, or 4K).
    pub fn target_resolution(&self) -> TargetResolution {
        match self {
            ExportPreset::Res4K => TargetResolution::Res4K,
            ExportPreset::UnrealEngine | ExportPreset::Unity => TargetResolution::Res2K,
            ExportPreset::MobileOptimized => TargetResolution::Res1K,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ExportPreset::Res4K => "4K",
            ExportPreset::UnrealEngine => "Unreal Engine",
            ExportPreset::Unity => "Unity",
            ExportPreset::MobileOptimized => "Mobile Optimized",
        }
    }

    /// Default LOD chain for this preset. Unreal/Unity: 512, 256, 128. Mobile: 256, 128.
    pub fn default_lod_levels(&self) -> &'static [TargetResolution] {
        match self {
            ExportPreset::Res4K | ExportPreset::UnrealEngine | ExportPreset::Unity => {
                TargetResolution::default_lod_levels()
            }
            ExportPreset::MobileOptimized => &[TargetResolution::Res256, TargetResolution::Res128],
        }
    }
}

/// Configurable optimization preset for a target platform.
///
/// Combines resolution (1K/2K/4K), channel packing (R=AO, G=Roughness, B=Metallic),
/// and optional LOD generation. All files saved locally.
#[derive(Debug, Clone)]
pub struct OptimizationPreset {
    /// Target platform (Unreal, Unity, Mobile).
    pub preset: ExportPreset,
    /// Override base resolution (None = use preset default).
    pub resolution: Option<TargetResolution>,
    /// Enable channel packing of R=AO, G=Roughness, B=Metallic. Always true for presets.
    pub pack_rma: bool,
    /// LOD levels for low-res textures (None = use preset default).
    pub lod_levels: Option<Vec<TargetResolution>>,
}

impl OptimizationPreset {
    /// Unreal Engine: 2K base, packed ORM, LOD 512/256/128.
    pub fn unreal() -> Self {
        Self {
            preset: ExportPreset::UnrealEngine,
            resolution: None,
            pack_rma: true,
            lod_levels: None,
        }
    }

    /// Unity: 2K base, packed ORM, LOD 512/256/128.
    pub fn unity() -> Self {
        Self {
            preset: ExportPreset::Unity,
            resolution: None,
            pack_rma: true,
            lod_levels: None,
        }
    }

    /// Mobile: 1K base, packed ORM, LOD 256/128.
    pub fn mobile() -> Self {
        Self {
            preset: ExportPreset::MobileOptimized,
            resolution: None,
            pack_rma: true,
            lod_levels: None,
        }
    }

    /// Res4K: 4K base, packed ORM, LOD 512/256/128.
    pub fn res_4k() -> Self {
        Self {
            preset: ExportPreset::Res4K,
            resolution: None,
            pack_rma: true,
            lod_levels: None,
        }
    }

    /// Override base resolution (1K, 2K, or 4K).
    pub fn with_resolution(mut self, resolution: TargetResolution) -> Self {
        self.resolution = Some(resolution);
        self
    }

    /// Override LOD levels for low-res textures.
    pub fn with_lod_levels(mut self, levels: &[TargetResolution]) -> Self {
        self.lod_levels = Some(levels.to_vec());
        self
    }

    /// Effective base resolution (override or preset default).
    pub fn effective_resolution(&self) -> TargetResolution {
        self.resolution
            .unwrap_or_else(|| self.preset.target_resolution())
    }

    /// Effective LOD levels (override or preset default).
    pub fn effective_lod_levels(&self) -> Vec<TargetResolution> {
        self.lod_levels
            .clone()
            .unwrap_or_else(|| self.preset.default_lod_levels().to_vec())
    }
}

/// Export with a specific target resolution (e.g. from plugin preset).
pub fn export_with_target<P: AsRef<std::path::Path>>(
    material: &crate::material::MaterialSet,
    output_dir: P,
    target: TargetResolution,
) -> Result<Vec<std::path::PathBuf>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;
    let optimized = resize_material_set(material, target)?;
    export_material_to_dir(&optimized, output_dir)
}

/// Exports an optimized material set to the given output directory.
/// Applies preset-specific optimizations (resize, RMA packing).
/// Creates output_dir if it doesn't exist.
pub fn export_with_preset<P: AsRef<std::path::Path>>(
    material: &crate::material::MaterialSet,
    output_dir: P,
    preset: ExportPreset,
) -> Result<Vec<std::path::PathBuf>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;
    let target = preset.target_resolution();
    let optimized = resize_material_set(material, target)?;
    export_material_to_dir(&optimized, output_dir)
}

/// Generate LOD (low-res) versions of a material set.
/// Returns a sequence of (level, resized_set) for each LOD level.
pub fn generate_lod_chain(
    material: &crate::material::MaterialSet,
    levels: &[TargetResolution],
) -> Result<Vec<(TargetResolution, crate::material::MaterialSet)>> {
    let mut result = Vec::with_capacity(levels.len());
    for &level in levels {
        let resized = resize_material_set(material, level)?;
        result.push((level, resized));
    }
    Ok(result)
}

/// Export with an optimization preset. Resizes to target resolution (1K/2K/4K),
/// packs R=AO, G=Roughness, B=Metallic, and optionally generates LOD chain.
/// All files saved locally.
pub fn export_with_optimization_preset<P: AsRef<std::path::Path>>(
    material: &crate::material::MaterialSet,
    output_dir: P,
    preset: OptimizationPreset,
    include_lod: bool,
) -> Result<Vec<std::path::PathBuf>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;

    let target = preset.effective_resolution();
    let optimized = resize_material_set(material, target)?;

    if include_lod {
        let lod_levels = preset.effective_lod_levels();
        export_with_target_and_lod(material, output_dir, target, &lod_levels)
    } else {
        export_material_to_dir(&optimized, output_dir)
    }
}

/// Export with explicit target resolution and LOD chain. Creates LOD0/, LOD1/, LOD2/ subdirs.
/// All files saved locally; no cloud or backend.
pub fn export_with_target_and_lod<P: AsRef<std::path::Path>>(
    material: &crate::material::MaterialSet,
    output_dir: P,
    base_resolution: TargetResolution,
    lod_levels: &[TargetResolution],
) -> Result<Vec<std::path::PathBuf>> {
    let output_dir = output_dir.as_ref();
    let optimized = resize_material_set(material, base_resolution)?;

    let mut written = Vec::new();
    let lod0_dir = output_dir.join("LOD0");
    std::fs::create_dir_all(&lod0_dir)?;
    written.extend(export_material_to_dir(&optimized, &lod0_dir)?);

    for (i, &level) in lod_levels.iter().enumerate() {
        let lod_dir = output_dir.join(format!("LOD{}", i + 1));
        std::fs::create_dir_all(&lod_dir)?;
        let resized = resize_material_set(material, level)?;
        written.extend(export_material_to_dir(&resized, &lod_dir)?);
    }

    Ok(written)
}

/// Export with preset plus LOD chain (low-res textures for streaming).
/// Creates subdirs: LOD0/, LOD1/, LOD2/ (or Base/, LOD1/, LOD2/ etc.)
pub fn export_with_lod<P: AsRef<std::path::Path>>(
    material: &crate::material::MaterialSet,
    output_dir: P,
    preset: ExportPreset,
    lod_levels: &[TargetResolution],
) -> Result<Vec<std::path::PathBuf>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;

    let target = preset.target_resolution();
    let optimized = resize_material_set(material, target)?;

    let mut written = Vec::new();

    // LOD 0 (full resolution)
    let lod0_dir = output_dir.join("LOD0");
    std::fs::create_dir_all(&lod0_dir)?;
    written.extend(export_material_to_dir(&optimized, &lod0_dir)?);

    // LOD 1, 2, 3...
    for (i, &level) in lod_levels.iter().enumerate() {
        let lod_dir = output_dir.join(format!("LOD{}", i + 1));
        std::fs::create_dir_all(&lod_dir)?;
        let resized = resize_material_set(material, level)?;
        written.extend(export_material_to_dir(&resized, &lod_dir)?);
    }

    Ok(written)
}

/// Export material set to output dir (BaseColor, Normal, ORM, etc.)
fn export_material_to_dir<P: AsRef<std::path::Path>>(
    material: &crate::material::MaterialSet,
    output_dir: P,
) -> Result<Vec<std::path::PathBuf>> {
    let output_dir = output_dir.as_ref();
    let mut written = Vec::new();

    if let Some(ref t) = material.albedo {
        let path = output_dir.join("BaseColor.png");
        save_texture(t, &path)?;
        written.push(path);
    }
    if let Some(ref t) = material.normal {
        let path = output_dir.join("Normal.png");
        save_texture(t, &path)?;
        written.push(path);
    }
    if let Some(rma) = pack_rma_from_material(material)? {
        let path = output_dir.join("ORM.png");
        save_texture(&rma, &path)?;
        written.push(path);
    } else {
        if let Some(ref t) = material.roughness {
            let path = output_dir.join("Roughness.png");
            save_texture(t, &path)?;
            written.push(path);
        }
        if let Some(ref t) = material.metallic {
            let path = output_dir.join("Metallic.png");
            save_texture(t, &path)?;
            written.push(path);
        }
        if let Some(ref t) = material.ao {
            let path = output_dir.join("AmbientOcclusion.png");
            save_texture(t, &path)?;
            written.push(path);
        }
    }
    if let Some(ref t) = material.height {
        let path = output_dir.join("Height.png");
        save_texture(t, &path)?;
        written.push(path);
    }

    Ok(written)
}

/// Batch export multiple materials with a preset.
/// Each material is exported to output_root/<material_name>/.
pub fn batch_export_with_preset<P: AsRef<std::path::Path>>(
    materials: &[(std::path::PathBuf, crate::material::MaterialSet)],
    output_root: P,
    preset: ExportPreset,
) -> Result<Vec<std::path::PathBuf>> {
    let output_root = output_root.as_ref();
    std::fs::create_dir_all(output_root)?;

    let mut all_written = Vec::new();
    for (folder, material) in materials {
        let name = material
            .name
            .clone()
            .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "material".to_string());
        let material_dir = output_root.join(&name);
        let written = export_with_preset(material, &material_dir, preset)?;
        all_written.extend(written);
    }
    Ok(all_written)
}

/// Batch export multiple materials with an optimization preset.
/// Each material is exported to output_root/<material_name>/ with optional LOD.
pub fn batch_export_with_optimization_preset<P: AsRef<std::path::Path>>(
    materials: &[(std::path::PathBuf, crate::material::MaterialSet)],
    output_root: P,
    preset: OptimizationPreset,
    include_lod: bool,
) -> Result<Vec<std::path::PathBuf>> {
    let output_root = output_root.as_ref();
    std::fs::create_dir_all(output_root)?;

    let mut all_written = Vec::new();
    for (folder, material) in materials {
        let name = material
            .name
            .clone()
            .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "material".to_string());
        let material_dir = output_root.join(&name);
        let written = export_with_optimization_preset(material, &material_dir, preset.clone(), include_lod)?;
        all_written.extend(written);
    }
    Ok(all_written)
}

/// Packs roughness, metallic, and AO from a material set if all three are present.
/// Returns `None` if any map is missing.
pub fn pack_rma_from_material(
    material: &crate::material::MaterialSet,
) -> Result<Option<TextureMap>> {
    let Some(ref roughness) = material.roughness else {
        return Ok(None);
    };
    let Some(ref metallic) = material.metallic else {
        return Ok(None);
    };
    let Some(ref ao) = material.ao else {
        return Ok(None);
    };
    pack_rma(roughness, metallic, ao).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_texture(w: u32, h: u32) -> TextureMap {
        let len = (w as usize) * (h as usize) * 4;
        TextureMap {
            width: w,
            height: h,
            data: vec![128u8; len],
            path: None,
        }
    }

    #[test]
    fn resize_texture_4k() {
        let tex = make_test_texture(5120, 5120);
        let resized = resize_texture(&tex, TargetResolution::Res4K).unwrap();
        assert_eq!(resized.width, 4096);
        assert_eq!(resized.height, 4096);
        assert_eq!(resized.data.len(), 4096 * 4096 * 4);
    }

    #[test]
    fn resize_texture_2k() {
        let tex = make_test_texture(4096, 4096);
        let resized = resize_texture(&tex, TargetResolution::Res2K).unwrap();
        assert_eq!(resized.width, 2048);
        assert_eq!(resized.height, 2048);
        assert_eq!(resized.data.len(), 2048 * 2048 * 4);
    }

    #[test]
    fn resize_texture_1k() {
        let tex = make_test_texture(2048, 2048);
        let resized = resize_texture(&tex, TargetResolution::Res1K).unwrap();
        assert_eq!(resized.width, 1024);
        assert_eq!(resized.height, 1024);
        assert_eq!(resized.data.len(), 1024 * 1024 * 4);
    }

    #[test]
    fn resize_texture_unchanged_when_smaller() {
        let tex = make_test_texture(512, 512);
        let resized = resize_texture(&tex, TargetResolution::Res2K).unwrap();
        assert_eq!(resized.width, 512);
        assert_eq!(resized.height, 512);
    }

    #[test]
    fn resize_texture_preserves_aspect_ratio() {
        let tex = make_test_texture(4096, 2048);
        let resized = resize_texture(&tex, TargetResolution::Res2K).unwrap();
        assert_eq!(resized.width, 2048);
        assert_eq!(resized.height, 1024);
    }

    #[test]
    fn pack_rma_combines_channels() {
        // Roughness=64, Metallic=128, AO=192 per pixel
        let roughness = make_grayscale_texture(4, 4, 64);
        let metallic = make_grayscale_texture(4, 4, 128);
        let ao = make_grayscale_texture(4, 4, 192);

        let packed = pack_rma(&roughness, &metallic, &ao).unwrap();
        assert_eq!(packed.width, 4);
        assert_eq!(packed.height, 4);
        assert_eq!(packed.data.len(), 4 * 4 * 4);

        // First pixel: R=AO(192), G=Roughness(64), B=Metallic(128), A=255
        assert_eq!(packed.pixel(0, 0), Some([192, 64, 128, 255]));
    }

    #[test]
    fn pack_rma_resizes_mismatched_dimensions() {
        let roughness = make_grayscale_texture(4, 4, 100);
        let metallic = make_grayscale_texture(2, 2, 150);
        let ao = make_grayscale_texture(8, 8, 200);

        let packed = pack_rma(&roughness, &metallic, &ao).unwrap();
        assert_eq!(packed.width, 4);
        assert_eq!(packed.height, 4);
        assert_eq!(packed.data.len(), 4 * 4 * 4);
    }

    #[test]
    fn optimization_preset_defaults() {
        let unreal = OptimizationPreset::unreal();
        assert_eq!(unreal.effective_resolution(), TargetResolution::Res2K);
        assert_eq!(unreal.effective_lod_levels().len(), 3);

        let mobile = OptimizationPreset::mobile();
        assert_eq!(mobile.effective_resolution(), TargetResolution::Res1K);
        assert_eq!(mobile.effective_lod_levels().len(), 2);

        let unity_4k = OptimizationPreset::unity().with_resolution(TargetResolution::Res4K);
        assert_eq!(unity_4k.effective_resolution(), TargetResolution::Res4K);
    }

    #[test]
    fn export_preset_lod_levels() {
        assert_eq!(ExportPreset::UnrealEngine.default_lod_levels().len(), 3);
        assert_eq!(ExportPreset::MobileOptimized.default_lod_levels().len(), 2);
    }

    fn make_grayscale_texture(w: u32, h: u32, value: u8) -> TextureMap {
        let len = (w as usize) * (h as usize) * 4;
        TextureMap {
            width: w,
            height: h,
            data: (0..len).map(|i| if i % 4 == 0 { value } else { value }).collect(),
            path: None,
        }
    }
}

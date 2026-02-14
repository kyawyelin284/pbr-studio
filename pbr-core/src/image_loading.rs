//! Image loading and texture metadata.
//!
//! Loads PNG, JPG, TGA, and EXR files and returns width, height, and RGBA color data.
//! Supports common PBR map names for automatic slot detection.
//! EXR (OpenEXR) HDR values are tone-mapped to 8-bit for analysis.

use crate::Result;
use image::GenericImageView;
use image::{DynamicImage, ImageFormat};
use std::path::Path;

/// Standard PBR texture slot identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextureSlot {
    Albedo,
    Normal,
    Metallic,
    Roughness,
    AmbientOcclusion,
    Emissive,
    Height,
}

impl TextureSlot {
    /// Common filename suffixes for this slot (without extension)
    pub fn common_suffixes(&self) -> &[&'static str] {
        match self {
            TextureSlot::Albedo => &["albedo", "basecolor", "diffuse", "color"],
            TextureSlot::Normal => &["normal", "norm"],
            TextureSlot::Metallic => &["metallic", "metal"],
            TextureSlot::Roughness => &["roughness", "rough"],
            TextureSlot::AmbientOcclusion => &["ao", "ambientocclusion", "ambient_occlusion"],
            TextureSlot::Emissive => &["emissive", "emission"],
            TextureSlot::Height => &["height", "displacement", "bump"],
        }
    }
}

/// Supported image formats for loading (PNG, JPG, TGA, EXR)
pub const SUPPORTED_FORMATS: &[ImageFormat] = &[
    ImageFormat::Png,
    ImageFormat::Jpeg,
    ImageFormat::Tga,
    ImageFormat::OpenExr,
];

/// A loaded texture image with pixel data
#[derive(Debug, Clone)]
pub struct LoadedImage {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// RGBA pixel data (4 bytes per pixel, row-major)
    pub data: Vec<u8>,
    /// Source format used when loading
    pub format: ImageFormat,
    /// Detected channel/color info
    pub color_type: String,
}

impl LoadedImage {
    /// Total number of pixels
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Size in bytes (width * height * 4)
    pub fn data_len(&self) -> usize {
        self.data.len()
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

    fn from_dynamic(image: DynamicImage, format: ImageFormat) -> Self {
        let (width, height) = image.dimensions();
        let color_type = format!("{:?}", image.color());
        let rgba = image.to_rgba8();
        let data = rgba.into_raw();

        Self {
            width,
            height,
            data,
            format,
            color_type,
        }
    }
}

/// Result of validating EXR channel data after loading.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ExrValidationReport {
    /// Whether the loaded data passed validation
    pub valid: bool,
    /// Number of channels (always 4 for RGBA output)
    pub channel_count: u32,
    /// Warnings (e.g. empty regions, unusual value range)
    pub warnings: Vec<String>,
}

impl LoadedImage {
    /// Validate EXR channel data. Call when format is OpenExr.
    /// Checks dimensions, data length, and basic data integrity.
    pub fn validate_exr_channels(&self) -> ExrValidationReport {
        let mut report = ExrValidationReport {
            valid: true,
            channel_count: 4,
            warnings: Vec::new(),
        };

        if self.width == 0 || self.height == 0 {
            report.valid = false;
            report.warnings.push("Invalid dimensions: width and height must be > 0".into());
            return report;
        }

        let expected_len = (self.width as usize) * (self.height as usize) * 4;
        if self.data.len() != expected_len {
            report.valid = false;
            report.warnings.push(format!(
                "Data length mismatch: expected {} bytes, got {}",
                expected_len,
                self.data.len()
            ));
            return report;
        }

        // Check for all-zero (possibly failed/corrupt load)
        if self.data.iter().all(|&b| b == 0) {
            report.warnings.push("All pixels are zero - image may be empty or corrupt".into());
        }

        report
    }
}

/// Loads and parses PBR texture images (PNG, JPG, TGA, EXR)
pub struct ImageLoader;

impl ImageLoader {
    /// Load an image from a file path (PNG, JPG, TGA, EXR)
    /// EXR HDR values are tone-mapped to 8-bit RGBA for analysis.
    /// For EXR, validates channel data and returns errors on failure.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<LoadedImage> {
        let path = path.as_ref();
        let reader = image::ImageReader::open(path)?;
        let format = reader.format().unwrap_or_else(|| {
            path.extension()
                .and_then(|e| e.to_str())
                .and_then(|s| ImageFormat::from_extension(s))
                .unwrap_or(ImageFormat::Png)
        });

        if !SUPPORTED_FORMATS.contains(&format) {
            return Err(crate::Error::Other(format!(
                "Unsupported format: {:?}. Use PNG, JPG, TGA, or EXR.",
                format
            )));
        }

        let image = reader.decode()?;
        let loaded = LoadedImage::from_dynamic(image, format);

        // Validate EXR channel data when applicable
        if format == ImageFormat::OpenExr {
            let validation = loaded.validate_exr_channels();
            if !validation.valid {
                return Err(crate::Error::Other(format!(
                    "EXR channel validation failed: {}",
                    validation.warnings.join("; ")
                )));
            }
        }

        Ok(loaded)
    }

    /// Load from file and detect PBR slot from filename
    pub fn load_with_slot<P: AsRef<Path>>(path: P) -> Result<(LoadedImage, Option<TextureSlot>)> {
        let slot = Self::detect_slot_from_path(path.as_ref());
        let image = Self::load(path)?;
        Ok((image, slot))
    }

    /// Attempt to detect texture slot from filename
    pub fn detect_slot_from_path<P: AsRef<Path>>(path: P) -> Option<TextureSlot> {
        let stem = path.as_ref().file_stem()?.to_str()?.to_lowercase();

        for slot in [
            TextureSlot::Albedo,
            TextureSlot::Normal,
            TextureSlot::Metallic,
            TextureSlot::Roughness,
            TextureSlot::AmbientOcclusion,
            TextureSlot::Emissive,
            TextureSlot::Height,
        ] {
            if slot.common_suffixes().iter().any(|s| stem.contains(s)) {
                return Some(slot);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_png_returns_width_height_and_data() {
        let img = image::RgbaImage::from_raw(
            3,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255,
                128, 128, 128, 255, 64, 64, 64, 128, 0, 0, 0, 255,
            ],
        )
        .unwrap();

        let tmp = std::env::temp_dir().join("pbr_core_test.png");
        img.save(&tmp).unwrap();

        let loaded = ImageLoader::load(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(loaded.width, 3);
        assert_eq!(loaded.height, 2);
        assert_eq!(loaded.data.len(), 3 * 2 * 4);
        assert_eq!(loaded.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(loaded.pixel(1, 0), Some([0, 255, 0, 255]));
    }

    #[test]
    fn load_exr_returns_width_height_and_data() {
        let tmp = std::env::temp_dir().join("pbr_core_test.exr");
        exr::image::write::write_rgba_file(
            &tmp,
            2,
            2,
            |x, y| {
                let (r, g, b) = match (x, y) {
                    (0, 0) => (1.0_f32, 0.0, 0.0),
                    (1, 0) => (0.0, 1.0, 0.0),
                    (0, 1) => (0.0, 0.0, 1.0),
                    _ => (0.5, 0.5, 0.5),
                };
                (r, g, b, 1.0_f32)
            },
        )
        .unwrap();

        let loaded = ImageLoader::load(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(loaded.width, 2);
        assert_eq!(loaded.height, 2);
        assert_eq!(loaded.data.len(), 2 * 2 * 4);
        assert_eq!(loaded.format, ImageFormat::OpenExr);
        // Tone-mapped: (1,0,0) -> red dominant, (0,1,0) -> green dominant
        let p00 = loaded.pixel(0, 0).unwrap();
        let p10 = loaded.pixel(1, 0).unwrap();
        assert!(p00[0] > p00[1] && p00[0] > p00[2], "pixel (0,0) should be red");
        assert!(p10[1] > p10[0] && p10[1] > p10[2], "pixel (1,0) should be green");
    }

    #[test]
    fn validate_exr_channels_valid() {
        let loaded = LoadedImage {
            width: 4,
            height: 4,
            data: vec![128; 4 * 4 * 4],
            format: ImageFormat::OpenExr,
            color_type: "Rgba32F".into(),
        };
        let report = loaded.validate_exr_channels();
        assert!(report.valid);
        assert_eq!(report.channel_count, 4);
    }

    #[test]
    fn validate_exr_channels_invalid_dimensions() {
        let loaded = LoadedImage {
            width: 0,
            height: 4,
            data: vec![],
            format: ImageFormat::OpenExr,
            color_type: "Rgba32F".into(),
        };
        let report = loaded.validate_exr_channels();
        assert!(!report.valid);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn detect_slot_albedo_exr() {
        assert_eq!(
            ImageLoader::detect_slot_from_path("material_albedo.exr"),
            Some(TextureSlot::Albedo)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("roughness.exr"),
            Some(TextureSlot::Roughness)
        );
    }

    #[test]
    fn detect_slot_albedo_basecolor_normal() {
        assert_eq!(
            ImageLoader::detect_slot_from_path("material_albedo.png"),
            Some(TextureSlot::Albedo)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("basecolor.jpg"),
            Some(TextureSlot::Albedo)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("normal_map.tga"),
            Some(TextureSlot::Normal)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("roughness.png"),
            Some(TextureSlot::Roughness)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("metallic.png"),
            Some(TextureSlot::Metallic)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("ao.png"),
            Some(TextureSlot::AmbientOcclusion)
        );
        assert_eq!(
            ImageLoader::detect_slot_from_path("height.tga"),
            Some(TextureSlot::Height)
        );
    }
}

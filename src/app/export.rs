//! Format-independent export entry point.

use egui::Color32;
use std::io::Cursor;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportFormat {
    #[default]
    Png,
    Jpeg,
    Bmp,
    WebP,
}

impl ExportFormat {
    pub const JPEG: Self = Self::Jpeg;
    pub const JPG: Self = Self::Jpeg;
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Bmp => "bmp",
            Self::WebP => "webp",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    pub format: ExportFormat,
}

impl ExportOptions {
    pub fn new(format: ExportFormat) -> Self {
        Self { format }
    }

    pub fn png() -> Self {
        Self {
            format: ExportFormat::Png,
        }
    }

    pub fn jpeg() -> Self {
        Self {
            format: ExportFormat::Jpeg,
        }
    }

    pub fn jpg() -> Self {
        Self::jpeg()
    }

    pub fn bmp() -> Self {
        Self {
            format: ExportFormat::Bmp,
        }
    }

    pub fn webp() -> Self {
        Self {
            format: ExportFormat::WebP,
        }
    }
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self::png()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedImage {
    pub width: usize,
    pub height: usize,
    rgba: Vec<u8>,
}

impl RenderedImage {
    pub fn new(width: usize, height: usize, rgba: Vec<u8>) -> Result<Self, String> {
        let expected = width
            .checked_mul(height)
            .and_then(|count| count.checked_mul(4))
            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?;
        if rgba.len() != expected {
            return Err("Dimensioni dei pixel non valide.".to_string());
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExportManager;

impl ExportManager {
    pub fn new() -> Self {
        Self
    }

    pub fn export(
        &self,
        width: usize,
        height: usize,
        pixels: &[Color32],
        options: ExportOptions,
    ) -> Result<Vec<u8>, String> {
        match options.format {
            ExportFormat::Png => Self::encode_png(width, height, pixels),
            ExportFormat::Jpeg => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                Self::encode_jpeg(width, height, &rgba)
            }
            ExportFormat::Bmp => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                Self::encode_bmp(width, height, &rgba)
            }
            ExportFormat::WebP => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                Self::encode_webp(width, height, &rgba)
            }
        }
    }

    pub fn export_to_file(
        &self,
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        pixels: &[Color32],
        options: ExportOptions,
    ) -> Result<(), String> {
        match options.format {
            ExportFormat::Png => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                image::save_buffer(
                    path,
                    &rgba,
                    width as u32,
                    height as u32,
                    image::ColorType::Rgba8,
                )
                .map_err(|error| error.to_string())
            }
            ExportFormat::Jpeg => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                Self::encode_jpeg_to_file(path, width, height, &rgba)
            }
            ExportFormat::Bmp => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                Self::encode_bmp_to_file(path, width, height, &rgba)
            }
            ExportFormat::WebP => {
                let mut rgba =
                    vec![
                        0_u8;
                        width
                            .checked_mul(height)
                            .and_then(|count| count.checked_mul(4))
                            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?
                    ];
                for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
                    destination.copy_from_slice(&source.to_srgba_unmultiplied());
                }
                Self::encode_webp_to_file(path, width, height, &rgba)
            }
        }
    }

    pub fn export_rgba_to_file(
        &self,
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        rgba: &[u8],
        options: ExportOptions,
    ) -> Result<(), String> {
        match options.format {
            ExportFormat::Png => image::save_buffer(
                path,
                rgba,
                width as u32,
                height as u32,
                image::ColorType::Rgba8,
            )
            .map_err(|error| error.to_string()),
            ExportFormat::Jpeg => Self::encode_jpeg_to_file(path, width, height, rgba),
            ExportFormat::Bmp => Self::encode_bmp_to_file(path, width, height, rgba),
            ExportFormat::WebP => Self::encode_webp_to_file(path, width, height, rgba),
        }
    }

    pub fn export_rendered(
        &self,
        path: impl AsRef<Path>,
        rendered: &RenderedImage,
        options: ExportOptions,
    ) -> Result<(), String> {
        self.export_rgba_to_file(
            path,
            rendered.width,
            rendered.height,
            rendered.rgba(),
            options,
        )
    }

    fn rgb_from_rgba(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let byte_count = width
            .checked_mul(height)
            .and_then(|count| count.checked_mul(3))
            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?;
        let mut rgb = vec![0_u8; byte_count];
        for (destination, source) in rgb.chunks_exact_mut(3).zip(rgba.chunks_exact(4)) {
            let alpha = source[3] as u32;
            for channel in 0..3 {
                destination[channel] =
                    ((source[channel] as u32 * alpha + 255 * (255 - alpha)) / 255) as u8;
            }
        }
        Ok(rgb)
    }

    fn encode_jpeg(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let rgb = Self::rgb_from_rgba(width, height, rgba)?;
        let image = image::RgbImage::from_raw(width as u32, height as u32, rgb)
            .ok_or_else(|| "Impossibile preparare l'immagine JPEG.".to_string())?;
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut output, image::ImageOutputFormat::Jpeg(85))
            .map_err(|error| error.to_string())?;
        Ok(output.into_inner())
    }

    fn encode_jpeg_to_file(
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        rgba: &[u8],
    ) -> Result<(), String> {
        let rgb = Self::rgb_from_rgba(width, height, rgba)?;
        let mut output = std::fs::File::create(path).map_err(|error| error.to_string())?;
        let image = image::RgbImage::from_raw(width as u32, height as u32, rgb)
            .ok_or_else(|| "Impossibile preparare l'immagine JPEG.".to_string())?;
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut output, image::ImageOutputFormat::Jpeg(85))
            .map_err(|error| error.to_string())
    }

    fn encode_bmp(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let rgb = Self::rgb_from_rgba(width, height, rgba)?;
        let image = image::RgbImage::from_raw(width as u32, height as u32, rgb)
            .ok_or_else(|| "Impossibile preparare l'immagine BMP.".to_string())?;
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut output, image::ImageOutputFormat::Bmp)
            .map_err(|error| error.to_string())?;
        Ok(output.into_inner())
    }

    fn encode_bmp_to_file(
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        rgba: &[u8],
    ) -> Result<(), String> {
        let rgb = Self::rgb_from_rgba(width, height, rgba)?;
        let mut output = std::fs::File::create(path).map_err(|error| error.to_string())?;
        let image = image::RgbImage::from_raw(width as u32, height as u32, rgb)
            .ok_or_else(|| "Impossibile preparare l'immagine BMP.".to_string())?;
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut output, image::ImageOutputFormat::Bmp)
            .map_err(|error| error.to_string())
    }

    fn encode_webp(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let encoder = webp::Encoder::from_rgba(rgba, width as u32, height as u32);
        let encoded = encoder.encode_lossless();
        Ok((&*encoded).to_vec())
    }

    fn encode_webp_to_file(
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        rgba: &[u8],
    ) -> Result<(), String> {
        let encoded = Self::encode_webp(width, height, rgba)?;
        std::fs::write(path, encoded).map_err(|error| error.to_string())
    }

    pub fn encode_png(width: usize, height: usize, pixels: &[Color32]) -> Result<Vec<u8>, String> {
        let pixel_count = width
            .checked_mul(height)
            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?;
        let byte_count = pixel_count
            .checked_mul(4)
            .ok_or_else(|| "Dimensioni immagine non valide.".to_string())?;
        let mut rgba = vec![0; byte_count];
        for (destination, source) in rgba.chunks_exact_mut(4).zip(pixels) {
            destination.copy_from_slice(&source.to_srgba_unmultiplied());
        }
        let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
            .ok_or_else(|| "Impossibile preparare un livello OpenRaster.".to_string())?;
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut output, image::ImageOutputFormat::Png)
            .map_err(|error| error.to_string())?;
        Ok(output.into_inner())
    }

    pub fn save_png(
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        pixels: &[Color32],
    ) -> Result<(), String> {
        Self::export_to_file_static(path, width, height, pixels)
    }

    fn export_to_file_static(
        path: impl AsRef<Path>,
        width: usize,
        height: usize,
        pixels: &[Color32],
    ) -> Result<(), String> {
        let manager = Self::new();
        manager.export_to_file(path, width, height, pixels, ExportOptions::png())
    }
}

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageFormat, ImageOutputFormat};
use std::io::Cursor;
use tracing::debug;

pub struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_thumbnail(&self, image_data: &[u8], max_size: u32) -> Result<Vec<u8>> {
        let img = image::load_from_memory(image_data).context("Failed to decode image")?;

        let thumbnail = self.resize_image(img, max_size)?;

        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);

        thumbnail
            .write_to(&mut cursor, ImageOutputFormat::Jpeg(85))
            .context("Failed to encode thumbnail")?;

        debug!(
            "Generated thumbnail: {} bytes -> {} bytes",
            image_data.len(),
            output.len()
        );
        Ok(output)
    }

    pub fn resize_image(&self, img: DynamicImage, max_size: u32) -> Result<DynamicImage> {
        let (width, height) = img.dimensions();

        if width <= max_size && height <= max_size {
            return Ok(img);
        }

        let (new_width, new_height) = if width > height {
            let ratio = max_size as f32 / width as f32;
            (max_size, (height as f32 * ratio) as u32)
        } else {
            let ratio = max_size as f32 / height as f32;
            ((width as f32 * ratio) as u32, max_size)
        };

        debug!(
            "Resizing image: {}x{} -> {}x{}",
            width, height, new_width, new_height
        );

        Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
    }

    pub fn validate_image(&self, data: &[u8]) -> bool {
        image::load_from_memory(data).is_ok()
    }

    pub fn get_image_dimensions(&self, data: &[u8]) -> Result<(u32, u32)> {
        let img = image::load_from_memory(data).context("Failed to decode image")?;
        Ok(img.dimensions())
    }

    pub fn detect_format(&self, data: &[u8]) -> Option<ImageFormat> {
        image::guess_format(data).ok()
    }

    pub fn convert_to_jpeg(&self, image_data: &[u8], quality: u8) -> Result<Vec<u8>> {
        let img = image::load_from_memory(image_data).context("Failed to decode image")?;

        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);

        img.write_to(&mut cursor, ImageOutputFormat::Jpeg(quality))
            .context("Failed to encode as JPEG")?;

        Ok(output)
    }

    pub fn extract_representative_image(&self, images: &[Vec<u8>]) -> Option<Vec<u8>> {
        if images.is_empty() {
            return None;
        }

        let target_index = if images.len() <= 5 {
            0
        } else {
            std::cmp::min(2, images.len() - 1)
        };

        images.get(target_index).cloned()
    }
}

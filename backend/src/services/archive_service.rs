use crate::utils::{extractor::ArchiveExtractor, image::ImageProcessor};
use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

pub struct ArchiveService {
    extractor: ArchiveExtractor,
    image_processor: ImageProcessor,
}

impl ArchiveService {
    pub fn new() -> Self {
        Self {
            extractor: ArchiveExtractor::new(),
            image_processor: ImageProcessor::new(),
        }
    }

    pub async fn process_archive<P: AsRef<Path>>(&self, archive_path: P) -> Result<ArchiveInfo> {
        let path = archive_path.as_ref();
        info!("Processing archive: {}", path.display());

        let extracted_files = self
            .extractor
            .extract_files(path)
            .context("Failed to extract archive")?;

        let image_files = self.extractor.get_image_files(extracted_files);
        let page_count = image_files.len() as i32;

        let file_size = std::fs::metadata(path)
            .context("Failed to get file metadata")?
            .len() as i64;

        let hash = self
            .calculate_file_hash(path)
            .context("Failed to calculate file hash")?;

        let thumbnail = if let Some(first_image) = image_files.first() {
            Some(
                self.image_processor
                    .generate_thumbnail(&first_image.data, 300)?,
            )
        } else {
            None
        };

        Ok(ArchiveInfo {
            path: path.to_path_buf(),
            file_size,
            page_count,
            hash,
            thumbnail,
            images: image_files,
        })
    }

    pub async fn get_archive_page<P: AsRef<Path>>(
        &self,
        archive_path: P,
        page_index: usize,
    ) -> Result<Vec<u8>> {
        let path = archive_path.as_ref();

        let extracted_files = self
            .extractor
            .extract_files(path)
            .context("Failed to extract archive")?;

        let image_files = self.extractor.get_image_files(extracted_files);

        image_files
            .get(page_index)
            .map(|file| file.data.clone())
            .ok_or_else(|| anyhow::anyhow!("Page {} not found in archive", page_index))
    }

    pub async fn get_archive_thumbnail<P: AsRef<Path>>(&self, archive_path: P) -> Result<Vec<u8>> {
        let path = archive_path.as_ref();

        let extracted_files = self
            .extractor
            .extract_files(path)
            .context("Failed to extract archive")?;

        let image_files = self.extractor.get_image_files(extracted_files);

        if let Some(first_image) = image_files.first() {
            self.image_processor
                .generate_thumbnail(&first_image.data, 200)
        } else {
            Err(anyhow::anyhow!("No images found in archive"))
        }
    }

    pub fn calculate_file_hash<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        use sha2::{Digest, Sha256};
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn get_supported_formats() -> Vec<&'static str> {
        vec!["cbz", "cbr", "cb7", "zip", "rar", "7z"]
    }

    pub fn is_supported_format<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        Self::get_supported_formats().contains(&extension.as_str())
    }
}

#[derive(Debug)]
pub struct ArchiveInfo {
    pub path: std::path::PathBuf,
    pub file_size: i64,
    pub page_count: i32,
    pub hash: String,
    pub thumbnail: Option<Vec<u8>>,
    pub images: Vec<crate::utils::extractor::ExtractedFile>,
}

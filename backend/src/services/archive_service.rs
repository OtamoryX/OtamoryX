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

        // 提取文件
        info!("Extracting files from archive...");
        let extracted_files = match self.extractor.extract_files(path) {
            Ok(files) => {
                info!("Successfully extracted {} files", files.len());
                files
            }
            Err(e) => {
                tracing::error!("Failed to extract archive {}: {:?}", path.display(), e);
                return Err(e).context("Failed to extract archive");
            }
        };

        // 筛选图像文件
        info!("Filtering image files...");
        let image_files = self.extractor.get_image_files(extracted_files);
        let page_count = image_files.len() as i32;
        info!("Found {} image files", page_count);

        if page_count == 0 {
            tracing::warn!("No image files found in archive: {}", path.display());
        }

        // 获取文件元数据
        info!("Getting file metadata...");
        let file_size = match std::fs::metadata(path) {
            Ok(metadata) => {
                let size = metadata.len() as i64;
                info!("File size: {} bytes", size);
                size
            }
            Err(e) => {
                tracing::error!("Failed to get metadata for {}: {:?}", path.display(), e);
                return Err(anyhow::anyhow!(e)).context("Failed to get file metadata");
            }
        };

        // 计算文件哈希
        info!("Calculating file hash...");
        let hash = match self.calculate_file_hash(path) {
            Ok(h) => {
                info!("File hash: {}", h);
                h
            }
            Err(e) => {
                tracing::error!("Failed to calculate hash for {}: {:?}", path.display(), e);
                return Err(e).context("Failed to calculate file hash");
            }
        };

        // 生成缩略图
        info!("Generating thumbnail...");
        let thumbnail = if let Some(first_image) = image_files.first() {
            match self
                .image_processor
                .generate_thumbnail(&first_image.data, 300)
            {
                Ok(thumb) => {
                    info!("Successfully generated thumbnail ({} bytes)", thumb.len());
                    Some(thumb)
                }
                Err(e) => {
                    tracing::warn!("Failed to generate thumbnail: {:?}", e);
                    None
                }
            }
        } else {
            info!("No images available for thumbnail generation");
            None
        };

        info!("Archive processing completed successfully");
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

    // 获取封面文件路径
    pub(crate) fn get_cover_file_path(archive_path: &str) -> std::path::PathBuf {
        use std::path::Path;

        let path = Path::new(archive_path);
        // 使用文件名（不含扩展名）作为封面文件名的基础
        let cover_name = match path.file_stem() {
            Some(stem) => format!("{}_cover.jpg", stem.to_string_lossy()),
            None => "cover.jpg".to_string(),
        };

        if let Some(parent) = path.parent() {
            parent.join(cover_name)
        } else {
            // 如果无法获取父目录，在同级目录创建
            path.with_file_name(cover_name)
        }
    }
    // 为存档生成封面文件
    pub(crate) async fn generate_cover_file_for_archive(
        archive_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::utils::ArchiveExtractor;
        use image::{load_from_memory, GenericImageView, ImageFormat};
        use std::{fs, path::Path};

        let archive_path_buf = Path::new(archive_path);
        let cover_path = Self::get_cover_file_path(archive_path);

        // 如果封面文件已存在，跳过生成
        if cover_path.exists() {
            return Ok(());
        }

        // 提取存档的第一页
        let extractor = ArchiveExtractor::new();
        let files = extractor.extract_files(archive_path_buf)?;

        let image_files = extractor.get_image_files(files);

        if image_files.is_empty() {
            return Err("No image files found in archive".into());
        }

        // 按文件名排序并获取第一个
        let mut sorted_files = image_files;
        sorted_files.sort_by(|a, b| natord::compare(&a.name, &b.name));

        let first_image = &sorted_files[0];

        // 解码图片
        let img = load_from_memory(&first_image.data)?;

        // 计算缩略图尺寸（保持宽高比）
        let (original_width, original_height) = img.dimensions();
        let target_width = 300u32;
        let target_height = 400u32;

        // 计算缩放比例
        let width_ratio = target_width as f32 / original_width as f32;
        let height_ratio = target_height as f32 / original_height as f32;
        let scale = width_ratio.min(height_ratio);

        let new_width = (original_width as f32 * scale) as u32;
        let new_height = (original_height as f32 * scale) as u32;

        // 调整图片大小
        let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

        // 确保目录存在
        if let Some(parent) = cover_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 保存为JPEG文件
        resized.save_with_format(&cover_path, ImageFormat::Jpeg)?;

        tracing::info!("Generated cover file: {}", cover_path.display());
        Ok(())
    }

    // 创建占位符缩略图
    pub(crate) async fn create_placeholder_thumbnail(archive_id: &str) -> Result<Vec<u8>> {
        use image::{ImageBuffer, ImageFormat, Rgba};
        use std::io::Cursor;

        let width = 300u32;
        let height = 400u32;

        // 创建图片缓冲区
        let mut img = ImageBuffer::new(width, height);

        // 根据漫画ID生成不同的背景色
        let color_seed = archive_id.chars().map(|c| c as u32).sum::<u32>();
        let r = ((color_seed * 37) % 200 + 55) as u8;
        let g = ((color_seed * 73) % 200 + 55) as u8;
        let b = ((color_seed * 131) % 200 + 55) as u8;

        // 填充背景色
        for pixel in img.pixels_mut() {
            *pixel = Rgba([r, g, b, 255]);
        }

        // 将图片编码为JPEG
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        img.write_to(&mut cursor, ImageFormat::Jpeg)?;

        Ok(buffer)
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

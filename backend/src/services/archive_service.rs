use anyhow::{Context, Result};
use sqlx::{Pool, Row, Sqlite};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct ArchiveService;
const DEFAULT_COVER_QUALITY: u8 = 85;

impl ArchiveService {
    pub fn new() -> Self {
        Self
    }

    pub async fn process_archive<P: AsRef<Path>>(&self, archive_path: P) -> Result<ArchiveInfo> {
        let path = archive_path.as_ref().to_path_buf();
        info!("Processing archive: {}", path.display());

        // 提取文件、元数据、哈希、缩略图 — 全部为同步 I/O 或 CPU 密集型操作，
        // 放入 spawn_blocking 避免阻塞 tokio 工作线程
        let result_path = path.clone();
        let archive_info = tokio::task::spawn_blocking(move || -> Result<ArchiveInfo> {
            let extractor = crate::utils::extractor::ArchiveExtractor::new();
            let image_processor = crate::utils::image::ImageProcessor::new();

            // 提取文件
            info!("Extracting files from archive...");
            let extracted_files = match extractor.extract_files(&result_path) {
                Ok(files) => {
                    info!("Successfully extracted {} files", files.len());
                    files
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to extract archive {}: {:?}",
                        result_path.display(),
                        e
                    );
                    return Err(e).context("Failed to extract archive");
                }
            };

            // 筛选图像文件
            info!("Filtering image files...");
            let image_files = extractor.get_image_files(extracted_files);
            let page_count = image_files.len() as i32;
            info!("Found {} image files", page_count);

            if page_count == 0 {
                tracing::warn!("No image files found in archive: {}", result_path.display());
            }

            // 获取文件元数据
            info!("Getting file metadata...");
            let file_size = match std::fs::metadata(&result_path) {
                Ok(metadata) => {
                    let size = metadata.len() as i64;
                    info!("File size: {} bytes", size);
                    size
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to get metadata for {}: {:?}",
                        result_path.display(),
                        e
                    );
                    return Err(anyhow::anyhow!(e)).context("Failed to get file metadata");
                }
            };

            // 计算文件哈希
            info!("Calculating file hash...");
            let hash = {
                use sha2::{Digest, Sha256};
                use std::fs::File;
                use std::io::Read;

                let mut file = File::open(&result_path).with_context(|| {
                    format!("Failed to open file for hashing: {}", result_path.display())
                })?;
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; 8192];
                loop {
                    let bytes_read = file.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                format!("{:x}", hasher.finalize())
            };
            info!("File hash: {}", hash);

            // 生成缩略图
            info!("Generating thumbnail...");
            let thumbnail = if let Some(first_image) = image_files.first() {
                match image_processor.generate_thumbnail(&first_image.data, 300) {
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
                path: result_path,
                file_size,
                page_count,
                hash,
                thumbnail,
                images: image_files,
            })
        })
        .await
        .context("spawn_blocking task panicked")??;

        Ok(archive_info)
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

    pub(crate) async fn get_image_cache_path(pool: &Pool<Sqlite>) -> PathBuf {
        if let Ok(Some(row)) =
            sqlx::query("SELECT image_cache_path FROM system_settings WHERE id = 'default'")
                .fetch_optional(pool)
                .await
        {
            let cache_path: String = row.get("image_cache_path");
            if !cache_path.trim().is_empty() {
                return PathBuf::from(cache_path);
            }
        }

        std::env::var("CACHE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/cache"))
    }

    pub(crate) async fn get_cover_quality(pool: &Pool<Sqlite>) -> u8 {
        if let Ok(Some(row)) =
            sqlx::query("SELECT image_cache_quality FROM system_settings WHERE id = 'default'")
                .fetch_optional(pool)
                .await
        {
            if let Ok(quality) = row.try_get::<i64, _>("image_cache_quality") {
                return quality.clamp(1, 100) as u8;
            }
        }

        DEFAULT_COVER_QUALITY
    }

    // 获取封面文件路径（统一落在 cache/covers 目录）
    pub(crate) fn get_cover_file_path(cache_path: &Path, archive_id: &str) -> PathBuf {
        cache_path
            .join("covers")
            .join(format!("{}.jpg", archive_id))
    }

    // 为存档生成封面文件（从存档中提取第一张图片并保存为封面）
    pub(crate) async fn generate_cover_file_for_archive(
        archive_path: &str,
        cover_path: PathBuf,
        quality: u8,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let archive_path_str = archive_path.to_string();

        // 如果封面文件已存在，跳过生成
        if cover_path.exists() {
            return Ok(());
        }

        // 提取第一张图片的原始数据
        let first_image_data = tokio::task::spawn_blocking(
            move || -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
                use crate::utils::ArchiveExtractor;

                let extractor = ArchiveExtractor::new();
                let files = extractor.extract_files(std::path::Path::new(&archive_path_str))?;
                let mut image_files = extractor.get_image_files(files);

                if image_files.is_empty() {
                    return Err("No image files found in archive".into());
                }

                image_files.sort_by(|a, b| natord::compare(&a.name, &b.name));
                Ok(image_files.remove(0).data)
            },
        )
        .await??;

        // 复用 save_cover_from_bytes 进行缩放和保存
        Self::save_cover_from_bytes(&first_image_data, cover_path, quality).await
    }

    /// 从已提取的图片字节数据生成封面文件（避免重复解压存档）
    pub(crate) async fn save_cover_from_bytes(
        image_data: &[u8],
        cover_path: std::path::PathBuf,
        quality: u8,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if cover_path.exists() {
            return Ok(());
        }
        let image_data = image_data.to_vec();
        let quality = quality.clamp(1, 100);
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                use image::{codecs::jpeg::JpegEncoder, load_from_memory, GenericImageView};

                let img = load_from_memory(&image_data)?;
                let (original_width, original_height) = img.dimensions();
                let target_width = 300u32;
                let target_height = 400u32;
                let width_ratio = target_width as f32 / original_width as f32;
                let height_ratio = target_height as f32 / original_height as f32;
                let scale = width_ratio.min(height_ratio);
                let new_width = (original_width as f32 * scale) as u32;
                let new_height = (original_height as f32 * scale) as u32;
                let resized =
                    img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

                if let Some(parent) = cover_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut file = std::fs::File::create(&cover_path)?;
                let mut encoder = JpegEncoder::new_with_quality(&mut file, quality);
                encoder.encode_image(&resized)?;
                tracing::info!("Generated cover file: {}", cover_path.display());
                Ok(())
            },
        )
        .await??;
        Ok(())
    }

    // 创建占位符缩略图
    pub(crate) async fn create_placeholder_thumbnail(archive_id: &str) -> Result<Vec<u8>> {
        let archive_id = archive_id.to_string();
        tokio::task::spawn_blocking(move || {
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
        })
        .await
        .context("spawn_blocking task panicked")?
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

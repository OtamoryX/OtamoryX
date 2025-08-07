use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::Archive;
use crate::services::archive_service::{ArchiveInfo, ArchiveService};

pub struct ArchiveProcessingService {
    db: Pool<Sqlite>,
    archive_service: ArchiveService,
}

impl ArchiveProcessingService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            db,
            archive_service: ArchiveService::new(),
        }
    }

    pub fn get_db(&self) -> &Pool<Sqlite> {
        &self.db
    }

    pub async fn process_new_archive<P: AsRef<Path>>(&self, archive_path: P) -> Result<Archive> {
        let path = archive_path.as_ref();
        info!("Processing new archive: {}", path.display());

        // 检查文件格式
        debug!("Checking if format is supported for: {}", path.display());
        if !ArchiveService::is_supported_format(path) {
            warn!("Unsupported archive format: {}", path.display());
            return Err(anyhow::anyhow!(
                "Unsupported archive format: {}",
                path.display()
            ));
        }
        debug!("Archive format is supported");

        // 处理档案
        debug!("Starting archive processing...");
        let archive_info = match self.archive_service.process_archive(path).await {
            Ok(info) => {
                debug!(
                    "Archive processing successful - Pages: {}, Size: {} bytes, Hash: {}",
                    info.page_count, info.file_size, info.hash
                );
                info
            }
            Err(e) => {
                warn!("Archive processing failed: {:?}", e);
                return Err(e).context("Failed to process archive");
            }
        };

        // 检查重复
        debug!("Checking for duplicates...");
        let is_duplicate = match self.check_for_duplicates(&archive_info).await {
            Ok(dup) => {
                debug!("Duplicate check result: {}", dup);
                dup
            }
            Err(e) => {
                warn!("Failed to check for duplicates: {:?}", e);
                return Err(e).context("Failed to check for duplicates");
            }
        };

        if is_duplicate {
            warn!("Archive is a duplicate, skipping: {}", path.display());
            return Err(anyhow::anyhow!("Archive is a duplicate"));
        }

        // 创建档案记录
        debug!("Creating archive record in database...");
        let archive = match self.create_archive_record(&archive_info).await {
            Ok(archive) => {
                debug!(
                    "Archive record created successfully with ID: {}",
                    archive.id
                );
                archive
            }
            Err(e) => {
                warn!("Failed to create archive record: {:?}", e);
                return Err(e).context("Failed to create archive record");
            }
        };

        // 分配新标签
        debug!("Assigning 'new' tag to archive...");
        if let Err(e) = self.assign_new_tag(&archive.id).await {
            warn!("Failed to assign 'new' tag: {:?}", e);
            return Err(e).context("Failed to assign 'new' tag");
        }
        debug!("'new' tag assigned successfully");

        info!(
            "Successfully processed new archive: {} (ID: {})",
            archive.title, archive.id
        );
        Ok(archive)
    }

    pub async fn scan_directory<P: AsRef<Path>>(&self, directory: P) -> Result<Vec<Archive>> {
        let dir = directory.as_ref();
        info!("Scanning directory for new archives: {}", dir.display());

        let mut new_archives = Vec::new();
        let mut total_files = 0;
        let mut _processed_files = 0;

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total_files += 1;
                let path = entry.path();

                if ArchiveService::is_supported_format(path) {
                    match self.process_new_archive(path).await {
                        Ok(archive) => {
                            new_archives.push(archive);
                            _processed_files += 1;
                        }
                        Err(e) => {
                            debug!("Skipped file {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        info!(
            "Directory scan complete: {} new archives from {} total files",
            new_archives.len(),
            total_files
        );
        Ok(new_archives)
    }

    pub async fn remove_new_tag_from_archive(&self, archive_id: &str) -> Result<()> {
        debug!("Removing 'new' tag from archive: {}", archive_id);

        let new_tag_id = self.get_new_tag_id().await?;

        sqlx::query!(
            "DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?",
            archive_id,
            new_tag_id
        )
        .execute(&self.db)
        .await
        .context("Failed to remove 'new' tag")?;

        Ok(())
    }

    pub async fn get_new_archives(&self, limit: Option<u32>) -> Result<Vec<Archive>> {
        let limit = limit.unwrap_or(50).min(100) as i64;
        let new_tag_id = self.get_new_tag_id().await?;

        let rows = sqlx::query(
            r#"
            SELECT a.id, a.title, a.path, a.file_size, COALESCE(a.page_count, 0) as page_count, 
                   a.file_hash, a.created_at, a.updated_at
            FROM archives a
            INNER JOIN archive_tags at ON a.id = at.archive_id
            WHERE at.tag_id = ?
            ORDER BY a.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&new_tag_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch new archives")?;

        let archives = rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let title: String = row.get("title");
                let path: String = row.get("path");
                let file_size: i64 = row.get("file_size");
                let page_count: i32 = row.get("page_count");
                let hash: String = row.get("file_hash");
                let created_at: chrono::DateTime<Utc> = row.get("created_at");
                let updated_at: chrono::DateTime<Utc> = row.get("updated_at");

                Archive {
                    id,
                    title,
                    path,
                    file_size,
                    page_count,
                    hash,
                    created_at,
                    updated_at,
                    tags: vec![],
                }
            })
            .collect();

        Ok(archives)
    }

    async fn check_for_duplicates(&self, archive_info: &ArchiveInfo) -> Result<bool> {
        let hash_duplicate = sqlx::query!(
            "SELECT COUNT(*) as count FROM archives WHERE file_hash = ?",
            archive_info.hash
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to check for hash duplicates")?;

        if hash_duplicate.count > 0 {
            debug!("Found hash duplicate for: {}", archive_info.path.display());
            return Ok(true);
        }

        if let Some(filename) = archive_info.path.file_stem() {
            if let Some(filename_str) = filename.to_str() {
                let title_pattern = format!("%{}%", filename_str);
                let title_duplicate = sqlx::query!(
                    "SELECT COUNT(*) as count FROM archives WHERE title LIKE ?",
                    title_pattern
                )
                .fetch_one(&self.db)
                .await
                .context("Failed to check for title duplicates")?;

                if title_duplicate.count > 0 {
                    debug!("Found potential title duplicate for: {}", filename_str);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn create_archive_record(&self, archive_info: &ArchiveInfo) -> Result<Archive> {
        let archive_id = Uuid::new_v4().to_string();
        let title = archive_info
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Archive")
            .to_string();

        let path_str = archive_info.path.to_string_lossy().to_string();
        let now = Utc::now();

        // 生成封面文件
        if let Err(e) = self.generate_cover_file(&archive_info.path).await {
            warn!("Failed to generate cover file for {}: {}", path_str, e);
        }

        sqlx::query(
            r#"
            INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&archive_id)
        .bind(&title)
        .bind(&path_str)
        .bind(&archive_info.hash)
        .bind(archive_info.file_size)
        .bind(archive_info.page_count)
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await
        .context("Failed to insert archive record")?;

        Ok(Archive {
            id: archive_id,
            title,
            path: path_str,
            file_size: archive_info.file_size,
            page_count: archive_info.page_count,
            hash: archive_info.hash.clone(),
            created_at: now,
            updated_at: now,
            tags: vec![],
        })
    }

    async fn assign_new_tag(&self, archive_id: &str) -> Result<()> {
        let new_tag_id = self.get_new_tag_id().await?;

        sqlx::query!(
            "INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)",
            archive_id,
            new_tag_id
        )
        .execute(&self.db)
        .await
        .context("Failed to assign 'new' tag")?;

        Ok(())
    }

    async fn get_new_tag_id(&self) -> Result<String> {
        let tag = sqlx::query!("SELECT id FROM tags WHERE name = 'new' AND namespace = 'system'")
            .fetch_optional(&self.db)
            .await
            .context("Failed to fetch 'new' tag")?;

        if let Some(tag) = tag {
            Ok(tag.id.unwrap_or_default())
        } else {
            let tag_id = "new-tag-id".to_string();
            sqlx::query!(
                "INSERT INTO tags (id, name, namespace) VALUES (?, 'new', 'system')",
                tag_id
            )
            .execute(&self.db)
            .await
            .context("Failed to create 'new' tag")?;

            Ok(tag_id)
        }
    }

    pub async fn batch_process_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        batch_size: usize,
    ) -> Result<u32> {
        let dir = directory.as_ref();
        info!("Starting batch processing of directory: {}", dir.display());

        let mut total_processed = 0u32;
        let mut current_batch = Vec::new();

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();

                if ArchiveService::is_supported_format(path) {
                    current_batch.push(path.to_path_buf());

                    if current_batch.len() >= batch_size {
                        let processed = self.process_batch(&current_batch).await?;
                        total_processed += processed;
                        current_batch.clear();
                    }
                }
            }
        }

        if !current_batch.is_empty() {
            let processed = self.process_batch(&current_batch).await?;
            total_processed += processed;
        }

        info!(
            "Batch processing complete: {} archives processed",
            total_processed
        );
        Ok(total_processed)
    }

    async fn process_batch(&self, paths: &[PathBuf]) -> Result<u32> {
        let mut processed = 0u32;

        for path in paths {
            match self.process_new_archive(path).await {
                Ok(_) => {
                    processed += 1;
                }
                Err(e) => {
                    debug!("Failed to process {}: {}", path.display(), e);
                }
            }
        }

        Ok(processed)
    }

    async fn generate_cover_file(&self, archive_path: &Path) -> Result<()> {
        use crate::utils::ArchiveExtractor;
        use image::{load_from_memory, GenericImageView, ImageFormat};
        use std::fs;

        // 获取封面文件路径
        let cover_path = self.get_cover_file_path(archive_path);

        // 如果封面文件已存在，跳过生成
        if cover_path.exists() {
            debug!("Cover file already exists: {}", cover_path.display());
            return Ok(());
        }

        // 提取存档的第一页
        let extractor = ArchiveExtractor::new();
        let files = extractor
            .extract_files(archive_path)
            .context("Failed to extract archive for cover")?;

        let image_files = extractor.get_image_files(files);

        if image_files.is_empty() {
            return Err(anyhow::anyhow!("No image files found in archive"));
        }

        // 按文件名排序并获取第一个
        let mut sorted_files = image_files;
        sorted_files.sort_by(|a, b| natord::compare(&a.name, &b.name));

        let first_image = &sorted_files[0];

        // 解码图片
        let img = load_from_memory(&first_image.data).context("Failed to decode first image")?;

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
            fs::create_dir_all(parent).context("Failed to create cover directory")?;
        }

        // 保存为JPEG文件
        resized
            .save_with_format(&cover_path, ImageFormat::Jpeg)
            .context("Failed to save cover file")?;

        info!("Generated cover file: {}", cover_path.display());
        Ok(())
    }

    fn get_cover_file_path(&self, archive_path: &Path) -> PathBuf {
        // 使用文件名（不含扩展名）作为封面文件名的基础
        let cover_name = match archive_path.file_stem() {
            Some(stem) => format!("{}_cover.jpg", stem.to_string_lossy()),
            None => "cover.jpg".to_string(),
        };

        if let Some(parent) = archive_path.parent() {
            parent.join(cover_name)
        } else {
            // 如果无法获取父目录，在同级目录创建
            archive_path.with_file_name(cover_name)
        }
    }
}

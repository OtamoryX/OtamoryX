use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::Archive;
use crate::plugins::application as plugin_application;
use crate::services::ai_service::enqueue_title_translation;
use crate::services::archive::service::{ArchiveInfo, ArchiveService};
use crate::services::content_analysis::service::ContentAnalysisService;

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

        // 使用已提取的缩略图数据生成封面文件（避免重复解压存档）
        let cache_path = ArchiveService::get_image_cache_path(&self.db).await;
        let cover_path = ArchiveService::get_cover_file_path(&cache_path, &archive.id);
        let cover_quality = ArchiveService::get_cover_quality(&self.db).await;
        if let Some(ref thumbnail_data) = archive_info.thumbnail {
            if let Err(e) = ArchiveService::save_cover_from_bytes(
                thumbnail_data,
                cover_path.clone(),
                cover_quality,
            )
            .await
            {
                warn!("Failed to save cover file for {}: {}", path.display(), e);
            }
        } else if !archive_info.images.is_empty() {
            // 如果没有缩略图但有图片数据，用第一张图片生成封面
            let mut sorted_images = archive_info.images;
            sorted_images.sort_by(|a, b| natord::compare(&a.name, &b.name));
            if let Err(e) = ArchiveService::save_cover_from_bytes(
                &sorted_images[0].data,
                cover_path,
                cover_quality,
            )
            .await
            {
                warn!("Failed to save cover file for {}: {}", path.display(), e);
            }
        }

        // 分配新标签
        debug!("Assigning 'new' tag to archive...");
        if let Err(e) = self.assign_new_tag(&archive.id).await {
            warn!("Failed to assign 'new' tag: {:?}", e);
            return Err(e).context("Failed to assign 'new' tag");
        }
        debug!("'new' tag assigned successfully");

        plugin_application::auto_execute_enabled_metadata_plugins_for_archive(
            &self.db,
            &archive.id,
        )
        .await;
        debug!(
            "Auto metadata plugin execution finished for archive {}",
            archive.id
        );
        if let Err(err) = enqueue_title_translation(&self.db, &archive.id).await {
            warn!(
                "Failed to enqueue title translation for archive {}: {err:#}",
                archive.id
            );
        }
        if let Err(err) = ContentAnalysisService::new(self.db.clone())
            .enqueue_for_archive(&archive.id)
            .await
        {
            warn!(
                "Failed to enqueue content analysis for archive {}: {err:#}",
                archive.id
            );
        }

        info!(
            "Successfully processed new archive: {} (ID: {})",
            archive.title, archive.id
        );
        Ok(archive)
    }

    pub async fn scan_directory<P: AsRef<Path>>(&self, directory: P) -> Result<Vec<Archive>> {
        let dir = directory.as_ref();
        info!("Scanning directory for new archives: {}", dir.display());

        // === 阶段1: 预加载 DB 已知路径 ===
        let known_rows = sqlx::query("SELECT id, path FROM archives")
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch known archives")?;

        let id_by_path: HashMap<String, String> = known_rows
            .into_iter()
            .map(|r| {
                let path: String = r.get("path");
                let id: String = r.get("id");
                (path, id)
            })
            .collect();

        info!(
            "Loaded {} known archive paths from database",
            id_by_path.len()
        );

        // === 阶段2: 遍历目录，收集新文件 + 记录磁盘上存在的路径 ===
        let mut new_file_paths: Vec<PathBuf> = Vec::new();
        let mut found_paths: HashSet<String> = HashSet::new();

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !ArchiveService::is_supported_format(path) {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            found_paths.insert(path_str.clone());

            if !id_by_path.contains_key(&path_str) {
                new_file_paths.push(path.to_path_buf());
            }
        }

        let existing_count = found_paths.len() - new_file_paths.len();
        info!(
            "Directory walk complete: {} new files to process, {} already known",
            new_file_paths.len(),
            existing_count
        );

        // === 阶段3: 并发处理新文件 ===
        let concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        info!("Processing new files with concurrency: {}", concurrency);

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::new();

        for path in new_file_paths {
            let sem = semaphore.clone();
            let db = self.db.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let service = ArchiveProcessingService::new(db);
                service.process_new_archive(&path).await
            }));
        }

        let mut new_archives = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(archive)) => new_archives.push(archive),
                Ok(Err(e)) => debug!("Skipped: {}", e),
                Err(e) => warn!("Task panicked: {}", e),
            }
        }

        // === 阶段4: 清理磁盘上已删除的文件（仅当前扫描目录下的） ===
        let dir_prefix = dir.to_string_lossy().to_string();
        let missing_ids: Vec<String> = id_by_path
            .iter()
            .filter(|(path, _)| {
                path.starts_with(&dir_prefix) && !found_paths.contains(path.as_str())
            })
            .map(|(_, id)| id.clone())
            .collect();

        if !missing_ids.is_empty() {
            info!(
                "Removing {} missing archives from database",
                missing_ids.len()
            );
            for id in &missing_ids {
                let _ = sqlx::query("DELETE FROM archive_tags WHERE archive_id = ?")
                    .bind(id)
                    .execute(&self.db)
                    .await;
                let _ = sqlx::query("DELETE FROM archives WHERE id = ?")
                    .bind(id)
                    .execute(&self.db)
                    .await;
            }
        }

        // Rebuild once after a scan batch, never once per file. This stays fully local and
        // keeps newly imported archives visible in the collection view without AI usage.
        if !new_archives.is_empty() || !missing_ids.is_empty() {
            if let Err(err) = crate::services::collections::rebuild_collections(&self.db).await {
                warn!("Collection rebuild after scan failed: {err:#}");
            }
        }

        info!(
            "Scan complete: {} new, {} removed, {} total on disk",
            new_archives.len(),
            missing_ids.len(),
            found_paths.len()
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
            SELECT a.id, a.title, a.subtitle, a.subtitle_language, a.path, a.file_size, COALESCE(a.page_count, 0) as page_count,
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
                let subtitle: Option<String> = row.get("subtitle");
                let subtitle_language: Option<String> = row.get("subtitle_language");
                let path: String = row.get("path");
                let file_size: i64 = row.get("file_size");
                let page_count: i32 = row.get("page_count");
                let hash: String = row.get("file_hash");
                let created_at: chrono::DateTime<Utc> = row.get("created_at");
                let updated_at: chrono::DateTime<Utc> = row.get("updated_at");

                Archive {
                    id,
                    title,
                    subtitle,
                    subtitle_language,
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
            subtitle: None,
            subtitle_language: None,
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
}

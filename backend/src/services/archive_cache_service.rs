use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

use crate::utils::ArchiveExtractor;

#[derive(Debug, Clone)]
pub struct CachedPage {
    pub data: Vec<u8>,
    pub content_type: String,
    pub original_filename: String,
    pub last_accessed: Instant,
    pub access_count: u32,
    pub created_at: Instant,
    pub storage_location: CacheLocation,
}

#[derive(Debug, Clone)]
pub enum CacheLocation {
    Memory,
    Disk,
    Both, // Hybrid: exists in both memory and disk
}

#[derive(Debug)]
pub struct CachedArchive {
    pub pages: HashMap<u32, CachedPage>,
    pub total_pages: u32,
    pub last_accessed: Instant,
    pub size_bytes: usize,
}

impl CachedArchive {
    fn new() -> Self {
        Self {
            pages: HashMap::new(),
            total_pages: 0,
            last_accessed: Instant::now(),
            size_bytes: 0,
        }
    }

    fn add_page(
        &mut self,
        page_num: u32,
        data: Vec<u8>,
        content_type: String,
        original_filename: String,
        location: CacheLocation,
    ) -> usize {
        let new_size = data.len();
        let previous_size = self.pages.get(&page_num).map(|p| p.data.len()).unwrap_or(0);
        let now = Instant::now();
        let page = CachedPage {
            data,
            content_type,
            original_filename,
            last_accessed: now,
            access_count: 1,
            created_at: now,
            storage_location: location,
        };
        self.pages.insert(page_num, page);
        apply_size_delta(&mut self.size_bytes, previous_size, new_size);
        self.total_pages = self.total_pages.max(page_num);
        self.last_accessed = Instant::now();
        previous_size
    }

    fn get_page(&mut self, page_num: u32) -> Option<&CachedPage> {
        if let Some(page) = self.pages.get_mut(&page_num) {
            page.last_accessed = Instant::now();
            page.access_count += 1;
            self.last_accessed = Instant::now();
            Some(page)
        } else {
            None
        }
    }

    fn get_page_metadata(&self, page_num: u32) -> Option<&CachedPage> {
        self.pages.get(&page_num)
    }

    /// Calculate page priority score for cache eviction (higher = keep longer)
    fn calculate_page_priority(&self, page: &CachedPage) -> f64 {
        let now = Instant::now();
        let age_seconds = now.duration_since(page.created_at).as_secs_f64();
        let last_access_seconds = now.duration_since(page.last_accessed).as_secs_f64();

        // LFU component: access frequency (access_count / age)
        let frequency_score = page.access_count as f64 / age_seconds.max(1.0);

        // LRU component: recency (inverse of time since last access)
        let recency_score = 1.0 / (last_access_seconds + 1.0);

        // Combined score with configurable weights
        frequency_score * 0.6 + recency_score * 0.4
    }
}

fn apply_size_delta(total: &mut usize, old_size: usize, new_size: usize) {
    if new_size >= old_size {
        *total += new_size - old_size;
    } else {
        *total = total.saturating_sub(old_size - new_size);
    }
}

#[derive(Debug, Clone)]
pub enum CacheStrategy {
    Conservative,              // 保守策略：小内存，短TTL，少预加载
    Balanced,                  // 平衡策略：中等配置
    Aggressive,                // 激进策略：大内存，长TTL，多预加载
    Custom(CustomCacheConfig), // 自定义配置
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomCacheConfig {
    pub max_memory_mb: usize,
    pub max_cached_archives: usize,
    pub cache_ttl_hours: u32,
    pub preload_next_pages: u32,
    pub preload_prev_pages: u32,
    pub cleanup_threshold_percent: u32, // 内存使用达到多少百分比时开始清理
    pub enable_background_preload: bool,
    pub max_concurrent_extractions: usize,
    pub disk_cache_path: Option<String>,
    pub disk_cache_size_mb: usize,
    pub memory_to_disk_ratio: f32, // 0.0 = all disk, 1.0 = all memory
}

#[derive(Debug)]
pub struct ArchiveCacheConfig {
    pub max_memory_mb: usize,
    pub max_cached_archives: usize,
    pub cache_ttl: Duration,
    pub preload_next_pages: u32,
    pub preload_prev_pages: u32,
    pub cleanup_threshold_percent: u32,
    pub enable_background_preload: bool,
    pub max_concurrent_extractions: usize,
    pub disk_cache_path: Option<PathBuf>,
    pub disk_cache_size_mb: usize,
    pub memory_to_disk_ratio: f32,
}

impl ArchiveCacheConfig {
    /// Get cache path from database, with fallback to environment variable
    async fn get_cache_path_from_db(pool: &Pool<Sqlite>) -> PathBuf {
        // Try to get from database first
        if let Ok(Some(row)) =
            sqlx::query!("SELECT image_cache_path FROM system_settings WHERE id = 'default'")
                .fetch_optional(pool)
                .await
        {
            return PathBuf::from(row.image_cache_path);
        }

        // Fallback to environment variable, then default
        let cache_path = std::env::var("CACHE_PATH").unwrap_or_else(|_| "./data/cache".to_string());

        // Save to database for future use
        if let Err(e) = Self::save_cache_path_to_db(pool, &cache_path).await {
            tracing::warn!("Failed to save cache path to database: {}", e);
        }

        PathBuf::from(cache_path)
    }

    /// Save cache path to database
    async fn save_cache_path_to_db(pool: &Pool<Sqlite>, cache_path: &str) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO system_settings (id, image_cache_path, updated_at)
            VALUES ('default', ?, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                image_cache_path = excluded.image_cache_path,
                updated_at = excluded.updated_at
            "#,
            cache_path
        )
        .execute(pool)
        .await
        .context("Failed to save cache path to database")?;

        Ok(())
    }

    /// Legacy method for backwards compatibility
    fn get_cache_path() -> Option<PathBuf> {
        std::env::var("CACHE_PATH")
            .map(PathBuf::from)
            .ok()
            .or_else(|| Some(PathBuf::from("./data/cache")))
    }

    /// Create cache configuration from strategy with database-sourced cache path
    pub async fn from_strategy_with_db(strategy: CacheStrategy, pool: &Pool<Sqlite>) -> Self {
        let disk_cache_path = Some(Self::get_cache_path_from_db(pool).await);

        match strategy {
            CacheStrategy::Conservative => Self {
                max_memory_mb: 128,
                max_cached_archives: 10,
                cache_ttl: Duration::from_secs(900), // 15分钟
                preload_next_pages: 1,
                preload_prev_pages: 0,
                cleanup_threshold_percent: 70,
                enable_background_preload: false,
                max_concurrent_extractions: 1,
                disk_cache_path,
                disk_cache_size_mb: 256,
                memory_to_disk_ratio: 0.3, // 30% memory, 70% disk
            },
            CacheStrategy::Balanced => Self {
                max_memory_mb: 512,
                max_cached_archives: 30,
                cache_ttl: Duration::from_secs(3600), // 1小时
                preload_next_pages: 3,
                preload_prev_pages: 1,
                cleanup_threshold_percent: 80,
                enable_background_preload: true,
                max_concurrent_extractions: 2,
                disk_cache_path,
                disk_cache_size_mb: 1024,
                memory_to_disk_ratio: 0.5, // 50% memory, 50% disk
            },
            CacheStrategy::Aggressive => Self {
                max_memory_mb: 2048,
                max_cached_archives: 100,
                cache_ttl: Duration::from_secs(14400), // 4小时
                preload_next_pages: 10,
                preload_prev_pages: 5,
                cleanup_threshold_percent: 90,
                enable_background_preload: true,
                max_concurrent_extractions: 4,
                disk_cache_path,
                disk_cache_size_mb: 4096,
                memory_to_disk_ratio: 0.7, // 70% memory, 30% disk
            },
            CacheStrategy::Custom(custom) => Self {
                max_memory_mb: custom.max_memory_mb,
                max_cached_archives: custom.max_cached_archives,
                cache_ttl: Duration::from_secs((custom.cache_ttl_hours as u64) * 3600),
                preload_next_pages: custom.preload_next_pages,
                preload_prev_pages: custom.preload_prev_pages,
                cleanup_threshold_percent: custom.cleanup_threshold_percent,
                enable_background_preload: custom.enable_background_preload,
                max_concurrent_extractions: custom.max_concurrent_extractions,
                disk_cache_path,
                disk_cache_size_mb: custom.disk_cache_size_mb,
                memory_to_disk_ratio: custom.memory_to_disk_ratio,
            },
        }
    }

    /// Legacy method for backwards compatibility
    pub fn from_strategy(strategy: CacheStrategy) -> Self {
        match strategy {
            CacheStrategy::Conservative => Self {
                max_memory_mb: 128,
                max_cached_archives: 10,
                cache_ttl: Duration::from_secs(900), // 15分钟
                preload_next_pages: 1,
                preload_prev_pages: 0,
                cleanup_threshold_percent: 70,
                enable_background_preload: false,
                max_concurrent_extractions: 1,
                disk_cache_path: Self::get_cache_path(),
                disk_cache_size_mb: 256,
                memory_to_disk_ratio: 0.3, // 30% memory, 70% disk
            },
            CacheStrategy::Balanced => Self {
                max_memory_mb: 512,
                max_cached_archives: 30,
                cache_ttl: Duration::from_secs(3600), // 1小时
                preload_next_pages: 3,
                preload_prev_pages: 1,
                cleanup_threshold_percent: 80,
                enable_background_preload: true,
                max_concurrent_extractions: 2,
                disk_cache_path: Self::get_cache_path(),
                disk_cache_size_mb: 1024,
                memory_to_disk_ratio: 0.5, // 50% memory, 50% disk
            },
            CacheStrategy::Aggressive => Self {
                max_memory_mb: 2048,
                max_cached_archives: 100,
                cache_ttl: Duration::from_secs(14400), // 4小时
                preload_next_pages: 10,
                preload_prev_pages: 5,
                cleanup_threshold_percent: 90,
                enable_background_preload: true,
                max_concurrent_extractions: 4,
                disk_cache_path: Self::get_cache_path(),
                disk_cache_size_mb: 4096,
                memory_to_disk_ratio: 0.7, // 70% memory, 30% disk
            },
            CacheStrategy::Custom(custom) => Self {
                max_memory_mb: custom.max_memory_mb,
                max_cached_archives: custom.max_cached_archives,
                cache_ttl: Duration::from_secs((custom.cache_ttl_hours as u64) * 3600),
                preload_next_pages: custom.preload_next_pages,
                preload_prev_pages: custom.preload_prev_pages,
                cleanup_threshold_percent: custom.cleanup_threshold_percent,
                enable_background_preload: custom.enable_background_preload,
                max_concurrent_extractions: custom.max_concurrent_extractions,
                disk_cache_path: custom.disk_cache_path.map(PathBuf::from),
                disk_cache_size_mb: custom.disk_cache_size_mb,
                memory_to_disk_ratio: custom.memory_to_disk_ratio,
            },
        }
    }
}

impl Default for ArchiveCacheConfig {
    fn default() -> Self {
        Self::from_strategy(CacheStrategy::Balanced)
    }
}

pub struct ArchiveCacheService {
    cache: Arc<DashMap<String, CachedArchive>>,
    extractor: ArchiveExtractor,
    config: ArchiveCacheConfig,
    current_memory_usage: Arc<AtomicUsize>,
    current_disk_usage: Arc<AtomicUsize>,
    cache_hits: Arc<AtomicU64>,
    cache_misses: Arc<AtomicU64>,
    /// Limits concurrent archive extraction operations to prevent memory explosion.
    extraction_semaphore: Arc<tokio::sync::Semaphore>,
}

impl ArchiveCacheService {
    pub fn new(config: ArchiveCacheConfig) -> Self {
        // Ensure disk cache directory exists if configured
        if let Some(ref disk_path) = config.disk_cache_path {
            // Create both base cache directory and pages subdirectory
            let pages_path = disk_path.join("pages");
            if let Err(e) = std::fs::create_dir_all(&pages_path) {
                debug!(
                    "Failed to create disk cache directory {:?}: {}",
                    pages_path, e
                );
            }
        }

        let max_concurrent = config.max_concurrent_extractions.max(1);
        debug!(
            "Initializing ArchiveCacheService with max_concurrent_extractions={}",
            max_concurrent
        );

        Self {
            cache: Arc::new(DashMap::new()),
            extractor: ArchiveExtractor::new(),
            config,
            current_memory_usage: Arc::new(AtomicUsize::new(0)),
            current_disk_usage: Arc::new(AtomicUsize::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            extraction_semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent)),
        }
    }

    pub async fn get_page(
        &self,
        archive_id: &str,
        archive_path: &str,
        page_num: u32,
    ) -> Result<CachedPage> {
        if page_num == 0 {
            return Err(anyhow::anyhow!("Invalid page number 0 (must start from 1)"));
        }

        // Step 1: Check memory cache first (fast path)
        {
            if let Some(mut cached_archive) = self.cache.get_mut(archive_id) {
                if let Some(page) = cached_archive.get_page(page_num) {
                    if !page.data.is_empty() {
                        debug!(
                            "Memory cache hit for archive {} page {} ({}KB)",
                            archive_id,
                            page_num,
                            page.data.len() / 1024
                        );
                        self.cache_hits.fetch_add(1, Ordering::Relaxed);
                        return Ok(page.clone());
                    } else {
                        debug!(
                            "Found disk-only placeholder for archive {} page {} in memory cache, checking disk",
                            archive_id, page_num
                        );
                    }
                }
            }
        }

        // Step 2: Check disk cache
        if let Ok(Some((disk_data, original_filename))) =
            self.load_from_disk(archive_id, page_num).await
        {
            debug!(
                "Disk cache hit for archive {} page {} ({}KB, filename: {}, type: {})",
                archive_id,
                page_num,
                disk_data.len() / 1024,
                original_filename,
                self.get_content_type(&original_filename)
            );
            let content_type = self.get_content_type(&original_filename);
            let now = std::time::Instant::now();

            let mut cached_page = CachedPage {
                data: disk_data,
                content_type: content_type.clone(),
                original_filename: original_filename.clone(),
                last_accessed: now,
                access_count: 1,
                created_at: now,
                storage_location: CacheLocation::Disk,
            };

            // Optionally promote frequently accessed pages to memory
            if self.should_store_in_memory() {
                if let Some(mut cached_archive) = self.cache.get_mut(archive_id) {
                    if let Some(existing_page) = cached_archive.pages.get_mut(&page_num) {
                        let old_size = existing_page.data.len();
                        existing_page.data = cached_page.data.clone();
                        existing_page.storage_location = CacheLocation::Both;
                        existing_page.last_accessed = now;
                        existing_page.access_count += 1;
                        let new_size = existing_page.data.len();
                        apply_size_delta(&mut cached_archive.size_bytes, old_size, new_size);

                        if new_size >= old_size {
                            self.current_memory_usage.fetch_add(new_size - old_size, Ordering::Relaxed);
                        } else {
                            self.atomic_saturating_sub(&self.current_memory_usage, old_size - new_size);
                        }
                    } else {
                        let previous_size = cached_archive.add_page(
                            page_num,
                            cached_page.data.clone(),
                            content_type,
                            original_filename.clone(),
                            CacheLocation::Both,
                        );
                        let new_size = cached_page.data.len();
                        if new_size >= previous_size {
                            self.current_memory_usage.fetch_add(new_size - previous_size, Ordering::Relaxed);
                        } else {
                            self.atomic_saturating_sub(&self.current_memory_usage, previous_size - new_size);
                        }
                    }

                    cached_page.storage_location = CacheLocation::Both;
                    debug!(
                        "Promoted page {}/{} from disk to memory cache",
                        archive_id, page_num
                    );
                }
            }

            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached_page);
        }

        // Step 3: Cache miss -- acquire semaphore to limit concurrent extractions
        debug!(
            "Cache miss for archive {} page {}, acquiring extraction permit...",
            archive_id, page_num
        );
        self.cache_misses.fetch_add(1, Ordering::Relaxed);

        let _permit = self
            .extraction_semaphore
            .acquire()
            .await
            .map_err(|_| anyhow::anyhow!("Extraction semaphore closed"))?;

        // Double-check: another request may have extracted this page while we waited
        {
            if let Some(mut cached_archive) = self.cache.get_mut(archive_id) {
                if let Some(page) = cached_archive.get_page(page_num) {
                    if !page.data.is_empty() {
                        debug!(
                            "Cache hit after semaphore wait for archive {} page {}",
                            archive_id, page_num
                        );
                        return Ok(page.clone());
                    }
                }
            }
        }
        // Also double-check disk
        if let Ok(Some((disk_data, original_filename))) =
            self.load_from_disk(archive_id, page_num).await
        {
            debug!(
                "Disk cache hit after semaphore wait for archive {} page {}",
                archive_id, page_num
            );
            let content_type = self.get_content_type(&original_filename);
            let now = std::time::Instant::now();
            return Ok(CachedPage {
                data: disk_data,
                content_type,
                original_filename,
                last_accessed: now,
                access_count: 1,
                created_at: now,
                storage_location: CacheLocation::Disk,
            });
        }

        // Step 4: Single-page extraction (memory efficient -- only one page at a time)
        self.cleanup_if_needed().await;

        // page_num is 1-based in the API, but extract_single_page expects 0-based index
        let page_index = if page_num > 0 {
            (page_num - 1) as usize
        } else {
            0
        };

        let archive_path_owned = archive_path.to_string();
        let extracted = tokio::task::spawn_blocking(move || {
            ArchiveExtractor::extract_single_page(&archive_path_owned, page_index)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Extraction task join error: {}", e))??;

        let content_type = self.get_content_type(&extracted.name);
        let now = std::time::Instant::now();

        let cached_page = CachedPage {
            data: extracted.data.clone(),
            content_type: content_type.clone(),
            original_filename: extracted.name.clone(),
            last_accessed: now,
            access_count: 1,
            created_at: now,
            storage_location: CacheLocation::Memory,
        };

        // Cache the extracted page
        let should_memory = self.should_store_in_memory();
        let storage_location = if should_memory {
            CacheLocation::Memory
        } else {
            CacheLocation::Disk
        };

        if should_memory {
            let mut cached_archive = self.cache
                .entry(archive_id.to_string())
                .or_insert_with(CachedArchive::new);
            let previous_size = cached_archive.add_page(
                page_num,
                extracted.data.clone(),
                content_type.clone(),
                extracted.name.clone(),
                storage_location.clone(),
            );

            // Also query total pages if this is the first page for this archive
            if cached_archive.total_pages == 0 || cached_archive.total_pages < page_num {
                let path_for_count = archive_path.to_string();
                if let Ok((count, _)) = ArchiveExtractor::get_page_count(&path_for_count) {
                    cached_archive.total_pages = count as u32;
                }
            }

            let new_size = extracted.data.len();
            if new_size >= previous_size {
                self.current_memory_usage.fetch_add(new_size - previous_size, Ordering::Relaxed);
            } else {
                self.current_memory_usage.fetch_sub(previous_size - new_size, Ordering::Relaxed);
            }
        }

        // Store to disk cache regardless
        if let Err(e) = self
            .store_to_disk(archive_id, page_num, &extracted.data, &extracted.name)
            .await
        {
            debug!(
                "Failed to store page {}/{} to disk: {}",
                archive_id, page_num, e
            );
        }

        // Background preloading of adjacent pages
        if self.config.enable_background_preload {
            let cache_service = self.clone();
            let archive_id_clone = archive_id.to_string();
            let archive_path_clone = archive_path.to_string();
            tokio::spawn(async move {
                cache_service
                    .preload_adjacent_pages(&archive_id_clone, &archive_path_clone, page_num)
                    .await;
            });
        }

        Ok(cached_page)
    }

    /// Preload adjacent pages using single-page extraction (memory-efficient).
    async fn preload_adjacent_pages(
        &self,
        archive_id: &str,
        archive_path: &str,
        current_page: u32,
    ) {
        let next_preload = self.config.preload_next_pages;
        let prev_preload = self.config.preload_prev_pages;

        // Determine total pages
        let total_pages = self.cache.get(archive_id)
            .map(|a| a.total_pages)
            .unwrap_or(0);

        if total_pages == 0 {
            return;
        }

        let start_page = current_page.saturating_sub(prev_preload).max(1);
        let end_page = std::cmp::min(current_page + next_preload, total_pages);

        for page_num in start_page..=end_page {
            if page_num == current_page {
                continue;
            }

            // Check if already cached
            let already_cached = self.cache
                .get(archive_id)
                .map(|a| a.pages.contains_key(&page_num))
                .unwrap_or(false);

            if already_cached {
                continue;
            }

            // Check disk cache
            if let Ok(Some(_)) = self.load_from_disk(archive_id, page_num).await {
                continue;
            }

            // Acquire semaphore for preload extraction
            let permit = match self.extraction_semaphore.try_acquire() {
                Ok(p) => p,
                Err(_) => {
                    debug!(
                        "Skipping preload of page {}/{}: semaphore full",
                        archive_id, page_num
                    );
                    continue;
                }
            };

            let page_index = (page_num - 1) as usize;
            let archive_path_owned = archive_path.to_string();

            match tokio::task::spawn_blocking(move || {
                ArchiveExtractor::extract_single_page(&archive_path_owned, page_index)
            })
            .await
            {
                Ok(Ok(extracted)) => {
                    // Store to disk
                    if let Err(e) = self
                        .store_to_disk(archive_id, page_num, &extracted.data, &extracted.name)
                        .await
                    {
                        debug!(
                            "Failed to store preloaded page {}/{} to disk: {}",
                            archive_id, page_num, e
                        );
                    }

                    // Optionally store in memory
                    if self.should_store_in_memory() {
                        let content_type = self.get_content_type(&extracted.name);
                        let mut cached_archive = self.cache
                            .entry(archive_id.to_string())
                            .or_insert_with(CachedArchive::new);
                        let previous_size = cached_archive.add_page(
                            page_num,
                            extracted.data.clone(),
                            content_type,
                            extracted.name,
                            CacheLocation::Both,
                        );
                        let new_size = extracted.data.len();
                        if new_size >= previous_size {
                            self.current_memory_usage.fetch_add(new_size - previous_size, Ordering::Relaxed);
                        } else {
                            self.atomic_saturating_sub(&self.current_memory_usage, previous_size - new_size);
                        }
                    }

                    debug!("Preloaded page {}/{}", archive_id, page_num);
                }
                Ok(Err(e)) => {
                    debug!("Failed to preload page {}/{}: {}", archive_id, page_num, e);
                }
                Err(e) => {
                    debug!(
                        "Preload task join error for page {}/{}: {}",
                        archive_id, page_num, e
                    );
                }
            }

            drop(permit);
        }
    }

    async fn cleanup_if_needed(&self) {
        let current_memory = self.current_memory_usage.load(Ordering::Relaxed);

        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        let cleanup_threshold =
            (max_memory_bytes * self.config.cleanup_threshold_percent as usize) / 100;

        debug!(
            "Checking cache cleanup: current={} MB, max={} MB, threshold={} MB ({}%)",
            current_memory / 1024 / 1024,
            max_memory_bytes / 1024 / 1024,
            cleanup_threshold / 1024 / 1024,
            self.config.cleanup_threshold_percent
        );

        if current_memory > cleanup_threshold {
            debug!(
                "Cache cleanup triggered: {} MB used, threshold: {} MB",
                current_memory / 1024 / 1024,
                cleanup_threshold / 1024 / 1024
            );
            self.cleanup_old_entries().await;
        }

        // Also cleanup disk cache if needed
        if let Err(e) = self.cleanup_disk_cache().await {
            debug!("Failed to cleanup disk cache: {}", e);
        }
    }

    async fn cleanup_old_entries(&self) {
        let now = Instant::now();
        let mut freed_memory = 0usize;

        let mut page_priorities = Vec::new();

        // Calculate priorities for all pages across all archives
        for entry in self.cache.iter() {
            let archive_id = entry.key();
            let archive = entry.value();
            for (page_num, page) in &archive.pages {
                let priority = archive.calculate_page_priority(page);
                let page_size = if page.data.is_empty()
                    && matches!(page.storage_location, CacheLocation::Disk)
                {
                    1024 * 1024
                } else {
                    page.data.len()
                };
                page_priorities.push((archive_id.clone(), *page_num, priority, page_size));
            }
        }

        // Sort by priority (lowest first - these will be removed)
        page_priorities.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let target_memory = (self.config.max_memory_mb
            * 1024
            * 1024
            * self.config.cleanup_threshold_percent as usize)
            / 100;
        let current_memory = self.current_memory_usage.load(Ordering::Relaxed);
        let mut memory_to_free = current_memory.saturating_sub(target_memory);

        let mut archives_to_remove = Vec::new();

        // Remove lowest priority pages first
        for (archive_id, page_num, _priority, _page_size) in page_priorities {
            if memory_to_free == 0 {
                break;
            }

            if let Some(mut archive) = self.cache.get_mut(&archive_id) {
                if let Some(removed_page) = archive.pages.remove(&page_num) {
                    let memory_freed = if matches!(
                        removed_page.storage_location,
                        CacheLocation::Memory | CacheLocation::Both
                    ) {
                        removed_page.data.len()
                    } else {
                        0
                    };

                    archive.size_bytes = archive.size_bytes.saturating_sub(memory_freed);
                    freed_memory += memory_freed;
                    memory_to_free = memory_to_free.saturating_sub(memory_freed);

                    debug!(
                        "Removed page {}/{} from cache (priority: {:.3}, freed: {}KB)",
                        archive_id,
                        page_num,
                        _priority,
                        memory_freed / 1024
                    );

                    if archive.pages.is_empty() {
                        archives_to_remove.push(archive_id.clone());
                    }
                }
            }
        }

        // Remove empty archives
        for archive_id in &archives_to_remove {
            self.cache.remove(archive_id);
            debug!("Removed empty archive {} from cache", archive_id);
        }

        // Also remove archives that exceed TTL
        let mut ttl_expired = Vec::new();
        for entry in self.cache.iter() {
            if now.duration_since(entry.value().last_accessed) > self.config.cache_ttl {
                ttl_expired.push(entry.key().clone());
            }
        }

        for id in ttl_expired {
            if let Some((_, removed)) = self.cache.remove(&id) {
                debug!(
                    "Removed TTL-expired archive {} from cache ({}KB)",
                    id,
                    removed.size_bytes / 1024
                );
                freed_memory += removed.size_bytes;
            }
        }

        // Update memory usage statistics (saturating to avoid underflow)
        if freed_memory > 0 {
            self.atomic_saturating_sub(&self.current_memory_usage, freed_memory);
            debug!(
                "Intelligent cleanup freed {}KB from archive cache",
                freed_memory / 1024
            );
        }
    }

    /// Atomically subtract `val` from an AtomicUsize, saturating at zero to prevent underflow.
    fn atomic_saturating_sub(&self, atomic: &AtomicUsize, val: usize) {
        let _ = atomic.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_sub(val))
        });
    }

    fn get_content_type(&self, filename: &str) -> String {
        match filename
            .split('.')
            .last()
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "gif" => "image/gif".to_string(),
            "webp" => "image/webp".to_string(),
            "bmp" => "image/bmp".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    pub async fn get_archive_info(&self, archive_id: &str) -> Option<(u32, usize)> {
        self.cache
            .get(archive_id)
            .map(|archive| (archive.total_pages, archive.size_bytes))
    }

    pub async fn cache_stats(&self) -> HashMap<String, serde_json::Value> {
        let memory_usage = self.current_memory_usage.load(Ordering::Relaxed);

        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        let cached_archives = self.cache.len();

        debug!(
            "Cache stats requested: {} archives cached, {}MB memory used, {:.1}% hit rate ({}/{} requests)",
            cached_archives,
            memory_usage / 1024 / 1024,
            hit_rate * 100.0,
            hits,
            total_requests
        );

        let mut stats = HashMap::new();
        stats.insert(
            "cached_archives".to_string(),
            serde_json::Value::from(cached_archives),
        );
        stats.insert(
            "memory_usage_mb".to_string(),
            serde_json::Value::from(memory_usage / 1024 / 1024),
        );
        stats.insert(
            "max_memory_mb".to_string(),
            serde_json::Value::from(self.config.max_memory_mb),
        );
        stats.insert("hit_rate".to_string(), serde_json::Value::from(hit_rate));
        stats.insert("cache_hits".to_string(), serde_json::Value::from(hits));
        stats.insert("cache_misses".to_string(), serde_json::Value::from(misses));

        let total_pages: u32 = self.cache.iter().map(|entry| entry.value().total_pages).sum();
        stats.insert(
            "total_cached_pages".to_string(),
            serde_json::Value::from(total_pages),
        );

        stats
    }

    /// Clear all cached data
    pub async fn clear_all(&self) {
        let cache_size_before = self.cache.len();

        self.cache.clear();

        // Reset memory usage
        self.current_memory_usage.store(0, Ordering::Relaxed);

        debug!("Cleared {} archives from memory cache", cache_size_before);

        // Clear disk cache (pages directory)
        if let Some(ref cache_dir) = self.config.disk_cache_path {
            let pages_dir = cache_dir.join("pages");
            if let Ok(mut dir) = tokio::fs::read_dir(&pages_dir).await {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "cache" || ext == "metadata" {
                            let _ = tokio::fs::remove_file(entry.path()).await;
                        }
                    }
                }
            }
        }

        // Reset disk usage
        self.current_disk_usage.store(0, Ordering::Relaxed);

        // Reset stats
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);

        debug!("Cleared all cache data");
    }

    /// Generate disk cache file path for a page
    fn get_disk_cache_path(&self, archive_id: &str, page_num: u32) -> Option<PathBuf> {
        self.config.disk_cache_path.as_ref().map(|base_path| {
            base_path
                .join("pages")
                .join(format!("{}_page_{}.cache", archive_id, page_num))
        })
    }

    /// Check if should store in memory based on cache ratio and current usage
    fn should_store_in_memory(&self) -> bool {
        if self.config.memory_to_disk_ratio >= 1.0 {
            debug!("Using memory-only storage (ratio >= 1.0)");
            return true;
        }
        if self.config.memory_to_disk_ratio <= 0.0 {
            debug!("Using disk-only storage (ratio <= 0.0)");
            return false;
        }

        let current_memory = self.current_memory_usage.load(Ordering::Relaxed);
        let max_memory = (self.config.max_memory_mb * 1024 * 1024) as f32;
        let memory_usage_ratio = current_memory as f32 / max_memory;

        let should_use_memory = memory_usage_ratio < self.config.memory_to_disk_ratio;

        debug!(
            "Memory storage decision: current={}MB ({:.1}%), max={}MB, ratio_threshold={:.1}% -> {}",
            current_memory / 1024 / 1024,
            memory_usage_ratio * 100.0,
            self.config.max_memory_mb,
            self.config.memory_to_disk_ratio * 100.0,
            if should_use_memory { "MEMORY" } else { "DISK" }
        );

        // Store in memory if we haven't reached the configured ratio threshold
        should_use_memory
    }

    /// Store page data to disk cache
    async fn store_to_disk(
        &self,
        archive_id: &str,
        page_num: u32,
        data: &[u8],
        original_filename: &str,
    ) -> Result<()> {
        if let Some(cache_path) = self.get_disk_cache_path(archive_id, page_num) {
            tokio::fs::write(&cache_path, data)
                .await
                .with_context(|| format!("Failed to write page to disk cache: {:?}", cache_path))?;

            // Store the original filename alongside the cached data
            let metadata_path = cache_path.with_extension("metadata");
            tokio::fs::write(&metadata_path, original_filename)
                .await
                .with_context(|| {
                    format!("Failed to write filename metadata: {:?}", metadata_path)
                })?;

            // Update disk usage
            self.current_disk_usage.fetch_add(data.len() + original_filename.len(), Ordering::Relaxed);

            debug!(
                "Stored page {}/{} to disk cache ({} bytes, filename: {})",
                archive_id,
                page_num,
                data.len(),
                original_filename
            );
        }
        Ok(())
    }

    /// Load page data from disk cache
    async fn load_from_disk(
        &self,
        archive_id: &str,
        page_num: u32,
    ) -> Result<Option<(Vec<u8>, String)>> {
        if let Some(cache_path) = self.get_disk_cache_path(archive_id, page_num) {
            if cache_path.exists() {
                let data = tokio::fs::read(&cache_path)
                    .await
                    .with_context(|| format!("Failed to read from disk cache: {:?}", cache_path))?;

                // Load the original filename
                let metadata_path = cache_path.with_extension("metadata");
                let original_filename = if metadata_path.exists() {
                    tokio::fs::read_to_string(&metadata_path)
                        .await
                        .unwrap_or_else(|_| format!("page_{}.jpg", page_num)) // Fallback to old format
                } else {
                    format!("page_{}.jpg", page_num) // Fallback for old cached files
                };

                debug!(
                    "Loaded page {}/{} from disk cache ({} bytes, filename: {})",
                    archive_id,
                    page_num,
                    data.len(),
                    original_filename
                );
                return Ok(Some((data, original_filename)));
            }
        }
        Ok(None)
    }

    /// Clean up disk cache when size limit is exceeded
    async fn cleanup_disk_cache(&self) -> Result<()> {
        if let Some(ref cache_dir) = self.config.disk_cache_path {
            let max_size = (self.config.disk_cache_size_mb * 1024 * 1024) as u64;
            let pages_dir = cache_dir.join("pages");

            // Get all cache files with their metadata
            let mut entries = Vec::new();
            if let Ok(mut dir) = tokio::fs::read_dir(&pages_dir).await {
                while let Some(entry) = dir.next_entry().await? {
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_file() {
                            entries.push((entry.path(), metadata));
                        }
                    }
                }
            }

            // Sort by last accessed time (oldest first)
            entries
                .sort_by_key(|(_, metadata)| metadata.accessed().unwrap_or(std::time::UNIX_EPOCH));

            let mut current_size: u64 = entries.iter().map(|(_, m)| m.len()).sum();

            // Remove oldest files until under the limit
            for (path, metadata) in entries {
                if current_size <= max_size {
                    break;
                }

                if let Err(e) = tokio::fs::remove_file(&path).await {
                    debug!("Failed to remove disk cache file {:?}: {}", path, e);
                } else {
                    current_size -= metadata.len();
                    debug!(
                        "Removed disk cache file {:?} ({} bytes)",
                        path,
                        metadata.len()
                    );

                    // Also remove corresponding metadata file if it exists
                    if path.extension() == Some(std::ffi::OsStr::new("cache")) {
                        let metadata_path = path.with_extension("metadata");
                        let _ = tokio::fs::remove_file(&metadata_path).await;
                    }
                }
            }

            // Update disk usage counter
            self.current_disk_usage.store(current_size as usize, Ordering::Relaxed);
        }

        Ok(())
    }
}

impl Clone for ArchiveCacheService {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            extractor: ArchiveExtractor::new(),
            config: ArchiveCacheConfig {
                max_memory_mb: self.config.max_memory_mb,
                max_cached_archives: self.config.max_cached_archives,
                cache_ttl: self.config.cache_ttl,
                preload_next_pages: self.config.preload_next_pages,
                preload_prev_pages: self.config.preload_prev_pages,
                cleanup_threshold_percent: self.config.cleanup_threshold_percent,
                enable_background_preload: self.config.enable_background_preload,
                max_concurrent_extractions: self.config.max_concurrent_extractions,
                disk_cache_path: self.config.disk_cache_path.clone(),
                disk_cache_size_mb: self.config.disk_cache_size_mb,
                memory_to_disk_ratio: self.config.memory_to_disk_ratio,
            },
            current_memory_usage: Arc::clone(&self.current_memory_usage),
            current_disk_usage: Arc::clone(&self.current_disk_usage),
            cache_hits: Arc::clone(&self.cache_hits),
            cache_misses: Arc::clone(&self.cache_misses),
            extraction_semaphore: Arc::clone(&self.extraction_semaphore),
        }
    }
}

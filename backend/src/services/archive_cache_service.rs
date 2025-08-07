use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock as AsyncRwLock;
use tracing::debug;

use crate::utils::ArchiveExtractor;

#[derive(Debug, Clone)]
pub struct CachedPage {
    pub data: Vec<u8>,
    pub content_type: String,
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
        location: CacheLocation,
    ) {
        self.size_bytes += data.len();
        let now = Instant::now();
        let page = CachedPage {
            data,
            content_type,
            last_accessed: now,
            access_count: 1,
            created_at: now,
            storage_location: location,
        };
        self.pages.insert(page_num, page);
        self.total_pages = self.total_pages.max(page_num);
        self.last_accessed = Instant::now();
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
    fn get_default_cache_path() -> Option<PathBuf> {
        std::env::var("CACHE_PATH")
            .or_else(|_| std::env::var("DISK_CACHE_PATH"))
            .map(PathBuf::from)
            .ok()
            .or_else(|| Some(PathBuf::from("./cache")))
    }

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
                disk_cache_path: Self::get_default_cache_path(),
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
                disk_cache_path: Self::get_default_cache_path(),
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
                disk_cache_path: Self::get_default_cache_path(),
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
    cache: Arc<AsyncRwLock<HashMap<String, CachedArchive>>>,
    extractor: ArchiveExtractor,
    config: ArchiveCacheConfig,
    current_memory_usage: Arc<RwLock<usize>>,
    current_disk_usage: Arc<RwLock<usize>>,
}

impl ArchiveCacheService {
    pub fn new(config: ArchiveCacheConfig) -> Self {
        // Ensure disk cache directory exists if configured
        if let Some(ref disk_path) = config.disk_cache_path {
            if let Err(e) = std::fs::create_dir_all(disk_path) {
                debug!(
                    "Failed to create disk cache directory {:?}: {}",
                    disk_path, e
                );
            }
        }

        Self {
            cache: Arc::new(AsyncRwLock::new(HashMap::new())),
            extractor: ArchiveExtractor::new(),
            config,
            current_memory_usage: Arc::new(RwLock::new(0)),
            current_disk_usage: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get_page(
        &self,
        archive_id: &str,
        archive_path: &str,
        page_num: u32,
    ) -> Result<CachedPage> {
        // Step 1: Check memory cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(cached_archive) = cache.get_mut(archive_id) {
                if let Some(page) = cached_archive.get_page(page_num) {
                    debug!(
                        "Memory cache hit for archive {} page {}",
                        archive_id, page_num
                    );
                    return Ok(page.clone());
                }
            }
        }

        // Step 2: Check disk cache
        if let Ok(Some(disk_data)) = self.load_from_disk(archive_id, page_num).await {
            let content_type = self.get_content_type(&format!("page_{}.jpg", page_num));
            let now = std::time::Instant::now();

            let mut cached_page = CachedPage {
                data: disk_data,
                content_type: content_type.clone(),
                last_accessed: now,
                access_count: 1,
                created_at: now,
                storage_location: CacheLocation::Disk,
            };

            // Optionally promote frequently accessed pages to memory
            if self.should_store_in_memory() {
                let mut cache = self.cache.write().await;
                if let Some(cached_archive) = cache.get_mut(archive_id) {
                    // Update the existing disk entry to Both state
                    if let Some(existing_page) = cached_archive.pages.get_mut(&page_num) {
                        existing_page.data = cached_page.data.clone();
                        existing_page.storage_location = CacheLocation::Both;
                        existing_page.last_accessed = now;
                        existing_page.access_count += 1;
                    } else {
                        cached_archive.add_page(
                            page_num,
                            cached_page.data.clone(),
                            content_type,
                            CacheLocation::Both,
                        );
                    }

                    // Update memory usage
                    if let Ok(mut memory_usage) = self.current_memory_usage.write() {
                        *memory_usage += cached_page.data.len();
                    }

                    cached_page.storage_location = CacheLocation::Both;
                    debug!(
                        "Promoted page {}/{} from disk to memory cache",
                        archive_id, page_num
                    );
                }
            }

            debug!(
                "Disk cache hit for archive {} page {}",
                archive_id, page_num
            );
            return Ok(cached_page);
        }

        // Step 3: Extract from archive and cache
        debug!(
            "Cache miss for archive {} page {}, extracting...",
            archive_id, page_num
        );
        self.extract_and_cache_archive(archive_id, archive_path)
            .await?;

        // Step 4: Background preloading
        if self.config.enable_background_preload {
            let cache_service = self.clone();
            let archive_id_clone = archive_id.to_string();
            tokio::spawn(async move {
                cache_service
                    .preload_pages(&archive_id_clone, page_num)
                    .await;
            });
        }

        // Step 5: Retrieve from cache after extraction
        let mut cache = self.cache.write().await;
        if let Some(cached_archive) = cache.get_mut(archive_id) {
            if let Some(page) = cached_archive.get_page(page_num) {
                return Ok(page.clone());
            }
        }

        Err(anyhow::anyhow!(
            "Failed to cache and retrieve page {} from archive {}",
            page_num,
            archive_id
        ))
    }

    async fn extract_and_cache_archive(&self, archive_id: &str, archive_path: &str) -> Result<()> {
        // 检查是否需要清理缓存
        self.cleanup_if_needed().await;

        // 解压整个存档
        let files = self
            .extractor
            .extract_files(archive_path)
            .context("Failed to extract archive")?;

        let image_files = self.extractor.get_image_files(files);

        if image_files.is_empty() {
            return Err(anyhow::anyhow!("No image files found in archive"));
        }

        // 按文件名排序（确保页面顺序正确）
        let mut sorted_files = image_files;
        sorted_files.sort_by(|a, b| natord::compare(&a.name, &b.name));

        // 创建缓存条目
        let mut cached_archive = CachedArchive::new();

        for (index, file) in sorted_files.iter().enumerate() {
            let page_num = (index + 1) as u32;
            let content_type = self.get_content_type(&file.name);

            // Determine storage location based on cache strategy
            let should_memory = self.should_store_in_memory();
            let storage_location = if should_memory {
                CacheLocation::Memory
            } else {
                CacheLocation::Disk
            };

            // Store in memory cache
            if should_memory {
                cached_archive.add_page(
                    page_num,
                    file.data.clone(),
                    content_type.clone(),
                    storage_location.clone(),
                );
            }

            // Store to disk cache if configured
            if let CacheLocation::Disk = storage_location {
                if let Err(e) = self.store_to_disk(archive_id, page_num, &file.data).await {
                    debug!(
                        "Failed to store page {}/{} to disk: {}",
                        archive_id, page_num, e
                    );
                } else {
                    // Add metadata to memory cache for disk-stored pages
                    let disk_page = CachedPage {
                        data: vec![], // Empty data placeholder - actual data on disk
                        content_type: content_type.clone(),
                        last_accessed: std::time::Instant::now(),
                        access_count: 0, // Will be incremented when accessed
                        created_at: std::time::Instant::now(),
                        storage_location: CacheLocation::Disk,
                    };
                    cached_archive.pages.insert(page_num, disk_page);
                    cached_archive.total_pages = cached_archive.total_pages.max(page_num);
                }
            }
        }

        // Update memory usage statistics (for memory-cached and Both state pages)
        if !cached_archive.pages.is_empty() {
            let memory_size: usize = cached_archive
                .pages
                .values()
                .filter(|page| {
                    matches!(
                        page.storage_location,
                        CacheLocation::Memory | CacheLocation::Both
                    )
                })
                .map(|page| page.data.len())
                .sum();

            cached_archive.size_bytes = memory_size;

            if let Ok(mut memory_usage) = self.current_memory_usage.write() {
                *memory_usage += memory_size;
            }
        }

        debug!(
            "Cached {} pages for archive {}, total size: {}KB",
            cached_archive.total_pages,
            archive_id,
            cached_archive.size_bytes / 1024
        );

        // 存入缓存
        let mut cache = self.cache.write().await;
        cache.insert(archive_id.to_string(), cached_archive);

        Ok(())
    }

    async fn preload_pages(&self, archive_id: &str, current_page: u32) {
        let next_preload = self.config.preload_next_pages;
        let prev_preload = self.config.preload_prev_pages;

        debug!(
            "Preloading {} previous and {} next pages around page {} for archive {}",
            prev_preload, next_preload, current_page, archive_id
        );

        // 获取存档总页数
        let total_pages = {
            let cache = self.cache.read().await;
            cache
                .get(archive_id)
                .map(|archive| archive.total_pages)
                .unwrap_or(0)
        };

        if total_pages == 0 {
            debug!(
                "Archive {} not fully cached yet, skipping preload",
                archive_id
            );
            return;
        }

        // 计算预加载页面范围
        let start_page = if current_page > prev_preload {
            current_page - prev_preload
        } else {
            1
        };
        let end_page = std::cmp::min(current_page + next_preload, total_pages);

        // 检查哪些页面需要预加载（还没有被缓存的）
        let mut pages_to_preload = Vec::new();
        {
            let cache = self.cache.read().await;
            if let Some(cached_archive) = cache.get(archive_id) {
                for page_num in start_page..=end_page {
                    if page_num != current_page && !cached_archive.pages.contains_key(&page_num) {
                        pages_to_preload.push(page_num);
                    }
                }
            }
        }

        if !pages_to_preload.is_empty() {
            debug!(
                "Preloading pages {:?} for archive {}",
                pages_to_preload, archive_id
            );

            // 模拟预加载（实际上这些页面已经在extract_and_cache_archive中被缓存了）
            // 这里可以添加更智能的预测逻辑，比如基于用户阅读习惯
        }
    }

    async fn cleanup_if_needed(&self) {
        let current_memory = self
            .current_memory_usage
            .read()
            .map(|usage| *usage)
            .unwrap_or(0);

        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        let cleanup_threshold =
            (max_memory_bytes * self.config.cleanup_threshold_percent as usize) / 100;

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
        let mut cache = self.cache.write().await;
        let now = Instant::now();
        let mut freed_memory = 0;

        let mut page_priorities = Vec::new();

        // Calculate priorities for all pages across all archives
        for (archive_id, archive) in cache.iter() {
            for (page_num, page) in &archive.pages {
                let priority = archive.calculate_page_priority(page);
                // For disk-cached pages, estimate size or use a default value
                let page_size = if page.data.is_empty()
                    && matches!(page.storage_location, CacheLocation::Disk)
                {
                    // Estimate average page size for disk-cached pages (e.g., 1MB)
                    1024 * 1024
                } else {
                    page.data.len()
                };
                page_priorities.push((archive_id.clone(), *page_num, priority, page_size));
            }
        }

        // Sort by priority (lowest first - these will be removed)
        page_priorities.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let mut archives_to_remove = Vec::new();
        let target_memory = (self.config.max_memory_mb
            * 1024
            * 1024
            * self.config.cleanup_threshold_percent as usize)
            / 100;
        let current_memory = self
            .current_memory_usage
            .read()
            .map(|usage| *usage)
            .unwrap_or(0);
        let mut memory_to_free = current_memory.saturating_sub(target_memory);

        // Remove lowest priority pages first
        for (archive_id, page_num, _priority, page_size) in page_priorities {
            if memory_to_free == 0 {
                break;
            }

            if let Some(archive) = cache.get_mut(&archive_id) {
                if let Some(removed_page) = archive.pages.remove(&page_num) {
                    // Only count memory freed for pages that actually had data in memory
                    let memory_freed = if matches!(
                        removed_page.storage_location,
                        CacheLocation::Memory | CacheLocation::Both
                    ) {
                        removed_page.data.len()
                    } else {
                        0 // Disk-only pages don't free memory
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

                    // If archive has no pages left, mark for removal
                    if archive.pages.is_empty() {
                        archives_to_remove.push(archive_id.clone());
                    }
                }
            }
        }

        // Remove empty archives
        for archive_id in archives_to_remove {
            cache.remove(&archive_id);
            debug!("Removed empty archive {} from cache", archive_id);
        }

        // Also remove archives that exceed TTL
        let mut ttl_expired = Vec::new();
        for (id, archive) in cache.iter() {
            if now.duration_since(archive.last_accessed) > self.config.cache_ttl {
                ttl_expired.push(id.clone());
            }
        }

        for id in ttl_expired {
            if let Some(removed) = cache.remove(&id) {
                debug!(
                    "Removed TTL-expired archive {} from cache ({}KB)",
                    id,
                    removed.size_bytes / 1024
                );
                freed_memory += removed.size_bytes;
            }
        }

        // Update memory usage statistics
        if freed_memory > 0 {
            if let Ok(mut memory_usage) = self.current_memory_usage.write() {
                *memory_usage = memory_usage.saturating_sub(freed_memory);
            }
            debug!(
                "Intelligent cleanup freed {}KB from archive cache",
                freed_memory / 1024
            );
        }
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
        let cache = self.cache.read().await;
        cache
            .get(archive_id)
            .map(|archive| (archive.total_pages, archive.size_bytes))
    }

    pub async fn cache_stats(&self) -> HashMap<String, serde_json::Value> {
        let cache = self.cache.read().await;
        let memory_usage = self
            .current_memory_usage
            .read()
            .map(|usage| *usage)
            .unwrap_or(0);

        let mut stats = HashMap::new();
        stats.insert(
            "cached_archives".to_string(),
            serde_json::Value::from(cache.len()),
        );
        stats.insert(
            "memory_usage_mb".to_string(),
            serde_json::Value::from(memory_usage / 1024 / 1024),
        );
        stats.insert(
            "max_memory_mb".to_string(),
            serde_json::Value::from(self.config.max_memory_mb),
        );

        let total_pages: u32 = cache.values().map(|a| a.total_pages).sum();
        stats.insert(
            "total_cached_pages".to_string(),
            serde_json::Value::from(total_pages),
        );

        stats
    }

    /// Generate disk cache file path for a page
    fn get_disk_cache_path(&self, archive_id: &str, page_num: u32) -> Option<PathBuf> {
        self.config
            .disk_cache_path
            .as_ref()
            .map(|base_path| base_path.join(format!("{}_page_{}.cache", archive_id, page_num)))
    }

    /// Check if should store in memory based on cache ratio and current usage
    fn should_store_in_memory(&self) -> bool {
        if self.config.memory_to_disk_ratio >= 1.0 {
            return true;
        }
        if self.config.memory_to_disk_ratio <= 0.0 {
            return false;
        }

        let current_memory = self
            .current_memory_usage
            .read()
            .map(|usage| *usage)
            .unwrap_or(0);
        let max_memory = (self.config.max_memory_mb * 1024 * 1024) as f32;
        let memory_usage_ratio = current_memory as f32 / max_memory;

        // Store in memory if we haven't reached the configured ratio threshold
        memory_usage_ratio < self.config.memory_to_disk_ratio
    }

    /// Store page data to disk cache
    async fn store_to_disk(&self, archive_id: &str, page_num: u32, data: &[u8]) -> Result<()> {
        if let Some(cache_path) = self.get_disk_cache_path(archive_id, page_num) {
            tokio::fs::write(&cache_path, data)
                .await
                .with_context(|| format!("Failed to write page to disk cache: {:?}", cache_path))?;

            // Update disk usage
            {
                if let Ok(mut disk_usage) = self.current_disk_usage.write() {
                    *disk_usage += data.len();
                }
            }

            debug!(
                "Stored page {}/{} to disk cache ({} bytes)",
                archive_id,
                page_num,
                data.len()
            );
        }
        Ok(())
    }

    /// Load page data from disk cache
    async fn load_from_disk(&self, archive_id: &str, page_num: u32) -> Result<Option<Vec<u8>>> {
        if let Some(cache_path) = self.get_disk_cache_path(archive_id, page_num) {
            if cache_path.exists() {
                let data = tokio::fs::read(&cache_path)
                    .await
                    .with_context(|| format!("Failed to read from disk cache: {:?}", cache_path))?;
                debug!(
                    "Loaded page {}/{} from disk cache ({} bytes)",
                    archive_id,
                    page_num,
                    data.len()
                );
                return Ok(Some(data));
            }
        }
        Ok(None)
    }

    /// Clean up disk cache when size limit is exceeded
    async fn cleanup_disk_cache(&self) -> Result<()> {
        if let Some(ref cache_dir) = self.config.disk_cache_path {
            let max_size = (self.config.disk_cache_size_mb * 1024 * 1024) as u64;

            // Get all cache files with their metadata
            let mut entries = Vec::new();
            let mut dir = tokio::fs::read_dir(cache_dir).await?;

            while let Some(entry) = dir.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_file() {
                        entries.push((entry.path(), metadata));
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
                }
            }

            // Update disk usage counter
            {
                if let Ok(mut disk_usage) = self.current_disk_usage.write() {
                    *disk_usage = current_size as usize;
                }
            }
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
        }
    }
}

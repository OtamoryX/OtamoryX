use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use tracing::debug;
use tokio::sync::RwLock as AsyncRwLock;

use crate::utils::ArchiveExtractor;

#[derive(Debug, Clone)]
pub struct CachedPage {
    pub data: Vec<u8>,
    pub content_type: String,
    pub last_accessed: Instant,
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

    fn add_page(&mut self, page_num: u32, data: Vec<u8>, content_type: String) {
        self.size_bytes += data.len();
        let page = CachedPage {
            data,
            content_type,
            last_accessed: Instant::now(),
        };
        self.pages.insert(page_num, page);
        self.total_pages = self.total_pages.max(page_num);
        self.last_accessed = Instant::now();
    }

    fn get_page(&mut self, page_num: u32) -> Option<&CachedPage> {
        if let Some(page) = self.pages.get_mut(&page_num) {
            page.last_accessed = Instant::now();
            self.last_accessed = Instant::now();
            Some(page)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum CacheStrategy {
    Conservative,  // 保守策略：小内存，短TTL，少预加载
    Balanced,     // 平衡策略：中等配置
    Aggressive,   // 激进策略：大内存，长TTL，多预加载
    Custom(CustomCacheConfig),  // 自定义配置
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomCacheConfig {
    pub max_memory_mb: usize,
    pub max_cached_archives: usize,
    pub cache_ttl_hours: u32,
    pub preload_next_pages: u32,
    pub preload_prev_pages: u32,
    pub cleanup_threshold_percent: u32,  // 内存使用达到多少百分比时开始清理
    pub enable_background_preload: bool,
    pub max_concurrent_extractions: usize,
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
}

impl ArchiveCacheConfig {
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
}

impl ArchiveCacheService {
    pub fn new(config: ArchiveCacheConfig) -> Self {
        Self {
            cache: Arc::new(AsyncRwLock::new(HashMap::new())),
            extractor: ArchiveExtractor::new(),
            config,
            current_memory_usage: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get_page(&self, archive_id: &str, archive_path: &str, page_num: u32) -> Result<CachedPage> {
        // 首先检查缓存
        {
            let mut cache = self.cache.write().await;
            if let Some(cached_archive) = cache.get_mut(archive_id) {
                if let Some(page) = cached_archive.get_page(page_num) {
                    debug!("Cache hit for archive {} page {}", archive_id, page_num);
                    return Ok(page.clone());
                }
            }
        }

        // 缓存未命中，需要解压存档
        debug!("Cache miss for archive {} page {}, extracting...", archive_id, page_num);
        self.extract_and_cache_archive(archive_id, archive_path).await?;

        // 预加载页面（异步）
        if self.config.enable_background_preload {
            let cache_service = self.clone();
            let archive_id_clone = archive_id.to_string();
            tokio::spawn(async move {
                cache_service.preload_pages(&archive_id_clone, page_num).await;
            });
        }

        // 再次从缓存获取
        let mut cache = self.cache.write().await;
        if let Some(cached_archive) = cache.get_mut(archive_id) {
            if let Some(page) = cached_archive.get_page(page_num) {
                return Ok(page.clone());
            }
        }

        Err(anyhow::anyhow!("Failed to cache and retrieve page {} from archive {}", page_num, archive_id))
    }

    async fn extract_and_cache_archive(&self, archive_id: &str, archive_path: &str) -> Result<()> {
        // 检查是否需要清理缓存
        self.cleanup_if_needed().await;

        // 解压整个存档
        let files = self.extractor.extract_files(archive_path)
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
            cached_archive.add_page(page_num, file.data.clone(), content_type);
        }

        // 更新内存使用统计
        {
            let mut memory_usage = self.current_memory_usage.write().unwrap();
            *memory_usage += cached_archive.size_bytes;
        }

        debug!("Cached {} pages for archive {}, total size: {}KB", 
               cached_archive.total_pages, archive_id, cached_archive.size_bytes / 1024);

        // 存入缓存
        let mut cache = self.cache.write().await;
        cache.insert(archive_id.to_string(), cached_archive);

        Ok(())
    }

    async fn preload_pages(&self, archive_id: &str, current_page: u32) {
        let next_preload = self.config.preload_next_pages;
        let prev_preload = self.config.preload_prev_pages;
        
        debug!("Preloading {} previous and {} next pages around page {} for archive {}", 
               prev_preload, next_preload, current_page, archive_id);
        
        // 获取存档总页数
        let total_pages = {
            let cache = self.cache.read().await;
            cache.get(archive_id).map(|archive| archive.total_pages).unwrap_or(0)
        };
        
        if total_pages == 0 {
            debug!("Archive {} not fully cached yet, skipping preload", archive_id);
            return;
        }
        
        // 计算预加载页面范围
        let start_page = if current_page > prev_preload { current_page - prev_preload } else { 1 };
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
            debug!("Preloading pages {:?} for archive {}", pages_to_preload, archive_id);
            
            // 模拟预加载（实际上这些页面已经在extract_and_cache_archive中被缓存了）
            // 这里可以添加更智能的预测逻辑，比如基于用户阅读习惯
        }
    }

    async fn cleanup_if_needed(&self) {
        let current_memory = {
            let memory_usage = self.current_memory_usage.read().unwrap();
            *memory_usage
        };

        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        let cleanup_threshold = (max_memory_bytes * self.config.cleanup_threshold_percent as usize) / 100;
        
        if current_memory > cleanup_threshold {
            debug!("Cache cleanup triggered: {} MB used, threshold: {} MB", 
                   current_memory / 1024 / 1024, cleanup_threshold / 1024 / 1024);
            self.cleanup_old_entries().await;
        }
    }

    async fn cleanup_old_entries(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();
        
        // 收集需要清理的条目
        let mut to_remove = Vec::new();
        let mut entries: Vec<_> = cache.iter().collect();
        
        // 按最后访问时间排序
        entries.sort_by_key(|(_, archive)| archive.last_accessed);
        
        let mut freed_memory = 0;
        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        let current_memory = {
            let memory_usage = self.current_memory_usage.read().unwrap();
            *memory_usage
        };
        
        for (id, archive) in entries {
            // 清理超过TTL的条目
            if now.duration_since(archive.last_accessed) > self.config.cache_ttl {
                to_remove.push(id.clone());
                freed_memory += archive.size_bytes;
                continue;
            }
            
            // 如果内存使用仍然过高，清理最老的条目
            if current_memory - freed_memory > max_memory_bytes {
                to_remove.push(id.clone());
                freed_memory += archive.size_bytes;
            }
            
            // 限制缓存条目数量
            if cache.len() - to_remove.len() <= self.config.max_cached_archives {
                break;
            }
        }
        
        // 执行清理
        for id in to_remove {
            if let Some(removed) = cache.remove(&id) {
                debug!("Removed archive {} from cache ({}KB)", id, removed.size_bytes / 1024);
            }
        }
        
        // 更新内存使用统计
        {
            let mut memory_usage = self.current_memory_usage.write().unwrap();
            *memory_usage = *memory_usage - freed_memory;
        }
        
        if freed_memory > 0 {
            debug!("Freed {}KB from archive cache", freed_memory / 1024);
        }
    }

    fn get_content_type(&self, filename: &str) -> String {
        match filename.split('.').last().unwrap_or("").to_lowercase().as_str() {
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
        cache.get(archive_id).map(|archive| (archive.total_pages, archive.size_bytes))
    }

    pub async fn cache_stats(&self) -> HashMap<String, serde_json::Value> {
        let cache = self.cache.read().await;
        let memory_usage = {
            let memory_usage = self.current_memory_usage.read().unwrap();
            *memory_usage
        };
        
        let mut stats = HashMap::new();
        stats.insert("cached_archives".to_string(), serde_json::Value::from(cache.len()));
        stats.insert("memory_usage_mb".to_string(), serde_json::Value::from(memory_usage / 1024 / 1024));
        stats.insert("max_memory_mb".to_string(), serde_json::Value::from(self.config.max_memory_mb));
        
        let total_pages: u32 = cache.values().map(|a| a.total_pages).sum();
        stats.insert("total_cached_pages".to_string(), serde_json::Value::from(total_pages));
        
        stats
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
            },
            current_memory_usage: Arc::clone(&self.current_memory_usage),
        }
    }
}
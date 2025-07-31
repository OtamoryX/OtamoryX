use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProcessingTask {
    pub id: String,
    pub archive_id: String,
    pub task_type: TaskType,
    pub priority: i32,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub retry_count: i32,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    InitialProcessing,
    ThumbnailGeneration,
    MetadataExtraction,
    AIAnalysis,
    Reprocessing,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub comic_paths: Vec<PathBuf>,
    pub recursive: bool,
    pub ignore_hidden: bool,
    pub file_extensions: HashSet<String>,
    pub duplicate_detection: DuplicateDetectionConfig,
}

#[derive(Debug, Clone)]
pub struct DuplicateDetectionConfig {
    pub enable_hash_detection: bool,
    pub enable_title_detection: bool,
    pub title_similarity_threshold: f32,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub path: PathBuf,
    pub title: String,
    pub file_size: u64,
    pub hash: String,
    pub is_duplicate: bool,
    pub duplicate_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub index: usize,
    pub filename: String,
    pub size: usize,
    pub mime_type: String,
}

#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    pub sizes: Vec<ThumbnailSize>,
    pub quality: u8,
    pub format: ImageFormat,
    pub cache_path: PathBuf,
    pub max_cache_size: u64,
}

#[derive(Debug, Clone)]
pub struct ThumbnailSize {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub enum ImageFormat {
    JPEG,
    PNG,
    WebP,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub title: Option<String>,
    pub series: Option<String>,
    pub volume: Option<String>,
    pub chapter: Option<String>,
    pub authors: Vec<String>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<DateTime<Utc>>,
    pub language: Option<String>,
    pub page_count: Option<usize>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum WorkerType {
    MetadataExtraction,
    ThumbnailGeneration,
    AIAnalysis,
    General,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            comic_paths: vec![],
            recursive: true,
            ignore_hidden: true,
            file_extensions: ["cbz", "cbr", "cb7", "zip", "rar", "7z"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            duplicate_detection: DuplicateDetectionConfig {
                enable_hash_detection: true,
                enable_title_detection: true,
                title_similarity_threshold: 0.8,
            },
        }
    }
}
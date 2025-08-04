use chrono::Utc;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{ProcessingTask, ScanConfig, ScanResult, TaskStatus, TaskType, WorkerType};

/// 统一处理流水线
pub struct ProcessingPipeline {
    pub scanner: Arc<FileScanner>,
    pub processor_pool: Arc<ProcessorPool>,
    pub task_queue: Arc<TaskQueue>,
    pub storage: Arc<dyn Storage + Send + Sync>,
}

/// 文件扫描器
pub struct FileScanner {
    config: ScanConfig,
    db: sqlx::Pool<sqlx::Sqlite>,
}

/// 处理器池，包含各种处理器
pub struct ProcessorPool {
    pub metadata_extractors: Vec<Box<dyn MetadataExtractor + Send + Sync>>,
    pub thumbnail_generators: Vec<Box<dyn ThumbnailGenerator + Send + Sync>>,
    pub ai_analyzers: Vec<Box<dyn AIAnalyzer + Send + Sync>>,
}

/// 任务队列管理器
pub struct TaskQueue {
    /// 按优先级排序的待处理任务队列
    pub pending: Arc<Mutex<BTreeMap<i32, VecDeque<ProcessingTask>>>>,
    /// 正在处理的任务
    pub processing: Arc<Mutex<HashMap<String, ProcessingTask>>>,
    /// 已完成的任务历史
    pub completed: Arc<Mutex<Vec<ProcessingTask>>>,
    /// 失败的任务
    pub failed: Arc<Mutex<Vec<ProcessingTask>>>,
    /// 工作线程
    pub workers: Vec<TaskWorker>,
}

/// 工作线程
pub struct TaskWorker {
    pub id: String,
    pub worker_type: WorkerType,
    pub handle: tokio::task::JoinHandle<()>,
}

/// 存储抽象接口
#[async_trait::async_trait]
pub trait Storage {
    async fn create_archive_record(&self, scan_result: &ScanResult)
        -> Result<String, StorageError>;
    async fn assign_new_tag(&self, archive_id: &str) -> Result<(), StorageError>;
    async fn remove_new_tag(&self, archive_id: &str) -> Result<(), StorageError>;
    async fn archive_exists_by_hash(&self, hash: &str) -> Result<bool, StorageError>;
    async fn archive_exists_by_title(
        &self,
        title: &str,
        similarity_threshold: f32,
    ) -> Result<bool, StorageError>;
}

/// 元数据提取器接口
#[async_trait::async_trait]
pub trait MetadataExtractor {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
    async fn extract(&self, archive_path: &Path) -> Result<crate::models::Metadata, MetadataError>;
    fn supported_formats(&self) -> &[String];
}

/// 缩略图生成器接口
#[async_trait::async_trait]
pub trait ThumbnailGenerator {
    fn name(&self) -> &str;
    async fn generate_thumbnail(
        &self,
        archive_path: &Path,
        output_path: &Path,
    ) -> Result<(), ThumbnailError>;
    fn supported_formats(&self) -> &[String];
}

/// AI分析器接口
#[async_trait::async_trait]
pub trait AIAnalyzer {
    fn name(&self) -> &str;
    fn model_type(&self) -> crate::models::AIModelType;
    async fn analyze_archive(
        &self,
        archive_path: &Path,
    ) -> Result<crate::models::AIAnalysisResult, AIError>;
    async fn health_check(&self) -> Result<bool, AIError>;
    fn supports_format(&self, format: &str) -> bool;
}

// 错误类型定义
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("存储错误: {0}")]
    Storage(#[from] StorageError),
    #[error("扫描错误: {0}")]
    Scan(String),
    #[error("数据库错误: {0}")]
    DatabaseError(String),
    #[error("AI服务不可用")]
    AIServiceUnavailable,
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("记录未找到")]
    NotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("提取失败: {0}")]
    ExtractionFailed(String),
    #[error("不支持的格式: {format}")]
    UnsupportedFormat { format: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ThumbnailError {
    #[error("生成失败: {0}")]
    GenerationFailed(String),
    #[error("不支持的格式: {format}")]
    UnsupportedFormat { format: String },
}

#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("分析失败: {0}")]
    AnalysisFailed(String),
    #[error("模型不可用")]
    ModelUnavailable,
    #[error("网络错误: {0}")]
    Network(String),
}

impl ProcessingPipeline {
    pub fn new(
        scanner: FileScanner,
        processor_pool: ProcessorPool,
        task_queue: TaskQueue,
        storage: Arc<dyn Storage + Send + Sync>,
    ) -> Self {
        Self {
            scanner: Arc::new(scanner),
            processor_pool: Arc::new(processor_pool),
            task_queue: Arc::new(task_queue),
            storage,
        }
    }

    /// 处理单个存档文件
    pub async fn process_archive(&self, archive_path: &Path) -> Result<(), ProcessingError> {
        // 1. 文件扫描和重复检测
        let scan_result = self.scanner.scan_file(archive_path).await?;

        if scan_result.is_duplicate {
            tracing::info!("跳过重复文件: {}", archive_path.display());
            return Ok(());
        }

        // 2. 创建存档记录并分配"new"标签
        let archive_id = self.storage.create_archive_record(&scan_result).await?;
        self.storage.assign_new_tag(&archive_id).await?;

        // 3. 提交处理任务到队列
        let tasks = vec![
            ProcessingTask {
                id: Uuid::new_v4().to_string(),
                archive_id: archive_id.clone(),
                task_type: TaskType::MetadataExtraction,
                priority: 1,
                status: TaskStatus::Pending,
                created_at: Utc::now(),
                retry_count: 0,
            },
            ProcessingTask {
                id: Uuid::new_v4().to_string(),
                archive_id: archive_id.clone(),
                task_type: TaskType::ThumbnailGeneration,
                priority: 1,
                status: TaskStatus::Pending,
                created_at: Utc::now(),
                retry_count: 0,
            },
            ProcessingTask {
                id: Uuid::new_v4().to_string(),
                archive_id: archive_id.clone(),
                task_type: TaskType::AIAnalysis,
                priority: 0, // 较低优先级
                status: TaskStatus::Pending,
                created_at: Utc::now(),
                retry_count: 0,
            },
        ];

        for task in tasks {
            self.task_queue.enqueue(task).await;
        }

        tracing::info!("已为存档 {} 提交处理任务", archive_id);
        Ok(())
    }

    /// 批量扫描目录
    pub async fn scan_directory(&self, path: &Path) -> Result<Vec<PathBuf>, ProcessingError> {
        self.scanner.scan_directory(path).await
    }
}

impl FileScanner {
    pub fn new(config: ScanConfig, db: sqlx::Pool<sqlx::Sqlite>) -> Self {
        Self { config, db }
    }

    /// 扫描单个文件
    pub async fn scan_file(&self, path: &Path) -> Result<ScanResult, ProcessingError> {
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();

        // 计算文件哈希
        let hash = self.calculate_file_hash(path).await?;

        // 从文件名提取标题
        let title = self.extract_title_from_path(path);

        // 检查重复
        let is_duplicate = self.check_duplicate(&hash, &title).await?;
        let duplicate_reason = if is_duplicate {
            Some("Content hash match".to_string())
        } else {
            None
        };

        Ok(ScanResult {
            path: path.to_path_buf(),
            title,
            file_size,
            hash,
            is_duplicate,
            duplicate_reason,
        })
    }

    /// 扫描目录
    pub async fn scan_directory(&self, path: &Path) -> Result<Vec<PathBuf>, ProcessingError> {
        let mut comic_files = Vec::new();

        if self.config.recursive {
            let mut entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();

                if entry_path.is_dir() {
                    if self.config.ignore_hidden
                        && entry_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(|s| s.starts_with('.'))
                            .unwrap_or(false)
                    {
                        continue;
                    }

                    let mut sub_files = Box::pin(self.scan_directory(&entry_path)).await?;
                    comic_files.append(&mut sub_files);
                } else if self.is_supported_format(&entry_path) {
                    comic_files.push(entry_path);
                }
            }
        } else {
            let mut entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                if entry_path.is_file() && self.is_supported_format(&entry_path) {
                    comic_files.push(entry_path);
                }
            }
        }

        Ok(comic_files)
    }

    fn is_supported_format(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            self.config
                .file_extensions
                .contains(&extension.to_lowercase())
        } else {
            false
        }
    }

    async fn calculate_file_hash(&self, path: &Path) -> Result<String, ProcessingError> {
        use sha2::{Digest, Sha256};

        let content = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    fn extract_title_from_path(&self, path: &Path) -> String {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }

    async fn check_duplicate(&self, hash: &str, title: &str) -> Result<bool, ProcessingError> {
        // 检查是否有相同哈希的档案（强重复检测）
        let hash_duplicate = sqlx::query!(
            "SELECT COUNT(*) as count FROM archives WHERE file_hash = ?",
            hash
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ProcessingError::DatabaseError(e.to_string()))?;

        if hash_duplicate.count > 0 {
            return Ok(true);
        }

        // 检查是否有相似标题的档案（弱重复检测）
        let title_pattern = format!("%{}%", title);
        let title_duplicate = sqlx::query!(
            "SELECT COUNT(*) as count FROM archives WHERE title LIKE ?",
            title_pattern
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ProcessingError::DatabaseError(e.to_string()))?;

        Ok(title_duplicate.count > 0)
    }
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(BTreeMap::new())),
            processing: Arc::new(Mutex::new(HashMap::new())),
            completed: Arc::new(Mutex::new(Vec::new())),
            failed: Arc::new(Mutex::new(Vec::new())),
            workers: Vec::new(),
        }
    }

    /// 将任务加入队列
    pub async fn enqueue(&self, task: ProcessingTask) {
        let mut pending = self.pending.lock().await;
        pending
            .entry(task.priority)
            .or_insert_with(VecDeque::new)
            .push_back(task);
    }

    /// 从队列中取出最高优先级的任务
    pub async fn dequeue(&self) -> Option<ProcessingTask> {
        let mut pending = self.pending.lock().await;

        // 按优先级从高到低遍历
        for (_, queue) in pending.iter_mut().rev() {
            if let Some(mut task) = queue.pop_front() {
                task.status = TaskStatus::Processing;

                // 移动到处理中队列
                let mut processing = self.processing.lock().await;
                processing.insert(task.id.clone(), task.clone());

                return Some(task);
            }
        }

        None
    }

    /// 标记任务完成
    pub async fn mark_completed(&self, task_id: &str) {
        let mut processing = self.processing.lock().await;
        if let Some(mut task) = processing.remove(task_id) {
            task.status = TaskStatus::Completed;

            let mut completed = self.completed.lock().await;
            completed.push(task);
        }
    }

    /// 标记任务失败
    pub async fn mark_failed(&self, task_id: &str) {
        let mut processing = self.processing.lock().await;
        if let Some(mut task) = processing.remove(task_id) {
            task.status = TaskStatus::Failed;
            task.retry_count += 1;

            // 如果重试次数未超限，重新加入队列
            if task.retry_count < 3 {
                task.status = TaskStatus::Pending;
                drop(processing);
                self.enqueue(task).await;
            } else {
                let mut failed = self.failed.lock().await;
                failed.push(task);
            }
        }
    }
}

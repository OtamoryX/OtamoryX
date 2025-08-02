use async_trait::async_trait;
use axum::{
    extract::{Path, State, Query},
    response::Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use image::{DynamicImage, ImageFormat, ImageOutputFormat};
use imageproc::filter;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use uuid::Uuid;

/// 插件错误类型
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("图像处理失败: {0}")]
    ImageProcessing(String),
    #[error("文件操作失败: {0}")]
    FileOperation(String),
    #[error("数据库操作失败: {0}")]
    Database(String),
    #[error("配置错误: {0}")]
    Configuration(String),
}

/// 插件配置结构
#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub processing_modes: Vec<String>,
    pub output_format: String,
    pub quality_settings: QualitySettings,
    pub batch_size: usize,
    pub auto_process: bool,
    pub enhancement_settings: EnhancementSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QualitySettings {
    pub webp_quality: u8,
    pub jpeg_quality: u8,
    pub png_compression: u8,
    pub max_width: u32,
    pub max_height: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnhancementSettings {
    pub sharpen: bool,
    pub noise_reduction: bool,
    pub contrast_adjust: bool,
    pub brightness_adjust: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            processing_modes: vec!["optimize".to_string(), "enhance".to_string()],
            output_format: "webp".to_string(),
            quality_settings: QualitySettings {
                webp_quality: 85,
                jpeg_quality: 90,
                png_compression: 6,
                max_width: 2048,
                max_height: 2048,
            },
            batch_size: 10,
            auto_process: false,
            enhancement_settings: EnhancementSettings {
                sharpen: true,
                noise_reduction: true,
                contrast_adjust: true,
                brightness_adjust: false,
            },
        }
    }
}

/// 处理任务状态
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[repr(i32)]
pub enum ProcessingStatus {
    Pending = 0,
    InProgress = 1,
    Completed = 2,
    Failed = 3,
}

/// 处理任务记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProcessingJob {
    pub id: String,
    pub archive_id: String,
    pub processing_type: String,
    pub status: ProcessingStatus,
    pub progress: f32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 处理结果
#[derive(Debug, Clone, Serialize)]
pub struct ProcessingResult {
    pub original_size: u64,
    pub processed_size: u64,
    pub compression_ratio: f32,
    pub processing_time_ms: u64,
    pub pages_processed: usize,
    pub pages_failed: usize,
}

/// 图像处理器主类
pub struct ImageProcessorPlugin {
    config: Arc<PluginConfig>,
    db: Pool<Sqlite>,
}

impl ImageProcessorPlugin {
    pub fn new(config: PluginConfig, db: Pool<Sqlite>) -> Self {
        Self {
            config: Arc::new(config),
            db,
        }
    }

    /// 处理整个归档文件
    pub async fn process_archive(&self, archive_id: &str, archive_path: &str) -> Result<ProcessingResult, PluginError> {
        // 创建处理任务记录
        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO processing_jobs (id, archive_id, processing_type, status, progress, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&job_id)
        .bind(archive_id)
        .bind("image_processing")
        .bind(ProcessingStatus::InProgress)
        .bind(0.0)
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(|e| PluginError::Database(e.to_string()))?;

        let start_time = std::time::Instant::now();
        let mut total_original_size = 0u64;
        let mut total_processed_size = 0u64;
        let mut pages_processed = 0;
        let mut pages_failed = 0;

        // 提取归档文件（这里简化处理，实际应该调用归档服务）
        let temp_dir = format!("/tmp/otamoryx/extract/{}", archive_id);
        let output_dir = format!("/tmp/otamoryx/processed/{}", archive_id);
        
        // 创建输出目录
        fs::create_dir_all(&output_dir).await
            .map_err(|e| PluginError::FileOperation(e.to_string()))?;

        // 获取所有图像文件
        let image_files = self.find_image_files(&temp_dir).await?;
        let total_files = image_files.len();

        // 并行处理图像文件
        let results: Vec<_> = image_files
            .into_par_iter()
            .enumerate()
            .map(|(index, file_path)| {
                let output_path = PathBuf::from(&output_dir).join(
                    format!("page_{:04}.{}", index, self.config.output_format)
                );
                self.process_single_image(&file_path, &output_path)
            })
            .collect();

        // 处理结果并更新进度
        for (index, result) in results.into_iter().enumerate() {
            match result {
                Ok((original_size, processed_size)) => {
                    total_original_size += original_size;
                    total_processed_size += processed_size;
                    pages_processed += 1;
                }
                Err(_) => {
                    pages_failed += 1;
                }
            }

            // 更新进度
            let progress = (index + 1) as f32 / total_files as f32 * 100.0;
            self.update_job_progress(&job_id, progress).await?;
        }

        let processing_time = start_time.elapsed().as_millis() as u64;
        let compression_ratio = if total_original_size > 0 {
            (total_original_size - total_processed_size) as f32 / total_original_size as f32 * 100.0
        } else {
            0.0
        };

        // 完成处理任务
        sqlx::query(
            r#"
            UPDATE processing_jobs 
            SET status = ?, progress = ?, completed_at = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(if pages_failed == 0 { ProcessingStatus::Completed } else { ProcessingStatus::Failed })
        .bind(100.0)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(&job_id)
        .execute(&self.db)
        .await
        .map_err(|e| PluginError::Database(e.to_string()))?;

        Ok(ProcessingResult {
            original_size: total_original_size,
            processed_size: total_processed_size,
            compression_ratio,
            processing_time_ms: processing_time,
            pages_processed,
            pages_failed,
        })
    }

    /// 处理单个图像文件
    fn process_single_image(&self, input_path: &PathBuf, output_path: &PathBuf) -> Result<(u64, u64), PluginError> {
        // 读取原始文件大小
        let original_size = std::fs::metadata(input_path)
            .map_err(|e| PluginError::FileOperation(e.to_string()))?
            .len();

        // 加载图像
        let mut img = image::open(input_path)
            .map_err(|e| PluginError::ImageProcessing(e.to_string()))?;

        // 应用处理模式
        for mode in &self.config.processing_modes {
            match mode.as_str() {
                "optimize" => img = self.optimize_image(img)?,
                "enhance" => img = self.enhance_image(img)?,
                "resize" => img = self.resize_image(img)?,
                _ => {} // 忽略未知模式
            }
        }

        // 确定输出格式
        let format = match self.config.output_format.as_str() {
            "webp" => ImageFormat::WebP,
            "jpeg" => ImageFormat::Jpeg,
            "png" => ImageFormat::Png,
            _ => ImageFormat::WebP, // 默认使用 WebP
        };

        // 保存处理后的图像
        img.save_with_format(output_path, format)
            .map_err(|e| PluginError::ImageProcessing(e.to_string()))?;

        // 获取处理后文件大小
        let processed_size = std::fs::metadata(output_path)
            .map_err(|e| PluginError::FileOperation(e.to_string()))?
            .len();

        Ok((original_size, processed_size))
    }

    /// 优化图像（压缩、格式转换）
    fn optimize_image(&self, img: DynamicImage) -> Result<DynamicImage, PluginError> {
        let (width, height) = img.dimensions();
        let max_width = self.config.quality_settings.max_width;
        let max_height = self.config.quality_settings.max_height;

        // 如果图像尺寸超过限制，则缩小
        if width > max_width || height > max_height {
            let ratio = (max_width as f32 / width as f32).min(max_height as f32 / height as f32);
            let new_width = (width as f32 * ratio) as u32;
            let new_height = (height as f32 * ratio) as u32;
            
            Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
        } else {
            Ok(img)
        }
    }

    /// 增强图像（锐化、降噪、对比度调整）
    fn enhance_image(&self, img: DynamicImage) -> Result<DynamicImage, PluginError> {
        let mut img = img.to_rgb8();

        // 锐化
        if self.config.enhancement_settings.sharpen {
            let kernel = [-1.0, -1.0, -1.0,
                         -1.0,  9.0, -1.0,
                         -1.0, -1.0, -1.0];
            img = filter::filter3x3(&img, &kernel);
        }

        // 对比度调整
        if self.config.enhancement_settings.contrast_adjust {
            img = imageproc::contrast::stretch_contrast(&img, 1, 254);
        }

        // 亮度调整
        if self.config.enhancement_settings.brightness_adjust {
            for pixel in img.pixels_mut() {
                let [r, g, b] = pixel.0;
                pixel.0 = [
                    (r as f32 * 1.1).min(255.0) as u8,
                    (g as f32 * 1.1).min(255.0) as u8,
                    (b as f32 * 1.1).min(255.0) as u8,
                ];
            }
        }

        Ok(DynamicImage::ImageRgb8(img))
    }

    /// 调整图像尺寸
    fn resize_image(&self, img: DynamicImage) -> Result<DynamicImage, PluginError> {
        let (width, height) = img.dimensions();
        let max_width = self.config.quality_settings.max_width;
        let max_height = self.config.quality_settings.max_height;

        if width > max_width || height > max_height {
            let ratio = (max_width as f32 / width as f32).min(max_height as f32 / height as f32);
            let new_width = (width as f32 * ratio) as u32;
            let new_height = (height as f32 * ratio) as u32;
            
            Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
        } else {
            Ok(img)
        }
    }

    /// 查找目录中的所有图像文件
    async fn find_image_files(&self, dir_path: &str) -> Result<Vec<PathBuf>, PluginError> {
        let mut image_files = Vec::new();
        let mut entries = fs::read_dir(dir_path).await
            .map_err(|e| PluginError::FileOperation(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| PluginError::FileOperation(e.to_string()))? {
            
            let path = entry.path();
            if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                match extension.to_lowercase().as_str() {
                    "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tiff" => {
                        image_files.push(path);
                    }
                    _ => {}
                }
            }
        }

        // 按文件名排序
        image_files.sort();
        Ok(image_files)
    }

    /// 更新处理任务进度
    async fn update_job_progress(&self, job_id: &str, progress: f32) -> Result<(), PluginError> {
        sqlx::query("UPDATE processing_jobs SET progress = ?, updated_at = ? WHERE id = ?")
            .bind(progress)
            .bind(Utc::now())
            .bind(job_id)
            .execute(&self.db)
            .await
            .map_err(|e| PluginError::Database(e.to_string()))?;
        
        Ok(())
    }

    /// 创建自定义 API 路由
    pub fn create_routes(&self) -> Router {
        Router::new()
            .route("/process/:archive_id", post(process_archive_endpoint))
            .route("/jobs", get(list_jobs_endpoint))
            .route("/jobs/:job_id", get(get_job_endpoint))
            .route("/jobs/:job_id/cancel", post(cancel_job_endpoint))
            .with_state(Arc::new(self.clone()))
    }

    /// 定时任务：自动处理新归档
    pub async fn scheduled_auto_process(&self) -> Result<(), PluginError> {
        if !self.config.auto_process {
            return Ok(());
        }

        // 查找未处理的归档
        let unprocessed_archives = sqlx::query!(
            r#"
            SELECT a.id, a.file_path
            FROM archives a
            LEFT JOIN processing_jobs pj ON a.id = pj.archive_id AND pj.processing_type = 'image_processing'
            WHERE pj.id IS NULL
            LIMIT ?
            "#,
            self.config.batch_size as i64
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| PluginError::Database(e.to_string()))?;

        // 批量处理
        for archive in unprocessed_archives {
            if let Err(e) = self.process_archive(&archive.id, &archive.file_path).await {
                tracing::error!("自动处理归档 {} 失败: {}", archive.id, e);
            }
        }

        Ok(())
    }
}

impl Clone for ImageProcessorPlugin {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            db: self.db.clone(),
        }
    }
}

// API 端点实现

#[derive(Deserialize)]
struct ProcessArchiveRequest {
    force_reprocess: Option<bool>,
}

async fn process_archive_endpoint(
    State(plugin): State<Arc<ImageProcessorPlugin>>,
    Path(archive_id): Path<String>,
    Query(params): Query<ProcessArchiveRequest>,
) -> Result<Json<ProcessingResult>, StatusCode> {
    // 获取归档文件路径
    let archive = sqlx::query!("SELECT file_path FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&plugin.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 检查是否已经处理过
    if !params.force_reprocess.unwrap_or(false) {
        let existing_job = sqlx::query!(
            "SELECT id FROM processing_jobs WHERE archive_id = ? AND processing_type = 'image_processing' AND status = ?",
            archive_id,
            ProcessingStatus::Completed as i32
        )
        .fetch_optional(&plugin.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if existing_job.is_some() {
            return Err(StatusCode::CONFLICT);
        }
    }

    // 处理归档
    let result = plugin.process_archive(&archive_id, &archive.file_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

async fn list_jobs_endpoint(
    State(plugin): State<Arc<ImageProcessorPlugin>>,
) -> Result<Json<Vec<ProcessingJob>>, StatusCode> {
    let jobs = sqlx::query_as::<_, ProcessingJob>(
        "SELECT * FROM processing_jobs WHERE processing_type = 'image_processing' ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&plugin.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(jobs))
}

async fn get_job_endpoint(
    State(plugin): State<Arc<ImageProcessorPlugin>>,
    Path(job_id): Path<String>,
) -> Result<Json<ProcessingJob>, StatusCode> {
    let job = sqlx::query_as::<_, ProcessingJob>(
        "SELECT * FROM processing_jobs WHERE id = ?"
    )
    .bind(&job_id)
    .fetch_optional(&plugin.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(job))
}

async fn cancel_job_endpoint(
    State(plugin): State<Arc<ImageProcessorPlugin>>,
    Path(job_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // 将任务状态设置为失败（简化的取消实现）
    sqlx::query(
        "UPDATE processing_jobs SET status = ?, error_message = ?, updated_at = ? WHERE id = ? AND status = ?"
    )
    .bind(ProcessingStatus::Failed)
    .bind("用户取消")
    .bind(Utc::now())
    .bind(&job_id)
    .bind(ProcessingStatus::InProgress)
    .execute(&plugin.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

// 插件入口点
#[no_mangle]
pub extern "C" fn plugin_init() -> *mut ImageProcessorPlugin {
    // 这里应该从配置中初始化，为了简化示例直接使用默认配置
    // 实际使用中需要传入数据库连接池
    std::ptr::null_mut() // 简化实现
}

#[no_mangle]
pub extern "C" fn plugin_cleanup(plugin: *mut ImageProcessorPlugin) {
    if !plugin.is_null() {
        unsafe {
            Box::from_raw(plugin);
        }
    }
}
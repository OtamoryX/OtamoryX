use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, Row};
use std::collections::HashMap;
use uuid::Uuid;

/// 插件配置结构
#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub enabled_extractors: Vec<String>,
    pub tag_patterns: HashMap<String, String>,
    pub auto_tag_threshold: f64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("artist".to_string(), r"\[([^\]]+)\]".to_string());
        patterns.insert("series".to_string(), r"^([^(\[]+)".to_string());
        patterns.insert("language".to_string(), r"\b(chinese|english|japanese|korean)\b".to_string());
        
        Self {
            enabled_extractors: vec![
                "filename".to_string(),
                "directory".to_string(),
                "basic_tags".to_string(),
            ],
            tag_patterns: patterns,
            auto_tag_threshold: 0.8,
        }
    }
}

/// 提取的元数据结构
#[derive(Debug, Clone, Serialize)]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub series: Option<String>,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub confidence: f64,
}

/// 插件主要接口
#[async_trait]
pub trait MetadataExtractor {
    async fn extract_metadata(&self, archive_path: &str, config: &PluginConfig) -> Result<ExtractedMetadata, Box<dyn std::error::Error>>;
    async fn apply_metadata(&self, archive_id: &str, metadata: &ExtractedMetadata, db: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct MetadataExtractorPlugin {
    filename_extractor: FilenameExtractor,
    directory_extractor: DirectoryExtractor,
    basic_tags_extractor: BasicTagsExtractor,
}

impl MetadataExtractorPlugin {
    pub fn new() -> Self {
        Self {
            filename_extractor: FilenameExtractor::new(),
            directory_extractor: DirectoryExtractor::new(),
            basic_tags_extractor: BasicTagsExtractor::new(),
        }
    }
}

#[async_trait]
impl MetadataExtractor for MetadataExtractorPlugin {
    async fn extract_metadata(&self, archive_path: &str, config: &PluginConfig) -> Result<ExtractedMetadata, Box<dyn std::error::Error>> {
        let mut metadata = ExtractedMetadata {
            title: None,
            artist: None,
            series: None,
            language: None,
            tags: Vec::new(),
            confidence: 0.0,
        };

        let mut total_confidence = 0.0;
        let mut extractor_count = 0;

        // 文件名提取
        if config.enabled_extractors.contains(&"filename".to_string()) {
            if let Ok(filename_metadata) = self.filename_extractor.extract_metadata(archive_path, config).await {
                merge_metadata(&mut metadata, &filename_metadata);
                total_confidence += filename_metadata.confidence;
                extractor_count += 1;
            }
        }

        // 目录结构提取
        if config.enabled_extractors.contains(&"directory".to_string()) {
            if let Ok(directory_metadata) = self.directory_extractor.extract_metadata(archive_path, config).await {
                merge_metadata(&mut metadata, &directory_metadata);
                total_confidence += directory_metadata.confidence;
                extractor_count += 1;
            }
        }

        // 基础标签提取
        if config.enabled_extractors.contains(&"basic_tags".to_string()) {
            if let Ok(tags_metadata) = self.basic_tags_extractor.extract_metadata(archive_path, config).await {
                merge_metadata(&mut metadata, &tags_metadata);
                total_confidence += tags_metadata.confidence;
                extractor_count += 1;
            }
        }

        // 计算平均置信度
        if extractor_count > 0 {
            metadata.confidence = total_confidence / extractor_count as f64;
        }

        Ok(metadata)
    }

    async fn apply_metadata(&self, archive_id: &str, metadata: &ExtractedMetadata, db: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
        // 只在置信度足够高时应用元数据
        if metadata.confidence < 0.5 {
            return Ok(());
        }

        // 更新归档标题
        if let Some(title) = &metadata.title {
            sqlx::query("UPDATE archives SET title = ? WHERE id = ?")
                .bind(title)
                .bind(archive_id)
                .execute(db)
                .await?;
        }

        // 添加标签
        for tag_name in &metadata.tags {
            let tag_id = self.get_or_create_tag(tag_name, "auto", db).await?;
            
            // 检查标签是否已存在 (使用动态查询)
            let existing = sqlx::query("SELECT 1 FROM archive_tags WHERE archive_id = ? AND tag_id = ?")
                .bind(archive_id)
                .bind(&tag_id)
                .fetch_optional(db)
                .await?;

            if existing.is_none() {
                sqlx::query("INSERT INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
                    .bind(archive_id)
                    .bind(&tag_id)
                    .execute(db)
                    .await?;
            }
        }

        Ok(())
    }
}

impl MetadataExtractorPlugin {
    async fn get_or_create_tag(&self, name: &str, namespace: &str, db: &Pool<Sqlite>) -> Result<String, Box<dyn std::error::Error>> {
        // 尝试查找现有标签 (使用动态查询避免编译时数据库检查)
        let existing_tag = sqlx::query("SELECT id FROM tags WHERE name = ? AND namespace = ?")
            .bind(name)
            .bind(namespace)
            .fetch_optional(db)
            .await?;
            
        if let Some(row) = existing_tag {
            return Ok(row.get("id"));
        }

        // 创建新标签
        let tag_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO tags (id, name, namespace) VALUES (?, ?, ?)")
            .bind(&tag_id)
            .bind(name)
            .bind(namespace)
            .execute(db)
            .await?;

        Ok(tag_id)
    }
}

// 文件名提取器
pub struct FilenameExtractor;

impl FilenameExtractor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MetadataExtractor for FilenameExtractor {
    async fn extract_metadata(&self, archive_path: &str, config: &PluginConfig) -> Result<ExtractedMetadata, Box<dyn std::error::Error>> {
        let filename = std::path::Path::new(archive_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        let mut metadata = ExtractedMetadata {
            title: Some(filename.clone()),
            artist: None,
            series: None,
            language: None,
            tags: Vec::new(),
            confidence: 0.6,
        };

        // 使用正则表达式提取信息
        for (tag_type, pattern) in &config.tag_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(&filename) {
                    if let Some(match_str) = captures.get(1) {
                        let value = match_str.as_str().trim().to_lowercase();
                        match tag_type.as_str() {
                            "artist" => metadata.artist = Some(value),
                            "series" => metadata.series = Some(value),
                            "language" => metadata.language = Some(value),
                            _ => metadata.tags.push(format!("{}:{}", tag_type, value)),
                        }
                    }
                }
            }
        }

        // 基于提取信息的数量调整置信度
        let extracted_fields = [&metadata.artist, &metadata.series, &metadata.language]
            .iter()
            .filter(|field| field.is_some())
            .count();
        
        metadata.confidence = 0.4 + (extracted_fields as f64 * 0.1) + (metadata.tags.len() as f64 * 0.05);
        metadata.confidence = metadata.confidence.min(1.0);

        Ok(metadata)
    }

    async fn apply_metadata(&self, _archive_id: &str, _metadata: &ExtractedMetadata, _db: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
        // 由主插件处理
        Ok(())
    }
}

// 目录结构提取器
pub struct DirectoryExtractor;

impl DirectoryExtractor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MetadataExtractor for DirectoryExtractor {
    async fn extract_metadata(&self, archive_path: &str, _config: &PluginConfig) -> Result<ExtractedMetadata, Box<dyn std::error::Error>> {
        let path = std::path::Path::new(archive_path);
        let mut metadata = ExtractedMetadata {
            title: None,
            artist: None,
            series: None,
            language: None,
            tags: Vec::new(),
            confidence: 0.3,
        };

        // 从目录名提取系列信息
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name().and_then(|name| name.to_str()) {
                metadata.series = Some(parent_name.to_string());
                metadata.confidence += 0.2;
            }
        }

        // 根据路径深度和结构添加分类标签
        let path_components: Vec<&str> = path.components()
            .filter_map(|comp| comp.as_os_str().to_str())
            .collect();

        // 检查常见的分类目录名
        for component in &path_components {
            let component_lower = component.to_lowercase();
            if component_lower.contains("manga") {
                metadata.tags.push("type:manga".to_string());
            } else if component_lower.contains("comic") {
                metadata.tags.push("type:comic".to_string());
            } else if component_lower.contains("doujin") {
                metadata.tags.push("type:doujinshi".to_string());
            }
        }

        Ok(metadata)
    }

    async fn apply_metadata(&self, _archive_id: &str, _metadata: &ExtractedMetadata, _db: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
        // 由主插件处理
        Ok(())
    }
}

// 基础标签提取器
pub struct BasicTagsExtractor;

impl BasicTagsExtractor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MetadataExtractor for BasicTagsExtractor {
    async fn extract_metadata(&self, archive_path: &str, _config: &PluginConfig) -> Result<ExtractedMetadata, Box<dyn std::error::Error>> {
        let filename = std::path::Path::new(archive_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut metadata = ExtractedMetadata {
            title: None,
            artist: None,
            series: None,
            language: None,
            tags: Vec::new(),
            confidence: 0.7,
        };

        // 语言检测
        if filename.contains("chinese") || filename.contains("中文") || filename.contains("汉化") {
            metadata.language = Some("chinese".to_string());
            metadata.tags.push("language:chinese".to_string());
        } else if filename.contains("english") {
            metadata.language = Some("english".to_string());
            metadata.tags.push("language:english".to_string());
        } else if filename.contains("japanese") || filename.contains("日文") {
            metadata.language = Some("japanese".to_string());
            metadata.tags.push("language:japanese".to_string());
        }

        // 分辨率检测
        let resolution_patterns = vec![
            (r"(?i)(\d{3,4}x\d{3,4})", "resolution"),
            (r"(?i)(hd|高清)", "quality:hd"),
            (r"(?i)(4k|uhd)", "quality:4k"),
        ];

        for (pattern, tag) in resolution_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(&filename) {
                    metadata.tags.push(tag.to_string());
                }
            }
        }

        // 文件格式检测
        if let Some(extension) = std::path::Path::new(archive_path).extension().and_then(|ext| ext.to_str()) {
            metadata.tags.push(format!("format:{}", extension.to_lowercase()));
        }

        Ok(metadata)
    }

    async fn apply_metadata(&self, _archive_id: &str, _metadata: &ExtractedMetadata, _db: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
        // 由主插件处理
        Ok(())
    }
}

// 辅助函数：合并元数据
fn merge_metadata(target: &mut ExtractedMetadata, source: &ExtractedMetadata) {
    if target.title.is_none() && source.title.is_some() {
        target.title = source.title.clone();
    }
    if target.artist.is_none() && source.artist.is_some() {
        target.artist = source.artist.clone();
    }
    if target.series.is_none() && source.series.is_some() {
        target.series = source.series.clone();
    }
    if target.language.is_none() && source.language.is_some() {
        target.language = source.language.clone();
    }
    
    // 合并标签（去重）
    for tag in &source.tags {
        if !target.tags.contains(tag) {
            target.tags.push(tag.clone());
        }
    }
}

// 插件入口点
#[no_mangle]
pub extern "C" fn plugin_init(config_json: *const i8) -> *mut MetadataExtractorPlugin {
    let _config = if config_json.is_null() {
        PluginConfig::default()
    } else {
        let config_str = unsafe { std::ffi::CStr::from_ptr(config_json).to_str().unwrap_or("{}") };
        serde_json::from_str(config_str).unwrap_or_default()
    };

    Box::into_raw(Box::new(MetadataExtractorPlugin::new()))
}

#[no_mangle]
pub extern "C" fn plugin_process_archive(
    plugin: *mut MetadataExtractorPlugin,
    archive_path: *const i8,
    archive_id: *const i8,
    config_json: *const i8,
) -> i32 {
    if plugin.is_null() || archive_path.is_null() || archive_id.is_null() {
        return -1;
    }

    let _plugin = unsafe { &*plugin };
    let _archive_path = unsafe { std::ffi::CStr::from_ptr(archive_path).to_str().unwrap_or("") };
    let _archive_id = unsafe { std::ffi::CStr::from_ptr(archive_id).to_str().unwrap_or("") };
    
    let _config = if config_json.is_null() {
        PluginConfig::default()
    } else {
        let config_str = unsafe { std::ffi::CStr::from_ptr(config_json).to_str().unwrap_or("{}") };
        serde_json::from_str(config_str).unwrap_or_default()
    };

    // 这里应该在异步运行时中执行，但为了简化示例，我们返回成功
    // 实际实现中需要使用适当的异步处理机制
    0
}

#[no_mangle]
pub extern "C" fn plugin_cleanup(plugin: *mut MetadataExtractorPlugin) {
    if !plugin.is_null() {
        unsafe {
            Box::from_raw(plugin);
        }
    }
}
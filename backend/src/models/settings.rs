use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    #[serde(rename = "comicsPath")]
    pub comics_path: String,
    #[serde(rename = "supportedFormats")]
    pub supported_formats: Vec<String>,
    #[serde(rename = "maxFileSize")]
    pub max_file_size: u64,
    #[serde(rename = "imageCacheSize")]
    pub image_cache_size: u64,
    #[serde(rename = "scanOnStartup")]
    pub scan_on_startup: bool,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            comics_path: "./comics".to_string(),
            supported_formats: vec![
                "cbz".to_string(),
                "zip".to_string(),
                "cbr".to_string(),
                "rar".to_string(),
                "cb7".to_string(),
                "7z".to_string(),
                "pdf".to_string(),
            ],
            max_file_size: 500 * 1024 * 1024, // 500MB
            image_cache_size: 1024 * 1024 * 1024, // 1GB
            scan_on_startup: true,
        }
    }
}
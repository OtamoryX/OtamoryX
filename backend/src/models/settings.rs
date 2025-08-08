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
    #[serde(rename = "imageCachePath")]
    pub image_cache_path: String,
    #[serde(rename = "scanOnStartup")]
    pub scan_on_startup: bool,
    #[serde(rename = "scanSettings")]
    pub scan_settings: ScanSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSettings {
    pub enabled: bool,
    pub recursive: bool,
    #[serde(rename = "ignoreHidden")]
    pub ignore_hidden: bool,
    #[serde(rename = "realtimeMonitoring")]
    pub realtime_monitoring: bool,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            comics_path: "./data/comics".to_string(),
            supported_formats: vec![
                "cbz".to_string(),
                "zip".to_string(),
                "cbr".to_string(),
                "rar".to_string(),
                "cb7".to_string(),
                "7z".to_string(),
                "pdf".to_string(),
            ],
            max_file_size: 500 * 1024 * 1024,     // 500MB
            image_cache_size: 1024 * 1024 * 1024, // 1GB
            image_cache_path: "./data/cache".to_string(),
            scan_on_startup: true,
            scan_settings: ScanSettings::default(),
        }
    }
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            recursive: true,
            ignore_hidden: true,
            realtime_monitoring: false,
        }
    }
}

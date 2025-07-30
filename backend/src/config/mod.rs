use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub comics_path: String,
    pub data_path: String,
    pub max_file_size: String,
    pub supported_formats: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    pub image_cache_size: u64,
    pub image_cache_ttl: u64,
    pub metadata_cache_ttl: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                workers: 4,
            },
            database: DatabaseConfig {
                url: "sqlite://./data/otamoryx.db".to_string(),
                max_connections: 20,
                min_connections: 1,
            },
            storage: StorageConfig {
                comics_path: "./comics".to_string(),
                data_path: "./data".to_string(),
                max_file_size: "100MB".to_string(),
                supported_formats: vec![
                    "cbz".to_string(),
                    "cbr".to_string(),
                    "zip".to_string(),
                    "rar".to_string(),
                    "7z".to_string(),
                    "pdf".to_string(),
                ],
            },
            cache: CacheConfig {
                image_cache_size: 1000,
                image_cache_ttl: 3600,
                metadata_cache_ttl: 86400,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::Environment::with_prefix("OTAMORYX"))
            .build()?;

        // 覆盖环境变量
        if let Ok(database_url) = std::env::var("DATABASE_URL") {
            cfg.set("database.url", database_url)?;
        }
        if let Ok(comics_path) = std::env::var("COMICS_PATH") {
            cfg.set("storage.comics_path", comics_path)?;
        }

        cfg.try_deserialize()
    }
}
pub mod config;
pub mod database;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod plugins;
pub mod services;
pub mod utils;

pub use config::Config;

use sqlx::{Pool, Sqlite};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Pool<Sqlite>>,
    pub config: Arc<Config>,
    pub services: Arc<Services>,
}

pub struct Services {
    pub archive_service: services::archive_service::ArchiveService,
    pub auth_service: services::auth_service::AuthService,
    pub search_service: services::search_service::SearchService,
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

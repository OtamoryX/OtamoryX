pub mod access_control_service;
pub mod admin_service;
pub mod ai_service;
pub mod archive;
// Compatibility modules keep the old paths stable while the implementation lives under the
// archive domain directory. New code should import from `services::archive`.
#[allow(unused_imports)]
pub mod archive_cache_service {
    pub use super::archive::cache::*;
}
#[allow(unused_imports)]
pub mod archive_deletion_service {
    pub use super::archive::deletion::*;
}
#[allow(unused_imports)]
pub mod archive_processing_service {
    pub use super::archive::processing::*;
}
#[allow(unused_imports)]
pub mod archive_query_service {
    pub use super::archive::query::*;
}
#[allow(unused_imports)]
pub mod archive_service {
    pub use super::archive::service::*;
}
pub mod auth_service;
pub mod auto_delete_service;
pub mod cleanup_service;
pub mod collection_service;
pub mod content_analysis_service;
pub mod curation_service;
pub mod file_monitor_service;
pub mod metadata;
pub mod ocr_service;
pub mod plugin_bootstrap;
pub mod plugin_event_bus;
pub mod plugin_executor;
pub mod plugin_manager;
pub mod plugin_manifest;
pub mod plugin_scheduler;
pub mod preferences;
#[allow(unused_imports)]
pub mod ehentai_metadata_service {
    pub use super::metadata::ehentai::*;
}
#[allow(unused_imports)]
pub mod nhentai_metadata_service {
    pub use super::metadata::nhentai::*;
}
#[allow(unused_imports)]
pub mod preference_decision_service {
    pub use super::preferences::decision::*;
}
#[allow(unused_imports)]
pub mod preference_learning_service {
    pub use super::preferences::learning::*;
}
pub mod processing_pipeline;
pub mod random_metrics_service;
pub mod random_service;
pub mod rate_limiter;
pub mod search_service;
pub mod trash_service;

pub use ai_service::*;
pub use archive::*;
pub use auth_service::*;
pub use auto_delete_service::*;
pub use cleanup_service::*;
pub use content_analysis_service::*;
pub use curation_service::*;
pub use file_monitor_service::*;
pub use ocr_service::*;
pub use plugin_bootstrap::*;
pub use plugin_event_bus::*;
pub use plugin_executor::*;
pub use plugin_manager::*;
pub use plugin_manifest::*;
pub use plugin_scheduler::*;
pub use preferences::*;
pub use random_metrics_service::*;
pub use random_service::*;
pub use search_service::*;
pub use trash_service::*;

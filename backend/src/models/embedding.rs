use serde::{Deserialize, Serialize};

pub const EMBEDDING_SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EmbeddingSettings {
    pub settings_version: u32,
    pub provider: EmbeddingProvider,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub request_interval_seconds: u64,
    pub dimensions: Option<u32>,
    pub auth_mode: EmbeddingAuthMode,
    /// Accepted on updates but never included in a serialized response.
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    #[serde(skip_deserializing)]
    pub api_key_configured: bool,
}

impl Default for EmbeddingSettings {
    fn default() -> Self {
        Self {
            settings_version: EMBEDDING_SETTINGS_VERSION,
            provider: EmbeddingProvider::Ollama,
            base_url: "http://localhost:11434".to_string(),
            model: String::new(),
            timeout_seconds: 120,
            request_interval_seconds: 20,
            dimensions: None,
            auth_mode: EmbeddingAuthMode::None,
            api_key: None,
            api_key_configured: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EmbeddingProvider {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "openaiCompatible")]
    OpenaiCompatible,
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        Self::Ollama
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum EmbeddingAuthMode {
    #[default]
    None,
    Bearer,
}

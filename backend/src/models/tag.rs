use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub namespace: String,
    /// The requested UI locale's display label. `name` remains the canonical storage and
    /// filtering identity, so integrations never depend on a translated string.
    #[serde(rename = "localizedName", skip_serializing_if = "Option::is_none")]
    pub localized_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArchiveTag {
    pub archive_id: String,
    pub tag_id: String,
}

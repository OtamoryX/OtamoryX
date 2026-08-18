use crate::middleware::auth::AuthInfo;
use crate::models::TrashQuery;
use crate::services::{ArchiveCacheService, CurationService, TrashService};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};

pub async fn list_trash_entries(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Query(query): Query<TrashQuery>,
) -> Result<Json<Vec<crate::models::TrashEntry>>, StatusCode> {
    if let Some(status) = query.status.as_deref() {
        if !matches!(status, "active" | "restored" | "expired") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let entries = TrashService::new(pool)
        .list_entries(
            &auth.user_id,
            query.status.as_deref(),
            query.limit.unwrap_or(100),
        )
        .await
        .map_err(|error| {
            tracing::warn!("Failed to list trash entries: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(entries))
}

pub async fn restore_trash_entry(
    State(pool): State<Pool<Sqlite>>,
    Path(entry_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<crate::models::TrashEntry>, StatusCode> {
    let restored = TrashService::new(pool.clone())
        .restore_entry(&auth.user_id, &entry_id)
        .await
        .map_err(|error| {
            tracing::warn!("Failed to restore trash entry {entry_id}: {error}");
            let message = error.to_string();
            if message.contains("not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("already exists")
                || message.contains("not active")
                || message.contains("must be restored through their operation")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    let behavior = crate::models::RecordBehaviorEventRequest {
        archive_id: Some(restored.archive_id.clone()),
        event_type: "restore".to_string(),
        event_key: Some(format!("trash-restore:{entry_id}")),
        page: None,
        metadata: serde_json::json!({ "source": "trash", "trashEntryId": entry_id }),
        occurred_at: Some(chrono::Utc::now()),
    };
    if let Err(error) = CurationService::new(pool.clone())
        .record_event(&auth.user_id, &behavior)
        .await
    {
        tracing::warn!("Failed to record restore behavior: {error}");
    }
    if let Err(error) = CurationService::new(pool.clone())
        .record_disposition(
            &auth.user_id,
            &restored.archive_id,
            "restored",
            Some("user restored archive from trash"),
            "user",
        )
        .await
    {
        tracing::warn!("Failed to record restore disposition: {error}");
    }

    // Automatic deletion provenance is relational, so correction accounting
    // does not depend on a historical JSON snapshot or decision-key parsing.
    if let (Some(rule_id), Some(rule_version), Some(evaluation_id)) = (
        restored.rule_id.as_deref(),
        restored.rule_version.as_deref(),
        restored.evaluation_id.as_deref(),
    ) {
        let correction = crate::models::RecordBehaviorEventRequest {
            archive_id: Some(restored.archive_id.clone()),
            event_type: "rule_correction".to_string(),
            event_key: Some(format!("auto-delete-restore:{entry_id}")),
            page: None,
            metadata: serde_json::json!({
                "source": "trash_restore",
                "correction": "auto_delete_restored",
                "trashEntryId": entry_id,
                "ruleId": rule_id,
                "ruleVersion": rule_version,
                "evaluationId": evaluation_id,
            }),
            occurred_at: Some(chrono::Utc::now()),
        };
        if let Err(error) = CurationService::new(pool.clone())
            .record_event(&auth.user_id, &correction)
            .await
        {
            tracing::warn!("Failed to record automatic deletion correction feedback: {error}");
        }
        let _ = sqlx::query(
            "INSERT INTO preference_rule_corrections (id, evaluation_id, user_id, correction, metadata_json)
             VALUES (?, ?, ?, 'restored_auto_delete', ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(evaluation_id)
        .bind(&auth.user_id)
        .bind(serde_json::json!({ "entryId": entry_id }).to_string())
        .execute(&pool)
        .await;
        let _ = sqlx::query(
            "UPDATE preference_rules SET false_positive_count = false_positive_count + 1, preference_weight = MAX(0.1, preference_weight * 0.5), auto_paused = CASE WHEN false_positive_count + 1 >= 3 THEN 1 ELSE auto_paused END, enabled = CASE WHEN false_positive_count + 1 >= 3 THEN 0 ELSE enabled END WHERE id = ? AND rule_version = ?",
        )
        .bind(rule_id)
        .bind(rule_version)
        .execute(&pool)
        .await;
    }

    Ok(Json(restored))
}

pub async fn restore_trash_operation(
    State(pool): State<Pool<Sqlite>>,
    Path(operation_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<
        std::sync::Arc<ArchiveCacheService>,
    >,
) -> Result<Json<Vec<crate::models::TrashEntry>>, StatusCode> {
    let restored = TrashService::new(pool.clone())
        .restore_operation(&auth.user_id, &operation_id)
        .await
        .map_err(|error| {
            tracing::warn!("Failed to restore trash operation {operation_id}: {error}");
            let message = error.to_string();
            if message.contains("not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("not restorable")
                || message.contains("no longer has all active")
                || message.contains("already exists")
                || message.contains("missing")
                || message.contains("already being restored")
                || message.contains("changed since cleanup")
                || message.contains("does not contain a recoverable relation snapshot")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    let curation = CurationService::new(pool.clone());
    if let Ok(Some(keeper_id)) = sqlx::query_scalar::<_, String>(
        "SELECT keep_archive_id FROM trash_operations WHERE id = ? AND user_id = ?",
    )
    .bind(&operation_id)
    .bind(&auth.user_id)
    .fetch_optional(&pool)
    .await
    {
        archive_cache.clear_archive_cache(&keeper_id).await;
    }
    for entry in &restored {
        archive_cache.clear_archive_cache(&entry.archive_id).await;
        let behavior = crate::models::RecordBehaviorEventRequest {
            archive_id: Some(entry.archive_id.clone()),
            event_type: "restore".to_string(),
            event_key: Some(format!(
                "version-cleanup-restore:{operation_id}:{}",
                entry.id
            )),
            page: None,
            metadata: serde_json::json!({
                "source": "version_cleanup",
                "trashEntryId": entry.id,
                "operationId": operation_id,
            }),
            occurred_at: Some(chrono::Utc::now()),
        };
        if let Err(error) = curation.record_event(&auth.user_id, &behavior).await {
            tracing::warn!(%operation_id, %error, "Failed to record version cleanup restore behavior");
        }
        if let Err(error) = curation
            .record_disposition(
                &auth.user_id,
                &entry.archive_id,
                "restored",
                Some("user restored version cleanup operation"),
                "version_cleanup",
            )
            .await
        {
            tracing::warn!(%operation_id, %error, "Failed to record version cleanup restore disposition");
        }
    }
    Ok(Json(restored))
}

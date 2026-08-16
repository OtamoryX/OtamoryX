use crate::middleware::auth::AuthInfo;
use crate::models::TrashQuery;
use crate::services::{CurationService, TrashService};
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
            } else if message.contains("already exists") || message.contains("not active") {
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

    // Restoring an automatically removed archive is a strong correction signal
    // for the rule that produced the decision.
    let snapshot = serde_json::from_str::<serde_json::Value>(&restored.metadata_json).ok();
    if snapshot
        .as_ref()
        .and_then(|value| value.get("source"))
        .and_then(serde_json::Value::as_str)
        == Some("auto_delete")
    {
        let correction = crate::models::RecordBehaviorEventRequest {
            archive_id: Some(restored.archive_id.clone()),
            event_type: "rule_correction".to_string(),
            event_key: Some(format!("auto-delete-restore:{entry_id}")),
            page: None,
            metadata: serde_json::json!({
                "source": "trash_restore",
                "correction": "auto_delete_restored",
                "trashEntryId": entry_id,
                "decisionKey": snapshot.as_ref().and_then(|value| value.get("decision_key")),
                "ruleVersion": snapshot.as_ref().and_then(|value| value.get("rule_version")),
            }),
            occurred_at: Some(chrono::Utc::now()),
        };
        if let Err(error) = CurationService::new(pool.clone())
            .record_event(&auth.user_id, &correction)
            .await
        {
            tracing::warn!("Failed to record automatic deletion correction feedback: {error}");
        }
        if let (Some(rule_id), Some(rule_version)) = (
            snapshot
                .as_ref()
                .and_then(|v| v.get("rule_id"))
                .and_then(|v| v.as_str()),
            snapshot
                .as_ref()
                .and_then(|v| v.get("rule_version"))
                .and_then(|v| v.as_str()),
        ) {
            let _ = sqlx::query(
                "INSERT INTO preference_rule_corrections (id, evaluation_id, user_id, correction, metadata_json) \
                 SELECT ?, e.id, ?, 'restored_auto_delete', ? FROM preference_rule_evaluations e \
                 WHERE e.rule_id = ? AND e.rule_version = ? AND e.analysis_id IN (SELECT id FROM content_analyses WHERE archive_id = ?) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&auth.user_id)
            .bind(serde_json::json!({ "entryId": entry_id }).to_string())
            .bind(rule_id)
            .bind(rule_version)
            .bind(&restored.archive_id)
            .execute(&pool)
            .await;
            let _ = sqlx::query(
                "UPDATE preference_rules SET false_positive_count = false_positive_count + 1, auto_paused = CASE WHEN false_positive_count + 1 >= 3 THEN 1 ELSE auto_paused END, enabled = CASE WHEN false_positive_count + 1 >= 3 THEN 0 ELSE enabled END WHERE id = ? AND rule_version = ?",
            )
            .bind(rule_id)
            .bind(rule_version)
            .execute(&pool)
            .await;
        }
    }

    Ok(Json(restored))
}

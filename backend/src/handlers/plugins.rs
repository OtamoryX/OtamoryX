use std::{collections::HashSet, fs::File, io::Read, path::Path as FsPath, time::Instant};

use axum::{
    extract::{Extension, Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sqlx::{Pool, QueryBuilder, Sqlite};
use tracing::{info, warn};
use uuid::Uuid;

use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{
    Plugin, PluginConfigRequest, PluginConfigSchemaResponse, PluginDetail, PluginExecuteRequest,
    PluginExecuteResponse, PluginExecutionDispatchResult, PluginExecutionListResponse,
    PluginExecutionRecord,
};
use crate::plugins::{
    builtin::{
        comicinfo_parser::ComicInfoParser, date_added::DateAdded, filename_parser::FilenameParser,
        tag_copier::TagCopier,
    },
    merge_plugin_output, BuiltinPlugin, PluginContext, PluginOutput, TagConflictDecision,
    TagConflictResolver, TagCopierRequest, TagProposal, TagProvenance, BUILTIN_COMICINFO_PARSER_ID,
    BUILTIN_DATE_ADDED_ID, BUILTIN_EHENTAI_METADATA_ID, BUILTIN_FILENAME_PARSER_ID,
    BUILTIN_METADATA_ORDER_COMICINFO, BUILTIN_METADATA_ORDER_DATE_ADDED,
    BUILTIN_METADATA_ORDER_FILENAME, BUILTIN_TAG_COPIER_ID, DEFAULT_TAG_CONFLICT_RESOLVER,
};
use crate::services::ehentai_metadata_service::{
    fetch_metadata as fetch_ehentai_metadata, parse_gallery_reference, search_candidates,
    EhentaiCandidate, EhentaiConfig, EHENTAI_METADATA_PLUGIN_ID,
};

pub struct PluginHandler;

type ApiError = (StatusCode, Json<PluginApiError>);

#[derive(Debug, Serialize)]
pub struct PluginApiError {
    code: &'static str,
    message: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PluginManifestRow {
    #[sqlx(rename = "id")]
    plugin_id: String,
    manifest: Option<JsonValue>,
}

#[derive(Debug, sqlx::FromRow)]
struct PluginExecutionContext {
    #[sqlx(rename = "id")]
    plugin_id: String,
    enabled: bool,
    manifest: Option<JsonValue>,
    last_executed_at: Option<DateTime<Utc>>,
    config: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct EhentaiCandidateSearchResponse {
    pub archive_id: String,
    pub candidates: Vec<EhentaiCandidate>,
}

#[derive(Debug)]
struct ParsedManifestMeta {
    cooldown: Option<u32>,
    config_schema: JsonValue,
}

#[derive(Debug, sqlx::FromRow)]
struct ArchiveExecutionRow {
    id: String,
    title: String,
    path: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct ArchiveExistingTagRow {
    #[sqlx(rename = "tag_id")]
    tag_id: String,
    #[sqlx(rename = "tag_name")]
    tag_name: String,
    namespace: String,
    plugin_generated: i64,
}

#[derive(Debug, Clone)]
struct ArchiveExistingTag {
    tag_id: String,
    tag_name: String,
    namespace: String,
    provenance: TagProvenance,
}

#[derive(Debug, Default)]
struct BuiltinPersistStats {
    title_updated: bool,
    tags_applied: usize,
    tags_skipped: usize,
    manual_review: usize,
    notes: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PluginExecutionsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
    pub archive_id: Option<String>,
    pub plugin_id: Option<String>,
}

impl PluginHandler {
    /// GET /api/v1/plugins - 获取已安装插件列表
    pub async fn list_plugins(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<Plugin>>, StatusCode> {
        let plugins = sqlx::query_as::<_, Plugin>(
            r#"
            SELECT
                id,
                name,
                version,
                plugin_type,
                description,
                author,
                enabled,
                config,
                execution_count,
                last_executed_at,
                created_at AS installed_at,
                updated_at
            FROM plugins
            ORDER BY name
            "#,
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(plugins))
    }

    /// GET /api/v1/plugins/:id - 获取插件详情
    pub async fn get_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
    ) -> Result<Json<PluginDetail>, ApiError> {
        let plugin = sqlx::query_as::<_, PluginDetail>(
            r#"
            SELECT
                id,
                name,
                version,
                manifest_version,
                plugin_api_version,
                plugin_type,
                description,
                author,
                icon,
                enabled,
                config,
                permissions,
                manifest,
                execution_count,
                last_executed_at,
                created_at AS installed_at,
                updated_at
            FROM plugins
            WHERE id = ?
            "#,
        )
        .bind(&plugin_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                "获取插件详情失败",
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "plugin_not_found",
                format!("插件 `{plugin_id}` 不存在"),
            )
        })?;

        Ok(Json(plugin))
    }

    /// GET /api/v1/plugins/:id/config/schema - 获取插件配置 schema
    pub async fn get_plugin_config_schema(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
    ) -> Result<Json<PluginConfigSchemaResponse>, ApiError> {
        let plugin =
            sqlx::query_as::<_, PluginManifestRow>("SELECT id, manifest FROM plugins WHERE id = ?")
                .bind(&plugin_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "db_error",
                        "获取插件 manifest 失败",
                    )
                })?
                .ok_or_else(|| {
                    api_error(
                        StatusCode::NOT_FOUND,
                        "plugin_not_found",
                        format!("插件 `{plugin_id}` 不存在"),
                    )
                })?;

        let parsed = match plugin.manifest {
            Some(manifest) => parse_manifest_meta(&plugin.plugin_id, Some(manifest))?,
            None => ParsedManifestMeta {
                cooldown: None,
                config_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        };
        Ok(Json(PluginConfigSchemaResponse {
            plugin_id: plugin.plugin_id,
            config_schema: parsed.config_schema,
            cooldown: parsed.cooldown,
        }))
    }

    /// POST /api/v1/plugins/install - 安装插件
    pub async fn install_plugin(
        State(pool): State<Pool<Sqlite>>,
        mut multipart: Multipart,
    ) -> Result<Json<Plugin>, StatusCode> {
        // TODO: 实际的插件安装逻辑
        // 这里应该包括：
        // 1. 验证插件文件
        // 2. 解析插件元数据
        // 3. 检查权限和依赖
        // 4. 安装插件到系统

        let mut uploaded_filename: Option<String> = None;
        let mut has_plugin_payload = false;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
        {
            if field.name() == Some("plugin") {
                uploaded_filename = field.file_name().map(|name| name.to_string());
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if bytes.is_empty() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                has_plugin_payload = true;
                break;
            }
        }

        if !has_plugin_payload {
            return Err(StatusCode::BAD_REQUEST);
        }

        let plugin_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let plugin_name = uploaded_filename
            .as_deref()
            .map(infer_plugin_name)
            .unwrap_or_else(|| format!("plugin-{}", &plugin_id[..8]));

        let plugin = Plugin {
            plugin_id: plugin_id.clone(),
            name: plugin_name,
            version: "1.0.0".to_string(), // 从插件元数据获取
            plugin_type: "metadata".to_string(),
            description: None,
            author: None,
            enabled: false,
            config: None,
            execution_count: 0,
            last_executed_at: None,
            installed_at: now,
            updated_at: now,
        };

        let manifest = default_manifest_for_uploaded_plugin(
            &plugin.plugin_id,
            &plugin.name,
            &plugin.version,
            &plugin.plugin_type,
        );

        sqlx::query(
            r#"
            INSERT INTO plugins (id, name, version, enabled, config, manifest, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&plugin.plugin_id)
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(plugin.enabled)
        .bind(&plugin.config)
        .bind(&manifest)
        .bind(plugin.installed_at)
        .bind(plugin.updated_at)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(plugin))
    }

    /// POST /api/v1/plugins/:id/execute - 手动执行插件（通用）
    pub async fn execute_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
        Extension(auth): Extension<AuthInfo>,
        request: Option<Json<PluginExecuteRequest>>,
    ) -> Result<(StatusCode, Json<PluginExecuteResponse>), ApiError> {
        let request = request.map(|Json(v)| v).unwrap_or_default();
        authorize_plugin_execution_targets(&pool, &auth, None, &request).await?;
        execute_plugin_internal(&pool, plugin_id, None, request).await
    }

    /// POST /api/v1/plugins/:id/execute/:archive_id - 对特定档案执行插件
    pub async fn execute_plugin_for_archive(
        State(pool): State<Pool<Sqlite>>,
        Path((plugin_id, archive_id)): Path<(String, String)>,
        Extension(auth): Extension<AuthInfo>,
        request: Option<Json<PluginExecuteRequest>>,
    ) -> Result<(StatusCode, Json<PluginExecuteResponse>), ApiError> {
        let request = request.map(|Json(v)| v).unwrap_or_default();
        authorize_plugin_execution_targets(&pool, &auth, Some(&archive_id), &request).await?;
        let result =
            execute_plugin_internal(&pool, plugin_id, Some(archive_id.clone()), request).await;
        if result.is_ok() {
            // Re-check after a manual metadata run. The service deduplicates unchanged titles;
            // automatic runs are queued only after the whole metadata pipeline finishes.
            if let Err(err) = crate::services::enqueue_title_translation(&pool, &archive_id).await {
                warn!("Failed to enqueue title translation after manual plugin for archive {archive_id}: {err:#}");
            }
        }
        result
    }

    /// GET /api/v1/plugins/ehentai-metadata/candidates/:archive_id
    /// A title search only returns choices. It never applies a result until the user selects one.
    pub async fn search_ehentai_candidates(
        State(pool): State<Pool<Sqlite>>,
        Path(archive_id): Path<String>,
        Extension(auth): Extension<AuthInfo>,
    ) -> Result<Json<EhentaiCandidateSearchResponse>, ApiError> {
        let plugin = sqlx::query_as::<_, (bool, Option<JsonValue>)>(
            "SELECT enabled, config FROM plugins WHERE id = ? LIMIT 1",
        )
        .bind(EHENTAI_METADATA_PLUGIN_ID)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                "读取插件状态失败",
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "plugin_not_found",
                "E-Hentai Metadata 未安装",
            )
        })?;
        if !plugin.0 {
            return Err(api_error(
                StatusCode::CONFLICT,
                "plugin_disabled",
                "请先启用 E-Hentai Metadata",
            ));
        }
        let archive = sqlx::query_as::<_, (String, String)>(
            "SELECT title, path FROM archives WHERE id = ? LIMIT 1",
        )
        .bind(&archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                "读取漫画信息失败",
            )
        })?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "archive_not_found", "漫画不存在"))?;
        if !path_permission::has_path_permission(&pool, &auth, &archive.1)
            .await
            .map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "permission_error",
                    "校验漫画访问权限失败",
                )
            })?
        {
            return Err(api_error(
                StatusCode::FORBIDDEN,
                "archive_forbidden",
                "没有访问这部漫画的权限",
            ));
        }
        let candidates =
            search_candidates(&archive.0, &EhentaiConfig::from_json(plugin.1.as_ref()))
                .await
                .map_err(|message| {
                    api_error(StatusCode::BAD_GATEWAY, "ehentai_search_failed", message)
                })?;
        Ok(Json(EhentaiCandidateSearchResponse {
            archive_id,
            candidates,
        }))
    }

    /// GET /api/v1/plugins/:id/executions - 获取插件执行历史
    pub async fn list_plugin_executions(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
        Query(params): Query<PluginExecutionsQuery>,
    ) -> Result<Json<PluginExecutionListResponse>, ApiError> {
        let plugin_exists =
            sqlx::query_scalar::<_, i64>("SELECT 1 FROM plugins WHERE id = ? LIMIT 1")
                .bind(&plugin_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "db_error",
                        "查询插件信息失败",
                    )
                })?
                .is_some();

        if !plugin_exists {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "plugin_not_found",
                format!("插件 `{plugin_id}` 不存在"),
            ));
        }

        let (limit, offset) = normalize_pagination(params.limit, params.offset);

        let status_filter = params.status.clone();
        let archive_filter = params.archive_id.clone();

        let mut count_builder = QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(*) as count FROM plugin_executions WHERE plugin_id = ",
        );
        count_builder.push_bind(&plugin_id);

        if let Some(status) = status_filter.as_deref() {
            count_builder.push(" AND status = ").push_bind(status);
        }
        if let Some(archive_id) = archive_filter.as_deref() {
            count_builder
                .push(" AND archive_id = ")
                .push_bind(archive_id);
        }

        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&pool)
            .await
            .map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    "统计插件执行历史失败",
                )
            })?;

        let mut list_builder = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT
                id,
                plugin_id,
                archive_id,
                execution_type,
                status,
                input_summary,
                output_summary,
                error_message,
                duration_ms,
                started_at,
                completed_at
            FROM plugin_executions
            WHERE plugin_id = 
            "#,
        );
        list_builder.push_bind(&plugin_id);

        if let Some(status) = status_filter.as_deref() {
            list_builder.push(" AND status = ").push_bind(status);
        }
        if let Some(archive_id) = archive_filter.as_deref() {
            list_builder
                .push(" AND archive_id = ")
                .push_bind(archive_id);
        }

        list_builder
            .push(" ORDER BY started_at DESC LIMIT ")
            .push_bind(limit as i64)
            .push(" OFFSET ")
            .push_bind(offset as i64);

        let items = list_builder
            .build_query_as::<PluginExecutionRecord>()
            .fetch_all(&pool)
            .await
            .map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    "获取插件执行历史失败",
                )
            })?;

        Ok(Json(PluginExecutionListResponse {
            total,
            limit,
            offset,
            items,
        }))
    }

    /// GET /api/v1/plugin-executions - 获取所有插件执行历史
    pub async fn list_all_plugin_executions(
        State(pool): State<Pool<Sqlite>>,
        Query(params): Query<PluginExecutionsQuery>,
    ) -> Result<Json<PluginExecutionListResponse>, ApiError> {
        let (limit, offset) = normalize_pagination(params.limit, params.offset);

        let mut count_builder = QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(*) as count FROM plugin_executions WHERE 1=1",
        );
        if let Some(plugin_id) = params.plugin_id.as_deref() {
            count_builder.push(" AND plugin_id = ").push_bind(plugin_id);
        }
        if let Some(status) = params.status.as_deref() {
            count_builder.push(" AND status = ").push_bind(status);
        }
        if let Some(archive_id) = params.archive_id.as_deref() {
            count_builder
                .push(" AND archive_id = ")
                .push_bind(archive_id);
        }

        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&pool)
            .await
            .map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    "统计执行历史失败",
                )
            })?;

        let mut list_builder = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT
                id,
                plugin_id,
                archive_id,
                execution_type,
                status,
                input_summary,
                output_summary,
                error_message,
                duration_ms,
                started_at,
                completed_at
            FROM plugin_executions
            WHERE 1=1
            "#,
        );
        if let Some(plugin_id) = params.plugin_id.as_deref() {
            list_builder.push(" AND plugin_id = ").push_bind(plugin_id);
        }
        if let Some(status) = params.status.as_deref() {
            list_builder.push(" AND status = ").push_bind(status);
        }
        if let Some(archive_id) = params.archive_id.as_deref() {
            list_builder
                .push(" AND archive_id = ")
                .push_bind(archive_id);
        }

        list_builder
            .push(" ORDER BY started_at DESC LIMIT ")
            .push_bind(limit as i64)
            .push(" OFFSET ")
            .push_bind(offset as i64);

        let items = list_builder
            .build_query_as::<PluginExecutionRecord>()
            .fetch_all(&pool)
            .await
            .map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    "获取执行历史失败",
                )
            })?;

        Ok(Json(PluginExecutionListResponse {
            total,
            limit,
            offset,
            items,
        }))
    }

    /// PUT /api/v1/plugins/:id/toggle - 启用/禁用插件
    pub async fn toggle_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let current_enabled =
            sqlx::query_as::<_, (bool,)>("SELECT enabled FROM plugins WHERE id = ?")
                .bind(&plugin_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?
                .0;

        let new_enabled = !current_enabled;

        sqlx::query("UPDATE plugins SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(new_enabled)
            .bind(Utc::now())
            .bind(&plugin_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // TODO: 实际启用/禁用插件的逻辑
        // 这里应该包括：
        // 1. 如果启用：加载插件，初始化，注册钩子
        // 2. 如果禁用：卸载插件，清理资源

        Ok(StatusCode::OK)
    }

    /// PUT /api/v1/plugins/:id/config - 配置插件
    pub async fn configure_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
        Json(request): Json<PluginConfigRequest>,
    ) -> Result<StatusCode, StatusCode> {
        let plugin_exists =
            sqlx::query_scalar::<_, i64>("SELECT 1 FROM plugins WHERE id = ? LIMIT 1")
                .bind(&plugin_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .is_some();

        if !plugin_exists {
            return Err(StatusCode::NOT_FOUND);
        }

        // TODO: 验证配置格式是否符合插件要求
        sqlx::query("UPDATE plugins SET config = ?, updated_at = ? WHERE id = ?")
            .bind(&request.config)
            .bind(Utc::now())
            .bind(&plugin_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // TODO: 如果插件已启用，应该重新加载配置
        Ok(StatusCode::OK)
    }

    /// DELETE /api/v1/plugins/:id - 卸载插件
    pub async fn uninstall_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let result = sqlx::query("DELETE FROM plugins WHERE id = ?")
            .bind(&plugin_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        Ok(StatusCode::NO_CONTENT)
    }
}

async fn authorize_plugin_execution_targets(
    pool: &Pool<Sqlite>,
    auth: &AuthInfo,
    archive_id_from_path: Option<&str>,
    request: &PluginExecuteRequest,
) -> Result<(), ApiError> {
    let mut archive_ids = HashSet::new();
    archive_ids.extend(archive_id_from_path.map(str::to_string));
    archive_ids.extend(request.archive_id.iter().cloned());
    archive_ids.extend(request.archive_ids.iter().cloned());

    for archive_id in archive_ids {
        path_permission::authorize_archive_access(pool, auth, &archive_id)
            .await
            .map_err(|status| match status {
                StatusCode::NOT_FOUND => api_error(
                    StatusCode::NOT_FOUND,
                    "archive_not_found",
                    format!("漫画 `{archive_id}` 不存在"),
                ),
                StatusCode::FORBIDDEN => api_error(
                    StatusCode::FORBIDDEN,
                    "archive_forbidden",
                    format!("没有访问漫画 `{archive_id}` 的权限"),
                ),
                _ => api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "permission_error",
                    "校验漫画访问权限失败",
                ),
            })?;
    }
    Ok(())
}

pub(crate) async fn execute_plugin_internal(
    pool: &Pool<Sqlite>,
    plugin_id: String,
    archive_id_from_path: Option<String>,
    request: PluginExecuteRequest,
) -> Result<(StatusCode, Json<PluginExecuteResponse>), ApiError> {
    let context = sqlx::query_as::<_, PluginExecutionContext>(
        "SELECT id, enabled, manifest, last_executed_at, config FROM plugins WHERE id = ?",
    )
    .bind(&plugin_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            "查询插件状态失败",
        )
    })?
    .ok_or_else(|| {
        api_error(
            StatusCode::NOT_FOUND,
            "plugin_not_found",
            format!("插件 `{plugin_id}` 不存在"),
        )
    })?;

    if !context.enabled {
        return Err(api_error(
            StatusCode::CONFLICT,
            "plugin_disabled",
            format!("插件 `{}` 当前未启用，无法执行", context.plugin_id),
        ));
    }

    let builtin_plugin_id = normalize_builtin_plugin_id(&context.plugin_id);
    let parsed_manifest = match parse_manifest_meta(&context.plugin_id, context.manifest) {
        Ok(v) => v,
        Err(_) if builtin_plugin_id.is_some() => ParsedManifestMeta {
            cooldown: None,
            config_schema: JsonValue::Object(Default::default()),
        },
        Err(err) => return Err(err),
    };
    check_plugin_cooldown(
        &context.plugin_id,
        context.last_executed_at,
        parsed_manifest.cooldown,
    )?;

    let targets = collect_execution_targets(archive_id_from_path, &request);
    let mut accepted = 0usize;
    let mut failed = 0usize;
    let mut results = Vec::with_capacity(targets.len());

    for archive_id in targets {
        if let Some(archive_id_value) = archive_id.as_deref() {
            let archive_exists =
                sqlx::query_scalar::<_, i64>("SELECT 1 FROM archives WHERE id = ? LIMIT 1")
                    .bind(archive_id_value)
                    .fetch_optional(pool)
                    .await
                    .map_err(|_| {
                        api_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "db_error",
                            "校验档案存在性失败",
                        )
                    })?
                    .is_some();

            if !archive_exists {
                failed += 1;
                results.push(PluginExecutionDispatchResult {
                    plugin_id: context.plugin_id.clone(),
                    archive_id: Some(archive_id_value.to_string()),
                    execution_id: None,
                    status: "failed".to_string(),
                    error: Some(format!("archive `{archive_id_value}` 不存在")),
                });
                continue;
            }
        }

        let execution_id = Uuid::new_v4().to_string();
        let input_summary = build_input_summary(archive_id.as_deref(), &request);
        let insert_result = sqlx::query(
            r#"
            INSERT INTO plugin_executions (
                id,
                plugin_id,
                archive_id,
                execution_type,
                status,
                input_summary
            )
            VALUES (?, ?, ?, 'api', 'pending', ?)
            "#,
        )
        .bind(&execution_id)
        .bind(&context.plugin_id)
        .bind(&archive_id)
        .bind(input_summary)
        .execute(pool)
        .await;

        match insert_result {
            Ok(_) => {
                if let Some(builtin_id) = builtin_plugin_id {
                    let started = Instant::now();
                    let execution_outcome = if builtin_id == BUILTIN_EHENTAI_METADATA_ID {
                        execute_ehentai_metadata_and_persist(
                            pool,
                            &context.plugin_id,
                            archive_id.as_deref(),
                            &request,
                            context.config.as_ref(),
                        )
                        .await
                    } else {
                        execute_builtin_plugin_and_persist(
                            pool,
                            &context.plugin_id,
                            builtin_id,
                            archive_id.as_deref(),
                            &request,
                        )
                        .await
                    };

                    let duration_ms = saturating_duration_ms(started.elapsed().as_millis());
                    match execution_outcome {
                        Ok(output_summary) => {
                            update_plugin_execution_result(
                                pool,
                                &execution_id,
                                "success",
                                Some(output_summary),
                                None,
                                duration_ms,
                            )
                            .await?;
                            accepted += 1;
                            results.push(PluginExecutionDispatchResult {
                                plugin_id: context.plugin_id.clone(),
                                archive_id,
                                execution_id: Some(execution_id),
                                status: "success".to_string(),
                                error: None,
                            });
                        }
                        Err(err) => {
                            let failure_summary = serde_json::to_string(&json!({
                                "builtin_plugin": builtin_id,
                                "archive_id": archive_id,
                                "result": "failed",
                            }))
                            .ok();
                            update_plugin_execution_result(
                                pool,
                                &execution_id,
                                "failed",
                                failure_summary,
                                Some(err.clone()),
                                duration_ms,
                            )
                            .await?;
                            failed += 1;
                            results.push(PluginExecutionDispatchResult {
                                plugin_id: context.plugin_id.clone(),
                                archive_id,
                                execution_id: Some(execution_id),
                                status: "failed".to_string(),
                                error: Some(err),
                            });
                        }
                    }
                } else {
                    let message = "该插件尚未安装可运行的执行器，无法执行。".to_string();
                    update_plugin_execution_result(
                        pool,
                        &execution_id,
                        "failed",
                        None,
                        Some(message.clone()),
                        0,
                    )
                    .await?;
                    failed += 1;
                    results.push(PluginExecutionDispatchResult {
                        plugin_id: context.plugin_id.clone(),
                        archive_id,
                        execution_id: Some(execution_id),
                        status: "failed".to_string(),
                        error: Some(message),
                    });
                }
            }
            Err(err) => {
                failed += 1;
                results.push(PluginExecutionDispatchResult {
                    plugin_id: context.plugin_id.clone(),
                    archive_id,
                    execution_id: None,
                    status: "failed".to_string(),
                    error: Some(format!("写入执行记录失败: {err}")),
                });
            }
        }
    }

    if accepted > 0 {
        sqlx::query(
            r#"
            UPDATE plugins
            SET
                last_executed_at = ?,
                execution_count = execution_count + ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now())
        .bind(accepted as i64)
        .bind(Utc::now())
        .bind(&context.plugin_id)
        .execute(pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                "更新插件执行统计失败",
            )
        })?;
    }

    let response = PluginExecuteResponse {
        plugin_id: context.plugin_id.clone(),
        total: accepted + failed,
        accepted,
        failed,
        results,
    };

    let status = if accepted > 0 {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    Ok((status, Json(response)))
}

pub async fn auto_execute_enabled_metadata_plugins_for_archive(
    pool: &Pool<Sqlite>,
    archive_id: &str,
) {
    let mut plugin_rows = match sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM plugins
        WHERE enabled = 1
          AND plugin_type = 'metadata'
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(err) => {
            warn!(
                "Failed to load enabled metadata plugins for archive {}: {}",
                archive_id, err
            );
            return;
        }
    };

    plugin_rows.sort_by(|left, right| {
        metadata_plugin_execution_rank(left)
            .cmp(&metadata_plugin_execution_rank(right))
            .then_with(|| left.cmp(right))
    });

    if plugin_rows.is_empty() {
        info!(
            "No enabled metadata plugins to auto-execute for archive {}",
            archive_id
        );
        return;
    }

    info!(
        "Auto-executing {} metadata plugins for archive {}",
        plugin_rows.len(),
        archive_id
    );

    for plugin_id in plugin_rows {
        if plugin_id == EHENTAI_METADATA_PLUGIN_ID
            && !archive_has_ehentai_source(pool, archive_id)
                .await
                .unwrap_or(false)
        {
            info!(
                "Skipping automatic E-Hentai metadata lookup for archive {} because it has no explicit source URL",
                archive_id
            );
            continue;
        }
        match execute_plugin_internal(
            pool,
            plugin_id.clone(),
            Some(archive_id.to_string()),
            PluginExecuteRequest::default(),
        )
        .await
        {
            Ok((status, Json(response))) => {
                info!(
                    "Auto plugin dispatch status={} plugin_id={} archive_id={} accepted={} failed={}",
                    status.as_u16(),
                    plugin_id,
                    archive_id,
                    response.accepted,
                    response.failed
                );
            }
            Err((status, err)) => {
                warn!(
                    "Auto plugin dispatch failed status={} plugin_id={} archive_id={} code={} message={}",
                    status.as_u16(),
                    plugin_id,
                    archive_id,
                    err.code,
                    err.message
                );
            }
        }
    }
}

fn normalize_builtin_plugin_id(plugin_id: &str) -> Option<&'static str> {
    match plugin_id {
        "filename" | BUILTIN_FILENAME_PARSER_ID => Some(BUILTIN_FILENAME_PARSER_ID),
        "comicinfo" | BUILTIN_COMICINFO_PARSER_ID => Some(BUILTIN_COMICINFO_PARSER_ID),
        BUILTIN_DATE_ADDED_ID => Some(BUILTIN_DATE_ADDED_ID),
        BUILTIN_TAG_COPIER_ID => Some(BUILTIN_TAG_COPIER_ID),
        BUILTIN_EHENTAI_METADATA_ID => Some(BUILTIN_EHENTAI_METADATA_ID),
        _ => None,
    }
}

fn metadata_plugin_execution_rank(plugin_id: &str) -> (u8, u16) {
    match plugin_id {
        BUILTIN_FILENAME_PARSER_ID => (0, BUILTIN_METADATA_ORDER_FILENAME),
        BUILTIN_COMICINFO_PARSER_ID => (0, BUILTIN_METADATA_ORDER_COMICINFO),
        BUILTIN_DATE_ADDED_ID => (0, BUILTIN_METADATA_ORDER_DATE_ADDED),
        _ => (1, u16::MAX),
    }
}

fn build_builtin_plugin(plugin_id: &str) -> Option<Box<dyn BuiltinPlugin>> {
    match plugin_id {
        BUILTIN_FILENAME_PARSER_ID => Some(Box::new(FilenameParser::default())),
        BUILTIN_COMICINFO_PARSER_ID => Some(Box::new(ComicInfoParser::default())),
        BUILTIN_DATE_ADDED_ID => Some(Box::new(DateAdded::default())),
        BUILTIN_TAG_COPIER_ID => Some(Box::new(TagCopier::default())),
        _ => None,
    }
}

async fn execute_builtin_plugin_and_persist(
    pool: &Pool<Sqlite>,
    db_plugin_id: &str,
    builtin_plugin_id: &str,
    archive_id: Option<&str>,
    request: &PluginExecuteRequest,
) -> Result<String, String> {
    let archive_id = archive_id
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("内置插件 `{builtin_plugin_id}` 需要 archive_id"))?;

    let archive = fetch_archive_execution_row(pool, archive_id).await?;
    let plugin = build_builtin_plugin(builtin_plugin_id)
        .ok_or_else(|| format!("未知内置插件 `{builtin_plugin_id}`"))?;

    let mut embedded_files = extract_embedded_files_from_input(request);
    if embedded_files.is_empty() {
        embedded_files = list_embedded_files_from_archive(&archive.path);
    }
    let comicinfo_xml = extract_comicinfo_xml_from_input(request)
        .or_else(|| read_comicinfo_xml_from_archive(&archive.path));

    let context = PluginContext {
        archive_id: archive.id.clone(),
        archive_path: archive.path.clone(),
        ingested_at_unix: archive.created_at.timestamp(),
        embedded_files,
        comicinfo_xml,
        tag_copier_request: parse_tag_copier_request(request),
    };

    let raw_output = plugin.run(&context).map_err(|err| err.to_string())?;
    let mut merged_output = PluginOutput::default();
    merge_plugin_output(
        &mut merged_output,
        raw_output,
        &DEFAULT_TAG_CONFLICT_RESOLVER,
    );

    let stats = persist_builtin_output(pool, db_plugin_id, &archive, merged_output).await?;

    serde_json::to_string(&json!({
        "builtin_plugin": builtin_plugin_id,
        "archive_id": archive.id,
        "title_updated": stats.title_updated,
        "tags_applied": stats.tags_applied,
        "tags_skipped": stats.tags_skipped,
        "manual_review": stats.manual_review,
        "notes": stats.notes,
    }))
    .map_err(|err| format!("构建执行摘要失败: {err}"))
}

async fn execute_ehentai_metadata_and_persist(
    pool: &Pool<Sqlite>,
    db_plugin_id: &str,
    archive_id: Option<&str>,
    request: &PluginExecuteRequest,
    config: Option<&JsonValue>,
) -> Result<String, String> {
    let archive_id = archive_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "E-Hentai Metadata 需要指定一部漫画".to_string())?;
    let archive = fetch_archive_execution_row(pool, archive_id).await?;
    let source = request
        .oneshot_param
        .as_deref()
        .and_then(parse_gallery_reference)
        .or(find_ehentai_source_tag(pool, archive_id).await?);
    let (gallery_id, token) = source.ok_or_else(|| {
        "请填写 E-Hentai/ExHentai 画廊链接，或先点击“搜索候选”并选择正确结果。为避免误匹配，插件不会自动采用标题搜索结果。".to_string()
    })?;
    let output =
        fetch_ehentai_metadata(&gallery_id, &token, &EhentaiConfig::from_json(config)).await?;
    let stats = persist_builtin_output(pool, db_plugin_id, &archive, output).await?;
    serde_json::to_string(&json!({
        "builtin_plugin": BUILTIN_EHENTAI_METADATA_ID,
        "archive_id": archive.id,
        "gallery_url": crate::services::ehentai_metadata_service::source_url(&gallery_id, &token),
        "tags_applied": stats.tags_applied,
        "tags_skipped": stats.tags_skipped,
        "notes": stats.notes,
    }))
    .map_err(|err| format!("构建 E-Hentai 执行摘要失败: {err}"))
}

async fn find_ehentai_source_tag(
    pool: &Pool<Sqlite>,
    archive_id: &str,
) -> Result<Option<(String, String)>, String> {
    let source_tags = sqlx::query_scalar::<_, String>(
        "SELECT t.name FROM tags t INNER JOIN archive_tags at ON at.tag_id = t.id WHERE at.archive_id = ? AND lower(t.namespace) = 'source'",
    )
    .bind(archive_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("读取漫画来源标签失败: {err}"))?;
    Ok(source_tags
        .iter()
        .find_map(|source_tag| parse_gallery_reference(source_tag)))
}

async fn archive_has_ehentai_source(pool: &Pool<Sqlite>, archive_id: &str) -> Result<bool, String> {
    Ok(find_ehentai_source_tag(pool, archive_id).await?.is_some())
}

async fn fetch_archive_execution_row(
    pool: &Pool<Sqlite>,
    archive_id: &str,
) -> Result<ArchiveExecutionRow, String> {
    sqlx::query_as::<_, ArchiveExecutionRow>(
        "SELECT id, title, path, created_at FROM archives WHERE id = ? LIMIT 1",
    )
    .bind(archive_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("读取档案信息失败: {err}"))?
    .ok_or_else(|| format!("archive `{archive_id}` 不存在"))
}

fn extract_embedded_files_from_input(request: &PluginExecuteRequest) -> Vec<String> {
    let Some(input) = request.input.as_ref() else {
        return Vec::new();
    };

    if let Some(files) = input
        .get("embedded_files")
        .or_else(|| input.get("embeddedFiles"))
        .and_then(JsonValue::as_array)
    {
        return files
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .collect();
    }

    Vec::new()
}

fn list_embedded_files_from_archive(path: &str) -> Vec<String> {
    let extension = FsPath::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if !matches!(extension.as_str(), "cbz" | "zip") {
        return Vec::new();
    }

    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for idx in 0..zip.len().min(2048) {
        if let Ok(entry) = zip.by_index(idx) {
            names.push(entry.name().to_string());
        }
    }
    names
}

fn parse_tag_copier_request(request: &PluginExecuteRequest) -> Option<TagCopierRequest> {
    let mut tags = Vec::new();

    if let Some(input) = request.input.as_ref() {
        let tag_array = input
            .get("tag_copier_request")
            .and_then(|v| v.get("tags"))
            .or_else(|| input.get("tags"))
            .and_then(JsonValue::as_array);

        if let Some(tag_array) = tag_array {
            for raw_tag in tag_array {
                if let Some(tag) = parse_tag_from_json(raw_tag) {
                    tags.push(tag);
                }
            }
        }
    }

    if tags.is_empty() {
        if let Some(raw) = request.oneshot_param.as_deref() {
            for token in raw.split(',') {
                if let Some(tag) = parse_tag_literal(token) {
                    tags.push(tag);
                }
            }
        }
    }

    if tags.is_empty() {
        None
    } else {
        Some(TagCopierRequest { tags })
    }
}

fn extract_comicinfo_xml_from_input(request: &PluginExecuteRequest) -> Option<String> {
    request
        .input
        .as_ref()
        .and_then(|input| {
            input
                .get("comicinfo_xml")
                .or_else(|| input.get("comicinfoXml"))
        })
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|xml| !xml.is_empty())
        .map(ToOwned::to_owned)
}

fn read_comicinfo_xml_from_archive(path: &str) -> Option<String> {
    let extension = FsPath::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "cbz" | "zip") {
        return None;
    }

    let file = File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    for idx in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(idx) else {
            continue;
        };
        let file_name = entry.name().rsplit(['/', '\\']).next().unwrap_or_default();
        if !file_name.eq_ignore_ascii_case("ComicInfo.xml") {
            continue;
        }

        let mut xml = String::new();
        if entry.read_to_string(&mut xml).is_ok() {
            let trimmed = xml.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        return None;
    }

    None
}

fn parse_tag_from_json(raw: &JsonValue) -> Option<TagProposal> {
    if let Some(tag_text) = raw.as_str() {
        return parse_tag_literal(tag_text);
    }

    let obj = raw.as_object()?;
    let namespace = obj
        .get("namespace")
        .and_then(JsonValue::as_str)
        .unwrap_or("general")
        .trim();
    let value = obj
        .get("value")
        .or_else(|| obj.get("name"))
        .or_else(|| obj.get("tag"))
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .trim();

    if value.is_empty() {
        return None;
    }

    Some(TagProposal::manual(
        if namespace.is_empty() {
            "general"
        } else {
            namespace
        },
        value,
    ))
}

fn parse_tag_literal(raw: &str) -> Option<TagProposal> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (namespace, value) = match trimmed.split_once(':') {
        Some((ns, val)) if !val.trim().is_empty() => {
            let ns = ns.trim();
            let namespace = if ns.is_empty() { "general" } else { ns };
            (namespace.to_string(), val.trim().to_string())
        }
        _ => ("general".to_string(), trimmed.to_string()),
    };

    if value.is_empty() {
        return None;
    }

    Some(TagProposal::manual(namespace, value))
}

async fn persist_builtin_output(
    pool: &Pool<Sqlite>,
    plugin_id: &str,
    archive: &ArchiveExecutionRow,
    output: crate::plugins::PluginOutput,
) -> Result<BuiltinPersistStats, String> {
    let mut stats = BuiltinPersistStats {
        notes: output.notes.clone(),
        ..Default::default()
    };

    if let Some(next_title) = output
        .metadata
        .title
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        if !archive.title.eq_ignore_ascii_case(next_title) {
            sqlx::query("UPDATE archives SET title = ?, updated_at = ? WHERE id = ?")
                .bind(next_title)
                .bind(Utc::now())
                .bind(&archive.id)
                .execute(pool)
                .await
                .map_err(|err| format!("更新档案标题失败: {err}"))?;
            stats.title_updated = true;
        }
    }

    let mut existing_tags = load_existing_archive_tags(pool, &archive.id).await?;

    for raw_tag in output.tags {
        let namespace = raw_tag.namespace.trim();
        let value = raw_tag.value.trim();
        if namespace.is_empty() || value.is_empty() {
            stats.tags_skipped += 1;
            stats
                .notes
                .push("跳过空 namespace/value 的标签提案".to_string());
            continue;
        }

        let incoming = TagProposal {
            namespace: namespace.to_string(),
            value: value.to_string(),
            source_plugin: raw_tag.source_plugin,
            provenance: raw_tag.provenance,
        };

        let same_namespace_indexes: Vec<usize> = existing_tags
            .iter()
            .enumerate()
            .filter_map(|(idx, existing)| {
                existing
                    .namespace
                    .eq_ignore_ascii_case(&incoming.namespace)
                    .then_some(idx)
            })
            .collect();

        let has_same_value = same_namespace_indexes.iter().any(|idx| {
            existing_tags[*idx]
                .tag_name
                .eq_ignore_ascii_case(&incoming.value)
        });

        let mut should_apply = true;
        let mut manual_review_required = false;
        let mut should_replace = false;

        if !is_multi_value_namespace(&incoming.namespace)
            && !same_namespace_indexes.is_empty()
            && !has_same_value
        {
            for idx in &same_namespace_indexes {
                let existing = &existing_tags[*idx];
                let existing_proposal = TagProposal {
                    namespace: existing.namespace.clone(),
                    value: existing.tag_name.clone(),
                    source_plugin: "database-existing",
                    provenance: existing.provenance,
                };

                match DEFAULT_TAG_CONFLICT_RESOLVER.resolve(&existing_proposal, &incoming) {
                    TagConflictDecision::KeepExisting => {}
                    TagConflictDecision::ReplaceWithIncoming => {
                        should_replace = true;
                    }
                    TagConflictDecision::RequireManualReview => {
                        manual_review_required = true;
                    }
                }
            }

            if manual_review_required {
                should_apply = false;
            } else if should_replace {
                for idx in same_namespace_indexes.iter().rev() {
                    let removed = existing_tags.remove(*idx);
                    sqlx::query("DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?")
                        .bind(&archive.id)
                        .bind(&removed.tag_id)
                        .execute(pool)
                        .await
                        .map_err(|err| format!("删除冲突标签失败: {err}"))?;
                }
            } else {
                should_apply = false;
            }
        }

        let tag_id = ensure_tag_id(pool, &incoming.namespace, &incoming.value).await?;

        if should_apply {
            sqlx::query("INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
                .bind(&archive.id)
                .bind(&tag_id)
                .execute(pool)
                .await
                .map_err(|err| format!("写入 archive_tags 失败: {err}"))?;

            insert_plugin_tag_audit(
                pool,
                plugin_id,
                &archive.id,
                &tag_id,
                incoming.provenance,
                Some(true),
            )
            .await?;

            if !existing_tags.iter().any(|existing| {
                existing.namespace.eq_ignore_ascii_case(&incoming.namespace)
                    && existing.tag_name.eq_ignore_ascii_case(&incoming.value)
            }) {
                existing_tags.push(ArchiveExistingTag {
                    tag_id: tag_id.clone(),
                    tag_name: incoming.value.clone(),
                    namespace: incoming.namespace.clone(),
                    provenance: incoming.provenance,
                });
            }

            stats.tags_applied += 1;
            continue;
        }

        insert_plugin_tag_audit(
            pool,
            plugin_id,
            &archive.id,
            &tag_id,
            incoming.provenance,
            Some(false),
        )
        .await?;

        stats.tags_skipped += 1;
        if manual_review_required {
            stats.manual_review += 1;
            stats.notes.push(format!(
                "Tag conflict requires manual review: namespace='{}', incoming='{}'",
                incoming.namespace, incoming.value
            ));
        } else {
            stats.notes.push(format!(
                "Tag conflict kept existing value: namespace='{}', incoming='{}'",
                incoming.namespace, incoming.value
            ));
        }
    }

    Ok(stats)
}

fn is_multi_value_namespace(namespace: &str) -> bool {
    matches!(
        namespace,
        "filename_token"
            | "genre"
            | "character"
            | "team"
            | "location"
            | "artist"
            | "group"
            | "parody"
            | "female"
            | "male"
            | "mixed"
            | "other"
            | "cosplayer"
    )
}

async fn load_existing_archive_tags(
    pool: &Pool<Sqlite>,
    archive_id: &str,
) -> Result<Vec<ArchiveExistingTag>, String> {
    let rows = sqlx::query_as::<_, ArchiveExistingTagRow>(
        r#"
        SELECT
            t.id AS tag_id,
            t.name AS tag_name,
            t.namespace AS namespace,
            EXISTS(
                SELECT 1
                FROM plugin_tags pt
                WHERE pt.archive_id = at.archive_id
                  AND pt.tag_id = at.tag_id
                LIMIT 1
            ) AS plugin_generated
        FROM archive_tags at
        INNER JOIN tags t ON t.id = at.tag_id
        WHERE at.archive_id = ?
        ORDER BY t.namespace, t.name
        "#,
    )
    .bind(archive_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("读取现有标签失败: {err}"))?;

    Ok(rows
        .into_iter()
        .map(|row| ArchiveExistingTag {
            tag_id: row.tag_id,
            tag_name: row.tag_name,
            namespace: row.namespace,
            provenance: if row.plugin_generated != 0 {
                TagProvenance::PluginDeterministic
            } else {
                TagProvenance::UserManual
            },
        })
        .collect())
}

async fn ensure_tag_id(
    pool: &Pool<Sqlite>,
    namespace: &str,
    value: &str,
) -> Result<String, String> {
    let candidate_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?)")
        .bind(&candidate_id)
        .bind(value)
        .bind(namespace)
        .execute(pool)
        .await
        .map_err(|err| format!("写入 tags 失败: {err}"))?;

    sqlx::query_scalar::<_, String>("SELECT id FROM tags WHERE name = ? AND namespace = ? LIMIT 1")
        .bind(value)
        .bind(namespace)
        .fetch_optional(pool)
        .await
        .map_err(|err| format!("查询 tags 失败: {err}"))?
        .ok_or_else(|| format!("标签查询为空: {namespace}:{value}"))
}

async fn insert_plugin_tag_audit(
    pool: &Pool<Sqlite>,
    plugin_id: &str,
    archive_id: &str,
    tag_id: &str,
    provenance: TagProvenance,
    approved: Option<bool>,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO plugin_tags (
            id,
            plugin_id,
            archive_id,
            tag_id,
            confidence,
            approved
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(plugin_id)
    .bind(archive_id)
    .bind(tag_id)
    .bind(provenance_confidence(provenance))
    .bind(approved)
    .execute(pool)
    .await
    .map_err(|err| format!("写入 plugin_tags 审计失败: {err}"))?;

    Ok(())
}

fn provenance_confidence(provenance: TagProvenance) -> Option<f64> {
    match provenance {
        TagProvenance::UserManual => None,
        TagProvenance::PluginDeterministic => Some(1.0),
        TagProvenance::PluginHeuristic => Some(0.7),
    }
}

async fn update_plugin_execution_result(
    pool: &Pool<Sqlite>,
    execution_id: &str,
    status: &str,
    output_summary: Option<String>,
    error_message: Option<String>,
    duration_ms: i64,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE plugin_executions
        SET
            status = ?,
            output_summary = ?,
            error_message = ?,
            duration_ms = ?,
            completed_at = ?
        WHERE id = ?
        "#,
    )
    .bind(status)
    .bind(output_summary)
    .bind(error_message)
    .bind(duration_ms)
    .bind(Utc::now())
    .bind(execution_id)
    .execute(pool)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            "更新插件执行结果失败",
        )
    })?;
    Ok(())
}

fn saturating_duration_ms(duration_ms: u128) -> i64 {
    duration_ms.min(i64::MAX as u128) as i64
}

fn collect_execution_targets(
    archive_id_from_path: Option<String>,
    request: &PluginExecuteRequest,
) -> Vec<Option<String>> {
    let mut seen = HashSet::new();
    let mut targets: Vec<Option<String>> = Vec::new();

    let mut push_archive = |archive_id: String| {
        if seen.insert(archive_id.clone()) {
            targets.push(Some(archive_id));
        }
    };

    if let Some(archive_id) = archive_id_from_path {
        push_archive(archive_id);
    }
    if let Some(archive_id) = &request.archive_id {
        push_archive(archive_id.clone());
    }
    for archive_id in &request.archive_ids {
        push_archive(archive_id.clone());
    }

    if targets.is_empty() {
        targets.push(None);
    }

    targets
}

fn build_input_summary(archive_id: Option<&str>, request: &PluginExecuteRequest) -> Option<String> {
    if archive_id.is_none()
        && request.oneshot_param.is_none()
        && request.input.is_none()
        && request.archive_id.is_none()
        && request.archive_ids.is_empty()
    {
        return None;
    }

    serde_json::to_string(&json!({
        "archive_id": archive_id,
        "oneshot_param": request.oneshot_param,
        "input": request.input
    }))
    .ok()
}

fn normalize_pagination(limit: Option<u32>, offset: Option<u32>) -> (u32, u32) {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0);
    (limit, offset)
}

fn check_plugin_cooldown(
    plugin_id: &str,
    last_executed_at: Option<DateTime<Utc>>,
    cooldown: Option<u32>,
) -> Result<(), ApiError> {
    let cooldown_secs = cooldown.unwrap_or(0);
    if cooldown_secs == 0 {
        return Ok(());
    }

    if let Some(last) = last_executed_at {
        let now = Utc::now();
        let elapsed_secs = (now - last).num_seconds();
        if elapsed_secs >= 0 && elapsed_secs < cooldown_secs as i64 {
            let remaining_secs = cooldown_secs - elapsed_secs as u32;
            return Err(api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "plugin_cooldown",
                format!("插件 `{plugin_id}` 正在冷却中，还需等待 {remaining_secs} 秒后再试"),
            ));
        }
    }

    Ok(())
}

fn parse_manifest_meta(
    plugin_id: &str,
    manifest: Option<JsonValue>,
) -> Result<ParsedManifestMeta, ApiError> {
    let manifest = manifest.ok_or_else(|| {
        api_error(
            StatusCode::BAD_REQUEST,
            "manifest_missing",
            format!("插件 `{plugin_id}` 缺少 manifest，无法读取配置 schema 或执行冷却信息"),
        )
    })?;

    let obj = manifest.as_object().ok_or_else(|| {
        api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "manifest_invalid",
            format!("插件 `{plugin_id}` 的 manifest 不是合法对象"),
        )
    })?;

    let cooldown = match obj.get("cooldown") {
        None | Some(JsonValue::Null) => None,
        Some(JsonValue::Number(value)) => {
            let raw = value.as_u64().ok_or_else(|| {
                api_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "manifest_invalid",
                    format!("插件 `{plugin_id}` 的 cooldown 必须是非负整数"),
                )
            })?;

            if raw > u32::MAX as u64 {
                return Err(api_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "manifest_invalid",
                    format!("插件 `{plugin_id}` 的 cooldown 超出允许范围"),
                ));
            }

            Some(raw as u32)
        }
        _ => {
            return Err(api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "manifest_invalid",
                format!("插件 `{plugin_id}` 的 cooldown 必须是数字"),
            ))
        }
    };

    let config_schema = obj
        .get("config_schema")
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(Default::default()));

    Ok(ParsedManifestMeta {
        cooldown,
        config_schema,
    })
}

fn default_manifest_for_uploaded_plugin(
    plugin_id: &str,
    plugin_name: &str,
    plugin_version: &str,
    plugin_type: &str,
) -> JsonValue {
    json!({
        "id": plugin_id,
        "name": plugin_name,
        "version": plugin_version,
        "plugin_type": plugin_type,
        "manifest_version": 1,
        "plugin_api_version": 1,
        "cooldown": null,
        "config_schema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

fn api_error(status: StatusCode, code: &'static str, message: impl Into<String>) -> ApiError {
    (
        status,
        Json(PluginApiError {
            code,
            message: message.into(),
        }),
    )
}

fn infer_plugin_name(filename: &str) -> String {
    let base = filename
        .strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".tgz"))
        .unwrap_or(filename);

    let mut normalized = String::with_capacity(base.len());
    let mut prev_dash = false;

    for ch in base.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };

        if mapped == '-' {
            if !prev_dash {
                normalized.push('-');
                prev_dash = true;
            }
        } else {
            normalized.push(mapped);
            prev_dash = false;
        }
    }

    let trimmed = normalized.trim_matches('-');
    if trimmed.is_empty() {
        "plugin".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_plugin_runtime_schema(pool: &Pool<Sqlite>) {
        sqlx::query(
            r#"
            CREATE TABLE plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                manifest_version INTEGER NOT NULL DEFAULT 1,
                plugin_api_version INTEGER NOT NULL DEFAULT 1,
                plugin_type TEXT NOT NULL,
                description TEXT,
                author TEXT,
                icon TEXT,
                enabled BOOLEAN NOT NULL DEFAULT FALSE,
                config TEXT,
                permissions TEXT,
                manifest TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                last_executed_at DATETIME,
                execution_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create plugins");

        sqlx::query(
            r#"
            CREATE TABLE archives (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archives");

        sqlx::query(
            r#"
            CREATE TABLE tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                namespace TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(name, namespace)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create tags");

        sqlx::query(
            r#"
            CREATE TABLE archive_tags (
                archive_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (archive_id, tag_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archive_tags");

        sqlx::query(
            r#"
            CREATE TABLE plugin_executions (
                id TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                archive_id TEXT,
                execution_type TEXT NOT NULL,
                status TEXT NOT NULL,
                input_summary TEXT,
                output_summary TEXT,
                error_message TEXT,
                duration_ms INTEGER,
                started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create plugin_executions");

        sqlx::query(
            r#"
            CREATE TABLE plugin_tags (
                id TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                archive_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                confidence REAL,
                approved BOOLEAN,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create plugin_tags");
    }

    #[tokio::test]
    async fn auto_execute_runs_enabled_builtin_metadata_plugins() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        setup_plugin_runtime_schema(&pool).await;

        sqlx::query(
            r#"
            INSERT INTO plugins (
                id, name, version, manifest_version, plugin_api_version, plugin_type,
                enabled, manifest, created_at, updated_at
            )
            VALUES (?, ?, ?, 1, 1, 'metadata', 1, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(BUILTIN_FILENAME_PARSER_ID)
        .bind("Filename Parser")
        .bind("1.0.0")
        .execute(&pool)
        .await
        .expect("insert plugin");

        sqlx::query(
            r#"
            INSERT INTO archives (
                id, title, path, file_hash, file_size, page_count, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind("archive-1")
        .bind("Old Title")
        .bind("/tmp/[Group] My.Book v01.cbz")
        .bind("hash-1")
        .bind(1234_i64)
        .bind(10_i32)
        .execute(&pool)
        .await
        .expect("insert archive");

        auto_execute_enabled_metadata_plugins_for_archive(&pool, "archive-1").await;

        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM plugin_executions WHERE plugin_id = ? LIMIT 1")
                .bind(BUILTIN_FILENAME_PARSER_ID)
                .fetch_optional(&pool)
                .await
                .expect("query execution");
        assert_eq!(status.as_deref(), Some("success"));

        let title: String = sqlx::query_scalar("SELECT title FROM archives WHERE id = ?")
            .bind("archive-1")
            .fetch_one(&pool)
            .await
            .expect("query title");
        assert_eq!(title, "My Book v01");

        let tag_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM archive_tags WHERE archive_id = ?")
                .bind("archive-1")
                .fetch_one(&pool)
                .await
                .expect("query archive tags");
        assert!(
            tag_count > 0,
            "filename parser should produce at least one tag"
        );
    }

    #[tokio::test]
    async fn plugin_execution_authorization_checks_every_requested_archive() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        setup_plugin_runtime_schema(&pool).await;
        sqlx::query("CREATE TABLE user_paths (user_id TEXT NOT NULL, path TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("create user paths");
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES
             ('allowed', 'Allowed', '/library/allowed/book.cbz', 'hash-a', 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
             ('denied', 'Denied', '/private/book.cbz', 'hash-b', 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .expect("insert archives");
        sqlx::query("INSERT INTO user_paths (user_id, path) VALUES ('reader', '/library/allowed')")
            .execute(&pool)
            .await
            .expect("insert user path");

        let reader = AuthInfo {
            user_id: "reader".to_string(),
            role: "user".to_string(),
        };
        let request = PluginExecuteRequest {
            archive_ids: vec!["allowed".to_string(), "denied".to_string()],
            ..Default::default()
        };
        let error = authorize_plugin_execution_targets(&pool, &reader, None, &request)
            .await
            .expect_err("one denied target must reject the request");
        assert_eq!(error.0, StatusCode::FORBIDDEN);

        let admin = AuthInfo {
            user_id: "admin".to_string(),
            role: "admin".to_string(),
        };
        authorize_plugin_execution_targets(&pool, &admin, None, &request)
            .await
            .expect("admin can access every archive");
    }
}

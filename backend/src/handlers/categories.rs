use crate::middleware::auth::AuthInfo;
use crate::models::{
    AddArchivesToCategoryRequest, Archive, Category, CategoryBatchDeleteResult,
    CategoryDeletePreview, CategorySearchParams, CreateCategoryRequest,
    CreateDynamicCategoryRequest, DynamicCategory, PaginatedResponse, SearchRequest,
    UpdateCategoryRequest,
};
use crate::services::{
    ArchiveCacheService, ArchiveDeleteTarget, ArchiveDeletionService, ArchiveFilters,
    ArchiveQueryService, QueryOptions,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;

fn dynamic_category_filters(
    search_criteria: Option<&str>,
    category_id: &str,
) -> Result<ArchiveFilters, StatusCode> {
    let dynamic_params = serde_json::from_str::<CategorySearchParams>(
        search_criteria.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
    )
    .map_err(|e| {
        tracing::warn!(
            "Invalid search criteria for dynamic category {}: {}",
            category_id,
            e
        );
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    // Sorting alone does not narrow a category and must not authorize a whole-library delete.
    if !dynamic_params.has_filter_criteria() {
        tracing::warn!(
            "Rejected whole-library deletion through dynamic category {}",
            category_id
        );
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    Ok(ArchiveFilters::from_search_request(
        &dynamic_params.into_search_request(None, None),
    ))
}

async fn resolve_category_delete_targets(
    pool: &Pool<Sqlite>,
    category_id: &str,
    user_id: &str,
) -> Result<Option<(String, Vec<ArchiveDeleteTarget>)>, StatusCode> {
    let category = sqlx::query!(
        "SELECT category_type, search_criteria FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to load category {} for deletion: {}",
            category_id,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(category) = category else {
        return Ok(None);
    };

    let targets = if category.category_type == "static" {
        sqlx::query!(
            "SELECT a.id, a.path
             FROM archives a
             INNER JOIN category_archives ca ON a.id = ca.archive_id
             WHERE ca.category_id = ?",
            category_id
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to resolve static category {} delete targets: {}",
                category_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_iter()
        .filter_map(|row| row.id.map(|id| ArchiveDeleteTarget { id, path: row.path }))
        .collect()
    } else {
        ArchiveQueryService::new(pool.clone())
            .query_delete_targets(
                dynamic_category_filters(category.search_criteria.as_deref(), category_id)?,
                QueryOptions {
                    random: false,
                    include_tags: false,
                    user_id: Some(user_id.to_string()),
                },
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to resolve dynamic category {} delete targets: {}",
                    category_id,
                    e
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    Ok(Some((category.category_type, targets)))
}

async fn count_category_delete_targets(
    pool: &Pool<Sqlite>,
    category_id: &str,
    user_id: &str,
) -> Result<Option<(String, u64)>, StatusCode> {
    let category = sqlx::query!(
        "SELECT category_type, search_criteria FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to load category {} deletion preview: {}",
            category_id,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(category) = category else {
        return Ok(None);
    };

    let matched = if category.category_type == "static" {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM archives a
             INNER JOIN category_archives ca ON a.id = ca.archive_id
             WHERE ca.category_id = ?",
        )
        .bind(category_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to count static category {} delete targets: {}",
                category_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })? as u64
    } else {
        ArchiveQueryService::new(pool.clone())
            .count_matching_archives(
                dynamic_category_filters(category.search_criteria.as_deref(), category_id)?,
                QueryOptions {
                    random: false,
                    include_tags: false,
                    user_id: Some(user_id.to_string()),
                },
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to count dynamic category {} delete targets: {}",
                    category_id,
                    e
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    Ok(Some((category.category_type, matched)))
}

fn empty_archive_response(params: &SearchRequest) -> PaginatedResponse<Archive> {
    PaginatedResponse {
        data: vec![],
        page_numb: params.page_numb.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
        total: 0,
        has_next: false,
    }
}

// 获取所有分类（静态+动态）
pub async fn get_categories(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<Vec<Category>>, StatusCode> {
    let rows = sqlx::query!(
        "SELECT id, name, description, category_type, created_at, updated_at FROM categories ORDER BY created_at"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting categories: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 批量获取所有分类的档案数量，避免N+1查询
    let archive_count_rows = sqlx::query!(
        "SELECT category_id, COUNT(*) as count FROM category_archives GROUP BY category_id"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archive counts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let archive_count_map: HashMap<String, i32> = archive_count_rows
        .into_iter()
        .map(|r| (r.category_id, r.count as i32))
        .collect();

    let mut categories = Vec::new();
    for row in rows {
        // 计算档案数量
        let archive_count = if row.category_type == "static" {
            // 静态分类：从预加载的映射中查找数量
            archive_count_map
                .get(&row.id.clone().unwrap_or_default())
                .copied()
                .unwrap_or(0)
        } else {
            // 动态分类：根据搜索条件计算（暂时返回0，完整实现需要解析search_criteria）
            0
        };

        categories.push(Category {
            id: row.id.unwrap_or_default(),
            name: row.name,
            description: row.description,
            is_static: row.category_type == "static",
            archive_count,
            created_at: chrono::DateTime::from_naive_utc_and_offset(row.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(row.updated_at, chrono::Utc),
        });
    }

    Ok(Json(categories))
}

// 创建静态分类
pub async fn create_category(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Json<Category>, StatusCode> {
    let category_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    sqlx::query!(
        "INSERT INTO categories (id, name, description, category_type, created_at, updated_at) VALUES (?, ?, ?, 'static', ?, ?)",
        category_id,
        request.name,
        request.description,
        now,
        now
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating category: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let category = Category {
        id: category_id,
        name: request.name,
        description: request.description,
        is_static: true,
        archive_count: 0,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(category))
}

// 创建动态分类
pub async fn create_dynamic_category(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<CreateDynamicCategoryRequest>,
) -> Result<Json<DynamicCategory>, StatusCode> {
    let search_params_json =
        serde_json::to_string(&request.search_params).map_err(|_| StatusCode::BAD_REQUEST)?;

    let category_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    sqlx::query!(
        "INSERT INTO categories (id, name, description, category_type, search_criteria, created_at, updated_at) VALUES (?, ?, ?, 'dynamic', ?, ?, ?)",
        category_id,
        request.name,
        request.description,
        search_params_json,
        now,
        now
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating dynamic category: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dynamic_category = DynamicCategory {
        id: category_id,
        name: request.name,
        description: request.description,
        search_params: search_params_json,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(dynamic_category))
}

// 获取分类下的漫画
pub async fn get_category_archives(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
    Query(params): Query<SearchRequest>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    use crate::services::{
        ArchiveFilters, ArchiveQueryService, PaginationParams, QueryOptions, SearchService,
    };

    // 首先检查分类是否存在和类型
    let category = sqlx::query!(
        "SELECT category_type, search_criteria FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting category: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(category) = category else {
        tracing::debug!(
            "Category {} not found when listing archives, returning empty result",
            category_id
        );
        return Ok(Json(empty_archive_response(&params)));
    };

    let query_service = ArchiveQueryService::new(pool.clone());

    if category.category_type == "static" {
        // 静态分类：使用统一查询服务，通过archive_ids过滤

        // 获取该分类下的所有档案ID
        let category_archive_ids = sqlx::query!(
            "SELECT archive_id FROM category_archives WHERE category_id = ?",
            category_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching category archive IDs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_iter()
        .map(|row| row.archive_id)
        .collect::<Vec<String>>();

        if category_archive_ids.is_empty() {
            // 分类下没有档案
            return Ok(Json(empty_archive_response(&params)));
        }

        let filters = ArchiveFilters {
            archive_ids: Some(category_archive_ids),
            ..ArchiveFilters::from_search_request(&params)
        };
        let pagination = PaginationParams::from_search_request(&params);
        let options = QueryOptions {
            random: false,
            include_tags: true,
            user_id: Some(auth.user_id.clone()),
        };

        match query_service
            .query_archives(filters, pagination, options)
            .await
        {
            Ok(result) => Ok(Json(result)),
            Err(e) => {
                tracing::error!("Query error for static category: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // 动态分类：根据存储的搜索条件查询漫画
        if let Some(search_criteria) = &category.search_criteria {
            match serde_json::from_str::<CategorySearchParams>(search_criteria) {
                Ok(dynamic_params) => {
                    // 合并传入的分页参数
                    let dynamic_params =
                        dynamic_params.into_search_request(params.page_numb, params.page_size);

                    let search_service = SearchService::new(pool);
                    match search_service
                        .search_archives(dynamic_params, &auth.user_id)
                        .await
                    {
                        Ok(result) => Ok(Json(result)),
                        Err(e) => {
                            tracing::error!("Search error for dynamic category: {}", e);
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to parse search criteria for category {}: {}",
                        category_id,
                        e
                    );
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        } else {
            // 没有搜索条件的动态分类，返回空结果
            Ok(Json(empty_archive_response(&params)))
        }
    }
}

// 更新分类信息
pub async fn update_category(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<Json<Category>, StatusCode> {
    // 检查分类是否存在
    let category = sqlx::query!(
        "SELECT id, name, description, category_type, created_at, updated_at FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching category {}: {}", category_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let category = category.ok_or_else(|| {
        tracing::warn!("Category not found: {}", category_id);
        StatusCode::NOT_FOUND
    })?;

    let updated_name = request.name.unwrap_or(category.name);
    let updated_description = request.description.or(category.description);
    let now = chrono::Utc::now();

    // 更新分类信息
    sqlx::query!(
        "UPDATE categories SET name = ?, description = ?, updated_at = ? WHERE id = ?",
        updated_name,
        updated_description,
        now,
        category_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating category {}: {}", category_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 计算档案数量
    let archive_count = if category.category_type == "static" {
        sqlx::query!(
            "SELECT COUNT(*) as count FROM category_archives WHERE category_id = ?",
            category_id
        )
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as i32
    } else {
        0 // 动态分类暂时返回0
    };

    let updated_category = Category {
        id: category_id,
        name: updated_name,
        description: updated_description,
        is_static: category.category_type == "static",
        archive_count,
        created_at: chrono::DateTime::from_naive_utc_and_offset(category.created_at, chrono::Utc),
        updated_at: now,
    };

    tracing::info!(
        "Updated category: {} ({})",
        updated_category.name,
        updated_category.id
    );
    Ok(Json(updated_category))
}

// 删除分类
pub async fn delete_category(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // 检查分类是否存在
    let category = sqlx::query!("SELECT id, name FROM categories WHERE id = ?", category_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching category {}: {}", category_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Category not found: {}", category_id);
            StatusCode::NOT_FOUND
        })?;

    // 删除分类（级联删除会自动处理category_archives关联表）
    sqlx::query!("DELETE FROM categories WHERE id = ?", category_id)
        .execute(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting category {}: {}", category_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Deleted category: {} ({})", category.name, category_id);
    Ok(StatusCode::NO_CONTENT)
}

// 向静态分类中添加漫画
pub async fn add_archives_to_category(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
    Json(request): Json<AddArchivesToCategoryRequest>,
) -> Result<StatusCode, StatusCode> {
    if request.archive_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 检查分类是否存在且为静态分类
    let category = sqlx::query!(
        "SELECT category_type, name FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching category {}: {}", category_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(category) = category else {
        tracing::debug!(
            "Category {} not found when adding archives, treating as no-op",
            category_id
        );
        return Ok(StatusCode::OK);
    };

    if category.category_type != "static" {
        tracing::warn!(
            "Cannot add archives to non-static category: {}",
            category_id
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // 批量插入档案-分类关联
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut inserted_count = 0;
    for archive_id in &request.archive_ids {
        // 检查档案是否存在
        let archive_exists = sqlx::query!("SELECT id FROM archives WHERE id = ?", archive_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if archive_exists.is_none() {
            tracing::warn!("Archive not found: {}", archive_id);
            continue;
        }

        // 插入关联（忽略重复）
        match sqlx::query!(
            "INSERT OR IGNORE INTO category_archives (category_id, archive_id) VALUES (?, ?)",
            category_id,
            archive_id
        )
        .execute(&mut *tx)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    inserted_count += 1;
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to add archive {} to category {}: {}",
                    archive_id,
                    category_id,
                    e
                );
            }
        }
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(
        "Added {} archives to category {} ({})",
        inserted_count,
        category.name,
        category_id
    );
    Ok(StatusCode::OK)
}

// 从静态分类中移除漫画
pub async fn remove_archives_from_category(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
    Json(request): Json<AddArchivesToCategoryRequest>,
) -> Result<StatusCode, StatusCode> {
    if request.archive_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 检查分类是否存在且为静态分类
    let category = sqlx::query!(
        "SELECT category_type, name FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching category {}: {}", category_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(category) = category else {
        tracing::debug!(
            "Category {} not found when removing archives, treating as no-op",
            category_id
        );
        return Ok(StatusCode::OK);
    };

    if category.category_type != "static" {
        tracing::warn!(
            "Cannot remove archives from non-static category: {}",
            category_id
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // 批量删除档案-分类关联
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut removed_count = 0;
    for archive_id in &request.archive_ids {
        match sqlx::query!(
            "DELETE FROM category_archives WHERE category_id = ? AND archive_id = ?",
            category_id,
            archive_id
        )
        .execute(&mut *tx)
        .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    removed_count += 1;
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to remove archive {} from category {}: {}",
                    archive_id,
                    category_id,
                    e
                );
            }
        }
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(
        "Removed {} archives from category {} ({})",
        removed_count,
        category.name,
        category_id
    );
    Ok(StatusCode::OK)
}

/// DELETE /api/v1/categories/prune - 清理空的分类
pub async fn prune_empty_categories(
    State(pool): State<Pool<Sqlite>>,
) -> Result<StatusCode, StatusCode> {
    // 删除没有关联任何档案的静态分类
    let deleted_static = sqlx::query!(
        r#"
        DELETE FROM categories 
        WHERE category_type = 'static' 
        AND id NOT IN (
            SELECT DISTINCT category_id FROM category_archives
        )
        "#
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error pruning empty static categories: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        "Pruned {} empty static categories",
        deleted_static.rows_affected()
    );
    Ok(StatusCode::OK)
}

/// GET /api/v1/archives/:id/categories - 获取档案所属的分类
pub async fn get_archive_categories(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    // 检查档案是否存在
    let archive_exists = sqlx::query!("SELECT id FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking archive {}: {}", archive_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if archive_exists.is_none() {
        tracing::debug!(
            "Archive {} not found when listing categories, returning empty result",
            archive_id
        );
        return Ok(Json(vec![]));
    }

    // 获取档案所属的所有静态分类ID
    let category_ids = sqlx::query!(
        "SELECT category_id FROM category_archives WHERE archive_id = ?",
        archive_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Database error fetching archive categories for {}: {}",
            archive_id,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .into_iter()
    .map(|row| row.category_id)
    .collect::<Vec<String>>();

    Ok(Json(category_ids))
}

/// DELETE /api/v1/categories/:id/archives/batch-delete - 批量删除分类下的漫画
pub async fn batch_delete_category_archives(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<CategoryBatchDeleteResult>, StatusCode> {
    let Some((category_type, archive_rows)) =
        resolve_category_delete_targets(&pool, &category_id, &auth.user_id).await?
    else {
        tracing::debug!(
            "Category {} not found during batch delete, treating as no-op",
            category_id
        );
        return Ok(Json(CategoryBatchDeleteResult {
            category_type: "unknown".to_string(),
            matched: 0,
            deleted: 0,
            failed: 0,
        }));
    };

    let summary = ArchiveDeletionService::new(pool, archive_cache)
        .delete_targets(
            &auth.user_id,
            archive_rows,
            "user initiated category batch deletion",
            "category_batch_delete",
        )
        .await
        .map_err(|e| {
            tracing::error!("Category {} batch deletion failed: {}", category_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "Batch deleted {} archives from category {}",
        summary.deleted,
        category_id
    );
    Ok(Json(CategoryBatchDeleteResult {
        category_type,
        matched: summary.matched,
        deleted: summary.deleted,
        failed: summary.failed,
    }))
}

/// GET /api/v1/categories/:id/archives/delete-preview - 预览分类批量删除数量
pub async fn preview_category_archive_deletion(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<CategoryDeletePreview>, StatusCode> {
    let (category_type, matched) =
        count_category_delete_targets(&pool, &category_id, &auth.user_id)
            .await?
            .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(CategoryDeletePreview {
        category_type,
        matched,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth::AuthInfo;
    use crate::services::archive::ArchiveCacheConfig;
    use axum::extract::{Extension, Query};
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_categories_schema(pool: &Pool<Sqlite>) {
        sqlx::query(
            r#"
            CREATE TABLE categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                category_type TEXT NOT NULL,
                search_criteria TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create categories");

        sqlx::query(
            r#"
            CREATE TABLE category_archives (
                category_id TEXT NOT NULL,
                archive_id TEXT NOT NULL,
                PRIMARY KEY (category_id, archive_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create category_archives");

        sqlx::query(
            r#"
            CREATE TABLE archives (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                title TEXT NOT NULL,
                subtitle TEXT,
                subtitle_language TEXT,
                file_size INTEGER NOT NULL DEFAULT 0,
                page_count INTEGER NOT NULL DEFAULT 0,
                file_hash TEXT NOT NULL DEFAULT '',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archives");

        sqlx::query(
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
        )
        .execute(pool)
        .await
        .expect("create tags");
        sqlx::query(
            "CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY (archive_id, tag_id))",
        )
        .execute(pool)
        .await
        .expect("create archive_tags");
        sqlx::query(
            "CREATE TABLE trash_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, original_path TEXT NOT NULL, trash_path TEXT, reason TEXT, rule_version TEXT, rule_id TEXT, evaluation_id TEXT, model_confidence REAL, metadata_json TEXT NOT NULL, operation_id TEXT, operation_type TEXT, decision_key TEXT, status TEXT NOT NULL, deleted_at DATETIME NOT NULL, expires_at DATETIME, restored_at DATETIME, cleanup_attempts INTEGER NOT NULL DEFAULT 0, last_cleanup_attempt_at DATETIME, last_cleanup_error TEXT, expired_at DATETIME)",
        )
        .execute(pool)
        .await
        .expect("create trash_entries");
        sqlx::query(
            "CREATE TABLE user_behavior_events (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT, event_type TEXT NOT NULL, event_key TEXT, page INTEGER, metadata_json TEXT NOT NULL, occurred_at DATETIME NOT NULL, created_at DATETIME NOT NULL, UNIQUE(user_id, event_key))",
        )
        .execute(pool)
        .await
        .expect("create user_behavior_events");
        sqlx::query(
            "CREATE TABLE archive_dispositions (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, disposition TEXT NOT NULL, reason TEXT, source TEXT NOT NULL, metadata_json TEXT NOT NULL, decision_key TEXT, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL)",
        )
        .execute(pool)
        .await
        .expect("create archive_dispositions");
    }

    fn test_auth_info() -> AuthInfo {
        AuthInfo {
            user_id: "user-1".to_string(),
            role: "admin".to_string(),
        }
    }

    fn test_search_request() -> SearchRequest {
        SearchRequest {
            query: None,
            tags: None,
            theme_ids: None,
            min_pages: None,
            max_pages: None,
            min_file_size: None,
            max_file_size: None,
            created_after: None,
            created_before: None,
            last_read_after: None,
            last_read_before: None,
            sort_by: Some("createdAt".to_string()),
            sort_order: Some("asc".to_string()),
            page_numb: Some(3),
            page_size: Some(30),
        }
    }

    fn test_cache_service() -> Arc<ArchiveCacheService> {
        Arc::new(ArchiveCacheService::new(ArchiveCacheConfig::default()))
    }

    #[tokio::test]
    async fn get_category_archives_returns_empty_for_missing_category() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_categories_schema(&pool).await;

        let response = get_category_archives(
            State(pool),
            Path("missing-category".to_string()),
            Query(test_search_request()),
            Extension(test_auth_info()),
        )
        .await
        .expect("empty result for missing category");

        assert!(response.0.data.is_empty());
        assert_eq!(response.0.page_numb, 3);
        assert_eq!(response.0.page_size, 30);
        assert_eq!(response.0.total, 0);
        assert!(!response.0.has_next);
    }

    #[tokio::test]
    async fn get_archive_categories_returns_empty_for_missing_archive() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_categories_schema(&pool).await;

        let response = get_archive_categories(State(pool), Path("missing-archive".to_string()))
            .await
            .expect("empty categories for missing archive");

        assert!(response.0.is_empty());
    }

    #[tokio::test]
    async fn batch_delete_category_archives_is_noop_for_missing_category() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_categories_schema(&pool).await;

        let status = batch_delete_category_archives(
            State(pool),
            Path("missing-category".to_string()),
            Extension(test_cache_service()),
            Extension(test_auth_info()),
        )
        .await
        .expect("missing category should be a no-op");

        assert_eq!(status.0.matched, 0);
        assert_eq!(status.0.deleted, 0);
        assert_eq!(status.0.failed, 0);
    }

    #[tokio::test]
    async fn batch_delete_dynamic_category_deletes_only_matching_archives() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_categories_schema(&pool).await;

        let test_dir = std::env::temp_dir().join(format!(
            "otamoryx-dynamic-delete-test-{}",
            uuid::Uuid::new_v4()
        ));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("create test directory");
        let matching_path = test_dir.join("matching.cbz");
        let other_path = test_dir.join("other.cbz");
        tokio::fs::write(&matching_path, b"matching")
            .await
            .expect("create matching archive file");
        tokio::fs::write(&other_path, b"other")
            .await
            .expect("create other archive file");

        let search_criteria = serde_json::json!({ "query": "match" }).to_string();
        sqlx::query(
            "INSERT INTO categories
             (id, name, category_type, search_criteria, created_at, updated_at)
             VALUES ('dynamic-1', 'Matches', 'dynamic', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(search_criteria)
        .execute(&pool)
        .await
        .expect("insert dynamic category");
        sqlx::query("INSERT INTO archives (id, path, title) VALUES (?, ?, ?)")
            .bind("archive-match")
            .bind(matching_path.to_string_lossy().to_string())
            .bind("matching title")
            .execute(&pool)
            .await
            .expect("insert matching archive");
        sqlx::query("INSERT INTO archives (id, path, title) VALUES (?, ?, ?)")
            .bind("archive-other")
            .bind(other_path.to_string_lossy().to_string())
            .bind("other title")
            .execute(&pool)
            .await
            .expect("insert other archive");

        let response = batch_delete_category_archives(
            State(pool.clone()),
            Path("dynamic-1".to_string()),
            Extension(test_cache_service()),
            Extension(test_auth_info()),
        )
        .await
        .expect("delete matching dynamic category archives");

        assert_eq!(response.0.matched, 1);
        assert_eq!(response.0.deleted, 1);
        assert_eq!(response.0.failed, 0);
        assert!(!matching_path.exists());
        assert!(other_path.exists());

        let trash_path: String = sqlx::query_scalar(
            "SELECT trash_path FROM trash_entries WHERE archive_id = 'archive-match' AND user_id = 'user-1' AND status = 'active'",
        )
        .fetch_one(&pool)
        .await
        .expect("load active trash entry");
        assert!(std::path::Path::new(&trash_path).exists());
        let event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_behavior_events WHERE archive_id = 'archive-match' AND event_type = 'manual_delete'",
        )
        .fetch_one(&pool)
        .await
        .expect("count delete events");
        assert_eq!(event_count, 1);
        let disposition_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM archive_dispositions WHERE archive_id = 'archive-match' AND disposition = 'manual_delete'",
        )
        .fetch_one(&pool)
        .await
        .expect("count delete dispositions");
        assert_eq!(disposition_count, 1);

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM archives")
            .fetch_one(&pool)
            .await
            .expect("count remaining archives");
        assert_eq!(remaining, 1);

        tokio::fs::remove_dir_all(test_dir)
            .await
            .expect("remove test directory");
    }

    #[tokio::test]
    async fn membership_updates_are_noop_for_missing_category() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_categories_schema(&pool).await;

        let request = AddArchivesToCategoryRequest {
            archive_ids: vec!["archive-1".to_string()],
        };

        let add_status = add_archives_to_category(
            State(pool.clone()),
            Path("missing-category".to_string()),
            Json(request.clone()),
        )
        .await
        .expect("missing category add should be a no-op");
        assert_eq!(add_status, StatusCode::OK);

        let remove_status = remove_archives_from_category(
            State(pool),
            Path("missing-category".to_string()),
            Json(request),
        )
        .await
        .expect("missing category remove should be a no-op");
        assert_eq!(remove_status, StatusCode::OK);
    }
}

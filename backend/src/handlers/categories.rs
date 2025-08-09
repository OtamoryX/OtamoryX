use crate::models::{
    AddArchivesToCategoryRequest, Archive, Category, CreateCategoryRequest,
    CreateDynamicCategoryRequest, DynamicCategory, PaginatedResponse, SearchRequest,
    UpdateCategoryRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};

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

    let mut categories = Vec::new();
    for row in rows {
        // 计算档案数量
        let archive_count = if row.category_type == "static" {
            // 静态分类：直接计算关联表中的数量
            sqlx::query!(
                "SELECT COUNT(*) as count FROM category_archives WHERE category_id = ?",
                row.id
            )
            .fetch_one(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .count as u32
        } else {
            // 动态分类：根据搜索条件计算（暂时返回0，完整实现需要解析search_criteria）
            0
        };

        categories.push(Category {
            id: row.id.unwrap_or_default(),
            name: row.name,
            description: row.description,
            is_static: row.category_type == "static",
            archive_count: archive_count as i32,
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
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    use crate::services::{ArchiveQueryService, ArchiveFilters, PaginationParams, QueryOptions, SearchService};
    
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
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

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
            return Ok(Json(PaginatedResponse {
                data: vec![],
                page_numb: params.page_numb.unwrap_or(1),
                page_size: params.page_size.unwrap_or(20),
                total: 0,
                has_next: false,
            }));
        }

        let filters = ArchiveFilters {
            archive_ids: Some(category_archive_ids),
            ..ArchiveFilters::from_search_request(&params)
        };
        let pagination = PaginationParams::from_search_request(&params);
        let options = QueryOptions {
            random: false,
            include_tags: true,
            user_id: Some(user_id),
        };

        match query_service.query_archives(filters, pagination, options).await {
            Ok(result) => Ok(Json(result)),
            Err(e) => {
                tracing::error!("Query error for static category: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // 动态分类：根据存储的搜索条件查询漫画
        if let Some(search_criteria) = &category.search_criteria {
            match serde_json::from_str::<SearchRequest>(search_criteria) {
                Ok(mut dynamic_params) => {
                    // 合并传入的分页参数
                    dynamic_params.page_numb = params.page_numb;
                    dynamic_params.page_size = params.page_size;
                    
                    let search_service = SearchService::new(pool);
                    match search_service.search_archives(dynamic_params, &user_id).await {
                        Ok(result) => Ok(Json(result)),
                        Err(e) => {
                            tracing::error!("Search error for dynamic category: {}", e);
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse search criteria for category {}: {}", category_id, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        } else {
            // 没有搜索条件的动态分类，返回空结果
            Ok(Json(PaginatedResponse {
                data: vec![],
                page_numb: params.page_numb.unwrap_or(1),
                page_size: params.page_size.unwrap_or(20),
                total: 0,
                has_next: false,
            }))
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
    })?
    .ok_or_else(|| {
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

    tracing::info!("Updated category: {} ({})", updated_category.name, updated_category.id);
    Ok(Json(updated_category))
}

// 删除分类
pub async fn delete_category(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // 检查分类是否存在
    let category = sqlx::query!(
        "SELECT id, name FROM categories WHERE id = ?",
        category_id
    )
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
    sqlx::query!(
        "DELETE FROM categories WHERE id = ?",
        category_id
    )
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
    })?
    .ok_or_else(|| {
        tracing::warn!("Category not found: {}", category_id);
        StatusCode::NOT_FOUND
    })?;

    if category.category_type != "static" {
        tracing::warn!("Cannot add archives to non-static category: {}", category_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    // 批量插入档案-分类关联
    let mut tx = pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
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
                tracing::error!("Failed to add archive {} to category {}: {}", archive_id, category_id, e);
            }
        }
    }

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    })?
    .ok_or_else(|| {
        tracing::warn!("Category not found: {}", category_id);
        StatusCode::NOT_FOUND
    })?;

    if category.category_type != "static" {
        tracing::warn!("Cannot remove archives from non-static category: {}", category_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    // 批量删除档案-分类关联
    let mut tx = pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
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
                tracing::error!("Failed to remove archive {} from category {}: {}", archive_id, category_id, e);
            }
        }
    }

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

/// DELETE /api/v1/categories/:id/archives/batch-delete - 批量删除分类下的漫画
pub async fn batch_delete_category_archives(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // 验证分类存在
    let category = sqlx::query!(
        "SELECT category_type FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let archive_ids: Vec<String> = if category.category_type == "static" {
        // 静态分类：从关联表获取档案ID
        sqlx::query!(
            "SELECT archive_id FROM category_archives WHERE category_id = ?",
            category_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|row| row.archive_id)
        .collect()
    } else {
        // 动态分类：根据搜索条件获取档案ID（暂时返回空，需要完整的搜索实现）
        vec![]
    };

    if archive_ids.is_empty() {
        return Ok(StatusCode::OK);
    }
    
    // 使用事务批量删除
    let mut tx = pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut deleted_count = 0;
    
    for archive_id in &archive_ids {
        match sqlx::query!("DELETE FROM archives WHERE id = ?", archive_id)
            .execute(&mut *tx)
            .await
        {
            Ok(result) => deleted_count += result.rows_affected(),
            Err(e) => {
                tracing::error!("Failed to delete archive {}: {}", archive_id, e);
            }
        }
    }
    
    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(
        "Batch deleted {} archives from category {}",
        deleted_count,
        category_id
    );
    Ok(StatusCode::OK)
}

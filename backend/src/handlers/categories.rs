use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};
use crate::models::{
    Category, DynamicCategory, CreateCategoryRequest, CreateDynamicCategoryRequest,
    UpdateCategoryRequest, AddArchivesToCategoryRequest, Archive, PaginatedResponse,
    SearchRequest,
};

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
            sqlx::query!("SELECT COUNT(*) as count FROM category_archives WHERE category_id = ?", row.id)
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
    let search_params_json = serde_json::to_string(&request.search_params)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

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
    Path(category_id): Path<String>,
    Query(params): Query<SearchRequest>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    // TODO: 
    // 1. 如果是静态分类，从关联表获取漫画
    // 2. 如果是动态分类，根据搜索参数查询漫画
    
    let mock_archives = vec![
        Archive {
            id: format!("archive_in_{}", category_id),
            title: format!("分类 {} 中的漫画", category_id),
            path: "/comics/category_comic.cbz".to_string(),
            file_size: 1024 * 1024,
            page_count: 20,
            hash: "category123".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec![],
        },
    ];

    Ok(Json(PaginatedResponse {
        data: mock_archives,
        page: params.page.unwrap_or(1),
        limit: params.limit.unwrap_or(20),
        total: 1,
        has_next: false,
    }))
}

// 更新分类信息
pub async fn update_category(
    Path(category_id): Path<String>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<StatusCode, StatusCode> {
    // TODO: 更新数据库中的分类信息
    tracing::info!("Updating category {}: {:?}", category_id, request);
    Ok(StatusCode::OK)
}

// 删除分类
pub async fn delete_category(
    Path(category_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // TODO: 从数据库删除分类
    tracing::info!("Deleting category: {}", category_id);
    Ok(StatusCode::OK)
}

// 向静态分类中添加漫画
pub async fn add_archives_to_category(
    Path(category_id): Path<String>,
    Json(request): Json<AddArchivesToCategoryRequest>,
) -> Result<StatusCode, StatusCode> {
    // TODO: 在关联表中添加漫画到分类的关系
    tracing::info!("Adding {} archives to category {}", request.archive_ids.len(), category_id);
    Ok(StatusCode::OK)
}

// 从静态分类中移除漫画
pub async fn remove_archives_from_category(
    Path(category_id): Path<String>,
    Json(request): Json<AddArchivesToCategoryRequest>,
) -> Result<StatusCode, StatusCode> {
    // TODO: 从关联表中删除漫画到分类的关系
    tracing::info!("Removing {} archives from category {}", request.archive_ids.len(), category_id);
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

    tracing::info!("Pruned {} empty static categories", deleted_static.rows_affected());
    Ok(StatusCode::OK)
}

/// DELETE /api/v1/categories/:id/archives/batch-delete - 批量删除分类下的漫画
pub async fn batch_delete_category_archives(
    State(pool): State<Pool<Sqlite>>,
    Path(category_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // 验证分类存在
    let category = sqlx::query!("SELECT category_type FROM categories WHERE id = ?", category_id)
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

    // 删除档案记录（级联删除会处理关联表）
    let placeholders = archive_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("DELETE FROM archives WHERE id IN ({})", placeholders);
    
    let mut sqlx_query = sqlx::query(&query);
    for archive_id in archive_ids {
        sqlx_query = sqlx_query.bind(archive_id);
    }

    let result = sqlx_query
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Batch deleted {} archives from category {}", result.rows_affected(), category_id);
    Ok(StatusCode::OK)
}
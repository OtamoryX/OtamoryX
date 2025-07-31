use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use crate::models::{
    Category, DynamicCategory, CreateCategoryRequest, CreateDynamicCategoryRequest,
    UpdateCategoryRequest, AddArchivesToCategoryRequest, Archive, PaginatedResponse,
    SearchRequest,
};

// 获取所有分类（静态+动态）
pub async fn get_categories() -> Result<Json<Vec<Category>>, StatusCode> {
    // TODO: 从数据库获取所有分类
    let mock_categories = vec![
        Category {
            id: "cat_1".to_string(),
            name: "收藏夹".to_string(),
            description: Some("我的收藏".to_string()),
            is_static: true,
            archive_count: 5,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        Category {
            id: "cat_2".to_string(),
            name: "最近阅读".to_string(),
            description: Some("最近30天阅读的漫画".to_string()),
            is_static: false,
            archive_count: 12,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];

    Ok(Json(mock_categories))
}

// 创建静态分类
pub async fn create_category(
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Json<Category>, StatusCode> {
    // TODO: 保存到数据库
    let category = Category {
        id: format!("cat_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        name: request.name,
        description: request.description,
        is_static: true,
        archive_count: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    Ok(Json(category))
}

// 创建动态分类
pub async fn create_dynamic_category(
    Json(request): Json<CreateDynamicCategoryRequest>,
) -> Result<Json<DynamicCategory>, StatusCode> {
    // TODO: 保存到数据库
    let search_params_json = serde_json::to_string(&request.search_params)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let dynamic_category = DynamicCategory {
        id: format!("dcat_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        name: request.name,
        description: request.description,
        search_params: search_params_json,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
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
pub async fn prune_empty_categories() -> Result<StatusCode, StatusCode> {
    // TODO: 删除没有漫画的分类
    tracing::info!("Pruning empty categories");
    Ok(StatusCode::OK)
}

/// DELETE /api/v1/categories/:id/archives/batch-delete - 批量删除分类下的漫画
pub async fn batch_delete_category_archives(
    Path(category_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // TODO: 批量删除分类下的所有漫画
    tracing::info!("Batch deleting archives from category {}", category_id);
    Ok(StatusCode::OK)
}
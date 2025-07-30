use axum::{
    extract::Query,
    http::StatusCode,
    Json,
};
use crate::models::{Archive, PaginatedResponse, SearchRequest, Tag};

pub async fn search_archives(
    Query(params): Query<SearchRequest>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    // TODO: 实现真实的搜索逻辑，支持以下搜索条件：
    // - query: 标题关键词搜索
    // - tags: 标签搜索
    // - min_pages/max_pages: 页数范围
    // - min_file_size/max_file_size: 文件大小范围
    // - sort_by/sort_order: 排序
    
    let mut search_desc = vec![];
    
    if let Some(query) = &params.query {
        search_desc.push(format!("标题包含: {}", query));
    }
    
    if let Some(tags) = &params.tags {
        search_desc.push(format!("标签: {:?}", tags));
    }
    
    if let Some(min_pages) = params.min_pages {
        search_desc.push(format!("最少{}页", min_pages));
    }
    
    if let Some(max_pages) = params.max_pages {
        search_desc.push(format!("最多{}页", max_pages));
    }
    
    let search_description = if search_desc.is_empty() {
        "全部漫画".to_string()
    } else {
        search_desc.join(", ")
    };

    let mock_archives = vec![
        Archive {
            id: "search-1".to_string(),
            title: format!("搜索结果: {}", search_description),
            path: "/comics/search1.cbz".to_string(),
            file_size: 2048 * 1024,
            page_count: 25,
            hash: "search123".to_string(),
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

pub async fn get_tags() -> Result<Json<Vec<Tag>>, StatusCode> {
    // TODO: 从数据库获取标签
    let mock_tags = vec![
        Tag {
            id: 1,
            name: "漫画".to_string(),
            namespace: "category".to_string(),
        },
        Tag {
            id: 2,
            name: "热门".to_string(),
            namespace: "popularity".to_string(),
        },
        Tag {
            id: 3,
            name: "新作".to_string(),
            namespace: "status".to_string(),
        },
    ];

    Ok(Json(mock_tags))
}
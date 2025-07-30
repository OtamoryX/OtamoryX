use axum::{
    extract::{Path, Query},
    http::{StatusCode, HeaderMap, header},
    Json,
    response::Response,
    body::Body,
};
use serde::Deserialize;
use crate::models::{Archive, PaginatedResponse, SearchRequest};

#[derive(Deserialize)]
pub struct ArchiveQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub async fn get_archives(
    Query(params): Query<ArchiveQuery>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    // TODO: 实现从数据库获取漫画列表
    let archives = vec![
        Archive {
            id: "1".to_string(),
            title: "海贼王 第1卷".to_string(),
            path: "/comics/onepiece_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 15, // 15MB
            page_count: 200,
            hash: "abc123".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(10),
            updated_at: chrono::Utc::now() - chrono::Duration::days(10),
            tags: vec![],
        },
        Archive {
            id: "2".to_string(),
            title: "火影忍者 第1卷".to_string(),
            path: "/comics/naruto_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 12, // 12MB
            page_count: 180,
            hash: "def456".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(8),
            updated_at: chrono::Utc::now() - chrono::Duration::days(8),
            tags: vec![],
        },
        Archive {
            id: "3".to_string(),
            title: "死神 第1卷".to_string(),
            path: "/comics/bleach_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 18, // 18MB
            page_count: 220,
            hash: "ghi789".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(5),
            updated_at: chrono::Utc::now() - chrono::Duration::days(5),
            tags: vec![],
        },
        Archive {
            id: "4".to_string(),
            title: "龙珠 第1卷".to_string(),
            path: "/comics/dragonball_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 10, // 10MB
            page_count: 160,
            hash: "jkl012".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(3),
            updated_at: chrono::Utc::now() - chrono::Duration::days(3),
            tags: vec![],
        },
        Archive {
            id: "5".to_string(),
            title: "进击的巨人 第1卷".to_string(),
            path: "/comics/aot_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 20, // 20MB
            page_count: 250,
            hash: "mno345".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(1),
            updated_at: chrono::Utc::now() - chrono::Duration::days(1),
            tags: vec![],
        },
        Archive {
            id: "6".to_string(),
            title: "鬼灭之刃 第1卷".to_string(),
            path: "/comics/demonslayer_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 14, // 14MB
            page_count: 190,
            hash: "pqr678".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec![],
        },
    ];

    Ok(Json(PaginatedResponse {
        data: archives,
        page: params.page.unwrap_or(1),
        limit: params.limit.unwrap_or(20),
        total: 6,
        has_next: false,
    }))
}

pub async fn get_archive(
    Path(id): Path<String>,
) -> Result<Json<Archive>, StatusCode> {
    // TODO: 从数据库获取漫画详情
    // 现在使用模拟数据，根据ID返回对应的漫画信息
    let archives = vec![
        Archive {
            id: "1".to_string(),
            title: "海贼王 第1卷".to_string(),
            path: "/comics/onepiece_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 15, // 15MB
            page_count: 200,
            hash: "abc123".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(10),
            updated_at: chrono::Utc::now() - chrono::Duration::days(10),
            tags: vec![
                crate::models::Tag {
                    id: 1,
                    name: "尾田荣一郎".to_string(),
                    namespace: "作者".to_string(),
                },
                crate::models::Tag {
                    id: 2,
                    name: "少年漫画".to_string(),
                    namespace: "分类".to_string(),
                },
            ],
        },
        Archive {
            id: "2".to_string(),
            title: "火影忍者 第1卷".to_string(),
            path: "/comics/naruto_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 12, // 12MB
            page_count: 180,
            hash: "def456".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(8),
            updated_at: chrono::Utc::now() - chrono::Duration::days(8),
            tags: vec![
                crate::models::Tag {
                    id: 3,
                    name: "岸本齐史".to_string(),
                    namespace: "作者".to_string(),
                },
                crate::models::Tag {
                    id: 4,
                    name: "忍者".to_string(),
                    namespace: "题材".to_string(),
                },
            ],
        },
        Archive {
            id: "3".to_string(),
            title: "死神 第1卷".to_string(),
            path: "/comics/bleach_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 18, // 18MB
            page_count: 220,
            hash: "ghi789".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(5),
            updated_at: chrono::Utc::now() - chrono::Duration::days(5),
            tags: vec![
                crate::models::Tag {
                    id: 5,
                    name: "久保带人".to_string(),
                    namespace: "作者".to_string(),
                },
            ],
        },
        Archive {
            id: "4".to_string(),
            title: "龙珠 第1卷".to_string(),
            path: "/comics/dragonball_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 10, // 10MB
            page_count: 160,
            hash: "jkl012".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(3),
            updated_at: chrono::Utc::now() - chrono::Duration::days(3),
            tags: vec![
                crate::models::Tag {
                    id: 6,
                    name: "鸟山明".to_string(),
                    namespace: "作者".to_string(),
                },
                crate::models::Tag {
                    id: 7,
                    name: "格斗".to_string(),
                    namespace: "题材".to_string(),
                },
            ],
        },
        Archive {
            id: "5".to_string(),
            title: "进击的巨人 第1卷".to_string(),
            path: "/comics/aot_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 20, // 20MB
            page_count: 250,
            hash: "mno345".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(1),
            updated_at: chrono::Utc::now() - chrono::Duration::days(1),
            tags: vec![
                crate::models::Tag {
                    id: 8,
                    name: "谏山创".to_string(),
                    namespace: "作者".to_string(),
                },
                crate::models::Tag {
                    id: 9,
                    name: "黑暗".to_string(),
                    namespace: "风格".to_string(),
                },
            ],
        },
        Archive {
            id: "6".to_string(),
            title: "鬼灭之刃 第1卷".to_string(),
            path: "/comics/demonslayer_vol1.cbz".to_string(),
            file_size: 1024 * 1024 * 14, // 14MB
            page_count: 190,
            hash: "pqr678".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec![
                crate::models::Tag {
                    id: 10,
                    name: "吾峠呼世晴".to_string(),
                    namespace: "作者".to_string(),
                },
                crate::models::Tag {
                    id: 11,
                    name: "和风".to_string(),
                    namespace: "风格".to_string(),
                },
            ],
        },
    ];

    // 根据ID查找对应的漫画
    let archive = archives.into_iter().find(|a| a.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(archive))
}

pub async fn get_archive_page(
    Path((id, page)): Path<(String, u32)>,
) -> Result<Response<Body>, StatusCode> {
    tracing::info!("Requesting page {} of archive {}", page, id);
    
    // 创建一个简单的占位符图片（200x300的灰色矩形，模拟漫画封面）
    use image::{ImageBuffer, Rgba, ImageFormat};
    use std::io::Cursor;
    
    let width = 200u32;
    let height = 300u32;
    
    // 创建图片缓冲区
    let mut img = ImageBuffer::new(width, height);
    
    // 根据漫画ID生成不同的背景色
    let color_seed = id.chars().map(|c| c as u32).sum::<u32>();
    let r = ((color_seed * 37) % 200 + 55) as u8;
    let g = ((color_seed * 73) % 200 + 55) as u8;
    let b = ((color_seed * 131) % 200 + 55) as u8;
    
    // 填充背景色
    for pixel in img.pixels_mut() {
        *pixel = Rgba([r, g, b, 255]);
    }
    
    // 将图片编码为PNG
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    img.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(buffer))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}

// 获取漫画封面缩略图
pub async fn get_archive_thumbnail(
    Path(id): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    tracing::info!("Requesting thumbnail for archive {}", id);
    
    // 创建缩略图（150x200，适合卡片显示）
    use image::{ImageBuffer, Rgba, ImageFormat};
    use std::io::Cursor;
    
    let width = 150u32;
    let height = 200u32;
    
    // 创建图片缓冲区
    let mut img = ImageBuffer::new(width, height);
    
    // 根据漫画ID生成不同的背景色
    let color_seed = id.chars().map(|c| c as u32).sum::<u32>();
    let r = ((color_seed * 37) % 200 + 55) as u8;
    let g = ((color_seed * 73) % 200 + 55) as u8;
    let b = ((color_seed * 131) % 200 + 55) as u8;
    
    // 填充背景色
    for pixel in img.pixels_mut() {
        *pixel = Rgba([r, g, b, 255]);
    }
    
    // 将图片编码为JPEG（更小的文件尺寸）
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    img.write_to(&mut cursor, ImageFormat::Jpeg)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=86400") // 24小时缓存
        .body(Body::from(buffer))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}
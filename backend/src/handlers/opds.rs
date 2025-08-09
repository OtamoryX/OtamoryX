use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::{DateTime, Utc, NaiveDateTime, TimeZone};
use serde::Deserialize;
use sqlx::{Pool, Sqlite, Row};

#[derive(Debug, Clone)]
struct ArchiveRow {
    pub id: String,
    pub title: String,
    pub file_size: i64,
    pub page_count: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct OpdsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

/// OPDS Root catalog - navigation entry point
pub async fn opds_root(
    State(_pool): State<Pool<Sqlite>>,
) -> Result<impl IntoResponse, StatusCode> {
    let feed_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opds="http://opds-spec.org/2010/catalog">
  <id>urn:uuid:opds-root</id>
  <title>OtamoryX OPDS Catalog</title>
  <updated>{}</updated>
  <author>
    <name>OtamoryX</name>
  </author>
  
  <link rel="self" href="/opds" type="application/atom+xml"/>
  <link rel="start" href="/opds" type="application/atom+xml"/>
  
  <entry>
    <title>All Archives</title>
    <id>urn:uuid:all-archives</id>
    <updated>{}</updated>
    <summary>Browse all comic archives</summary>
    <link rel="subsection" href="/opds/archives" type="application/atom+xml"/>
  </entry>
  
  <entry>
    <title>Recent Archives</title>
    <id>urn:uuid:recent-archives</id>
    <updated>{}</updated>
    <summary>Recently added comic archives</summary>
    <link rel="subsection" href="/opds/archives?sort=recent" type="application/atom+xml"/>
  </entry>
  
  <entry>
    <title>Search Archives</title>
    <id>urn:uuid:search-archives</id>
    <updated>{}</updated>
    <summary>Search comic archives</summary>
    <link rel="search" href="/opds/search?q={{searchTerms}}" type="application/atom+xml"/>
  </entry>
</feed>"#,
        Utc::now().to_rfc3339(),
        Utc::now().to_rfc3339(),
        Utc::now().to_rfc3339(),
        Utc::now().to_rfc3339()
    );
    
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/atom+xml;charset=utf-8".parse().unwrap());
    
    Ok((headers, feed_xml))
}

/// OPDS Archives feed - list all archives with pagination
pub async fn opds_archives(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<OpdsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20).min(100); // Cap at 100 entries per page
    let offset = (page - 1) * limit;
    
    // Get total count for pagination
    let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM archives")
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Fetch archives with pagination using raw SQL to handle type conversions
    let archive_rows = sqlx::query(
        "SELECT id, title, file_size, page_count, updated_at
         FROM archives 
         ORDER BY created_at DESC 
         LIMIT ? OFFSET ?"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Convert raw rows to ArchiveRow structs
    let archives: Vec<ArchiveRow> = archive_rows
        .into_iter()
        .map(|row| {
            let updated_at_str: String = row.get("updated_at");
            
            // Parse the datetime strings (SQLite returns them as strings)
            let updated_at = NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| Utc::now().naive_utc());
            
            ArchiveRow {
                id: row.get("id"),
                title: row.get("title"),
                file_size: row.get("file_size"),
                page_count: row.get("page_count"),
                updated_at: Utc.from_utc_datetime(&updated_at),
            }
        })
        .collect();
    
    // Build pagination info
    let total_pages = ((total_count as f64) / (limit as f64)).ceil() as u32;
    
    // Create navigation links
    let mut nav_links = String::new();
    
    nav_links.push_str(&format!(
        r#"  <link rel="self" href="/opds/archives?page={}&amp;limit={}" type="application/atom+xml"/>
  <link rel="start" href="/opds" type="application/atom+xml"/>"#,
        page, limit
    ));
    
    if page > 1 {
        nav_links.push_str(&format!(
            r#"
  <link rel="previous" href="/opds/archives?page={}&amp;limit={}" type="application/atom+xml"/>"#,
            page - 1, limit
        ));
    }
    
    if page < total_pages {
        nav_links.push_str(&format!(
            r#"
  <link rel="next" href="/opds/archives?page={}&amp;limit={}" type="application/atom+xml"/>"#,
            page + 1, limit
        ));
    }
    
    // Build entries XML
    let mut entries_xml = String::new();
    for archive in archives {
        let entry_xml = format!(
            r#"  <entry>
    <title>{}</title>
    <id>urn:uuid:{}</id>
    <updated>{}</updated>
    <summary>Pages: {}, Size: {} bytes</summary>
    <link rel="http://opds-spec.org/acquisition" href="/api/v1/archives/{}" type="application/zip"/>
    <link rel="http://opds-spec.org/image/thumbnail" href="/api/v1/archives/{}/thumbnail" type="image/jpeg"/>
    <category term="{}-pages" label="{} pages"/>
  </entry>"#,
            escape_xml(&archive.title),
            archive.id,
            archive.updated_at.to_rfc3339(),
            archive.page_count,
            archive.file_size,
            archive.id,
            archive.id,
            archive.page_count,
            archive.page_count
        );
        entries_xml.push_str(&entry_xml);
        entries_xml.push('\n');
    }
    
    let feed_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opds="http://opds-spec.org/2010/catalog">
  <id>urn:uuid:archives-feed</id>
  <title>OtamoryX Archives</title>
  <updated>{}</updated>
  <author>
    <name>OtamoryX</name>
  </author>
  
{}

{}
</feed>"#,
        Utc::now().to_rfc3339(),
        nav_links,
        entries_xml
    );
    
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/atom+xml;charset=utf-8".parse().unwrap());
    
    Ok((headers, feed_xml))
}

/// OPDS Search - search archives by title
pub async fn opds_search(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let search_term = query.q.unwrap_or_default();
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;
    
    if search_term.is_empty() {
        // Return empty feed for empty search
        let feed_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opds="http://opds-spec.org/2010/catalog">
  <id>urn:uuid:search-results</id>
  <title>Search Results</title>
  <updated>{}</updated>
  <author>
    <name>OtamoryX</name>
  </author>
  
  <link rel="self" href="/opds/search" type="application/atom+xml"/>
  <link rel="start" href="/opds" type="application/atom+xml"/>
</feed>"#,
            Utc::now().to_rfc3339()
        );
        
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/atom+xml;charset=utf-8".parse().unwrap());
        return Ok((headers, feed_xml));
    }
    
    let search_pattern = format!("%{}%", search_term);
    
    // Get matching archives using raw SQL
    let archive_rows = sqlx::query(
        "SELECT id, title, file_size, page_count, updated_at
         FROM archives 
         WHERE title LIKE ? 
         ORDER BY created_at DESC 
         LIMIT ? OFFSET ?"
    )
    .bind(search_pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Convert raw rows to ArchiveRow structs
    let archives: Vec<ArchiveRow> = archive_rows
        .into_iter()
        .map(|row| {
            let updated_at_str: String = row.get("updated_at");
            
            // Parse the datetime strings (SQLite returns them as strings)
            let updated_at = NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| Utc::now().naive_utc());
            
            ArchiveRow {
                id: row.get("id"),
                title: row.get("title"),
                file_size: row.get("file_size"),
                page_count: row.get("page_count"),
                updated_at: Utc.from_utc_datetime(&updated_at),
            }
        })
        .collect();
    
    // Build entries XML
    let mut entries_xml = String::new();
    for archive in archives {
        let entry_xml = format!(
            r#"  <entry>
    <title>{}</title>
    <id>urn:uuid:{}</id>
    <updated>{}</updated>
    <summary>Pages: {}, Size: {} bytes</summary>
    <link rel="http://opds-spec.org/acquisition" href="/api/v1/archives/{}" type="application/zip"/>
    <link rel="http://opds-spec.org/image/thumbnail" href="/api/v1/archives/{}/thumbnail" type="image/jpeg"/>
  </entry>"#,
            escape_xml(&archive.title),
            archive.id,
            archive.updated_at.to_rfc3339(),
            archive.page_count,
            archive.file_size,
            archive.id,
            archive.id
        );
        entries_xml.push_str(&entry_xml);
        entries_xml.push('\n');
    }
    
    let feed_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opds="http://opds-spec.org/2010/catalog">
  <id>urn:uuid:search-results</id>
  <title>Search Results for '{}'</title>
  <updated>{}</updated>
  <author>
    <name>OtamoryX</name>
  </author>
  
  <link rel="self" href="/opds/search?q={}&amp;page={}&amp;limit={}" type="application/atom+xml"/>
  <link rel="start" href="/opds" type="application/atom+xml"/>

{}
</feed>"#,
        escape_xml(&search_term),
        Utc::now().to_rfc3339(),
        urlencoding::encode(&search_term),
        page,
        limit,
        entries_xml
    );
    
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/atom+xml;charset=utf-8".parse().unwrap());
    
    Ok((headers, feed_xml))
}

/// Simple XML escaping for basic characters
fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}
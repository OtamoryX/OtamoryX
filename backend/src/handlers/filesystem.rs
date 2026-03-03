use crate::middleware::auth::AuthInfo;
use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, error};

#[derive(Debug, Deserialize)]
pub struct DirectoryListQuery {
    path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DirectoryInfo {
    name: String,
    path: String,
    is_accessible: bool,
}

#[derive(Debug, Serialize)]
pub struct DirectoryListResponse {
    current_path: String,
    parent_path: Option<String>,
    directories: Vec<DirectoryInfo>,
}

/// List directories for path selection in frontend settings
/// GET /api/v1/filesystem/directories?path={optional_path}
pub async fn list_directories(
    Extension(_auth): Extension<AuthInfo>, // 管理员认证通过middleware
    Query(query): Query<DirectoryListQuery>,
) -> Result<Json<DirectoryListResponse>, StatusCode> {
    debug!("Directory listing request with path: {:?}", query.path);

    // Determine the base path to list
    let base_path = match query.path {
        Some(ref path_str) if !path_str.is_empty() => {
            let path = PathBuf::from(path_str);
            // Security check: ensure path is absolute and doesn't contain .. traversal
            if !path.is_absolute() {
                error!("Non-absolute path requested: {}", path_str);
                return Err(StatusCode::BAD_REQUEST);
            }

            // Check for directory traversal attempts
            if path_str.contains("..") {
                error!("Directory traversal attempt detected: {}", path_str);
                return Err(StatusCode::BAD_REQUEST);
            }

            path
        }
        _ => {
            // Default to root directory on Unix-like systems, or current working directory
            #[cfg(unix)]
            {
                PathBuf::from("/")
            }
            #[cfg(windows)]
            {
                // On Windows, list available drives
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\"))
            }
        }
    };

    debug!("Listing directories in: {:?}", base_path);

    // Check if the base path exists and is a directory
    if !base_path.exists() {
        error!("Requested path does not exist: {:?}", base_path);
        return Err(StatusCode::NOT_FOUND);
    }

    if !base_path.is_dir() {
        error!("Requested path is not a directory: {:?}", base_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get parent directory path
    let parent_path = base_path.parent().map(|p| p.to_string_lossy().to_string());

    // Read directory contents
    let entries = match fs::read_dir(&base_path) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read directory {:?}: {}", base_path, e);
            return Err(StatusCode::FORBIDDEN);
        }
    };

    let mut directories = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                debug!("Error reading directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Only include directories, skip files
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories (starting with .)
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str.starts_with('.') {
                    continue;
                }
            }
        }

        let name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        let path_str = path.to_string_lossy().to_string();

        // Check if directory is accessible (can be read)
        let is_accessible = match fs::read_dir(&path) {
            Ok(_) => true,
            Err(_) => false,
        };

        directories.push(DirectoryInfo {
            name,
            path: path_str,
            is_accessible,
        });
    }

    // Sort directories alphabetically
    directories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let response = DirectoryListResponse {
        current_path: base_path.to_string_lossy().to_string(),
        parent_path,
        directories,
    };

    debug!("Returning {} directories", response.directories.len());
    Ok(Json(response))
}

#[cfg(windows)]
/// List available drives on Windows
/// GET /api/v1/filesystem/drives
pub async fn list_drives(
    Extension(_auth): Extension<AuthInfo>, // 管理员认证通过middleware
) -> Result<Json<Vec<DirectoryInfo>>, StatusCode> {
    debug!("Listing available drives on Windows");

    let mut drives = Vec::new();

    // Get available drives (A: through Z:)
    for drive_letter in b'A'..=b'Z' {
        let drive_path = format!("{}:\\", drive_letter as char);
        let path = PathBuf::from(&drive_path);

        if path.exists() {
            let is_accessible = match fs::read_dir(&path) {
                Ok(_) => true,
                Err(_) => false,
            };

            drives.push(DirectoryInfo {
                name: drive_path.clone(),
                path: drive_path,
                is_accessible,
            });
        }
    }

    debug!("Found {} available drives", drives.len());
    Ok(Json(drives))
}

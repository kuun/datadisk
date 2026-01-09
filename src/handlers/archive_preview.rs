//! Archive preview handlers
//!
//! Supports previewing contents of ZIP, TAR, TAR.GZ, TAR.XZ, RAR, and 7Z archives

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::handlers::file::get_user_path;
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;

/// Archive file entry for preview
#[derive(Debug, Serialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub dir: bool,
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct ArchivePreviewQuery {
    pub path: String,
}

/// Detect archive type by MIME type (reading file magic bytes)
fn detect_mime_type(path: &PathBuf) -> Option<&'static str> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return None,
    };

    let mut buffer = [0u8; 16];
    use std::io::Read;
    if file.read(&mut buffer).is_err() {
        return None;
    }

    // Check magic bytes
    // ZIP: PK (0x50 0x4B)
    if buffer.starts_with(&[0x50, 0x4B, 0x03, 0x04])
        || buffer.starts_with(&[0x50, 0x4B, 0x05, 0x06])
        || buffer.starts_with(&[0x50, 0x4B, 0x07, 0x08])
    {
        return Some("application/zip");
    }

    // RAR: Rar! (0x52 0x61 0x72 0x21)
    if buffer.starts_with(&[0x52, 0x61, 0x72, 0x21]) {
        return Some("application/vnd.rar");
    }

    // 7Z: 7z (0x37 0x7A 0xBC 0xAF 0x27 0x1C)
    if buffer.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        return Some("application/x-7z-compressed");
    }

    // GZIP: (0x1F 0x8B)
    if buffer.starts_with(&[0x1F, 0x8B]) {
        return Some("application/gzip");
    }

    // XZ: (0xFD 0x37 0x7A 0x58 0x5A 0x00)
    if buffer.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]) {
        return Some("application/x-xz");
    }

    // TAR: ustar at offset 257 (need to read more bytes)
    let mut tar_buffer = [0u8; 265];
    use std::io::Seek;
    if file.seek(std::io::SeekFrom::Start(0)).is_ok() {
        if let Ok(n) = file.read(&mut tar_buffer) {
            if n >= 265 && &tar_buffer[257..262] == b"ustar" {
                return Some("application/x-tar");
            }
        }
    }

    None
}

/// GET /api/archive/preview - Preview archive file contents
pub async fn archive_preview(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<ArchivePreviewQuery>,
) -> Result<Json<Vec<ArchiveEntry>>, (StatusCode, Json<serde_json::Value>)> {
    let user_path = get_user_path(&state.config, &current_user.username);
    let file_path = user_path.join(query.path.trim_start_matches('/'));

    if !file_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "文件不存在"})),
        ));
    }

    // First try to detect by MIME type (magic bytes)
    let mime_type = detect_mime_type(&file_path);

    let entries = match mime_type {
        Some("application/zip") => preview_zip(&file_path),
        Some("application/x-tar") => preview_tar(&file_path),
        Some("application/gzip") => preview_tar_gz(&file_path),
        Some("application/x-xz") => preview_tar_xz(&file_path),
        Some("application/vnd.rar") => preview_rar(&file_path),
        Some("application/x-7z-compressed") => preview_7z(&file_path),
        _ => {
            // Fall back to extension detection
            let extension = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            // Check for .tar.xz extension
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            if file_name.ends_with(".tar.xz") || file_name.ends_with(".txz") {
                return match preview_tar_xz(&file_path) {
                    Ok(list) => Ok(Json(list)),
                    Err(e) => {
                        tracing::error!("Failed to preview archive: {}", e);
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": format!("无法解析压缩文件: {}", e)})),
                        ))
                    }
                };
            }

            match extension.as_str() {
                "zip" => preview_zip(&file_path),
                "tar" => preview_tar(&file_path),
                "gz" | "tgz" => preview_tar_gz(&file_path),
                "xz" => preview_tar_xz(&file_path),
                "rar" => preview_rar(&file_path),
                "7z" => preview_7z(&file_path),
                _ => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "不支持的压缩格式"})),
                    ));
                }
            }
        }
    };

    match entries {
        Ok(list) => Ok(Json(list)),
        Err(e) => {
            tracing::error!("Failed to preview archive: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("无法解析压缩文件: {}", e)})),
            ))
        }
    }
}

/// Preview ZIP file contents
fn preview_zip(path: &PathBuf) -> Result<Vec<ArchiveEntry>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        let is_dir = file.is_dir();

        // Get file name from path
        let file_name = if is_dir {
            name.trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or(&name)
                .to_string()
        } else {
            name.split('/').last().unwrap_or(&name).to_string()
        };

        let date = file
            .last_modified()
            .map(|dt| {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    dt.year(),
                    dt.month(),
                    dt.day(),
                    dt.hour(),
                    dt.minute(),
                    dt.second()
                )
            })
            .unwrap_or_default();

        entries.push(ArchiveEntry {
            name: file_name,
            path: name.trim_end_matches('/').to_string(),
            size: file.size(),
            dir: is_dir,
            date,
        });
    }

    Ok(entries)
}

/// Preview TAR file contents
fn preview_tar(path: &PathBuf) -> Result<Vec<ArchiveEntry>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(file);

    let mut entries = Vec::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path_str = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        let is_dir = entry.header().entry_type().is_dir();

        let file_name = if is_dir {
            path_str
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or(&path_str)
                .to_string()
        } else {
            path_str.split('/').last().unwrap_or(&path_str).to_string()
        };

        let date = entry
            .header()
            .mtime()
            .ok()
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        entries.push(ArchiveEntry {
            name: file_name,
            path: path_str.trim_end_matches('/').to_string(),
            size: entry.header().size().unwrap_or(0),
            dir: is_dir,
            date,
        });
    }

    Ok(entries)
}

/// Preview TAR.GZ / TGZ file contents
fn preview_tar_gz(path: &PathBuf) -> Result<Vec<ArchiveEntry>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut entries = Vec::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path_str = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        let is_dir = entry.header().entry_type().is_dir();

        let file_name = if is_dir {
            path_str
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or(&path_str)
                .to_string()
        } else {
            path_str.split('/').last().unwrap_or(&path_str).to_string()
        };

        let date = entry
            .header()
            .mtime()
            .ok()
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        entries.push(ArchiveEntry {
            name: file_name,
            path: path_str.trim_end_matches('/').to_string(),
            size: entry.header().size().unwrap_or(0),
            dir: is_dir,
            date,
        });
    }

    Ok(entries)
}

/// Preview TAR.XZ / TXZ file contents
fn preview_tar_xz(path: &PathBuf) -> Result<Vec<ArchiveEntry>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let xz = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(xz);

    let mut entries = Vec::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path_str = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        let is_dir = entry.header().entry_type().is_dir();

        let file_name = if is_dir {
            path_str
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or(&path_str)
                .to_string()
        } else {
            path_str.split('/').last().unwrap_or(&path_str).to_string()
        };

        let date = entry
            .header()
            .mtime()
            .ok()
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        entries.push(ArchiveEntry {
            name: file_name,
            path: path_str.trim_end_matches('/').to_string(),
            size: entry.header().size().unwrap_or(0),
            dir: is_dir,
            date,
        });
    }

    Ok(entries)
}

/// Preview RAR file contents
fn preview_rar(path: &PathBuf) -> Result<Vec<ArchiveEntry>, String> {
    let archive =
        unrar::Archive::new(path).open_for_listing().map_err(|e| format!("{:?}", e))?;

    let mut entries = Vec::new();
    for entry in archive {
        let entry = entry.map_err(|e| format!("{:?}", e))?;
        let path_str = entry.filename.to_string_lossy().to_string();
        let is_dir = entry.is_directory();

        let file_name = if is_dir {
            path_str
                .trim_end_matches(['/', '\\'])
                .split(['/', '\\'])
                .last()
                .unwrap_or(&path_str)
                .to_string()
        } else {
            path_str
                .split(['/', '\\'])
                .last()
                .unwrap_or(&path_str)
                .to_string()
        };

        // RAR uses Windows-style paths, normalize to Unix-style
        let normalized_path = path_str.replace('\\', "/");

        // file_time is a Unix timestamp (u32)
        let date = chrono::DateTime::from_timestamp(entry.file_time as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        entries.push(ArchiveEntry {
            name: file_name,
            path: normalized_path.trim_end_matches('/').to_string(),
            size: entry.unpacked_size as u64,
            dir: is_dir,
            date,
        });
    }

    Ok(entries)
}

/// Preview 7z file contents
fn preview_7z(path: &PathBuf) -> Result<Vec<ArchiveEntry>, String> {
    let mut entries = Vec::new();

    sevenz_rust::decompress_file_with_extract_fn(path, ".", |entry, _, _| {
        let path_str = entry.name().to_string();
        let is_dir = entry.is_directory();

        let file_name = if is_dir {
            path_str
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or(&path_str)
                .to_string()
        } else {
            path_str.split('/').last().unwrap_or(&path_str).to_string()
        };

        // Get modification time - sevenz_rust returns FileTime which is Windows FILETIME
        let date = if entry.has_last_modified_date {
            let ft = entry.last_modified_date();
            // Convert Windows FILETIME (100-nanosecond intervals since 1601-01-01) to Unix timestamp
            // Windows FILETIME epoch is 11644473600 seconds before Unix epoch
            let unix_ts = (ft.to_raw() / 10_000_000) as i64 - 11644473600;
            chrono::DateTime::from_timestamp(unix_ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        entries.push(ArchiveEntry {
            name: file_name,
            path: path_str.trim_end_matches('/').to_string(),
            size: entry.size(),
            dir: is_dir,
            date,
        });

        // Return Ok with true to continue iteration without extracting
        Ok(true)
    })
    .map_err(|e| format!("{:?}", e))?;

    Ok(entries)
}

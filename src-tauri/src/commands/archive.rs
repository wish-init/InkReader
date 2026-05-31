use std::path::PathBuf;

use crate::{errors::AppResult, scanner::archive_reader};

/// Return the cover entry name (e.g., "cover.jpg" or "001.jpg") from a CBZ/ZIP archive.
/// The frontend uses this to construct an `archive://` URL for the cover image.
#[tauri::command]
pub fn get_archive_cover_entry(archive_path: String) -> AppResult<String> {
    let path = PathBuf::from(&archive_path);
    if !path.exists() {
        return Err(crate::errors::AppError::PathMissing(archive_path));
    }
    let scan_result = archive_reader::scan_archive(&path)?;
    scan_result.cover_entry.ok_or_else(|| {
        crate::errors::AppError::ArchiveError(format!("压缩包中未找到封面图片: {}", archive_path))
    })
}

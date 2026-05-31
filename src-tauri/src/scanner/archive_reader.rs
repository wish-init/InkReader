use std::fs::File;
use std::io::Read;
use std::path::Path;

use unrar_ng::Archive as RarArchive;
use zip::ZipArchive;

use crate::errors::{AppError, AppResult};
use crate::metadata::comic_metadata::ComicMetadata;
use crate::scanner::{image::is_supported_image, image::mime_from_path, sort::natural_cmp};

enum ArchiveFormat {
    Zip,
    Rar,
}

/// Result of an archive scan that collects images, cover, and metadata.
pub struct ArchiveScanResult {
    /// Image entry names sorted naturally, excluding cover.jpg.
    pub image_entries: Vec<String>,
    /// Cover entry name (cover.jpg if present, otherwise the first image entry).
    pub cover_entry: Option<String>,
    /// Optional metadata parsed from 元数据.json inside the archive.
    pub metadata: Option<ComicMetadata>,
}

/// Scan a ZIP/CBZ or RAR/CBR archive: collect images, cover entry name, and metadata.
pub fn scan_archive(path: &Path) -> AppResult<ArchiveScanResult> {
    match archive_format(path)? {
        ArchiveFormat::Zip => scan_zip_archive(path),
        ArchiveFormat::Rar => scan_rar_archive(path),
    }
}

/// Read the raw bytes of a specific entry from a zip/cbz/rar/cbr archive.
pub fn read_archive_entry(archive_path: &Path, entry_name: &str) -> AppResult<Vec<u8>> {
    match archive_format(archive_path)? {
        ArchiveFormat::Zip => read_zip_entry(archive_path, entry_name),
        ArchiveFormat::Rar => read_rar_entry(archive_path, entry_name),
    }
}

/// Determine the MIME type from a file entry name.
/// Delegates to the canonical `mime_from_path` in `image.rs` to avoid drift.
pub fn mime_from_entry_name(entry_name: &str) -> &'static str {
    mime_from_path(entry_name)
}

fn archive_format(path: &Path) -> AppResult<ArchiveFormat> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("zip" | "cbz") => Ok(ArchiveFormat::Zip),
        Some("rar" | "cbr") => Ok(ArchiveFormat::Rar),
        _ => Err(AppError::ArchiveError(format!(
            "不支持的压缩包格式: {}",
            path.display()
        ))),
    }
}

fn scan_zip_archive(path: &Path) -> AppResult<ArchiveScanResult> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut image_entries = Vec::new();
    let mut cover_entry: Option<String> = None;
    let mut metadata: Option<ComicMetadata> = None;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();

        if entry.is_dir() {
            continue;
        }

        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Check for metadata
        if file_name == "元数据.json" {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            if let Ok(parsed) = serde_json::from_str::<ComicMetadata>(&content) {
                metadata = Some(parsed);
            }
            continue;
        }

        // Check for cover.jpg (record it but don't add to image list)
        if file_name.eq_ignore_ascii_case("cover.jpg") {
            cover_entry = Some(name);
            continue;
        }

        // Check for supported image
        if is_supported_image(Path::new(file_name)) {
            image_entries.push(name);
        }
    }

    image_entries.sort_by(|a, b| natural_cmp(a, b));

    // If no cover.jpg was found, use the first image as cover
    if cover_entry.is_none() && !image_entries.is_empty() {
        cover_entry = Some(image_entries[0].clone());
    }

    Ok(ArchiveScanResult {
        image_entries,
        cover_entry,
        metadata,
    })
}

fn scan_rar_archive(path: &Path) -> AppResult<ArchiveScanResult> {
    let mut archive = RarArchive::new(path)
        .open_for_processing()
        .map_err(|error| AppError::ArchiveError(error.to_string()))?;

    let mut image_entries = Vec::new();
    let mut cover_entry: Option<String> = None;
    let mut metadata: Option<ComicMetadata> = None;

    while let Some(header) = archive
        .read_header()
        .map_err(|error| AppError::ArchiveError(error.to_string()))?
    {
        let name = normalize_archive_entry_name(&header.entry().filename);
        if header.entry().is_directory() {
            archive = header
                .skip()
                .map_err(|error| AppError::ArchiveError(error.to_string()))?;
            continue;
        }

        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name == "元数据.json" {
            let (data, rest) = header
                .read()
                .map_err(|error| AppError::ArchiveError(error.to_string()))?;
            if let Ok(content) = String::from_utf8(data) {
                if let Ok(parsed) = serde_json::from_str::<ComicMetadata>(&content) {
                    metadata = Some(parsed);
                }
            }
            archive = rest;
            continue;
        }

        if file_name.eq_ignore_ascii_case("cover.jpg") {
            cover_entry = Some(name);
            archive = header
                .skip()
                .map_err(|error| AppError::ArchiveError(error.to_string()))?;
            continue;
        }

        if is_supported_image(Path::new(file_name)) {
            image_entries.push(name);
        }

        archive = header
            .skip()
            .map_err(|error| AppError::ArchiveError(error.to_string()))?;
    }

    image_entries.sort_by(|a, b| natural_cmp(a, b));

    if cover_entry.is_none() && !image_entries.is_empty() {
        cover_entry = Some(image_entries[0].clone());
    }

    Ok(ArchiveScanResult {
        image_entries,
        cover_entry,
        metadata,
    })
}

fn read_zip_entry(archive_path: &Path, entry_name: &str) -> AppResult<Vec<u8>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let target_name = normalize_entry_name(entry_name);

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if normalize_entry_name(entry.name()) == target_name {
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;
            return Ok(buffer);
        }
    }

    Err(AppError::ArchiveError(format!(
        "压缩包中未找到条目: {}",
        entry_name
    )))
}

fn read_rar_entry(archive_path: &Path, entry_name: &str) -> AppResult<Vec<u8>> {
    let mut archive = RarArchive::new(archive_path)
        .open_for_processing()
        .map_err(|error| AppError::ArchiveError(error.to_string()))?;
    let target_name = normalize_entry_name(entry_name);

    while let Some(header) = archive
        .read_header()
        .map_err(|error| AppError::ArchiveError(error.to_string()))?
    {
        let name = normalize_archive_entry_name(&header.entry().filename);
        if !header.entry().is_directory() && normalize_entry_name(&name) == target_name {
            let (data, _rest) = header
                .read()
                .map_err(|error| AppError::ArchiveError(error.to_string()))?;
            return Ok(data);
        }

        archive = header
            .skip()
            .map_err(|error| AppError::ArchiveError(error.to_string()))?;
    }

    Err(AppError::ArchiveError(format!(
        "压缩包中未找到条目: {}",
        entry_name
    )))
}

fn normalize_archive_entry_name(path: &Path) -> String {
    normalize_entry_name(&path.to_string_lossy())
}

fn normalize_entry_name(name: &str) -> String {
    name.replace('\\', "/")
}

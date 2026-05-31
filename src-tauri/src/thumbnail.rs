use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, GenericImageView};

use crate::{errors::AppResult, scanner::archive_reader};

const THUMBNAIL_MAX_WIDTH: u32 = 320;
const THUMBNAIL_MAX_HEIGHT: u32 = 460;
const JPEG_QUALITY: u8 = 78;

pub fn thumbnail_dir_from_database_path(database_path: &Path) -> PathBuf {
    database_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("thumbs")
}

pub fn thumbnail_path(thumbnail_dir: &Path, book_id: &str) -> PathBuf {
    thumbnail_dir.join(format!("{book_id}.jpg"))
}

pub fn ensure_book_thumbnail(
    thumbnail_dir: &Path,
    book_id: &str,
    book_path: &str,
    kind: &str,
    cover_path: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(cover_path) = cover_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    fs::create_dir_all(thumbnail_dir)?;
    let output_path = thumbnail_path(thumbnail_dir, book_id);
    if output_path.exists() {
        return Ok(Some(output_path.to_string_lossy().to_string()));
    }

    let cover_bytes = if kind == "folder" {
        fs::read(cover_path)?
    } else {
        archive_reader::read_archive_entry(Path::new(book_path), cover_path)?
    };

    let image = image::load_from_memory(&cover_bytes)?;
    let (width, height) = image.dimensions();
    let ratio = (THUMBNAIL_MAX_WIDTH as f32 / width as f32)
        .min(THUMBNAIL_MAX_HEIGHT as f32 / height as f32)
        .min(1.0);
    let target_width = ((width as f32 * ratio).round() as u32).max(1);
    let target_height = ((height as f32 * ratio).round() as u32).max(1);
    let thumbnail = image.resize(target_width, target_height, FilterType::Lanczos3);

    let mut bytes = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    let rgb = thumbnail.to_rgb8();
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, JPEG_QUALITY);
    encoder.encode(
        &rgb,
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    fs::write(&output_path, bytes)?;

    Ok(Some(output_path.to_string_lossy().to_string()))
}

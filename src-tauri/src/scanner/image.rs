use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Canonical extension-to-MIME mapping for image formats.
/// This is the single source of truth — `is_supported_image` and `mime_from_extension`
/// both reference this list. Add new formats here first.
const EXTENSION_MIME_MAP: &[(&str, &str)] = &[
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("png", "image/png"),
    ("webp", "image/webp"),
    ("gif", "image/gif"),
    ("bmp", "image/bmp"),
];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| IMAGE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Get the MIME type for an image file extension.
/// Covers both supported formats (in IMAGE_EXTENSIONS) and additional formats
/// that may appear in archives but aren't currently supported as page images.
pub fn mime_from_extension(extension: &str) -> &'static str {
    let lower = extension.to_ascii_lowercase();
    EXTENSION_MIME_MAP
        .iter()
        .find(|(ext, _)| ext == &lower)
        .map(|(_, mime)| *mime)
        .unwrap_or("application/octet-stream")
}

/// Get the MIME type for an image file from its path/entry name.
pub fn mime_from_path(path: &str) -> &'static str {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    mime_from_extension(extension)
}

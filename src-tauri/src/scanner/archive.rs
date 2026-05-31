use std::path::Path;

pub fn is_supported_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "zip" | "cbz" | "rar" | "cbr"
            )
        })
        .unwrap_or(false)
}

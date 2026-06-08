use std::{
    collections::{HashMap, HashSet},
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    metadata::comic_metadata::{value_to_string, ComicMetadata},
    models::{
        book::Book,
        chapter::Chapter,
        page::Page,
        repository::{Repository, RepositoryScanResult},
        repository::{
            RepositoryDuplicateBook, RepositoryScanIssue, RepositoryScanProgress,
            RepositoryScanSummary,
        },
    },
    scanner::{
        archive::is_supported_archive, archive_reader, image::is_supported_image, sort::natural_cmp,
    },
};

pub fn scan_repository(path: PathBuf) -> AppResult<RepositoryScanResult> {
    scan_repository_with_options(
        path,
        None,
        &HashMap::new(),
        None::<fn(RepositoryScanProgress)>,
    )
}

pub fn scan_repository_incremental<F>(
    path: PathBuf,
    existing_repository_id: Option<String>,
    existing_signatures: &HashMap<String, String>,
    on_progress: F,
) -> AppResult<RepositoryScanResult>
where
    F: Fn(RepositoryScanProgress),
{
    scan_repository_with_options(
        path,
        existing_repository_id,
        existing_signatures,
        Some(on_progress),
    )
}

fn scan_repository_with_options<F>(
    path: PathBuf,
    existing_repository_id: Option<String>,
    existing_signatures: &HashMap<String, String>,
    on_progress: Option<F>,
) -> AppResult<RepositoryScanResult>
where
    F: Fn(RepositoryScanProgress),
{
    if !path.exists() {
        return Err(AppError::PathMissing(path.display().to_string()));
    }

    if !path.is_dir() {
        return Err(AppError::NotDirectory(path.display().to_string()));
    }

    let now = timestamp();
    let scan_id = Uuid::new_v4().to_string();
    let repository_id = existing_repository_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let repository_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("漫画仓库")
        .to_string();

    let mut books = Vec::new();
    let mut current_book_paths = Vec::new();
    let mut skipped_entries = Vec::new();
    let mut failed_entries = Vec::new();
    let mut entries = required_readable_entries(&path)?
        .into_iter()
        .filter(|entry| entry.path().is_dir() || is_supported_archive(&entry.path()))
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a = a.file_name().to_string_lossy().to_string();
        let b = b.file_name().to_string_lossy().to_string();
        natural_cmp(&a, &b)
    });

    let total_entries = entries.len();
    emit_scan_progress(
        &on_progress,
        &scan_id,
        &path,
        0,
        total_entries,
        "start",
        format!("开始扫描 {total_entries} 个条目"),
    );

    for (index, entry) in entries.into_iter().enumerate() {
        let entry_path = entry.path();
        let entry_path_string = entry_path.to_string_lossy().to_string();
        emit_scan_progress(
            &on_progress,
            &scan_id,
            &path,
            index + 1,
            total_entries,
            "scan",
            entry_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&entry_path_string)
                .to_string(),
        );

        let signature = match scan_signature(&entry_path) {
            Ok(signature) => signature,
            Err(error) => {
                failed_entries.push(RepositoryScanIssue {
                    path: entry_path_string,
                    reason: error.to_string(),
                });
                continue;
            }
        };

        if existing_signatures
            .get(&entry_path_string)
            .is_some_and(|existing| existing == &signature)
        {
            current_book_paths.push(entry_path_string.clone());
            skipped_entries.push(RepositoryScanIssue {
                path: entry_path_string,
                reason: "文件未变化，已跳过深度扫描".to_string(),
            });
            continue;
        }

        let scanned = if is_supported_archive(&entry_path) {
            scan_archive_book(&repository_id, entry_path, &now)
        } else {
            scan_book(&repository_id, entry_path, &now)
        };

        match scanned {
            Ok(Some(mut book)) => {
                book.scan_signature = Some(signature);
                current_book_paths.push(entry_path_string);
                books.push(book);
            }
            Ok(None) => skipped_entries.push(RepositoryScanIssue {
                path: entry_path_string,
                reason: "没有找到可阅读的图片章节".to_string(),
            }),
            Err(error) => {
                if existing_signatures.contains_key(&entry_path_string) {
                    current_book_paths.push(entry_path_string.clone());
                }
                failed_entries.push(RepositoryScanIssue {
                    path: entry_path_string,
                    reason: error.to_string(),
                });
            }
        }
    }

    let unchanged_books = skipped_entries
        .iter()
        .filter(|entry| entry.reason.contains("未变化"))
        .count();

    if books.is_empty() && unchanged_books == 0 {
        return Err(AppError::EmptyRepository(path.display().to_string()));
    }

    let duplicate_books = find_duplicate_books(&books);

    let repository = Repository {
        id: repository_id,
        name: repository_name,
        path: path.to_string_lossy().to_string(),
        book_count: current_book_paths.len(),
        last_scanned_at: Some(now.clone()),
        created_at: now.clone(),
        updated_at: now,
    };

    emit_scan_progress(
        &on_progress,
        &scan_id,
        &path,
        total_entries,
        total_entries,
        "finish",
        "扫描完成".to_string(),
    );

    let scanned_books = books.len();
    Ok(RepositoryScanResult {
        repository,
        books,
        summary: RepositoryScanSummary {
            total_entries,
            scanned_books,
            unchanged_books,
            skipped_entries,
            failed_entries,
            duplicate_books,
        },
        current_book_paths,
    })
}

/// Scan a ZIP/CBZ/RAR/CBR archive as a book.
/// Reads metadata, organizes images into chapters, and returns a Book struct.
/// Extracts book title, source_id, description, authors, and tags from optional metadata.
/// Shared between scan_book and scan_archive_book to prevent logic drift.
pub(crate) struct BookMetadataFields {
    title: String,
    source_id: Option<String>,
    description: Option<String>,
    authors: Vec<String>,
    tags: Vec<String>,
}

impl BookMetadataFields {
    fn from_metadata_and_path(metadata: Option<&ComicMetadata>, path: &Path) -> Self {
        let title = metadata
            .and_then(|m| m.name.clone())
            .or_else(|| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "未命名漫画".to_string());

        Self {
            title,
            source_id: metadata.and_then(ComicMetadata::source_id),
            description: metadata.and_then(|m| m.description.clone()),
            authors: metadata.map(|m| m.author.clone()).unwrap_or_default(),
            tags: metadata.map(|m| m.tags.clone()).unwrap_or_default(),
        }
    }
}

fn emit_scan_progress<F>(
    on_progress: &Option<F>,
    scan_id: &str,
    repository_path: &Path,
    current: usize,
    total: usize,
    phase: &str,
    message: String,
) where
    F: Fn(RepositoryScanProgress),
{
    if let Some(callback) = on_progress {
        callback(RepositoryScanProgress {
            scan_id: scan_id.to_string(),
            repository_path: repository_path.to_string_lossy().to_string(),
            current,
            total,
            phase: phase.to_string(),
            message,
        });
    }
}

fn scan_signature(path: &Path) -> AppResult<String> {
    let mut parts = Vec::new();
    collect_signature_parts(path, &mut parts)?;
    parts.sort();
    Ok(parts.join("|"))
}

fn collect_signature_parts(path: &Path, parts: &mut Vec<String>) -> AppResult<()> {
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    parts.push(format!(
        "{}:{}:{}",
        path.to_string_lossy(),
        metadata.len(),
        modified
    ));

    if metadata.is_dir() {
        for entry in readable_entries(path) {
            let entry_path = entry.path();
            if entry_path.is_dir()
                || entry_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name == "元数据.json" || name.eq_ignore_ascii_case("cover.jpg")
                    })
                || entry_path.is_file()
                    && (is_supported_image(&entry_path) || is_supported_archive(&entry_path))
            {
                collect_signature_parts(&entry_path, parts)?;
            }
        }
    }

    Ok(())
}

fn find_duplicate_books(books: &[Book]) -> Vec<RepositoryDuplicateBook> {
    let mut seen_source_ids = HashMap::<String, &Book>::new();
    let mut seen_titles = HashMap::<String, &Book>::new();
    let mut duplicates = Vec::new();
    let mut reported = HashSet::new();

    for book in books {
        if let Some(source_id) = book
            .source_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            if let Some(existing) = seen_source_ids.get(source_id) {
                if reported.insert(book.path.clone()) {
                    duplicates.push(RepositoryDuplicateBook {
                        path: book.path.clone(),
                        duplicate_of: existing.path.clone(),
                        title: book.title.clone(),
                    });
                }
            } else {
                seen_source_ids.insert(source_id.to_string(), book);
            }
        }

        let normalized_title = book.title.trim().to_lowercase();
        if normalized_title.is_empty() {
            continue;
        }
        if let Some(existing) = seen_titles.get(&normalized_title) {
            if reported.insert(book.path.clone()) {
                duplicates.push(RepositoryDuplicateBook {
                    path: book.path.clone(),
                    duplicate_of: existing.path.clone(),
                    title: book.title.clone(),
                });
            }
        } else {
            seen_titles.insert(normalized_title, book);
        }
    }

    duplicates
}

fn scan_archive_book(repository_id: &str, path: PathBuf, now: &str) -> AppResult<Option<Book>> {
    let scan_result = archive_reader::scan_archive(&path)?;
    let metadata = scan_result.metadata;
    let all_images = scan_result.image_entries;
    let cover_entry = scan_result.cover_entry;

    if all_images.is_empty() && metadata.is_none() {
        return Ok(None);
    }

    let book_id = Uuid::new_v4().to_string();
    let mut chapters = Vec::new();

    // Group images by directory component to create chapters
    let grouped = group_archive_images_by_directory(&all_images);

    for (dir_name, entry_names) in grouped {
        let chapter_id = Uuid::new_v4().to_string();
        let pages: Vec<Page> = entry_names
            .into_iter()
            .enumerate()
            .map(|(index, entry_name)| Page {
                index,
                name: Path::new(&entry_name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string(),
                path: path.to_string_lossy().to_string(),
                uri: entry_name,
            })
            .collect();

        let page_count = pages.len();
        let chapter_title = if dir_name.is_empty() {
            "正文".to_string()
        } else {
            dir_name.clone()
        };

        chapters.push(Chapter {
            id: chapter_id,
            book_id: book_id.clone(),
            source_chapter_id: None,
            title: chapter_title.clone(),
            // For archive books, use the archive-internal directory path (e.g., "第1话")
            // as chapter.path so each chapter has a unique, path-matchable identifier.
            // For flat archives (dir_name is empty), use the archive file path.
            path: if dir_name.is_empty() {
                path.to_string_lossy().to_string()
            } else {
                dir_name
            },
            order: i64::MAX,
            page_count,
            pages,
        });
    }

    if chapters.is_empty() {
        return Ok(None);
    }

    sort_chapters(&mut chapters);

    let total_pages = chapters.iter().map(|c| c.page_count).sum();

    let fields = BookMetadataFields::from_metadata_and_path(metadata.as_ref(), &path);

    // Cover entry name already determined by the single-pass scan
    let cover_path = cover_entry;

    Ok(Some(Book {
        id: book_id,
        repository_id: repository_id.to_string(),
        source_id: fields.source_id,
        title: fields.title.clone(),
        scanned_title: fields.title,
        title_override: None,
        path: path.to_string_lossy().to_string(),
        kind: archive_book_kind(&path).to_string(),
        metadata_path: None,
        cover_path,
        thumbnail_path: None,
        description: fields.description,
        authors: fields.authors,
        tags: fields.tags,
        chapter_count: chapters.len(),
        total_pages,
        last_chapter_id: chapters.first().map(|c| c.id.clone()),
        last_page: 0,
        last_read_at: None,
        is_read_complete: false,
        is_favorite: false,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        scan_signature: None,
        chapters,
    }))
}

/// Group archive image entry names by their directory component.
/// E.g., "第1话/001.jpg" → ("第1话", "第1话/001.jpg")
/// Flat images like "001.jpg" → ("", "001.jpg")
fn group_archive_images_by_directory(entries: &[String]) -> Vec<(String, Vec<String>)> {
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for entry in entries {
        let path = Path::new(entry);
        let dir = path
            .parent()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        groups.entry(dir).or_default().push(entry.clone());
    }

    groups.into_iter().collect()
}

fn archive_book_kind(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("zip") => "zip",
        Some("rar") => "rar",
        Some("cbr") => "cbr",
        _ => "cbz",
    }
}

fn scan_book(repository_id: &str, path: PathBuf, now: &str) -> AppResult<Option<Book>> {
    let metadata_path = path.join("元数据.json");
    let metadata = read_metadata(&metadata_path).ok();
    let cover_path = find_cover_path(&path);
    let book_id = Uuid::new_v4().to_string();

    let mut chapters = scan_chapters(&book_id, &path, metadata.as_ref())?;
    let mut fallback_cover_path = None;
    if chapters.is_empty() {
        let pages = scan_root_pages(&path, cover_path.as_deref())?;
        fallback_cover_path = pages.first().map(|page| page.path.clone());
        if !pages.is_empty() {
            chapters.push(Chapter {
                id: Uuid::new_v4().to_string(),
                book_id: book_id.clone(),
                source_chapter_id: None,
                title: "正文".to_string(),
                path: path.to_string_lossy().to_string(),
                order: 1,
                page_count: pages.len(),
                pages,
            });
        }
    }
    if chapters.is_empty() && metadata.is_none() && cover_path.is_none() {
        return Ok(None);
    }

    let total_pages = chapters.iter().map(|chapter| chapter.page_count).sum();
    if chapters.is_empty() && total_pages == 0 {
        return Ok(None);
    }

    sort_chapters(&mut chapters);

    let fields = BookMetadataFields::from_metadata_and_path(metadata.as_ref(), &path);

    Ok(Some(Book {
        id: book_id,
        repository_id: repository_id.to_string(),
        source_id: fields.source_id,
        title: fields.title.clone(),
        scanned_title: fields.title,
        title_override: None,
        path: path.to_string_lossy().to_string(),
        kind: "folder".to_string(),
        metadata_path: metadata_path
            .exists()
            .then(|| metadata_path.to_string_lossy().to_string()),
        cover_path: cover_path
            .map(|path| path.to_string_lossy().to_string())
            .or(fallback_cover_path),
        thumbnail_path: None,
        description: fields.description,
        authors: fields.authors,
        tags: fields.tags,
        chapter_count: chapters.len(),
        total_pages,
        last_chapter_id: chapters.first().map(|chapter| chapter.id.clone()),
        last_page: 0,
        last_read_at: None,
        is_read_complete: false,
        is_favorite: false,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        scan_signature: None,
        chapters,
    }))
}

fn find_cover_path(book_path: &Path) -> Option<PathBuf> {
    let root_cover = book_path.join("cover.jpg");
    if root_cover.exists() {
        return Some(root_cover);
    }

    let mut nested_covers = readable_entries(book_path)
        .map(|entry| entry.path().join("cover.jpg"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    nested_covers.sort_by(|a, b| {
        let a = a.to_string_lossy().to_string();
        let b = b.to_string_lossy().to_string();
        natural_cmp(&a, &b)
    });

    nested_covers.into_iter().next()
}

fn read_metadata(path: &Path) -> AppResult<ComicMetadata> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn scan_chapters(
    book_id: &str,
    book_path: &Path,
    metadata: Option<&ComicMetadata>,
) -> AppResult<Vec<Chapter>> {
    let mut chapters = Vec::new();
    let chapter_infos = metadata
        .map(|metadata| {
            metadata
                .chapter_infos
                .iter()
                .filter_map(|info| {
                    info.chapter_title
                        .as_ref()
                        .map(|title| (title.clone(), info))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    for entry in readable_entries(book_path) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let mut archive_paths = readable_entries(&path)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && is_supported_archive(path))
            .collect::<Vec<_>>();
        archive_paths.sort_by(|a, b| {
            let a = a
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let b = b
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            natural_cmp(a, b)
        });

        if !archive_paths.is_empty() {
            for archive_path in archive_paths {
                if let Some(chapter) = scan_archive_chapter(book_id, &archive_path)? {
                    chapters.push(chapter);
                }
            }
            continue;
        }

        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("未命名章节")
            .to_string();
        let pages = scan_pages(&path)?;

        if pages.is_empty() {
            continue;
        }

        let info = chapter_infos.get(&title);
        let order = info.and_then(|info| info.order).unwrap_or(i64::MAX);
        let source_chapter_id = info.and_then(|info| value_to_string(info.chapter_id.as_ref()));

        chapters.push(Chapter {
            id: Uuid::new_v4().to_string(),
            book_id: book_id.to_string(),
            source_chapter_id,
            title,
            path: path.to_string_lossy().to_string(),
            order,
            page_count: pages.len(),
            pages,
        });
    }

    Ok(chapters)
}

fn scan_archive_chapter(book_id: &str, path: &Path) -> AppResult<Option<Chapter>> {
    let scan_result = archive_reader::scan_archive(path)?;
    if scan_result.image_entries.is_empty() {
        return Ok(None);
    }

    let chapter_id = Uuid::new_v4().to_string();
    let pages = scan_result
        .image_entries
        .into_iter()
        .enumerate()
        .map(|(index, entry_name)| Page {
            index,
            name: Path::new(&entry_name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
            path: path.to_string_lossy().to_string(),
            uri: entry_name,
        })
        .collect::<Vec<_>>();

    let title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名章节")
        .to_string();

    Ok(Some(Chapter {
        id: chapter_id,
        book_id: book_id.to_string(),
        source_chapter_id: None,
        title,
        path: path.to_string_lossy().to_string(),
        order: i64::MAX,
        page_count: pages.len(),
        pages,
    }))
}

fn scan_pages(chapter_path: &Path) -> AppResult<Vec<Page>> {
    scan_image_pages(chapter_path, None)
}

fn scan_root_pages(book_path: &Path, cover_path: Option<&Path>) -> AppResult<Vec<Page>> {
    scan_image_pages(book_path, cover_path)
}

fn scan_image_pages(directory_path: &Path, excluded_path: Option<&Path>) -> AppResult<Vec<Page>> {
    let mut image_paths = readable_entries(directory_path)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_supported_image(path))
        .filter(|path| excluded_path.is_none_or(|excluded| path != excluded))
        .collect::<Vec<_>>();

    image_paths.sort_by(|a, b| {
        let a = a
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let b = b
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        natural_cmp(a, b)
    });

    Ok(image_paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| Page {
            index,
            name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
            path: path.to_string_lossy().to_string(),
            uri: path.to_string_lossy().to_string(),
        })
        .collect())
}

fn required_readable_entries(path: &Path) -> AppResult<Vec<fs::DirEntry>> {
    fs::read_dir(path)
        .map_err(|error| {
            if error.kind() == ErrorKind::PermissionDenied {
                AppError::Io(format!(
                    "无法读取目录 {}，请确认应用有访问权限或换一个目录: {error}",
                    path.display()
                ))
            } else {
                AppError::from(error)
            }
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)
}

fn readable_entries(path: &Path) -> impl Iterator<Item = fs::DirEntry> {
    fs::read_dir(path)
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
}

fn sort_chapters(chapters: &mut [Chapter]) {
    chapters.sort_by(|a, b| match (a.order == i64::MAX, b.order == i64::MAX) {
        (false, false) => a.order.cmp(&b.order),
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
        (true, true) => natural_cmp(&a.title, &b.title),
    });

    for (index, chapter) in chapters.iter_mut().enumerate() {
        if chapter.order == i64::MAX {
            chapter.order = index as i64 + 1;
        }
    }
}

fn timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
    };

    use uuid::Uuid;
    use zip::{write::SimpleFileOptions, ZipWriter};

    use crate::errors::AppError;

    use super::scan_repository;

    struct TempRepository {
        path: PathBuf,
    }

    impl TempRepository {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("inkreader-scanner-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> PathBuf {
            self.path.clone()
        }
    }

    impl Drop for TempRepository {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_image(path: impl AsRef<Path>) {
        fs::write(path, b"image").unwrap();
    }

    fn write_zip(path: impl AsRef<Path>, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (name, bytes) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }

        zip.finish().unwrap();
    }

    #[test]
    fn nested_chapter_comic_stays_unchanged() {
        let repository = TempRepository::new();
        let book = repository.path.join("漫画A");
        let chapter = book.join("第1话");
        fs::create_dir_all(&chapter).unwrap();
        write_image(book.join("cover.jpg"));
        write_image(chapter.join("001.jpg"));

        let result = scan_repository(repository.path()).unwrap();

        assert_eq!(result.books.len(), 1);
        assert_eq!(result.books[0].chapters.len(), 1);
        assert_eq!(result.books[0].chapters[0].title, "第1话");
        assert_eq!(result.books[0].chapters[0].pages.len(), 1);
        assert_eq!(result.books[0].chapters[0].pages[0].name, "001.jpg");
        assert!(result.books[0]
            .cover_path
            .as_deref()
            .is_some_and(|path| path.ends_with("cover.jpg")));
    }

    #[test]
    fn single_layer_comic_creates_synthetic_chapter() {
        let repository = TempRepository::new();
        let book = repository.path.join("漫画A");
        fs::create_dir_all(&book).unwrap();
        write_image(book.join("10.jpg"));
        write_image(book.join("1.jpg"));
        write_image(book.join("2.png"));

        let result = scan_repository(repository.path()).unwrap();

        assert_eq!(result.books.len(), 1);
        assert_eq!(result.books[0].title, "漫画A");
        assert_eq!(
            result.books[0]
                .cover_path
                .as_deref()
                .map(Path::new)
                .and_then(Path::file_name)
                .and_then(|name| name.to_str()),
            Some("1.jpg")
        );
        assert_eq!(result.books[0].chapters.len(), 1);
        assert_eq!(result.books[0].chapters[0].title, "正文");
        assert_eq!(
            result.books[0].chapters[0].path,
            book.to_string_lossy().to_string()
        );
        assert_eq!(
            result.books[0].chapters[0]
                .pages
                .iter()
                .map(|page| page.name.as_str())
                .collect::<Vec<_>>(),
            vec!["1.jpg", "2.png", "10.jpg"]
        );
    }

    #[test]
    fn single_layer_cover_is_excluded_from_pages() {
        let repository = TempRepository::new();
        let book = repository.path.join("漫画A");
        fs::create_dir_all(&book).unwrap();
        write_image(book.join("cover.jpg"));
        write_image(book.join("001.jpg"));

        let result = scan_repository(repository.path()).unwrap();

        assert_eq!(result.books.len(), 1);
        assert!(result.books[0]
            .cover_path
            .as_deref()
            .is_some_and(|path| path.ends_with("cover.jpg")));
        assert_eq!(result.books[0].chapters.len(), 1);
        assert_eq!(
            result.books[0].chapters[0]
                .pages
                .iter()
                .map(|page| page.name.as_str())
                .collect::<Vec<_>>(),
            vec!["001.jpg"]
        );
    }

    #[test]
    fn nested_archive_chapters_are_scanned_as_one_folder_book() {
        let repository = TempRepository::new();
        let book = repository.path.join("[组名] 标题");
        let cbz_dir = book.join("cbz");
        fs::create_dir_all(&cbz_dir).unwrap();
        write_image(cbz_dir.join("cover.jpg"));
        write_zip(
            cbz_dir.join("第1话 全篇.cbz"),
            &[("0002.jpg", b"image"), ("0001.jpg", b"image")],
        );
        write_zip(cbz_dir.join("第2话.cbz"), &[("0001.jpg", b"image")]);

        let result = scan_repository(repository.path()).unwrap();

        assert_eq!(result.books.len(), 1);
        let book = &result.books[0];
        assert_eq!(book.title, "[组名] 标题");
        assert_eq!(book.kind, "folder");
        assert!(book.cover_path.as_deref().is_some_and(
            |path| path.ends_with("cbz\\cover.jpg") || path.ends_with("cbz/cover.jpg")
        ));
        assert_eq!(
            book.chapters
                .iter()
                .map(|chapter| chapter.title.as_str())
                .collect::<Vec<_>>(),
            vec!["第1话 全篇", "第2话"]
        );
        assert_eq!(book.chapters[0].pages.len(), 2);
        assert!(book.chapters[0].pages[0].path.ends_with("第1话 全篇.cbz"));
        assert_eq!(book.chapters[0].pages[0].uri, "0001.jpg");
        assert_eq!(book.chapters[0].pages[1].uri, "0002.jpg");
    }

    #[test]
    fn empty_book_directory_is_skipped() {
        let repository = TempRepository::new();
        fs::create_dir_all(repository.path.join("空目录")).unwrap();

        let error = scan_repository(repository.path()).unwrap_err();
        assert!(matches!(error, AppError::EmptyRepository(_)));
    }
}

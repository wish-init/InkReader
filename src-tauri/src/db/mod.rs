use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use rusqlite::{params, params_from_iter, types::Value, Connection, Transaction};
use tauri::Manager;

use crate::{
    errors::{AppError, AppResult},
    models::{
        backup::{DatabaseBackupResult, DatabaseRestoreResult},
        book::BookAggregationItem,
        book::UpdateBookMetadataRequest,
        book::{Book, BookListRequest, BookListResponse, BookSummary, BookThumbnail},
        cache::{CacheMaintenanceFailure, CacheMaintenanceResult, CacheMaintenanceSummary},
        chapter::Chapter,
        favorite::FavoriteCollection,
        history::ReadingHistoryRecord,
        metadata_health::{
            MetadataHealthBookIssue, MetadataHealthDuplicateIssue, MetadataHealthScanIssue,
            MetadataHealthSummary,
        },
        operation_log::{OperationLogRecord, OperationLogRequest},
        page::Page,
        repository::{
            Repository, RepositoryScanHistoryRecord, RepositoryScanIssueCode, RepositoryScanSummary,
        },
        settings::{
            EffectiveReaderSettingsState, LibraryViewSettings, PerBookReaderSettings,
            ReaderSettings, SettingsExport, SettingsRestoreScope, SETTINGS_SCHEMA_VERSION,
        },
    },
    thumbnail,
};

const BOOK_TAGS_BACKFILL_SETTING_KEY: &str = "migration:book_tags_backfilled";
const SEARCH_INDEX_BACKFILL_SETTING_KEY: &str = "migration:search_indexes_backfilled_v1";
const REPOSITORY_SCAN_HISTORY_LIMIT: i64 = 20;
const OPERATION_LOG_RETENTION_LIMIT: i64 = 500;

thread_local! {
    static T2S_CONVERTER: RefCell<Option<OpenCC>> =
        RefCell::new(OpenCC::from_config(BuiltinConfig::T2s).ok());
}

pub struct Database {
    path: PathBuf,
}

struct PreservedBookState {
    last_chapter_id: Option<String>,
    last_page: usize,
    created_at: String,
    chapter_path: Option<String>,
    last_read_at: Option<String>,
    title_override: Option<String>,
}

struct PreservedBookmark {
    chapter_path: Option<String>,
    page_index: usize,
    title: String,
    note: Option<String>,
}

impl Database {
    pub fn new(app: &tauri::AppHandle) -> AppResult<Self> {
        let path = app_local_database_path()?;
        let data_dir = path
            .parent()
            .ok_or_else(|| AppError::Database("无法确定 InkReader 数据目录".to_string()))?;

        fs::create_dir_all(data_dir).map_err(|error| {
            AppError::Database(format!(
                "无法创建应用本地数据目录 {}。请将 InkReader 安装或解压到当前用户可写入的目录: {error}",
                data_dir.display()
            ))
        })?;

        copy_legacy_database_if_needed(app, &path)?;

        let database = Self { path };
        database.migrate()?;
        Ok(database)
    }

    fn connect(&self) -> AppResult<Connection> {
        let connection = Connection::open(&self.path).map_err(|error| {
            AppError::Database(format!(
                "无法打开应用本地数据库 {}。请确认 InkReader 安装或解压目录可写: {error}",
                self.path.display()
            ))
        })?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        Ok(connection)
    }

    pub fn migrate(&self) -> AppResult<()> {
        let connection = self.connect()?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS repositories (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              path TEXT NOT NULL UNIQUE,
              book_count INTEGER NOT NULL DEFAULT 0,
              last_scanned_at TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS books (
              id TEXT PRIMARY KEY,
              repository_id TEXT NOT NULL,
              source_id TEXT,
              title TEXT NOT NULL,
              path TEXT NOT NULL UNIQUE,
              kind TEXT NOT NULL,
              metadata_path TEXT,
              cover_path TEXT,
              thumbnail_path TEXT,
              published_at TEXT,
              description TEXT,
              authors_json TEXT NOT NULL DEFAULT '[]',
              tags_json TEXT NOT NULL DEFAULT '[]',
              chapter_count INTEGER NOT NULL DEFAULT 0,
              total_pages INTEGER NOT NULL DEFAULT 0,
              last_chapter_id TEXT,
              last_page INTEGER NOT NULL DEFAULT 0,
              last_read_at TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS book_tags (
              book_id TEXT NOT NULL,
              tag TEXT NOT NULL,
              normalized_tag TEXT,
              PRIMARY KEY (book_id, tag),
              FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS book_authors (
              book_id TEXT NOT NULL,
              author TEXT NOT NULL,
              normalized_author TEXT,
              PRIMARY KEY (book_id, author),
              FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS reading_history (
              id TEXT PRIMARY KEY,
              book_path TEXT NOT NULL,
              chapter_path TEXT,
              chapter_title TEXT,
              page INTEGER NOT NULL,
              read_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_reading_history_read_at
            ON reading_history(read_at DESC);

            CREATE INDEX IF NOT EXISTS idx_reading_history_book_path
            ON reading_history(book_path);

            CREATE INDEX IF NOT EXISTS idx_reading_history_book_latest
            ON reading_history(book_path, read_at DESC);

            CREATE TABLE IF NOT EXISTS favorite_books (
              book_path TEXT PRIMARY KEY,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_favorite_books_updated_at
            ON favorite_books(updated_at DESC);

            CREATE TABLE IF NOT EXISTS favorite_collections (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              cover_path TEXT,
              description TEXT,
              is_default INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS favorite_collection_books (
              collection_id TEXT NOT NULL,
              book_path TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              PRIMARY KEY (collection_id, book_path),
              FOREIGN KEY (collection_id) REFERENCES favorite_collections(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_favorite_collection_books_book_path
            ON favorite_collection_books(book_path);

            CREATE INDEX IF NOT EXISTS idx_favorite_collection_books_collection_updated_at
            ON favorite_collection_books(collection_id, updated_at DESC);

            INSERT OR IGNORE INTO favorite_collections (id, name, is_default, created_at, updated_at)
            VALUES ('default', '默认收藏', 1, datetime('now'), datetime('now'));

            INSERT OR IGNORE INTO favorite_collection_books (collection_id, book_path, created_at, updated_at)
            SELECT 'default', book_path, created_at, updated_at FROM favorite_books;

            CREATE TABLE IF NOT EXISTS chapters (
              id TEXT PRIMARY KEY,
              book_id TEXT NOT NULL,
              source_chapter_id TEXT,
              title TEXT NOT NULL,
              path TEXT NOT NULL,
              order_index INTEGER NOT NULL,
              page_count INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS pages (
              id TEXT PRIMARY KEY,
              chapter_id TEXT NOT NULL,
              page_index INTEGER NOT NULL,
              name TEXT NOT NULL,
              path TEXT NOT NULL,
              uri TEXT NOT NULL,
              FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS book_reader_settings (
              book_id TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS repository_scan_history (
              id TEXT PRIMARY KEY,
              repository_id TEXT NOT NULL,
              repository_name TEXT NOT NULL,
              repository_path TEXT NOT NULL,
              scanned_at TEXT NOT NULL,
              summary_json TEXT NOT NULL,
              FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_repository_scan_history_repository_scanned_at
            ON repository_scan_history(repository_id, scanned_at DESC);

            CREATE TABLE IF NOT EXISTS bookmarks (
              id TEXT PRIMARY KEY,
              book_id TEXT NOT NULL,
              chapter_id TEXT NOT NULL,
              page_index INTEGER NOT NULL,
              title TEXT NOT NULL,
              note TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE,
              FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_bookmarks_book_id
            ON bookmarks(book_id, created_at DESC);

            CREATE INDEX IF NOT EXISTS idx_bookmarks_chapter_page
            ON bookmarks(book_id, chapter_id, page_index);

            CREATE TABLE IF NOT EXISTS operation_logs (
              id TEXT PRIMARY KEY,
              operation_type TEXT NOT NULL,
              target TEXT NOT NULL,
              summary TEXT NOT NULL,
              reversible INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_operation_logs_created_at
            ON operation_logs(created_at DESC);
            ",
        )?;
        add_column_if_missing(&connection, "books", "last_read_at", "TEXT")?;
        add_column_if_missing(&connection, "books", "scanned_title", "TEXT")?;
        add_column_if_missing(&connection, "books", "title_override", "TEXT")?;
        add_column_if_missing(&connection, "books", "scan_signature", "TEXT")?;
        add_column_if_missing(&connection, "books", "thumbnail_path", "TEXT")?;
        add_column_if_missing(&connection, "books", "published_at", "TEXT")?;
        add_column_if_missing(&connection, "books", "search_text_normalized", "TEXT")?;
        add_column_if_missing(&connection, "book_tags", "normalized_tag", "TEXT")?;
        add_column_if_missing(&connection, "book_authors", "normalized_author", "TEXT")?;
        add_column_if_missing(&connection, "favorite_collections", "cover_path", "TEXT")?;
        add_column_if_missing(&connection, "favorite_collections", "description", "TEXT")?;
        connection.execute(
            "UPDATE books SET scanned_title = title WHERE scanned_title IS NULL OR TRIM(scanned_title) = ''",
            [],
        )?;
        connection.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_books_last_read_at ON books(last_read_at DESC);
            CREATE INDEX IF NOT EXISTS idx_books_updated_at ON books(updated_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_updated_at ON books(repository_id, updated_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_created_at ON books(created_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_created_at ON books(repository_id, created_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_last_read_at_id ON books(last_read_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_last_read_at ON books(repository_id, last_read_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_published_at ON books(published_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_published_at ON books(repository_id, published_at DESC, id);
            CREATE INDEX IF NOT EXISTS idx_books_total_pages ON books(total_pages, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_total_pages ON books(repository_id, total_pages, id);
            CREATE INDEX IF NOT EXISTS idx_books_title ON books(title COLLATE NOCASE, id);
            CREATE INDEX IF NOT EXISTS idx_books_title_path_id ON books(title COLLATE NOCASE, path COLLATE NOCASE, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_title_path_id ON books(repository_id, title COLLATE NOCASE, path COLLATE NOCASE, id);
            CREATE INDEX IF NOT EXISTS idx_chapters_book_order ON chapters(book_id, order_index ASC, title ASC);
            CREATE INDEX IF NOT EXISTS idx_chapters_book_final_order ON chapters(book_id, order_index DESC, title COLLATE NOCASE DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_pages_chapter_page_index ON pages(chapter_id, page_index ASC);
            CREATE INDEX IF NOT EXISTS idx_book_tags_tag_book_id ON book_tags(tag, book_id);
            CREATE INDEX IF NOT EXISTS idx_book_tags_book_id ON book_tags(book_id);
            CREATE INDEX IF NOT EXISTS idx_book_tags_normalized_tag_book_id ON book_tags(normalized_tag, book_id);
            CREATE INDEX IF NOT EXISTS idx_book_authors_author_book_id ON book_authors(author, book_id);
            CREATE INDEX IF NOT EXISTS idx_book_authors_normalized_author_book_id ON book_authors(normalized_author, book_id);
            CREATE INDEX IF NOT EXISTS idx_book_authors_book_id ON book_authors(book_id);
            ",
        )?;
        backfill_book_tags(&connection)?;
        backfill_search_indexes(&connection)?;
        Ok(())
    }

    pub fn existing_repository_id_by_path(&self, path: &str) -> AppResult<Option<String>> {
        let connection = self.connect()?;
        let result = connection
            .query_row(
                "SELECT id FROM repositories WHERE path = ?1",
                params![path],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(result)
    }

    pub fn book_scan_signatures(&self, repository_id: &str) -> AppResult<HashMap<String, String>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT path, scan_signature FROM books WHERE repository_id = ?1 AND scan_signature IS NOT NULL",
        )?;
        let rows = statement.query_map(params![repository_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut signatures = HashMap::new();
        for row in rows {
            let (path, signature) = row?;
            signatures.insert(path, signature);
        }
        Ok(signatures)
    }

    pub fn upsert_scan(&self, repository: &Repository, books: &[Book]) -> AppResult<()> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;

        let existing_repository_id = transaction
            .query_row(
                "SELECT id FROM repositories WHERE path = ?1",
                params![&repository.path],
                |row| row.get::<_, String>(0),
            )
            .ok();

        let mut progress_by_path = HashMap::new();
        let mut bookmarks_by_book_path: HashMap<String, Vec<PreservedBookmark>> = HashMap::new();
        if let Some(existing_id) = existing_repository_id.as_ref() {
            {
                let mut statement = transaction.prepare(
                    "SELECT books.path, books.last_chapter_id, books.last_page, books.created_at, chapters.path, books.last_read_at, books.title_override
                     FROM books
                     LEFT JOIN chapters ON chapters.id = books.last_chapter_id
                     WHERE books.repository_id = ?1",
                )?;
                let rows = statement.query_map(params![existing_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        PreservedBookState {
                            last_chapter_id: row.get::<_, Option<String>>(1)?,
                            last_page: row.get::<_, i64>(2)? as usize,
                            created_at: row.get::<_, String>(3)?,
                            chapter_path: row.get::<_, Option<String>>(4)?,
                            last_read_at: row.get::<_, Option<String>>(5)?,
                            title_override: row.get::<_, Option<String>>(6)?,
                        },
                    ))
                })?;

                for row in rows {
                    let (path, state) = row?;
                    progress_by_path.insert(path, state);
                }
            }

            // Preserve bookmarks by book path
            {
                let mut statement = transaction.prepare(
                    "SELECT books.path, chapters.path, bookmarks.page_index, bookmarks.title, bookmarks.note
                     FROM bookmarks
                     INNER JOIN books ON books.id = bookmarks.book_id
                     LEFT JOIN chapters ON chapters.id = bookmarks.chapter_id
                     WHERE books.repository_id = ?1",
                )?;
                let rows = statement.query_map(params![existing_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        PreservedBookmark {
                            chapter_path: row.get::<_, Option<String>>(1)?,
                            page_index: row.get::<_, i64>(2)? as usize,
                            title: row.get::<_, String>(3)?,
                            note: row.get::<_, Option<String>>(4)?,
                        },
                    ))
                })?;

                for row in rows {
                    let (book_path, bookmark) = row?;
                    bookmarks_by_book_path
                        .entry(book_path)
                        .or_default()
                        .push(bookmark);
                }
            }

            delete_repository_records(&transaction, existing_id)?;
        }

        transaction.execute(
            "INSERT INTO repositories (id, name, path, book_count, last_scanned_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &repository.id,
                &repository.name,
                &repository.path,
                repository.book_count as i64,
                repository.last_scanned_at.as_deref(),
                &repository.created_at,
                &repository.updated_at,
            ],
        )?;

        for book in books {
            insert_book(
                &transaction,
                book,
                progress_by_path.get(&book.path),
                bookmarks_by_book_path.get(&book.path),
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn upsert_incremental_scan(
        &self,
        repository: &Repository,
        changed_books: &[Book],
        current_book_paths: &[String],
    ) -> AppResult<()> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;

        let existing_repository_id = transaction
            .query_row(
                "SELECT id FROM repositories WHERE path = ?1",
                params![&repository.path],
                |row| row.get::<_, String>(0),
            )
            .ok();

        let Some(existing_repository_id) = existing_repository_id else {
            transaction.commit()?;
            return self.upsert_scan(repository, changed_books);
        };

        let changed_paths = changed_books
            .iter()
            .map(|book| book.path.clone())
            .collect::<HashSet<_>>();
        let current_paths = current_book_paths.iter().cloned().collect::<HashSet<_>>();
        let mut progress_by_path = HashMap::new();
        let mut bookmarks_by_book_path: HashMap<String, Vec<PreservedBookmark>> = HashMap::new();

        if !changed_paths.is_empty() {
            let placeholders = sql_placeholders(changed_paths.len());
            let mut values = vec![Value::Text(existing_repository_id.clone())];
            values.extend(changed_paths.iter().cloned().map(Value::Text));

            let mut statement = transaction.prepare(&format!(
                "SELECT books.path, books.last_chapter_id, books.last_page, books.created_at, chapters.path, books.last_read_at, books.title_override
                 FROM books
                 LEFT JOIN chapters ON chapters.id = books.last_chapter_id
                 WHERE books.repository_id = ? AND books.path IN ({placeholders})"
            ))?;
            let rows = statement.query_map(params_from_iter(values), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    PreservedBookState {
                        last_chapter_id: row.get::<_, Option<String>>(1)?,
                        last_page: row.get::<_, i64>(2)? as usize,
                        created_at: row.get::<_, String>(3)?,
                        chapter_path: row.get::<_, Option<String>>(4)?,
                        last_read_at: row.get::<_, Option<String>>(5)?,
                        title_override: row.get::<_, Option<String>>(6)?,
                    },
                ))
            })?;

            for row in rows {
                let (path, state) = row?;
                progress_by_path.insert(path, state);
            }

            let placeholders = sql_placeholders(changed_paths.len());
            let mut values = vec![Value::Text(existing_repository_id.clone())];
            values.extend(changed_paths.iter().cloned().map(Value::Text));
            let mut statement = transaction.prepare(&format!(
                "SELECT books.path, chapters.path, bookmarks.page_index, bookmarks.title, bookmarks.note
                 FROM bookmarks
                 INNER JOIN books ON books.id = bookmarks.book_id
                 LEFT JOIN chapters ON chapters.id = bookmarks.chapter_id
                 WHERE books.repository_id = ? AND books.path IN ({placeholders})"
            ))?;
            let rows = statement.query_map(params_from_iter(values), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    PreservedBookmark {
                        chapter_path: row.get::<_, Option<String>>(1)?,
                        page_index: row.get::<_, i64>(2)? as usize,
                        title: row.get::<_, String>(3)?,
                        note: row.get::<_, Option<String>>(4)?,
                    },
                ))
            })?;

            for row in rows {
                let (book_path, bookmark) = row?;
                bookmarks_by_book_path
                    .entry(book_path)
                    .or_default()
                    .push(bookmark);
            }
        }

        let existing_paths = {
            let mut statement =
                transaction.prepare("SELECT path FROM books WHERE repository_id = ?1")?;
            let rows = statement.query_map(params![&existing_repository_id], |row| {
                row.get::<_, String>(0)
            })?;
            collect_rows(rows)?
        };

        for path in existing_paths {
            if !current_paths.contains(&path) {
                delete_book_external_records_by_path(&transaction, &path)?;
                delete_book_records_by_path(&transaction, &path)?;
            } else if changed_paths.contains(&path) {
                delete_book_records_by_path(&transaction, &path)?;
            }
        }

        transaction.execute(
            "UPDATE repositories
             SET name = ?1, book_count = ?2, last_scanned_at = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                &repository.name,
                repository.book_count as i64,
                repository.last_scanned_at.as_deref(),
                &repository.updated_at,
                &existing_repository_id,
            ],
        )?;

        for book in changed_books {
            insert_book(
                &transaction,
                book,
                progress_by_path.get(&book.path),
                bookmarks_by_book_path.get(&book.path),
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn list_repositories(&self) -> AppResult<Vec<Repository>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, name, path, book_count, last_scanned_at, created_at, updated_at
             FROM repositories ORDER BY updated_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Repository {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                book_count: row.get::<_, i64>(3)? as usize,
                last_scanned_at: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn save_repository_scan_history(
        &self,
        repository: &Repository,
        summary: &RepositoryScanSummary,
    ) -> AppResult<()> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let summary_json = serde_json::to_string(summary)?;
        transaction.execute(
            "INSERT INTO repository_scan_history
               (id, repository_id, repository_name, repository_path, scanned_at, summary_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                repository.id,
                repository.name,
                repository.path,
                repository
                    .last_scanned_at
                    .as_deref()
                    .unwrap_or(repository.updated_at.as_str()),
                summary_json
            ],
        )?;
        transaction.execute(
            "DELETE FROM repository_scan_history
             WHERE repository_id = ?1
               AND id NOT IN (
                 SELECT id FROM repository_scan_history
                 WHERE repository_id = ?1
                 ORDER BY scanned_at DESC, rowid DESC
                 LIMIT ?2
               )",
            params![repository.id, REPOSITORY_SCAN_HISTORY_LIMIT],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_repository_scan_history(&self) -> AppResult<Vec<RepositoryScanHistoryRecord>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, repository_id, repository_name, repository_path, scanned_at, summary_json
             FROM repository_scan_history
             ORDER BY scanned_at DESC, rowid DESC
             LIMIT 100",
        )?;
        let rows = statement.query_map([], map_repository_scan_history_row)?;
        collect_rows(rows)
    }

    pub fn metadata_health_summary(&self) -> AppResult<MetadataHealthSummary> {
        let missing_metadata = self.list_missing_metadata_books()?;
        let missing_covers = self.list_missing_cover_books()?;
        let latest_histories = self.latest_repository_scan_histories()?;
        let mut no_image_issues = Vec::new();
        let mut duplicate_issues = Vec::new();

        for history in latest_histories {
            for issue in history
                .summary
                .skipped_entries
                .iter()
                .chain(history.summary.failed_entries.iter())
                .filter(|issue| issue.code == RepositoryScanIssueCode::NoImages)
            {
                no_image_issues.push(MetadataHealthScanIssue {
                    repository_id: history.repository_id.clone(),
                    repository_name: history.repository_name.clone(),
                    repository_path: history.repository_path.clone(),
                    scanned_at: history.scanned_at.clone(),
                    path: issue.path.clone(),
                    reason: issue.reason.clone(),
                    code: issue.code.clone(),
                    severity: issue.severity.clone(),
                    suggestion: issue.suggestion.clone(),
                });
            }

            for duplicate in &history.summary.duplicate_books {
                duplicate_issues.push(MetadataHealthDuplicateIssue {
                    repository_id: history.repository_id.clone(),
                    repository_name: history.repository_name.clone(),
                    repository_path: history.repository_path.clone(),
                    scanned_at: history.scanned_at.clone(),
                    path: duplicate.path.clone(),
                    duplicate_of: duplicate.duplicate_of.clone(),
                    title: duplicate.title.clone(),
                });
            }
        }

        Ok(MetadataHealthSummary {
            missing_metadata,
            missing_covers,
            no_image_issues,
            duplicate_issues,
        })
    }

    pub fn list_books(&self, request: BookListRequest) -> AppResult<BookListResponse> {
        self.list_book_summaries(request, false)
    }

    pub fn ensure_book_thumbnails(&self, book_ids: Vec<String>) -> AppResult<Vec<BookThumbnail>> {
        let connection = self.connect()?;
        let thumbnail_dir = thumbnail::thumbnail_dir_from_database_path(&self.path);
        let mut results = Vec::new();

        for requested_book_id in book_ids {
            let book = connection
                .query_row(
                    "SELECT id, path, kind, cover_path, thumbnail_path FROM books WHERE id = ?1",
                    params![&requested_book_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .ok();

            let Some((book_id, book_path, kind, cover_path, existing_thumbnail_path)) = book else {
                results.push(BookThumbnail {
                    book_id: requested_book_id,
                    thumbnail_path: None,
                });
                continue;
            };

            if let Some(existing_thumbnail_path) =
                existing_thumbnail_path.filter(|path| Path::new(path).is_file())
            {
                results.push(BookThumbnail {
                    book_id,
                    thumbnail_path: Some(existing_thumbnail_path),
                });
                continue;
            }

            let thumbnail_path = thumbnail::ensure_book_thumbnail(
                &thumbnail_dir,
                &book_id,
                &book_path,
                &kind,
                cover_path.as_deref(),
            )
            .unwrap_or(None);

            connection.execute(
                "UPDATE books SET thumbnail_path = ?1 WHERE id = ?2",
                params![thumbnail_path.as_deref(), &book_id],
            )?;

            results.push(BookThumbnail {
                book_id,
                thumbnail_path,
            });
        }

        Ok(results)
    }

    pub fn cache_maintenance_summary(&self) -> AppResult<CacheMaintenanceSummary> {
        let connection = self.connect()?;
        let thumbnail_dir = thumbnail::thumbnail_dir_from_database_path(&self.path);
        let (thumbnail_files, thumbnail_bytes) = managed_thumbnail_dir_stats(&thumbnail_dir)?;
        let books_with_thumbnails = connection.query_row(
            "SELECT COUNT(*) FROM books WHERE COALESCE(TRIM(thumbnail_path), '') != ''",
            [],
            |row| row.get::<_, i64>(0),
        )? as usize;
        let missing_thumbnails = self.list_thumbnail_rebuild_candidates(&connection)?.len();

        Ok(CacheMaintenanceSummary {
            thumbnail_cache_dir: thumbnail_dir.to_string_lossy().to_string(),
            thumbnail_files,
            thumbnail_bytes,
            books_with_thumbnails,
            missing_thumbnails,
        })
    }

    pub fn cleanup_thumbnail_cache(&self) -> AppResult<CacheMaintenanceResult> {
        let connection = self.connect()?;
        let thumbnail_dir = thumbnail::thumbnail_dir_from_database_path(&self.path);
        let entries = managed_thumbnail_files(&thumbnail_dir)?;
        let mut removed_files = 0;
        let mut removed_bytes = 0;
        let mut failed = Vec::new();

        for entry in &entries {
            match fs::remove_file(&entry.path) {
                Ok(()) => {
                    removed_files += 1;
                    removed_bytes += entry.bytes;
                }
                Err(error) => failed.push(CacheMaintenanceFailure {
                    path: entry.path.to_string_lossy().to_string(),
                    title: None,
                    reason: error.to_string(),
                }),
            }
        }

        for thumbnail_path in self.list_managed_book_thumbnail_paths(&connection, &thumbnail_dir)? {
            connection.execute(
                "UPDATE books SET thumbnail_path = NULL WHERE thumbnail_path = ?1",
                params![thumbnail_path],
            )?;
        }

        let result = CacheMaintenanceResult {
            operation: "cleanupThumbnailCache".to_string(),
            total: entries.len(),
            succeeded: removed_files,
            failed,
            removed_files,
            removed_bytes,
            rebuilt_thumbnails: 0,
            source_files_affected: false,
        };
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "cache.cleanupThumbnails".to_string(),
            target: thumbnail_dir.to_string_lossy().to_string(),
            summary: format!(
                "Removed {} managed thumbnail files, failed {} entries",
                result.removed_files,
                result.failed.len()
            ),
            reversible: false,
        });
        Ok(result)
    }

    pub fn rebuild_missing_thumbnails(&self) -> AppResult<CacheMaintenanceResult> {
        let connection = self.connect()?;
        let thumbnail_dir = thumbnail::thumbnail_dir_from_database_path(&self.path);
        let candidates = self.list_thumbnail_rebuild_candidates(&connection)?;
        let mut rebuilt_thumbnails = 0;
        let mut failed = Vec::new();

        for candidate in &candidates {
            match thumbnail::ensure_book_thumbnail(
                &thumbnail_dir,
                &candidate.book_id,
                &candidate.book_path,
                &candidate.kind,
                Some(candidate.cover_path.as_str()),
            ) {
                Ok(Some(thumbnail_path)) => {
                    connection.execute(
                        "UPDATE books SET thumbnail_path = ?1 WHERE id = ?2",
                        params![thumbnail_path, &candidate.book_id],
                    )?;
                    rebuilt_thumbnails += 1;
                }
                Ok(None) => failed.push(CacheMaintenanceFailure {
                    path: candidate.book_path.clone(),
                    title: Some(candidate.title.clone()),
                    reason: "No cover path available for thumbnail rebuild".to_string(),
                }),
                Err(error) => failed.push(CacheMaintenanceFailure {
                    path: candidate.book_path.clone(),
                    title: Some(candidate.title.clone()),
                    reason: error.to_string(),
                }),
            }
        }

        let result = CacheMaintenanceResult {
            operation: "rebuildMissingThumbnails".to_string(),
            total: candidates.len(),
            succeeded: rebuilt_thumbnails,
            failed,
            removed_files: 0,
            removed_bytes: 0,
            rebuilt_thumbnails,
            source_files_affected: false,
        };
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "cache.rebuildThumbnails".to_string(),
            target: thumbnail_dir.to_string_lossy().to_string(),
            summary: format!(
                "Rebuilt {} missing thumbnails, failed {} entries",
                result.rebuilt_thumbnails,
                result.failed.len()
            ),
            reversible: false,
        });
        Ok(result)
    }

    fn list_managed_book_thumbnail_paths(
        &self,
        connection: &Connection,
        thumbnail_dir: &Path,
    ) -> AppResult<Vec<String>> {
        let mut statement = connection.prepare(
            "SELECT thumbnail_path FROM books WHERE COALESCE(TRIM(thumbnail_path), '') != ''",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut paths = Vec::new();
        for row in rows {
            let path = row?;
            if is_managed_thumbnail_path(thumbnail_dir, &path) {
                paths.push(path);
            }
        }
        Ok(paths)
    }

    fn list_thumbnail_rebuild_candidates(
        &self,
        connection: &Connection,
    ) -> AppResult<Vec<ThumbnailRebuildCandidate>> {
        let mut statement = connection.prepare(
            "SELECT id, title, path, kind, cover_path, thumbnail_path
             FROM books
             WHERE COALESCE(TRIM(cover_path), '') != ''
             ORDER BY updated_at DESC, title COLLATE NOCASE ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ThumbnailRebuildCandidate {
                book_id: row.get(0)?,
                title: row.get(1)?,
                book_path: row.get(2)?,
                kind: row.get(3)?,
                cover_path: row.get(4)?,
                thumbnail_path: row.get(5)?,
            })
        })?;
        let mut candidates = Vec::new();
        for row in rows {
            let candidate = row?;
            let has_thumbnail = candidate
                .thumbnail_path
                .as_deref()
                .filter(|path| !path.trim().is_empty())
                .is_some_and(|path| Path::new(path).is_file());
            if !has_thumbnail {
                candidates.push(candidate);
            }
        }
        Ok(candidates)
    }

    pub fn list_favorite_books(&self, request: BookListRequest) -> AppResult<BookListResponse> {
        self.list_book_summaries(request, true)
    }

    pub fn list_book_tags(&self, repository_id: Option<String>) -> AppResult<Vec<String>> {
        let connection = self.connect()?;
        if let Some(repository_id) = repository_id.filter(|value| !value.trim().is_empty()) {
            let mut statement = connection.prepare(
                "SELECT DISTINCT book_tags.tag
                 FROM book_tags
                 INNER JOIN books ON books.id = book_tags.book_id
                 WHERE books.repository_id = ?1
                 ORDER BY book_tags.tag COLLATE NOCASE ASC",
            )?;
            let rows =
                statement.query_map(params![repository_id], |row| row.get::<_, String>(0))?;
            return collect_rows(rows);
        }

        let mut statement = connection
            .prepare("SELECT DISTINCT tag FROM book_tags ORDER BY tag COLLATE NOCASE ASC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        collect_rows(rows)
    }

    pub fn list_book_authors(&self, repository_id: Option<String>) -> AppResult<Vec<String>> {
        let connection = self.connect()?;
        if let Some(repository_id) = repository_id.filter(|value| !value.trim().is_empty()) {
            let mut statement = connection.prepare(
                "SELECT DISTINCT book_authors.author
                 FROM book_authors
                 INNER JOIN books ON books.id = book_authors.book_id
                 WHERE books.repository_id = ?1
                 ORDER BY book_authors.author COLLATE NOCASE ASC",
            )?;
            let rows =
                statement.query_map(params![repository_id], |row| row.get::<_, String>(0))?;
            return collect_rows(rows);
        }

        let mut statement = connection
            .prepare("SELECT DISTINCT author FROM book_authors ORDER BY author COLLATE NOCASE ASC")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        collect_rows(rows)
    }

    pub fn list_book_tag_aggregations(&self, query: Option<String>) -> AppResult<Vec<BookAggregationItem>> {
        let connection = self.connect()?;
        let (where_clause, params) = normalized_aggregation_filter("book_tags.normalized_tag", query);
        let sql = format!(
            "SELECT book_tags.tag, COUNT(DISTINCT book_tags.book_id) AS book_count
             FROM book_tags
             INNER JOIN books ON books.id = book_tags.book_id
             {where_clause}
             GROUP BY book_tags.tag
             ORDER BY book_count DESC, book_tags.tag COLLATE NOCASE ASC"
        );
        let mut statement = connection.prepare(sql.as_str())?;
        let rows = statement.query_map(params_from_iter(params), |row| {
            Ok(BookAggregationItem {
                name: row.get(0)?,
                count: row.get::<_, i64>(1)? as usize,
            })
        })?;
        collect_rows(rows)
    }

    pub fn list_book_author_aggregations(&self, query: Option<String>) -> AppResult<Vec<BookAggregationItem>> {
        let connection = self.connect()?;
        let (where_clause, params) = normalized_aggregation_filter("book_authors.normalized_author", query);
        let sql = format!(
            "SELECT book_authors.author, COUNT(DISTINCT book_authors.book_id) AS book_count
             FROM book_authors
             INNER JOIN books ON books.id = book_authors.book_id
             {where_clause}
             GROUP BY book_authors.author
             ORDER BY book_count DESC, book_authors.author COLLATE NOCASE ASC"
        );
        let mut statement = connection.prepare(sql.as_str())?;
        let rows = statement.query_map(params_from_iter(params), |row| {
            Ok(BookAggregationItem {
                name: row.get(0)?,
                count: row.get::<_, i64>(1)? as usize,
            })
        })?;
        collect_rows(rows)
    }

    fn list_book_summaries(
        &self,
        request: BookListRequest,
        favorites_only: bool,
    ) -> AppResult<BookListResponse> {
        let connection = self.connect()?;
        let limit = request.limit.unwrap_or(80).clamp(1, 200);
        let offset = request.offset.unwrap_or(0);
        let (where_clause, params) = build_book_list_filters(&request, favorites_only);
        let order_clause = book_list_order_clause(
            request.sort_key.as_deref().unwrap_or("createdAt"),
            request.sort_direction.as_deref().unwrap_or("desc"),
        );

        let count_sql = format!("SELECT COUNT(*) FROM books {where_clause}");
        let total = connection.query_row(
            count_sql.as_str(),
            params_from_iter(params.clone()),
            |row| row.get::<_, i64>(0),
        )? as usize;

        let mut list_params = params;
        list_params.push(Value::from(limit as i64));
        list_params.push(Value::from(offset as i64));
        let list_sql = format!(
            "{} {where_clause} {order_clause} LIMIT ? OFFSET ?",
            book_summary_select_sql()
        );
        let mut statement = connection.prepare(list_sql.as_str())?;
        let rows = statement.query_map(params_from_iter(list_params), map_book_summary_row)?;
        Ok(BookListResponse {
            books: collect_rows(rows)?,
            total,
        })
    }

    fn list_missing_metadata_books(&self) -> AppResult<Vec<MetadataHealthBookIssue>> {
        let connection = self.connect()?;
        let sql = format!(
            "{} WHERE COALESCE(TRIM(books.description), '') = ''
                OR COALESCE(books.authors_json, '[]') = '[]'
                OR COALESCE(books.tags_json, '[]') = '[]'
             ORDER BY books.updated_at DESC, books.title COLLATE NOCASE ASC",
            book_summary_select_sql()
        );
        let mut statement = connection.prepare(sql.as_str())?;
        let rows = statement.query_map([], |row| {
            let book = map_book_summary_row(row)?;
            let mut reasons = Vec::new();
            if book
                .description
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                reasons.push("缺描述".to_string());
            }
            if book.authors.is_empty() {
                reasons.push("缺作者".to_string());
            }
            if book.tags.is_empty() {
                reasons.push("缺标签".to_string());
            }
            Ok(MetadataHealthBookIssue { book, reasons })
        })?;
        collect_rows(rows)
    }

    fn list_missing_cover_books(&self) -> AppResult<Vec<MetadataHealthBookIssue>> {
        let connection = self.connect()?;
        let sql = format!(
            "{} WHERE COALESCE(TRIM(books.cover_path), '') = ''
                OR COALESCE(TRIM(books.thumbnail_path), '') = ''
             ORDER BY books.updated_at DESC, books.title COLLATE NOCASE ASC",
            book_summary_select_sql()
        );
        let mut statement = connection.prepare(sql.as_str())?;
        let rows = statement.query_map([], |row| {
            let book = map_book_summary_row(row)?;
            let mut reasons = Vec::new();
            if book
                .cover_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                reasons.push("缺封面".to_string());
            }
            if book
                .thumbnail_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                reasons.push("缺缩略图".to_string());
            }
            Ok(MetadataHealthBookIssue { book, reasons })
        })?;
        collect_rows(rows)
    }

    fn latest_repository_scan_histories(&self) -> AppResult<Vec<RepositoryScanHistoryRecord>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, repository_id, repository_name, repository_path, scanned_at, summary_json
             FROM repository_scan_history AS history
             WHERE NOT EXISTS (
                SELECT 1 FROM repository_scan_history AS newer
                WHERE newer.repository_id = history.repository_id
                  AND (
                    newer.scanned_at > history.scanned_at
                    OR (newer.scanned_at = history.scanned_at AND newer.rowid > history.rowid)
                  )
             )
             ORDER BY scanned_at DESC, rowid DESC",
        )?;
        let rows = statement.query_map([], map_repository_scan_history_row)?;
        collect_rows(rows)
    }

    pub fn list_favorite_collections(&self) -> AppResult<Vec<FavoriteCollection>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT favorite_collections.id, favorite_collections.name,
                    favorite_collections.cover_path, favorite_collections.description,
                    COUNT(favorite_books.id) AS book_count,
                    favorite_collections.is_default, favorite_collections.created_at, favorite_collections.updated_at
             FROM favorite_collections
             LEFT JOIN favorite_collection_books ON favorite_collection_books.collection_id = favorite_collections.id
             LEFT JOIN books AS favorite_books ON favorite_books.path = favorite_collection_books.book_path
             GROUP BY favorite_collections.id
             ORDER BY favorite_collections.is_default DESC, favorite_collections.created_at ASC",
        )?;
        let rows = statement.query_map([], map_favorite_collection_row)?;
        collect_rows(rows)
    }

    pub fn create_favorite_collection(&self, name: &str) -> AppResult<FavoriteCollection> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::Database("收藏夹名称不能为空".to_string()));
        }

        let connection = self.connect()?;
        let id = uuid::Uuid::new_v4().to_string();
        connection.execute(
            "INSERT INTO favorite_collections (id, name, is_default, created_at, updated_at)
             VALUES (?1, ?2, 0, datetime('now'), datetime('now'))",
            params![&id, trimmed_name],
        )?;
        self.get_favorite_collection(&id)
    }

    pub fn rename_favorite_collection(
        &self,
        collection_id: &str,
        name: &str,
    ) -> AppResult<FavoriteCollection> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::Database("收藏夹名称不能为空".to_string()));
        }

        let connection = self.connect()?;
        connection.execute(
            "UPDATE favorite_collections SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![trimmed_name, collection_id],
        )?;
        self.get_favorite_collection(collection_id)
    }

    pub fn update_favorite_collection_metadata(
        &self,
        collection_id: &str,
        cover_path: Option<&str>,
        description: Option<&str>,
    ) -> AppResult<FavoriteCollection> {
        let normalized_cover_path = cover_path.map(str::trim).filter(|value| !value.is_empty());
        let normalized_description = description.map(str::trim).filter(|value| !value.is_empty());

        let connection = self.connect()?;
        connection.execute(
            "UPDATE favorite_collections
             SET cover_path = ?1, description = ?2, updated_at = datetime('now')
             WHERE id = ?3",
            params![normalized_cover_path, normalized_description, collection_id],
        )?;
        self.get_favorite_collection(collection_id)
    }

    pub fn delete_favorite_collection(&self, collection_id: &str) -> AppResult<()> {
        if collection_id == "default" {
            return Err(AppError::Database("默认收藏夹不能删除".to_string()));
        }

        let connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        connection.execute(
            "DELETE FROM favorite_collections WHERE id = ?1 AND is_default = 0",
            params![collection_id],
        )?;
        Ok(())
    }

    pub fn add_book_to_favorite_collection(
        &self,
        book_path: &str,
        collection_id: &str,
    ) -> AppResult<()> {
        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        add_book_to_favorite_collection_tx(&transaction, book_path, collection_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn add_books_to_favorite_collection(
        &self,
        book_paths: &[String],
        collection_id: &str,
    ) -> AppResult<()> {
        if book_paths.is_empty() {
            return Ok(());
        }

        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        for book_path in book_paths {
            add_book_to_favorite_collection_tx(&transaction, book_path, collection_id)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn remove_book_from_favorite_collection(
        &self,
        book_path: &str,
        collection_id: &str,
    ) -> AppResult<()> {
        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        remove_book_from_favorite_collection_tx(&transaction, book_path, collection_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn remove_books_from_favorite_collection(
        &self,
        book_paths: &[String],
        collection_id: &str,
    ) -> AppResult<()> {
        if book_paths.is_empty() {
            return Ok(());
        }

        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        for book_path in book_paths {
            remove_book_from_favorite_collection_tx(&transaction, book_path, collection_id)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn move_books_between_favorite_collections(
        &self,
        book_paths: &[String],
        source_collection_id: &str,
        target_collection_id: &str,
    ) -> AppResult<()> {
        if book_paths.is_empty() || source_collection_id == target_collection_id {
            return Ok(());
        }

        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        for book_path in book_paths {
            add_book_to_favorite_collection_tx(&transaction, book_path, target_collection_id)?;
            remove_book_from_favorite_collection_tx(&transaction, book_path, source_collection_id)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn remove_books_from_all_favorite_collections(
        &self,
        book_paths: &[String],
    ) -> AppResult<()> {
        if book_paths.is_empty() {
            return Ok(());
        }

        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        for book_path in book_paths {
            remove_book_from_all_favorite_collections_tx(&transaction, book_path)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_book_favorite_collections(
        &self,
        book_path: &str,
    ) -> AppResult<Vec<FavoriteCollection>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT favorite_collections.id, favorite_collections.name,
                    favorite_collections.cover_path, favorite_collections.description,
                    COUNT(favorite_books.id) AS book_count,
                    favorite_collections.is_default, favorite_collections.created_at, favorite_collections.updated_at
             FROM favorite_collections
             INNER JOIN favorite_collection_books ON favorite_collection_books.collection_id = favorite_collections.id
             LEFT JOIN books AS favorite_books ON favorite_books.path = favorite_collection_books.book_path
             WHERE favorite_collection_books.book_path = ?1
             GROUP BY favorite_collections.id
             ORDER BY favorite_collections.is_default DESC, favorite_collections.created_at ASC",
        )?;
        let rows = statement.query_map(params![book_path], map_favorite_collection_row)?;
        collect_rows(rows)
    }

    fn get_favorite_collection(&self, collection_id: &str) -> AppResult<FavoriteCollection> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT favorite_collections.id, favorite_collections.name,
                    favorite_collections.cover_path, favorite_collections.description,
                    COUNT(favorite_books.id) AS book_count,
                    favorite_collections.is_default, favorite_collections.created_at, favorite_collections.updated_at
             FROM favorite_collections
             LEFT JOIN favorite_collection_books ON favorite_collection_books.collection_id = favorite_collections.id
             LEFT JOIN books AS favorite_books ON favorite_books.path = favorite_collection_books.book_path
             WHERE favorite_collections.id = ?1
             GROUP BY favorite_collections.id",
        )?;
        Ok(statement.query_row(params![collection_id], map_favorite_collection_row)?)
    }

    pub fn set_book_favorite(&self, book_path: &str, favorite: bool) -> AppResult<()> {
        let mut connection = self.connect()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        let transaction = connection.transaction()?;
        if favorite {
            add_book_to_favorite_collection_tx(&transaction, book_path, "default")?;
        } else {
            remove_book_from_all_favorite_collections_tx(&transaction, book_path)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn rename_book_title(&self, book_path: &str, title: &str) -> AppResult<Book> {
        let trimmed_title = title.trim();
        if trimmed_title.is_empty() {
            return Err(AppError::Database("漫画标题不能为空".to_string()));
        }

        let connection = self.connect()?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE books
             SET title = ?1, title_override = ?1, updated_at = datetime('now')
             WHERE path = ?2",
            params![trimmed_title, book_path],
        )?;
        refresh_book_search_text_tx(&transaction, book_path)?;
        transaction.commit()?;
        self.get_book_by_path(book_path)
    }

    pub fn reset_book_title(&self, book_path: &str) -> AppResult<Book> {
        let connection = self.connect()?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE books
             SET title = COALESCE(NULLIF(scanned_title, ''), title),
                 title_override = NULL,
                 updated_at = datetime('now')
             WHERE path = ?1",
            params![book_path],
        )?;
        refresh_book_search_text_tx(&transaction, book_path)?;
        transaction.commit()?;
        self.get_book_by_path(book_path)
    }

    pub fn update_book_metadata(&self, request: UpdateBookMetadataRequest) -> AppResult<Book> {
        let trimmed_title = request.title.trim();
        if trimmed_title.is_empty() {
            return Err(AppError::Database("漫画标题不能为空".to_string()));
        }

        let description = request
            .description
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let authors = normalize_people(&request.authors);
        let tags = normalize_tags(&request.tags);
        let authors_json = serde_json::to_string(&authors)?;
        let tags_json = serde_json::to_string(&tags)?;

        let connection = self.connect()?;
        let transaction = connection.unchecked_transaction()?;
        let book_id = transaction.query_row(
            "SELECT id FROM books WHERE path = ?1",
            params![&request.book_path],
            |row| row.get::<_, String>(0),
        )?;
        transaction.execute(
            "UPDATE books
             SET title = ?1,
                 title_override = ?1,
                 description = ?2,
                 authors_json = ?3,
                 tags_json = ?4,
                 updated_at = datetime('now')
             WHERE path = ?5",
            params![
                trimmed_title,
                description.as_deref(),
                &authors_json,
                &tags_json,
                &request.book_path,
            ],
        )?;
        replace_book_authors_tx(&transaction, &book_id, &authors)?;
        transaction.execute("DELETE FROM book_tags WHERE book_id = ?1", params![&book_id])?;
        for tag in tags {
            transaction.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag, normalized_tag) VALUES (?1, ?2, ?3)",
                params![&book_id, &tag, normalize_search_text(&tag)],
            )?;
        }
        refresh_book_search_text_tx(&transaction, &request.book_path)?;
        transaction.commit()?;
        self.get_book_by_path(&request.book_path)
    }

    pub fn update_book_authors(&self, book_path: &str, authors: Vec<String>) -> AppResult<Book> {
        let authors = normalize_people(&authors);
        let authors_json = serde_json::to_string(&authors)?;

        let connection = self.connect()?;
        let transaction = connection.unchecked_transaction()?;
        let book_id = transaction.query_row(
            "SELECT id FROM books WHERE path = ?1",
            params![book_path],
            |row| row.get::<_, String>(0),
        )?;
        transaction.execute(
            "UPDATE books
             SET authors_json = ?1,
                 updated_at = datetime('now')
             WHERE path = ?2",
            params![&authors_json, book_path],
        )?;
        replace_book_authors_tx(&transaction, &book_id, &authors)?;
        refresh_book_search_text_tx(&transaction, book_path)?;
        transaction.commit()?;
        let book = self.get_book_by_path(book_path)?;
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "metadata.updateAuthors".to_string(),
            target: book_path.to_string(),
            summary: format!("Updated authors for {}", book.title),
            reversible: false,
        });
        Ok(book)
    }

    pub fn update_book_tags(&self, book_path: &str, tags: Vec<String>) -> AppResult<Book> {
        let tags = normalize_tags(&tags);
        let tags_json = serde_json::to_string(&tags)?;

        let connection = self.connect()?;
        let transaction = connection.unchecked_transaction()?;
        let book_id = transaction.query_row(
            "SELECT id FROM books WHERE path = ?1",
            params![book_path],
            |row| row.get::<_, String>(0),
        )?;
        transaction.execute(
            "UPDATE books
             SET tags_json = ?1,
                 updated_at = datetime('now')
             WHERE path = ?2",
            params![&tags_json, book_path],
        )?;
        transaction.execute(
            "DELETE FROM book_tags WHERE book_id = ?1",
            params![&book_id],
        )?;
        for tag in tags {
            transaction.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag, normalized_tag) VALUES (?1, ?2, ?3)",
                params![&book_id, &tag, normalize_search_text(&tag)],
            )?;
        }
        refresh_book_search_text_tx(&transaction, book_path)?;
        transaction.commit()?;
        let book = self.get_book_by_path(book_path)?;
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "metadata.updateTags".to_string(),
            target: book_path.to_string(),
            summary: format!("Updated tags for {}", book.title),
            reversible: false,
        });
        Ok(book)
    }

    pub fn get_book(&self, book_id: &str) -> AppResult<Book> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(book_select_sql("WHERE id = ?1").as_str())?;
        let mut book = statement.query_row(params![book_id], map_book_row)?;
        book.chapters = self.list_chapters(book_id)?;
        Ok(book)
    }

    fn get_book_by_path(&self, book_path: &str) -> AppResult<Book> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(book_select_sql("WHERE path = ?1").as_str())?;
        let mut book = statement.query_row(params![book_path], map_book_row)?;
        book.chapters = self.list_chapters(&book.id)?;
        Ok(book)
    }

    pub fn list_chapters(&self, book_id: &str) -> AppResult<Vec<Chapter>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, book_id, source_chapter_id, title, path, order_index, page_count
             FROM chapters WHERE book_id = ?1 ORDER BY order_index ASC, title ASC",
        )?;
        let rows = statement.query_map(params![book_id], |row| {
            Ok(Chapter {
                id: row.get(0)?,
                book_id: row.get(1)?,
                source_chapter_id: row.get(2)?,
                title: row.get(3)?,
                path: row.get(4)?,
                order: row.get(5)?,
                page_count: row.get::<_, i64>(6)? as usize,
                pages: Vec::new(),
            })
        })?;
        collect_rows(rows)
    }

    pub fn list_pages(&self, chapter_id: &str) -> AppResult<Vec<Page>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT page_index, name, path, uri FROM pages WHERE chapter_id = ?1 ORDER BY page_index ASC",
        )?;
        let rows = statement.query_map(params![chapter_id], |row| {
            Ok(Page {
                index: row.get::<_, i64>(0)? as usize,
                name: row.get(1)?,
                path: row.get(2)?,
                uri: row.get(3)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn update_progress(&self, book_id: &str, chapter_id: &str, page: usize) -> AppResult<()> {
        let connection = self.connect()?;
        let now = sqlite_rfc3339_now();
        let (book_path, chapter_path, chapter_title) = connection.query_row(
            "SELECT books.path, chapters.path, chapters.title
             FROM books
             INNER JOIN chapters ON chapters.book_id = books.id
             WHERE books.id = ?1 AND chapters.id = ?2",
            params![book_id, chapter_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        connection.execute(
            "UPDATE books
             SET last_chapter_id = ?1, last_page = ?2, last_read_at = ?3, updated_at = ?3
             WHERE id = ?4",
            params![chapter_id, page as i64, &now, book_id],
        )?;
        connection.execute(
            "INSERT INTO reading_history (id, book_path, chapter_path, chapter_title, page, read_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![uuid::Uuid::new_v4().to_string(), book_path, chapter_path, chapter_title, page as i64, now],
        )?;
        Ok(())
    }

    pub fn mark_book_read(&self, book_id: &str) -> AppResult<Book> {
        let connection = self.connect()?;
        let sql = format!(
            "SELECT id, page_count
             FROM chapters
             WHERE book_id = ?1
             {FINAL_CHAPTER_ORDER_SQL}
             LIMIT 1"
        );
        let (chapter_id, page_count) =
            connection.query_row(sql.as_str(), params![book_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?;
        self.update_progress(book_id, &chapter_id, page_count.saturating_sub(1))?;
        self.get_book(book_id)
    }

    pub fn mark_book_unread(&self, book_id: &str) -> AppResult<Book> {
        let connection = self.connect()?;
        let book_path = connection.query_row(
            "SELECT path FROM books WHERE id = ?1",
            params![book_id],
            |row| row.get::<_, String>(0),
        )?;
        connection.execute(
            "UPDATE books
             SET last_chapter_id = NULL, last_page = 0, last_read_at = NULL, updated_at = datetime('now')
             WHERE id = ?1",
            params![book_id],
        )?;
        connection.execute(
            "DELETE FROM reading_history WHERE book_path = ?1",
            params![book_path],
        )?;
        self.get_book(book_id)
    }

    pub fn list_reading_history(&self) -> AppResult<Vec<ReadingHistoryRecord>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT reading_history.id, books.id, books.title, books.path, books.kind, books.cover_path,
                    chapters.id, COALESCE(chapters.title, reading_history.chapter_title),
                    reading_history.page, reading_history.read_at
             FROM reading_history
             INNER JOIN books ON books.path = reading_history.book_path
             LEFT JOIN chapters ON chapters.book_id = books.id AND chapters.path = reading_history.chapter_path
             ORDER BY reading_history.read_at DESC
             LIMIT 500",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ReadingHistoryRecord {
                id: row.get(0)?,
                book_id: row.get(1)?,
                book_title: row.get(2)?,
                book_path: row.get(3)?,
                book_kind: row.get(4)?,
                cover_path: row.get(5)?,
                chapter_id: row.get(6)?,
                chapter_title: row.get(7)?,
                page: row.get::<_, i64>(8)? as usize,
                read_at: row.get(9)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn list_reading_history_by_book(&self) -> AppResult<Vec<ReadingHistoryRecord>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT history.id, books.id, books.title, books.path, books.kind, books.cover_path,
                    chapters.id, COALESCE(chapters.title, history.chapter_title),
                    history.page, history.read_at
             FROM reading_history AS history
             INNER JOIN books ON books.path = history.book_path
             LEFT JOIN chapters ON chapters.book_id = books.id AND chapters.path = history.chapter_path
             WHERE NOT EXISTS (
                SELECT 1
                FROM reading_history AS newer
                WHERE newer.book_path = history.book_path
                  AND (
                    newer.read_at > history.read_at
                    OR (newer.read_at = history.read_at AND newer.rowid > history.rowid)
                  )
             )
             ORDER BY history.read_at DESC, history.rowid DESC
             LIMIT 500",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ReadingHistoryRecord {
                id: row.get(0)?,
                book_id: row.get(1)?,
                book_title: row.get(2)?,
                book_path: row.get(3)?,
                book_kind: row.get(4)?,
                cover_path: row.get(5)?,
                chapter_id: row.get(6)?,
                chapter_title: row.get(7)?,
                page: row.get::<_, i64>(8)? as usize,
                read_at: row.get(9)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn get_reader_settings(&self) -> AppResult<ReaderSettings> {
        let connection = self.connect()?;
        let value = connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'reader'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();

        match value {
            Some(value) => Ok(serde_json::from_str(&value).unwrap_or_default()),
            None => Ok(ReaderSettings::default()),
        }
    }

    pub fn save_reader_settings(&self, settings: &ReaderSettings) -> AppResult<()> {
        let connection = self.connect()?;
        let value = serde_json::to_string(settings)?;
        connection.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES ('reader', ?1, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![value],
        )?;
        Ok(())
    }

    pub fn get_book_reader_settings(&self, book_id: &str) -> AppResult<Option<ReaderSettings>> {
        let connection = self.connect()?;
        let value = connection
            .query_row(
                "SELECT value FROM book_reader_settings WHERE book_id = ?1",
                params![book_id],
                |row| row.get::<_, String>(0),
            )
            .ok();

        Ok(value.map(|value| serde_json::from_str(&value).unwrap_or_default()))
    }

    pub fn get_effective_reader_settings(&self, book_id: &str) -> AppResult<ReaderSettings> {
        Ok(self.get_effective_reader_settings_state(book_id)?.settings)
    }

    pub fn get_effective_reader_settings_state(
        &self,
        book_id: &str,
    ) -> AppResult<EffectiveReaderSettingsState> {
        if let Some(settings) = self.get_book_reader_settings(book_id)? {
            return Ok(EffectiveReaderSettingsState {
                settings,
                has_book_reader_settings: true,
            });
        }

        Ok(EffectiveReaderSettingsState {
            settings: self.get_reader_settings()?,
            has_book_reader_settings: false,
        })
    }

    pub fn save_book_reader_settings(
        &self,
        book_id: &str,
        settings: &ReaderSettings,
    ) -> AppResult<()> {
        let connection = self.connect()?;
        let value = serde_json::to_string(settings)?;
        connection.execute(
            "INSERT INTO book_reader_settings (book_id, value, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(book_id) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![book_id, value],
        )?;
        Ok(())
    }

    pub fn clear_book_reader_settings(&self, book_id: &str) -> AppResult<()> {
        let connection = self.connect()?;
        connection.execute(
            "DELETE FROM book_reader_settings WHERE book_id = ?1",
            params![book_id],
        )?;
        Ok(())
    }

    fn list_book_reader_settings(&self) -> AppResult<Vec<PerBookReaderSettings>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT book_id, value FROM book_reader_settings ORDER BY book_id COLLATE NOCASE ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let value: String = row.get(1)?;
            Ok(PerBookReaderSettings {
                book_id: row.get(0)?,
                settings: serde_json::from_str(&value).unwrap_or_default(),
            })
        })?;
        collect_rows(rows)
    }

    pub fn get_library_view_settings(&self) -> AppResult<LibraryViewSettings> {
        let connection = self.connect()?;
        let value = connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'library_view'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();

        match value {
            Some(value) => Ok(serde_json::from_str(&value).unwrap_or_default()),
            None => Ok(LibraryViewSettings::default()),
        }
    }

    pub fn save_library_view_settings(&self, settings: &LibraryViewSettings) -> AppResult<()> {
        let connection = self.connect()?;
        let value = serde_json::to_string(settings)?;
        connection.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES ('library_view', ?1, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![value],
        )?;
        Ok(())
    }

    pub fn export_settings(&self) -> AppResult<SettingsExport> {
        Ok(SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: sqlite_rfc3339_now(),
            reader: self.get_reader_settings()?,
            library_view: self.get_library_view_settings()?,
            per_book_reader_settings: self.list_book_reader_settings()?,
        })
    }

    pub fn import_settings_export(
        &self,
        settings_export: SettingsExport,
    ) -> AppResult<SettingsExport> {
        settings_export
            .validate_for_import(SETTINGS_SCHEMA_VERSION)
            .map_err(AppError::Serde)?;

        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        upsert_setting_value(
            &transaction,
            "reader",
            &serde_json::to_string(&settings_export.reader)?,
        )?;
        upsert_setting_value(
            &transaction,
            "library_view",
            &serde_json::to_string(&settings_export.library_view)?,
        )?;
        transaction.execute("DELETE FROM book_reader_settings", [])?;
        for override_settings in &settings_export.per_book_reader_settings {
            transaction.execute(
                "INSERT INTO book_reader_settings (book_id, value, updated_at)
                 SELECT ?1, ?2, datetime('now')
                 WHERE EXISTS (SELECT 1 FROM books WHERE id = ?1)
                 ON CONFLICT(book_id) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![
                    override_settings.book_id.as_str(),
                    serde_json::to_string(&override_settings.settings)?
                ],
            )?;
        }
        transaction.commit()?;

        let exported = self.export_settings()?;
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "settings.import".to_string(),
            target: "settings".to_string(),
            summary: format!(
                "Imported settings schema version {}",
                exported.schema_version
            ),
            reversible: true,
        });
        Ok(exported)
    }

    pub fn restore_default_settings(
        &self,
        scope: SettingsRestoreScope,
    ) -> AppResult<SettingsExport> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;

        if matches!(
            scope,
            SettingsRestoreScope::All | SettingsRestoreScope::Reader
        ) {
            upsert_setting_value(
                &transaction,
                "reader",
                &serde_json::to_string(&ReaderSettings::default())?,
            )?;
            transaction.execute("DELETE FROM book_reader_settings", [])?;
        }

        if matches!(
            scope,
            SettingsRestoreScope::All | SettingsRestoreScope::LibraryView
        ) {
            upsert_setting_value(
                &transaction,
                "library_view",
                &serde_json::to_string(&LibraryViewSettings::default())?,
            )?;
        }

        transaction.commit()?;
        let exported = self.export_settings()?;
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "settings.restoreDefaults".to_string(),
            target: format!("{scope:?}"),
            summary: "Restored default settings".to_string(),
            reversible: true,
        });
        Ok(exported)
    }

    pub fn create_database_backup(&self, backup_path: &str) -> AppResult<DatabaseBackupResult> {
        let backup_path = normalize_non_empty_path(backup_path, "backup path")?;
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if backup_path == self.path {
            return Err(AppError::Database(
                "Backup path must be different from the active database path".to_string(),
            ));
        }
        if backup_path.is_dir() {
            return Err(AppError::Database(
                "Backup path must point to a file, not a directory".to_string(),
            ));
        }
        self.write_database_backup_atomically(&backup_path, &temporary_backup_path(&backup_path))?;
        let bytes = fs::metadata(&backup_path)?.len();

        let result = DatabaseBackupResult {
            backup_path: backup_path.to_string_lossy().to_string(),
            created_at: sqlite_rfc3339_now(),
            bytes,
            source_files_affected: false,
        };
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "database.backup".to_string(),
            target: result.backup_path.clone(),
            summary: format!("Created database backup ({} bytes)", result.bytes),
            reversible: false,
        });
        Ok(result)
    }

    fn write_database_backup_atomically(
        &self,
        backup_path: &Path,
        temporary_path: &Path,
    ) -> AppResult<()> {
        let previous_path = temporary_replacement_path(backup_path);
        let connection = self.connect()?;
        let escaped_path = temporary_path.to_string_lossy().replace('\'', "''");
        let backup_result = (|| -> AppResult<()> {
            if temporary_path.exists() {
                fs::remove_file(temporary_path)?;
            }
            if previous_path.exists() {
                fs::remove_file(&previous_path)?;
            }

            connection.execute_batch(&format!("VACUUM INTO '{}';", escaped_path))?;
            validate_database_restore_candidate(temporary_path)?;

            let had_existing_backup = backup_path.exists();
            if had_existing_backup {
                fs::rename(backup_path, &previous_path)?;
            }

            if let Err(error) = fs::rename(temporary_path, backup_path) {
                if had_existing_backup {
                    let _ = fs::rename(&previous_path, backup_path);
                }
                return Err(AppError::Io(error.to_string()));
            }

            if had_existing_backup {
                let _ = fs::remove_file(&previous_path);
            }
            Ok(())
        })();

        if let Err(error) = backup_result {
            let _ = fs::remove_file(temporary_path);
            return Err(error);
        }

        Ok(())
    }

    pub fn restore_database_backup(&self, backup_path: &str) -> AppResult<DatabaseRestoreResult> {
        let backup_path = normalize_non_empty_path(backup_path, "backup path")?;
        if backup_path == self.path {
            return Err(AppError::Database(
                "Restore path must be different from the active database path".to_string(),
            ));
        }
        validate_database_restore_candidate(&backup_path)?;

        let restored_at = sqlite_rfc3339_now();
        let rollback_path = self.rollback_database_path(&restored_at);
        if let Some(parent) = rollback_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&self.path, &rollback_path)?;
        let replace_result = (|| -> AppResult<()> {
            fs::copy(&backup_path, &self.path)?;
            self.migrate()?;
            validate_database_file(&self.path)
        })();

        if let Err(error) = replace_result {
            let _ = fs::copy(&rollback_path, &self.path);
            let _ = self.migrate();
            return Err(error);
        }

        let result = DatabaseRestoreResult {
            restored_from: backup_path.to_string_lossy().to_string(),
            restored_at,
            rollback_path: rollback_path.to_string_lossy().to_string(),
            source_files_affected: false,
        };
        self.append_operation_log_non_blocking(OperationLogRequest {
            operation_type: "database.restore".to_string(),
            target: result.restored_from.clone(),
            summary: format!("Restored database; rollback at {}", result.rollback_path),
            reversible: true,
        });
        Ok(result)
    }

    fn rollback_database_path(&self, restored_at: &str) -> PathBuf {
        let suffix = restored_at
            .chars()
            .map(|value| {
                if value.is_ascii_alphanumeric() {
                    value
                } else {
                    '-'
                }
            })
            .collect::<String>();
        self.path
            .with_file_name(format!("inkreader-rollback-{suffix}.sqlite3"))
    }

    pub fn list_operation_logs(&self, limit: Option<usize>) -> AppResult<Vec<OperationLogRecord>> {
        let connection = self.connect()?;
        let limit = limit.unwrap_or(100).clamp(1, 500);
        let mut statement = connection.prepare(
            "SELECT id, operation_type, target, summary, reversible, created_at
             FROM operation_logs
             ORDER BY created_at DESC, rowid DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], map_operation_log_row)?;
        collect_rows(rows)
    }

    fn append_operation_log(&self, request: OperationLogRequest) -> AppResult<()> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        append_operation_log_tx(&transaction, &request)?;
        trim_operation_logs_tx(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    fn append_operation_log_non_blocking(&self, request: OperationLogRequest) {
        let _ = self.append_operation_log(request);
    }

    pub fn remove_repository(&self, repository_id: &str) -> AppResult<()> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        delete_repository_external_records(&transaction, repository_id)?;
        delete_repository_records(&transaction, repository_id)?;
        transaction.commit()?;
        Ok(())
    }

    // ── Bookmarks ──────────────────────────────────────────────────────

    pub fn list_bookmarks(
        &self,
        book_id: &str,
    ) -> AppResult<Vec<crate::models::bookmark::Bookmark>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, book_id, chapter_id, page_index, title, note, created_at, updated_at
             FROM bookmarks WHERE book_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map(params![book_id], |row| {
            Ok(crate::models::bookmark::Bookmark {
                id: row.get(0)?,
                book_id: row.get(1)?,
                chapter_id: row.get(2)?,
                page_index: row.get::<_, i64>(3)? as usize,
                title: row.get(4)?,
                note: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn create_bookmark(
        &self,
        request: &crate::models::bookmark::CreateBookmarkRequest,
    ) -> AppResult<crate::models::bookmark::Bookmark> {
        let id = uuid::Uuid::new_v4().to_string();

        let title = request.title.as_deref().unwrap_or("").to_string();
        let title = if title.is_empty() {
            format!("第{}页", request.page_index + 1)
        } else {
            title
        };

        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO bookmarks (id, book_id, chapter_id, page_index, title, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))",
            params![id, request.book_id, request.chapter_id, request.page_index as i64, title, request.note],
        )?;

        let now = connection.query_row(
            "SELECT created_at FROM bookmarks WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )?;

        Ok(crate::models::bookmark::Bookmark {
            id,
            book_id: request.book_id.clone(),
            chapter_id: request.chapter_id.clone(),
            page_index: request.page_index,
            title,
            note: request.note.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn delete_bookmark(&self, bookmark_id: &str) -> AppResult<()> {
        let connection = self.connect()?;
        connection.execute("DELETE FROM bookmarks WHERE id = ?1", params![bookmark_id])?;
        Ok(())
    }

    pub fn is_page_bookmarked(
        &self,
        book_id: &str,
        chapter_id: &str,
        page_index: usize,
    ) -> AppResult<bool> {
        let connection = self.connect()?;
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM bookmarks WHERE book_id = ?1 AND chapter_id = ?2 AND page_index = ?3)",
            params![book_id, chapter_id, page_index as i64],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
}

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> AppResult<()> {
    let mut statement = connection.prepare(format!("PRAGMA table_info({table})").as_str())?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing_column in columns {
        if existing_column? == column {
            return Ok(());
        }
    }

    connection.execute(
        format!("ALTER TABLE {table} ADD COLUMN {column} {definition}").as_str(),
        [],
    )?;
    Ok(())
}

fn sqlite_rfc3339_now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn upsert_setting_value(transaction: &Transaction<'_>, key: &str, value: &str) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO settings (key, value, updated_at)
         VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value],
    )?;
    Ok(())
}

fn app_local_database_path() -> AppResult<PathBuf> {
    let exe_path = std::env::current_exe().map_err(|error| {
        AppError::Database(format!("无法确定 InkReader 可执行文件位置: {error}"))
    })?;
    let app_dir = exe_path.parent().ok_or_else(|| {
        AppError::Database(format!(
            "无法根据可执行文件位置确定应用目录: {}",
            exe_path.display()
        ))
    })?;
    Ok(app_dir.join("data").join("inkreader.sqlite3"))
}

fn copy_legacy_database_if_needed(app: &tauri::AppHandle, new_path: &Path) -> AppResult<()> {
    if new_path.exists() {
        return Ok(());
    }

    let legacy_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Database(error.to_string()))?;
    let legacy_path = legacy_dir.join("inkreader.sqlite3");
    if !legacy_path.exists() {
        return Ok(());
    }

    fs::copy(&legacy_path, new_path).map_err(|error| {
        AppError::Database(format!(
            "无法将旧数据文件从 {} 复制到 {}: {error}",
            legacy_path.display(),
            new_path.display()
        ))
    })?;

    for suffix in ["wal", "shm"] {
        let legacy_sidecar = legacy_dir.join(format!("inkreader.sqlite3-{suffix}"));
        if !legacy_sidecar.exists() {
            continue;
        }

        let new_sidecar = new_path.with_file_name(format!("inkreader.sqlite3-{suffix}"));
        fs::copy(&legacy_sidecar, &new_sidecar).map_err(|error| {
            AppError::Database(format!(
                "无法将旧 SQLite 附加文件从 {} 复制到 {}: {error}",
                legacy_sidecar.display(),
                new_sidecar.display()
            ))
        })?;
    }

    Ok(())
}

fn normalize_non_empty_path(value: &str, label: &str) -> AppResult<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Database(format!("{label} is required")));
    }
    Ok(PathBuf::from(trimmed))
}

fn validate_database_file(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::Database(format!(
            "Database backup file does not exist: {}",
            path.display()
        )));
    }

    let database = Database {
        path: path.to_path_buf(),
    };
    database.migrate()?;
    let connection = database.connect()?;
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Database(format!(
            "Database integrity check failed: {integrity}"
        )));
    }

    for table in ["repositories", "books", "chapters", "pages", "settings"] {
        let exists = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get::<_, i64>(0),
        )?;
        if exists != 1 {
            return Err(AppError::Database(format!(
                "Database backup is missing required table: {table}"
            )));
        }
    }

    Ok(())
}

fn validate_database_restore_candidate(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::Database(format!(
            "Database backup file does not exist: {}",
            path.display()
        )));
    }

    let validation_path = std::env::temp_dir().join(format!(
        "inkreader-restore-validate-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    fs::copy(path, &validation_path)?;
    let result = validate_database_file(&validation_path);
    let _ = fs::remove_file(&validation_path);
    result
}

fn temporary_backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("inkreader-backup.sqlite3");
    path.with_file_name(format!(".{file_name}.tmp-{}", uuid::Uuid::new_v4()))
}

fn temporary_replacement_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("inkreader-backup.sqlite3");
    path.with_file_name(format!(".{file_name}.previous-{}", uuid::Uuid::new_v4()))
}

fn delete_repository_records(
    transaction: &rusqlite::Transaction<'_>,
    repository_id: &str,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM pages WHERE chapter_id IN (
            SELECT chapters.id FROM chapters
            INNER JOIN books ON books.id = chapters.book_id
            WHERE books.repository_id = ?1
        )",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM chapters WHERE book_id IN (SELECT id FROM books WHERE repository_id = ?1)",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM book_tags WHERE book_id IN (SELECT id FROM books WHERE repository_id = ?1)",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM book_authors WHERE book_id IN (SELECT id FROM books WHERE repository_id = ?1)",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM books WHERE repository_id = ?1",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM repositories WHERE id = ?1",
        params![repository_id],
    )?;
    Ok(())
}

fn delete_book_records_by_path(
    transaction: &rusqlite::Transaction<'_>,
    book_path: &str,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM pages WHERE chapter_id IN (
            SELECT chapters.id FROM chapters
            INNER JOIN books ON books.id = chapters.book_id
            WHERE books.path = ?1
        )",
        params![book_path],
    )?;
    transaction.execute(
        "DELETE FROM chapters WHERE book_id IN (SELECT id FROM books WHERE path = ?1)",
        params![book_path],
    )?;
    transaction.execute(
        "DELETE FROM book_tags WHERE book_id IN (SELECT id FROM books WHERE path = ?1)",
        params![book_path],
    )?;
    transaction.execute(
        "DELETE FROM book_authors WHERE book_id IN (SELECT id FROM books WHERE path = ?1)",
        params![book_path],
    )?;
    transaction.execute("DELETE FROM books WHERE path = ?1", params![book_path])?;
    Ok(())
}

fn delete_repository_external_records(
    transaction: &rusqlite::Transaction<'_>,
    repository_id: &str,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM favorite_books WHERE book_path IN (SELECT path FROM books WHERE repository_id = ?1)",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM favorite_collection_books WHERE book_path IN (SELECT path FROM books WHERE repository_id = ?1)",
        params![repository_id],
    )?;
    transaction.execute(
        "DELETE FROM reading_history WHERE book_path IN (SELECT path FROM books WHERE repository_id = ?1)",
        params![repository_id],
    )?;
    Ok(())
}

fn delete_book_external_records_by_path(
    transaction: &rusqlite::Transaction<'_>,
    book_path: &str,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM favorite_books WHERE book_path = ?1",
        params![book_path],
    )?;
    transaction.execute(
        "DELETE FROM favorite_collection_books WHERE book_path = ?1",
        params![book_path],
    )?;
    transaction.execute(
        "DELETE FROM reading_history WHERE book_path = ?1",
        params![book_path],
    )?;
    Ok(())
}

fn sql_placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}

fn add_book_to_favorite_collection_tx(
    transaction: &rusqlite::Transaction<'_>,
    book_path: &str,
    collection_id: &str,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO favorite_collection_books (collection_id, book_path, created_at, updated_at)
         VALUES (?1, ?2, datetime('now'), datetime('now'))
         ON CONFLICT(collection_id, book_path) DO UPDATE SET updated_at = excluded.updated_at",
        params![collection_id, book_path],
    )?;

    if collection_id == "default" {
        transaction.execute(
            "INSERT INTO favorite_books (book_path, created_at, updated_at)
             VALUES (?1, datetime('now'), datetime('now'))
             ON CONFLICT(book_path) DO UPDATE SET updated_at = excluded.updated_at",
            params![book_path],
        )?;
    }

    Ok(())
}

fn remove_book_from_favorite_collection_tx(
    transaction: &rusqlite::Transaction<'_>,
    book_path: &str,
    collection_id: &str,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM favorite_collection_books WHERE collection_id = ?1 AND book_path = ?2",
        params![collection_id, book_path],
    )?;

    if collection_id == "default" {
        transaction.execute(
            "DELETE FROM favorite_books WHERE book_path = ?1",
            params![book_path],
        )?;
    }

    Ok(())
}

fn remove_book_from_all_favorite_collections_tx(
    transaction: &rusqlite::Transaction<'_>,
    book_path: &str,
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM favorite_books WHERE book_path = ?1",
        params![book_path],
    )?;
    transaction.execute(
        "DELETE FROM favorite_collection_books WHERE book_path = ?1",
        params![book_path],
    )?;
    Ok(())
}

fn insert_book(
    transaction: &rusqlite::Transaction<'_>,
    book: &Book,
    progress: Option<&PreservedBookState>,
    preserved_bookmarks: Option<&Vec<PreservedBookmark>>,
) -> AppResult<()> {
    let normalized_authors = normalize_people(&book.authors);
    let authors_json = serde_json::to_string(&normalized_authors)?;
    let normalized_tags = normalize_tags(&book.tags);
    let tags_json = serde_json::to_string(&normalized_tags)?;
    let last_chapter_id = progress
        .and_then(|state| {
            if state.chapter_path.is_some() {
                state.last_chapter_id.as_deref()
            } else {
                None
            }
        })
        .and_then(|previous_chapter_id| {
            book.chapters
                .iter()
                .any(|chapter| chapter.id == previous_chapter_id)
                .then_some(previous_chapter_id)
        })
        .or_else(|| {
            progress
                .and_then(|state| state.chapter_path.as_deref())
                .and_then(|previous_chapter_path| {
                    book.chapters
                        .iter()
                        .find(|chapter| chapter.path == previous_chapter_path)
                        .map(|chapter| chapter.id.as_str())
                })
        })
        .or(book.last_chapter_id.as_deref());
    let last_page = progress
        .map(|state| state.last_page)
        .unwrap_or(book.last_page);
    let created_at = progress
        .map(|state| state.created_at.as_str())
        .unwrap_or(&book.created_at);
    let last_read_at = progress
        .and_then(|state| state.last_read_at.as_deref())
        .or(book.last_read_at.as_deref());
    let scanned_title = &book.scanned_title;
    let title_override = progress
        .and_then(|state| state.title_override.as_deref())
        .or(book.title_override.as_deref());
    let title = title_override.unwrap_or(scanned_title);
    let search_text_normalized = build_book_search_text(title, scanned_title, &normalized_authors, &normalized_tags);

    transaction.execute(
        "INSERT INTO books (
          id, repository_id, source_id, title, scanned_title, title_override, path, kind, metadata_path, cover_path, thumbnail_path,
          published_at, description, authors_json, tags_json, chapter_count, total_pages,
          last_chapter_id, last_page, last_read_at, created_at, updated_at, scan_signature, search_text_normalized
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
        params![
            &book.id,
            &book.repository_id,
            book.source_id.as_deref(),
            title,
            scanned_title,
            title_override,
            &book.path,
            &book.kind,
            book.metadata_path.as_deref(),
            book.cover_path.as_deref(),
            book.thumbnail_path.as_deref(),
            book.published_at.as_deref(),
            book.description.as_deref(),
            &authors_json,
            &tags_json,
            book.chapter_count as i64,
            book.total_pages as i64,
            last_chapter_id,
            last_page as i64,
            last_read_at,
            created_at,
            &book.updated_at,
            book.scan_signature.as_deref(),
            &search_text_normalized,
        ],
    )?;

    replace_book_authors_tx(transaction, &book.id, &normalized_authors)?;
    for tag in normalized_tags {
        transaction.execute(
            "INSERT OR IGNORE INTO book_tags (book_id, tag, normalized_tag) VALUES (?1, ?2, ?3)",
            params![&book.id, &tag, normalize_search_text(&tag)],
        )?;
    }

    for chapter in &book.chapters {
        transaction.execute(
            "INSERT INTO chapters (id, book_id, source_chapter_id, title, path, order_index, page_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &chapter.id,
                &chapter.book_id,
                chapter.source_chapter_id.as_deref(),
                &chapter.title,
                &chapter.path,
                chapter.order,
                chapter.page_count as i64,
                &book.created_at,
                &book.updated_at,
            ],
        )?;

        for page in &chapter.pages {
            transaction.execute(
                "INSERT INTO pages (id, chapter_id, page_index, name, path, uri) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    &chapter.id,
                    page.index as i64,
                    &page.name,
                    &page.path,
                    &page.uri,
                ],
            )?;
        }
    }

    // Restore preserved bookmarks by matching chapter_path + page_index
    if let Some(bookmarks) = preserved_bookmarks {
        for bookmark in bookmarks {
            // Find the new chapter ID by matching chapter path
            let new_chapter_id = bookmark.chapter_path.as_deref().and_then(|chapter_path| {
                book.chapters
                    .iter()
                    .find(|chapter| chapter.path == chapter_path)
                    .map(|chapter| chapter.id.as_str())
            });

            if let Some(chapter_id) = new_chapter_id {
                let id = uuid::Uuid::new_v4().to_string();
                transaction.execute(
                    "INSERT INTO bookmarks (id, book_id, chapter_id, page_index, title, note, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))",
                    params![id, &book.id, chapter_id, bookmark.page_index as i64, &bookmark.title, &bookmark.note],
                )?;
            }
        }
    }

    Ok(())
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let value = tag.trim();
        if value.is_empty() {
            continue;
        }
        let value = value.to_string();
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    normalized
}

fn normalize_people(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let value = value.to_string();
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    normalized
}

fn normalize_search_text(value: &str) -> String {
    let simplified = T2S_CONVERTER.with(|converter| {
        converter
            .borrow()
            .as_ref()
            .map(|converter| converter.convert(value))
            .unwrap_or_else(|| value.to_string())
    });
    simplified
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_book_search_text(title: &str, scanned_title: &str, authors: &[String], tags: &[String]) -> String {
    let authors_text = authors.join(" ");
    let tags_text = tags.join(" ");
    normalize_search_text(&[title, scanned_title, &authors_text, &tags_text].join(" "))
}

fn replace_book_authors_tx(
    transaction: &rusqlite::Transaction<'_>,
    book_id: &str,
    authors: &[String],
) -> AppResult<()> {
    transaction.execute("DELETE FROM book_authors WHERE book_id = ?1", params![book_id])?;
    for author in authors {
        transaction.execute(
            "INSERT OR IGNORE INTO book_authors (book_id, author, normalized_author) VALUES (?1, ?2, ?3)",
            params![book_id, author, normalize_search_text(author)],
        )?;
    }
    Ok(())
}

fn refresh_book_search_text_tx(
    transaction: &rusqlite::Transaction<'_>,
    book_path: &str,
) -> AppResult<()> {
    let (title, scanned_title, authors_json, tags_json) = transaction.query_row(
        "SELECT title, COALESCE(NULLIF(scanned_title, ''), title), authors_json, tags_json
         FROM books WHERE path = ?1",
        params![book_path],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    let authors: Vec<String> = serde_json::from_str(&authors_json).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let search_text = build_book_search_text(&title, &scanned_title, &authors, &tags);
    transaction.execute(
        "UPDATE books SET search_text_normalized = ?1 WHERE path = ?2",
        params![search_text, book_path],
    )?;
    Ok(())
}

fn backfill_book_tags(connection: &Connection) -> AppResult<()> {
    let already_backfilled = connection
        .query_row(
            "SELECT 1 FROM settings WHERE key = ?1",
            params![BOOK_TAGS_BACKFILL_SETTING_KEY],
            |_| Ok(()),
        )
        .is_ok();
    if already_backfilled {
        return Ok(());
    }

    let mut statement = connection.prepare(
        "SELECT books.id, books.tags_json
         FROM books
         WHERE books.tags_json <> '[]'
           AND NOT EXISTS (
             SELECT 1 FROM book_tags WHERE book_tags.book_id = books.id
           )",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (book_id, tags_json) = row?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let normalized = normalize_tags(&tags);
        for tag in normalized {
            connection.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag, normalized_tag) VALUES (?1, ?2, ?3)",
                params![&book_id, &tag, normalize_search_text(&tag)],
            )?;
        }
    }

    connection.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at)
         VALUES (?1, 'true', datetime('now'))",
        params![BOOK_TAGS_BACKFILL_SETTING_KEY],
    )?;

    Ok(())
}

fn backfill_search_indexes(connection: &Connection) -> AppResult<()> {
    let already_backfilled = connection
        .query_row(
            "SELECT 1 FROM settings WHERE key = ?1",
            params![SEARCH_INDEX_BACKFILL_SETTING_KEY],
            |_| Ok(()),
        )
        .is_ok();
    if already_backfilled {
        return Ok(());
    }

    let mut statement = connection.prepare(
        "SELECT id, title, COALESCE(NULLIF(scanned_title, ''), title), path, authors_json, tags_json
         FROM books",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;

    for row in rows {
        let (book_id, title, scanned_title, book_path, authors_json, tags_json) = row?;
        let authors = normalize_people(&serde_json::from_str::<Vec<String>>(&authors_json).unwrap_or_default());
        let tags = normalize_tags(&serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default());
        let search_text = build_book_search_text(&title, &scanned_title, &authors, &tags);

        connection.execute(
            "UPDATE books
             SET search_text_normalized = ?1,
                 authors_json = ?2,
                 tags_json = ?3
             WHERE path = ?4",
            params![
                search_text,
                serde_json::to_string(&authors)?,
                serde_json::to_string(&tags)?,
                book_path,
            ],
        )?;
        connection.execute("DELETE FROM book_authors WHERE book_id = ?1", params![&book_id])?;
        for author in &authors {
            connection.execute(
                "INSERT OR IGNORE INTO book_authors (book_id, author, normalized_author) VALUES (?1, ?2, ?3)",
                params![&book_id, author, normalize_search_text(author)],
            )?;
        }
        connection.execute("DELETE FROM book_tags WHERE book_id = ?1", params![&book_id])?;
        for tag in &tags {
            connection.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag, normalized_tag) VALUES (?1, ?2, ?3)",
                params![&book_id, tag, normalize_search_text(tag)],
            )?;
        }
    }

    if !already_backfilled {
        connection.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at)
             VALUES (?1, 'true', datetime('now'))",
            params![SEARCH_INDEX_BACKFILL_SETTING_KEY],
        )?;
    }

    Ok(())
}

const FINAL_CHAPTER_ORDER_SQL: &str =
    "ORDER BY order_index DESC, title COLLATE NOCASE DESC, id DESC";

fn read_complete_exists_sql() -> String {
    "EXISTS (
      SELECT 1 FROM chapters
      WHERE chapters.book_id = books.id
        AND chapters.id = books.last_chapter_id
        AND books.last_page + 1 >= chapters.page_count
        AND chapters.id = (
          SELECT last_chapter.id
          FROM chapters AS last_chapter
          WHERE last_chapter.book_id = books.id
          ORDER BY last_chapter.order_index DESC, last_chapter.title COLLATE NOCASE DESC, last_chapter.id DESC
          LIMIT 1
        )
    )"
    .to_string()
}

fn book_summary_select_sql() -> String {
    format!(
        "SELECT id, repository_id, source_id, title,
            COALESCE(NULLIF(scanned_title, ''), title) AS scanned_title,
            title_override,
            path, kind, metadata_path, cover_path,
            thumbnail_path, published_at, description, authors_json, tags_json, chapter_count, total_pages,
            last_chapter_id, last_page,
            EXISTS (SELECT 1 FROM favorite_collection_books WHERE favorite_collection_books.book_path = books.path) AS is_favorite,
            {} AS is_read_complete,
            created_at, updated_at, last_read_at
     FROM books",
        read_complete_exists_sql()
    )
}

fn book_select_sql(where_clause: &str) -> String {
    format!(
        "{} {where_clause} ORDER BY updated_at DESC",
        book_summary_select_sql()
    )
}

fn build_book_list_filters(
    request: &BookListRequest,
    favorites_only: bool,
) -> (String, Vec<Value>) {
    let mut filters = Vec::new();
    let mut values = Vec::new();

    if let Some(repository_id) = request
        .repository_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        filters.push("books.repository_id = ?".to_string());
        values.push(Value::Text(repository_id.to_string()));
    }

    if favorites_only {
        if let Some(collection_id) = request
            .collection_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            filters.push("EXISTS (SELECT 1 FROM favorite_collection_books WHERE favorite_collection_books.book_path = books.path AND favorite_collection_books.collection_id = ?)".to_string());
            values.push(Value::Text(collection_id.to_string()));
        } else {
            filters.push("EXISTS (SELECT 1 FROM favorite_collection_books WHERE favorite_collection_books.book_path = books.path)".to_string());
        }
    }

    if let Some(query) = request
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        for term in normalize_search_text(query)
            .split_whitespace()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            filters.push("COALESCE(books.search_text_normalized, '') LIKE ? ESCAPE '\\'".to_string());
            values.push(Value::Text(format!("%{}%", escape_like_pattern(term))));
        }
    }

    let mut selected_authors = request
        .authors
        .as_ref()
        .map(|authors| normalize_people(authors))
        .unwrap_or_default();

    if selected_authors.is_empty() {
        if let Some(author) = request
            .author
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            selected_authors.push(author.to_string());
        }
    }

    for author in selected_authors {
        filters.push("books.id IN (SELECT book_authors.book_id FROM book_authors WHERE book_authors.normalized_author = ?)".to_string());
        values.push(Value::Text(normalize_search_text(&author)));
    }

    let mut selected_tags = request
        .tags
        .as_ref()
        .map(|tags| {
            tags.iter()
                .map(|tag| tag.trim())
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if selected_tags.is_empty() {
        if let Some(tag) = request
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            selected_tags.push(tag.to_string());
        }
    }

    for tag in selected_tags {
        filters.push("books.id IN (SELECT book_tags.book_id FROM book_tags WHERE book_tags.tag = ?)".to_string());
        values.push(Value::Text(tag));
    }

    let excluded_tags = request
        .exclude_tags
        .as_ref()
        .map(|tags| normalize_tags(tags))
        .unwrap_or_default();
    for tag in excluded_tags {
        filters.push("books.id NOT IN (SELECT book_tags.book_id FROM book_tags WHERE book_tags.tag = ?)".to_string());
        values.push(Value::Text(tag));
    }

    if let Some(metadata_filters) = &request.metadata_filters {
        let metadata_filters = metadata_filters
            .iter()
            .map(|filter| filter.trim())
            .filter(|filter| !filter.is_empty())
            .collect::<HashSet<_>>();
        if metadata_filters.contains("missingDescription") {
            filters.push("COALESCE(TRIM(books.description), '') = ''".to_string());
        }
        if metadata_filters.contains("missingAuthors") {
            filters.push("COALESCE(books.authors_json, '[]') = '[]'".to_string());
        }
        if metadata_filters.contains("missingTags") {
            filters.push("COALESCE(books.tags_json, '[]') = '[]'".to_string());
        }
        if metadata_filters.contains("missingCover") {
            filters.push("COALESCE(TRIM(books.cover_path), '') = ''".to_string());
        }
        if metadata_filters.contains("missingPublishedAt") {
            filters.push("(books.published_at IS NULL OR CAST(books.published_at AS INTEGER) <= 0)".to_string());
        }
    }

    if let Some(reading_status) = request
        .reading_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        match reading_status {
            "reading" => filters.push(format!(
                "(books.last_read_at IS NOT NULL OR books.last_page > 0) AND NOT {}",
                read_complete_exists_sql()
            )),
            "read" => filters.push(format!(
                "(books.last_read_at IS NOT NULL OR books.last_page > 0) AND {}",
                read_complete_exists_sql()
            )),
            "unread" => {
                filters.push("(books.last_read_at IS NULL AND books.last_page = 0)".to_string())
            }
            _ => {}
        }
    }

    if let Some(favorite_status) = request
        .favorite_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        match favorite_status {
            "favorited" => filters.push("EXISTS (SELECT 1 FROM favorite_collection_books WHERE favorite_collection_books.book_path = books.path)".to_string()),
            "notFavorited" => filters.push("NOT EXISTS (SELECT 1 FROM favorite_collection_books WHERE favorite_collection_books.book_path = books.path)".to_string()),
            _ => {}
        }
    }

    if filters.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", filters.join(" AND ")), values)
    }
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn normalized_aggregation_filter(column: &str, query: Option<String>) -> (String, Vec<Value>) {
    let Some(query) = query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_search_text)
    else {
        return (String::new(), Vec::new());
    };

    (
        format!("WHERE COALESCE({column}, '') LIKE ? ESCAPE '\\'"),
        vec![Value::Text(format!("%{}%", escape_like_pattern(&query)))],
    )
}

fn book_list_order_clause(sort_key: &str, sort_direction: &str) -> &'static str {
    let direction = if sort_direction == "asc" {
        "ASC"
    } else {
        "DESC"
    };
    match sort_key {
        "title" if direction == "ASC" => "ORDER BY books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "title" => "ORDER BY books.title COLLATE NOCASE DESC, books.path COLLATE NOCASE ASC, books.id ASC",
        "totalPages" if direction == "ASC" => "ORDER BY books.total_pages ASC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "totalPages" => "ORDER BY books.total_pages DESC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "lastReadAt" if direction == "ASC" => "ORDER BY books.last_read_at IS NULL ASC, books.last_read_at ASC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "lastReadAt" => "ORDER BY books.last_read_at IS NULL ASC, books.last_read_at DESC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "publishedAt" if direction == "ASC" => "ORDER BY CASE WHEN CAST(books.published_at AS INTEGER) > 0 THEN 0 ELSE 1 END ASC, CAST(books.published_at AS INTEGER) ASC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "publishedAt" => "ORDER BY CASE WHEN CAST(books.published_at AS INTEGER) > 0 THEN 0 ELSE 1 END ASC, CAST(books.published_at AS INTEGER) DESC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        "createdAt" if direction == "ASC" => "ORDER BY books.created_at ASC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        _ => "ORDER BY books.created_at DESC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
    }
}

fn map_book_summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BookSummary> {
    let authors_json: String = row.get(13)?;
    let tags_json: String = row.get(14)?;
    Ok(BookSummary {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        source_id: row.get(2)?,
        title: row.get(3)?,
        scanned_title: row.get(4)?,
        title_override: row.get(5)?,
        path: row.get(6)?,
        kind: row.get(7)?,
        metadata_path: row.get(8)?,
        cover_path: row.get(9)?,
        thumbnail_path: row.get(10)?,
        published_at: row.get(11)?,
        description: row.get(12)?,
        authors: serde_json::from_str(&authors_json).unwrap_or_default(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        chapter_count: row.get::<_, i64>(15)? as usize,
        total_pages: row.get::<_, i64>(16)? as usize,
        last_chapter_id: row.get(17)?,
        last_page: row.get::<_, i64>(18)? as usize,
        is_favorite: row.get::<_, i64>(19)? != 0,
        is_read_complete: row.get::<_, i64>(20)? != 0,
        created_at: row.get(21)?,
        updated_at: row.get(22)?,
        last_read_at: row.get(23)?,
    })
}

fn map_book_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Book> {
    let summary = map_book_summary_row(row)?;
    Ok(Book {
        id: summary.id,
        repository_id: summary.repository_id,
        source_id: summary.source_id,
        title: summary.title,
        scanned_title: summary.scanned_title,
        title_override: summary.title_override,
        path: summary.path,
        kind: summary.kind,
        metadata_path: summary.metadata_path,
        cover_path: summary.cover_path,
        thumbnail_path: summary.thumbnail_path,
        published_at: summary.published_at,
        description: summary.description,
        authors: summary.authors,
        tags: summary.tags,
        chapter_count: summary.chapter_count,
        total_pages: summary.total_pages,
        last_chapter_id: summary.last_chapter_id,
        last_page: summary.last_page,
        last_read_at: summary.last_read_at,
        is_read_complete: summary.is_read_complete,
        is_favorite: summary.is_favorite,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        scan_signature: None,
        chapters: Vec::new(),
    })
}

fn map_favorite_collection_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavoriteCollection> {
    Ok(FavoriteCollection {
        id: row.get(0)?,
        name: row.get(1)?,
        cover_path: row.get(2)?,
        description: row.get(3)?,
        book_count: row.get::<_, i64>(4)? as usize,
        is_default: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn map_repository_scan_history_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RepositoryScanHistoryRecord> {
    let summary_json: String = row.get(5)?;
    let summary = serde_json::from_str::<RepositoryScanSummary>(&summary_json)
        .unwrap_or_else(|_| RepositoryScanSummary::default());
    Ok(RepositoryScanHistoryRecord {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        repository_name: row.get(2)?,
        repository_path: row.get(3)?,
        scanned_at: row.get(4)?,
        summary,
    })
}

fn map_operation_log_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationLogRecord> {
    Ok(OperationLogRecord {
        id: row.get(0)?,
        operation_type: row.get(1)?,
        target: row.get(2)?,
        summary: row.get(3)?,
        reversible: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
    })
}

fn append_operation_log_tx(
    transaction: &rusqlite::Transaction<'_>,
    request: &OperationLogRequest,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO operation_logs (id, operation_type, target, summary, reversible, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
        params![
            uuid::Uuid::new_v4().to_string(),
            request.operation_type.trim(),
            request.target.trim(),
            request.summary.trim(),
            if request.reversible { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

fn trim_operation_logs_tx(transaction: &rusqlite::Transaction<'_>) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM operation_logs
         WHERE id NOT IN (
             SELECT id FROM operation_logs
             ORDER BY created_at DESC, rowid DESC
             LIMIT ?1
         )",
        params![OPERATION_LOG_RETENTION_LIMIT],
    )?;
    Ok(())
}

fn collect_rows<T, F>(rows: rusqlite::MappedRows<'_, F>) -> AppResult<Vec<T>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

struct ManagedThumbnailFile {
    path: PathBuf,
    bytes: u64,
}

struct ThumbnailRebuildCandidate {
    book_id: String,
    title: String,
    book_path: String,
    kind: String,
    cover_path: String,
    thumbnail_path: Option<String>,
}

fn managed_thumbnail_dir_stats(thumbnail_dir: &Path) -> AppResult<(usize, u64)> {
    let files = managed_thumbnail_files(thumbnail_dir)?;
    let bytes = files.iter().map(|file| file.bytes).sum();
    Ok((files.len(), bytes))
}

fn managed_thumbnail_files(thumbnail_dir: &Path) -> AppResult<Vec<ManagedThumbnailFile>> {
    if !thumbnail_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(thumbnail_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_managed_thumbnail_filename(&path) {
            continue;
        }
        files.push(ManagedThumbnailFile {
            bytes: entry.metadata()?.len(),
            path,
        });
    }
    Ok(files)
}

fn is_managed_thumbnail_path(thumbnail_dir: &Path, value: &str) -> bool {
    let path = Path::new(value);
    path.parent().is_some_and(|parent| parent == thumbnail_dir)
        && is_managed_thumbnail_filename(path)
}

fn is_managed_thumbnail_filename(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jpg"))
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| uuid::Uuid::parse_str(stem).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDatabase {
        database: Database,
        dir: PathBuf,
    }

    impl TempDatabase {
        fn new() -> Self {
            let dir =
                std::env::temp_dir().join(format!("inkreader-db-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("inkreader.sqlite3");
            let database = Database { path };
            database.migrate().unwrap();
            Self { database, dir }
        }
    }

    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn repository(path: &str, id: &str) -> Repository {
        Repository {
            id: id.to_string(),
            name: "仓库".to_string(),
            path: path.to_string(),
            book_count: 1,
            last_scanned_at: Some("2026-01-01T00:00:00Z".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn book(repository_id: &str, path: &str, title: &str) -> Book {
        Book {
            id: uuid::Uuid::new_v4().to_string(),
            repository_id: repository_id.to_string(),
            source_id: None,
            title: title.to_string(),
            scanned_title: title.to_string(),
            title_override: None,
            path: path.to_string(),
            kind: "folder".to_string(),
            metadata_path: None,
            cover_path: None,
            thumbnail_path: None,
            published_at: None,
            description: None,
            authors: Vec::new(),
            tags: Vec::new(),
            chapter_count: 0,
            total_pages: 0,
            last_chapter_id: None,
            last_page: 0,
            last_read_at: None,
            is_read_complete: false,
            is_favorite: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            scan_signature: None,
            chapters: Vec::new(),
        }
    }

    fn book_with_chapters(repository_id: &str, path: &str, title: &str) -> Book {
        let mut next_book = book(repository_id, path, title);
        let first_chapter_id = uuid::Uuid::new_v4().to_string();
        let second_chapter_id = uuid::Uuid::new_v4().to_string();
        next_book.chapter_count = 2;
        next_book.total_pages = 5;
        next_book.last_chapter_id = Some(first_chapter_id.clone());
        next_book.chapters = vec![
            Chapter {
                id: first_chapter_id,
                book_id: next_book.id.clone(),
                source_chapter_id: None,
                title: "Chapter 1".to_string(),
                path: format!("{path}/chapter-1"),
                order: 1,
                page_count: 2,
                pages: Vec::new(),
            },
            Chapter {
                id: second_chapter_id,
                book_id: next_book.id.clone(),
                source_chapter_id: None,
                title: "Chapter 2".to_string(),
                path: format!("{path}/chapter-2"),
                order: 2,
                page_count: 3,
                pages: Vec::new(),
            },
        ];
        next_book
    }

    fn scan_summary(scanned_books: usize) -> RepositoryScanSummary {
        RepositoryScanSummary {
            total_entries: scanned_books + 1,
            scanned_books,
            unchanged_books: 1,
            skipped_entries: Vec::new(),
            failed_entries: Vec::new(),
            duplicate_books: Vec::new(),
        }
    }

    fn scan_issue(
        path: &str,
        code: crate::models::repository::RepositoryScanIssueCode,
    ) -> crate::models::repository::RepositoryScanIssue {
        crate::models::repository::RepositoryScanIssue {
            path: path.to_string(),
            reason: "diagnostic reason".to_string(),
            code,
            severity: crate::models::repository::RepositoryScanIssueSeverity::Warning,
            suggestion: Some("check the source entry".to_string()),
        }
    }

    fn book_list_request() -> BookListRequest {
        BookListRequest {
            repository_id: None,
            collection_id: None,
            query: None,
            author: None,
            authors: None,
            tag: None,
            tags: None,
            exclude_tags: None,
            metadata_filters: None,
            reading_status: None,
            favorite_status: None,
            sort_key: None,
            sort_direction: None,
            limit: None,
            offset: None,
        }
    }

    fn list_first_book(database: &Database) -> BookSummary {
        database
            .list_books(book_list_request())
            .unwrap()
            .books
            .into_iter()
            .next()
            .unwrap()
    }

    fn explain_query_plan(connection: &Connection, sql: &str, params: Vec<Value>) -> Vec<String> {
        let explain_sql = format!("EXPLAIN QUERY PLAN {sql}");
        let mut statement = connection.prepare(&explain_sql).unwrap();
        let rows = statement
            .query_map(params_from_iter(params), |row| row.get::<_, String>(3))
            .unwrap();
        collect_rows(rows).unwrap()
    }

    fn assert_plan_uses_index(plan: &[String], index_name: &str) {
        assert!(
            plan.iter().any(|entry| entry.contains(index_name)),
            "query plan did not use {index_name}: {plan:#?}"
        );
    }

    fn custom_reader_settings() -> ReaderSettings {
        ReaderSettings {
            mode: "scroll".to_string(),
            fit: "width".to_string(),
            direction: "rtl".to_string(),
            background: "#000000".to_string(),
            space_scroll_ratio: 0.75,
            space_hold_speed_ratio: 3.5,
            brightness: 1.2,
            contrast: 0.9,
            page_animation: "fade".to_string(),
            preload_cache_limit: 120,
            auto_scroll_speed: 100,
            auto_scroll_start_delay: 1.0,
            auto_scroll_stop_on_manual_scroll: false,
        }
    }

    fn custom_library_view_settings() -> LibraryViewSettings {
        LibraryViewSettings {
            layout: "list".to_string(),
            cover_size: "large".to_string(),
            show_authors: false,
            show_tags: true,
            tag_limit: 8,
            title_line_clamp: 3,
            title_font_size: 17,
        }
    }

    #[test]
    fn export_settings_includes_current_domains() {
        let temp = TempDatabase::new();
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();
        temp.database
            .save_library_view_settings(&custom_library_view_settings())
            .unwrap();

        let settings_export = temp.database.export_settings().unwrap();

        assert_eq!(settings_export.schema_version, SETTINGS_SCHEMA_VERSION);
        assert!(!settings_export.exported_at.is_empty());
        assert_eq!(settings_export.reader.mode, "scroll");
        assert_eq!(settings_export.library_view.layout, "list");
    }

    #[test]
    fn book_reader_settings_override_and_clear_global_reader_settings() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book(&repo.id, "F:/repo/book", "Book");
        let book_id = source_book.id.clone();
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();
        temp.database
            .save_reader_settings(&ReaderSettings {
                mode: "double".to_string(),
                ..ReaderSettings::default()
            })
            .unwrap();
        let override_settings = ReaderSettings {
            mode: "scroll".to_string(),
            fit: "width".to_string(),
            ..ReaderSettings::default()
        };

        assert_eq!(
            temp.database
                .get_effective_reader_settings(&book_id)
                .unwrap()
                .mode,
            "double"
        );

        temp.database
            .save_book_reader_settings(&book_id, &override_settings)
            .unwrap();

        assert_eq!(
            temp.database
                .get_book_reader_settings(&book_id)
                .unwrap()
                .unwrap()
                .fit,
            "width"
        );
        assert_eq!(
            temp.database
                .get_effective_reader_settings(&book_id)
                .unwrap()
                .mode,
            "scroll"
        );
        let effective_state = temp
            .database
            .get_effective_reader_settings_state(&book_id)
            .unwrap();
        assert!(effective_state.has_book_reader_settings);
        assert_eq!(effective_state.settings.mode, "scroll");

        temp.database.clear_book_reader_settings(&book_id).unwrap();

        assert!(temp
            .database
            .get_book_reader_settings(&book_id)
            .unwrap()
            .is_none());
        assert_eq!(
            temp.database
                .get_effective_reader_settings(&book_id)
                .unwrap()
                .mode,
            "double"
        );
        let effective_state = temp
            .database
            .get_effective_reader_settings_state(&book_id)
            .unwrap();
        assert!(!effective_state.has_book_reader_settings);
        assert_eq!(effective_state.settings.mode, "double");
    }

    #[test]
    fn settings_export_import_round_trips_per_book_reader_settings() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book(&repo.id, "F:/repo/book", "Book");
        let book_id = source_book.id.clone();
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();
        temp.database
            .save_book_reader_settings(&book_id, &custom_reader_settings())
            .unwrap();

        let exported = temp.database.export_settings().unwrap();
        assert_eq!(exported.per_book_reader_settings.len(), 1);
        assert_eq!(exported.per_book_reader_settings[0].book_id, book_id);
        assert_eq!(exported.per_book_reader_settings[0].settings.mode, "scroll");

        temp.database.clear_book_reader_settings(&book_id).unwrap();
        assert!(temp
            .database
            .get_book_reader_settings(&book_id)
            .unwrap()
            .is_none());

        temp.database.import_settings_export(exported).unwrap();

        assert_eq!(
            temp.database
                .get_book_reader_settings(&book_id)
                .unwrap()
                .unwrap()
                .mode,
            "scroll"
        );
    }

    #[test]
    fn settings_import_skips_per_book_reader_settings_for_missing_books() {
        let temp = TempDatabase::new();
        let settings_export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: custom_reader_settings(),
            library_view: custom_library_view_settings(),
            per_book_reader_settings: vec![PerBookReaderSettings {
                book_id: "missing-book".to_string(),
                settings: ReaderSettings {
                    mode: "double".to_string(),
                    ..ReaderSettings::default()
                },
            }],
        };

        let imported = temp
            .database
            .import_settings_export(settings_export)
            .unwrap();

        assert_eq!(imported.reader.mode, "scroll");
        assert!(imported.per_book_reader_settings.is_empty());
        assert!(temp
            .database
            .get_book_reader_settings("missing-book")
            .unwrap()
            .is_none());
    }

    #[test]
    fn database_backup_restore_round_trips_current_database() {
        let temp = TempDatabase::new();
        let backup_path = std::env::temp_dir().join(format!(
            "inkreader-backup-test-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();

        let backup = temp
            .database
            .create_database_backup(&backup_path.to_string_lossy())
            .unwrap();
        assert_eq!(backup.backup_path, backup_path.to_string_lossy());
        assert!(backup.bytes > 0);
        assert!(!backup.source_files_affected);
        assert!(backup_path.is_file());

        temp.database
            .save_reader_settings(&ReaderSettings::default())
            .unwrap();
        assert_eq!(
            temp.database.get_reader_settings().unwrap().mode,
            ReaderSettings::default().mode
        );

        let restore = temp
            .database
            .restore_database_backup(&backup_path.to_string_lossy())
            .unwrap();
        assert_eq!(restore.restored_from, backup_path.to_string_lossy());
        assert!(!restore.source_files_affected);
        assert!(Path::new(&restore.rollback_path).is_file());
        assert_eq!(
            temp.database.get_reader_settings().unwrap().mode,
            custom_reader_settings().mode
        );

        let _ = fs::remove_file(backup_path);
        let _ = fs::remove_file(restore.rollback_path);
    }

    #[test]
    fn failed_backup_replacement_preserves_existing_backup() {
        let temp = TempDatabase::new();
        let backup_path = temp.dir.join("existing-backup.sqlite3");
        let invalid_temporary_path = temp.dir.join("invalid-temporary-target");
        fs::create_dir_all(&invalid_temporary_path).unwrap();
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();
        temp.database
            .create_database_backup(&backup_path.to_string_lossy())
            .unwrap();
        let original_backup = fs::read(&backup_path).unwrap();

        temp.database
            .save_reader_settings(&ReaderSettings::default())
            .unwrap();
        let result = temp
            .database
            .write_database_backup_atomically(&backup_path, &invalid_temporary_path);

        assert!(result.is_err());
        assert_eq!(fs::read(&backup_path).unwrap(), original_backup);
    }

    #[test]
    fn database_restore_rejects_invalid_backup_without_replacing_current_database() {
        let temp = TempDatabase::new();
        let invalid_path = std::env::temp_dir().join(format!(
            "inkreader-invalid-backup-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        fs::write(&invalid_path, b"not sqlite").unwrap();
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();

        let result = temp
            .database
            .restore_database_backup(&invalid_path.to_string_lossy());

        assert!(result.is_err());
        assert_eq!(
            temp.database.get_reader_settings().unwrap().mode,
            custom_reader_settings().mode
        );

        let _ = fs::remove_file(invalid_path);
    }

    #[test]
    fn completed_operations_append_operation_log_entries() {
        let temp = TempDatabase::new();
        let settings_export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: custom_reader_settings(),
            library_view: custom_library_view_settings(),
            per_book_reader_settings: Vec::new(),
        };

        temp.database
            .import_settings_export(settings_export)
            .unwrap();

        let logs = temp.database.list_operation_logs(Some(10)).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].operation_type, "settings.import");
        assert_eq!(logs[0].target, "settings");
        assert!(logs[0].reversible);
        assert!(logs[0].summary.contains("Imported settings"));
    }

    #[test]
    fn operation_log_retention_keeps_latest_records() {
        let temp = TempDatabase::new();
        for index in 0..(OPERATION_LOG_RETENTION_LIMIT + 5) {
            temp.database
                .append_operation_log(OperationLogRequest {
                    operation_type: "test.operation".to_string(),
                    target: format!("target-{index:03}"),
                    summary: format!("summary-{index:03}"),
                    reversible: index % 2 == 0,
                })
                .unwrap();
        }

        let logs = temp.database.list_operation_logs(Some(600)).unwrap();
        assert_eq!(logs.len(), OPERATION_LOG_RETENTION_LIMIT as usize);
        assert_eq!(logs[0].target, "target-504");
        assert_eq!(logs.last().unwrap().target, "target-005");
    }

    #[test]
    fn import_settings_export_replaces_supported_domains() {
        let temp = TempDatabase::new();
        let settings_export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: custom_reader_settings(),
            library_view: custom_library_view_settings(),
            per_book_reader_settings: Vec::new(),
        };

        let imported = temp
            .database
            .import_settings_export(settings_export)
            .unwrap();

        assert_eq!(imported.reader.mode, "scroll");
        assert_eq!(imported.library_view.layout, "list");
        assert_eq!(temp.database.get_reader_settings().unwrap().fit, "width");
        assert_eq!(
            temp.database
                .get_library_view_settings()
                .unwrap()
                .cover_size,
            "large"
        );
    }

    #[test]
    fn import_settings_export_rejects_unsupported_schema_without_changes() {
        let temp = TempDatabase::new();
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();
        let original_library_view = temp.database.get_library_view_settings().unwrap();
        let settings_export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION + 1,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: ReaderSettings::default(),
            library_view: custom_library_view_settings(),
            per_book_reader_settings: Vec::new(),
        };

        let result = temp.database.import_settings_export(settings_export);

        assert!(result.is_err());
        assert_eq!(temp.database.get_reader_settings().unwrap().mode, "scroll");
        assert_eq!(
            temp.database.get_library_view_settings().unwrap().layout,
            original_library_view.layout
        );
    }

    #[test]
    fn import_settings_export_rejects_invalid_values_without_changes() {
        let temp = TempDatabase::new();
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();
        let settings_export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: ReaderSettings {
                mode: "unsupported".to_string(),
                ..ReaderSettings::default()
            },
            library_view: custom_library_view_settings(),
            per_book_reader_settings: Vec::new(),
        };

        let result = temp.database.import_settings_export(settings_export);

        assert!(result.is_err());
        assert_eq!(temp.database.get_reader_settings().unwrap().mode, "scroll");
        assert_eq!(
            temp.database.get_library_view_settings().unwrap().layout,
            LibraryViewSettings::default().layout
        );
    }

    #[test]
    fn restore_default_settings_respects_scope() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book(&repo.id, "F:/repo/book", "Book");
        let book_id = source_book.id.clone();
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();
        temp.database
            .save_reader_settings(&custom_reader_settings())
            .unwrap();
        temp.database
            .save_library_view_settings(&custom_library_view_settings())
            .unwrap();
        temp.database
            .save_book_reader_settings(&book_id, &custom_reader_settings())
            .unwrap();

        let restored = temp
            .database
            .restore_default_settings(SettingsRestoreScope::Reader)
            .unwrap();

        assert_eq!(restored.reader.mode, ReaderSettings::default().mode);
        assert_eq!(restored.library_view.layout, "list");
        assert!(temp
            .database
            .get_book_reader_settings(&book_id)
            .unwrap()
            .is_none());

        temp.database
            .save_book_reader_settings(&book_id, &custom_reader_settings())
            .unwrap();
        let restored_all = temp
            .database
            .restore_default_settings(SettingsRestoreScope::All)
            .unwrap();

        assert_eq!(
            restored_all.library_view.layout,
            LibraryViewSettings::default().layout
        );
        assert!(temp
            .database
            .get_book_reader_settings(&book_id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_books_filters_by_reading_status() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut read_book = book_with_chapters(&repo.id, "F:/repo/read-book", "Read Book");
        read_book.last_chapter_id = Some(read_book.chapters[1].id.clone());
        read_book.last_page = 2;
        read_book.last_read_at = Some("2026-01-02T00:00:00Z".to_string());
        let mut reading_book = book_with_chapters(&repo.id, "F:/repo/reading-book", "Reading Book");
        reading_book.last_chapter_id = Some(reading_book.chapters[0].id.clone());
        reading_book.last_page = 1;
        reading_book.last_read_at = Some("2026-01-02T00:00:00Z".to_string());
        let unread_book = book_with_chapters(&repo.id, "F:/repo/unread-book", "Unread Book");
        temp.database
            .upsert_scan(&repo, &[read_book, reading_book, unread_book])
            .unwrap();

        let mut read_request = book_list_request();
        read_request.reading_status = Some("read".to_string());
        let read_titles = temp
            .database
            .list_books(read_request)
            .unwrap()
            .books
            .into_iter()
            .map(|book| book.title)
            .collect::<Vec<_>>();
        assert_eq!(read_titles, vec!["Read Book"]);

        let mut reading_request = book_list_request();
        reading_request.reading_status = Some("reading".to_string());
        let reading_titles = temp
            .database
            .list_books(reading_request)
            .unwrap()
            .books
            .into_iter()
            .map(|book| book.title)
            .collect::<Vec<_>>();
        assert_eq!(reading_titles, vec!["Reading Book"]);

        let mut unread_request = book_list_request();
        unread_request.reading_status = Some("unread".to_string());
        let unread_titles = temp
            .database
            .list_books(unread_request)
            .unwrap()
            .books
            .into_iter()
            .map(|book| book.title)
            .collect::<Vec<_>>();
        assert_eq!(unread_titles, vec!["Unread Book"]);
    }

    #[test]
    fn list_books_filters_by_favorite_status() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let favorited_path = "F:/repo/favorited-book";
        let plain_path = "F:/repo/plain-book";
        temp.database
            .upsert_scan(
                &repo,
                &[
                    book(&repo.id, favorited_path, "Favorited Book"),
                    book(&repo.id, plain_path, "Plain Book"),
                ],
            )
            .unwrap();
        temp.database
            .add_book_to_favorite_collection(favorited_path, "default")
            .unwrap();

        let mut favorited_request = book_list_request();
        favorited_request.favorite_status = Some("favorited".to_string());
        let favorited_titles = temp
            .database
            .list_books(favorited_request)
            .unwrap()
            .books
            .into_iter()
            .map(|book| book.title)
            .collect::<Vec<_>>();
        assert_eq!(favorited_titles, vec!["Favorited Book"]);

        let mut not_favorited_request = book_list_request();
        not_favorited_request.favorite_status = Some("notFavorited".to_string());
        let not_favorited_titles = temp
            .database
            .list_books(not_favorited_request)
            .unwrap()
            .books
            .into_iter()
            .map(|book| book.title)
            .collect::<Vec<_>>();
        assert_eq!(not_favorited_titles, vec!["Plain Book"]);
    }

    #[test]
    fn favorite_collection_metadata_saves_and_reloads() {
        let temp = TempDatabase::new();

        let default_collection = temp
            .database
            .list_favorite_collections()
            .unwrap()
            .into_iter()
            .find(|collection| collection.id == "default")
            .unwrap();
        assert!(default_collection.cover_path.is_none());
        assert!(default_collection.description.is_none());
        assert!(default_collection.is_default);

        let collection = temp
            .database
            .create_favorite_collection("Reading Queue")
            .unwrap();
        assert!(collection.cover_path.is_none());
        assert!(collection.description.is_none());

        let updated = temp
            .database
            .update_favorite_collection_metadata(
                &collection.id,
                Some("F:/covers/queue.webp"),
                Some("Books to read next"),
            )
            .unwrap();
        assert_eq!(updated.cover_path.as_deref(), Some("F:/covers/queue.webp"));
        assert_eq!(updated.description.as_deref(), Some("Books to read next"));

        let reloaded = temp
            .database
            .list_favorite_collections()
            .unwrap()
            .into_iter()
            .find(|value| value.id == collection.id)
            .unwrap();
        assert_eq!(reloaded.cover_path.as_deref(), Some("F:/covers/queue.webp"));
        assert_eq!(reloaded.description.as_deref(), Some("Books to read next"));

        let cleared = temp
            .database
            .update_favorite_collection_metadata(&collection.id, Some("  "), Some(""))
            .unwrap();
        assert!(cleared.cover_path.is_none());
        assert!(cleared.description.is_none());

        let default_updated = temp
            .database
            .update_favorite_collection_metadata(
                "default",
                Some("F:/covers/default.png"),
                Some("Default favorites"),
            )
            .unwrap();
        assert!(default_updated.is_default);
        assert_eq!(
            default_updated.description.as_deref(),
            Some("Default favorites")
        );
        assert!(temp.database.delete_favorite_collection("default").is_err());
    }

    #[test]
    fn author_and_tag_aggregations_match_filtered_book_totals() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut first_book = book(&repo.id, "F:/repo/first", "First");
        first_book.authors = vec!["Alice".to_string(), "Bob".to_string()];
        first_book.tags = vec!["Action".to_string(), "Long".to_string()];
        let mut second_book = book(&repo.id, "F:/repo/second", "Second");
        second_book.authors = vec!["Alice".to_string()];
        second_book.tags = vec!["Action".to_string()];
        temp.database
            .upsert_scan(&repo, &[first_book, second_book])
            .unwrap();

        let authors = temp.database.list_book_author_aggregations(None).unwrap();
        assert_eq!(authors[0].name, "Alice");
        assert_eq!(authors[0].count, 2);
        assert_eq!(authors[1].name, "Bob");
        assert_eq!(authors[1].count, 1);

        let tags = temp.database.list_book_tag_aggregations(None).unwrap();
        assert_eq!(tags[0].name, "Action");
        assert_eq!(tags[0].count, 2);
        assert_eq!(tags[1].name, "Long");
        assert_eq!(tags[1].count, 1);

        let mut author_request = book_list_request();
        author_request.author = Some("Alice".to_string());
        let author_response = temp.database.list_books(author_request).unwrap();
        assert_eq!(author_response.total, 2);

        let mut tag_request = book_list_request();
        tag_request.tag = Some("Long".to_string());
        let tag_response = temp.database.list_books(tag_request).unwrap();
        assert_eq!(tag_response.total, 1);
        assert_eq!(tag_response.books[0].title, "First");
    }

    #[test]
    fn list_books_search_matches_simplified_and_traditional_text() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut traditional_book = book(&repo.id, "F:/repo/traditional", "龍珠經典");
        traditional_book.authors = vec!["鳥山明".to_string()];
        traditional_book.tags = vec!["熱血".to_string()];
        let mut simplified_book = book(&repo.id, "F:/repo/simplified", "龙族幻想");
        simplified_book.authors = vec!["江南".to_string()];
        simplified_book.tags = vec!["冒险".to_string()];
        temp.database
            .upsert_scan(&repo, &[traditional_book, simplified_book])
            .unwrap();

        let mut simplified_request = book_list_request();
        simplified_request.query = Some("龙珠 热血".to_string());
        let simplified_response = temp.database.list_books(simplified_request).unwrap();
        assert_eq!(simplified_response.total, 1);
        assert_eq!(simplified_response.books[0].title, "龍珠經典");

        let mut traditional_request = book_list_request();
        traditional_request.query = Some("龍族".to_string());
        let traditional_response = temp.database.list_books(traditional_request).unwrap();
        assert_eq!(traditional_response.total, 1);
        assert_eq!(traditional_response.books[0].title, "龙族幻想");
    }

    #[test]
    fn list_books_filters_by_authors_tags_exclusions_and_missing_metadata() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut complete = book(&repo.id, "F:/repo/complete", "Complete");
        complete.description = Some("description".to_string());
        complete.authors = vec!["Alice".to_string(), "Bob".to_string()];
        complete.tags = vec!["Action".to_string(), "Long".to_string()];
        complete.cover_path = Some("F:/repo/complete/cover.jpg".to_string());
        complete.published_at = Some("1660330835".to_string());
        let mut missing = book(&repo.id, "F:/repo/missing", "Missing");
        missing.authors = vec!["Alice".to_string()];
        missing.tags = vec!["Action".to_string(), "Draft".to_string()];
        missing.published_at = Some("0".to_string());
        temp.database.upsert_scan(&repo, &[complete, missing]).unwrap();

        let mut combo_request = book_list_request();
        combo_request.authors = Some(vec!["Alice".to_string(), "Bob".to_string()]);
        combo_request.tags = Some(vec!["Action".to_string()]);
        combo_request.exclude_tags = Some(vec!["Draft".to_string()]);
        let combo_response = temp.database.list_books(combo_request).unwrap();
        assert_eq!(combo_response.total, 1);
        assert_eq!(combo_response.books[0].title, "Complete");

        let mut metadata_request = book_list_request();
        metadata_request.metadata_filters = Some(vec![
            "missingDescription".to_string(),
            "missingCover".to_string(),
            "missingPublishedAt".to_string(),
        ]);
        let metadata_response = temp.database.list_books(metadata_request).unwrap();
        assert_eq!(metadata_response.total, 1);
        assert_eq!(metadata_response.books[0].title, "Missing");
    }

    #[test]
    fn author_and_tag_exact_filters_use_secondary_indexes() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut first = book(&repo.id, "F:/repo/first", "First");
        first.authors = vec!["Alice".to_string()];
        first.tags = vec!["Action".to_string()];
        temp.database.upsert_scan(&repo, &[first]).unwrap();

        let connection = temp.database.connect().unwrap();
        let author_plan = explain_query_plan(
            &connection,
            "SELECT id FROM books
             WHERE books.id IN (
                SELECT book_authors.book_id FROM book_authors
                WHERE book_authors.normalized_author = ?
             )",
            vec![Value::Text(normalize_search_text("Alice"))],
        );
        assert_plan_uses_index(&author_plan, "idx_book_authors_normalized_author_book_id");

        let tag_plan = explain_query_plan(
            &connection,
            "SELECT id FROM books
             WHERE books.id IN (
                SELECT book_tags.book_id FROM book_tags
                WHERE book_tags.tag = ?
             )",
            vec![Value::Text("Action".to_string())],
        );
        assert_plan_uses_index(&tag_plan, "idx_book_tags_tag_book_id");
    }

    #[test]
    fn list_books_uses_repository_title_index_for_sorted_pagination() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        temp.database
            .upsert_scan(
                &repo,
                &[
                    book(&repo.id, "F:/repo/book-c", "Charlie"),
                    book(&repo.id, "F:/repo/book-a", "alpha"),
                    book(&repo.id, "F:/repo/book-b", "Bravo"),
                ],
            )
            .unwrap();

        let mut request = book_list_request();
        request.repository_id = Some(repo.id.clone());
        request.sort_key = Some("title".to_string());
        request.sort_direction = Some("asc".to_string());
        let response = temp.database.list_books(request).unwrap();

        assert_eq!(
            response
                .books
                .into_iter()
                .map(|book| book.title)
                .collect::<Vec<_>>(),
            vec!["alpha", "Bravo", "Charlie"]
        );

        let connection = temp.database.connect().unwrap();
        let plan = explain_query_plan(
            &connection,
            "SELECT id FROM books
             WHERE books.repository_id = ?
             ORDER BY books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC
             LIMIT ? OFFSET ?",
            vec![Value::Text(repo.id), Value::from(20), Value::from(0)],
        );

        assert_plan_uses_index(&plan, "idx_books_repository_title_path_id");
        assert!(!plan.iter().any(|entry| entry.contains("USE TEMP B-TREE")));
    }

    #[test]
    fn list_books_returns_and_sorts_by_published_at() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut older_book = book(&repo.id, "F:/repo/older", "Older");
        older_book.published_at = Some("1660330835".to_string());
        let mut newer_book = book(&repo.id, "F:/repo/newer", "Newer");
        newer_book.published_at = Some("1760330835".to_string());
        let mut epoch_book = book(&repo.id, "F:/repo/epoch", "Epoch");
        epoch_book.published_at = Some("0".to_string());
        let missing_book = book(&repo.id, "F:/repo/missing", "Missing");
        temp.database
            .upsert_scan(&repo, &[older_book, newer_book, epoch_book, missing_book])
            .unwrap();

        let mut request = book_list_request();
        request.repository_id = Some(repo.id.clone());
        request.sort_key = Some("publishedAt".to_string());
        request.sort_direction = Some("desc".to_string());
        let response = temp.database.list_books(request).unwrap();

        assert_eq!(
            response
                .books
                .iter()
                .map(|book| book.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Newer", "Older", "Epoch", "Missing"]
        );
        assert_eq!(
            response.books[0].published_at.as_deref(),
            Some("1760330835")
        );
    }

    #[test]
    fn read_complete_subquery_uses_final_chapter_order_index() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut source_book = book_with_chapters(&repo.id, "F:/repo/read-plan", "Read Plan");
        source_book.last_chapter_id = Some(source_book.chapters[1].id.clone());
        source_book.last_page = 2;
        source_book.last_read_at = Some("2026-01-02T00:00:00Z".to_string());
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        let connection = temp.database.connect().unwrap();
        let plan = explain_query_plan(
            &connection,
            &format!(
                "SELECT id FROM books WHERE books.repository_id = ? AND {}",
                read_complete_exists_sql()
            ),
            vec![Value::Text(repo.id)],
        );

        assert_plan_uses_index(&plan, "idx_chapters_book_final_order");
    }

    #[test]
    fn mark_book_read_and_unread_updates_progress_and_history() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book_with_chapters(&repo.id, "F:/repo/progress-book", "Progress Book");
        let book_id = source_book.id.clone();
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        let read_book = temp.database.mark_book_read(&book_id).unwrap();
        assert_eq!(read_book.last_page, 2);
        assert!(read_book.last_read_at.is_some());
        assert!(read_book.is_read_complete);
        assert_eq!(
            read_book.last_chapter_id.as_deref(),
            Some(read_book.chapters[1].id.as_str())
        );
        assert_eq!(temp.database.list_reading_history().unwrap().len(), 1);

        let unread_book = temp.database.mark_book_unread(&book_id).unwrap();
        assert_eq!(unread_book.last_page, 0);
        assert!(unread_book.last_read_at.is_none());
        assert!(!unread_book.is_read_complete);
        assert!(unread_book.last_chapter_id.is_none());
        assert!(temp.database.list_reading_history().unwrap().is_empty());
    }

    #[test]
    fn list_reading_history_by_book_returns_latest_entry_per_book() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let first_book = book_with_chapters(&repo.id, "F:/repo/history-a", "History A");
        let second_book = book_with_chapters(&repo.id, "F:/repo/history-b", "History B");
        let first_chapter = first_book.chapters[0].clone();
        let second_chapter = first_book.chapters[1].clone();
        let other_chapter = second_book.chapters[0].clone();
        temp.database
            .upsert_scan(&repo, &[first_book.clone(), second_book.clone()])
            .unwrap();

        let connection = temp.database.connect().unwrap();
        connection
            .execute(
                "INSERT INTO reading_history (id, book_path, chapter_path, chapter_title, page, read_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    "history-a-old",
                    &first_book.path,
                    &first_chapter.path,
                    &first_chapter.title,
                    0_i64,
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO reading_history (id, book_path, chapter_path, chapter_title, page, read_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    "history-b-latest",
                    &second_book.path,
                    &other_chapter.path,
                    &other_chapter.title,
                    1_i64,
                    "2026-01-03T00:00:00Z"
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO reading_history (id, book_path, chapter_path, chapter_title, page, read_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    "history-a-latest",
                    &first_book.path,
                    &second_chapter.path,
                    &second_chapter.title,
                    2_i64,
                    "2026-01-02T00:00:00Z"
                ],
            )
            .unwrap();

        let raw_history = temp.database.list_reading_history().unwrap();
        assert_eq!(raw_history.len(), 3);

        let grouped_history = temp.database.list_reading_history_by_book().unwrap();
        let grouped_ids = grouped_history
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(grouped_ids, vec!["history-b-latest", "history-a-latest"]);
        assert_eq!(grouped_history[1].book_title, "History A");
        assert_eq!(
            grouped_history[1].chapter_title.as_deref(),
            Some("Chapter 2")
        );
        assert_eq!(grouped_history[1].page, 2);
    }

    #[test]
    fn remove_repository_cleans_path_based_favorites_and_history() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book_with_chapters(&repo.id, "F:/repo/cleanup-book", "Cleanup Book");
        let book_id = source_book.id.clone();
        let chapter_id = source_book.chapters[0].id.clone();
        let book_path = source_book.path.clone();
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();
        temp.database
            .add_book_to_favorite_collection(&book_path, "default")
            .unwrap();
        temp.database
            .update_progress(&book_id, &chapter_id, 1)
            .unwrap();

        temp.database.remove_repository(&repo.id).unwrap();

        let collections = temp.database.list_favorite_collections().unwrap();
        assert_eq!(collections[0].book_count, 0);
        assert_path_row_count(&temp.database, "favorite_books", &book_path, 0);
        assert_path_row_count(&temp.database, "favorite_collection_books", &book_path, 0);
        assert_path_row_count(&temp.database, "reading_history", &book_path, 0);
    }

    #[test]
    fn rescan_preserves_path_based_favorites_and_history_for_existing_books() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book_with_chapters(&repo.id, "F:/repo/rescan-book", "Rescan Book");
        let book_id = source_book.id.clone();
        let chapter_id = source_book.chapters[0].id.clone();
        let book_path = source_book.path.clone();
        temp.database
            .upsert_scan(&repo, std::slice::from_ref(&source_book))
            .unwrap();
        temp.database
            .add_book_to_favorite_collection(&book_path, "default")
            .unwrap();
        temp.database
            .update_progress(&book_id, &chapter_id, 1)
            .unwrap();

        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        let collections = temp.database.list_favorite_collections().unwrap();
        assert_eq!(collections[0].book_count, 1);
        assert_eq!(temp.database.list_reading_history().unwrap().len(), 1);
        assert_path_row_count(&temp.database, "favorite_books", &book_path, 1);
        assert_path_row_count(&temp.database, "favorite_collection_books", &book_path, 1);
        assert_path_row_count(&temp.database, "reading_history", &book_path, 1);
    }

    #[test]
    fn read_complete_uses_deterministic_final_chapter_when_order_ties() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let mut source_book =
            book_with_chapters(&repo.id, "F:/repo/tied-order-book", "Tied Order Book");
        source_book.chapters[0].title = "A Chapter".to_string();
        source_book.chapters[1].title = "Z Chapter".to_string();
        source_book.chapters[0].order = 2;
        source_book.chapters[1].order = 2;
        let book_id = source_book.id.clone();
        let first_chapter_id = source_book.chapters[0].id.clone();
        let second_chapter_id = source_book.chapters[1].id.clone();
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        temp.database
            .update_progress(&book_id, &first_chapter_id, 1)
            .unwrap();
        let reading_book = temp.database.get_book(&book_id).unwrap();
        assert!(!reading_book.is_read_complete);

        let mut reading_request = book_list_request();
        reading_request.reading_status = Some("reading".to_string());
        assert_eq!(
            temp.database.list_books(reading_request).unwrap().books[0].title,
            "Tied Order Book"
        );

        let read_book = temp.database.mark_book_read(&book_id).unwrap();
        assert!(read_book.is_read_complete);
        assert_eq!(
            read_book.last_chapter_id.as_deref(),
            Some(second_chapter_id.as_str())
        );
    }

    fn assert_path_row_count(
        database: &Database,
        table: &str,
        book_path: &str,
        expected_count: i64,
    ) {
        let connection = database.connect().unwrap();
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE book_path = ?1");
        let count = connection
            .query_row(sql.as_str(), params![book_path], |row| row.get::<_, i64>(0))
            .unwrap();
        assert_eq!(count, expected_count);
    }

    #[test]
    fn update_book_metadata_updates_fields_and_tag_index() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let book_path = "F:/repo/metadata-book";
        let mut source_book = book(&repo.id, book_path, "Original Title");
        source_book.tags = vec!["old".to_string()];
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        let updated = temp
            .database
            .update_book_metadata(UpdateBookMetadataRequest {
                book_path: book_path.to_string(),
                title: "Updated Title".to_string(),
                description: Some("  Updated description  ".to_string()),
                authors: vec![" Alice ".to_string(), "".to_string(), "Bob".to_string()],
                tags: vec!["new".to_string(), "new".to_string(), " tag ".to_string()],
            })
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.title_override.as_deref(), Some("Updated Title"));
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.authors, vec!["Alice", "Bob"]);
        assert_eq!(updated.tags, vec!["new", "tag"]);
        assert_eq!(
            temp.database.list_book_tags(None).unwrap(),
            vec!["new".to_string(), "tag".to_string()]
        );
    }

    #[test]
    fn update_book_authors_preserves_title_override() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let book_path = "F:/repo/authors-book";
        temp.database
            .upsert_scan(&repo, &[book(&repo.id, book_path, "Original Title")])
            .unwrap();
        temp.database
            .rename_book_title(book_path, "Custom Title")
            .unwrap();

        let updated = temp
            .database
            .update_book_authors(
                book_path,
                vec![" Alice ".to_string(), "".to_string(), "Bob".to_string()],
            )
            .unwrap();

        assert_eq!(updated.title, "Custom Title");
        assert_eq!(updated.title_override.as_deref(), Some("Custom Title"));
        assert_eq!(updated.authors, vec!["Alice", "Bob"]);
    }

    #[test]
    fn update_book_tags_preserves_title_and_updates_tag_index() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let book_path = "F:/repo/tags-book";
        let mut source_book = book(&repo.id, book_path, "Original Title");
        source_book.tags = vec!["old".to_string()];
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        let updated = temp
            .database
            .update_book_tags(
                book_path,
                vec![" new ".to_string(), "new".to_string(), "tag".to_string()],
            )
            .unwrap();

        assert_eq!(updated.title, "Original Title");
        assert_eq!(updated.title_override, None);
        assert_eq!(updated.tags, vec!["new", "tag"]);
        assert_eq!(
            temp.database.list_book_tags(None).unwrap(),
            vec!["new".to_string(), "tag".to_string()]
        );
    }

    #[test]
    fn repository_scan_history_is_persisted_and_limited_per_repository() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let source_book = book(&repo.id, "F:/repo/book", "Book");
        temp.database.upsert_scan(&repo, &[source_book]).unwrap();

        for index in 0..25 {
            let mut scanned_repo = repo.clone();
            scanned_repo.last_scanned_at = Some(format!("2026-01-01T00:{index:02}:00Z"));
            scanned_repo.updated_at = format!("2026-01-01T00:{index:02}:00Z");
            temp.database
                .save_repository_scan_history(&scanned_repo, &scan_summary(index))
                .unwrap();
        }

        let history = temp.database.list_repository_scan_history().unwrap();
        assert_eq!(history.len(), 20);
        assert_eq!(history[0].repository_id, repo.id);
        assert_eq!(history[0].repository_path, repo.path);
        assert_eq!(history[0].scanned_at, "2026-01-01T00:24:00Z");
        assert_eq!(history[0].summary.scanned_books, 24);
        assert_eq!(history[19].scanned_at, "2026-01-01T00:05:00Z");
    }

    #[test]
    fn metadata_health_summary_uses_book_fields_and_latest_structured_scan_issues() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let missing_book = book(&repo.id, "F:/repo/missing", "Missing");
        let mut complete_book = book(&repo.id, "F:/repo/complete", "Complete");
        complete_book.description = Some("description".to_string());
        complete_book.authors = vec!["author".to_string()];
        complete_book.tags = vec!["tag".to_string()];
        complete_book.cover_path = Some("F:/repo/complete/cover.jpg".to_string());
        complete_book.thumbnail_path = Some("F:/repo/complete/thumb.jpg".to_string());
        temp.database
            .upsert_scan(&repo, &[missing_book, complete_book])
            .unwrap();

        let mut older_repo = repo.clone();
        older_repo.last_scanned_at = Some("2026-01-01T00:00:00Z".to_string());
        let mut older_summary = scan_summary(1);
        older_summary.skipped_entries.push(scan_issue(
            "F:/repo/old-empty",
            crate::models::repository::RepositoryScanIssueCode::NoImages,
        ));
        temp.database
            .save_repository_scan_history(&older_repo, &older_summary)
            .unwrap();

        let mut latest_repo = repo.clone();
        latest_repo.last_scanned_at = Some("2026-01-01T00:01:00Z".to_string());
        let mut latest_summary = scan_summary(2);
        latest_summary.skipped_entries.push(scan_issue(
            "F:/repo/latest-empty",
            crate::models::repository::RepositoryScanIssueCode::NoImages,
        ));
        latest_summary.failed_entries.push(scan_issue(
            "F:/repo/read-failed",
            crate::models::repository::RepositoryScanIssueCode::ReadFailed,
        ));
        latest_summary
            .duplicate_books
            .push(crate::models::repository::RepositoryDuplicateBook {
                path: "F:/repo/duplicate".to_string(),
                duplicate_of: "F:/repo/original".to_string(),
                title: "Duplicate".to_string(),
            });
        temp.database
            .save_repository_scan_history(&latest_repo, &latest_summary)
            .unwrap();

        let summary = temp.database.metadata_health_summary().unwrap();

        assert_eq!(summary.missing_metadata.len(), 1);
        assert_eq!(summary.missing_metadata[0].book.path, "F:/repo/missing");
        assert_eq!(summary.missing_metadata[0].reasons.len(), 3);
        assert_eq!(summary.missing_covers.len(), 1);
        assert_eq!(summary.missing_covers[0].book.path, "F:/repo/missing");
        assert_eq!(summary.missing_covers[0].reasons.len(), 2);
        assert_eq!(
            summary
                .no_image_issues
                .iter()
                .map(|issue| issue.path.as_str())
                .collect::<Vec<_>>(),
            vec!["F:/repo/latest-empty"]
        );
        assert_eq!(summary.duplicate_issues.len(), 1);
        assert_eq!(summary.duplicate_issues[0].path, "F:/repo/duplicate");
        assert_eq!(summary.duplicate_issues[0].duplicate_of, "F:/repo/original");
    }

    #[test]
    fn cleanup_thumbnail_cache_only_removes_managed_thumbnail_files() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        let thumbnail_dir = thumbnail::thumbnail_dir_from_database_path(&temp.database.path);
        fs::create_dir_all(&thumbnail_dir).unwrap();
        let managed_path =
            thumbnail::thumbnail_path(&thumbnail_dir, &uuid::Uuid::new_v4().to_string());
        let unmanaged_path = thumbnail_dir.join("manual-note.txt");
        fs::write(&managed_path, b"managed").unwrap();
        fs::write(&unmanaged_path, b"manual").unwrap();

        let mut managed_book = book(&repo.id, "F:/repo/managed", "Managed");
        managed_book.thumbnail_path = Some(managed_path.to_string_lossy().to_string());
        let mut external_book = book(&repo.id, "F:/repo/external", "External");
        external_book.thumbnail_path = Some("F:/external/thumb.jpg".to_string());
        temp.database
            .upsert_scan(&repo, &[managed_book.clone(), external_book.clone()])
            .unwrap();

        let result = temp.database.cleanup_thumbnail_cache().unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.removed_files, 1);
        assert!(!result.source_files_affected);
        assert!(!managed_path.exists());
        assert!(unmanaged_path.exists());

        let managed_after = temp.database.get_book(&managed_book.id).unwrap();
        let external_after = temp.database.get_book(&external_book.id).unwrap();
        assert_eq!(managed_after.thumbnail_path, None);
        assert_eq!(
            external_after.thumbnail_path.as_deref(),
            Some("F:/external/thumb.jpg")
        );
    }

    #[test]
    fn rebuild_missing_thumbnails_generates_managed_thumbnail_files() {
        let temp = TempDatabase::new();
        let temp_dir =
            std::env::temp_dir().join(format!("inkreader-thumb-source-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let cover_path = temp_dir.join("cover.jpg");
        let image = image::RgbImage::from_pixel(4, 4, image::Rgb([240, 240, 240]));
        image.save(&cover_path).unwrap();

        let repo = repository("F:/repo", "repo-1");
        let mut source_book = book(&repo.id, &temp_dir.to_string_lossy(), "Thumb Source");
        source_book.cover_path = Some(cover_path.to_string_lossy().to_string());
        temp.database
            .upsert_scan(&repo, &[source_book.clone()])
            .unwrap();

        let result = temp.database.rebuild_missing_thumbnails().unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.rebuilt_thumbnails, 1);
        assert!(result.failed.is_empty());
        assert!(!result.source_files_affected);

        let updated = temp.database.get_book(&source_book.id).unwrap();
        let thumbnail_path = updated.thumbnail_path.unwrap();
        assert!(Path::new(&thumbnail_path).is_file());
        assert!(is_managed_thumbnail_path(
            &thumbnail::thumbnail_dir_from_database_path(&temp.database.path),
            &thumbnail_path
        ));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn renamed_title_survives_rescan_and_reset_uses_latest_scanned_title() {
        let temp = TempDatabase::new();
        let book_path = "F:/repo/book-a";
        let repo = repository("F:/repo", "repo-1");
        temp.database
            .upsert_scan(&repo, &[book(&repo.id, book_path, "原始标题")])
            .unwrap();

        let renamed = temp
            .database
            .rename_book_title(book_path, "自定义标题")
            .unwrap();
        assert_eq!(renamed.title, "自定义标题");
        assert_eq!(renamed.scanned_title, "原始标题");
        assert_eq!(renamed.title_override.as_deref(), Some("自定义标题"));

        let rescanned_repo = repository("F:/repo", "repo-2");
        temp.database
            .upsert_scan(
                &rescanned_repo,
                &[book(&rescanned_repo.id, book_path, "新默认标题")],
            )
            .unwrap();
        let after_rescan = list_first_book(&temp.database);
        assert_eq!(after_rescan.title, "自定义标题");
        assert_eq!(after_rescan.scanned_title, "新默认标题");
        assert_eq!(after_rescan.title_override.as_deref(), Some("自定义标题"));

        let reset = temp.database.reset_book_title(book_path).unwrap();
        assert_eq!(reset.title, "新默认标题");
        assert_eq!(reset.scanned_title, "新默认标题");
        assert_eq!(reset.title_override, None);
    }

    #[test]
    fn migration_backfills_scanned_title_from_existing_title() {
        let path = std::env::temp_dir().join(format!(
            "inkreader-migration-test-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch(
                "CREATE TABLE books (
                  id TEXT PRIMARY KEY,
                  repository_id TEXT NOT NULL,
                  source_id TEXT,
                  title TEXT NOT NULL,
                  path TEXT NOT NULL UNIQUE,
                  kind TEXT NOT NULL,
                  metadata_path TEXT,
                  cover_path TEXT,
                  description TEXT,
                  authors_json TEXT NOT NULL DEFAULT '[]',
                  tags_json TEXT NOT NULL DEFAULT '[]',
                  chapter_count INTEGER NOT NULL DEFAULT 0,
                  total_pages INTEGER NOT NULL DEFAULT 0,
                  last_chapter_id TEXT,
                  last_page INTEGER NOT NULL DEFAULT 0,
                  last_read_at TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                INSERT INTO books (id, repository_id, title, path, kind, created_at, updated_at)
                VALUES ('book-1', 'repo-1', '旧标题', 'F:/repo/book-a', 'folder', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');",
            ).unwrap();
        }

        let database = Database { path: path.clone() };
        database.migrate().unwrap();
        let connection = Connection::open(&path).unwrap();
        let scanned_title: String = connection
            .query_row(
                "SELECT scanned_title FROM books WHERE id = 'book-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(scanned_title, "旧标题");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn migration_skips_search_index_backfill_after_marker_exists() {
        let temp = TempDatabase::new();
        let repo = repository("F:/repo", "repo-1");
        temp.database
            .upsert_scan(&repo, &[book(&repo.id, "F:/repo/book-a", "Title")])
            .unwrap();

        temp.database.migrate().unwrap();
        {
            let connection = temp.database.connect().unwrap();
            connection
                .execute(
                    "UPDATE books SET search_text_normalized = 'sentinel' WHERE path = ?1",
                    params!["F:/repo/book-a"],
                )
                .unwrap();
        }

        temp.database.migrate().unwrap();
        let connection = temp.database.connect().unwrap();
        let search_text: String = connection
            .query_row(
                "SELECT search_text_normalized FROM books WHERE path = ?1",
                params!["F:/repo/book-a"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(search_text, "sentinel");
    }
}

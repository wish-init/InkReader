use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, params_from_iter, types::Value, Connection};
use tauri::Manager;

use crate::{
    errors::{AppError, AppResult},
    models::{
        book::UpdateBookMetadataRequest,
        book::{Book, BookListRequest, BookListResponse, BookSummary, BookThumbnail},
        chapter::Chapter,
        favorite::FavoriteCollection,
        history::ReadingHistoryRecord,
        page::Page,
        repository::Repository,
    },
    thumbnail,
};

const BOOK_TAGS_BACKFILL_SETTING_KEY: &str = "migration:book_tags_backfilled";

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
              PRIMARY KEY (book_id, tag),
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
            ",
        )?;
        add_column_if_missing(&connection, "books", "last_read_at", "TEXT")?;
        add_column_if_missing(&connection, "books", "scanned_title", "TEXT")?;
        add_column_if_missing(&connection, "books", "title_override", "TEXT")?;
        add_column_if_missing(&connection, "books", "scan_signature", "TEXT")?;
        add_column_if_missing(&connection, "books", "thumbnail_path", "TEXT")?;
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
            CREATE INDEX IF NOT EXISTS idx_books_total_pages ON books(total_pages, id);
            CREATE INDEX IF NOT EXISTS idx_books_repository_total_pages ON books(repository_id, total_pages, id);
            CREATE INDEX IF NOT EXISTS idx_books_title ON books(title COLLATE NOCASE, id);
            CREATE INDEX IF NOT EXISTS idx_chapters_book_order ON chapters(book_id, order_index ASC, title ASC);
            CREATE INDEX IF NOT EXISTS idx_pages_chapter_page_index ON pages(chapter_id, page_index ASC);
            CREATE INDEX IF NOT EXISTS idx_book_tags_tag_book_id ON book_tags(tag, book_id);
            CREATE INDEX IF NOT EXISTS idx_book_tags_book_id ON book_tags(book_id);
            ",
        )?;
        backfill_book_tags(&connection)?;
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

    pub fn list_favorite_collections(&self) -> AppResult<Vec<FavoriteCollection>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT favorite_collections.id, favorite_collections.name,
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
        connection.execute(
            "UPDATE books
             SET title = ?1, title_override = ?1, updated_at = datetime('now')
             WHERE path = ?2",
            params![trimmed_title, book_path],
        )?;
        self.get_book_by_path(book_path)
    }

    pub fn reset_book_title(&self, book_path: &str) -> AppResult<Book> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE books
             SET title = COALESCE(NULLIF(scanned_title, ''), title),
                 title_override = NULL,
                 updated_at = datetime('now')
             WHERE path = ?1",
            params![book_path],
        )?;
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
        let authors = request
            .authors
            .iter()
            .map(|author| author.trim())
            .filter(|author| !author.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
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
        transaction.execute(
            "DELETE FROM book_tags WHERE book_id = ?1",
            params![&book_id],
        )?;
        for tag in tags {
            transaction.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag) VALUES (?1, ?2)",
                params![&book_id, &tag],
            )?;
        }
        transaction.commit()?;
        self.get_book_by_path(&request.book_path)
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
            "DELETE FROM reading_history WHERE book_path = ?1",
            params![&book_path],
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

    pub fn get_reader_settings(&self) -> AppResult<crate::models::settings::ReaderSettings> {
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
            None => Ok(crate::models::settings::ReaderSettings::default()),
        }
    }

    pub fn save_reader_settings(
        &self,
        settings: &crate::models::settings::ReaderSettings,
    ) -> AppResult<()> {
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

    pub fn get_library_view_settings(
        &self,
    ) -> AppResult<crate::models::settings::LibraryViewSettings> {
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
            None => Ok(crate::models::settings::LibraryViewSettings::default()),
        }
    }

    pub fn save_library_view_settings(
        &self,
        settings: &crate::models::settings::LibraryViewSettings,
    ) -> AppResult<()> {
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
    let authors_json = serde_json::to_string(&book.authors)?;
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

    transaction.execute(
        "INSERT INTO books (
          id, repository_id, source_id, title, scanned_title, title_override, path, kind, metadata_path, cover_path, thumbnail_path,
          description, authors_json, tags_json, chapter_count, total_pages,
          last_chapter_id, last_page, last_read_at, created_at, updated_at, scan_signature
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
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
        ],
    )?;

    for tag in normalized_tags {
        transaction.execute(
            "INSERT OR IGNORE INTO book_tags (book_id, tag) VALUES (?1, ?2)",
            params![&book.id, &tag],
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
                "INSERT OR IGNORE INTO book_tags (book_id, tag) VALUES (?1, ?2)",
                params![&book_id, &tag],
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
            thumbnail_path, description, authors_json, tags_json, chapter_count, total_pages,
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
        filters.push("(LOWER(books.title) LIKE ? ESCAPE '\\' OR LOWER(books.authors_json) LIKE ? ESCAPE '\\' OR LOWER(books.tags_json) LIKE ? ESCAPE '\\')".to_string());
        let pattern = format!("%{}%", escape_like_pattern(&query.to_lowercase()));
        values.push(Value::Text(pattern.clone()));
        values.push(Value::Text(pattern.clone()));
        values.push(Value::Text(pattern));
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
        filters.push("EXISTS (SELECT 1 FROM book_tags WHERE book_tags.book_id = books.id AND book_tags.tag = ?)".to_string());
        values.push(Value::Text(tag));
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
        "createdAt" if direction == "ASC" => "ORDER BY books.created_at ASC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
        _ => "ORDER BY books.created_at DESC, books.title COLLATE NOCASE ASC, books.path COLLATE NOCASE ASC, books.id ASC",
    }
}

fn map_book_summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BookSummary> {
    let authors_json: String = row.get(12)?;
    let tags_json: String = row.get(13)?;
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
        description: row.get(11)?,
        authors: serde_json::from_str(&authors_json).unwrap_or_default(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        chapter_count: row.get::<_, i64>(14)? as usize,
        total_pages: row.get::<_, i64>(15)? as usize,
        last_chapter_id: row.get(16)?,
        last_page: row.get::<_, i64>(17)? as usize,
        is_favorite: row.get::<_, i64>(18)? != 0,
        is_read_complete: row.get::<_, i64>(19)? != 0,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
        last_read_at: row.get(22)?,
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
        book_count: row.get::<_, i64>(2)? as usize,
        is_default: row.get::<_, i64>(3)? != 0,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDatabase {
        database: Database,
    }

    impl TempDatabase {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "inkreader-db-test-{}.sqlite3",
                uuid::Uuid::new_v4()
            ));
            let database = Database { path };
            database.migrate().unwrap();
            Self { database }
        }
    }

    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.database.path);
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

    fn book_list_request() -> BookListRequest {
        BookListRequest {
            repository_id: None,
            collection_id: None,
            query: None,
            tag: None,
            tags: None,
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
}

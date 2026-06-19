use tauri::State;

use crate::{
    errors::AppResult,
    models::{
        book::{
            Book, BookAggregationItem, BookListRequest, BookListResponse, BookThumbnail,
            UpdateBookMetadataRequest,
        },
        chapter::Chapter,
        favorite::FavoriteCollection,
    },
    AppState,
};

#[tauri::command]
pub fn list_books(
    state: State<'_, AppState>,
    request: BookListRequest,
) -> AppResult<BookListResponse> {
    state.database.list_books(request)
}

#[tauri::command]
pub fn ensure_book_thumbnails(
    state: State<'_, AppState>,
    book_ids: Vec<String>,
) -> AppResult<Vec<BookThumbnail>> {
    state.database.ensure_book_thumbnails(book_ids)
}

#[tauri::command]
pub fn list_favorite_books(
    state: State<'_, AppState>,
    request: BookListRequest,
) -> AppResult<BookListResponse> {
    state.database.list_favorite_books(request)
}

#[tauri::command]
pub fn list_book_tags(
    state: State<'_, AppState>,
    repository_id: Option<String>,
) -> AppResult<Vec<String>> {
    state.database.list_book_tags(repository_id)
}

#[tauri::command]
pub fn list_book_authors(
    state: State<'_, AppState>,
    repository_id: Option<String>,
) -> AppResult<Vec<String>> {
    state.database.list_book_authors(repository_id)
}

#[tauri::command]
pub fn list_book_tag_aggregations(
    state: State<'_, AppState>,
    query: Option<String>,
) -> AppResult<Vec<BookAggregationItem>> {
    state.database.list_book_tag_aggregations(query)
}

#[tauri::command]
pub fn list_book_author_aggregations(
    state: State<'_, AppState>,
    query: Option<String>,
) -> AppResult<Vec<BookAggregationItem>> {
    state.database.list_book_author_aggregations(query)
}

#[tauri::command]
pub fn list_favorite_collections(state: State<'_, AppState>) -> AppResult<Vec<FavoriteCollection>> {
    state.database.list_favorite_collections()
}

#[tauri::command]
pub fn create_favorite_collection(
    state: State<'_, AppState>,
    name: String,
) -> AppResult<FavoriteCollection> {
    state.database.create_favorite_collection(&name)
}

#[tauri::command]
pub fn rename_favorite_collection(
    state: State<'_, AppState>,
    collection_id: String,
    name: String,
) -> AppResult<FavoriteCollection> {
    state
        .database
        .rename_favorite_collection(&collection_id, &name)
}

#[tauri::command]
pub fn update_favorite_collection_metadata(
    state: State<'_, AppState>,
    collection_id: String,
    cover_path: Option<String>,
    description: Option<String>,
) -> AppResult<FavoriteCollection> {
    state.database.update_favorite_collection_metadata(
        &collection_id,
        cover_path.as_deref(),
        description.as_deref(),
    )
}

#[tauri::command]
pub fn delete_favorite_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> AppResult<()> {
    state.database.delete_favorite_collection(&collection_id)
}

#[tauri::command]
pub fn add_book_to_favorite_collection(
    state: State<'_, AppState>,
    book_path: String,
    collection_id: String,
) -> AppResult<()> {
    state
        .database
        .add_book_to_favorite_collection(&book_path, &collection_id)
}

#[tauri::command]
pub fn add_books_to_favorite_collection(
    state: State<'_, AppState>,
    book_paths: Vec<String>,
    collection_id: String,
) -> AppResult<()> {
    state
        .database
        .add_books_to_favorite_collection(&book_paths, &collection_id)
}

#[tauri::command]
pub fn remove_book_from_favorite_collection(
    state: State<'_, AppState>,
    book_path: String,
    collection_id: String,
) -> AppResult<()> {
    state
        .database
        .remove_book_from_favorite_collection(&book_path, &collection_id)
}

#[tauri::command]
pub fn remove_books_from_favorite_collection(
    state: State<'_, AppState>,
    book_paths: Vec<String>,
    collection_id: String,
) -> AppResult<()> {
    state
        .database
        .remove_books_from_favorite_collection(&book_paths, &collection_id)
}

#[tauri::command]
pub fn move_books_between_favorite_collections(
    state: State<'_, AppState>,
    book_paths: Vec<String>,
    source_collection_id: String,
    target_collection_id: String,
) -> AppResult<()> {
    state.database.move_books_between_favorite_collections(
        &book_paths,
        &source_collection_id,
        &target_collection_id,
    )
}

#[tauri::command]
pub fn remove_books_from_all_favorite_collections(
    state: State<'_, AppState>,
    book_paths: Vec<String>,
) -> AppResult<()> {
    state
        .database
        .remove_books_from_all_favorite_collections(&book_paths)
}

#[tauri::command]
pub fn list_book_favorite_collections(
    state: State<'_, AppState>,
    book_path: String,
) -> AppResult<Vec<FavoriteCollection>> {
    state.database.list_book_favorite_collections(&book_path)
}

#[tauri::command]
pub fn set_book_favorite(
    state: State<'_, AppState>,
    book_path: String,
    favorite: bool,
) -> AppResult<()> {
    state.database.set_book_favorite(&book_path, favorite)
}

#[tauri::command]
pub fn rename_book_title(
    state: State<'_, AppState>,
    book_path: String,
    title: String,
) -> AppResult<Book> {
    state.database.rename_book_title(&book_path, &title)
}

#[tauri::command]
pub fn reset_book_title(state: State<'_, AppState>, book_path: String) -> AppResult<Book> {
    state.database.reset_book_title(&book_path)
}

#[tauri::command]
pub fn update_book_metadata(
    state: State<'_, AppState>,
    request: UpdateBookMetadataRequest,
) -> AppResult<Book> {
    state.database.update_book_metadata(request)
}

#[tauri::command]
pub fn update_book_authors(
    state: State<'_, AppState>,
    book_path: String,
    authors: Vec<String>,
) -> AppResult<Book> {
    state.database.update_book_authors(&book_path, authors)
}

#[tauri::command]
pub fn update_book_tags(
    state: State<'_, AppState>,
    book_path: String,
    tags: Vec<String>,
) -> AppResult<Book> {
    state.database.update_book_tags(&book_path, tags)
}

#[tauri::command]
pub fn get_book(state: State<'_, AppState>, book_id: String) -> AppResult<Book> {
    state.database.get_book(&book_id)
}

#[tauri::command]
pub fn list_book_chapters(state: State<'_, AppState>, book_id: String) -> AppResult<Vec<Chapter>> {
    state.database.list_chapters(&book_id)
}

use tauri::State;

use crate::{
    errors::AppResult,
    models::bookmark::{Bookmark, CreateBookmarkRequest},
    AppState,
};

#[tauri::command]
pub fn list_bookmarks(state: State<'_, AppState>, book_id: String) -> AppResult<Vec<Bookmark>> {
    state.database.list_bookmarks(&book_id)
}

#[tauri::command]
pub fn create_bookmark(
    state: State<'_, AppState>,
    request: CreateBookmarkRequest,
) -> AppResult<Bookmark> {
    state.database.create_bookmark(&request)
}

#[tauri::command]
pub fn delete_bookmark(state: State<'_, AppState>, bookmark_id: String) -> AppResult<()> {
    state.database.delete_bookmark(&bookmark_id)
}

#[tauri::command]
pub fn is_page_bookmarked(
    state: State<'_, AppState>,
    book_id: String,
    chapter_id: String,
    page_index: usize,
) -> AppResult<bool> {
    state
        .database
        .is_page_bookmarked(&book_id, &chapter_id, page_index)
}

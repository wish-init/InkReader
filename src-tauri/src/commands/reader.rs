use tauri::State;

use crate::{
    errors::AppResult,
    models::{book::Book, history::ReadingHistoryRecord, page::Page},
    AppState,
};

#[tauri::command]
pub fn list_chapter_pages(state: State<'_, AppState>, chapter_id: String) -> AppResult<Vec<Page>> {
    state.database.list_pages(&chapter_id)
}

#[tauri::command]
pub fn update_book_progress(
    state: State<'_, AppState>,
    book_id: String,
    chapter_id: String,
    page: usize,
) -> AppResult<()> {
    state.database.update_progress(&book_id, &chapter_id, page)
}

#[tauri::command]
pub fn mark_book_read(state: State<'_, AppState>, book_id: String) -> AppResult<Book> {
    state.database.mark_book_read(&book_id)
}

#[tauri::command]
pub fn mark_book_unread(state: State<'_, AppState>, book_id: String) -> AppResult<Book> {
    state.database.mark_book_unread(&book_id)
}

#[tauri::command]
pub fn list_reading_history(state: State<'_, AppState>) -> AppResult<Vec<ReadingHistoryRecord>> {
    state.database.list_reading_history()
}

#[tauri::command]
pub fn list_reading_history_by_book(
    state: State<'_, AppState>,
) -> AppResult<Vec<ReadingHistoryRecord>> {
    state.database.list_reading_history_by_book()
}

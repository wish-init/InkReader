use tauri::State;

use crate::{
    errors::AppResult,
    models::settings::{LibraryViewSettings, ReaderSettings},
    AppState,
};

#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}

#[tauri::command]
pub fn get_reader_settings(state: State<'_, AppState>) -> AppResult<ReaderSettings> {
    state.database.get_reader_settings()
}

#[tauri::command]
pub fn save_reader_settings(state: State<'_, AppState>, settings: ReaderSettings) -> AppResult<()> {
    state.database.save_reader_settings(&settings)
}

#[tauri::command]
pub fn get_library_view_settings(state: State<'_, AppState>) -> AppResult<LibraryViewSettings> {
    state.database.get_library_view_settings()
}

#[tauri::command]
pub fn save_library_view_settings(
    state: State<'_, AppState>,
    settings: LibraryViewSettings,
) -> AppResult<()> {
    state.database.save_library_view_settings(&settings)
}

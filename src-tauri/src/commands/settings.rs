use tauri::State;

use crate::{
    errors::AppResult,
    models::settings::{
        EffectiveReaderSettingsState, LibraryViewSettings, ReaderSettings, SettingsExport,
        SettingsRestoreScope,
    },
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
pub fn get_book_reader_settings(
    state: State<'_, AppState>,
    book_id: String,
) -> AppResult<Option<ReaderSettings>> {
    state.database.get_book_reader_settings(&book_id)
}

#[tauri::command]
pub fn get_effective_reader_settings(
    state: State<'_, AppState>,
    book_id: String,
) -> AppResult<ReaderSettings> {
    state.database.get_effective_reader_settings(&book_id)
}

#[tauri::command]
pub fn get_effective_reader_settings_state(
    state: State<'_, AppState>,
    book_id: String,
) -> AppResult<EffectiveReaderSettingsState> {
    state.database.get_effective_reader_settings_state(&book_id)
}

#[tauri::command]
pub fn save_book_reader_settings(
    state: State<'_, AppState>,
    book_id: String,
    settings: ReaderSettings,
) -> AppResult<()> {
    state
        .database
        .save_book_reader_settings(&book_id, &settings)
}

#[tauri::command]
pub fn clear_book_reader_settings(state: State<'_, AppState>, book_id: String) -> AppResult<()> {
    state.database.clear_book_reader_settings(&book_id)
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

#[tauri::command]
pub fn export_settings(state: State<'_, AppState>) -> AppResult<SettingsExport> {
    state.database.export_settings()
}

#[tauri::command]
pub fn import_settings_export(
    state: State<'_, AppState>,
    settings_json: String,
) -> AppResult<SettingsExport> {
    let settings_export = parse_settings_export_json(&settings_json)?;
    state.database.import_settings_export(settings_export)
}

#[tauri::command]
pub fn restore_default_settings(
    state: State<'_, AppState>,
    scope: SettingsRestoreScope,
) -> AppResult<SettingsExport> {
    state.database.restore_default_settings(scope)
}

fn parse_settings_export_json(settings_json: &str) -> AppResult<SettingsExport> {
    Ok(serde_json::from_str(settings_json)?)
}

#[cfg(test)]
mod tests {
    use super::parse_settings_export_json;

    #[test]
    fn parse_settings_export_json_rejects_malformed_json() {
        assert!(parse_settings_export_json("{").is_err());
    }
}

use tauri::State;

use crate::{
    errors::AppResult,
    models::backup::{DatabaseBackupResult, DatabaseRestoreResult},
    AppState,
};

#[tauri::command]
pub fn create_database_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> AppResult<DatabaseBackupResult> {
    state.database.create_database_backup(&backup_path)
}

#[tauri::command]
pub fn restore_database_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> AppResult<DatabaseRestoreResult> {
    state.database.restore_database_backup(&backup_path)
}

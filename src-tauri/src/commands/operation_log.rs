use tauri::State;

use crate::{errors::AppResult, models::operation_log::OperationLogRecord, AppState};

#[tauri::command]
pub fn list_operation_logs(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> AppResult<Vec<OperationLogRecord>> {
    state.database.list_operation_logs(limit)
}

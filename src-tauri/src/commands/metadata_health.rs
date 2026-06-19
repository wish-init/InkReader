use tauri::State;

use crate::{errors::AppResult, models::metadata_health::MetadataHealthSummary, AppState};

#[tauri::command]
pub fn list_metadata_health(state: State<'_, AppState>) -> AppResult<MetadataHealthSummary> {
    state.database.metadata_health_summary()
}

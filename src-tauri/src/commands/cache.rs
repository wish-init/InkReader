use tauri::State;

use crate::{
    errors::AppResult,
    models::cache::{CacheMaintenanceResult, CacheMaintenanceSummary},
    AppState,
};

#[tauri::command]
pub fn get_cache_maintenance_summary(
    state: State<'_, AppState>,
) -> AppResult<CacheMaintenanceSummary> {
    state.database.cache_maintenance_summary()
}

#[tauri::command]
pub fn cleanup_thumbnail_cache(state: State<'_, AppState>) -> AppResult<CacheMaintenanceResult> {
    state.database.cleanup_thumbnail_cache()
}

#[tauri::command]
pub fn rebuild_missing_thumbnails(state: State<'_, AppState>) -> AppResult<CacheMaintenanceResult> {
    state.database.rebuild_missing_thumbnails()
}

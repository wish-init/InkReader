use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::{
    errors::AppResult,
    models::repository::{Repository, RepositoryScanProgress, RepositoryScanResult},
    scanner::repository as repository_scanner,
    AppState,
};

#[tauri::command]
pub fn scan_repository(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<RepositoryScanResult> {
    let existing_repository_id = state.database.existing_repository_id_by_path(&path)?;
    let existing_signatures = match existing_repository_id.as_deref() {
        Some(repository_id) => state.database.book_scan_signatures(repository_id)?,
        None => Default::default(),
    };
    let result = repository_scanner::scan_repository_incremental(
        PathBuf::from(path),
        existing_repository_id,
        &existing_signatures,
        |progress| emit_scan_progress(&app, progress),
    )?;
    state.database.upsert_incremental_scan(
        &result.repository,
        &result.books,
        &result.current_book_paths,
    )?;
    Ok(result)
}

#[tauri::command]
pub fn list_repositories(state: State<'_, AppState>) -> AppResult<Vec<Repository>> {
    state.database.list_repositories()
}

#[tauri::command]
pub fn auto_scan_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<RepositoryScanResult>> {
    let repositories = state.database.list_repositories()?;
    let mut results = Vec::new();

    for repository in repositories {
        let existing_signatures = state.database.book_scan_signatures(&repository.id)?;
        let result = repository_scanner::scan_repository_incremental(
            PathBuf::from(&repository.path),
            Some(repository.id),
            &existing_signatures,
            |progress| emit_scan_progress(&app, progress),
        )?;
        state.database.upsert_incremental_scan(
            &result.repository,
            &result.books,
            &result.current_book_paths,
        )?;
        results.push(result);
    }

    Ok(results)
}

#[tauri::command]
pub fn remove_repository(state: State<'_, AppState>, repository_id: String) -> AppResult<()> {
    state.database.remove_repository(&repository_id)
}

fn emit_scan_progress(app: &AppHandle, progress: RepositoryScanProgress) {
    let _ = app.emit("repository-scan-progress", progress);
}

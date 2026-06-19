use std::{path::PathBuf, sync::Arc};

use tauri::{AppHandle, Emitter, State};

use crate::{
    db::Database,
    errors::{AppError, AppResult},
    models::repository::{
        Repository, RepositoryScanHistoryRecord, RepositoryScanProgress, RepositoryScanResult,
    },
    scanner::repository as repository_scanner,
    AppState,
};

#[tauri::command]
pub async fn scan_repository(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<RepositoryScanResult> {
    let database = state.database.clone();
    tauri::async_runtime::spawn_blocking(move || scan_repository_blocking(app, database, path))
        .await
        .map_err(|error| AppError::Io(format!("扫描任务执行失败: {error}")))?
}

fn scan_repository_blocking(
    app: AppHandle,
    database: Arc<Database>,
    path: String,
) -> AppResult<RepositoryScanResult> {
    let existing_repository_id = database.existing_repository_id_by_path(&path)?;
    let existing_signatures = match existing_repository_id.as_deref() {
        Some(repository_id) => database.book_scan_signatures(repository_id)?,
        None => Default::default(),
    };
    let mut result = repository_scanner::scan_repository_incremental(
        PathBuf::from(path),
        existing_repository_id,
        &existing_signatures,
        |progress| emit_scan_progress(&app, progress),
    )?;

    persist_scan_result(&app, &database, &mut result)?;
    Ok(result)
}

fn persist_scan_result(
    app: &AppHandle,
    database: &Database,
    result: &mut RepositoryScanResult,
) -> AppResult<()> {
    emit_scan_phase(app, result, "persist", "正在保存扫描结果");
    database.upsert_incremental_scan(
        &result.repository,
        &result.books,
        &result.current_book_paths,
    )?;
    database.save_repository_scan_history(&result.repository, &result.summary)?;
    emit_scan_phase(app, result, "finish", "扫描完成");

    result.books.clear();
    result.current_book_paths.clear();
    Ok(())
}

#[tauri::command]
pub fn list_repositories(state: State<'_, AppState>) -> AppResult<Vec<Repository>> {
    state.database.list_repositories()
}

#[tauri::command]
pub async fn auto_scan_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<RepositoryScanResult>> {
    let database = state.database.clone();
    tauri::async_runtime::spawn_blocking(move || auto_scan_repositories_blocking(app, database))
        .await
        .map_err(|error| AppError::Io(format!("自动扫描任务执行失败: {error}")))?
}

fn auto_scan_repositories_blocking(
    app: AppHandle,
    database: Arc<Database>,
) -> AppResult<Vec<RepositoryScanResult>> {
    let repositories = database.list_repositories()?;
    let mut results = Vec::new();

    for repository in repositories {
        let existing_signatures = database.book_scan_signatures(&repository.id)?;
        let mut result = repository_scanner::scan_repository_incremental(
            PathBuf::from(&repository.path),
            Some(repository.id),
            &existing_signatures,
            |progress| emit_scan_progress(&app, progress),
        )?;
        persist_scan_result(&app, &database, &mut result)?;
        results.push(result);
    }

    Ok(results)
}

#[tauri::command]
pub fn list_repository_scan_history(
    state: State<'_, AppState>,
) -> AppResult<Vec<RepositoryScanHistoryRecord>> {
    state.database.list_repository_scan_history()
}

#[tauri::command]
pub fn remove_repository(state: State<'_, AppState>, repository_id: String) -> AppResult<()> {
    state.database.remove_repository(&repository_id)
}

fn emit_scan_progress(app: &AppHandle, progress: RepositoryScanProgress) {
    let _ = app.emit("repository-scan-progress", progress);
}

fn emit_scan_phase(app: &AppHandle, result: &RepositoryScanResult, phase: &str, message: &str) {
    emit_scan_progress(
        app,
        RepositoryScanProgress {
            scan_id: result.scan_id.clone(),
            repository_path: result.repository.path.clone(),
            current: result.summary.total_entries,
            total: result.summary.total_entries,
            phase: phase.to_string(),
            message: message.to_string(),
        },
    );
}

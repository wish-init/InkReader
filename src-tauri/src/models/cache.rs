use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMaintenanceSummary {
    pub thumbnail_cache_dir: String,
    pub thumbnail_files: usize,
    pub thumbnail_bytes: u64,
    pub books_with_thumbnails: usize,
    pub missing_thumbnails: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMaintenanceFailure {
    pub path: String,
    pub title: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMaintenanceResult {
    pub operation: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: Vec<CacheMaintenanceFailure>,
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub rebuilt_thumbnails: usize,
    pub source_files_affected: bool,
}

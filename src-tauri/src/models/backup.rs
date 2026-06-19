use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseBackupResult {
    pub backup_path: String,
    pub created_at: String,
    pub bytes: u64,
    pub source_files_affected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRestoreResult {
    pub restored_from: String,
    pub restored_at: String,
    pub rollback_path: String,
    pub source_files_affected: bool,
}

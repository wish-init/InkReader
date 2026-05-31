use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
    pub book_count: usize,
    pub last_scanned_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryScanResult {
    pub repository: Repository,
    pub books: Vec<crate::models::book::Book>,
    pub summary: RepositoryScanSummary,
    #[serde(skip_serializing)]
    pub current_book_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryScanSummary {
    pub total_entries: usize,
    pub scanned_books: usize,
    pub unchanged_books: usize,
    pub skipped_entries: Vec<RepositoryScanIssue>,
    pub failed_entries: Vec<RepositoryScanIssue>,
    pub duplicate_books: Vec<RepositoryDuplicateBook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryScanIssue {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryDuplicateBook {
    pub path: String,
    pub duplicate_of: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryScanProgress {
    pub scan_id: String,
    pub repository_path: String,
    pub current: usize,
    pub total: usize,
    pub phase: String,
    pub message: String,
}

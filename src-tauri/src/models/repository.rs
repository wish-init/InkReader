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
    #[serde(skip_serializing)]
    pub scan_id: String,
    #[serde(skip_serializing)]
    pub books: Vec<crate::models::book::Book>,
    pub summary: RepositoryScanSummary,
    #[serde(skip_serializing)]
    pub current_book_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryScanHistoryRecord {
    pub id: String,
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub scanned_at: String,
    pub summary: RepositoryScanSummary,
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
    pub code: RepositoryScanIssueCode,
    pub severity: RepositoryScanIssueSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepositoryScanIssueCode {
    UnchangedBook,
    NoImages,
    ReadFailed,
    DuplicateBook,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepositoryScanIssueSeverity {
    Info,
    Warning,
    Error,
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

#[cfg(test)]
mod tests {
    use super::{Repository, RepositoryScanResult, RepositoryScanSummary};

    #[test]
    fn scan_result_serialization_omits_internal_scan_data() {
        let result = RepositoryScanResult {
            repository: Repository {
                id: "repo-1".to_string(),
                name: "Repository".to_string(),
                path: "F:/repo".to_string(),
                book_count: 1,
                last_scanned_at: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            },
            scan_id: "scan-1".to_string(),
            books: Vec::new(),
            summary: RepositoryScanSummary::default(),
            current_book_paths: vec!["F:/repo/book".to_string()],
        };

        let serialized = serde_json::to_value(result).unwrap();
        let object = serialized.as_object().unwrap();

        assert!(object.contains_key("repository"));
        assert!(object.contains_key("summary"));
        assert!(!object.contains_key("books"));
        assert!(!object.contains_key("scanId"));
        assert!(!object.contains_key("currentBookPaths"));
    }
}

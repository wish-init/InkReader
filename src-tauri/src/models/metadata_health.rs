use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataHealthBookIssue {
    pub book: crate::models::book::BookSummary,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataHealthScanIssue {
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub scanned_at: String,
    pub path: String,
    pub reason: String,
    pub code: crate::models::repository::RepositoryScanIssueCode,
    pub severity: crate::models::repository::RepositoryScanIssueSeverity,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataHealthDuplicateIssue {
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub scanned_at: String,
    pub path: String,
    pub duplicate_of: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataHealthSummary {
    pub missing_metadata: Vec<MetadataHealthBookIssue>,
    pub missing_covers: Vec<MetadataHealthBookIssue>,
    pub no_image_issues: Vec<MetadataHealthScanIssue>,
    pub duplicate_issues: Vec<MetadataHealthDuplicateIssue>,
}

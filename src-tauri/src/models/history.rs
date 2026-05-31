use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingHistoryRecord {
    pub id: String,
    pub book_id: String,
    pub book_title: String,
    pub book_path: String,
    pub book_kind: String,
    pub cover_path: Option<String>,
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    pub page: usize,
    pub read_at: String,
}

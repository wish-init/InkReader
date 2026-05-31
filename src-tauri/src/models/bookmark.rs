use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub book_id: String,
    pub chapter_id: String,
    pub page_index: usize,
    pub title: String,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookmarkRequest {
    pub book_id: String,
    pub chapter_id: String,
    pub page_index: usize,
    pub title: Option<String>,
    pub note: Option<String>,
}

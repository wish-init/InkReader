use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub book_id: String,
    pub source_chapter_id: Option<String>,
    pub title: String,
    pub path: String,
    pub order: i64,
    pub page_count: usize,
    pub pages: Vec<crate::models::page::Page>,
}

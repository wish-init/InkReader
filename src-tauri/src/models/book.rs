use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSummary {
    pub id: String,
    pub repository_id: String,
    pub source_id: Option<String>,
    pub title: String,
    pub scanned_title: String,
    pub title_override: Option<String>,
    pub path: String,
    pub kind: String,
    pub metadata_path: Option<String>,
    pub cover_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub published_at: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
    pub chapter_count: usize,
    pub total_pages: usize,
    pub last_chapter_id: Option<String>,
    pub last_page: usize,
    pub last_read_at: Option<String>,
    pub is_read_complete: bool,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookListRequest {
    pub repository_id: Option<String>,
    pub collection_id: Option<String>,
    pub query: Option<String>,
    pub author: Option<String>,
    pub authors: Option<Vec<String>>,
    pub tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub metadata_filters: Option<Vec<String>>,
    pub reading_status: Option<String>,
    pub favorite_status: Option<String>,
    pub sort_key: Option<String>,
    pub sort_direction: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookAggregationItem {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookListResponse {
    pub books: Vec<BookSummary>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookMetadataRequest {
    pub book_path: String,
    pub title: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookThumbnail {
    pub book_id: String,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub repository_id: String,
    pub source_id: Option<String>,
    pub title: String,
    pub scanned_title: String,
    pub title_override: Option<String>,
    pub path: String,
    pub kind: String,
    pub metadata_path: Option<String>,
    pub cover_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub published_at: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
    pub chapter_count: usize,
    pub total_pages: usize,
    pub last_chapter_id: Option<String>,
    pub last_page: usize,
    pub last_read_at: Option<String>,
    pub is_read_complete: bool,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing)]
    pub scan_signature: Option<String>,
    pub chapters: Vec<crate::models::chapter::Chapter>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteCollection {
    pub id: String,
    pub name: String,
    pub cover_path: Option<String>,
    pub description: Option<String>,
    pub book_count: usize,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

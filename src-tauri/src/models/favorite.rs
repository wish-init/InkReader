use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteCollection {
    pub id: String,
    pub name: String,
    pub book_count: usize,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

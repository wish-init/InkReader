use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub index: usize,
    pub name: String,
    pub path: String,
    pub uri: String,
}

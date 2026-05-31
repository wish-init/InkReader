use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComicMetadata {
    pub id: Option<serde_json::Value>,
    pub name: Option<String>,
    pub addtime: Option<serde_json::Value>,
    pub description: Option<String>,
    #[serde(default)]
    pub author: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub chapter_infos: Vec<ChapterInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub chapter_id: Option<serde_json::Value>,
    pub chapter_title: Option<String>,
    pub order: Option<i64>,
}

impl ComicMetadata {
    pub fn source_id(&self) -> Option<String> {
        value_to_string(self.id.as_ref())
    }
}

pub fn value_to_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

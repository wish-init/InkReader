use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReaderSettings {
    pub mode: String,
    pub fit: String,
    pub direction: String,
    pub background: String,
    pub space_scroll_ratio: f64,
    pub space_hold_speed_ratio: f64,
    pub brightness: f64,
    pub contrast: f64,
    pub page_animation: String,
    pub preload_cache_limit: usize,
}

impl Default for ReaderSettings {
    fn default() -> Self {
        Self {
            mode: "single".to_string(),
            fit: "height".to_string(),
            direction: "ltr".to_string(),
            background: "#111410".to_string(),
            space_scroll_ratio: 0.88,
            space_hold_speed_ratio: 2.5,
            brightness: 1.0,
            contrast: 1.0,
            page_animation: "none".to_string(),
            preload_cache_limit: 80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryViewSettings {
    pub layout: String,
    pub cover_size: String,
    pub show_authors: bool,
    pub show_tags: bool,
    pub tag_limit: usize,
}

impl Default for LibraryViewSettings {
    fn default() -> Self {
        Self {
            layout: "grid".to_string(),
            cover_size: "medium".to_string(),
            show_authors: true,
            show_tags: true,
            tag_limit: 4,
        }
    }
}

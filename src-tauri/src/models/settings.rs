use serde::{Deserialize, Serialize};

pub const SETTINGS_SCHEMA_VERSION: u16 = 1;
const READER_MODE_OPTIONS: &[&str] = &["single", "double", "scroll"];
const READER_FIT_OPTIONS: &[&str] = &["width", "height", "original"];
const READER_DIRECTION_OPTIONS: &[&str] = &["ltr", "rtl"];
const READER_PAGE_ANIMATION_OPTIONS: &[&str] = &["none", "slide", "fade"];
const LIBRARY_LAYOUT_OPTIONS: &[&str] = &["grid", "compact", "list"];
const LIBRARY_COVER_SIZE_OPTIONS: &[&str] = &["small", "medium", "large"];
const LIBRARY_TAG_LIMIT_OPTIONS: &[usize] = &[0, 2, 4, 8, 999];
const LIBRARY_TITLE_LINE_CLAMP_OPTIONS: &[usize] = &[1, 2, 3, 4];
const LIBRARY_TITLE_FONT_SIZE_OPTIONS: &[usize] = &[13, 15, 17, 19];

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
    pub auto_scroll_speed: usize,
    pub auto_scroll_start_delay: f64,
    pub auto_scroll_stop_on_manual_scroll: bool,
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
            auto_scroll_speed: 80,
            auto_scroll_start_delay: 0.0,
            auto_scroll_stop_on_manual_scroll: true,
        }
    }
}

impl ReaderSettings {
    pub fn validate_for_import(&self) -> Result<(), String> {
        validate_option("reader.mode", &self.mode, READER_MODE_OPTIONS)?;
        validate_option("reader.fit", &self.fit, READER_FIT_OPTIONS)?;
        validate_option(
            "reader.direction",
            &self.direction,
            READER_DIRECTION_OPTIONS,
        )?;
        validate_option(
            "reader.pageAnimation",
            &self.page_animation,
            READER_PAGE_ANIMATION_OPTIONS,
        )?;
        validate_number("reader.spaceScrollRatio", self.space_scroll_ratio, 0.1, 2.0)?;
        validate_number(
            "reader.spaceHoldSpeedRatio",
            self.space_hold_speed_ratio,
            0.5,
            10.0,
        )?;
        validate_number("reader.brightness", self.brightness, 0.2, 2.0)?;
        validate_number("reader.contrast", self.contrast, 0.2, 2.0)?;
        validate_usize("reader.preloadCacheLimit", self.preload_cache_limit, 0, 500)?;
        validate_usize("reader.autoScrollSpeed", self.auto_scroll_speed, 20, 400)?;
        validate_number(
            "reader.autoScrollStartDelay",
            self.auto_scroll_start_delay,
            0.0,
            5.0,
        )?;
        if self.background.trim().is_empty() {
            return Err("reader.background 不能为空".to_string());
        }
        Ok(())
    }
}

fn default_library_layout() -> String {
    "grid".to_string()
}

fn default_library_cover_size() -> String {
    "medium".to_string()
}

fn default_show_authors() -> bool {
    true
}

fn default_show_tags() -> bool {
    true
}

fn default_tag_limit() -> usize {
    4
}

fn default_title_line_clamp() -> usize {
    2
}

fn default_title_font_size() -> usize {
    15
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryViewSettings {
    #[serde(default = "default_library_layout")]
    pub layout: String,
    #[serde(default = "default_library_cover_size")]
    pub cover_size: String,
    #[serde(default = "default_show_authors")]
    pub show_authors: bool,
    #[serde(default = "default_show_tags")]
    pub show_tags: bool,
    #[serde(default = "default_tag_limit")]
    pub tag_limit: usize,
    #[serde(default = "default_title_line_clamp")]
    pub title_line_clamp: usize,
    #[serde(default = "default_title_font_size")]
    pub title_font_size: usize,
}

impl Default for LibraryViewSettings {
    fn default() -> Self {
        Self {
            layout: default_library_layout(),
            cover_size: default_library_cover_size(),
            show_authors: default_show_authors(),
            show_tags: default_show_tags(),
            tag_limit: default_tag_limit(),
            title_line_clamp: default_title_line_clamp(),
            title_font_size: default_title_font_size(),
        }
    }
}

impl LibraryViewSettings {
    pub fn validate_for_import(&self) -> Result<(), String> {
        validate_option("libraryView.layout", &self.layout, LIBRARY_LAYOUT_OPTIONS)?;
        validate_option(
            "libraryView.coverSize",
            &self.cover_size,
            LIBRARY_COVER_SIZE_OPTIONS,
        )?;
        validate_allowed_usize(
            "libraryView.tagLimit",
            self.tag_limit,
            LIBRARY_TAG_LIMIT_OPTIONS,
        )?;
        validate_allowed_usize(
            "libraryView.titleLineClamp",
            self.title_line_clamp,
            LIBRARY_TITLE_LINE_CLAMP_OPTIONS,
        )?;
        validate_allowed_usize(
            "libraryView.titleFontSize",
            self.title_font_size,
            LIBRARY_TITLE_FONT_SIZE_OPTIONS,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerBookReaderSettings {
    pub book_id: String,
    pub settings: ReaderSettings,
}

impl PerBookReaderSettings {
    pub fn validate_for_import(&self) -> Result<(), String> {
        if self.book_id.trim().is_empty() {
            return Err("perBookReaderSettings.bookId 不能为空".to_string());
        }
        self.settings.validate_for_import()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveReaderSettingsState {
    pub settings: ReaderSettings,
    pub has_book_reader_settings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsExport {
    pub schema_version: u16,
    pub exported_at: String,
    pub reader: ReaderSettings,
    pub library_view: LibraryViewSettings,
    #[serde(default)]
    pub per_book_reader_settings: Vec<PerBookReaderSettings>,
}

impl SettingsExport {
    pub fn validate_for_import(&self, current_schema_version: u16) -> Result<(), String> {
        if self.schema_version != current_schema_version {
            return Err(format!("不支持的设置版本: {}", self.schema_version));
        }
        self.reader.validate_for_import()?;
        self.library_view.validate_for_import()?;
        for override_settings in &self.per_book_reader_settings {
            override_settings.validate_for_import()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SettingsRestoreScope {
    All,
    Reader,
    LibraryView,
}

fn validate_option(field: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} 不支持: {value}"))
    }
}

fn validate_number(field: &str, value: f64, min: f64, max: f64) -> Result<(), String> {
    if value.is_finite() && value >= min && value <= max {
        Ok(())
    } else {
        Err(format!("{field} 超出范围: {value}"))
    }
}

fn validate_usize(field: &str, value: usize, min: usize, max: usize) -> Result<(), String> {
    if value >= min && value <= max {
        Ok(())
    } else {
        Err(format!("{field} 超出范围: {value}"))
    }
}

fn validate_allowed_usize(field: &str, value: usize, allowed: &[usize]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} 不支持: {value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryViewSettings, PerBookReaderSettings, ReaderSettings, SettingsExport,
        SettingsRestoreScope, SETTINGS_SCHEMA_VERSION,
    };

    #[test]
    fn reader_settings_deserializes_missing_auto_scroll_fields() {
        let settings: ReaderSettings = serde_json::from_str(
            r##"{
              "mode": "scroll",
              "fit": "width",
              "direction": "rtl",
              "background": "#000000",
              "spaceScrollRatio": 0.75,
              "spaceHoldSpeedRatio": 3.5,
              "brightness": 1.2,
              "contrast": 0.9,
              "pageAnimation": "fade",
              "preloadCacheLimit": 120
            }"##,
        )
        .expect("legacy reader settings should deserialize");

        assert_eq!(settings.mode, "scroll");
        assert_eq!(settings.fit, "width");
        assert_eq!(settings.direction, "rtl");
        assert_eq!(settings.background, "#000000");
        assert_eq!(settings.space_scroll_ratio, 0.75);
        assert_eq!(settings.space_hold_speed_ratio, 3.5);
        assert_eq!(settings.brightness, 1.2);
        assert_eq!(settings.contrast, 0.9);
        assert_eq!(settings.page_animation, "fade");
        assert_eq!(settings.preload_cache_limit, 120);
        assert_eq!(settings.auto_scroll_speed, 80);
        assert_eq!(settings.auto_scroll_start_delay, 0.0);
        assert!(settings.auto_scroll_stop_on_manual_scroll);
    }

    #[test]
    fn library_view_settings_deserializes_missing_title_fields() {
        let settings: LibraryViewSettings = serde_json::from_str(
            r#"{
              "layout": "list",
              "coverSize": "large",
              "showAuthors": false,
              "showTags": true,
              "tagLimit": 8
            }"#,
        )
        .expect("legacy library view settings should deserialize");

        assert_eq!(settings.layout, "list");
        assert_eq!(settings.cover_size, "large");
        assert!(!settings.show_authors);
        assert!(settings.show_tags);
        assert_eq!(settings.tag_limit, 8);
        assert_eq!(settings.title_line_clamp, 2);
        assert_eq!(settings.title_font_size, 15);
    }

    #[test]
    fn settings_export_deserializes_current_schema() {
        let export: SettingsExport = serde_json::from_str(
            r##"{
              "schemaVersion": 1,
              "exportedAt": "2026-06-18T00:00:00Z",
              "reader": {
                "mode": "scroll",
                "fit": "width",
                "direction": "rtl",
                "background": "#000000",
                "spaceScrollRatio": 0.75,
                "spaceHoldSpeedRatio": 3.5,
                "brightness": 1.2,
                "contrast": 0.9,
                "pageAnimation": "fade",
                "preloadCacheLimit": 120,
                "autoScrollSpeed": 100,
                "autoScrollStartDelay": 1.0,
                "autoScrollStopOnManualScroll": false
              },
              "libraryView": {
                "layout": "list",
                "coverSize": "large",
                "showAuthors": false,
                "showTags": true,
                "tagLimit": 8,
                "titleLineClamp": 3,
                "titleFontSize": 17
              },
              "perBookReaderSettings": []
            }"##,
        )
        .expect("current settings export should deserialize");

        assert_eq!(export.schema_version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(export.reader.mode, "scroll");
        assert_eq!(export.library_view.layout, "list");
        assert!(export.per_book_reader_settings.is_empty());
    }

    #[test]
    fn settings_export_deserializes_legacy_without_per_book_reader_settings() {
        let export: SettingsExport = serde_json::from_str(
            r##"{
              "schemaVersion": 1,
              "exportedAt": "2026-06-18T00:00:00Z",
              "reader": {},
              "libraryView": {}
            }"##,
        )
        .expect("legacy settings export should deserialize");

        assert!(export.per_book_reader_settings.is_empty());
    }

    #[test]
    fn settings_restore_scope_uses_camel_case_values() {
        let scope: SettingsRestoreScope =
            serde_json::from_str(r#""libraryView""#).expect("scope should deserialize");

        assert_eq!(scope, SettingsRestoreScope::LibraryView);
        assert_eq!(
            serde_json::to_string(&SettingsRestoreScope::LibraryView).unwrap(),
            r#""libraryView""#
        );
    }

    #[test]
    fn settings_export_validation_rejects_unsupported_values() {
        let mut export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: ReaderSettings::default(),
            library_view: LibraryViewSettings::default(),
            per_book_reader_settings: Vec::new(),
        };

        export.reader.mode = "bad".to_string();

        assert!(export.validate_for_import(SETTINGS_SCHEMA_VERSION).is_err());
    }

    #[test]
    fn settings_export_validation_rejects_invalid_per_book_settings() {
        let export = SettingsExport {
            schema_version: SETTINGS_SCHEMA_VERSION,
            exported_at: "2026-06-18T00:00:00Z".to_string(),
            reader: ReaderSettings::default(),
            library_view: LibraryViewSettings::default(),
            per_book_reader_settings: vec![PerBookReaderSettings {
                book_id: "book-1".to_string(),
                settings: ReaderSettings {
                    fit: "bad".to_string(),
                    ..ReaderSettings::default()
                },
            }],
        };

        assert!(export.validate_for_import(SETTINGS_SCHEMA_VERSION).is_err());
    }

    #[test]
    fn per_book_reader_settings_validation_reports_readable_empty_book_id() {
        let override_settings = PerBookReaderSettings {
            book_id: " ".to_string(),
            settings: ReaderSettings::default(),
        };

        let error = override_settings.validate_for_import().unwrap_err();

        assert_eq!(error, "perBookReaderSettings.bookId 不能为空");
    }
}

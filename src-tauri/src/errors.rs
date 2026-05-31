use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("路径不存在: {0}")]
    PathMissing(String),
    #[error("路径不是目录: {0}")]
    NotDirectory(String),
    #[error("没有找到有效漫画: {0}")]
    EmptyRepository(String),
    #[error("文件系统错误: {0}")]
    Io(String),
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("压缩包读取错误: {0}")]
    ArchiveError(String),
    #[error("序列化错误: {0}")]
    Serde(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::ArchiveError(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

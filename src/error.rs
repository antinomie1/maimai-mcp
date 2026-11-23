use std::fmt;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Network(String),
    InvalidInput(String),
    Io(String),
    Json(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "未找到数据: {msg}"),
            Self::Network(msg) => write!(f, "网络请求错误: {msg}"),
            Self::InvalidInput(msg) => write!(f, "参数错误: {msg}"),
            Self::Io(msg) => write!(f, "文件读写错误: {msg}"),
            Self::Json(msg) => write!(f, "JSON 解析错误: {msg}"),
            Self::Other(msg) => write!(f, "错误: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::Network(err.to_string())
    }
}

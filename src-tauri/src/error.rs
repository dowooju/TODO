use serde::{Serialize, Serializer};

/// 앱 전체에서 쓰는 오류 타입. 화면(React)으로는 사람이 읽을 수 있는 문자열로 전달된다.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("데이터베이스 오류: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("항목을 찾을 수 없습니다: {0}")]
    NotFound(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

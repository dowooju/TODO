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
    /// 구글 로그인이 필요함 (리프레시 토큰이 없거나 만료·취소됨)
    #[error("구글 계정으로 다시 로그인해 주세요.")]
    AuthRequired,
    /// 인터넷 연결 문제 등 다시 시도하면 될 수 있는 오류
    #[error("네트워크 오류: {0}")]
    Network(String),
    /// 구글 API 가 돌려준 HTTP 오류
    #[error("구글 API 오류 ({status}): {message}")]
    Google { status: u16, message: String },
    #[error("{0}")]
    Other(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] mongodb::error::Error),

    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Session expired")]
    SessionExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("LiveKit error: {0}")]
    LiveKit(String),

    #[error("Interview not found: {0}")]
    InterviewNotFound(uuid::Uuid),

    #[error("Session not found: {0}")]
    SessionNotFound(uuid::Uuid),

    #[error("User not found: {0}")]
    UserNotFound(uuid::Uuid),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("BSON serialization error: {0}")]
    Bson(#[from] bson::ser::Error),

    #[error("BSON deserialization error: {0}")]
    BsonDe(#[from] bson::de::Error),

    #[error("BSON value access error: {0}")]
    BsonValueAccess(String),
}

impl From<bson::document::ValueAccessError> for AppError {
    fn from(err: bson::document::ValueAccessError) -> Self {
        AppError::BsonValueAccess(err.to_string())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::Validation(err.to_string())
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;
        use axum::http::StatusCode;
        let (status, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::SessionExpired => (StatusCode::UNAUTHORIZED, "Session expired".to_string()),
            AppError::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired".to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::InterviewNotFound(id) => (
                StatusCode::NOT_FOUND,
                format!("Interview not found: {}", id),
            ),
            AppError::SessionNotFound(id) => {
                (StatusCode::NOT_FOUND, format!("Session not found: {}", id))
            }
            AppError::UserNotFound(id) => {
                (StatusCode::NOT_FOUND, format!("User not found: {}", id))
            }
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            AppError::LiveKit(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("LiveKit error: {}", e),
            ),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("IO error: {}", e),
            ),
            AppError::Json(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("JSON error: {}", e),
            ),
            AppError::Bson(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("BSON error: {}", e),
            ),
            AppError::BsonDe(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("BSON deserialization error: {}", e),
            ),
            AppError::BsonValueAccess(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("BSON value access error: {}", e),
            ),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

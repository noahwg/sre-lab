use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized,
    NotFound,
    Gone,
    Conflict(String),
    Unavailable,
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "missing or invalid API key".into()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".into()),
            Self::Gone => (StatusCode::GONE, "this link has expired".into()),
            Self::Conflict(m) => (StatusCode::CONFLICT, m),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "store unavailable".into()),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl From<crate::store::StoreError> for AppError {
    fn from(e: crate::store::StoreError) -> Self {
        match e {
            crate::store::StoreError::Conflict => Self::Conflict("already exists".into()),
            crate::store::StoreError::Unavailable => Self::Unavailable,
        }
    }
}

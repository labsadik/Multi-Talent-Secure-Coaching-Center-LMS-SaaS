use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request")]
    BadRequest(&'static str),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("resource conflict")]
    Conflict(&'static str),
    #[error("service is not configured")]
    ServiceUnavailable(&'static str),
    #[error("database operation failed")]
    Database(#[from] mongodb::error::Error),
    #[error("upstream service request failed")]
    Upstream(#[from] reqwest::Error),
    #[error("internal server error")]
    Internal,
}

impl AppError {
    pub fn from_mongo(error: mongodb::error::Error) -> Self {
        if error.to_string().contains("E11000") {
            Self::Conflict("resource already exists")
        } else {
            Self::Database(error)
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", *message),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", "Authentication is required"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Access is not allowed"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Resource was not found"),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", *message),
            Self::ServiceUnavailable(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_not_configured",
                "A required service is not configured",
            ),
            Self::Database(error) => {
                tracing::error!(error = ?error, "database operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "An unexpected error occurred")
            }
            Self::Upstream(error) => {
                tracing::error!(error = ?error, "upstream service request failed");
                (StatusCode::BAD_GATEWAY, "upstream_error", "An upstream service request failed")
            }
            Self::Internal => {
                tracing::error!("internal operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "An unexpected error occurred")
            }
        };

        (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

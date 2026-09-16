use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication is required")]
    Unauthorized,
    #[error("you do not have access to this resource")]
    Forbidden,
    #[error("{0}")]
    Validation(String),
    #[error("resource not found")]
    NotFound,
    #[error("the service is not configured: {0}")]
    Configuration(String),
    #[error("a conflicting change already exists")]
    Conflict,
    #[error("upstream service is unavailable")]
    Upstream,
    #[error("internal service error")]
    Internal(#[source] anyhow::Error),
}

impl AppError {
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    pub fn internal(error: impl Into<anyhow::Error>) -> Self {
        Self::Internal(error.into())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string()),
            Self::Validation(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                self.to_string(),
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            Self::Configuration(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_not_configured",
                self.to_string(),
            ),
            Self::Conflict => (StatusCode::CONFLICT, "conflict", self.to_string()),
            Self::Upstream => (
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                self.to_string(),
            ),
            Self::Internal(error) => {
                tracing::error!(error = ?error, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Something went wrong. Please try again.".to_owned(),
                )
            }
        };

        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::NotFound,
            other => Self::internal(other),
        }
    }
}

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::SendError;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    InternalError(String),
    ValidationError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    NotFoundError(String),
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => AppError::NotFoundError("not found".to_string()),
            _ => AppError::InternalError("internal error".to_string()),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        AppError::InternalError(format!("{:?}", error))
    }
}

impl From<argon2::Error> for AppError {
    fn from(error: argon2::Error) -> Self {
        AppError::InternalError(format!("{:?}", error))
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(error: argon2::password_hash::Error) -> Self {
        AppError::InternalError(format!("{:?}", error))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::InternalError(format!("{:?}", error))
    }
}

impl<T> From<SendError<T>> for AppError {
    fn from(error: SendError<T>) -> Self {
        AppError::InternalError(format!("{}", error))
    }
}

impl From<axum::Error> for AppError {
    fn from(error: axum::Error) -> Self {
        AppError::InternalError(format!("{}", error))
    }
}

impl From<uuid::Error> for AppError {
    fn from(error: uuid::Error) -> Self {
        AppError::InternalError(format!("{}", error))
    }
}

impl From<hyper::Error> for AppError {
    fn from(error: hyper::Error) -> Self {
        AppError::InternalError(format!("{}", error))
    }
}

#[derive(Deserialize, Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            AppError::InternalError(error) => (StatusCode::INTERNAL_SERVER_ERROR, error),
            AppError::ValidationError(error) => (StatusCode::BAD_REQUEST, error),
            AppError::AuthenticationError(error) => (StatusCode::UNAUTHORIZED, error),
            AppError::AuthorizationError(error) => (StatusCode::FORBIDDEN, error),
            AppError::NotFoundError(error) => (StatusCode::NOT_FOUND, error),
        };

        (status, Json(ErrorResponse { error })).into_response()
    }
}

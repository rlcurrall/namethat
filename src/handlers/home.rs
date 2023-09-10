use axum::{
    extract::Path,
    response::{Html, IntoResponse},
};

use crate::{
    error::AppResult,
    extractors::auth::AuthUser,
    view::{Forbidden, Index, NotFound, ServerError, ServiceUnavailable, Unauthorized},
    Assets,
};

pub async fn index(AuthUser(user): AuthUser) -> AppResult<Html<String>> {
    Ok(Index::new(user.is_some()).to_html()?)
}

pub async fn unauthorized(AuthUser(user): AuthUser) -> AppResult<Html<String>> {
    Ok(Unauthorized::new(user.is_some()).to_html()?)
}

pub async fn forbidden(AuthUser(user): AuthUser) -> AppResult<Html<String>> {
    Ok(Forbidden::new(user.is_some()).to_html()?)
}

pub async fn not_found(AuthUser(user): AuthUser) -> AppResult<Html<String>> {
    Ok(NotFound::new(user.is_some()).to_html()?)
}

pub async fn internal_server_error(AuthUser(user): AuthUser) -> AppResult<Html<String>> {
    Ok(ServerError::new(user.is_some()).to_html()?)
}

pub async fn service_unavailable(AuthUser(user): AuthUser) -> AppResult<Html<String>> {
    Ok(ServiceUnavailable::new(user.is_some()).to_html()?)
}

// static handler using Assets
pub async fn static_handler(Path(path): Path<String>) -> AppResult<impl IntoResponse> {
    tracing::info!("static_handler: path={}", path);
    Assets::render(&path)
}

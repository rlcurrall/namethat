use std::sync::Arc;

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Json,
};
use axum_sessions::extractors::WritableSession;
use serde::Deserialize;
use serde_json::json;

use crate::{
    error::{AppError, AppResult},
    extractors::auth::{ApiAuth, WebAuth},
    models::users::UserUpdate,
    services::auth::AuthService,
    view::Profile,
    AppState,
};

pub async fn show_profile(_: WebAuth) -> AppResult<Html<String>> {
    Ok(Profile::new().to_html()?)
}

pub async fn get(ApiAuth(user): ApiAuth) -> AppResult<impl IntoResponse> {
    Ok(Json(user).into_response())
}

pub async fn update(
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
    Json(update_request): Json<UpdateUserRequest>,
) -> AppResult<impl IntoResponse> {
    let password = match (update_request.new_password, update_request.old_password) {
        (Some(new_password), Some(old_password)) => {
            if !AuthService::check_password(&old_password, &user.password)? {
                return Err(AppError::AuthorizationError("Invalid password".into()));
            }
            AuthService::hash_password(&new_password)?
        }
        (Some(_), None) => {
            return Err(AppError::ValidationError("Old password is required".into()))
        }
        (None, _) => user.password,
    };

    let updated_user = state
        .user_repo
        .update(
            user.id,
            UserUpdate {
                email: update_request.email,
                password,
            },
        )
        .await?;

    Ok(Json(updated_user).into_response())
}

pub async fn delete(
    mut session: WritableSession,
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    session.destroy();
    state.user_repo.delete(user.id).await?;
    Ok(Json(json!({ "success": true })).into_response())
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: String,
    pub old_password: Option<String>,
    pub new_password: Option<String>,
}

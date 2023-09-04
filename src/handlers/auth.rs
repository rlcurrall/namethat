use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Json,
};
use axum_sessions::extractors::WritableSession;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::users::NewUser,
    services::auth::AuthService,
    AppState, Assets,
};

pub async fn show_login_form(session: WritableSession) -> Result<impl IntoResponse> {
    if is_logged_in(&session) {
        Ok(Redirect::to("/games").into_response())
    } else {
        Ok(Assets::render("login.html")?.into_response())
    }
}

pub async fn show_register_form(session: WritableSession) -> Result<impl IntoResponse> {
    if is_logged_in(&session) {
        Ok(Redirect::to("/games").into_response())
    } else {
        Ok(Assets::render("register.html")?.into_response())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Login {
    email: String,
    password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    mut session: WritableSession,
    Json(data): Json<Login>,
) -> Result<impl IntoResponse> {
    if is_logged_in(&session) {
        return Err(AppError::ValidationError("Already logged in".to_string()));
    }

    let user = state
        .user_repo
        .get_by_email(data.email)
        .await?
        .ok_or(AppError::NotFoundError("User not found".to_string()))?;

    if AuthService::check_password(&data.password, &user.password)? {
        session.insert("user_id", user.id).unwrap();
        Ok(Json(json!({ "success": true })))
    } else {
        Err(AppError::AuthorizationError("Invalid password".into()))
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    mut session: WritableSession,
    Json(mut form): Json<NewUser>,
) -> Result<impl IntoResponse> {
    if is_logged_in(&session) {
        return Err(AppError::ValidationError("Already logged in".into()));
    }

    let user = state.user_repo.get_by_email(form.email.clone()).await?;

    if user.is_some() {
        return Err(AppError::ValidationError("Email already in use".into()));
    }

    form.password = AuthService::hash_password(&form.password)?;

    let user = state.user_repo.insert(form).await?;
    session.insert("user_id", user.id).unwrap();

    Ok(Json(json!({ "success": true })))
}

pub async fn logout(mut session: WritableSession) -> impl IntoResponse {
    session.destroy();
    Redirect::to("/")
}

fn is_logged_in(session: &WritableSession) -> bool {
    session.get::<Uuid>("user_id").is_some()
}

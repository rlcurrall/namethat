use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    extractors::auth::{ApiAuth, AuthUser, WebAuth},
    models::games::{GameFilter, NewGame, UpdateGame},
    view::{CreateGame, Games, PlayGame, RunGame},
    AppState,
};

pub async fn games_page(_: WebAuth) -> AppResult<Html<String>> {
    Ok(Games::new().to_html()?)
}

pub async fn create_page(_: WebAuth) -> AppResult<Html<String>> {
    Ok(CreateGame::new().to_html()?)
}

pub async fn run_page(
    WebAuth(user): WebAuth,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let game = state.game_repo.get(&game_id).await?;

    if game.user_id != user.id {
        return Ok(Redirect::to("/403").into_response());
    }

    Ok(RunGame::new(true).to_html()?.into_response())
}

pub async fn play_page(
    AuthUser(user): AuthUser,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let game = state.game_repo.get(&game_id).await?;
    let authenticated = user.is_some();

    if let Some(user) = user {
        if game.user_id == user.id {
            return Ok(Redirect::to(&format!("/games/{}/run", game_id)).into_response());
        }
    }

    Ok(PlayGame::new(authenticated).to_html()?.into_response())
}

pub async fn list(
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let games = state
        .game_repo
        .list(GameFilter {
            user_id: Some(user.id),
            status: Some(crate::models::games::GameStatus::Pending),
        })
        .await?;
    Ok(Json(games).into_response())
}

pub async fn create(
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
    Json(new_game): Json<NewGameRequest>,
) -> AppResult<impl IntoResponse> {
    let game = state
        .game_repo
        .insert(NewGame {
            user_id: user.id,
            name: format!("Name that {}", new_game.name),
            image_urls: new_game.images,
        })
        .await?;
    Ok(Json(game).into_response())
}

pub async fn get(
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let game = state.game_repo.get(&game_id).await?;

    if game.user_id != user.id {
        return Err(AppError::AuthorizationError(
            "You are not authorized to view this game".to_string(),
        ));
    }

    Ok(Json(game).into_response())
}

pub async fn update(
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<Uuid>,
    Json(game_update): Json<UpdateGameRequest>,
) -> AppResult<impl IntoResponse> {
    let game = state.game_repo.get(&game_id).await?;

    if game.user_id != user.id {
        return Err(AppError::AuthorizationError(
            "You are not authorized to modify this game".to_string(),
        ));
    }

    let game = state
        .game_repo
        .update(
            game_id,
            UpdateGame {
                name: game_update.name.unwrap_or(game.name),
                image_urls: game_update.images.unwrap_or(game.image_urls),
            },
        )
        .await?;
    Ok(Json(game).into_response())
}

pub async fn delete(
    ApiAuth(user): ApiAuth,
    State(state): State<Arc<AppState>>,
    Path(game_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let game = state.game_repo.get(&game_id).await?;

    if game.user_id != user.id {
        return Err(AppError::AuthorizationError(
            "You are not authorized to delete this game".to_string(),
        ));
    }

    let game = state.game_repo.delete(game_id).await?;
    Ok(Json(game).into_response())
}

#[derive(Deserialize)]
pub struct NewGameRequest {
    pub name: String,
    pub images: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateGameRequest {
    pub name: Option<String>,
    pub images: Option<Vec<String>>,
}

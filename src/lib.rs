use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use axum::response::{Html, IntoResponse};
use error::AppError;
use models::games::GameBroadcast;
use repositories::games::GameRepo;
use rust_embed::RustEmbed;
use tokio::sync::broadcast;

use crate::repositories::users::UserRepo;
use crate::services::session::SessionManager;

pub mod error;
pub mod extractors;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod services;
pub mod session;

#[derive(Clone, Debug)]
pub struct AppState {
    pub user_set: Arc<Mutex<HashSet<String>>>,
    pub tx: broadcast::Sender<GameBroadcast>,
    pub user_repo: UserRepo,
    pub game_repo: GameRepo,
    pub session_manager: SessionManager,
}

#[derive(RustEmbed)]
#[folder = "assets/"]
pub(crate) struct Assets;

impl Assets {
    pub(crate) fn render(path: &str) -> Result<impl IntoResponse, AppError> {
        match Assets::get(path) {
            Some(content) => Ok(Html(content.data)),
            None => Err(AppError::InternalError(format!(
                "Could not load {} page",
                path
            ))),
        }
    }
}

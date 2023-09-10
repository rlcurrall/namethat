use std::sync::Arc;

use axum::{
    body::Body,
    routing::{get, post},
    Router,
};

use crate::AppState;

pub mod auth;
pub mod games;
pub mod home;
pub mod profile;
pub mod websocket;

pub struct AppRouter;

impl AppRouter {
    pub fn build() -> Router<Arc<AppState>, Body> {
        Router::new()
            .route("/", get(home::index))
            .route("/401", get(home::unauthorized))
            .route("/403", get(home::forbidden))
            .route("/404", get(home::not_found))
            .route("/500", get(home::internal_server_error))
            .route("/503", get(home::service_unavailable))
            .route("/login", get(auth::show_login_form))
            .route("/api/login", post(auth::login))
            .route("/register", get(auth::show_register_form))
            .route("/api/register", post(auth::register))
            .route("/logout", get(auth::logout))
            .route("/games", get(games::games_page))
            .route("/games/create", get(games::create_page))
            .route("/games/:id/run", get(games::run_page))
            .route("/games/:id/play", get(games::play_page))
            .route("/games/:id/ws", get(websocket::game_websocket))
            .route("/api/games", get(games::list).post(games::create))
            .route(
                "/api/games/:id",
                get(games::get).put(games::update).delete(games::delete),
            )
            .route("/profile", get(profile::show_profile))
            .route(
                "/api/profile",
                get(profile::get)
                    .put(profile::update)
                    .delete(profile::delete),
            )
            .route("/assets/:path", get(home::static_handler))
    }
}

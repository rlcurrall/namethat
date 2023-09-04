use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    routing::{get, post},
    Router,
};

use crate::{error::AppError, models::games::GameFilter, AppState};

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
            .route("/test", get(test))
    }
}

async fn test(State(state): State<Arc<AppState>>) -> crate::error::Result<&'static str> {
    let games = state
        .game_repo
        .list(GameFilter {
            user_id: None,
            status: None,
        })
        .await?;

    let _game = games
        .first()
        .ok_or(AppError::NotFoundError("nope".into()))?;

    // Add player
    // let _ = state.game_repo.add_player(game.id, "robb".into()).await;
    // let r = state.game_repo.add_player(game.id, "tom".into()).await;
    // println!("{:?}", r);

    // Add Round
    // let round = NewRound {
    //     game_id: game.id,
    //     round_number: 1,
    //     image_url: "https://example.com/image.png".into(),
    // };
    // let r = state.game_repo.add_round(round).await?;
    // println!("{:?}", r);

    // Add answer
    // let r = state
    //     .game_repo
    //     .add_answer(NewAnswer {
    //         game_id: game.id,
    //         round_number: 1,
    //         username: "robb".into(),
    //         value: "answer".into(),
    //     })
    //     .await;
    // println!("{:?}", r);

    // Increment like
    // let r = {
    //     let ref this = state.game_repo;
    //     let answer_id = game.id;
    //     async move {
    //         sqlx::query!(
    //             r#"
    //         UPDATE answers
    //         SET likes = COALESCE(likes, 0) + 1
    //         WHERE id = $1
    //         "#,
    //             answer_id
    //         )
    //         .execute(&this.client)
    //         .await?;
    //         Ok(this.get_game_by_answer_id(&answer_id).await?)
    //     }
    // }
    // .await;
    // println!("{:?}", r);

    // Increment score
    // let r = state
    //     .game_repo
    //     .increment_score(game.id, "robb".into())
    //     .await;
    // println!("{:?}", r);

    return Ok("ok");
}

use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use axum_sessions::SessionLayer;
use dotenv::dotenv;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use namethat::{
    error::AppResult,
    handlers::AppRouter,
    repositories::{games::GameRepo, users::UserRepo},
    services::session::SessionManager,
    session::SessionStore,
    AppConfig, AppState,
};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let session_secret = std::env::var("SESSION_SECRET").expect("SESSION_SECRET must be set");
    let app_log = std::env::var("APP_LOG").unwrap_or("namethat=debug".to_string());
    let app_config = AppConfig {
        database_url,
        session_secret,
        app_log,
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&app_config.app_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tokio::select! {
        _ = tokio::signal::ctrl_c() => (),
        _ = serve(&app_config) => (),
        _ = tasks(&app_config) => (),
    }

    tracing::info!("Shutting down");
}

async fn serve(app_config: &AppConfig) -> AppResult<()> {
    let client = sqlx::PgPool::connect(&app_config.database_url)
        .await
        .expect("Could not connect to database");

    sqlx::migrate!().run(&client).await.unwrap();

    let session_store = SessionStore::from_client(client.clone());
    let user_repo = UserRepo::new(client.clone());
    let game_repo = GameRepo::new(client.clone());
    let user_set = Arc::new(Mutex::new(HashSet::new()));
    let session_manager = SessionManager::new(session_store.clone());
    let (tx, _) = broadcast::channel(50);

    let app = AppRouter::build()
        .layer(SessionLayer::new(
            session_store,
            app_config.session_secret.as_bytes(),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(AppState {
            user_set,
            tx,
            user_repo,
            game_repo,
            session_manager,
        }));

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3000));
    tracing::debug!("listening on {}", addr);

    Ok(axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?)
}

async fn tasks(_app_config: &AppConfig) -> AppResult<()> {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // tracing::info!("running tasks");
    }
}

use axum::response::IntoResponse;

use crate::Assets;

pub async fn index() -> impl IntoResponse {
    Assets::render("index.html")
}

pub async fn unauthorized() -> impl IntoResponse {
    Assets::render("401.html")
}

pub async fn forbidden() -> impl IntoResponse {
    Assets::render("403.html")
}

pub async fn not_found() -> impl IntoResponse {
    Assets::render("404.html")
}

pub async fn internal_server_error() -> impl IntoResponse {
    Assets::render("500.html")
}

pub async fn service_unavailable() -> impl IntoResponse {
    Assets::render("503.html")
}

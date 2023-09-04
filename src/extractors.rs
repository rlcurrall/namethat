pub mod auth {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        extract::FromRequestParts,
        http::request::Parts,
        response::IntoResponse,
        response::{Redirect, Response},
        RequestPartsExt,
    };
    use axum_sessions::extractors::ReadableSession;

    use crate::{error::AppError, AppState};

    #[derive(Debug)]
    pub struct WebAuth(pub crate::models::users::User);

    #[async_trait]
    impl FromRequestParts<Arc<AppState>> for WebAuth {
        type Rejection = Response;

        async fn from_request_parts(
            parts: &mut Parts,
            state: &Arc<AppState>,
        ) -> Result<Self, Self::Rejection> {
            let session = parts
                .extract::<ReadableSession>()
                .await
                .map_err(|_| Redirect::to("/500").into_response())?;
            let user_id = session.get::<uuid::Uuid>("user_id");
            match user_id {
                None => Err(Redirect::to("/login").into_response()),
                Some(user_id) => {
                    let user = state
                        .user_repo
                        .get(user_id)
                        .await
                        .map_err(|_| Redirect::to("/500").into_response())?;
                    Ok(WebAuth(user))
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct ApiAuth(pub crate::models::users::User);

    #[async_trait]
    impl FromRequestParts<Arc<AppState>> for ApiAuth {
        type Rejection = AppError;

        async fn from_request_parts(
            parts: &mut Parts,
            state: &Arc<AppState>,
        ) -> Result<Self, Self::Rejection> {
            let session = parts
                .extract::<ReadableSession>()
                .await
                .map_err(|_| AppError::InternalError("Could not load session".into()))?;
            let user_id = session.get::<uuid::Uuid>("user_id");
            match user_id {
                None => Err(AppError::AuthenticationError("Not logged in".to_string())),
                Some(user_id) => {
                    let user =
                        state.user_repo.get(user_id).await.map_err(|_| {
                            AppError::InternalError("Could not find the user".into())
                        })?;
                    Ok(ApiAuth(user))
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct AuthUser(pub Option<crate::models::users::User>);

    #[async_trait]
    impl FromRequestParts<Arc<AppState>> for AuthUser {
        type Rejection = AppError;

        async fn from_request_parts(
            parts: &mut Parts,
            state: &Arc<AppState>,
        ) -> Result<Self, Self::Rejection> {
            let session = parts
                .extract::<ReadableSession>()
                .await
                .map_err(|_| AppError::InternalError("Could not load session".into()))?;
            match session.get::<uuid::Uuid>("user_id") {
                None => Ok(AuthUser(None)),
                Some(user_id) => match state.user_repo.get(user_id).await {
                    Ok(user) => Ok(AuthUser(Some(user))),
                    Err(e) => match e {
                        AppError::NotFoundError(_) => Ok(AuthUser(None)),
                        _ => Err(AppError::InternalError("Could not find the user".into())),
                    },
                },
            }
        }
    }
}

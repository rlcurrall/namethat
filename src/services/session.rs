use async_session::SessionStore;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    session::SessionStore as Store,
};

#[derive(Clone, Debug)]
pub struct SessionManager {
    session_store: Store,
}

impl SessionManager {
    pub fn new(session_store: Store) -> Self {
        Self { session_store }
    }

    pub async fn get<T: DeserializeOwned>(&self, id: &str, key: String) -> Result<Option<T>> {
        let session = self
            .session_store
            .load_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        match session {
            Some(session) => Ok(session.get::<T>(&key)),
            None => Ok(None),
        }
    }

    pub async fn set<T: serde::ser::Serialize>(
        &self,
        id: &str,
        key: String,
        value: T,
    ) -> Result<()> {
        let mut session = self
            .session_store
            .load_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .ok_or(AppError::InternalError("Session not found".into()))?;

        session
            .insert(&key, value)
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        self.session_store
            .store_session(session)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(())
    }

    pub async fn set_game_display_name(
        &self,
        id: &str,
        game_id: &Uuid,
        player_id: &Uuid,
    ) -> Result<()> {
        self.set(id, format!("game-{}-username", game_id), player_id)
            .await
    }

    pub async fn get_game_display_name(&self, id: &str, game_id: &Uuid) -> Result<Option<Uuid>> {
        self.get(id, format!("game-{}-username", game_id)).await
    }
}

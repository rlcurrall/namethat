use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewUser {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserFilter {
    pub email: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserUpdate {
    pub email: String,
    pub password: String,
}

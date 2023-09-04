use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::users::{NewUser, User, UserFilter, UserUpdate},
};

#[derive(Clone, Debug)]
pub struct UserRepo {
    client: PgPool,
}

impl UserRepo {
    pub fn new(client: PgPool) -> Self {
        Self { client }
    }

    pub async fn insert(&self, new_user: NewUser) -> Result<User> {
        // Verify that the NewUser password is hashed
        if !new_user.password.starts_with("$argon2") {
            return Err(AppError::ValidationError("Password is not hashed".into()));
        }

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password)
            VALUES ($1, $2)
            RETURNING id, email, password
            "#,
            new_user.email,
            new_user.password
        )
        .fetch_one(&self.client)
        .await?;
        Ok(user)
    }

    pub async fn get(&self, id: Uuid) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.client)
        .await?;
        Ok(user)
    }

    pub async fn get_by_email(&self, email: String) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.client)
        .await?;
        Ok(user)
    }

    pub async fn list(&self, filter: UserFilter) -> Result<Vec<User>> {
        let query = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password
            FROM users
            WHERE ($1::varchar IS NULL OR email = $1::varchar)
            "#,
            filter.email
        );
        let users = query.fetch_all(&self.client).await.map_err(|e| {
            tracing::error!("error listing users: {}", e);
            e
        })?;
        Ok(users)
    }

    pub async fn update(&self, user_id: Uuid, update: UserUpdate) -> Result<User> {
        let query = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET
                email = $1,
                password = $2
            WHERE id = $3
            RETURNING id, email, password
            "#,
            update.email,
            update.password,
            user_id
        );
        let update = query.fetch_one(&self.client).await?;
        Ok(update)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.client)
        .await?;
        Ok(())
    }
}

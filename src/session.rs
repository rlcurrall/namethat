use async_session::{async_trait, serde_json, Result, Session, SessionStore as BaseSessionStore};
use sqlx::{pool::PoolConnection, PgPool, Postgres};

use time::OffsetDateTime as DateTime;

/// sqlx postgres session store for async-sessions
///
/// ```rust
/// use namethat::session::SessionStore;
/// use async_session::{Session, SessionStore};
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() -> async_session::Result {
/// # dotenv::dotenv().ok();
/// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?;
/// store.migrate().await?;
/// # store.clear_store().await?;
///
/// let mut session = Session::new();
/// session.insert("key", vec![1,2,3]);
///
/// let cookie_value = store.store_session(session).await?.unwrap();
/// let session = store.load_session(cookie_value).await?.unwrap();
/// assert_eq!(session.get::<Vec<i8>>("key").unwrap(), vec![1,2,3]);
/// # Ok(()) }
///
#[derive(Clone, Debug)]
pub struct SessionStore {
    client: PgPool,
    table_name: String,
}

impl SessionStore {
    /// constructs a new SessionStore from an existing
    /// sqlx::PgPool.  the default table name for this session
    /// store will be "async_sessions". To override this, chain this
    /// with [`with_table_name`](crate::SessionStore::with_table_name).
    ///
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let pool = sqlx::PgPool::connect(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await.unwrap();
    /// let store = SessionStore::from_client(pool)
    ///     .with_table_name("custom_table_name");
    /// store.migrate().await;
    /// # Ok(()) }
    /// ```
    pub fn from_client(client: PgPool) -> Self {
        Self {
            client,
            table_name: "sessions".into(),
        }
    }

    /// Constructs a new SessionStore from a postgres://
    /// database url. The default table name for this session store
    /// will be "async_sessions". To override this, either chain with
    /// [`with_table_name`](crate::SessionStore::with_table_name)
    /// or use
    /// [`new_with_table_name`](crate::SessionStore::new_with_table_name)
    ///
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?;
    /// store.migrate().await;
    /// # Ok(()) }
    /// ```
    pub async fn new(database_url: &str) -> sqlx::Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self::from_client(pool))
    }

    /// constructs a new SessionStore from a postgres:// url. the
    /// default table name for this session store will be
    /// "async_sessions". To override this, either chain with
    /// [`with_table_name`](crate::SessionStore::with_table_name) or
    /// use
    /// [`new_with_table_name`](crate::SessionStore::new_with_table_name)
    ///
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new_with_table_name(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap(), "custom_table_name").await?;
    /// store.migrate().await;
    /// # Ok(()) }
    /// ```
    pub async fn new_with_table_name(database_url: &str, table_name: &str) -> sqlx::Result<Self> {
        Ok(Self::new(database_url).await?.with_table_name(table_name))
    }

    /// Chainable method to add a custom table name. This will panic
    /// if the table name is not `[a-zA-Z0-9_-]+`.
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?
    ///     .with_table_name("custom_name");
    /// store.migrate().await;
    /// # Ok(()) }
    /// ```
    ///
    /// ```should_panic
    /// # use namethat::session::SessionStore;
    /// # use async_session::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?
    ///     .with_table_name("johnny (); drop users;");
    /// # Ok(()) }
    /// ```
    pub fn with_table_name(mut self, table_name: impl AsRef<str>) -> Self {
        let table_name = table_name.as_ref();
        if table_name.is_empty()
            || !table_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            panic!(
                "table name must be [a-zA-Z0-9_-]+, but {} was not",
                table_name
            );
        }

        self.table_name = table_name.to_owned();
        self
    }

    /// Creates a session table if it does not already exist. If it
    /// does, this will noop, making it safe to call repeatedly on
    /// store initialization. In the future, this may make
    /// exactly-once modifications to the schema of the session table
    /// on breaking releases.
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::{Result, SessionStore, Session};
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?;
    /// # store.clear_store().await?;
    /// store.migrate().await?;
    /// store.store_session(Session::new()).await?;
    /// store.migrate().await?; // calling it a second time is safe
    /// assert_eq!(store.count().await?, 1);
    /// # Ok(()) }
    /// ```
    pub async fn migrate(&self) -> sqlx::Result<()> {
        sqlx::migrate!().run(&self.client).await?;
        Ok(())
    }

    fn substitute_table_name(&self, query: &str) -> String {
        query.replace("%%TABLE_NAME%%", &self.table_name)
    }

    /// retrieve a connection from the pool
    async fn connection(&self) -> sqlx::Result<PoolConnection<Postgres>> {
        self.client.acquire().await
    }

    /// Performs a one-time cleanup task that clears out stale
    /// (expired) sessions. You may want to call this from cron.
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::{Result, SessionStore, Session};
    /// # use time::Duration;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?;
    /// store.migrate().await?;
    /// # store.clear_store().await?;
    /// let mut session = Session::new();
    /// session.set_expiry(chrono::Utc::now() - chrono::Duration::seconds(5));
    /// store.store_session(session).await?;
    /// assert_eq!(store.count().await?, 1);
    /// store.cleanup().await?;
    /// assert_eq!(store.count().await?, 0);
    /// # Ok(()) }
    /// ```
    pub async fn cleanup(&self) -> sqlx::Result<()> {
        let mut connection = self.connection().await?;
        sqlx::query(&self.substitute_table_name("DELETE FROM %%TABLE_NAME%% WHERE expires < $1"))
            .bind(DateTime::now_utc())
            .execute(&mut *connection)
            .await?;

        Ok(())
    }

    /// retrieves the number of sessions currently stored, including
    /// expired sessions
    ///
    /// ```rust
    /// # use namethat::session::SessionStore;
    /// # use async_session::{Result, SessionStore, Session};
    /// # use std::time::Duration;
    /// # #[tokio::main]
    /// # async fn main() -> Result {
    /// # dotenv::dotenv().ok();
    /// let store = SessionStore::new(&std::env::var("TEST_SESSION_DATABASE_URL").unwrap()).await?;
    /// store.migrate().await?;
    /// # store.clear_store().await?;
    /// assert_eq!(store.count().await?, 0);
    /// store.store_session(Session::new()).await?;
    /// assert_eq!(store.count().await?, 1);
    /// # Ok(()) }
    /// ```
    pub async fn count(&self) -> sqlx::Result<i64> {
        let (count,) =
            sqlx::query_as(&self.substitute_table_name("SELECT COUNT(*) FROM %%TABLE_NAME%%"))
                .fetch_one(&mut *self.connection().await?)
                .await?;

        Ok(count)
    }

    pub async fn load_by_id(&self, id: &str) -> Result<Option<Session>> {
        let mut connection = self.connection().await?;

        let result: Option<(String,)> = sqlx::query_as(&self.substitute_table_name(
            "SELECT session FROM %%TABLE_NAME%% WHERE id = $1 AND (expires IS NULL OR expires > $2)"
        ))
        .bind(id)
        .bind(DateTime::now_utc())
        .fetch_optional(&mut *connection)
        .await?;

        Ok(result
            .map(|(session,)| serde_json::from_str(&session))
            .transpose()?)
    }
}

#[async_trait]
impl BaseSessionStore for SessionStore {
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        let id = Session::id_from_cookie_value(&cookie_value)?;
        let mut connection = self.connection().await?;

        let result: Option<(String,)> = sqlx::query_as(&self.substitute_table_name(
            "SELECT session FROM %%TABLE_NAME%% WHERE id = $1 AND (expires IS NULL OR expires > $2)"
        ))
        .bind(&id)
        .bind(DateTime::now_utc())
        .fetch_optional(&mut *connection)
        .await?;

        Ok(result
            .map(|(session,)| serde_json::from_str(&session))
            .transpose()?)
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        let id = session.id();
        let string = serde_json::to_string(&session)?;
        let mut connection = self.connection().await?;

        sqlx::query(&self.substitute_table_name(
            r#"
            INSERT INTO %%TABLE_NAME%%
              (id, session, expires) SELECT $1, $2, $3
            ON CONFLICT(id) DO UPDATE SET
              expires = EXCLUDED.expires,
              session = EXCLUDED.session
            "#,
        ))
        .bind(&id)
        .bind(&string)
        .bind(&session.expiry())
        .execute(&mut *connection)
        .await?;

        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: Session) -> Result {
        let id = session.id();
        let mut connection = self.connection().await?;
        sqlx::query(&self.substitute_table_name("DELETE FROM %%TABLE_NAME%% WHERE id = $1"))
            .bind(&id)
            .execute(&mut *connection)
            .await?;

        Ok(())
    }

    async fn clear_store(&self) -> Result {
        let mut connection = self.connection().await?;
        sqlx::query(&self.substitute_table_name("TRUNCATE %%TABLE_NAME%%"))
            .execute(&mut *connection)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::time::Duration;

    async fn test_store() -> SessionStore {
        dotenv::dotenv().ok();

        let database_url =
            std::env::var("TEST_SESSION_DATABASE_URL").expect("Did not find database URL");

        let store = SessionStore::new(&database_url)
            .await
            .expect("building a SessionStore");

        store.migrate().await.expect("migrating a SessionStore");

        store.clear_store().await.expect("clearing");

        store
    }

    #[tokio::test]
    #[serial]
    async fn creating_a_new_session_with_no_expiry() -> Result {
        let store = test_store().await;
        let mut session = Session::new();
        session.insert("key", "value")?;
        let cloned = session.clone();
        let cookie_value = store.store_session(session).await?.unwrap();

        let (id, expires, serialized, count): (String, Option<DateTime>, String, i64) =
            sqlx::query_as("select id, expires, session, (select count(*) from async_sessions) from async_sessions")
                .fetch_one(&mut *store.connection().await?)
                .await?;

        assert_eq!(1, count);
        assert_eq!(id, cloned.id());
        assert_eq!(expires, None);

        let deserialized_session: Session = serde_json::from_str(&serialized)?;
        assert_eq!(cloned.id(), deserialized_session.id());
        assert_eq!("value", &deserialized_session.get::<String>("key").unwrap());

        let loaded_session = store.load_session(cookie_value).await?.unwrap();
        assert_eq!(cloned.id(), loaded_session.id());
        assert_eq!("value", &loaded_session.get::<String>("key").unwrap());

        assert!(!loaded_session.is_expired());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn updating_a_session() -> Result {
        let store = test_store().await;
        let mut session = Session::new();
        let original_id = session.id().to_owned();

        session.insert("key", "value")?;
        let cookie_value = store.store_session(session).await?.unwrap();

        let mut session = store.load_session(cookie_value.clone()).await?.unwrap();
        session.insert("key", "other value")?;
        assert_eq!(None, store.store_session(session).await?);

        let session = store.load_session(cookie_value.clone()).await?.unwrap();
        assert_eq!(session.get::<String>("key").unwrap(), "other value");

        let (id, count): (String, i64) =
            sqlx::query_as("select id, (select count(*) from async_sessions) from async_sessions")
                .fetch_one(&mut *store.connection().await?)
                .await?;

        assert_eq!(1, count);
        assert_eq!(original_id, id);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn updating_a_session_extending_expiry() -> Result {
        let store = test_store().await;
        let mut session = Session::new();
        session.expire_in(Duration::from_secs(10));
        let original_id = session.id().to_owned();
        let original_expires = session.expiry().unwrap().clone();
        let cookie_value = store.store_session(session).await?.unwrap();

        let mut session = store.load_session(cookie_value.clone()).await?.unwrap();
        assert_eq!(session.expiry().unwrap(), &original_expires);
        session.expire_in(Duration::from_secs(20));
        let new_expires = session.expiry().unwrap().clone();
        store.store_session(session).await?;

        let session = store.load_session(cookie_value.clone()).await?.unwrap();
        assert_eq!(session.expiry().unwrap(), &new_expires);

        let (id, expires, count): (String, DateTime, i64) = sqlx::query_as(
            "select id, expires, (select count(*) from async_sessions) from async_sessions",
        )
        .fetch_one(&mut *store.connection().await?)
        .await?;

        assert_eq!(1, count);
        assert_eq!(expires.unix_timestamp(), new_expires.timestamp());
        assert_eq!(original_id, id);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn creating_a_new_session_with_expiry() -> Result {
        let store = test_store().await;
        let mut session = Session::new();
        session.expire_in(Duration::from_secs(1));
        session.insert("key", "value")?;
        let cloned = session.clone();

        let cookie_value = store.store_session(session).await?.unwrap();

        let (id, expires, serialized, count): (String, Option<DateTime>, String, i64) =
            sqlx::query_as("select id, expires, session, (select count(*) from async_sessions) from async_sessions")
                .fetch_one(&mut *store.connection().await?)
                .await?;

        assert_eq!(1, count);
        assert_eq!(id, cloned.id());
        assert!(expires.unwrap() > DateTime::now_utc());

        let deserialized_session: Session = serde_json::from_str(&serialized)?;
        assert_eq!(cloned.id(), deserialized_session.id());
        assert_eq!("value", &deserialized_session.get::<String>("key").unwrap());

        let loaded_session = store.load_session(cookie_value.clone()).await?.unwrap();
        assert_eq!(cloned.id(), loaded_session.id());
        assert_eq!("value", &loaded_session.get::<String>("key").unwrap());

        assert!(!loaded_session.is_expired());

        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(None, store.load_session(cookie_value).await?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn destroying_a_single_session() -> Result {
        let store = test_store().await;
        for _ in 0..3i8 {
            store.store_session(Session::new()).await?;
        }

        let cookie = store.store_session(Session::new()).await?.unwrap();
        assert_eq!(4, store.count().await?);
        let session = store.load_session(cookie.clone()).await?.unwrap();
        store.destroy_session(session.clone()).await.unwrap();
        assert_eq!(None, store.load_session(cookie).await?);
        assert_eq!(3, store.count().await?);

        // // attempting to destroy the session again is not an error
        assert!(store.destroy_session(session).await.is_ok());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn clearing_the_whole_store() -> Result {
        let store = test_store().await;
        for _ in 0..3i8 {
            store.store_session(Session::new()).await?;
        }

        assert_eq!(3, store.count().await?);
        store.clear_store().await.unwrap();
        assert_eq!(0, store.count().await?);

        Ok(())
    }
}

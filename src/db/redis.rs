use crate::db::DatabaseProvider;
use crate::utils::gazette::Gazette;
use anyhow::{anyhow, Result};
use futures::StreamExt;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::Serialize;

#[derive(Clone)]
pub struct RedisProvider {
    connection: MultiplexedConnection,
}

impl RedisProvider {
    const FLAGGED_PREFIX: &'static str = "flagged:";
    const DISCARDED_PREFIX: &'static str = "discarded:";

    pub async fn new() -> Result<Self> {
        let connection = Self::connect().await?;
        Ok(Self { connection })
    }

    async fn check_key_exists(
        connection: &mut MultiplexedConnection,
        prefix: &str,
        id: &str,
    ) -> Result<bool> {
        connection
            .exists::<String, bool>(format!("{prefix}{id}"))
            .await
            .map_err(|e| anyhow!("Failed to check Redis key: {e}"))
    }
}

#[async_trait::async_trait]
impl DatabaseProvider for RedisProvider {
    type DBResult = MultiplexedConnection;

    async fn connect() -> Result<Self::DBResult> {
        let redis_url: String = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string())
            .parse()
            .expect("Failed to get REDIS_URL");

        let insecure = std::env::var("REDIS_INSECURE_TLS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let final_url = if insecure {
            format!("{redis_url}#insecure")
        } else {
            redis_url
        };

        let client = redis::Client::open(final_url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        Ok(connection)
    }

    async fn has_entry(&self, id: &str) -> Result<bool> {
        let mut conn = self.connection.clone();

        let flagged_exists =
            Self::check_key_exists(&mut conn, Self::FLAGGED_PREFIX, id).await?;
        let discarded_exists =
            Self::check_key_exists(&mut conn, Self::DISCARDED_PREFIX, id).await?;

        Ok(flagged_exists || discarded_exists)
    }

    async fn create_entry(&self, id: &str, value: &Gazette) -> Result<bool> {
        let mut conn = self.connection.clone();
        conn.set::<&str, &Gazette, ()>(id, value).await?;
        Ok(true)
    }

    async fn fetch_entries(&self) -> Result<Vec<Gazette>> {
        let mut conn = self.connection.clone();
        let mut cursor: u64 = 0;
        let mut keys: Vec<String> = Vec::new();

        // Paginate through keys using non-blocking SCAN with an explicit cursor
        loop {
            let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("flagged:*")
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| anyhow!("Failed to SCAN keys: {e}"))?;

            keys.extend(batch);
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        let mut gazettes: Vec<Gazette> = Vec::new();
        for key in keys {
            if let Ok(gazette) = conn.get::<&str, Gazette>(&key).await {
                gazettes.push(gazette);
            }
        }

        gazettes.sort_by(|a, b| b.uri.cmp(&a.uri));
        Ok(gazettes)
    }
}
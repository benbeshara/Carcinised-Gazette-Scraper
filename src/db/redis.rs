use anyhow::{anyhow, Result};
use redis::{Connection, TypedCommands};
use crate::db::db::DatabaseProvider;
use crate::utils::gazette::Gazette;

pub struct RedisProvider;

pub struct RedisConnection;

#[async_trait::async_trait]
impl DatabaseProvider for RedisProvider {
    type DBResult = Connection;

    async fn connect() -> Result<Self::DBResult> {
        let redis_url: String = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string())
            .parse()
            .expect("Failed to get REDIS_URL");
        let redis_url_insecure = redis_url + "#insecure";
        let redis = redis::Client::open(redis_url_insecure)?;
        let redis_client = redis.get_connection()?;

        Ok(redis_client)
    }

    async fn has_entry(&self, id: &str) -> Result<bool> {
        if let Ok(mut connection) = Self::connect().await {
            let result = !(!connection
                .exists::<String>(format!("flagged:{}", id))? && !connection
                .exists::<String>(format!("discarded:{}", id))?);
            return Ok(result);
        }
        Err(anyhow!("Could not check entry"))
    }

    async fn create_entry(&self, id: &str, value: &Gazette) -> Result<bool> {
        if let Ok(mut connection) = Self::connect().await {
            connection.set(id, value)?;
            return Ok(true)
        }
        Err(anyhow!("Could not create entry"))
    }

    async fn fetch_entries(&self) -> Result<Vec<Gazette>> {
        // We don't want to use the typed commands here because we're deserialising them directly into
        // gazettes; hence this section looks a little funky
        use redis::Commands;

        if let Ok(mut connection) = Self::connect().await {
            let mut gazettes: Vec<Gazette> = vec![];
            if let Ok(keys) = Commands::keys::<&str, Vec<String>>(&mut connection, "flagged:*"){
                let _ = keys.into_iter().map(|key| {
                    if let Ok(gazette) = Commands::get::<&String, Gazette>(&mut connection, &key){
                        gazettes.push(gazette)
                    }
                }).collect::<Vec<_>>();
            };

            gazettes.sort_by(|a, b| b.uri.cmp(&a.uri));

            return Ok(gazettes);
        };

        Err(anyhow!("Could not fetch entries"))
    }
}

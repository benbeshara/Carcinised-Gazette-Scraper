use crate::{db::DatabaseProvider, utils::gazette::Gazette};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MockDatabaseProvider {
    storage: Arc<RwLock<HashMap<String, Gazette>>>,
}

impl MockDatabaseProvider {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl DatabaseProvider for MockDatabaseProvider {
    type DBResult = Self;

    async fn connect() -> Result<Self::DBResult> {
        Ok(MockDatabaseProvider::new())
    }

    async fn has_entry(&self, id: &str) -> Result<bool> {
        let storage = self.storage.read().await;
        Ok(storage.contains_key(id))
    }

    async fn create_entry(&self, id: &str, value: &Gazette) -> Result<bool> {
        let mut storage = self.storage.write().await;
        storage.insert(id.to_string(), value.clone());
        Ok(true)
    }

    async fn fetch_entries(&self) -> Result<Vec<Gazette>> {
        let storage = self.storage.read().await;
        Ok(storage.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_database() {
        let db = MockDatabaseProvider::new();
        let gazette = Gazette::default();

        let result = db.create_entry("test_id", &gazette).await.unwrap();
        assert!(result);

        assert!(db.has_entry("test_id").await.unwrap());
        assert!(!db.has_entry("non_existent").await.unwrap());

        let entries = db.fetch_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }
}

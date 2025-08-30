use crate::utils::gazette::Gazette;
use anyhow::Result;

#[async_trait::async_trait]
pub trait DatabaseProvider {
    type DBResult;
    async fn connect() -> Result<Self::DBResult>;
    async fn has_entry(&self, id: &str) -> Result<bool>;
    async fn create_entry(&self, id: &str, value: &Gazette) -> Result<bool>;
    async fn fetch_entries(&self) -> Result<Vec<Gazette>>;
}

pub struct DatabaseConnection<T>
where
    T: DatabaseProvider,
{
    pub provider: T,
}

impl<T> DatabaseConnection<T>
where
    T: DatabaseProvider,
{
    pub async fn has_entry(&self, id: &str) -> Result<bool> {
        T::has_entry(&self.provider, id).await
    }

    pub async fn create_entry(&self, id: &str, value: &Gazette) -> Result<bool> {
        T::create_entry(&self.provider, id, value).await
    }

    pub async fn fetch_entries(&self) -> Result<Vec<Gazette>> {
        T::fetch_entries(&self.provider).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::mock::MockDatabaseProvider;

    #[tokio::test]
    async fn test_database_connection() {
        let provider = MockDatabaseProvider::new();
        let connection = DatabaseConnection { provider };

        let gazette = Gazette::default();
        let id = "test123";

        assert!(connection.create_entry(id, &gazette).await.unwrap());
        assert!(connection.has_entry(id).await.unwrap());

        let entries = connection.fetch_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
    }
}

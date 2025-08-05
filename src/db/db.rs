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
    async fn connect() -> Result<T::DBResult> {
        T::connect().await
    }

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

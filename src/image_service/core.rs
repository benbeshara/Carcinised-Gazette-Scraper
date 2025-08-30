use anyhow::{anyhow, Result};

pub struct Image<T>
where
    T: ImageService + Copy,
{
    pub filename: String,
    pub data: Vec<u8>,
    pub service: T,
}

#[async_trait::async_trait]
pub trait ImageService {
    async fn upload(&self, filename: String, data: Vec<u8>) -> Result<Option<String>>;
}

impl<T> Image<T>
where
    T: ImageService + Copy,
{
    pub async fn upload(&self) -> Result<Option<String>> {
        self.service
            .upload(self.filename.clone(), self.data.clone())
            .await
    }
}

#[async_trait::async_trait]
impl ImageService for () {
    async fn upload(&self, _filename: String, _data: Vec<u8>) -> Result<Option<String>> {
        Err(anyhow!("This should never happen"))
    }
}

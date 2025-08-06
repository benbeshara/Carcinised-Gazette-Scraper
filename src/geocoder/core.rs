use crate::utils::maptypes::GeoPosition;
use anyhow::Result;

#[async_trait::async_trait]
pub trait GeocoderProvider {
    async fn geocode(&self, input: &str) -> Result<GeoPosition>;
}

#[derive(Clone, Debug)]
pub struct GeocoderRequest<T>
where
    T: GeocoderProvider + Copy,
{
    pub input: String,
    pub service: T,
}

impl<T> GeocoderRequest<T>
where
    T: GeocoderProvider + Copy,
{
    pub async fn geocode(&self) -> Result<GeoPosition> {
        self.service.geocode(&self.input).await
    }
}

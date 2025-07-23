use crate::utils::maptypes::GeoPosition;
use crate::GenericError;

#[async_trait::async_trait]
pub trait GeocoderProvider {
    async fn geocode(&self, input: &String) -> Result<GeoPosition, GenericError>;
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
    pub async fn geocode(&self) -> Result<GeoPosition, GenericError> {
        self.service.geocode(&self.input).await
    }
}

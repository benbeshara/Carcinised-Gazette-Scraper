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

#[cfg(test)]
mod test {
    use super::*;
    use crate::geocoder::mock::MockGeocoderProvider;

    #[tokio::test]
    async fn test_your_application() {
        let geocoder = MockGeocoderProvider::new();

        // Test with a known location
        let request = GeocoderRequest {
            input: "Tokyo".to_string(),
            service: geocoder,
        };

        let position = request.geocode().await.unwrap();
        assert_eq!(position.latitude, 35.6762);
        assert_eq!(position.longitude, 139.6503);
    }
}

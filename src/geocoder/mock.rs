use crate::{geocoder::GeocoderProvider, utils::maptypes::GeoPosition};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
pub struct MockGeocoderProvider;

impl MockGeocoderProvider {
    fn get_mock_responses() -> HashMap<&'static str, GeoPosition> {
        let mut responses = HashMap::new();
        responses.insert(
            "New York",
            GeoPosition {
                latitude: 40.7128,
                longitude: -74.0060,
            },
        );
        responses.insert(
            "London",
            GeoPosition {
                latitude: 51.5074,
                longitude: -0.1278,
            },
        );
        responses.insert(
            "Tokyo",
            GeoPosition {
                latitude: 35.6762,
                longitude: 139.6503,
            },
        );
        responses
    }
}

#[async_trait::async_trait]
impl GeocoderProvider for MockGeocoderProvider {
    async fn geocode(&self, input: &str) -> Result<GeoPosition> {
        let responses = Self::get_mock_responses();

        Ok(responses.get(input).cloned().unwrap_or(GeoPosition {
            latitude: 0.0,
            longitude: 0.0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geocoder::GeocoderRequest;

    #[tokio::test]
    async fn test_mock_geocoder() {
        let geocoder = MockGeocoderProvider {};

        // Test known location
        let ny_position = geocoder.geocode("New York").await.unwrap();
        assert_eq!(ny_position.latitude, 40.7128);
        assert_eq!(ny_position.longitude, -74.0060);

        // Test unknown location (should return default position)
        let unknown_position = geocoder.geocode("Unknown Location").await.unwrap();
        assert_eq!(unknown_position.latitude, 0.0);
        assert_eq!(unknown_position.longitude, 0.0);
    }

    #[tokio::test]
    async fn test_geocoder_request() {
        let service = MockGeocoderProvider {};
        let request = GeocoderRequest {
            input: "London".to_string(),
            service,
        };

        let position = request.geocode().await.unwrap();
        assert_eq!(position.latitude, 51.5074);
        assert_eq!(position.longitude, -0.1278);
    }
}

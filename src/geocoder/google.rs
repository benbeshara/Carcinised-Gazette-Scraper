use crate::geocoder::GeocoderProvider;
use crate::utils::maptypes::GeoPosition;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Clone, Copy, Debug)]
pub struct GoogleGeocoderProvider;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GoogleGeocoderPosition {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GoogleGeocoderGeometry {
    location: GoogleGeocoderPosition,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GoogleGeocoderResult {
    geometry: GoogleGeocoderGeometry,
    types: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct GoogleGeocoderResponse {
    results: Vec<GoogleGeocoderResult>,
}

#[async_trait::async_trait]
impl GeocoderProvider for GoogleGeocoderProvider {
    async fn geocode(&self, input: &str, area: &str) -> Result<GeoPosition> {
        if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            let client = Client::new();
            let input = format!("{input}, {area}, VICTORIA, AUSTRALIA");
            let request = format!("https://maps.googleapis.com/maps/api/geocode/json?key={api_key}&api-version=1.0&language=en-AU&region=AU&address={input}");
            let res = client.get(request).send().await?;

            let body = res.json::<GoogleGeocoderResponse>().await?;
            return body.into();
        }
        Err(anyhow!("Geocoding failed"))
    }
}

impl From<GoogleGeocoderResponse> for Result<GeoPosition> {
    fn from(value: GoogleGeocoderResponse) -> Self {
        // The google geocoder seems to be good enough to get a hit on the first result,
        // but I'm not sure how i'd filter them to find relevant ones if it doesn't
        if let Some(result) = value.results.first() {
            Ok((&result.clone().geometry.location).into())
        } else {
            Err(anyhow!("Geocoder returned no results"))
        }
    }
}

impl From<&GoogleGeocoderPosition> for GeoPosition {
    fn from(value: &GoogleGeocoderPosition) -> Self {
        Self {
            latitude: value.lat,
            longitude: value.lng,
        }
    }
}

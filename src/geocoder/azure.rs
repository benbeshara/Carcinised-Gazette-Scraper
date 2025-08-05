use crate::geocoder::geocoder::GeocoderProvider;
use crate::utils::maptypes::GeoPosition;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use anyhow::{anyhow, Result};

#[derive(Clone, Copy, Debug)]
pub struct AzureGeocoderProvider;

#[derive(Deserialize, Copy, Clone, Debug)]
pub struct AzureGeocoderPosition {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Deserialize)]
struct AzureGeocoderResult {
    id: String,
    r#type: String,
    position: AzureGeocoderPosition,
}

#[derive(Deserialize)]
struct AzureGeocoderResponse {
    results: Vec<AzureGeocoderResult>,
}

#[async_trait::async_trait]
impl GeocoderProvider for AzureGeocoderProvider {
    async fn geocode(&self, input: &String) -> Result<GeoPosition> {
        if let Ok(api_key) = env::var("AZURE_API_KEY") {
            let client = Client::new();
            let req = format!("{}, VICTORIA, AUSTRALIA", input);
            let request = format!("https://atlas.microsoft.com/search/address/json?&subscription-key={api_key}&api-version=1.0&language=en-AU&countrySet=AU&query={req}");
            let res = client.get(request).send().await?;

            let body = res.json::<AzureGeocoderResponse>().await?;
            return body.into();
        }
        Err(anyhow!("Invalid Azure API key provided"))
    }
}

impl From<AzureGeocoderResponse> for Result<GeoPosition> {
    fn from(value: AzureGeocoderResponse) -> Self {
        // We are *likely* to want a cross-street value
        for result in &value.results {
            if result.r#type == "Cross Street" {
                return Ok((&result.position).into());
            }
        }

        // If there is none, the first result is all we've got
        if let Some(result) = value.results.first() {
            Ok((&result.position).into())
        } else {
            Err(anyhow!("Geocoder returned no results"))
        }
    }
}

impl From<&AzureGeocoderPosition> for GeoPosition {
    fn from(value: &AzureGeocoderPosition) -> Self {
        Self {
            latitude: value.lat,
            longitude: value.lon,
        }
    }
}

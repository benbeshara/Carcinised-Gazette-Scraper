use std::env;
use reqwest::Client;
use serde::Deserialize;
use crate::GenericError;
use crate::geocoder::geoposition::{GeoPosition, GeocoderRequest};

#[derive(Deserialize, Clone, Debug)]
pub struct GoogleGeocoderPosition {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Deserialize, Clone, Debug)]
struct GoogleGeocoderGeometry {
    location: GoogleGeocoderPosition,
}

#[derive(Deserialize, Clone, Debug)]
struct GoogleGeocoderResult {
    geometry: GoogleGeocoderGeometry,
    types: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct GoogleGeocoderResponse {
    results: Vec<GoogleGeocoderResult>,
}

pub trait GoogleGeocoderRequest {
    async fn google_geocoder_request(&self) -> Result<GoogleGeocoderPosition, GenericError>;
}

impl From<GoogleGeocoderResponse> for Result<GoogleGeocoderPosition, GenericError> {
    fn from(value: GoogleGeocoderResponse) -> Self {
        // The google geocoder seems to be good enough to get a hit on the first result,
        // but I'm not sure how i'd filter them to find relevant ones if it doesn't
        if let Some(result) = value.results.first() {
            Ok(result.clone().geometry.location)
        } else {
            Err("Geocoder returned no results".into())
        }
    }
}

impl GoogleGeocoderRequest for GeocoderRequest {
    async fn google_geocoder_request(&self) -> Result<GoogleGeocoderPosition, GenericError> {
        if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            let client = Client::new();
            let req = format!("{}, VICTORIA, AUSTRALIA", self.request);
            let request = format!("https://maps.googleapis.com/maps/api/geocode/json?key={api_key}&api-version=1.0&language=en-AU&region=AU&address={req}");
            let res = client
                .get(request)
                .send()
                .await?;

            let body = res.json::<GoogleGeocoderResponse>().await?;
            return body.into();
        }
        Err("Geocoding failed".into())
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
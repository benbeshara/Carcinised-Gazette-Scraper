use std::env;
use std::fmt::{Debug, Display, Formatter};
use itertools::Itertools;
use reqwest::Client;
use crate::GenericError;
use serde_json::json;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
struct OpenAIMessage {
    content: String,
}

#[derive(Deserialize, Clone, Debug)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize, Debug)]
struct OpenAIResponse {
    id: String,
    choices: Vec<OpenAIChoice>,
}

impl From<OpenAIResponse> for Vec<String> {
    fn from(value: OpenAIResponse) -> Self {
        let mut ret = Vec::new();
        for v in value.choices[0].clone().message.content.split(",") {
            ret.push(v.to_string());
        };
        ret
    }
}

pub async fn openai_request(req: String) -> Result<Vec<String>, GenericError> {
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        let client = Client::new();
        let res = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&json!({
                "model": "gpt-4o-mini",
                "messages": [
                    {"role": "system", "content": "You are a service that receives a paragraph describing a physical area on a map, in the form of a polygon. You are to produce a list of landmarks that are suitable for forward geocoding from this paragraph, to be fed into a geocoder to produce latitudes and longitudes that can reproduce this polygon. When you are given terms such as \"Intersection of\" it means that you should specify that as the corner of where those places meet. When something is \"PLACE_NAME Rail\" it refers to railway station - this should be specified, for example \"Werribee Rail\" would be \"Werribee Railway Station\". If something is a \"precinct\" you can ignore it. If one of the locations is a river and a bridge, just tell me the bridge. Annotations in parenthesis can be ignored, even if they contain rules mentioned previously to this one. You will return the information as a comma separated list and return no other information. Ensure that the list you give me has the locations in the same order tha the paragtaph gives them."},
                    {"role": "user", "content": req}
                ],
                "temperature": 0.7
            }))
            .send()
            .await?;

        if let Ok(response) = res.json::<OpenAIResponse>().await {
            return Ok(response.into());
        }
    }
    Err("Parsing locations failed".into())
}

#[derive(Deserialize, Copy, Clone, Debug)]
pub struct AzureGeocoderPosition {
    lat: f64,
    lon: f64,
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

impl From<AzureGeocoderResponse> for Result<AzureGeocoderPosition, GenericError> {
    fn from(value: AzureGeocoderResponse) -> Self {
        // We are *likely* to want a cross-street value
        for result in &value.results {
            if result.r#type == "Cross Street" {
                return Ok(result.position);
            }
        }

        // If there is none, the first result is all we've got
        if let Some(result) = value.results.first() {
            Ok(result.position)
        } else {
            Err("Geocoder returned no results".into())
        }
    }
}

pub async fn azure_geocoder_request(req: &str) -> Result<AzureGeocoderPosition, GenericError> {
    if let Ok(api_key) = env::var("AZURE_API_KEY") {
        let client = Client::new();
        let req = format!("{}, MELBOURNE, AUSTRALIA", req);
        let request = format!("https://atlas.microsoft.com/search/address/json?&subscription-key={api_key}&api-version=1.0&language=en-AU&countrySet=AU&query={req}");
        let res = client
            .get(request)
            .send()
            .await?;

        let body = res.json::<AzureGeocoderResponse>().await?;
        return body.into();
    }
    Err("Geocoding failed".into())
}

#[derive(Deserialize, Clone, Debug)]
pub struct GoogleGeocoderPosition {
    lat: f64,
    lng: f64,
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

pub async fn google_geocoder_request(req: &str) -> Result<GoogleGeocoderPosition, GenericError> {
    if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
        let client = Client::new();
        let req = format!("{}, MELBOURNE, AUSTRALIA", req);
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

pub async fn map_points_from_desc(req: String) -> Vec<GoogleGeocoderPosition> {
    let mut response: Vec<GoogleGeocoderPosition> = vec![];
    if let Ok(locs) = openai_request(req).await {
        for loc in locs {
            if let Ok(result) = google_geocoder_request(loc.as_str()).await {
                response.push(result);
            }
        }
    }
    response
}

#[derive(Clone, Debug)]
pub struct GeoPosition {
    latitude: f64,
    longitude: f64,
}

impl From<&AzureGeocoderPosition> for GeoPosition {
    fn from(value: &AzureGeocoderPosition) -> Self {
        Self {
            latitude: value.lat,
            longitude: value.lon,
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

impl Display for GeoPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}, {:?}]", self.latitude, self.longitude)
    }
}

pub struct MapPolygon {
    pub data: Vec<GeoPosition>,
}

impl From<MapPolygon> for String {
    fn from(value: MapPolygon) -> String {
        let mut output = String::new();
        output = value.data.iter().map(|f| f.to_string()).join(", ");
        format!("[{}]", output)
    }
}
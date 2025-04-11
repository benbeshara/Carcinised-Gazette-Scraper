use std::env;
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

#[derive(Deserialize)]
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
                    {"role": "system", "content": "You are a service that receives a paragraphs describing a physical area on a map, in the form of a polygon. You are to produce a list of landmarks that are suitable for forward geocoding from this paragraph, to be fed into a geocoder to produce latitudes and longitudes that can reproduce this polygon. When you are given terms such as \"Intersection of\" it means that you should specify that as the corner of where those places meet. If something is a 'precinct' you can ignore it. You will return the information as a comma separated list and return no other information."},
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
    position: AzureGeocoderPosition,
}

#[derive(Deserialize)]
struct AzureGeocoderResponse {
    results: Vec<AzureGeocoderResult>,
}

pub async fn azure_geocoder_request(req: &str) -> Result<AzureGeocoderPosition, GenericError> {
    if let Ok(api_key) = env::var("AZURE_API_KEY") {
        let client = Client::new();
        let request = format!("https://atlas.microsoft.com/search/address/json?&subscription-key={api_key}&api-version=1.0&language=en-US&query={req}");
        let res = client
            .get(request)
            .send()
            .await?;

        let body = res.json::<AzureGeocoderResponse>().await?;
        if let Some(position) = body.results.first() {
            return Ok(position.position);
        }
    }
    Err("Geocoding failed".into())
}

pub async fn map_points_from_desc(req: String) -> Vec<AzureGeocoderPosition> {
    let mut response: Vec<AzureGeocoderPosition> = vec![];
    if let Ok(locs) = openai_request(req).await {
        for loc in locs {
            if let Ok(result) = azure_geocoder_request(loc.as_str()).await {
                response.push(result);
            }
        }
    }
    response
}
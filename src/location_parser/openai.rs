use crate::location_parser::LocationParserService;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OpenAIMessage {
    content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIResponse {
    id: String,
    choices: Vec<OpenAIChoice>,
}

impl From<OpenAIResponse> for Vec<String> {
    fn from(value: OpenAIResponse) -> Self {
        let mut ret = Vec::new();
        for v in value.choices[0].clone().message.content.split('\n') {
            ret.push(v.to_string());
        }
        ret
    }
}

#[derive(Copy, Clone)]
pub struct OpenAI;

#[async_trait::async_trait]
impl LocationParserService for OpenAI {
    async fn parse_locations(&self, locations: String) -> Result<Vec<String>> {
        let api_key = env::var("OPENAI_API_KEY")?;
        let client = Client::new();
        let res = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a location extraction service for Victorian Government Gazettes. Extract the boundary corner points/intersections forming the designated area polygon following these rules:\n\
                    1. Return only a list of intersections or landmark points in perimeter order (clockwise or counter-clockwise), one per line.\n\
                    2. When the text lists boundaries by direction (e.g., 'north to the intersection of X and Y', 'east to the intersection of Y and Z'), extract each intersection: 'Corner of X and Y'.\n\
                    3. If a street segment is given (e.g. 'bordered by Street A, Street B, Street C'), identify the successive street intersections to form a closed polygon.\n\
                    4. Always format intersections as: 'Corner of [Street 1] and [Street 2]'.\n\
                    5. For railway stations: format as '[Name] Railway Station'.\n\
                    6. Include the specific suburb or locality name on each line if mentioned in the text (e.g., 'Corner of Derrimut Road and Barber Drive, Hoppers Crossing').\n\
                    7. Do not include extra explanations, bullet points, numbering, or empty lines."
                },
                {"role": "user", "content": locations}
            ],
            "temperature": 0.0,
            "response_format": { "type": "text" },
            "max_tokens": 1000
        }))
            .send()
            .await?;

        res.json::<OpenAIResponse>()
            .await
            .map(Into::into)
            .map_err(Into::into)
    }
}

#[tokio::test]
async fn test_location_parser() {
    use crate::location_parser::LocationParser;

    let para = "Southern Cross Railway Station – all public places within the
area bound by and including, the intersection of Southern Cross Railway Station and surrounding
area encompassing Collins Street, Spencer Street including Bourke Street Pedestrian Bridge,
Lonsdale Street west of Spencer Street, La Trobe Street and Wurundjeri Way.";
    let s = LocationParser {
        provider: OpenAI,
        locations: para.to_string(),
    };
    let r = s.parse_locations().await.unwrap();

    assert!(r.len() == 7);
}

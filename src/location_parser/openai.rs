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
        for v in value.choices[0].clone().message.content.split("\n") {
            ret.push(v.to_string());
        }
        ret
    }
}

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
            "model": "gpt-4",
            "messages": [
                {
                        "role": "system",
                        "content": "You are a location extraction service. Extract locations from the input text following these rules:\n\
                        1. Return only a list of locations, one per line\n\
                        2. All locations should try to be an intersection\n\
                        3. For intersections: Format as 'Corner of X and Y Streets'\n\
                        4. For railways: Convert 'X Rail' to 'X Railway Station'\n\
                        5. For shopping areas: Include 'Shopping Centre' in full\n\
                        6. For CBDs: Keep format 'X CBD'\n\
                        7. Ignore content in parentheses\n\
                        8. For bridges and rivers, prefer the bridge name\n\
                        9. Remove any duplicate locations\n\
                        10. Maintain the order of locations as they appear in the text\n\
                        11. Include full location names with relevant context\n\
                        12. Locations mentioning both 'Swanston Street' and 'Yarra Walk' mean the Princes Bridge instead\n\
                        13. The 'Bourke Street Pedestrian Bridge' should be the 'Southern Cross Bridge'\n\
                        14. If there are no clear intersections specified, assume all locations are listed clockwise and create an the intersections like a polygon\n\
                        Do not include any additional text, explanations, or headers in the output."
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
            .map(|r| r.into())
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

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
                        2. For intersections: Format as 'Corner of X and Y Streets'\n\
                        3. For railways: Convert 'X Rail' to 'X Railway Station'\n\
                        4. For shopping areas: Include 'Shopping Centre' in full\n\
                        5. For CBDs: Keep format 'X CBD'\n\
                        6. Ignore content in parentheses\n\
                        7. For bridges and rivers, prefer the bridge name\n\
                        8. Remove any duplicate locations\n\
                        9. Maintain the order of locations as they appear in the text\n\
                        10. Include full location names with relevant context\n\
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

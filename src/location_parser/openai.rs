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
            "model": "gpt-4o-mini",
            "messages": [
                // {"role": "system", "content": "You are a service that receives a paragraph describing a physical area on a map, in the form of a polygon. You are to produce a list of landmarks that are suitable for forward geocoding from this paragraph, to be fed into a geocoder to produce latitudes and longitudes that can reproduce this polygon. When you are given terms such as \"Intersection of\" it means that you should specify that as the corner of where those places meet. When something is \"PLACE_NAME Rail\" it refers to railway station - this should be specified, for example \"Werribee Rail\" would be \"Werribee Railway Station\". If something is a \"precinct\" you can ignore it. If one of the locations is a river and a bridge, just tell me the bridge. Annotations in parenthesis can be ignored, even if they contain rules mentioned previously to this one. You will return the information as a comma separated list and return no other information. Ensure that the list you give me has the locations in the same order tha the paragtaph gives them."},
                {"role": "system", "content": "You will be given a paragraph of text. You need to return a list of streets and landmarks present in it. It would be very helpful if you could figure out which streets connect, so we have 'corner of x and x streets'. If you have a complete list of these, don't include individual streets, just the pairs. If there is something that looks like a suburb (ie, {SOMEWHERE} CBD, or {SOMEWHERE} train station, or the area in {SOMEWHERE}) in the paragraph, append this to the street pair name. If {SOMEWHERE} shopping centres is mentioned in the paragraph, append 'near {SOMEWHERE} shopping centre}. Specify it in a machine-parsable newline delimited list. Do not add any extra text - ne headers or descriptors at all. This is supposed to be passed to a geocoder so we can get coordinates to draw a polygon on a map. If you do well, I'll get a cookie and I'll share it with you."},
                {"role": "user", "content": locations}
            ],
            "temperature": 0.1
        }))
            .send()
            .await?;

        res.json::<OpenAIResponse>()
            .await
            .map(|r| r.into())
            .map_err(Into::into)
    }
}

mod web;

use std::{collections::HashMap, error::Error};

use redis::{from_redis_value, Commands, Connection, ErrorKind, FromRedisValue, RedisError};
use select::document::Document;
use select::predicate::Name;
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};
use tokio::task::JoinError;

type GenericError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct Gazette {
    title: std::string::String,
    uri: std::string::String,
}

impl FromRedisValue for Gazette {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Gazette> {
        if let Ok(v) = from_redis_value::<HashMap<std::string::String, std::string::String>>(v) {
            if let (Some(title), Some(uri)) = (v.get("title"), v.get("uri")) {
                return Ok(Gazette {
                    title: title.to_owned(),
                    uri: uri.to_owned(),
                });
            }
        }
        Err(RedisError::from((
            ErrorKind::ParseError,
            "Could not parse Gazette from redis result",
        )))
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(async move {update_pdfs().await});

    crate::web::start_server().await;
}

fn get_redis_connection() -> Result<Connection, GenericError> {
    let redis_url: String = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string()) // Get the port as a string or default to "3000"
        .parse() // Parse the port string into a u16
        .expect("Failed to get REDIS_URL");
    let redis_url_insecure = redis_url + "#insecure";
    let redis = redis::Client::open(redis_url_insecure)?;
    let redis_client = redis.get_connection()?;

    Ok(redis_client)
}

pub async fn update_pdfs() -> Result<(), GenericError> {
    let url = "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm";
    let base_uri = "http://www.gazette.vic.gov.au";

    match parse_webpage(url, base_uri).await {
        Ok(res) => {println!("PDF Update succeeded"); Ok(res)},
        Err(e) => {println!("PDF Update failed: {:?}", e.to_string()); Err(e)},
    }
}

async fn make_hash(key: &str) -> String {
    let mut hasher: CoreWrapper<Sha1Core> = Sha1::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

async fn entry_is_in_redis(entry: String) -> Result<bool, GenericError> {
    let mut redis_client = get_redis_connection()?;

    Ok(redis_client
        .exists::<std::string::String, std::string::String>(format!("flagged:{}", entry))
        .unwrap()
        .parse::<isize>()
        .unwrap()
        != 0
        || redis_client
            .exists::<std::string::String, std::string::String>(format!("discarded:{}", entry))
            .unwrap()
            .parse::<isize>()
            .unwrap()
            != 0)
}

async fn push_to_redis(uri: &str, title: &str, condition: &str) -> Result<(), GenericError> {
    let hash = make_hash(uri).await;
    let mut redis_client = get_redis_connection()?;

    redis_client
        .hset::<String, &str, &str, String>(format!("{}:{}", condition, hash), "title", title)
        .unwrap();
    redis_client
        .hset::<String, &str, &str, String>(format!("{}:{}", condition, hash), "uri", uri)
        .unwrap();

    Ok(())
}

async fn parse_webpage(uri: &str, base_uri: &str) -> Result<(), GenericError> {
    let res = reqwest::get(uri).await?.text().await?;

    let pdf_list: Vec<(String, String)> = Document::from(res.as_str())
        .find(Name("a"))
        .filter(|e| e.attr("href").is_some())
        .map(|e| {
            (
                e.inner_html(),
                base_uri.to_owned() + e.attr("href").unwrap(),
            )
        })
        .filter(|n| n.1.contains(".pdf"))
        .collect();

    let chunks = pdf_list.chunks(16);

    chunks.for_each(|chunk| {
        let _ = filter_gazettes(chunk.to_vec());
    });

    Ok(())
}

async fn filter_gazettes(uri_list: Vec<(String, String)>) -> Result<Vec<String>, GenericError> {
    let uri_list = uri_list.clone();
    let tasks: Vec<_> = uri_list
        .into_iter()
        .map(|(title, uri)| {
            tokio::spawn(async move {
                let hash = make_hash(&uri).await;

                if let Ok(exists) = entry_is_in_redis(hash.to_string()).await {
                    if exists {
                        return None;
                    }
                } else {
                    return None;
                }

                if let Ok(req) = reqwest::get(&uri).await {
                    if let Ok(bytes) = req.bytes().await {
                        if let Ok(pdf) = lopdf::Document::load_mem(&bytes) {
                            let page_zero: u32 = 1;
                            if let Ok(page_text) = pdf.extract_text(&[page_zero]) {
                                if page_text.contains("Control of Weapons Act 1990") {
                                    let _ = push_to_redis(&uri, &title, "flagged").await;
                                    return Some(uri);
                                } else {
                                    let _ = push_to_redis(&uri, "", "discarded").await;
                                }
                            }
                        }
                    }
                }
                None
            })
        })
        .collect();

    let res: Vec<Result<Option<String>, JoinError>> = futures::future::join_all(tasks).await;

    let res: Vec<String> = res
        .into_iter()
        .filter_map(|item| item.ok())
        .filter(|item| item.is_some())
        .map(|item| item.unwrap().to_owned())
        .collect();

    Ok(res)
}

pub async fn retrieve_gazettes_from_redis() -> Result<Vec<Gazette>, GenericError> {
    let mut redis_client = get_redis_connection()?;

    let mut gazettes: Vec<Gazette> = vec![];

    let keys = redis_client
        .keys::<std::string::String, Vec<std::string::String>>("flagged:*".to_string())?;

    for key in keys {
        if let Ok(res) = redis_client.hgetall::<std::string::String, Gazette>(key) {
            gazettes.push(res);
        }
    }

    gazettes.sort_by(|a, b| b.uri.cmp(&a.uri));

    Ok(gazettes)
}

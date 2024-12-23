use std::{collections::HashMap, error::Error};

use redis::{from_redis_value, Commands, Connection, ErrorKind, FromRedisValue, RedisError};
use select::document::Document;
use select::predicate::Name;
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};
use tokio::task::JoinError;

#[derive(Debug, Default)]
#[allow(dead_code)]
struct Gazette {
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
    match update_pdfs().await {
        Ok(_) => println!("PDF Update succeeded"),
        Err(e) => println!("PDF Update failed: {:?}", e.to_string()),
    }
    match retrieve_gazettes_from_redis().await {
        Ok(_) => println!("Gazettes retrieved from redis"),
        Err(e) => println!("Gazette retrieval failed: {:?}", e.to_string()),
    }
}

fn get_redis_connection() -> Result<Connection, Box<dyn Error + Send + Sync + 'static>> {
    let redis = redis::Client::open("redis://localhost/")?;
    let redis_client = redis.get_connection()?;

    Ok(redis_client)
}

async fn update_pdfs() -> Result<Vec<std::string::String>, Box<dyn Error + Send + Sync + 'static>> {
    let url = "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm";
    let base_uri = "http://www.gazette.vic.gov.au";
    parse_webpage(url, base_uri).await
}

async fn make_hash(key: &str) -> String {
    let mut hasher: CoreWrapper<Sha1Core> = Sha1::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

async fn entry_is_in_redis(entry: String) -> Result<bool, Box<dyn Error + Send + Sync + 'static>> {
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

async fn push_to_redis(
    uri: &str,
    title: &str,
    condition: &str,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
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

async fn parse_webpage(
    uri: &str,
    base_uri: &str,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
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

    let pdf_list = filter_gazettes(pdf_list);

    pdf_list.await
}

#[allow(unused_must_use)]
async fn filter_gazettes(
    uri_list: Vec<(String, String)>,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
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
                                    push_to_redis(&uri, &title, "flagged").await;
                                    return Some(uri);
                                } else {
                                    push_to_redis(&uri, "", "discarded").await;
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

async fn retrieve_gazettes_from_redis(
) -> Result<Vec<Gazette>, Box<dyn Error + Send + Sync + 'static>> {
    let mut redis_client = get_redis_connection()?;

    let mut gazettes: Vec<Gazette> = vec![];

    let keys = redis_client
        .keys::<std::string::String, Vec<std::string::String>>("flagged:*".to_string())?;

    for key in keys {
        if let Ok(res) = redis_client.hgetall::<std::string::String, Gazette>(key) {
            gazettes.push(res);
        }
    }

    println!("{:?}", gazettes);

    Ok(gazettes)
}

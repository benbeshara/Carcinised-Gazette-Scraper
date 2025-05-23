mod web;

use std::error::Error;

use redis::{Commands, Connection};
use redis_macros::{ToRedisArgs, FromRedisValue};
use serde::{Deserialize, Serialize};
use select::document::Document;
use select::predicate::Name;
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};
use tokio::task::JoinError;
use imgurs::ImgurClient;

type GenericError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Clone, Debug, Default, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
#[allow(dead_code)]
pub struct Gazette {
    uri: String,
    title: Option<String>,
    img_uri: Option<String>,
    flagged: bool
}

#[tokio::main]
async fn main() {
    tokio::spawn(async move { update_pdfs().await });
    web::start_server().await;
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
        .exists::<String, String>(format!("flagged:{}", entry))
        .unwrap()
        .parse::<isize>()
        .unwrap()
        != 0
        || redis_client
            .exists::<String, String>(format!("discarded:{}", entry))
            .unwrap()
            .parse::<isize>()
            .unwrap()
            != 0)
}

async fn push_to_redis(gazette: Gazette) -> Result<(), GenericError> {
    let mut hash = make_hash(&gazette.uri).await;

    if gazette.flagged {
        hash = format!("flagged:{}", hash);
    } else {
        hash = format!("discarded:{}", hash);
    }

    let mut redis_client = get_redis_connection()?;

    redis_client.set(&hash, &gazette)?;

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

    let chunks = pdf_list.chunks(32);

    for chunk in chunks {
        let _ = filter_gazettes(chunk.to_vec()).await;
    };

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
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }

                let req = reqwest::get(&uri).await?;
                let bytes = req.bytes().await?;
                let pdf = lopdf::Document::load_mem(&bytes)?;
                let page_zero: u32 = 1;
                let page_text = pdf.extract_text(&[page_zero])?;

                if page_text.contains("Control of Weapons Act 1990") {
                    let img_uri = upload_map_from_gazette(&uri, &pdf, &hash).await.unwrap_or(None);
                    let gazette = Gazette {
                        uri: uri.clone(),
                        title: Some(title),
                        img_uri,
                        flagged: true
                    };

                    let _ = push_to_redis(gazette).await;
                    return Ok(Some(uri));
                }

                let gazette = Gazette {
                    uri: uri.clone(),
                    title: None,
                    img_uri: None,
                    flagged: false,
                };
                let _ = push_to_redis(gazette).await;
                Ok(None)
            })
        })
        .collect();

    let res: Vec<Result<Result<Option<String>, GenericError>, JoinError>> = futures::future::join_all(tasks).await;

    let res: Vec<String> = res
        .into_iter()
        .filter_map(|item| item.ok())
        .filter_map(|item| item.ok())
        .filter(|item| item.is_some())
        .map(|item| item.unwrap().to_owned())
        .collect();

    Ok(res)
}

pub async fn upload_map_from_gazette(uri: &str, pdf: &lopdf::Document, filename: &str) -> Result<Option<String>, GenericError> {
    let mut images: Vec<lopdf::xobject::PdfImage> = Vec::new();

    for page in pdf.get_pages() {
        if let Ok(page_images) = &mut pdf.get_page_images(page.1) {
            images.append(page_images)
        }
    }

    if let Some(image) = images.first() {
        let filename = format!("./{}.jpg", filename);
        if std::fs::write(filename.clone(), image.content).is_ok() {
            let client = ImgurClient::new("");
            let result = client.upload_image(&filename).await;
            match result {
                Ok(r) => Ok(Some(r.data.link)),
                Err(e) => Err(e.into()),
            }
        } else {
            println!("Could not write image {}", filename);
            Err("Could not write image".into())
        }
    } else {
        println!("No image found in {}", uri);
        Ok(None)
    }
}

pub async fn retrieve_gazettes_from_redis() -> Result<Vec<Gazette>, GenericError> {
    let mut redis_client = get_redis_connection()?;

    let mut gazettes: Vec<Gazette> = vec![];

    let keys = redis_client
        .keys::<String, Vec<String>>("flagged:*".to_string())?;

    for key in keys {
        if let Ok(res) = redis_client.get::<String, Gazette>(key) {
            gazettes.push(res);
        }
    }

    gazettes.sort_by(|a, b| b.uri.cmp(&a.uri));

    Ok(gazettes)
}

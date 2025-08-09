use crate::db::redis::RedisProvider;
use crate::db::DatabaseConnection;
use crate::geocoder::google::GoogleGeocoderProvider;
use crate::geocoder::GeocoderRequest;
use crate::location_parser::openai::OpenAI;
use crate::location_parser::LocationParser;
use crate::utils::maptypes::{MapPolygon, Sanitise};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use imgurs::ImgurClient;
use lopdf::Document;
use redis_macros::{FromRedisValue, ToRedisArgs};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::chrono;
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};

#[derive(Clone, Debug, Default, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
#[allow(dead_code)]
pub struct Gazette {
    pub uri: String,
    pub title: Option<String>,
    pub img_uri: Option<String>,
    pub flagged: bool,
    pub polygon: Option<MapPolygon>,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

pub fn make_hash(key: &str) -> String {
    let mut hasher: CoreWrapper<Sha1Core> = Sha1::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

impl Gazette {
    async fn get_pdf(&self) -> Result<Document> {
        let req = reqwest::get(&self.uri).await?;
        let bytes = req.bytes().await?;
        Document::load_mem(&bytes).map_err(|e| e.into())
    }

    pub(crate) async fn save(&self) -> Result<bool> {
        let mut hash = make_hash(&self.uri);

        if self.flagged {
            hash = format!("flagged:{hash}");
        } else {
            hash = format!("discarded:{hash}");
        }

        let db = DatabaseConnection {
            provider: RedisProvider,
        };

        db.create_entry(&hash, self).await?;

        Ok(true)
    }

    pub(crate) async fn try_upload_image(&self) -> Result<Option<String>> {
        let mut images: Vec<lopdf::xobject::PdfImage> = Vec::new();

        let pdf = self.get_pdf().await?;

        for page in pdf.get_pages() {
            if let Ok(page_images) = &mut pdf.get_page_images(page.1) {
                images.append(page_images)
            }
        }

        if let Some(image) = images.first() {
            let hash = make_hash(&self.uri);
            let filename = format!("./{hash}.jpg");
            if std::fs::write(filename.clone(), image.content).is_ok() {
                let client = ImgurClient::new("");
                let result = client.upload_image(&filename).await;
                match result {
                    Ok(r) => Ok(Some(r.data.link)),
                    Err(e) => Err(e.into()),
                }
            } else {
                Err(anyhow!("Could not upload image"))
            }
        } else {
            println!("No image found in {}", &self.uri);
            Ok(None)
        }
    }

    pub(crate) async fn get_polygon(&self) -> Result<Option<MapPolygon>> {
        let pdf = self.get_pdf().await?;

        for page in pdf.get_pages() {
            if let Ok(page_text) = &mut pdf.extract_text(&[page.0]) {
                let loc = LocationParser {
                    provider: OpenAI,
                    locations: page_text.to_owned(),
                };
                if let Ok(places) = loc.parse_locations().await {
                    let futures = places.into_iter().map(|place| async move {
                        let gc = GeocoderRequest {
                            service: GoogleGeocoderProvider,
                            input: place,
                        };
                        gc.geocode().await.unwrap_or_default()
                    });
                    let mut polygon = futures::future::join_all(futures).await;
                    polygon.sanitise();
                    return Ok(Some(MapPolygon { data: polygon }));
                }
            }
        }
        Ok(None)
    }

    pub(crate) async fn get_date(&self) -> Result<(NaiveDate, NaiveDate)> {
        let pdf = self.get_pdf().await?;

        let mut all_text = String::new();
        for page in pdf.get_pages() {
            if let Ok(page_text) = pdf.extract_text(&[page.0]) {
                all_text.push_str(&page_text);
            }
        }

        let declaration_start_index = all_text
            .find("This declaration will be in place from")
            .ok_or(anyhow!("Could not find date identifier"))?;
        let search_text = &all_text[declaration_start_index..];
        let declaration_end_index = search_text.find('.').unwrap_or(search_text.len());

        let date_string = search_text
            ["This declaration will be in place from".len()..declaration_end_index]
            .trim()
            .to_string();

        let mut date_result = Vec::new();
        let date_regex = Regex::new(r"\b\d{1,2}\s\w{1,9}\s\d{4}\b")?;

        for cap in date_regex.captures_iter(&date_string) {
            if let Some(date_str) = cap.get(0) {
                match NaiveDate::parse_from_str(date_str.as_str(), "%e %B %Y") {
                    Ok(parsed_date) => date_result.push(parsed_date),
                    Err(_) => continue,
                }
            }
        }

        date_result.sort();
        let start = date_result[0];
        let mut end = start;
        if date_result.len() > 1 {
            end = date_result[1];
        }

        Ok((start, end))
    }
}

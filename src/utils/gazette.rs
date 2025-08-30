use crate::db::redis::RedisProvider;
use crate::db::DatabaseConnection;
use crate::geocoder::google::GoogleGeocoderProvider;
use crate::geocoder::GeocoderRequest;
use crate::location_parser::openai::OpenAI;
use crate::location_parser::LocationParser;
use crate::utils::maptypes::{MapPolygon, Sanitise};
use anyhow::{anyhow, Result};
use chrono::{Datelike, Local, NaiveDate};
use lopdf::Document;
use redis_macros::{FromRedisValue, ToRedisArgs};
use regex::Regex;
use s3;
use s3::creds::Credentials;
use serde::{Deserialize, Serialize};
use serde_with::chrono;
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};
use std::env;

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

            if let Ok(access_key) = env::var("OBJECT_STORAGE_ACCESS_KEY_ID") {
                if let Ok(secret_key) = env::var("OBJECT_STORAGE_SECRET_ACCESS_KEY") {
                    let bucket = s3::Bucket::new(
                        "vicpolsearches",
                        s3::Region::ApSoutheast2,
                        Credentials {
                            access_key: Some(access_key),
                            secret_key: Some(secret_key),
                            security_token: None,
                            session_token: None,
                            expiration: None,
                        },
                    )?;

                    let _ = bucket.put_object(&filename, image.content).await?;
                    return Ok(Some(filename));
                }
                Err(anyhow!("No object storage credentials provided"))?
            } else {
                Err(anyhow!("No object storage credentials provided"))?
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

    fn parse_date_text(date_string: &str) -> Result<Vec<NaiveDate>> {
        let mut date_result = Vec::new();
        let date_regex = Regex::new(r"\b\d{1,2}\s+[ADFJMNOS]\w+(?:\s+\d{4})?\b")?;
        let current_year = Local::now().year();

        for cap in date_regex.captures_iter(&date_string) {
            if let Some(date_str) = cap.get(0) {
                match NaiveDate::parse_from_str(date_str.as_str(), "%e %B %Y").or_else(|_| {
                    let with_year = format!("{} {}", date_str.as_str(), current_year);
                    NaiveDate::parse_from_str(with_year.as_str(), "%e %B %Y")
                }) {
                    Ok(parsed_date) => date_result.push(parsed_date),
                    Err(_) => continue,
                }
            }
        }

        Ok(date_result)
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
        let search_text = &all_text[declaration_start_index..].replace("\n", "");
        let declaration_end_index = search_text.find(". ").unwrap_or(search_text.len());

        let date_string = search_text
            ["This declaration will be in place from".len()..declaration_end_index]
            .trim()
            .to_string();

        let mut date_result = Gazette::parse_date_text(&date_string)?;

        if date_result.is_empty() {
            Err(anyhow!("No dates found in {}", &self.uri))?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_date() {
        let gazette = Gazette {
            uri: "http://www.gazette.vic.gov.au/gazette/Gazettes2025/GG2025S467.pdf".to_string(),
            ..Default::default()
        };

        let date_range = gazette.get_date().await.unwrap();
        assert!(date_range.0 < date_range.1);
    }

    #[test]
    fn test_date_regex() {
        let date_string = "1.00 pm on Monday 1 September, to 1.59 am on Saturday  11 October 2025";
        let date_result = Gazette::parse_date_text(&date_string);

        assert!(
            date_result.is_ok(),
            "Date regex failed to parse date string: {}",
            date_string
        );

        let dates = date_result.unwrap();
        assert_eq!(dates.len(), 2);
        assert_eq!(
            dates[0],
            NaiveDate::from_ymd_opt(Local::now().year(), 9, 1).unwrap()
        );
        assert_eq!(dates[1], NaiveDate::from_ymd_opt(2025, 10, 11).unwrap());

        let date_string = "1.00 pm on Monday 1 September, to 1.59 am on Saturday  11 October";
        let date_result = Gazette::parse_date_text(&date_string);

        assert!(
            date_result.is_ok(),
            "Date regex failed to parse date string: {}",
            date_string
        );

        let dates = date_result.unwrap();
        assert_eq!(dates.len(), 2);
        assert_eq!(
            dates[0],
            NaiveDate::from_ymd_opt(Local::now().year(), 9, 1).unwrap()
        );
        assert_eq!(
            dates[1],
            NaiveDate::from_ymd_opt(Local::now().year(), 10, 11).unwrap()
        );

        let date_string =
            "1.00 pm on Monday 1 September 2199, to 1.59 am on Saturday 11 October 2275";
        let date_result = Gazette::parse_date_text(&date_string);

        assert!(
            date_result.is_ok(),
            "Date regex failed to parse date string: {}",
            date_string
        );

        let dates = date_result.unwrap();
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2199, 9, 1).unwrap());
        assert_eq!(dates[1], NaiveDate::from_ymd_opt(2275, 10, 11).unwrap());
    }

    #[tokio::test]
    async fn test_get_polygon() {
        let gazette = Gazette {
            uri: "http://www.gazette.vic.gov.au/gazette/Gazettes2025/GG2025S467.pdf".to_string(),
            ..Default::default()
        };

        let polygon = gazette.get_polygon().await.unwrap();

        assert!(polygon.is_some());
    }
}

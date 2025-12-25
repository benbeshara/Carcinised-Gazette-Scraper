use crate::db::core::DatabaseProvider;
use crate::db::DatabaseConnection;
use crate::geocoder::core::GeocoderProvider;
use crate::geocoder::GeocoderRequest;
use crate::image_service::{Image, ImageService};
use crate::location_parser::core::LocationParserService;
use crate::location_parser::LocationParser;
use crate::utils::maptypes::{MapPolygon, Sanitise};
use anyhow::{anyhow, Result};
use chrono::{Datelike, Local, NaiveDate};
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

pub struct GazetteHandler<T, U, V, W>
where
    T: DatabaseProvider,
    U: ImageService,
    V: LocationParserService,
    W: GeocoderProvider,
{
    pub gazette: Gazette,
    pub database_provider: T,
    pub image_service: U,
    pub location_parser: V,
    pub geocoder: W,
}

pub fn make_hash(key: &str) -> String {
    let mut hasher: CoreWrapper<Sha1Core> = Sha1::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

impl<T, U, V, W> GazetteHandler<T, U, V, W>
where
    T: DatabaseProvider + Clone,
    U: ImageService + Copy,
    V: LocationParserService + Clone,
    W: GeocoderProvider + Clone,
{
    pub(crate) async fn save(&self) -> Result<bool> {
        let mut hash = make_hash(&self.gazette.uri);

        if self.gazette.flagged {
            hash = format!("flagged:{hash}");
        } else {
            hash = format!("discarded:{hash}");
        }

        let db = DatabaseConnection {
            provider: self.database_provider.clone(),
        };

        db.create_entry(&hash, &self.gazette).await?;

        Ok(true)
    }

    pub(crate) async fn try_upload_image(&self) -> Result<Option<String>> {
        let hash = make_hash(&self.gazette.uri);

        if let Ok(map) = &self.gazette.extract_map().await {
            let image = Image {
                filename: format!("./{hash}.jpg"),
                data: map.clone(),
                service: self.image_service,
            };

            return image.upload().await;
        }
        Err(anyhow!("No map found in {}", &self.gazette.uri))
    }

    async fn get_operation_area(page_text: &str) -> Result<String> {
        let area_regex = Regex::new(r"Planned Operation in (.*)")?;

        for cap in area_regex.captures_iter(&page_text) {
            if let Some(area_string) = cap.get(1) {
                return Ok(area_string.as_str().to_string());
            }
        }

        Err(anyhow!("Could not find operation area"))
    }

    pub(crate) async fn get_polygon(&self) -> Result<Option<MapPolygon>> {
        let page_text = &self.gazette.get_doc_text().await?;
        let loc = LocationParser {
            provider: self.location_parser.clone(),
            locations: page_text.to_owned(),
        };
        let places = loc.parse_locations().await?;
        let area = &Self::get_operation_area(&page_text).await?.clone();
        let futures = places.into_iter().map(|place| async move {
            let gc = GeocoderRequest {
                service: self.geocoder.clone(),
                input: place,
                area: area.into(),
            };
            gc.geocode().await.unwrap_or_default()
        });
        let mut polygon = futures::future::join_all(futures).await;
        polygon.sanitise();
        Ok(Some(MapPolygon { data: polygon }))
    }

    pub(crate) async fn get_date(&self) -> Result<(NaiveDate, NaiveDate)> {
        self.gazette.get_date().await
    }
}

impl Gazette {
    async fn get_pdf(&self) -> Result<Document> {
        let req = reqwest::get(&self.uri).await?;
        let bytes = req.bytes().await?;
        Document::load_mem(&bytes).map_err(Into::into)
    }

    pub(crate) async fn extract_map(&self) -> Result<Vec<u8>> {
        let mut images: Vec<lopdf::xobject::PdfImage> = Vec::new();

        let pdf = self.get_pdf().await?;

        for page in pdf.get_pages() {
            if let Ok(page_images) = &mut pdf.get_page_images(page.1) {
                images.append(page_images);
            }
        }

        if let Some(map) = images.first() {
            return Ok(Vec::from(map.content));
        }

        Err(anyhow!("No map found in {}", &self.uri))
    }

    pub(crate) async fn get_doc_text(&self) -> Result<String> {
        let pdf = self.get_pdf().await?;

        let mut doc_text = String::new();
        for page in pdf.get_pages() {
            let page_text = &mut pdf.extract_text(&[page.0])?;
            doc_text += page_text;
        }
        Ok(doc_text)
    }

    fn parse_date_text(date_string: &str) -> Result<Vec<NaiveDate>> {
        let mut date_result = Vec::new();
        let date_regex = Regex::new(r"\b\d{1,2}\s+[ADFJMNOS]\w+(?:\s+\d{4})?\b")?;
        let current_year = Local::now().year();

        for cap in date_regex.captures_iter(date_string) {
            if let Some(date_str) = cap.get(0) {
                if let Ok(parsed_date) = NaiveDate::parse_from_str(date_str.as_str(), "%e %B %Y").or_else(|_| {
                    let with_year = format!("{} {}", date_str.as_str(), current_year);
                    NaiveDate::parse_from_str(with_year.as_str(), "%e %B %Y")
                }) {
                    date_result.push(parsed_date);
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
            .find("This declaration will be in place")
            .or_else(|| all_text.find("This declaration will operate as follows"))
            .or_else(|| all_text.find("The declared Designated Area will be operating"))
            .ok_or(anyhow!("Could not find date identifier"))?;
        let search_text = &all_text[declaration_start_index..].replace('\n', "");
        let declaration_end_index = search_text.find(". ").unwrap_or(search_text.len());

        let date_string = if search_text.starts_with("This declaration will be in place") {
            search_text
                ["This declaration will be in place".len()..declaration_end_index]
                .trim()
                .to_string()
        } else if search_text.starts_with("This declaration will operate as follows") {
            search_text
                ["This declaration will operate as follows".len()..declaration_end_index]
                .trim()
                .to_string()
        } else {
            search_text
                ["The declared Designated Area will be operating".len()..declaration_end_index]
                .trim()
                .to_string()
        };

        let mut date_result = Gazette::parse_date_text(&date_string)?;

        if date_result.is_empty() {
            Err(anyhow!("No dates found in {}", &self.uri))?;
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
        let date_result = Gazette::parse_date_text(date_string);

        assert!(
            date_result.is_ok(),
            "Date regex failed to parse date string: {date_string}",
        );

        let dates = date_result.unwrap();
        assert_eq!(dates.len(), 2);
        assert_eq!(
            dates[0],
            NaiveDate::from_ymd_opt(Local::now().year(), 9, 1).unwrap()
        );
        assert_eq!(dates[1], NaiveDate::from_ymd_opt(2025, 10, 11).unwrap());

        let date_string = "1.00 pm on Monday 1 September, to 1.59 am on Saturday  11 October";
        let date_result = Gazette::parse_date_text(date_string);

        assert!(
            date_result.is_ok(),
            "Date regex failed to parse date string: {date_string}",
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
        let date_result = Gazette::parse_date_text(date_string);

        assert!(
            date_result.is_ok(),
            "Date regex failed to parse date string: {date_string}"
        );

        let dates = date_result.unwrap();
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2199, 9, 1).unwrap());
        assert_eq!(dates[1], NaiveDate::from_ymd_opt(2275, 10, 11).unwrap());
    }

    #[tokio::test]
    async fn test_gazette_extraction() {
        let gazette = GazetteHandler {
            gazette: Gazette
            {
                uri: "http://www.gazette.vic.gov.au/gazette/Gazettes2025/GG2025S737.pdf".to_string(),
                ..Default::default()
            },
            database_provider: crate::db::mock::MockDatabaseProvider::new(),
            image_service: crate::image_service::mock::MockImageService::new(true),
            location_parser: crate::location_parser::mock::MockLocationParser::new(),
            geocoder: crate::geocoder::mock::MockGeocoderProvider { },
        };

        let polygon = gazette.get_polygon().await.unwrap();
        let image = gazette.try_upload_image().await.unwrap();
        let date_range = gazette.get_date().await.unwrap();

        assert!(polygon.is_some());
        assert!(image.is_some());
        assert!(date_range.0 <= date_range.1);
    }
}

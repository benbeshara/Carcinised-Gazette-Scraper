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
    pub loaded_doc: Option<Document>,
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

    pub(crate) async fn try_upload_image(&mut self) -> Result<Option<String>> {
        let hash = make_hash(&self.gazette.uri);

        let doc = self.get_or_load_doc().await?;
        if let Ok(map) = Self::extract_map_from_doc(doc) {
            let image = Image {
                filename: format!("./{hash}.jpg"),
                data: map,
                service: self.image_service,
            };

            return image.upload().await;
        }
        Err(anyhow!("No map found in {}", &self.gazette.uri))
    }

    async fn get_or_load_doc(&mut self) -> Result<&Document> {
        if self.loaded_doc.is_none() {
            let doc = self.gazette.get_pdf().await?;
            self.loaded_doc = Some(doc);
        }
        Ok(self.loaded_doc.as_ref().unwrap())
    }

    async fn get_operation_area(page_text: &str) -> Result<String> {
        let area_patterns = [
            r"Planned Operation (?:in|at)\s+([^\n\r\.]+)",
            r"Designated Area\s+under\s+section\s+10D(?:\(1\))?\s*–?\s*([^\n\r\.]+)",
            r"public places within\s+([^,]+,\s*[^,\n]+)",
        ];

        for pattern in &area_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(cap) = re.captures(page_text) {
                    if let Some(matched) = cap.get(1) {
                        let cleaned = matched.as_str().trim().trim_end_matches('.').to_string();
                        if !cleaned.is_empty() {
                            return Ok(cleaned);
                        }
                    }
                }
            }
        }

        // Fallback: search for LGA or "City of ..." or "Shire of ..."
        let lga_regex = Regex::new(r"(City of [A-Za-z\s]+|Shire of [A-Za-z\s]+)")?;
        if let Some(cap) = lga_regex.captures(page_text) {
            if let Some(matched) = cap.get(1) {
                return Ok(matched.as_str().trim().to_string());
            }
        }

        Err(anyhow!("Could not find operation area"))
    }

    pub(crate) async fn get_polygon(&mut self) -> Result<Option<MapPolygon>> {
        let doc = self.get_or_load_doc().await?;
        let page_text = Self::extract_doc_text(doc)?;
        let loc = LocationParser {
            provider: self.location_parser.clone(),
            locations: page_text.clone(),
        };
        let places = loc.parse_locations().await?;
        let area = Self::get_operation_area(&page_text).await?;
        let futures = places.into_iter().map(|place| {
            let area = area.clone();
            let geocoder = self.geocoder.clone();
            async move {
                let gc = GeocoderRequest {
                    service: geocoder,
                    input: place,
                    area,
                };
                gc.geocode().await.unwrap_or_default()
            }
        });
        let mut polygon = futures::future::join_all(futures).await;
        polygon.sanitise();

        let mut map_polygon = MapPolygon { data: polygon };
        // Remove points that accidentally resolved far outside the local cluster
        map_polygon.remove_isolated_points(15.0, 2);
        map_polygon.remove_identical_points();

        if map_polygon.data.is_empty() {
            return Ok(None);
        }

        Ok(Some(map_polygon))
    }

    pub(crate) async fn get_date(&mut self) -> Result<(NaiveDate, NaiveDate)> {
        let uri = self.gazette.uri.clone();
        let doc = self.get_or_load_doc().await?;
        Gazette::extract_date_from_doc(doc, &uri)
    }

    fn extract_map_from_doc(pdf: &Document) -> Result<Vec<u8>> {
        let mut images: Vec<lopdf::xobject::PdfImage> = Vec::new();

        for page in pdf.get_pages() {
            if let Ok(page_images) = &mut pdf.get_page_images(page.1) {
                images.append(page_images);
            }
        }

        if let Some(map) = images.first() {
            return Ok(Vec::from(map.content));
        }

        Err(anyhow!("No map found in document"))
    }

    fn extract_doc_text(pdf: &Document) -> Result<String> {
        let mut doc_text = String::new();
        for page in pdf.get_pages() {
            let page_text = pdf.extract_text(&[page.0])?;
            doc_text.push_str(&page_text);
        }
        Ok(doc_text)
    }

    pub async fn get_target_amended_gazette_uri(&mut self) -> Result<Option<String>> {
        let doc = self.get_or_load_doc().await?;
        let text = Self::extract_doc_text(doc)?;

        let is_corrigendum = text.contains("CORRIGENDUM")
            || text.contains("amends the notice")
            || text.contains("amended to")
            || text.contains("is amended");

        if !is_corrigendum {
            return Ok(None);
        }

        // Match patterns like: "Victoria Government Gazette No. S 82", "Gazette No. S82", or "No. S 82"
        let re = Regex::new(r"(?:Victoria Government Gazette No\.\s*|Gazette No\.\s*|No\.\s*)S\s*(\d+)")?;
        if let Some(cap) = re.captures(&text) {
            if let Some(num_match) = cap.get(1) {
                let notice_num: u32 = num_match.as_str().parse()?;
                // Infer the year from the current notice URI (e.g. Gazettes2026) or default to current year
                let year_re = Regex::new(r"Gazettes(\d{4})")?;
                let year = year_re
                    .captures(&self.gazette.uri)
                    .and_then(|c| c.get(1))
                    .and_then(|y| y.as_str().parse::<i32>().ok())
                    .unwrap_or_else(|| Local::now().year());

                let target_uri = format!(
                    "http://www.gazette.vic.gov.au/gazette/Gazettes{}/GG{}S{:03}.pdf",
                    year, year, notice_num
                );
                return Ok(Some(target_uri));
            }
        }

        Ok(None)
    }
}

impl Gazette {
    async fn get_pdf(&self) -> Result<Document> {
        let req = reqwest::get(&self.uri).await?;
        let bytes = req.bytes().await?;
        Document::load_mem(&bytes).map_err(Into::into)
    }

    pub(crate) fn extract_date_from_doc(
        pdf: &Document,
        uri: &str,
    ) -> Result<(NaiveDate, NaiveDate)> {
        let mut all_text = String::new();
        for page in pdf.get_pages() {
            if let Ok(page_text) = pdf.extract_text(&[page.0]) {
                all_text.push_str(&page_text);
                all_text.push(' ');
            }
        }

        let lower = all_text.to_lowercase();
        let start_markers = [
            "will be in place",
            "will operate as follows",
            "will be operating",
            "will be in effect",
            "will operate from",
        ];

        let marker_pos = start_markers
            .iter()
            .filter_map(|&marker| {
                lower.find(marker).map(|idx| idx + marker.len())
            })
            .min()
            .ok_or_else(|| anyhow!("Could not find date identifier in {uri}"))?;

        let search_text = all_text[marker_pos..].replace('\n', " ");

        // Boundary markers that signal the end of the date/schedule section
        let stop_markers = [
            "SCHEDULE",
            "Schedule",
            "Designated Area",
            "Victoria Police powers",
            "It is an offence",
            "Dated ",
            "This page was left blank",
        ];

        let end_idx = stop_markers
            .iter()
            .filter_map(|&marker| search_text.find(marker))
            .min()
            .unwrap_or_else(|| search_text.len().min(1200));

        let date_string = &search_text[..end_idx];
        let mut date_result = Gazette::parse_date_text(date_string)?;

        if date_result.is_empty() {
            return Err(anyhow!("No dates found in {uri}"));
        }
        date_result.sort();
        let start = date_result[0];
        let end = *date_result.last().unwrap_or(&start);

        Ok((start, end))
    }

    fn parse_date_text(date_string: &str) -> Result<Vec<NaiveDate>> {
        let mut date_result = Vec::new();
        let date_regex = Regex::new(r"\b\d{1,2}\s+[ADFJMNOS]\w+(?:\s+\d{4})?\b")?;
        let current_year = Local::now().year();

        for cap in date_regex.captures_iter(date_string) {
            if let Some(date_str) = cap.get(0) {
                let s = date_str.as_str();
                let parse_attempt = NaiveDate::parse_from_str(s, "%e %B %Y")
                    .or_else(|_| NaiveDate::parse_from_str(s, "%d %B %Y"))
                    .or_else(|_| {
                        let with_year = format!("{} {}", s, current_year);
                        NaiveDate::parse_from_str(&with_year, "%e %B %Y")
                            .or_else(|_| NaiveDate::parse_from_str(&with_year, "%d %B %Y"))
                    });

                if let Ok(parsed_date) = parse_attempt {
                    date_result.push(parsed_date);
                }
            }
        }

        Ok(date_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut gazette = GazetteHandler {
            gazette: Gazette {
                uri: "http://www.gazette.vic.gov.au/gazette/Gazettes2025/GG2025S737.pdf".to_string(),
                ..Default::default()
            },
            database_provider: crate::db::mock::MockDatabaseProvider::new(),
            image_service: crate::image_service::mock::MockImageService::new(true),
            location_parser: crate::location_parser::mock::MockLocationParser::new(),
            geocoder: crate::geocoder::mock::MockGeocoderProvider {},
            loaded_doc: None,
        };

        let polygon = gazette.get_polygon().await.unwrap();
        let image = gazette.try_upload_image().await.unwrap();
        let date_range = gazette.get_date().await.unwrap();

        assert!(polygon.is_some());
        assert!(image.is_some());
        assert!(date_range.0 <= date_range.1);
    }
}
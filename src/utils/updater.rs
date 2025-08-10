use crate::db::redis::RedisProvider;
use crate::db::DatabaseConnection;
use crate::utils::gazette::{make_hash, Gazette};
use anyhow::Result;
use futures::stream::StreamExt;
use select::document::Document;
use select::predicate::Name;

const FIRST_PAGE: u32 = 1;
const TARGET_TEXT: &str = "Control of Weapons Act 1990";

#[derive(Clone, Debug)]
pub struct Updater {
    pub uri: String,
    pub base_uri: String,
}

impl Updater {
    pub async fn update(&mut self) -> Result<Vec<String>> {
        let results = self.parse_webpage().await?;

        let filtered_results = futures::stream::iter(results)
            .map(|result| async {
                let (_, uri) = &result;
                let hash = make_hash(uri);
                let db = DatabaseConnection {
                    provider: RedisProvider,
                };

                match db.has_entry(&hash).await {
                    Ok(false) => match self.filter_result(&result).await {
                        Ok(is_flagged) => Some((result, is_flagged)),
                        Err(_) => None,
                    },
                    _ => None,
                }
            })
            .buffer_unordered(12)
            .collect::<Vec<_>>()
            .await;

        let (flagged, discarded): (Vec<_>, Vec<_>) = filtered_results
            .into_iter()
            .flatten()
            .partition(|(_result, is_flagged)| *is_flagged);

        let flagged_futures = flagged.into_iter().map(|((title, uri), _)| async move {
            let mut gazette = Gazette {
                uri: uri.clone(),
                title: Some(title),
                img_uri: None,
                flagged: true,
                polygon: None,
                start: None,
                end: None,
            };

            if let Ok(img) = gazette.try_upload_image().await {
                gazette.img_uri = img;
            }

            if let Ok(polygon) = gazette.get_polygon().await {
                gazette.polygon = polygon;
            }

            if let Ok(date) = gazette.get_date().await {
                gazette.start = Some(date.0);
                gazette.end = Some(date.1);
            }

            let _ = gazette.save().await;
            uri
        });

        let discarded_futures = discarded.into_iter().map(|((title, uri), _)| async move {
            let gazette = Gazette {
                uri: uri.clone(),
                title: Some(title),
                img_uri: None,
                flagged: false,
                polygon: None,
                start: None,
                end: None,
            };

            let _ = gazette.save().await;
        });

        let (flagged_uris, _) = tokio::join!(
            futures::future::join_all(flagged_futures),
            futures::future::join_all(discarded_futures)
        );

        println!("PDF Update Complete");
        Ok(flagged_uris)
    }

    async fn parse_webpage(&self) -> Result<Vec<(String, String)>> {
        let response = reqwest::get(&self.uri)
            .await?
            .error_for_status()?
            .text()
            .await?;

        let pdf_list: Vec<(String, String)> = Document::from(response.as_str())
            .find(Name("a"))
            .filter_map(|element| {
                element
                    .attr("href")
                    .map(|href| (element.inner_html(), format!("{}{}", self.base_uri, href)))
            })
            .filter(|(_, url)| url.ends_with(".pdf"))
            .collect();

        Ok(pdf_list)
    }

    async fn filter_result(&self, chunk: &(String, String)) -> Result<bool> {
        let (_, uri) = chunk;

        let pdf_bytes = reqwest::get(uri).await?.bytes().await?;
        let pdf = lopdf::Document::load_mem(&pdf_bytes)?;
        let page_text = pdf.extract_text(&[FIRST_PAGE])?;

        Ok(page_text.contains(TARGET_TEXT))
    }
}

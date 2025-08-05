use crate::db::db::DatabaseConnection;
use crate::db::redis::RedisProvider;
use crate::utils::gazette::{make_hash, Gazette, Save, UploadImage};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
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
        if let Ok(results) = self.parse_webpage().await {
            let mut flagged: Vec<(String, String)> = vec![];
            let mut discarded: Vec<(String, String)> = vec![];

            for result in results {
                let (title, uri) = &result;
                let hash = make_hash(&uri);
                let db = DatabaseConnection {
                    provider: RedisProvider,
                };
                if db.has_entry(&hash).await? == false {
                    if self.filter_result(&result).await? {
                        flagged.push((title.clone(), uri.clone()));
                    } else {
                        discarded.push((title.clone(), uri.clone()));
                    }
                }
            };

            for (title, uri) in &flagged {
                let mut gazette = Gazette {
                    uri: uri.clone(),
                    title: Some(title.clone()),
                    img_uri: None,
                    flagged: true,
                };

                if let Ok(img) = gazette.try_upload_image().await {
                    gazette.img_uri = img;
                }

                let _ = gazette.save().await;
            }

            for (title, uri) in discarded {
                let gazette = Gazette {
                    uri: uri.clone(),
                    title: None,
                    img_uri: None,
                    flagged: false,
                };

                let _ = gazette.save().await;
            }

            println!("PDF Update Complete");
            Ok(flagged.iter().map(|(_, uri)| uri.clone()).collect())
        } else {
            Err(anyhow::anyhow!("PDF Update Failed"))
        }
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

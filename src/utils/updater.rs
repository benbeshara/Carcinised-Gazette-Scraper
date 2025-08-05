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
        if let Ok(chunks) = self.parse_webpage().await {
            let result = self.process_chunks(chunks).await?;
            let (flagged, discarded) = result;
            let _ = flagged.iter().map(async |(title, uri)| {
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
            });

            let _ = discarded.iter().map(async |(_, uri)| {
                let gazette = Gazette {
                    uri: uri.clone(),
                    title: None,
                    img_uri: None,
                    flagged: false,
                };

                let _ = gazette.save().await;
            });

            Ok(flagged.iter().map(|(_, uri)| uri.clone()).collect())
        } else {
            Err(anyhow::anyhow!("PDF Update Failed"))
        }
    }

    async fn process_chunks(
        &self,
        chunks: Vec<Vec<(String, String)>>,
    ) -> Result<(Vec<(String, String)>, Vec<(String, String)>)> {
        let results = stream::iter(chunks)
            .map(|chunk| async move {
                let mut flagged = Vec::new();
                let mut discarded = Vec::new();
                for item in chunk.iter() {
                    if self.filter_result(item).await? {
                        flagged.push(item.clone());
                    } else {
                        discarded.push(item.clone());
                    }
                }
                Ok::<(Vec<(String, String)>, Vec<(String, String)>), anyhow::Error>((
                    flagged, discarded,
                ))
            })
            .buffer_unordered(3)
            .collect::<Vec<_>>()
            .await;

        let mut all_flagged = Vec::new();
        let mut all_discarded = Vec::new();

        for result in results {
            if let Ok((flagged, discarded)) = result {
                all_flagged.extend(flagged);
                all_discarded.extend(discarded);
            }
        }

        Ok((all_flagged, all_discarded))
    }

    async fn parse_webpage(&self) -> Result<Vec<Vec<(String, String)>>> {
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

        Ok(pdf_list.chunks(32).map(|c| c.to_vec()).collect())
    }

    async fn filter_result(&self, chunk: &(String, String)) -> Result<bool> {
        let (_, uri) = chunk;
        let db = DatabaseConnection {
            provider: RedisProvider,
        };

        let hash = make_hash(&uri).await;

        match db.has_entry(hash.as_str()).await {
            Ok(true) => return Ok(false),
            Err(_) => return Ok(false),
            Ok(false) => {}
        }

        let pdf_bytes = reqwest::get(uri).await?.bytes().await?;
        let pdf = lopdf::Document::load_mem(&pdf_bytes)?;
        let page_text = pdf.extract_text(&[FIRST_PAGE])?;

        Ok(page_text.contains(TARGET_TEXT))
    }
}

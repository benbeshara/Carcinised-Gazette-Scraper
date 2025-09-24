use crate::db::core::DatabaseProvider;
use crate::db::DatabaseConnection;
use crate::geocoder::core::GeocoderProvider;
use crate::image_service::ImageService;
use crate::location_parser::core::LocationParserService;
use crate::utils::gazette::{make_hash, Gazette, GazetteHandler};
use anyhow::Result;
use futures::stream::StreamExt;
use select::document::Document;
use select::predicate::Name;

const FIRST_PAGE: u32 = 1;
const TARGET_TEXT: &str = "Control of Weapons Act 1990";

#[derive(Clone, Debug)]
pub struct ServiceConfig<T, U, V, W>
where
    T: DatabaseProvider + Clone + Send + Sync,
    U: ImageService + Clone + Send + Sync,
    V: LocationParserService + Clone + Send + Sync,
    W: GeocoderProvider + Clone + Send + Sync,
{
    pub database_provider: T,
    pub image_service: U,
    pub location_parser: V,
    pub geocoder: W,
}

#[derive(Clone, Debug)]
pub struct Updater<T, U, V, W>
where
    T: DatabaseProvider + Clone + Send + Sync,
    U: ImageService + Copy + Send + Sync,
    V: LocationParserService + Clone + Send + Sync,
    W: GeocoderProvider + Clone + Send + Sync,
{
    pub uri: String,
    pub base_uri: String,
    pub config: ServiceConfig<T, U, V, W>,
}

impl<T, U, V, W> Updater<T, U, V, W>
where
    T: DatabaseProvider + Clone + Send + Sync,
    U: ImageService + Copy + Send + Sync,
    V: LocationParserService + Clone + Send + Sync,
    W: GeocoderProvider + Clone + Send + Sync,
{
    pub async fn update(&self) -> Result<Vec<String>> {
        let results = self.parse_webpage().await?;

        let filtered_results = futures::stream::iter(results)
            .map(|result| async {
                let (_, uri) = &result;
                let hash = make_hash(uri);
                let db = DatabaseConnection {
                    provider: self.config.database_provider.clone(),
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
            let mut gazette_handler = GazetteHandler {
                gazette: Gazette {
                    uri: uri.clone(),
                    title: Some(title),
                    flagged: true,
                    ..Default::default()
                },
                database_provider: self.config.database_provider.clone(),
                image_service: self.config.image_service,
                location_parser: self.config.location_parser.clone(),
                geocoder: self.config.geocoder.clone(),
            };

            if let Ok(img) = gazette_handler.try_upload_image().await {
                gazette_handler.gazette.img_uri = img;
            }

            if let Ok(polygon) = gazette_handler.get_polygon().await {
                gazette_handler.gazette.polygon = polygon;
            }

            if let Ok(date) = gazette_handler.get_date().await {
                gazette_handler.gazette.start = Some(date.0);
                gazette_handler.gazette.end = Some(date.1);
            }

            let _ = gazette_handler.save().await;
            uri
        });

        let discarded_futures = discarded.into_iter().map(|((title, uri), _)| async move {
            let gazette_handler = GazetteHandler {
                gazette: Gazette {
                    uri: uri.clone(),
                    title: Some(title),
                    ..Default::default()
                },
                database_provider: self.config.database_provider.clone(),
                image_service: self.config.image_service,
                location_parser: self.config.location_parser.clone(),
                geocoder: self.config.geocoder.clone(),
            };

            let _ = gazette_handler.save().await;
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
            .filter(|(_, url)| std::path::Path::new(url)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")))
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

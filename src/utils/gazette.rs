use crate::db::db::DatabaseConnection;
use crate::db::redis::RedisProvider;
use anyhow::{anyhow, Result};
use imgurs::ImgurClient;
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};
use crate::geocoder::geocoder::GeocoderRequest;
use crate::geocoder::google::GoogleGeocoderProvider;
use crate::location_parser::location_parser::LocationParser;
use crate::location_parser::openai::OpenAI;
use crate::utils::maptypes::{GeoPosition, MapPolygon, Sanitise};

#[derive(Clone, Debug, Default, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
#[allow(dead_code)]
pub struct Gazette {
    pub uri: String,
    pub title: Option<String>,
    pub img_uri: Option<String>,
    pub flagged: bool,
    pub polygon: Option<MapPolygon>,
}

pub trait Save {
    async fn save(&self) -> Result<bool>;
}

pub trait UploadImage {
    async fn try_upload_image(&self) -> Result<Option<String>>;
}

pub fn make_hash(key: &str) -> String {
    let mut hasher: CoreWrapper<Sha1Core> = Sha1::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

impl Gazette {
    pub(crate) async fn save(&self) -> Result<bool> {
        let mut hash = make_hash(&self.uri);

        if self.flagged {
            hash = format!("flagged:{}", hash);
        } else {
            hash = format!("discarded:{}", hash);
        }

        let db = DatabaseConnection {
            provider: RedisProvider,
        };

        db.create_entry(&hash, self).await?;

        Ok(true)
    }

    pub(crate) async fn try_upload_image(&self) -> Result<Option<String>> {
        let mut images: Vec<lopdf::xobject::PdfImage> = Vec::new();

        let req = reqwest::get(&self.uri).await?;
        let bytes = req.bytes().await?;
        let pdf = lopdf::Document::load_mem(&bytes)?;

        for page in pdf.get_pages() {
            if let Ok(page_images) = &mut pdf.get_page_images(page.1) {
                images.append(page_images)
            }
        }

        if let Some(image) = images.first() {
            let hash = make_hash(&self.uri);
            let filename = format!("./{}.jpg", hash);
            if std::fs::write(filename.clone(), image.content).is_ok() {
                let client = ImgurClient::new("");
                let result = client.upload_image(&filename).await;
                match result {
                    Ok(r) => Ok(Some(r.data.link)),
                    Err(e) => Err(e.into()),
                }
            } else {
                println!("Could not write image {}", filename);
                Err(anyhow!("Could not upload image"))
            }
        } else {
            println!("No image found in {}", &self.uri);
            Ok(None)
        }
    }

    pub(crate) async fn get_polygon(&self) -> Result<Option<MapPolygon>> {
        let req = reqwest::get(&self.uri).await?;
        let bytes = req.bytes().await?;
        let pdf = lopdf::Document::load_mem(&bytes)?;

        for page in pdf.get_pages() {
            if let Ok(page_text) = &mut pdf.extract_text(&[page.0]) {
                let loc = LocationParser {
                    provider: OpenAI,
                    locations: page_text.to_owned()
                };
                if let Ok(places) = loc.parse_locations().await {
                    let futures = places.into_iter().map(|place| async move {
                        let gc = GeocoderRequest {
                            service: GoogleGeocoderProvider,
                            input: place
                        };
                        let pos = gc.geocode().await.unwrap_or_default();
                        GeoPosition {
                            latitude: pos.latitude.clone(),
                            longitude: pos.longitude.clone()
                        }
                    });
                    let mut polygon = futures::future::join_all(futures).await;
                    polygon.sanitise();
                    return Ok(Some(MapPolygon {
                        data: polygon
                    }));
                }
            }
        }
        Ok(None)
    }
}

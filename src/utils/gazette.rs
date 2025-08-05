use anyhow::{anyhow, Result};
use imgurs::ImgurClient;
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1, Sha1Core};
use crate::db::db::DatabaseConnection;
use crate::db::redis::RedisProvider;

#[derive(Clone, Debug, Default, Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
#[allow(dead_code)]
pub struct Gazette {
    pub uri: String,
    pub title: Option<String>,
    pub img_uri: Option<String>,
    pub flagged: bool,
}

pub trait Save {
    async fn save(&self) -> Result<bool>;
}

pub trait UploadImage {
    async fn try_upload_image(&self) -> Result<Option<String>>;
}

pub async fn make_hash(key: &str) -> String {
    let mut hasher: CoreWrapper<Sha1Core> = Sha1::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

impl Save for Gazette {
    async fn save(&self) -> Result<bool> {
        let mut hash = make_hash(&self.uri).await;

        if self.flagged {
            hash = format!("flagged:{}", hash);
        } else {
            hash = format!("discarded:{}", hash);
        }

        let db = DatabaseConnection {
            provider: RedisProvider
        };

        db.create_entry(&hash, self).await?;

        Ok(true)
    }
}

impl UploadImage for Gazette {
    async fn try_upload_image(&self) -> Result<Option<String>> {
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
            let hash = make_hash(&self.uri).await;
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
}
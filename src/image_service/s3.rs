use crate::image_service::core::ImageService;
use anyhow::{anyhow, Result};
use s3::creds::Credentials;
use std::env;

#[derive(Clone, Copy, Debug)]
pub struct S3;

#[async_trait::async_trait]
impl ImageService for S3 {
    async fn upload(&self, filename: String, data: Vec<u8>) -> Result<Option<String>> {
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

                let _ = bucket.put_object(&filename, &data).await?;
                return Ok(Some(filename));
            }
            Err(anyhow!("No object storage credentials provided"))?
        } else {
            Err(anyhow!("No object storage credentials provided"))?
        }
    }
}

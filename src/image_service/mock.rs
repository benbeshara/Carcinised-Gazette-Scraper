use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;
use anyhow::{anyhow, Result};
use crate::image_service::{Image, ImageService};

static UPLOAD_COUNT: AtomicUsize = AtomicUsize::new(0);
static RESET: Once = Once::new();

#[derive(Clone, Copy)]
pub(crate) struct MockImageService {
    should_succeed: bool,
}

impl MockImageService {
    pub fn new(should_succeed: bool) -> Self {
        RESET.call_once(|| {
            UPLOAD_COUNT.store(0, Ordering::SeqCst);
        });

        Self { should_succeed }
    }

    fn upload_count() -> usize {
        UPLOAD_COUNT.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ImageService for MockImageService {
    async fn upload(&self, filename: String, _data: Vec<u8>) -> Result<Option<String>> {
        UPLOAD_COUNT.fetch_add(1, Ordering::SeqCst);

        if self.should_succeed {
            Ok(Some(format!("mock_url/{}", filename)))
        } else {
            Err(anyhow!("Mock upload failed"))
        }
    }
}

#[tokio::test]
async fn test_successful_upload() {
    let service = MockImageService::new(true);
    let image = Image {
        filename: "test.jpg".to_string(),
        data: vec![1, 2, 3],
        service,
    };

    let result = image.upload().await;
    assert!(result.is_ok());
    assert_eq!(MockImageService::upload_count(), 1);
    assert_eq!(result.unwrap(), Some("mock_url/test.jpg".to_string()));
}

#[tokio::test]
async fn test_failed_upload() {
    let service = MockImageService::new(false);
    let image = Image {
        filename: "test.jpg".to_string(),
        data: vec![1, 2, 3],
        service,
    };

    let result = image.upload().await;
    assert!(result.is_err());
    assert_eq!(MockImageService::upload_count(), 1);
}
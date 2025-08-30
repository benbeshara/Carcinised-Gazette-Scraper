use anyhow::Result;

#[async_trait::async_trait]
pub trait LocationParserService {
    async fn parse_locations(&self, locations: String) -> Result<Vec<String>>;
}

pub struct LocationParser<T>
where
    T: LocationParserService,
{
    pub provider: T,
    pub locations: String,
}

impl<T> LocationParser<T>
where
    T: LocationParserService,
{
    pub async fn parse_locations(&self) -> Result<Vec<String>> {
        T::parse_locations(&self.provider, self.locations.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location_parser::mock::MockLocationParser;

    #[tokio::test]
    async fn test_location_parser() {
        let mock_parser = MockLocationParser::new();
        let parser = LocationParser {
            provider: mock_parser,
            locations: "Southern Cross area".to_string(),
        };

        let locations = parser.parse_locations().await.unwrap();
        assert_eq!(locations.len(), 5);
    }
}

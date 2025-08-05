use anyhow::Result;

#[async_trait::async_trait]
pub trait LocationParserService {
    async fn parse_locations(&self, locations: String) -> Result<Vec<String>>;
}

pub struct LocationParser<T> where T: LocationParserService {
    provider: T,
    locations: String
}

impl <T> LocationParser<T> where T: LocationParserService {
    pub async fn parse_locations(&self) -> Result<Vec<String>> {
        T::parse_locations(&self.provider, self.locations.clone()).await
    }
}
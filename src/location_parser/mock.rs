use crate::location_parser::core::LocationParserService;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MockLocationParser {
    predefined_responses: HashMap<String, Vec<String>>,
}

impl MockLocationParser {
    pub fn new() -> Self {
        let mut responses = HashMap::new();

        responses.insert(
            "Southern Cross area".to_string(),
            vec![
                "Southern Cross Railway Station".to_string(),
                "Corner of Collins Street and Spencer Street".to_string(),
                "Southern Cross Bridge".to_string(),
                "Corner of Lonsdale Street and Spencer Street".to_string(),
                "Corner of La Trobe Street and Wurundjeri Way".to_string(),
            ],
        );

        responses.insert(
            "CBD locations".to_string(),
            vec![
                "Corner of Flinders Street and Swanston Street".to_string(),
                "Flinders Street Railway Station".to_string(),
                "Melbourne CBD".to_string(),
            ],
        );

        Self {
            predefined_responses: responses,
        }
    }

    // Helper method to add custom responses for testing
    pub fn with_response(mut self, input: &str, response: Vec<String>) -> Self {
        self.predefined_responses
            .insert(input.to_string(), response);
        self
    }
}

#[async_trait::async_trait]
impl LocationParserService for MockLocationParser {
    async fn parse_locations(&self, locations: String) -> Result<Vec<String>> {
        Ok(self
            .predefined_responses
            .get(&locations)
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location_parser::LocationParser;

    #[tokio::test]
    async fn test_mock_location_parser() {
        let parser = MockLocationParser::new();

        let locations = parser
            .parse_locations("Southern Cross area".to_string())
            .await
            .unwrap();
        assert_eq!(locations.len(), 5);
        assert!(locations.contains(&"Southern Cross Railway Station".to_string()));

        let empty_locations = parser
            .parse_locations("Unknown location".to_string())
            .await
            .unwrap();
        assert!(empty_locations.is_empty());
    }

    #[tokio::test]
    async fn test_custom_response() {
        let custom_response = vec![
            "Custom Location 1".to_string(),
            "Custom Location 2".to_string(),
        ];
        let parser =
            MockLocationParser::new().with_response("custom test", custom_response.clone());

        let result = parser
            .parse_locations("custom test".to_string())
            .await
            .unwrap();
        assert_eq!(result, custom_response);
    }

    #[tokio::test]
    async fn test_with_location_parser_struct() {
        let parser = LocationParser {
            provider: MockLocationParser::new(),
            locations: "CBD locations".to_string(),
        };

        let locations = parser.parse_locations().await.unwrap();
        assert_eq!(locations.len(), 3);
        assert!(locations.contains(&"Melbourne CBD".to_string()));
    }
}

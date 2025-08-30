mod db;
mod geocoder;
mod image_service;
mod location_parser;
mod utils;
mod web;

use crate::db::redis::RedisProvider;
use crate::geocoder::google::GoogleGeocoderProvider;
use crate::image_service::S3;
use crate::location_parser::openai::OpenAI;
use crate::utils::updater::ServiceConfig;
use crate::utils::updater::Updater;
use crate::web::start_server;

#[tokio::main]
async fn main() {
    let config = ServiceConfig {
        database_provider: RedisProvider,
        image_service: S3,
        location_parser: OpenAI,
        geocoder: GoogleGeocoderProvider,
    };

    let server = tokio::spawn(start_server(config.clone()));

    let update = tokio::spawn(async move {
        let _ = Updater {
            uri: "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm".to_string(),
            base_uri: "http://www.gazette.vic.gov.au".to_string(),
            config,
        }.update().await;
    });

    let (_, _) = tokio::join!(update, server);
}

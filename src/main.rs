mod db;
mod geocoder;
mod image_service;
mod location_parser;
mod utils;
mod web;

use crate::db::redis::RedisProvider;
use crate::web::{start_server, ServerConfig};
use utils::updater::Updater;

#[tokio::main]
async fn main() {
    let updater = Updater {
        uri: "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm".to_string(),
        base_uri: "http://www.gazette.vic.gov.au".to_string(),
        database_provider: RedisProvider,
        image_service: image_service::S3,
    };
    let update = tokio::spawn(async move {
        let _ = updater.update().await;
    });

    let config = ServerConfig {
        database_provider: RedisProvider,
        image_service: image_service::S3,
    };

    let server = tokio::spawn(start_server(config));
    let (_, _) = tokio::join!(update, server);
}

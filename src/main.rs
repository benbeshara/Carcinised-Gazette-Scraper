mod db;
mod geocoder;
mod location_parser;
mod utils;
mod web;

use utils::updater::Updater;
use web::start_server;

#[tokio::main]
async fn main() {
    let updater = Updater {
        uri: "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm".to_string(),
        base_uri: "http://www.gazette.vic.gov.au".to_string(),
    };
    let update = tokio::spawn(async move {
        let _ = updater.update().await;
    });
    let server = tokio::spawn(start_server());
    let (_, _) = tokio::join!(update, server);
}

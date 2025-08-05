mod geocoder;
mod utils;
mod web;
mod db;
mod location_parser;

use utils::updater::Updater;
use web::web::start_server;

#[tokio::main]
async fn main() {
    let mut updater = Updater {
        uri: "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm".to_string(),
        base_uri: "http://www.gazette.vic.gov.au".to_string(),
    };
    let _ = updater.update().await;
    start_server().await;
}

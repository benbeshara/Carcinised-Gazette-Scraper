use crate::geocoder::geoposition::{GeoPosition, GeocoderRequest, Sanitise};
use crate::geocoder::google::{GoogleGeocoderPosition, GoogleGeocoderRequest};
use crate::parser::openai::openai_request;

pub async fn map_points_from_desc(req: String) -> Vec<GeoPosition> {
    let mut response: Vec<GoogleGeocoderPosition> = vec![];
    match openai_request(req).await {
        Ok(locs) => {
            for loc in locs {
                let req = GeocoderRequest { request: loc.clone() };
                match req.google_geocoder_request().await {
                    Ok(result) => response.push(result),
                    Err(geocode_err) => eprintln!("Failed to process location '{}': {:?}", loc, geocode_err),
                }
            }
        },
        Err(err) => {
            eprintln!("Error in OpenAI request: {:?}", err);
        }
    }

    let mut points: Vec<GeoPosition> = response.iter().map(|point| point.into()).collect();
    points.sanitise();
    points
}

#[tokio::main]
async fn main() {
    let mut points = map_points_from_desc("of Police, under section 10D(1) of the Control of Weapons Act 1990, declares as a designated
area, all public places within the area containing Warragul CBD and Railway Station, bordered by
(approximately):
south and west: Witton Street, Queen Street, Alfred Street, Princes Way
north: Barkley Street, Smith Street, Biggs Lane, Albert Street
east: Mason Street, Princes Way/Queen Street
Warragul Railway Station and carparks.".into()).await;
    println!("{:?}", points);
}

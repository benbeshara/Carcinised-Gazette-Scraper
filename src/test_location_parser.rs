use crate::geocoder::geocoder::{GeocoderProvider, GeocoderRequest};
use crate::geocoder::google::GoogleGeocoderProvider;
use crate::parser::openai::openai_request;
use crate::utils::maptypes::{GeoPosition, MapPolygon, Sanitise};

pub async fn map_points_from_desc<T>(req: String, provider: T) -> MapPolygon where T: GeocoderProvider + Copy {
    let mut response: Vec<GeoPosition> = vec![];
    match openai_request(req).await {
        Ok(locs) => {
            for loc in locs {
                let req = GeocoderRequest {
                    input: loc.clone(),
                    service: provider,
                };
                match req.geocode().await {
                    Ok(result) => response.push(result),
                    Err(geocode_err) => {
                        eprintln!("Failed to process location '{}': {:?}", loc, geocode_err)
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error in OpenAI request: {:?}", err);
        }
    }

    let mut points: Vec<GeoPosition> = response.iter().map(|point| point.to_owned()).collect();
    points.sanitise();
    MapPolygon {
        data: points.clone(),
    }
}

#[tokio::main]
async fn main() {
    let points = map_points_from_desc("of Police, under section 10D(1) of the Control of Weapons Act 1990, declares as a designated
area, all public places within the area containing Warragul CBD and Railway Station, bordered by
(approximately):
south and west: Witton Street, Queen Street, Alfred Street, Princes Way
north: Barkley Street, Smith Street, Biggs Lane, Albert Street
east: Mason Street, Princes Way/Queen Street
Warragul Railway Station and carparks.".into(), GoogleGeocoderProvider).await;
    println!("{}", points);
}

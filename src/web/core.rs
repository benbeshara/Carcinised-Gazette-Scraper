use crate::db::redis::RedisProvider;
use crate::db::DatabaseConnection;
use crate::utils::geojson::{
    GeoJsonFeature, GeoJsonFeatureCollection, GeoJsonGeometry, GeoJsonProperties,
};
use crate::utils::updater::Updater;
use crate::web::templates::base::base_template;
use crate::web::templates::components::{
    footer_section, header_section, list_section, map_section, notice_section,
};
use crate::web::templates::styles::get_styles;
use axum::{
    self,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use futures::stream::{self, Stream};
use maud::{html, Markup, PreEscaped};
use std::net::SocketAddr;
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt as _;

pub async fn start_server() {
    let app = Router::new()
        .route("/", get(landing))
        .route("/data", get(list_sse));

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string()) // Get the port as a string or default to "3000"
        .parse() // Parse the port string into a u16
        .expect("Failed to parse PORT");

    let address = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn list_sse() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let data = send_data().await;
    let polygons = fetch_polygons().await;
    let circles = fetch_circles().await;
    let stream = stream::iter([
        Event::default().data(data).event("list"),
        Event::default().data(circles).event("circles"),
        Event::default().data(polygons).event("close"),
    ])
    .map(Ok);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

async fn send_data() -> String {
    let mut updater = Updater {
        uri: "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm".to_string(),
        base_uri: "http://www.gazette.vic.gov.au".to_string(),
    };
    let _ = updater.update().await;
    format!("<ul>{}</ul>", render_list().await)
}

async fn initial_list() -> Markup {
    html! {
        (PreEscaped(render_list().await))
    }
}

async fn fetch_polygons() -> String {
    let db = DatabaseConnection {
        provider: RedisProvider,
    };
    if let Ok(gazettes) = db.fetch_entries().await {
        let mut feature_collection = GeoJsonFeatureCollection::new();

        for gazette in gazettes {
            if let Some(polygon) = &gazette.polygon {
                let processed_polygon = polygon
                    .clone()
                    .remove_isolated_points(5.0, 2)
                    .remove_identical_points()
                    .convex_hull();

                // Convert polygon data to GeoJSON format
                let coordinates = vec![processed_polygon
                    .data
                    .iter()
                    .map(|point| [point.longitude, point.latitude])
                    .collect::<Vec<[f64; 2]>>()];

                let feature = GeoJsonFeature {
                    type_field: "Feature".to_string(),
                    geometry: GeoJsonGeometry::Polygon {
                        coordinates,
                    },
                    properties: GeoJsonProperties {
                        title: gazette.title,
                        uri: gazette.uri,
                        img_uri: gazette.img_uri,
                    },
                };

                feature_collection.features.push(feature);
            }
        }

        return serde_json::to_string(&feature_collection).unwrap_or_else(|_| "{}".to_string());
    }
    "[]".to_string()
}

async fn fetch_circles() -> String {
    let db = DatabaseConnection {
        provider: RedisProvider,
    };
    if let Ok(gazettes) = db.fetch_entries().await {
        let mut feature_collection = GeoJsonFeatureCollection::new();

        for gazette in gazettes {
            if let Some(polygon) = &gazette.polygon {
                let processed_polygon = polygon
                    .to_owned()
                    .remove_identical_points()
                    .remove_isolated_points(2.5, 2) // these numbers are a best-guess and should be tweaked over time
                    .clone();

                if processed_polygon.data.len() == 2 {
                    let coordinates = processed_polygon
                        .centre()
                        .into();

                    let feature = GeoJsonFeature {
                        type_field: "Feature".to_string(),
                        geometry: GeoJsonGeometry::Point {
                            coordinates
                        },
                        properties: GeoJsonProperties {
                            title: gazette.title,
                            uri: gazette.uri,
                            img_uri: gazette.img_uri,
                        },
                    };

                    feature_collection.features.push(feature);
                }
            }
        }

        return serde_json::to_string(&feature_collection).unwrap_or_else(|_| "{}".to_string());
    }
    "[]".to_string()
}

async fn render_list() -> String {
    let db = DatabaseConnection {
        provider: RedisProvider,
    };
    if let Ok(gazettes) = db.fetch_entries().await {
        let acc = gazettes.iter().fold(String::new(), |mut acc, gz| {
            acc += html!(
                li {
                    div {
                        span {
                            a href=(gz.uri) target="_blank" {
                                @if let Some(title) = &gz.title {
                                    (title)
                                }
                                span.uri {
                                    (gz.uri)
                                }
                            }
                        }
                    }
                    div.thumbnail {
                        @if let Some(img_uri) = &gz.img_uri {
                            a href=(img_uri) target="_blank" {
                                img src=(img_uri) {}
                            }
                        }
                    }
                }
            )
            .into_string()
            .as_str();

            acc
        });
        return acc;
    }
    "Failed to fetch data".to_string()
}

async fn landing() -> Markup {
    let initial_polygons = fetch_polygons().await;
    let initial_circles = fetch_circles().await;
    let initial_list_content = initial_list().await;

    base_template(html! {
        div.center {
            (header_section())
            (notice_section())
            (map_section())
            (list_section(initial_list_content))
            (footer_section())
        }
        (get_styles())
        (map_javascript(&initial_polygons, &initial_circles))
    })
}

fn map_javascript(initial_polygons: &str, initial_circles: &str) -> Markup {
    html! {
        script {
            (PreEscaped(include_str!("js/map-init.js")))
            (PreEscaped(include_str!("js/update-functions.js")))
            (PreEscaped(format!(
                "let initialPolygons = {};
                let initialCircles = {};
                updatePolygons(initialPolygons);
                updateCircles(initialCircles);",
                initial_polygons, initial_circles
            )))
            (PreEscaped(include_str!("js/event-source.js")))
        }
    }
}

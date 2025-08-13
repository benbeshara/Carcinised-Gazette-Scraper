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
use futures::stream::Stream;
use maud::{html, Markup, PreEscaped};
use std::net::SocketAddr;
use std::{convert::Infallible, env, time::Duration};

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
    let updater = Updater {
        uri: "http://www.gazette.vic.gov.au/gazette_bin/gazette_archives.cfm".to_string(),
        base_uri: "http://www.gazette.vic.gov.au".to_string(),
    };

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    tokio::spawn(async move {
        let update_task = tokio::spawn(async move {
            let _ = updater.update().await;
        });

        while !update_task.is_finished() {
            let _ = tx.send(Ok(Event::default()
                .data("updating...")
                .event("heartbeat"))).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let _ = update_task.await;

        let data_future = render_list();
        let polygons_future = fetch_polygons();
        let (data, polygons) = tokio::join!(data_future, polygons_future);

        let _ = tx.send(Ok(Event::default()
            .data("update complete")
            .event("update"))).await;
        let _ = tx.send(Ok(Event::default()
            .data(data)
            .event("list"))).await;
        let _ = tx.send(Ok(Event::default()
            .data(polygons)
            .event("close"))).await;
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(25))
            .text("keep-alive-text"),
    )
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
    let base_uri = env::var("OBJECT_STORAGE_URL").unwrap_or_default();
    if let Ok(gazettes) = db.fetch_entries().await {
        let mut feature_collection = GeoJsonFeatureCollection::new();

        for gazette in gazettes {
            if let Some(polygon) = &gazette.polygon {
                let mut start = String::new();
                let mut end = String::new();
                let mut img_uri = None;

                if let Some(start_date) = &gazette.start {
                    start = start_date.format("%Y-%m-%d").to_string();
                }

                if let Some(end_date) = &gazette.end {
                    end = end_date.format("%Y-%m-%d").to_string();
                }

                if let Some(img) = &gazette.img_uri {
                    img_uri = Some(format!("{}{}", base_uri, img));
                }

                let processed_polygon = polygon
                    .clone()
                    .remove_isolated_points(5.0, 2)
                    .remove_identical_points()
                    .clone();

                let geometry;

                if processed_polygon.data.len() < 3 {
                    geometry = GeoJsonGeometry::Point {
                        coordinates: processed_polygon.centre().into(),
                    };
                } else {
                    geometry = GeoJsonGeometry::Polygon {
                        coordinates: vec![processed_polygon
                            .convex_hull()
                            .data
                            .iter()
                            .map(|point| [point.longitude, point.latitude])
                            .collect::<Vec<[f64; 2]>>()],
                    };
                }

                let feature = GeoJsonFeature {
                    type_field: "Feature".to_string(),
                    geometry,
                    properties: GeoJsonProperties {
                        title: gazette.title,
                        uri: gazette.uri,
                        img_uri,
                        start,
                        end,
                    },
                };

                feature_collection.features.push(feature);
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
    let base_uri = env::var("OBJECT_STORAGE_URL").unwrap_or_default();
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
                            a href=(&gz.uri) target="_blank" {
                                img src=(format!("{}{}", &base_uri, &img_uri)) {}
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
        (map_javascript(&initial_polygons))
    })
}

fn map_javascript(initial_polygons: &str) -> Markup {
    html! {
        script {
            (PreEscaped(include_str!("js/map-init.js")))
            (PreEscaped(include_str!("js/update-functions.js")))
            (PreEscaped(format!(
                "let initialPolygons = {};
                updatePolygons(initialPolygons);",
                initial_polygons
            )))
            (PreEscaped(include_str!("js/event-source.js")))
        }
    }
}

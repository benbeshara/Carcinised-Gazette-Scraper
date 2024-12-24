use futures::stream::{self, Stream};
use tokio_stream::StreamExt as _;
use std::{convert::Infallible, time::Duration};
use std::net::SocketAddr;
use axum::{self, routing::get, Router, response::sse::{Sse, Event}};
use maud::{html, Markup};
use crate::update_pdfs;

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
    let gazette_stream = stream::once(async{ Event::default().data(send_data().await).event("list")}).map(Ok);

    Sse::new(gazette_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

async fn send_data() -> String {
    match update_pdfs().await {
        Ok(_) => println!("PDF Update succeeded"),
        Err(e) => println!("PDF Update failed: {:?}", e.to_string()),
    }

    if let Ok(gazettes) = crate::retrieve_gazettes_from_redis().await {
        let acc = gazettes.iter().fold(String::new(), |mut acc, gz| {
            acc += html!(
                li {
                    span {
                        a href=(gz.uri) target="_blank" {
                            (gz.title)
                            span.uri {
                                (gz.uri)
                            }
                        }
                    }
                }
            ).into_string().as_str();

            acc
        });
        return acc;
    }
    "Failed to fetch data".to_string()
}

async fn landing() -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                title {
                    "Control of Weapons Acts"
                }
                meta name="viewport" content="width=device-width";
                link rel="icon" type="image/x-icon" href="favicon.ico";
                script type="text/javascript" src="https://unpkg.com/htmx.org@2.0.0-beta3"{}
                script type="text/javascript" src="https://unpkg.com/htmx-ext-sse@2.1.0/sse.js"{}
            }
            body {
                div.center {
                    span.heading {
                        "Control of Weapons Act Notices"
                    }
                    span.subheading {
                        "Gazettes sourced from the Victorian Gazette website"
                    }
                    ul {
                        li hx-ext="sse" sse-connect="/data" sse-close="close" sse-swap="list" hx-swap="outerHTML" {
                            span hx-swap="innerHTML" sse-swap="heartbeat" {
                                "Loading (If there's a lot of new gazettes, this could take some time)"
                            }
                        }

                    }
                    a class="attribution" href="https://github.com/benbeshara/Carcinised-Gazette-Scraper" target="_blank" {
                        "Source available here under the permissive AGPL-3.0 license"
                    }
                }
                (stylesheet())
            }
        }
    }
}

fn stylesheet() -> Markup {
    html!(
    style {
        "html {
                background-color: #225;
                color: #ccc;
                font-family: sans-serif;
            }
            div.center {
                margin: auto;
                width: 60%;
            }
            span.heading {
                font-size: 1.8rem;
                display: block;
                margin-top: 1rem;
            }
            span.subheading {
                font-size: 1.4rem;
                display: block;
                word-wrap: break-word;
                white-space: normal;
                margin-bottom: 1rem;
            }
            span.uri {
                font-size: 0.9rem;
                color: #aaa;
                display: block;
                word-wrap: break-word;
            }
            a {
                color: #ccc;
                text-decoration: none;
                font-size: 1.2rem;
            }
            ul {
                margin: 0;
                padding: 0;
                list-style-type: none;
            }
            li {
                padding: 0.5em 1rem;
                margin: 0 -1rem;
            }
            li:hover {
                background-color: #447;
            }
            li:nth-child(2n) {
                background-color: #114;
            }
            li:nth-child(2n):hover {
                background-color: #225;
            }
            .attribution {
                margin: 1rem 0;
                font-size: 0.65rem;
                display: block;
            }
            @media (max-width: 430px) {
                div.center {
                    width: 95%;
                }
                span.uri {
                    font-size: 1rem;
                    padding-top: 0.5rem;
                }
                a {
                    font-size: 1.4rem;
                }
                li {
                    padding: 1rem;
                    margin: 0;
                    background-color: #336;
                }
                .attribution {
                    font-size: 0.8rem;
                }
            }"
        }
    )
}

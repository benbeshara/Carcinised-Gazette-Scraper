use serde::Serialize;
use crate::utils::maptypes::GeoPosition;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum GeoJsonGeometry {
    #[serde(rename = "Point")]
    Point {
        coordinates: [f64; 2]
    },
    #[serde(rename = "Polygon")]
    Polygon {
        coordinates: Vec<Vec<[f64; 2]>>
    }
}

#[derive(Serialize)]
pub struct GeoJsonFeatureCollection {
    #[serde(rename = "type")]
    pub type_field: String,
    pub features: Vec<GeoJsonFeature>,
}

#[derive(Serialize)]
pub struct GeoJsonFeature {
    #[serde(rename = "type")]
    pub type_field: String,
    pub geometry: GeoJsonGeometry,
    pub properties: GeoJsonProperties,
}

#[derive(Serialize)]
pub struct GeoJsonProperties {
    pub title: Option<String>,
    pub uri: String,
    pub img_uri: Option<String>,
}

impl GeoJsonFeatureCollection {
    pub fn new() -> Self {
        Self {
            type_field: "FeatureCollection".to_string(),
            features: Vec::new(),
        }
    }
}

impl From<GeoPosition> for [f64; 2] {
    fn from(pos: GeoPosition) -> Self {
        [pos.longitude, pos.latitude]
    }
}

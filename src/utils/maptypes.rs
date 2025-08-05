use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeoPosition {
    pub latitude: f64,
    pub longitude: f64,
}

pub trait Sanitise {
    fn sanitise(&mut self);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapPolygon {
    pub data: Vec<GeoPosition>,
}

impl From<MapPolygon> for String {
    fn from(value: MapPolygon) -> String {
        format!(
            "[{}]",
            value
                .data
                .iter()
                .map(|f| format!("[{}, {}]", f.latitude, f.longitude))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Display for MapPolygon {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}]",
            self.data
                .iter()
                .map(|f| format!("[{}, {}]", f.latitude, f.longitude))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Sanitise for Vec<GeoPosition> {
    // This is a basic check to se that google hasn't returned WILDLY bad data; it is probably still going to give us bad data though
    fn sanitise(&mut self) {
        let victoria_lat_range = -39.2..=-33.9;
        let victoria_lon_range = 140.7..=149.0;

        let _ = &self.retain(|point| {
            victoria_lat_range.contains(&point.latitude)
                && victoria_lon_range.contains(&point.longitude)
        });
    }
}

impl Display for GeoPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}, {:?}]", self.latitude, self.longitude)
    }
}

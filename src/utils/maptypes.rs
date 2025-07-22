use crate::geocoder::geoposition::GeoPosition;
use itertools::Itertools;

pub struct MapPolygon {
    pub data: Vec<GeoPosition>,
}

impl From<MapPolygon> for String {
    fn from(value: MapPolygon) -> String {
        let mut output = String::new();
        output = value.data.iter().map(|f| f.to_string()).join(", ");
        format!("[{}]", output)
    }
}
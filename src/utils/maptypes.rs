use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
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

impl MapPolygon {
    pub fn centre(&self) -> GeoPosition {
        let count = self.data.len() as f64;
        let sum = self.data.iter().fold(
            GeoPosition {
                latitude: 0.0,
                longitude: 0.0,
            },
            |mut acc, pos| {
                acc.latitude += pos.latitude;
                acc.longitude += pos.longitude;
                acc
            },
        );

        GeoPosition {
            latitude: sum.latitude / count,
            longitude: sum.longitude / count,
        }
    }

    pub fn convex_hull(&self) -> Self {
        let points = self.data.clone();
        if points.len() < 3 {
            return MapPolygon {
                data: points.to_vec(),
            };
        }

        let mut bottom_point = 0;
        for i in 1..points.len() {
            if points[i].latitude < points[bottom_point].latitude
                || (points[i].latitude == points[bottom_point].latitude
                    && points[i].longitude < points[bottom_point].longitude)
            {
                bottom_point = i;
            }
        }

        let mut hull_points: Vec<(f64, GeoPosition)> = points
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != bottom_point)
            .map(|(_, p)| {
                let dx = p.longitude - points[bottom_point].longitude;
                let dy = p.latitude - points[bottom_point].latitude;
                let angle = (dy).atan2(dx);
                (angle, p.clone())
            })
            .collect();

        hull_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut hull = vec![points[bottom_point].clone()];

        for (_angle, point) in hull_points {
            while hull.len() >= 2 {
                let next_to_top = &hull[hull.len() - 2];
                let top = &hull[hull.len() - 1];

                if GeoPosition::orientation(next_to_top, top, &point) > 0.0 {
                    break;
                }
                hull.pop();
            }
            hull.push(point);
        }

        if !hull.is_empty() && hull[0] != *hull.last().unwrap() {
            hull.push(hull[0].clone());
        }

        MapPolygon { data: hull }
    }

    pub fn remove_outliers_by_proximity(&mut self, threshold: f64, buffer_km: f64) -> &mut Self {
        if self.data.len() < 4 {
            return self;
        }

        let nearest_distances: Vec<f64> = self
            .data
            .iter()
            .map(|point| {
                self.data
                    .iter()
                    .filter(|&other| other != point)
                    .map(|other| point.distance_to(other))
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(f64::MAX)
            })
            .collect();

        let mean_distance = nearest_distances.iter().sum::<f64>() / nearest_distances.len() as f64;

        let variance: f64 = nearest_distances
            .iter()
            .map(|&d| (d - mean_distance).powi(2))
            .sum::<f64>()
            / nearest_distances.len() as f64;
        let std_dev = variance.sqrt();

        let filtered_data: Vec<GeoPosition> = self
            .data
            .iter()
            .zip(nearest_distances.iter())
            .filter(|(_, &distance)| {
                if distance <= buffer_km {
                    return true;
                }
                let z_score = (distance - mean_distance).abs() / std_dev;
                z_score <= threshold
            })
            .map(|(pos, _)| pos.clone())
            .collect();

        self.data = filtered_data;
        self
    }

    pub fn remove_isolated_points(&mut self, distance_km: f64, min_neighbours: usize) -> &mut Self {
        if self.data.len() < 4 {
            return self;
        }

        let filtered_data: Vec<GeoPosition> = self
            .data
            .iter()
            .filter(|&point| {
                let neighbour_count = self
                    .data
                    .iter()
                    .filter(|&other| other != point && point.distance_to(other) <= distance_km)
                    .count();
                neighbour_count >= min_neighbours
            })
            .cloned()
            .collect();

        self.data = filtered_data;
        self
    }

    pub fn remove_identical_points(&mut self) -> &mut Self {
        if self.data.len() < 3 {
            return self;
        }

        let mut filtered_data: Vec<GeoPosition> = Vec::new();

        for point in self.data.iter() {
            if !filtered_data
                .iter()
                .any(|p| p.latitude == point.latitude && p.longitude == point.longitude)
            {
                filtered_data.push(point.clone());
            }
        }

        self.data = filtered_data;
        self
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

impl GeoPosition {
    fn distance_to(&self, other: &GeoPosition) -> f64 {
        let r = 6371.0; // Earth's radius in kilometers

        let d_lat = (other.latitude - self.latitude).to_radians();
        let d_lng = (other.longitude - self.longitude).to_radians();
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();

        let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        r * c
    }

    fn orientation(p: &GeoPosition, q: &GeoPosition, r: &GeoPosition) -> f64 {
        (q.longitude - p.longitude) * (r.latitude - p.latitude)
            - (q.latitude - p.latitude) * (r.longitude - p.longitude)
    }
}

impl Display for GeoPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}, {:?}]", self.latitude, self.longitude)
    }
}

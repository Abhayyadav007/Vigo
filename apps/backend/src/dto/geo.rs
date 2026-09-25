use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

/// India's bounding box (with a little margin). Vigo is India-only.
pub const INDIA_LAT: std::ops::RangeInclusive<f64> = 6.0..=37.5;
pub const INDIA_LNG: std::ops::RangeInclusive<f64> = 68.0..=97.5;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate, TS)]
#[ts(export)]
pub struct LatLng {
    #[validate(range(min = 6.0, max = 37.5, message = "must be inside India"))]
    pub lat: f64,
    #[validate(range(min = 68.0, max = 97.5, message = "must be inside India"))]
    pub lng: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum PolygonType {
    Polygon,
}

/// A GeoJSON Polygon: one outer ring of `[lng, lat]` positions (no holes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GeoJsonPolygon {
    #[serde(rename = "type")]
    pub kind: PolygonType,
    pub coordinates: Vec<Vec<[f64; 2]>>,
}

impl GeoJsonPolygon {
    /// Structural checks PostGIS can't phrase nicely. Closes the ring if the
    /// client sent it open. Geometric validity (self-intersection) is checked
    /// in Postgres with `ST_IsValid`.
    pub fn normalized(mut self) -> Result<Self, String> {
        if self.coordinates.len() != 1 {
            return Err("service area must be a single ring without holes".into());
        }
        let ring = &mut self.coordinates[0];
        if ring.first() != ring.last()
            && let Some(&first) = ring.first()
        {
            ring.push(first);
        }
        if ring.len() < 4 {
            return Err("service area needs at least 3 distinct points".into());
        }
        if ring.len() > 500 {
            return Err("service area has too many points (max 500)".into());
        }
        if ring
            .iter()
            .any(|[lng, lat]| !INDIA_LNG.contains(lng) || !INDIA_LAT.contains(lat))
        {
            return Err("service area must be inside India".into());
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(ring: Vec<[f64; 2]>) -> GeoJsonPolygon {
        GeoJsonPolygon {
            kind: PolygonType::Polygon,
            coordinates: vec![ring],
        }
    }

    #[test]
    fn closes_open_rings() {
        let p = poly(vec![[77.6, 12.9], [77.7, 12.9], [77.7, 13.0]])
            .normalized()
            .unwrap();
        assert_eq!(p.coordinates[0].len(), 4);
        assert_eq!(p.coordinates[0][0], p.coordinates[0][3]);
    }

    #[test]
    fn rejects_bad_shapes() {
        assert!(poly(vec![[77.6, 12.9], [77.7, 12.9]]).normalized().is_err());
        assert!(
            poly(vec![[-0.1, 51.5], [0.0, 51.5], [0.0, 51.6]])
                .normalized()
                .is_err(),
            "London is not in India"
        );
        let holes = GeoJsonPolygon {
            kind: PolygonType::Polygon,
            coordinates: vec![vec![], vec![]],
        };
        assert!(holes.normalized().is_err());
    }
}

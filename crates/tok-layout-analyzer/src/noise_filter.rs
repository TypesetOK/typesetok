//! Marginalia and ink-blot noise filter for scanned document pages.
//!
//! Removes tiny specks, scanner edge artifacts, and isolated outer-margin smudges
//! while preserving legitimate narrow marginalia columns (e.g. Ein Mishpat / Masoret HaShas).

use crate::profile::{DetectedRegion, RegionClass};

/// Filters out micro-artifacts and classifies narrow outer-margin reference blocks.
pub fn filter_noise_and_tag_marginalia(regions: Vec<DetectedRegion>) -> Vec<DetectedRegion> {
    regions
        .into_iter()
        .filter(|r| {
            if r.region_class == RegionClass::Rule {
                return r.bbox.width() >= 0.10;
            }
            // Drop tiny specks (< 0.08% of page area and < 1.2% height).
            let area = r.bbox.area();
            !(area < 0.0008 && r.bbox.height() < 0.012)
        })
        .map(|mut r| {
            if r.region_class == RegionClass::Text {
                let near_outer_edge = r.bbox.x0 <= 0.10 || r.bbox.x1 >= 0.90;
                let narrow = r.bbox.width() <= 0.14 && r.bbox.height() <= 0.25;
                if near_outer_edge && narrow && r.bbox.area() < 0.025 {
                    r.region_class = RegionClass::Marginalia;
                }
            }
            r
        })
        .collect()
}

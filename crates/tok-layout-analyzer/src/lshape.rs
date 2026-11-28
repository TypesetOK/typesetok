//! L-Shape signature detector for Tzurat HaDaf (Talmud / Shas) pages.
//!
//! In classic Tzurat HaDaf, the central Gemara block occupies `35%–48%` of the page width
//! flanked by Rashi (inner spine, `22%–32%`) and Tosafot (outer margin, `22%–32%`).
//! When the central Gemara block ends above the bottom of the page, one or both commentary
//! columns expand horizontally into an L-shape beneath the central block.

use crate::profile::{DetectedRegion, RegionClass};

/// Diagnostic result of L-Shape topology detection on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct LShapeAnalysis {
    /// Whether a central column + flanking side columns + lower expansion was detected.
    pub detected: bool,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Estimated central column X-span `(x0, x1)` if found.
    pub central_x_span: Option<(f32, f32)>,
}

/// Detects whether `regions` exhibit the Tzurat HaDaf 3-zone + L-shape wrapping signature.
pub fn detect_l_shape_signature(regions: &[DetectedRegion]) -> LShapeAnalysis {
    let text_regions: Vec<&DetectedRegion> = regions
        .iter()
        .filter(|r| matches!(r.region_class, RegionClass::Text | RegionClass::Marginalia))
        .collect();

    if text_regions.len() < 3 {
        return LShapeAnalysis {
            detected: false,
            confidence: 0.0,
            central_x_span: None,
        };
    }

    // Find candidate central region: center_x near [0.35, 0.65], width in [0.28, 0.55].
    let central_opt = text_regions
        .iter()
        .filter(|r| {
            let cx = r.bbox.center_x();
            let w = r.bbox.width();
            (0.32..=0.68).contains(&cx) && (0.25..=0.56).contains(&w) && r.bbox.height() >= 0.15
        })
        .max_by(|a, b| a.bbox.area().total_cmp(&b.bbox.area()))
        .copied();

    let Some(central) = central_opt else {
        return LShapeAnalysis {
            detected: false,
            confidence: 0.0,
            central_x_span: None,
        };
    };

    let has_left = text_regions.iter().any(|r| {
        r.bbox.x1 <= central.bbox.x0 + 0.03
            && r.bbox.vertical_overlap_ratio(&central.bbox) >= 0.25
            && r.bbox.width() >= 0.12
    });

    let has_right = text_regions.iter().any(|r| {
        r.bbox.x0 >= central.bbox.x1 - 0.03
            && r.bbox.vertical_overlap_ratio(&central.bbox) >= 0.25
            && r.bbox.width() >= 0.12
    });

    // Check if any commentary block wraps beneath the central block (crossing central's X span).
    let has_under_wrap = text_regions
        .iter()
        .any(|r| !std::ptr::eq(*r, central) && r.bbox.wraps_under(&central.bbox, 0.20));

    let mut score = 0.0_f32;
    if has_left && has_right {
        score += 0.55;
    } else if has_left || has_right {
        score += 0.25;
    }
    if has_under_wrap {
        score += 0.40;
    }

    LShapeAnalysis {
        detected: score >= 0.80,
        confidence: score.clamp(0.0, 1.0),
        central_x_span: Some((central.bbox.x0, central.bbox.x1)),
    }
}

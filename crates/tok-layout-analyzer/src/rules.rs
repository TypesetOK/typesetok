//! Horizontal rule and decorative ornament frame detection.

use crate::geometry::NormalizedBBox;
use crate::profile::{DetectedRegion, RegionClass};
use crate::raster::GrayImage;

/// Detects thin horizontal separator rules (e.g. above footnote bands) in a grayscale image.
pub fn detect_horizontal_rules(img: &GrayImage, ink_threshold: u8) -> Vec<DetectedRegion> {
    let w = img.width as usize;
    let h = img.height as usize;
    if w < 32 || h < 32 {
        return Vec::new();
    }

    let mut rules = Vec::new();
    let mut y = 0usize;
    while y < h {
        let mut run_start: Option<usize> = None;
        let mut best_run: Option<(usize, usize)> = None;

        for x in 0..w {
            if img.data[y * w + x] < ink_threshold {
                if run_start.is_none() {
                    run_start = Some(x);
                }
            } else if let Some(s) = run_start.take() {
                if x - s > best_run.map(|(bs, be)| be - bs).unwrap_or(0) {
                    best_run = Some((s, x));
                }
            }
        }
        if let Some(s) = run_start {
            if w - s > best_run.map(|(bs, be)| be - bs).unwrap_or(0) {
                best_run = Some((s, w));
            }
        }

        if let Some((rx0, rx1)) = best_run {
            let run_w_ratio = (rx1 - rx0) as f32 / w as f32;
            if run_w_ratio >= 0.22 {
                // Measure vertical thickness of this horizontal stroke.
                let mut y_end = y + 1;
                while y_end < h && y_end - y <= 8 {
                    let row_ink = (rx0..rx1)
                        .filter(|&x| img.data[y_end * w + x] < ink_threshold)
                        .count();
                    if (row_ink as f32 / (rx1 - rx0) as f32) >= 0.65 {
                        y_end += 1;
                    } else {
                        break;
                    }
                }
                let thickness = y_end - y;
                if thickness <= 6 {
                    rules.push(DetectedRegion {
                        bbox: NormalizedBBox::new(
                            rx0 as f32 / w as f32,
                            y as f32 / h as f32,
                            rx1 as f32 / w as f32,
                            y_end as f32 / h as f32,
                        ),
                        region_class: RegionClass::Rule,
                        confidence: 0.92,
                        font_size_pt: None,
                        font_name: None,
                        median_line_height: None,
                        is_rashi_script: None,
                    });
                    y = y_end + 2;
                    continue;
                }
            }
        }
        y += 1;
    }

    rules
}

/// Checks whether a horizontal rule sits above a dense bottom footnote band (`y0 >= 0.50`).
pub fn has_footnote_rule_above_band(regions: &[DetectedRegion]) -> bool {
    let rules: Vec<&DetectedRegion> = regions
        .iter()
        .filter(|r| r.region_class == RegionClass::Rule && r.bbox.y0 >= 0.45 && r.bbox.y1 <= 0.92)
        .collect();

    if rules.is_empty() {
        return false;
    }

    for rule in rules {
        let has_below = regions.iter().any(|r| {
            matches!(r.region_class, RegionClass::Text | RegionClass::Footnote)
                && r.bbox.y0 >= rule.bbox.y1 - 0.01
                && r.bbox.height() >= 0.04
        });
        if has_below {
            return true;
        }
    }

    false
}

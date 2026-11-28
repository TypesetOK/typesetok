//! Spatial and typographic feature extraction into [`PageSpatialProfile`].

use crate::lshape::detect_l_shape_signature;
use crate::profile::{DetectedRegion, PageSpatialProfile, RegionClass};
use crate::rules::has_footnote_rule_above_band;
use crate::script_classifier::aggregate_page_script_mode;
use crate::spread::SpreadSide;
use crate::typography::TypographyMetrics;

/// Builds a complete [`PageSpatialProfile`] from a page's detected regions and metadata.
pub fn extract_page_spatial_profile(
    page_index: usize,
    aspect_ratio: f32,
    regions: Vec<DetectedRegion>,
    source_kind: impl Into<String>,
) -> PageSpatialProfile {
    let spread_side = SpreadSide::from_page_index(page_index);
    let body_regions: Vec<&DetectedRegion> = regions
        .iter()
        .filter(|r| {
            matches!(
                r.region_class,
                RegionClass::Text | RegionClass::Footnote | RegionClass::Title
            )
        })
        .collect();

    let text_coverage: f32 = body_regions
        .iter()
        .map(|r| r.bbox.area())
        .sum::<f32>()
        .clamp(0.0, 1.0);

    let (effective_column_count, column_proportions, estimated_gutter_ratio) =
        estimate_columns_from_regions(&body_regions);

    let horizontal_asymmetry = compute_horizontal_asymmetry(&body_regions);
    let lshape = detect_l_shape_signature(&regions);
    let has_footnote_rule_and_band = has_footnote_rule_above_band(&regions);
    let has_spanning_header = detect_spanning_header(&body_regions, effective_column_count);
    let has_title_ornament_or_whitespace =
        detect_title_or_sparse_opening(&regions, &body_regions, text_coverage);

    let typography = compute_typography_metrics(&regions);

    PageSpatialProfile {
        page_index,
        aspect_ratio,
        spread_side,
        regions,
        effective_column_count,
        column_proportions,
        estimated_gutter_ratio,
        horizontal_asymmetry,
        text_coverage,
        has_l_shape_signature: lshape.detected,
        has_footnote_rule_and_band,
        has_spanning_header,
        has_title_ornament_or_whitespace,
        typography,
        source_kind: source_kind.into(),
    }
}

fn estimate_columns_from_regions(regions: &[&DetectedRegion]) -> (usize, Vec<f32>, f32) {
    // Focus on main vertical body bands (exclude tiny headers/footers).
    let mut intervals: Vec<(f32, f32)> = regions
        .iter()
        .filter(|r| r.bbox.height() >= 0.08 && r.bbox.width() >= 0.08)
        .map(|r| (r.bbox.x0, r.bbox.x1))
        .collect();

    if intervals.is_empty() {
        return (1, vec![1.0], 0.0);
    }

    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Cluster overlapping X-intervals into distinct vertical column tracks.
    let mut clusters: Vec<(f32, f32)> = Vec::new();
    for (x0, x1) in intervals {
        if let Some(last) = clusters.last_mut() {
            // Merge if substantial horizontal overlap exists (> 35% of smaller width).
            let inter = (last.1.min(x1) - last.0.max(x0)).max(0.0);
            let min_w = (last.1 - last.0).min(x1 - x0).max(1e-4);
            if inter / min_w > 0.35 {
                last.0 = last.0.min(x0);
                last.1 = last.1.max(x1);
                continue;
            }
        }
        clusters.push((x0, x1));
    }

    let count = clusters.len().clamp(1, 6);
    let widths: Vec<f32> = clusters.iter().map(|(a, b)| (b - a).max(0.01)).collect();
    let total_w: f32 = widths.iter().sum::<f32>().max(1e-4);
    let proportions: Vec<f32> = widths.iter().map(|w| w / total_w).collect();

    let gutter = if clusters.len() >= 2 {
        let mut gaps = Vec::new();
        for pair in clusters.windows(2) {
            let g = (pair[1].0 - pair[0].1).max(0.0);
            gaps.push(g);
        }
        gaps.iter().sum::<f32>() / gaps.len() as f32
    } else {
        0.0
    };

    (count, proportions, gutter)
}

fn compute_horizontal_asymmetry(regions: &[&DetectedRegion]) -> f32 {
    if regions.is_empty() {
        return 0.0;
    }
    let mut left_area = 0.0_f32;
    let mut right_area = 0.0_f32;
    for r in regions {
        let a = r.bbox.area();
        if r.bbox.center_x() < 0.5 {
            left_area += a;
        } else {
            right_area += a;
        }
    }
    let total = (left_area + right_area).max(1e-4);
    ((left_area - right_area).abs() / total).clamp(0.0, 1.0)
}

fn detect_spanning_header(regions: &[&DetectedRegion], col_count: usize) -> bool {
    if col_count < 2 {
        return false;
    }
    regions
        .iter()
        .any(|r| r.bbox.y0 <= 0.25 && r.bbox.width() >= 0.58 && r.bbox.height() <= 0.16)
}

fn detect_title_or_sparse_opening(
    all_regions: &[DetectedRegion],
    body_regions: &[&DetectedRegion],
    text_coverage: f32,
) -> bool {
    let has_ornament = all_regions
        .iter()
        .any(|r| r.region_class == RegionClass::Ornament);
    if has_ornament && text_coverage < 0.45 {
        return true;
    }
    if text_coverage < 0.30 {
        let min_y0 = body_regions
            .iter()
            .map(|r| r.bbox.y0)
            .fold(1.0_f32, f32::min);
        if min_y0 >= 0.25 || text_coverage < 0.18 {
            return true;
        }
    }
    false
}

fn compute_typography_metrics(regions: &[DetectedRegion]) -> TypographyMetrics {
    let (script_mode, has_square_vs_rashi_contrast) = aggregate_page_script_mode(regions);

    let mut sizes: Vec<f32> = regions
        .iter()
        .filter_map(|r| {
            r.font_size_pt
                .or_else(|| r.median_line_height.map(|h| h * 700.0))
        })
        .collect();

    if sizes.is_empty() {
        return TypographyMetrics {
            script_mode,
            has_square_vs_rashi_contrast,
            ..Default::default()
        };
    }

    sizes.sort_by(|a, b| a.total_cmp(b));
    let min_sz = sizes[0];
    let max_sz = sizes[sizes.len() - 1];
    let median_sz = sizes[sizes.len() / 2];

    // Count distinct size tiers separated by at least 15%.
    let mut tiers = vec![sizes[0]];
    for &s in &sizes[1..] {
        if let Some(&last) = tiers.last() {
            if s > last * 1.15 {
                tiers.push(s);
            }
        }
    }

    let primary_to_secondary_ratio = if min_sz > 0.5 && max_sz > min_sz * 1.15 {
        Some(median_sz / min_sz)
    } else {
        Some(1.0)
    };

    let max_to_body_ratio = if median_sz > 0.5 {
        Some(max_sz / median_sz)
    } else {
        None
    };

    TypographyMetrics {
        primary_font_size_pt: Some(median_sz),
        secondary_font_size_pt: Some(min_sz),
        primary_to_secondary_ratio,
        max_to_body_ratio,
        font_size_tier_count: tiers.len(),
        script_mode,
        has_square_vs_rashi_contrast,
        has_cantillation_or_masoretic_marks: false,
    }
}

//! Recursive XY-Cut segmentation algorithm for clean scans and vector regions.

use crate::geometry::NormalizedBBox;
use crate::profile::{DetectedRegion, RegionClass};
use crate::projection::{compute_projection_profiles, find_valleys};
use crate::raster::GrayImage;

/// Configuration parameters for recursive XY-Cut segmentation.
#[derive(Debug, Clone, Copy)]
pub struct XyCutConfig {
    /// Ink threshold below which a grayscale pixel counts as foreground.
    pub ink_threshold: u8,
    /// Minimum normalized horizontal gap (gutter) required to split columns.
    pub min_col_gap_ratio: f32,
    /// Minimum normalized vertical gap required to split horizontal bands.
    pub min_row_gap_ratio: f32,
    /// Maximum ink density in a valley to be considered whitespace.
    pub valley_ink_threshold: f32,
    /// Maximum recursion depth.
    pub max_depth: usize,
}

impl Default for XyCutConfig {
    fn default() -> Self {
        Self {
            ink_threshold: 140,
            min_col_gap_ratio: 0.015,
            min_row_gap_ratio: 0.018,
            valley_ink_threshold: 0.012,
            max_depth: 5,
        }
    }
}

/// Segments a binarized/grayscale page image into non-overlapping [`DetectedRegion`] blocks
/// using recursive XY-Cut.
pub fn segment_page_xycut(img: &GrayImage, cfg: XyCutConfig) -> Vec<DetectedRegion> {
    if img.width == 0 || img.height == 0 {
        return Vec::new();
    }
    let mut boxes = Vec::new();
    xycut_recurse(img, 0, 0, img.width, img.height, 0, cfg, &mut boxes);

    let w = img.width as f32;
    let h = img.height as f32;
    boxes
        .into_iter()
        .filter(|(x0, y0, x1, y1)| (*x1 - *x0) >= 8 && (*y1 - *y0) >= 8)
        .map(|(x0, y0, x1, y1)| DetectedRegion {
            bbox: NormalizedBBox::new(x0 as f32 / w, y0 as f32 / h, x1 as f32 / w, y1 as f32 / h),
            region_class: RegionClass::Text,
            confidence: 0.90,
            font_size_pt: None,
            font_name: None,
            median_line_height: None,
            is_rashi_script: None,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn xycut_recurse(
    img: &GrayImage,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    depth: usize,
    cfg: XyCutConfig,
    out: &mut Vec<(u32, u32, u32, u32)>,
) {
    if depth >= cfg.max_depth || x1 <= x0 + 8 || y1 <= y0 + 8 {
        out.push((x0, y0, x1, y1));
        return;
    }

    let prof = compute_projection_profiles(img, x0, y0, x1, y1, cfg.ink_threshold);
    let min_col_gap = ((img.width as f32 * cfg.min_col_gap_ratio).round() as usize).max(4);
    let min_row_gap = ((img.height as f32 * cfg.min_row_gap_ratio).round() as usize).max(4);

    // Filter out outer-edge valleys so we only split *internal* gaps.
    let v_valleys: Vec<(usize, usize)> = find_valleys(&prof.vertical_ink, cfg.valley_ink_threshold, min_col_gap)
        .into_iter()
        .filter(|&(s, e)| s > 2 && e + 2 < prof.vertical_ink.len())
        .collect();

    let h_valleys: Vec<(usize, usize)> = find_valleys(&prof.horizontal_ink, cfg.valley_ink_threshold, min_row_gap)
        .into_iter()
        .filter(|&(s, e)| s > 2 && e + 2 < prof.horizontal_ink.len())
        .collect();

    // Choose the widest normalized cut (preferring vertical column splits on ties).
    let best_v = v_valleys.iter().max_by_key(|(s, e)| e - s).copied();
    let best_h = h_valleys.iter().max_by_key(|(s, e)| e - s).copied();

    match (best_v, best_h) {
        (Some((vs, ve)), Some((hs, he))) => {
            let v_norm = (ve - vs) as f32 / img.width as f32;
            let h_norm = (he - hs) as f32 / img.height as f32;
            if v_norm >= h_norm {
                let mid = x0 + ((vs + ve) / 2) as u32;
                xycut_recurse(img, x0, y0, mid, y1, depth + 1, cfg, out);
                xycut_recurse(img, mid, y0, x1, y1, depth + 1, cfg, out);
            } else {
                let mid = y0 + ((hs + he) / 2) as u32;
                xycut_recurse(img, x0, y0, x1, mid, depth + 1, cfg, out);
                xycut_recurse(img, x0, mid, x1, y1, depth + 1, cfg, out);
            }
        }
        (Some((vs, ve)), None) => {
            let mid = x0 + ((vs + ve) / 2) as u32;
            xycut_recurse(img, x0, y0, mid, y1, depth + 1, cfg, out);
            xycut_recurse(img, mid, y0, x1, y1, depth + 1, cfg, out);
        }
        (None, Some((hs, he))) => {
            let mid = y0 + ((hs + he) / 2) as u32;
            xycut_recurse(img, x0, y0, x1, mid, depth + 1, cfg, out);
            xycut_recurse(img, x0, mid, x1, y1, depth + 1, cfg, out);
        }
        (None, None) => {
            out.push((x0, y0, x1, y1));
        }
    }
}

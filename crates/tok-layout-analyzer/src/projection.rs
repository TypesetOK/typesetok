//! Horizontal and vertical ink projection profiles for binarized page images.

use crate::raster::GrayImage;

/// Normalized 1D projection profiles along X (columns) and Y (rows).
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionProfiles {
    /// Fraction of ink pixels in each X column `[0.0, 1.0]`, length = `width`.
    pub vertical_ink: Vec<f32>,
    /// Fraction of ink pixels in each Y row `[0.0, 1.0]`, length = `height`.
    pub horizontal_ink: Vec<f32>,
}

/// Computes horizontal and vertical ink projection profiles inside `[x0, x1) x [y0, y1)`.
pub fn compute_projection_profiles(
    img: &GrayImage,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    ink_threshold: u8,
) -> ProjectionProfiles {
    let cx0 = x0.min(img.width) as usize;
    let cy0 = y0.min(img.height) as usize;
    let cx1 = x1.min(img.width).max(cx0 as u32) as usize;
    let cy1 = y1.min(img.height).max(cy0 as u32) as usize;

    let rw = (cx1 - cx0).max(1);
    let rh = (cy1 - cy0).max(1);
    let w = img.width as usize;

    let mut col_counts = vec![0u32; rw];
    let mut row_counts = vec![0u32; rh];

    for y in cy0..cy1 {
        let mut row_sum = 0u32;
        for x in cx0..cx1 {
            if img.data[y * w + x] < ink_threshold {
                col_counts[x - cx0] += 1;
                row_sum += 1;
            }
        }
        row_counts[y - cy0] = row_sum;
    }

    let vertical_ink = col_counts.into_iter().map(|c| c as f32 / rh as f32).collect();
    let horizontal_ink = row_counts.into_iter().map(|r| r as f32 / rw as f32).collect();

    ProjectionProfiles {
        vertical_ink,
        horizontal_ink,
    }
}

/// Finds contiguous white-space valleys (`val <= valley_threshold`) of at least `min_gap` elements.
pub fn find_valleys(profile: &[f32], valley_threshold: f32, min_gap: usize) -> Vec<(usize, usize)> {
    let mut valleys = Vec::new();
    let mut in_valley = false;
    let mut start = 0usize;

    for (i, &v) in profile.iter().enumerate() {
        if v <= valley_threshold {
            if !in_valley {
                in_valley = true;
                start = i;
            }
        } else if in_valley {
            in_valley = false;
            if i - start >= min_gap {
                valleys.push((start, i));
            }
        }
    }
    if in_valley && profile.len() - start >= min_gap {
        valleys.push((start, profile.len()));
    }

    valleys
}

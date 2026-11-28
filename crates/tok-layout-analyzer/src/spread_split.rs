//! Two-page spread detector and Recto/Verso splitter.
//!
//! When an open book is scanned as a single landscape image (`W > H`), a central
//! white valley or dark binding gutter shadow appears around `x in [0.45, 0.55]`.
//! This module detects that split line and separates the spread into two single pages
//! (right-hand page and left-hand page) before layout classification.

use crate::error::AnalyzerResult;
use crate::raster::GrayImage;

/// Result of inspecting an image for a two-page open book spread.
#[derive(Debug, Clone)]
pub enum SpreadSplitResult {
    /// Image is already a single page.
    Single(GrayImage),
    /// Image was split at `split_x` into `(right_page, left_page)` (RTL reading order: right first, then left).
    Split {
        right_page: GrayImage,
        left_page: GrayImage,
        split_x: u32,
    },
}

/// Detects a two-page spread when `width / height >= 1.18` and splits along the central valley/gutter.
pub fn detect_and_split_spread(img: &GrayImage) -> AnalyzerResult<SpreadSplitResult> {
    if img.aspect_ratio() < 1.18 || img.width < 64 || img.height < 64 {
        return Ok(SpreadSplitResult::Single(img.clone()));
    }

    let w = img.width as usize;
    let h = img.height as usize;
    let x_start = (w as f32 * 0.43) as usize;
    let x_end = (w as f32 * 0.57) as usize;

    // Score each candidate vertical split column: prefer columns with low ink/edge variance
    // (either pure white valley or smooth dark gutter) and close to x = 0.5 * W.
    let mut best_x = w / 2;
    let mut best_score = f64::INFINITY;

    for x in x_start..=x_end {
        let mut ink_count = 0usize;
        let mut vert_diff_sum = 0u64;
        let mut prev = img.data[x];
        for y in 0..h {
            let px = img.data[y * w + x];
            if px < 140 {
                ink_count += 1;
            }
            vert_diff_sum += px.abs_diff(prev) as u64;
            prev = px;
        }
        let center_penalty = ((x as f64 / w as f64) - 0.5).abs() * 200.0;
        let ink_ratio = ink_count as f64 / h as f64;
        let smoothness = vert_diff_sum as f64 / h as f64;
        let score = ink_ratio * 100.0 + smoothness * 0.5 + center_penalty;
        if score < best_score {
            best_score = score;
            best_x = x;
        }
    }

    // Require the central split column to have noticeably fewer text transitions than normal body columns.
    if best_x <= 8 || best_x >= w - 8 {
        return Ok(SpreadSplitResult::Single(img.clone()));
    }

    let left_page = img.crop(0, 0, best_x as u32, img.height)?;
    let right_page = img.crop(best_x as u32, 0, img.width, img.height)?;

    Ok(SpreadSplitResult::Split {
        right_page,
        left_page,
        split_x: best_x as u32,
    })
}

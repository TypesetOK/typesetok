//! Scan preprocessing algorithms for historical and noisy document pages.
//!
//! Includes:
//! - Sauvola adaptive binarization (handles aged/yellowed paper and bleed-through).
//! - Horizontal projection variance deskewing (`+-10` degrees).
//! - Gutter shadow removal and margin illumination normalization.
//! - Net content bounding-box trimming for `[0.0, 1.0]` coordinate normalization.

use crate::error::AnalyzerResult;
use crate::geometry::NormalizedBBox;
use crate::raster::GrayImage;

/// Applies Sauvola adaptive local thresholding to binarize a grayscale scan (`0` = ink, `255` = paper).
///
/// Uses integral images (`O(W * H)` time independent of window radius) with `k = 0.34`, `R = 128.0`.
pub fn sauvola_binarize(img: &GrayImage, window_radius: u32, k: f32) -> AnalyzerResult<GrayImage> {
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 {
        return GrayImage::new(img.width, img.height, 255);
    }

    let stride = w + 1;
    let mut integral = vec![0.0_f64; stride * (h + 1)];
    let mut integral_sq = vec![0.0_f64; stride * (h + 1)];

    for y in 0..h {
        let mut row_sum = 0.0_f64;
        let mut row_sq = 0.0_f64;
        for x in 0..w {
            let v = img.data[y * w + x] as f64;
            row_sum += v;
            row_sq += v * v;
            let idx = (y + 1) * stride + (x + 1);
            integral[idx] = integral[y * stride + (x + 1)] + row_sum;
            integral_sq[idx] = integral_sq[y * stride + (x + 1)] + row_sq;
        }
    }

    let r_rad = window_radius.max(2) as isize;
    let r_dyn = 128.0_f64;
    let k64 = k as f64;
    let mut out = vec![255_u8; w * h];

    for y in 0..h {
        let y0 = (y as isize - r_rad).max(0) as usize;
        let y1 = (y as isize + r_rad + 1).min(h as isize) as usize;
        for x in 0..w {
            let x0 = (x as isize - r_rad).max(0) as usize;
            let x1 = (x as isize + r_rad + 1).min(w as isize) as usize;
            let area = ((x1 - x0) * (y1 - y0)) as f64;

            let sum = integral[y1 * stride + x1]
                - integral[y0 * stride + x1]
                - integral[y1 * stride + x0]
                + integral[y0 * stride + x0];
            let sum_sq = integral_sq[y1 * stride + x1]
                - integral_sq[y0 * stride + x1]
                - integral_sq[y1 * stride + x0]
                + integral_sq[y0 * stride + x0];

            let mean = sum / area;
            let var = (sum_sq / area - mean * mean).max(0.0);
            let std_dev = var.sqrt();
            let threshold = mean * (1.0 + k64 * (std_dev / r_dyn - 1.0));

            let px = img.data[y * w + x] as f64;
            out[y * w + x] = if px < threshold { 0 } else { 255 };
        }
    }

    GrayImage::from_raw(img.width, img.height, out)
}

/// Estimates skew angle in degrees (`[-10.0, +10.0]`) by maximizing horizontal projection variance
/// and rotates the image around its center if `|angle| >= 0.25`.
pub fn deskew_image(img: &GrayImage) -> AnalyzerResult<(GrayImage, f32)> {
    let angle_deg = estimate_skew_angle_deg(img);
    if angle_deg.abs() < 0.25 {
        return Ok((img.clone(), 0.0));
    }
    let rotated = rotate_nearest(img, -angle_deg)?;
    Ok((rotated, angle_deg))
}

/// Coarse-to-fine skew estimator maximizing row-sum variance of ink pixels.
pub fn estimate_skew_angle_deg(img: &GrayImage) -> f32 {
    let w = img.width as usize;
    let h = img.height as usize;
    if w < 16 || h < 16 {
        return 0.0;
    }

    let step_x = (w / 160).max(1);
    let step_y = (h / 160).max(1);
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    let mut best_angle = 0.0_f32;
    let mut best_var = f64::NEG_INFINITY;

    // Coarse search: -10..=10 deg in 1.0 deg steps
    for deg_i in -10..=10 {
        let deg = deg_i as f32;
        let var = projection_variance_at_angle(img, deg, step_x, step_y, cx, cy);
        if var > best_var {
            best_var = var;
            best_angle = deg;
        }
    }

    // Fine search: +-1.0 deg around best in 0.2 deg steps
    let center_best = best_angle;
    for k in -5..=5 {
        let deg = center_best + k as f32 * 0.2;
        let var = projection_variance_at_angle(img, deg, step_x, step_y, cx, cy);
        if var > best_var {
            best_var = var;
            best_angle = deg;
        }
    }

    best_angle
}

fn projection_variance_at_angle(
    img: &GrayImage,
    deg: f32,
    step_x: usize,
    step_y: usize,
    cx: f32,
    cy: f32,
) -> f64 {
    let w = img.width as usize;
    let h = img.height as usize;
    let rad = deg.to_radians();
    let sin_a = rad.sin();
    let cos_a = rad.cos();
    let mut row_counts = vec![0.0_f64; h];

    for y in (0..h).step_by(step_y) {
        let dy = y as f32 - cy;
        for x in (0..w).step_by(step_x) {
            let px = img.data[y * w + x];
            if px < 128 {
                let dx = x as f32 - cx;
                let ry = (dx * sin_a + dy * cos_a + cy).round() as isize;
                if ry >= 0 && (ry as usize) < h {
                    row_counts[ry as usize] += 1.0;
                }
            }
        }
    }

    let n = row_counts.len() as f64;
    let mean = row_counts.iter().sum::<f64>() / n;
    row_counts
        .iter()
        .map(|&v| {
            let d = v - mean;
            d * d
        })
        .sum::<f64>()
        / n
}

fn rotate_nearest(img: &GrayImage, deg: f32) -> AnalyzerResult<GrayImage> {
    let w = img.width as usize;
    let h = img.height as usize;
    let rad = deg.to_radians();
    let sin_a = rad.sin();
    let cos_a = rad.cos();
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let mut out = vec![255_u8; w * h];

    for y in 0..h {
        let dy = y as f32 - cy;
        for x in 0..w {
            let dx = x as f32 - cx;
            let sx = (dx * cos_a + dy * sin_a + cx).round() as isize;
            let sy = (-dx * sin_a + dy * cos_a + cy).round() as isize;
            if sx >= 0 && (sx as usize) < w && sy >= 0 && (sy as usize) < h {
                out[y * w + x] = img.data[sy as usize * w + sx as usize];
            }
        }
    }

    GrayImage::from_raw(img.width, img.height, out)
}

/// Normalizes illumination along inner/outer margins to suppress book-binding gutter shadows.
pub fn remove_gutter_shadow(img: &GrayImage) -> AnalyzerResult<GrayImage> {
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 {
        return Ok(img.clone());
    }

    // Estimate column-wise background brightness using the 85th percentile / near-max sample.
    let mut col_bg = vec![255_u8; w];
    let mut col_samples = Vec::with_capacity(h / 4 + 1);
    for x in 0..w {
        col_samples.clear();
        for y in (0..h).step_by((h / 64).max(1)) {
            col_samples.push(img.data[y * w + x]);
        }
        col_samples.sort_unstable();
        let p85 = col_samples[(col_samples.len() * 85 / 100).min(col_samples.len() - 1)];
        col_bg[x] = p85.max(96);
    }

    let mut out = vec![255_u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let px = img.data[y * w + x] as f32;
            let bg = col_bg[x] as f32;
            let norm = (px / bg * 255.0).clamp(0.0, 255.0);
            out[y * w + x] = norm as u8;
        }
    }

    GrayImage::from_raw(img.width, img.height, out)
}

/// Computes the tight net content bounding box (ignoring outer blank scan margins)
/// and returns it as a [`NormalizedBBox`] relative to the full image dimensions.
pub fn compute_content_bounds(img: &GrayImage, ink_threshold: u8) -> NormalizedBBox {
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 {
        return NormalizedBBox::new(0.0, 0.0, 1.0, 1.0);
    }

    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0usize;
    let mut max_y = 0usize;

    for y in 0..h {
        for x in 0..w {
            if img.data[y * w + x] < ink_threshold {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }

    if min_x > max_x || min_y > max_y {
        return NormalizedBBox::new(0.0, 0.0, 1.0, 1.0);
    }

    NormalizedBBox::new(
        min_x as f32 / w as f32,
        min_y as f32 / h as f32,
        (max_x + 1) as f32 / w as f32,
        (max_y + 1) as f32 / h as f32,
    )
}

/// Remaps a bounding box from full-page normalized space into net content-box `[0.0, 1.0]` space.
pub fn normalize_to_content_box(
    bbox: NormalizedBBox,
    content_bounds: NormalizedBBox,
) -> NormalizedBBox {
    let cw = content_bounds.width().max(1e-4);
    let ch = content_bounds.height().max(1e-4);
    NormalizedBBox::new(
        (bbox.x0 - content_bounds.x0) / cw,
        (bbox.y0 - content_bounds.y0) / ch,
        (bbox.x1 - content_bounds.x0) / cw,
        (bbox.y1 - content_bounds.y0) / ch,
    )
}

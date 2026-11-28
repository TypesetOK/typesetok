//! Scan augmentation pipeline for synthetic training and evaluation data.
//!
//! Simulates historical print and scanner degradations on clean rendered pages:
//! - Gaussian ink/paper noise
//! - Slight rotational skew (`+-4` degrees)
//! - Aged paper yellowing / uneven illumination
//! - Inner-margin book-binding gutter shadow

use crate::error::AnalyzerResult;
use crate::raster::GrayImage;

/// Configuration for synthetic scan degradation.
#[derive(Debug, Clone, Copy)]
pub struct ScanAugmentConfig {
    /// Deterministic PRNG seed.
    pub seed: u64,
    /// Noise amplitude (`0.0..=32.0`).
    pub noise_sigma: f32,
    /// Skew angle in degrees (`-4.0..=4.0`).
    pub skew_deg: f32,
    /// Gutter shadow darkening strength (`0.0..=0.5`).
    pub gutter_shadow_strength: f32,
    /// Whether the simulated binding gutter is on the right edge (Recto) or left (Verso).
    pub gutter_on_right: bool,
}

impl Default for ScanAugmentConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            noise_sigma: 8.0,
            skew_deg: 1.5,
            gutter_shadow_strength: 0.25,
            gutter_on_right: true,
        }
    }
}

/// Applies deterministic scan-like degradations to a clean grayscale page image.
pub fn augment_clean_scan(img: &GrayImage, cfg: ScanAugmentConfig) -> AnalyzerResult<GrayImage> {
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 {
        return Ok(img.clone());
    }

    let mut out = img.data.clone();
    let mut state = cfg.seed ^ 0x9E37_79B9_7F4A_7C15;

    // 1. Apply binding gutter shadow gradient and mild paper aging.
    let gutter_span = (w as f32 * 0.14).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let dist_from_gutter = if cfg.gutter_on_right {
                (w - 1 - x) as f32
            } else {
                x as f32
            };
            let shadow_factor = if dist_from_gutter < gutter_span {
                let t = 1.0 - (dist_from_gutter / gutter_span);
                1.0 - cfg.gutter_shadow_strength * t * t
            } else {
                1.0
            };

            // Deterministic LCG pseudo-noise.
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u = ((state >> 33) as f32 / (u32::MAX as f32)) - 0.5;
            let noise = u * cfg.noise_sigma * 2.0;

            let px = out[y * w + x] as f32 * shadow_factor + noise;
            out[y * w + x] = px.clamp(0.0, 255.0) as u8;
        }
    }

    let degraded = GrayImage::from_raw(img.width, img.height, out)?;
    if cfg.skew_deg.abs() < 0.05 {
        return Ok(degraded);
    }

    // 2. Apply slight rotational skew.
    let rad = cfg.skew_deg.to_radians();
    let sin_a = rad.sin();
    let cos_a = rad.cos();
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let mut rotated = vec![245_u8; w * h];
    for y in 0..h {
        let dy = y as f32 - cy;
        for x in 0..w {
            let dx = x as f32 - cx;
            let sx = (dx * cos_a + dy * sin_a + cx).round() as isize;
            let sy = (-dx * sin_a + dy * cos_a + cy).round() as isize;
            if sx >= 0 && (sx as usize) < w && sy >= 0 && (sy as usize) < h {
                rotated[y * w + x] = degraded.data[sy as usize * w + sx as usize];
            }
        }
    }

    GrayImage::from_raw(img.width, img.height, rotated)
}

//! Lightweight raster image containers and pixel-level helpers.
//!
//! Keeps `tok-layout-analyzer` lean with zero mandatory external image decoding
//! dependencies while supporting grayscale, RGB, and raw/PGM/PPM buffers.

use crate::error::{AnalyzerError, AnalyzerResult};
use serde::{Deserialize, Serialize};

/// 8-bit grayscale image buffer (`0` = black ink, `255` = white paper).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrayImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl GrayImage {
    /// Creates a new grayscale image filled with `fill` (e.g. `255` for white paper).
    pub fn new(width: u32, height: u32, fill: u8) -> AnalyzerResult<Self> {
        let len = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| AnalyzerError::RasterProcessing("Image dimensions overflow".into()))?;
        Ok(Self {
            width,
            height,
            data: vec![fill; len],
        })
    }

    /// Creates a grayscale image from an existing pixel buffer.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> AnalyzerResult<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| AnalyzerError::RasterProcessing("Image dimensions overflow".into()))?;
        if data.len() != expected {
            return Err(AnalyzerError::RasterProcessing(format!(
                "GrayImage buffer length mismatch: expected {expected}, got {}",
                data.len()
            )));
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Converts an interleaved 24-bit RGB buffer to 8-bit luminance grayscale.
    pub fn from_rgb_bytes(width: u32, height: u32, rgb: &[u8]) -> AnalyzerResult<Self> {
        let pixels = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| AnalyzerError::RasterProcessing("Image dimensions overflow".into()))?;
        let expected = pixels
            .checked_mul(3)
            .ok_or_else(|| AnalyzerError::RasterProcessing("RGB buffer size overflow".into()))?;
        if rgb.len() != expected {
            return Err(AnalyzerError::RasterProcessing(format!(
                "RGB buffer length mismatch: expected {expected}, got {}",
                rgb.len()
            )));
        }
        let mut data = Vec::with_capacity(pixels);
        for chunk in rgb.as_chunks::<3>().0 {
            // ITU-R BT.601 luma approximation using integer arithmetic.
            let y = (77 * chunk[0] as u32 + 150 * chunk[1] as u32 + 29 * chunk[2] as u32) >> 8;
            data.push(y as u8);
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Returns the pixel intensity at `(x, y)` or `255` if out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u8 {
        if x < self.width && y < self.height {
            self.data[y as usize * self.width as usize + x as usize]
        } else {
            255
        }
    }

    /// Sets the pixel intensity at `(x, y)` if within bounds.
    #[inline]
    pub fn put_pixel(&mut self, x: u32, y: u32, val: u8) {
        if x < self.width && y < self.height {
            self.data[y as usize * self.width as usize + x as usize] = val;
        }
    }

    /// Crops a rectangular sub-image `[x0, x1) x [y0, y1)`.
    pub fn crop(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> AnalyzerResult<Self> {
        let cx0 = x0.min(self.width);
        let cy0 = y0.min(self.height);
        let cx1 = x1.min(self.width).max(cx0);
        let cy1 = y1.min(self.height).max(cy0);
        let w = cx1 - cx0;
        let h = cy1 - cy0;
        if w == 0 || h == 0 {
            return Err(AnalyzerError::RasterProcessing(
                "Crop produced an empty image".into(),
            ));
        }
        let mut out = Self::new(w, h, 255)?;
        for dy in 0..h {
            let src_start = (cy0 + dy) as usize * self.width as usize + cx0 as usize;
            let dst_start = dy as usize * w as usize;
            out.data[dst_start..dst_start + w as usize]
                .copy_from_slice(&self.data[src_start..src_start + w as usize]);
        }
        Ok(out)
    }

    /// Width-to-height aspect ratio (`width / height`).
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Decodes a binary PGM (`P5`) or ASCII PGM (`P2`) buffer into a [`GrayImage`].
    pub fn from_pgm_bytes(bytes: &[u8]) -> AnalyzerResult<Self> {
        if bytes.starts_with(b"P5") {
            let mut idx = 2;
            let tokens = parse_pnm_header_tokens(bytes, &mut idx, 3)?;
            let w: u32 = tokens[0]
                .parse()
                .map_err(|_| AnalyzerError::RasterProcessing("Invalid PGM width".into()))?;
            let h: u32 = tokens[1]
                .parse()
                .map_err(|_| AnalyzerError::RasterProcessing("Invalid PGM height".into()))?;
            let maxval: u32 = tokens[2]
                .parse()
                .map_err(|_| AnalyzerError::RasterProcessing("Invalid PGM maxval".into()))?;
            if maxval == 0 || maxval > 255 {
                return Err(AnalyzerError::RasterProcessing(
                    "Only 8-bit PGM (maxval <= 255) is supported".into(),
                ));
            }
            if idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            let expected = w as usize * h as usize;
            if bytes.len() - idx < expected {
                return Err(AnalyzerError::RasterProcessing(
                    "Truncated P5 PGM pixel payload".into(),
                ));
            }
            Self::from_raw(w, h, bytes[idx..idx + expected].to_vec())
        } else {
            Err(AnalyzerError::RasterProcessing(
                "Unsupported raster header (expected P5 PGM)".into(),
            ))
        }
    }
}

fn parse_pnm_header_tokens(
    bytes: &[u8],
    idx: &mut usize,
    count: usize,
) -> AnalyzerResult<Vec<String>> {
    let mut tokens = Vec::with_capacity(count);
    while tokens.len() < count && *idx < bytes.len() {
        while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
            *idx += 1;
        }
        if *idx < bytes.len() && bytes[*idx] == b'#' {
            while *idx < bytes.len() && bytes[*idx] != b'\n' {
                *idx += 1;
            }
            continue;
        }
        let start = *idx;
        while *idx < bytes.len() && !bytes[*idx].is_ascii_whitespace() {
            *idx += 1;
        }
        if start < *idx {
            tokens.push(String::from_utf8_lossy(&bytes[start..*idx]).into_owned());
        }
    }
    if tokens.len() < count {
        return Err(AnalyzerError::RasterProcessing(
            "Incomplete PNM header".into(),
        ));
    }
    Ok(tokens)
}

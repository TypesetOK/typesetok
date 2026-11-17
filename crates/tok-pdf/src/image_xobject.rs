//! Pure-Rust PNG & JPEG decoder and PDF/X image XObject encoder.
//!
//! Zero third-party image crates: parses PNG chunks (IHDR, PLTE, tRNS, IDAT) with
//! `miniz_oxide` decompression and all 5 PNG scanline filters, plus Baseline/Progressive
//! JPEG SOF dimensions/component metadata.
//!
//! - PDF/X-1a: Alpha channels are composited over white and converted to managed
//!   `/DeviceCMYK` via [`ColorManagementEngine::rgb_to_cmyk_managed`].
//! - PDF/X-4: Alpha channels are preserved as `/SMask` grayscale soft-mask XObjects.

use crate::color::{ColorManagementEngine, IccProfileId};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageColorModel {
    Gray8,
    Rgb8,
    Cmyk8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub color_model: ImageColorModel,
    pub alpha: Option<Vec<u8>>,
    /// Present for JPEG inputs that stream directly with `/DCTDecode`.
    pub raw_jpeg_bytes: Option<Vec<u8>>,
    /// Indicates Adobe-style inverted CMYK JPEG requiring `[1 0 1 0 1 0 1 0]` decode.
    pub jpeg_cmyk_inverted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedColorSpace {
    DeviceCmyk,
    DeviceGray,
    CalRgb,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedImageXObject {
    pub width: u32,
    pub height: u32,
    pub color_space: PreparedColorSpace,
    pub bits_per_component: i32,
    pub is_dct_jpeg: bool,
    pub cmyk_decode_inverted: bool,
    /// Compressed (FlateDecode) or raw JPEG (DCTDecode) image bytes.
    pub data: Vec<u8>,
    /// Optional compressed grayscale `/SMask` stream bytes (PDF/X-4 only).
    pub smask_flate_data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImageDecodeError {
    #[error("Unsupported or unrecognized image format (expected PNG or JPEG)")]
    UnsupportedFormat,
    #[error("Invalid PNG stream: {0}")]
    InvalidPng(String),
    #[error("Invalid JPEG stream: {0}")]
    InvalidJpeg(String),
    #[error("Unsupported PNG color type / bit depth: color_type={0}, bit_depth={1}")]
    UnsupportedColorType(u8, u8),
}

/// Computes effective print DPI for an axis given pixel count and placed dimension in points.
#[inline]
pub fn effective_dpi(intrinsic_px: u32, placed_pt: f32) -> f32 {
    if !placed_pt.is_finite() || placed_pt <= 0.0 {
        return 0.0;
    }
    (intrinsic_px as f32 * 72.0) / placed_pt
}

/// Decodes raw image bytes (PNG or JPEG) into pixel/header metadata.
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        decode_png(bytes)
    } else if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        decode_jpeg_headers(bytes)
    } else {
        Err(ImageDecodeError::UnsupportedFormat)
    }
}

/// Inspects width and height in pixels from raw PNG or JPEG bytes without full pixel decoding
/// where possible (falls back cleanly for PNG `IHDR` O(1) read).
pub fn inspect_image_dimensions(bytes: &[u8]) -> Result<(u32, u32), ImageDecodeError> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
            return Err(ImageDecodeError::InvalidPng("missing IHDR chunk".into()));
        }
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return Ok((w, h));
    }
    if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        let j = decode_jpeg_headers(bytes)?;
        return Ok((j.width, j.height));
    }
    Err(ImageDecodeError::UnsupportedFormat)
}

#[inline]
fn paeth_predictor(a: i32, b: i32, c: i32) -> u8 {
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

fn unfilter_scanlines(
    raw: &[u8],
    width: usize,
    height: usize,
    bpp: usize,
    row_bytes: usize,
) -> Result<Vec<u8>, ImageDecodeError> {
    let expected_len = height * (1 + row_bytes);
    if raw.len() < expected_len {
        return Err(ImageDecodeError::InvalidPng(format!(
            "decompressed IDAT too short: got {}, expected {}",
            raw.len(),
            expected_len
        )));
    }
    let mut recon = vec![0u8; height * row_bytes];
    for y in 0..height {
        let src_off = y * (1 + row_bytes);
        let filter_type = raw[src_off];
        let src_row = &raw[src_off + 1..src_off + 1 + row_bytes];
        let dst_off = y * row_bytes;
        let prev_off = y.saturating_sub(1) * row_bytes;

        for x in 0..row_bytes {
            let filt = src_row[x];
            let a = if x >= bpp {
                recon[dst_off + x - bpp]
            } else {
                0
            };
            let b = if y > 0 { recon[prev_off + x] } else { 0 };
            let c = if y > 0 && x >= bpp {
                recon[prev_off + x - bpp]
            } else {
                0
            };
            let val = match filter_type {
                0 => filt,
                1 => filt.wrapping_add(a),
                2 => filt.wrapping_add(b),
                3 => {
                    let avg = ((a as u16 + b as u16) / 2) as u8;
                    filt.wrapping_add(avg)
                }
                4 => filt.wrapping_add(paeth_predictor(a as i32, b as i32, c as i32)),
                other => {
                    return Err(ImageDecodeError::InvalidPng(format!(
                        "unknown PNG filter type {}",
                        other
                    )));
                }
            };
            recon[dst_off + x] = val;
        }
    }
    let _ = width;
    Ok(recon)
}

/// Decodes 8-bit non-interlaced PNG bytes (`0` Gray, `2` RGB, `3` Palette, `4` GrayA, `6` RGBA).
pub fn decode_png(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
    if bytes.len() < 33 || !bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Err(ImageDecodeError::InvalidPng(
            "bad signature or truncated".into(),
        ));
    }

    let mut pos = 8usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut ihdr_seen = false;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut trns_alpha: Vec<u8> = Vec::new();
    let mut idat = Vec::new();

    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        let ctype = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start.saturating_add(len);
        if data_end + 4 > bytes.len() {
            return Err(ImageDecodeError::InvalidPng("truncated chunk".into()));
        }
        let chunk_data = &bytes[data_start..data_end];
        match ctype {
            b"IHDR" => {
                if len < 13 {
                    return Err(ImageDecodeError::InvalidPng("IHDR < 13 bytes".into()));
                }
                width = u32::from_be_bytes([
                    chunk_data[0],
                    chunk_data[1],
                    chunk_data[2],
                    chunk_data[3],
                ]);
                height = u32::from_be_bytes([
                    chunk_data[4],
                    chunk_data[5],
                    chunk_data[6],
                    chunk_data[7],
                ]);
                bit_depth = chunk_data[8];
                color_type = chunk_data[9];
                let interlace = chunk_data[12];
                if interlace != 0 {
                    return Err(ImageDecodeError::InvalidPng(
                        "interlaced PNG not supported".into(),
                    ));
                }
                ihdr_seen = true;
            }
            b"PLTE" => {
                for rgb in chunk_data.as_chunks::<3>().0 {
                    palette.push([rgb[0], rgb[1], rgb[2]]);
                }
            }
            b"tRNS" => {
                trns_alpha.extend_from_slice(chunk_data);
            }
            b"IDAT" => {
                idat.extend_from_slice(chunk_data);
            }
            b"IEND" => break,
            _ => {}
        }
        pos = data_end + 4; // skip CRC32
    }

    if !ihdr_seen || width == 0 || height == 0 {
        return Err(ImageDecodeError::InvalidPng(
            "missing or zero-sized IHDR".into(),
        ));
    }
    if bit_depth != 8 {
        return Err(ImageDecodeError::UnsupportedColorType(
            color_type, bit_depth,
        ));
    }

    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat)
        .map_err(|e| ImageDecodeError::InvalidPng(format!("zlib inflate error: {:?}", e)))?;

    let w = width as usize;
    let h = height as usize;
    let channels: usize = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            return Err(ImageDecodeError::UnsupportedColorType(
                color_type, bit_depth,
            ))
        }
    };
    let row_bytes = w * channels;
    let recon = unfilter_scanlines(&raw, w, h, channels, row_bytes)?;

    match color_type {
        0 => Ok(DecodedImage {
            width,
            height,
            pixels: recon,
            color_model: ImageColorModel::Gray8,
            alpha: None,
            raw_jpeg_bytes: None,
            jpeg_cmyk_inverted: false,
        }),
        4 => {
            let mut gray = Vec::with_capacity(w * h);
            let mut alpha = Vec::with_capacity(w * h);
            for px in recon.as_chunks::<2>().0 {
                gray.push(px[0]);
                alpha.push(px[1]);
            }
            Ok(DecodedImage {
                width,
                height,
                pixels: gray,
                color_model: ImageColorModel::Gray8,
                alpha: Some(alpha),
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
            })
        }
        2 => Ok(DecodedImage {
            width,
            height,
            pixels: recon,
            color_model: ImageColorModel::Rgb8,
            alpha: None,
            raw_jpeg_bytes: None,
            jpeg_cmyk_inverted: false,
        }),
        6 => {
            let mut rgb = Vec::with_capacity(w * h * 3);
            let mut alpha = Vec::with_capacity(w * h);
            for px in recon.as_chunks::<4>().0 {
                rgb.push(px[0]);
                rgb.push(px[1]);
                rgb.push(px[2]);
                alpha.push(px[3]);
            }
            Ok(DecodedImage {
                width,
                height,
                pixels: rgb,
                color_model: ImageColorModel::Rgb8,
                alpha: Some(alpha),
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
            })
        }
        3 => {
            let mut rgb = Vec::with_capacity(w * h * 3);
            let mut alpha = if trns_alpha.is_empty() {
                None
            } else {
                Some(Vec::with_capacity(w * h))
            };
            for &idx in &recon {
                let entry = palette.get(idx as usize).copied().unwrap_or([0, 0, 0]);
                rgb.extend_from_slice(&entry);
                if let Some(ref mut a) = alpha {
                    a.push(trns_alpha.get(idx as usize).copied().unwrap_or(255));
                }
            }
            Ok(DecodedImage {
                width,
                height,
                pixels: rgb,
                color_model: ImageColorModel::Rgb8,
                alpha,
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
            })
        }
        _ => Err(ImageDecodeError::UnsupportedColorType(
            color_type, bit_depth,
        )),
    }
}

/// Parses SOF0 / SOF2 JPEG markers and Adobe APP14 CMYK inversion flag without external crates.
pub fn decode_jpeg_headers(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(ImageDecodeError::InvalidJpeg("missing SOI marker".into()));
    }
    let mut pos = 2usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut components = 0u8;
    let mut adobe_transform: Option<u8> = None;

    while pos + 4 <= bytes.len() {
        while pos < bytes.len() && bytes[pos] != 0xFF {
            pos += 1;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;

        // Standalone markers (RST, TEM, SOI, EOI)
        if marker == 0xD9 || marker == 0x00 || (0xD0..=0xD8).contains(&marker) || marker == 0x01 {
            if marker == 0xD9 {
                break;
            }
            continue;
        }
        if pos + 2 > bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return Err(ImageDecodeError::InvalidJpeg(
                "invalid marker length".into(),
            ));
        }
        let seg = &bytes[pos + 2..pos + seg_len];

        // APP14 Adobe marker checks color transform for 4-component CMYK JPEGs
        if marker == 0xEE && seg.len() >= 12 && seg.starts_with(b"Adobe") {
            adobe_transform = Some(seg[11]);
        }

        // SOF0..SOF3, SOF5..SOF7, SOF9..SOF11, SOF13..SOF15 carry image geometry
        if matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        ) && seg.len() >= 6
        {
            height = u16::from_be_bytes([seg[1], seg[2]]) as u32;
            width = u16::from_be_bytes([seg[3], seg[4]]) as u32;
            components = seg[5];
            break;
        }

        pos += seg_len;
    }

    if width == 0 || height == 0 {
        return Err(ImageDecodeError::InvalidJpeg(
            "missing SOF frame header".into(),
        ));
    }
    let color_model = match components {
        1 => ImageColorModel::Gray8,
        3 => ImageColorModel::Rgb8,
        4 => ImageColorModel::Cmyk8,
        other => {
            return Err(ImageDecodeError::InvalidJpeg(format!(
                "unsupported component count {}",
                other
            )));
        }
    };

    Ok(DecodedImage {
        width,
        height,
        pixels: Vec::new(),
        color_model,
        alpha: None,
        raw_jpeg_bytes: Some(bytes.to_vec()),
        jpeg_cmyk_inverted: color_model == ImageColorModel::Cmyk8 && adobe_transform.is_some(),
    })
}

/// Converts a decoded image into compressed PDF XObject payloads conforming to `is_pdf_x1a`.
///
/// - In PDF/X-1a: flattens any alpha onto white (`255`) and converts RGB -> managed `/DeviceCMYK`
///   via [`ColorManagementEngine::rgb_to_cmyk_managed`].
/// - In PDF/X-4: converts RGB PNGs -> managed `/DeviceCMYK` (keeping `/SMask` separate when alpha exists)
///   or streams RGB JPEGs with `CalRGB` (`srgb()`) / CMYK JPEGs with `DeviceCMYK`.
pub fn encode_image_xobject_data(
    bytes: &[u8],
    is_pdf_x1a: bool,
    profile: IccProfileId,
) -> Result<PreparedImageXObject, ImageDecodeError> {
    let decoded = decode_image(bytes)?;
    if let Some(jpeg) = decoded.raw_jpeg_bytes {
        let color_space = match decoded.color_model {
            ImageColorModel::Gray8 => PreparedColorSpace::DeviceGray,
            ImageColorModel::Cmyk8 => PreparedColorSpace::DeviceCmyk,
            ImageColorModel::Rgb8 => PreparedColorSpace::CalRgb,
        };
        return Ok(PreparedImageXObject {
            width: decoded.width,
            height: decoded.height,
            color_space,
            bits_per_component: 8,
            is_dct_jpeg: true,
            cmyk_decode_inverted: decoded.jpeg_cmyk_inverted,
            data: jpeg,
            smask_flate_data: None,
        });
    }

    let pixel_count = (decoded.width as usize) * (decoded.height as usize);
    let smask_flate_data = if !is_pdf_x1a {
        decoded
            .alpha
            .as_ref()
            .map(|a| miniz_oxide::deflate::compress_to_vec_zlib(a, 6))
    } else {
        None
    };

    match decoded.color_model {
        ImageColorModel::Gray8 => {
            let mut out = decoded.pixels;
            if is_pdf_x1a {
                if let Some(ref a) = decoded.alpha {
                    for (g, &alpha) in out.iter_mut().zip(a.iter()) {
                        let af = alpha as f32 / 255.0;
                        *g = ((*g as f32 * af) + 255.0 * (1.0 - af))
                            .round()
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
            Ok(PreparedImageXObject {
                width: decoded.width,
                height: decoded.height,
                color_space: PreparedColorSpace::DeviceGray,
                bits_per_component: 8,
                is_dct_jpeg: false,
                cmyk_decode_inverted: false,
                data: miniz_oxide::deflate::compress_to_vec_zlib(&out, 6),
                smask_flate_data,
            })
        }
        ImageColorModel::Rgb8 => {
            let mut cmyk = Vec::with_capacity(pixel_count * 4);
            for idx in 0..pixel_count {
                let mut r = decoded.pixels[idx * 3] as f32 / 255.0;
                let mut g = decoded.pixels[idx * 3 + 1] as f32 / 255.0;
                let mut b = decoded.pixels[idx * 3 + 2] as f32 / 255.0;
                if is_pdf_x1a {
                    if let Some(ref alpha) = decoded.alpha {
                        let a = alpha[idx] as f32 / 255.0;
                        r = r * a + (1.0 - a);
                        g = g * a + (1.0 - a);
                        b = b * a + (1.0 - a);
                    }
                }
                let conv = ColorManagementEngine::rgb_to_cmyk_managed(r, g, b, profile);
                cmyk.push((conv.c.clamp(0.0, 1.0) * 255.0).round() as u8);
                cmyk.push((conv.m.clamp(0.0, 1.0) * 255.0).round() as u8);
                cmyk.push((conv.y.clamp(0.0, 1.0) * 255.0).round() as u8);
                cmyk.push((conv.k.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            Ok(PreparedImageXObject {
                width: decoded.width,
                height: decoded.height,
                color_space: PreparedColorSpace::DeviceCmyk,
                bits_per_component: 8,
                is_dct_jpeg: false,
                cmyk_decode_inverted: false,
                data: miniz_oxide::deflate::compress_to_vec_zlib(&cmyk, 6),
                smask_flate_data,
            })
        }
        ImageColorModel::Cmyk8 => Ok(PreparedImageXObject {
            width: decoded.width,
            height: decoded.height,
            color_space: PreparedColorSpace::DeviceCmyk,
            bits_per_component: 8,
            is_dct_jpeg: false,
            cmyk_decode_inverted: false,
            data: miniz_oxide::deflate::compress_to_vec_zlib(&decoded.pixels, 6),
            smask_flate_data,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFFFFFFu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB88320 & mask);
            }
        }
        !crc
    }

    fn make_png_rgba(w: u32, h: u32, pixels: &[u8]) -> Vec<u8> {
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let push_chunk = |dst: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]| {
            dst.extend_from_slice(&(data.len() as u32).to_be_bytes());
            dst.extend_from_slice(tag);
            dst.extend_from_slice(data);
            let mut crc_buf = Vec::with_capacity(4 + data.len());
            crc_buf.extend_from_slice(tag);
            crc_buf.extend_from_slice(data);
            dst.extend_from_slice(&crc32(&crc_buf).to_be_bytes());
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        push_chunk(&mut out, b"IHDR", &ihdr);

        let mut raw = Vec::new();
        for y in 0..h as usize {
            raw.push(0); // filter None
            let off = y * w as usize * 4;
            raw.extend_from_slice(&pixels[off..off + w as usize * 4]);
        }
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        push_chunk(&mut out, b"IDAT", &z);
        push_chunk(&mut out, b"IEND", &[]);
        out
    }

    #[test]
    fn png_rgba_flattens_over_white_in_x1a_and_keeps_smask_in_x4() {
        // 2x1 PNG: transparent pixel (255,0,0,0) + opaque pure black (0,0,0,255)
        let png = make_png_rgba(2, 1, &[255, 0, 0, 0, 0, 0, 0, 255]);
        let decoded = decode_png(&png).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.alpha.as_ref().unwrap(), &[0, 255]);

        // PDF/X-1a flattens alpha 0 red over white -> C=0,M=0,Y=0,K=0 and no SMask
        let prep_x1a = encode_image_xobject_data(&png, true, IccProfileId::Fogra39).unwrap();
        assert!(prep_x1a.smask_flate_data.is_none());
        assert_eq!(prep_x1a.color_space, PreparedColorSpace::DeviceCmyk);
        let cmyk = miniz_oxide::inflate::decompress_to_vec_zlib(&prep_x1a.data).unwrap();
        assert_eq!(&cmyk[0..4], &[0, 0, 0, 0]);
        assert!(
            cmyk[7] >= 240,
            "Black pixel must map to strong K: {:?}",
            &cmyk[4..8]
        );

        // PDF/X-4 emits separate /SMask
        let prep_x4 = encode_image_xobject_data(&png, false, IccProfileId::Fogra39).unwrap();
        assert!(prep_x4.smask_flate_data.is_some());
    }

    #[test]
    fn jpeg_sof0_dimensions_and_effective_dpi() {
        // Minimal valid JPEG SOI + SOF0 (8-bit, 600x300, 1 component Gray) + EOI
        let jpeg = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, 0x00, 0x0B, // SOF0 len=11
            0x08, // precision=8
            0x01, 0x2C, // height=300
            0x02, 0x58, // width=600
            0x01, // 1 component
            0x01, 0x11, 0x00, // component descriptor
            0xFF, 0xD9, // EOI
        ];
        let (w, h) = inspect_image_dimensions(&jpeg).unwrap();
        assert_eq!((w, h), (600, 300));
        // Placed in 144pt (2 inch) width => 600 * 72 / 144 = 300 DPI
        assert!((effective_dpi(w, 144.0) - 300.0).abs() < 1e-2);
    }
}

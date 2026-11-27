//! Pure-Rust TIFF 6.0 decoder supporting RGB, CMYK, Grayscale, Palette, LZW, ZIP/Deflate,
//! PackBits, horizontal differencing predictors, multi-page IFD chains, tiles/strips,
//! planar/chunky layouts, DPI resolution metadata, and embedded ICC profiles.
//!
//! Zero `unsafe` code and zero external image crates (`miniz_oxide` is reused for Deflate/ZIP).

use std::collections::BTreeSet;

use crate::image_xobject::{DecodedImage, ImageColorModel, ImageDecodeError};

/// TIFF byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TiffEndianness {
    Little,
    Big,
}

/// Summary metadata for a single page (IFD) inside a TIFF stream.
#[derive(Debug, Clone, PartialEq)]
pub struct TiffPageInfo {
    pub page_index: usize,
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: Vec<u16>,
    pub samples_per_pixel: u16,
    pub photometric: u16,
    pub compression: u16,
    pub predictor: u16,
    pub planar_configuration: u16,
    pub dpi_x: Option<f32>,
    pub dpi_y: Option<f32>,
    pub has_icc_profile: bool,
    pub is_tiled: bool,
}

/// Summary metadata for an entire TIFF stream (including multi-page IFD count).
#[derive(Debug, Clone, PartialEq)]
pub struct TiffInfo {
    pub endianness: TiffEndianness,
    pub page_count: usize,
    pub pages: Vec<TiffPageInfo>,
}

/// Returns `true` if `bytes` starts with a standard TIFF `II*\0` or `MM\0*` header.
#[inline]
pub fn is_tiff_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 8
        && ((bytes[0] == b'I' && bytes[1] == b'I' && bytes[2] == 42 && bytes[3] == 0)
            || (bytes[0] == b'M' && bytes[1] == b'M' && bytes[2] == 0 && bytes[3] == 42))
}

#[inline]
fn read_u16(bytes: &[u8], off: usize, end: TiffEndianness) -> Result<u16, ImageDecodeError> {
    let slice = bytes
        .get(off..off + 2)
        .ok_or_else(|| ImageDecodeError::InvalidTiff("unexpected EOF reading u16".into()))?;
    Ok(match end {
        TiffEndianness::Little => u16::from_le_bytes([slice[0], slice[1]]),
        TiffEndianness::Big => u16::from_be_bytes([slice[0], slice[1]]),
    })
}

#[inline]
fn read_u32(bytes: &[u8], off: usize, end: TiffEndianness) -> Result<u32, ImageDecodeError> {
    let slice = bytes
        .get(off..off + 4)
        .ok_or_else(|| ImageDecodeError::InvalidTiff("unexpected EOF reading u32".into()))?;
    Ok(match end {
        TiffEndianness::Little => u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]),
        TiffEndianness::Big => u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]),
    })
}

fn parse_header(bytes: &[u8]) -> Result<(TiffEndianness, u32), ImageDecodeError> {
    if bytes.len() < 8 {
        return Err(ImageDecodeError::InvalidTiff("TIFF header < 8 bytes".into()));
    }
    let end = match (bytes[0], bytes[1]) {
        (b'I', b'I') => TiffEndianness::Little,
        (b'M', b'M') => TiffEndianness::Big,
        _ => return Err(ImageDecodeError::InvalidTiff("invalid TIFF byte order".into())),
    };
    let magic = read_u16(bytes, 2, end)?;
    if magic == 43 {
        return Err(ImageDecodeError::InvalidTiff(
            "BigTIFF (magic 43) is not supported".into(),
        ));
    }
    if magic != 42 {
        return Err(ImageDecodeError::InvalidTiff(format!(
            "invalid TIFF magic number {}",
            magic
        )));
    }
    let first_ifd = read_u32(bytes, 4, end)?;
    Ok((end, first_ifd))
}

#[derive(Debug, Clone, Copy)]
struct IfdEntry {
    tag: u16,
    field_type: u16,
    count: u32,
    value_or_offset: [u8; 4],
}

impl IfdEntry {
    fn type_size(field_type: u16) -> Option<usize> {
        match field_type {
            1 | 2 | 6 | 7 => Some(1), // BYTE, ASCII, SBYTE, UNDEFINED
            3 | 8 => Some(2),         // SHORT, SSHORT
            4 | 9 | 11 => Some(4),    // LONG, SLONG, FLOAT
            5 | 10 | 12 => Some(8),   // RATIONAL, SRATIONAL, DOUBLE
            _ => None,
        }
    }

    fn data_slice<'a>(
        &self,
        bytes: &'a [u8],
        end: TiffEndianness,
    ) -> Result<&'a [u8], ImageDecodeError> {
        let elem_size = Self::type_size(self.field_type).ok_or_else(|| {
            ImageDecodeError::InvalidTiff(format!("unsupported IFD field type {}", self.field_type))
        })?;
        let total = (self.count as usize)
            .checked_mul(elem_size)
            .ok_or_else(|| ImageDecodeError::InvalidTiff("IFD tag byte size overflow".into()))?;
        if total <= 4 {
            // Return slice from original bytes? We don't have a reference to self.value_or_offset
            // with 'a lifetime, so callers use data_vec or inline helpers.
            // This method is only for >4 byte out-of-line data; use `read_bytes` below instead.
            Err(ImageDecodeError::InvalidTiff(
                "internal: inline data_slice".into(),
            ))
        } else {
            let offset = match end {
                TiffEndianness::Little => u32::from_le_bytes(self.value_or_offset),
                TiffEndianness::Big => u32::from_be_bytes(self.value_or_offset),
            } as usize;
            let end_off = offset
                .checked_add(total)
                .ok_or_else(|| ImageDecodeError::InvalidTiff("IFD tag offset overflow".into()))?;
            bytes.get(offset..end_off).ok_or_else(|| {
                ImageDecodeError::InvalidTiff(format!(
                    "IFD tag {} out of bounds (offset {}, len {})",
                    self.tag, offset, total
                ))
            })
        }
    }

    fn read_bytes(&self, bytes: &[u8], end: TiffEndianness) -> Result<Vec<u8>, ImageDecodeError> {
        let elem_size = Self::type_size(self.field_type).unwrap_or(1);
        let total = (self.count as usize)
            .checked_mul(elem_size)
            .ok_or_else(|| ImageDecodeError::InvalidTiff("IFD tag byte size overflow".into()))?;
        if total <= 4 {
            Ok(self.value_or_offset[..total].to_vec())
        } else {
            Ok(self.data_slice(bytes, end)?.to_vec())
        }
    }

    fn read_u32_vec(
        &self,
        bytes: &[u8],
        end: TiffEndianness,
    ) -> Result<Vec<u32>, ImageDecodeError> {
        let raw = self.read_bytes(bytes, end)?;
        let count = self.count as usize;
        let mut out = Vec::with_capacity(count);
        match self.field_type {
            1 | 7 => {
                for i in 0..count {
                    out.push(*raw.get(i).ok_or_else(|| {
                        ImageDecodeError::InvalidTiff("truncated BYTE tag".into())
                    })? as u32);
                }
            }
            3 => {
                for i in 0..count {
                    out.push(read_u16(&raw, i * 2, end)? as u32);
                }
            }
            4 => {
                for i in 0..count {
                    out.push(read_u32(&raw, i * 4, end)?);
                }
            }
            other => {
                return Err(ImageDecodeError::InvalidTiff(format!(
                    "expected integer tag for {}, got field_type {}",
                    self.tag, other
                )));
            }
        }
        Ok(out)
    }

    fn read_u16_vec(
        &self,
        bytes: &[u8],
        end: TiffEndianness,
    ) -> Result<Vec<u16>, ImageDecodeError> {
        let raw = self.read_bytes(bytes, end)?;
        let count = self.count as usize;
        let mut out = Vec::with_capacity(count);
        match self.field_type {
            3 => {
                for i in 0..count {
                    out.push(read_u16(&raw, i * 2, end)?);
                }
            }
            4 => {
                for i in 0..count {
                    let v = read_u32(&raw, i * 4, end)?;
                    out.push(u16::try_from(v).map_err(|_| {
                        ImageDecodeError::InvalidTiff(format!(
                            "tag {} value {} overflows u16",
                            self.tag, v
                        ))
                    })?);
                }
            }
            1 => {
                for i in 0..count {
                    out.push(*raw.get(i).ok_or_else(|| {
                        ImageDecodeError::InvalidTiff("truncated BYTE tag".into())
                    })? as u16);
                }
            }
            other => {
                return Err(ImageDecodeError::InvalidTiff(format!(
                    "expected SHORT tag for {}, got field_type {}",
                    self.tag, other
                )));
            }
        }
        Ok(out)
    }

    fn read_rational_f32(
        &self,
        bytes: &[u8],
        end: TiffEndianness,
    ) -> Result<Option<f32>, ImageDecodeError> {
        if self.count == 0 {
            return Ok(None);
        }
        let raw = self.read_bytes(bytes, end)?;
        if raw.len() < 8 {
            return Err(ImageDecodeError::InvalidTiff(
                "RATIONAL tag < 8 bytes".into(),
            ));
        }
        let num = read_u32(&raw, 0, end)?;
        let den = read_u32(&raw, 4, end)?;
        if den == 0 {
            Ok(None)
        } else {
            Ok(Some(num as f32 / den as f32))
        }
    }
}

struct ParsedIfd {
    width: u32,
    height: u32,
    bits_per_sample: Vec<u16>,
    compression: u16,
    photometric: u16,
    strip_offsets: Vec<u32>,
    samples_per_pixel: u16,
    rows_per_strip: u32,
    strip_byte_counts: Vec<u32>,
    x_resolution: Option<f32>,
    y_resolution: Option<f32>,
    resolution_unit: u16,
    planar_configuration: u16,
    predictor: u16,
    color_map: Vec<u16>,
    tile_width: Option<u32>,
    tile_length: Option<u32>,
    tile_offsets: Vec<u32>,
    tile_byte_counts: Vec<u32>,
    extra_samples: Vec<u16>,
    icc_profile: Option<Vec<u8>>,
}

impl ParsedIfd {
    fn dpi(&self) -> (Option<f32>, Option<f32>) {
        let factor = match self.resolution_unit {
            2 => 1.0_f32,  // Inch
            3 => 2.54_f32, // Centimeter -> DPI
            _ => return (None, None),
        };
        (
            self.x_resolution.filter(|v| *v > 0.0).map(|v| v * factor),
            self.y_resolution.filter(|v| *v > 0.0).map(|v| v * factor),
        )
    }

    fn to_page_info(&self, page_index: usize) -> TiffPageInfo {
        let (dpi_x, dpi_y) = self.dpi();
        TiffPageInfo {
            page_index,
            width: self.width,
            height: self.height,
            bits_per_sample: self.bits_per_sample.clone(),
            samples_per_pixel: self.samples_per_pixel,
            photometric: self.photometric,
            compression: self.compression,
            predictor: self.predictor,
            planar_configuration: self.planar_configuration,
            dpi_x,
            dpi_y,
            has_icc_profile: self.icc_profile.is_some(),
            is_tiled: self.tile_width.is_some() && !self.tile_offsets.is_empty(),
        }
    }
}

fn parse_ifd_at(
    bytes: &[u8],
    offset: u32,
    end: TiffEndianness,
) -> Result<(ParsedIfd, u32), ImageDecodeError> {
    let base = offset as usize;
    let entry_count = read_u16(bytes, base, end)? as usize;
    let entries_start = base + 2;
    let entries_end = entries_start + entry_count * 12;
    if entries_end + 4 > bytes.len() {
        return Err(ImageDecodeError::InvalidTiff(
            "IFD entries extend past EOF".into(),
        ));
    }

    let mut width = 0u32;
    let mut height = 0u32;
    let mut bits_per_sample: Option<Vec<u16>> = None;
    let mut compression = 1u16;
    let mut photometric: Option<u16> = None;
    let mut strip_offsets = Vec::new();
    let mut samples_per_pixel = 1u16;
    let mut rows_per_strip: Option<u32> = None;
    let mut strip_byte_counts = Vec::new();
    let mut x_resolution = None;
    let mut y_resolution = None;
    let mut resolution_unit = 2u16;
    let mut planar_configuration = 1u16;
    let mut predictor = 1u16;
    let mut color_map = Vec::new();
    let mut tile_width = None;
    let mut tile_length = None;
    let mut tile_offsets = Vec::new();
    let mut tile_byte_counts = Vec::new();
    let mut extra_samples = Vec::new();
    let mut icc_profile = None;

    for i in 0..entry_count {
        let e_off = entries_start + i * 12;
        let entry = IfdEntry {
            tag: read_u16(bytes, e_off, end)?,
            field_type: read_u16(bytes, e_off + 2, end)?,
            count: read_u32(bytes, e_off + 4, end)?,
            value_or_offset: [
                bytes[e_off + 8],
                bytes[e_off + 9],
                bytes[e_off + 10],
                bytes[e_off + 11],
            ],
        };

        match entry.tag {
            256 => {
                width = *entry
                    .read_u32_vec(bytes, end)?
                    .first()
                    .ok_or_else(|| ImageDecodeError::InvalidTiff("empty ImageWidth".into()))?;
            }
            257 => {
                height = *entry
                    .read_u32_vec(bytes, end)?
                    .first()
                    .ok_or_else(|| ImageDecodeError::InvalidTiff("empty ImageLength".into()))?;
            }
            258 => {
                bits_per_sample = Some(entry.read_u16_vec(bytes, end)?);
            }
            259 => {
                compression = *entry
                    .read_u16_vec(bytes, end)?
                    .first()
                    .unwrap_or(&1);
            }
            262 => {
                photometric = entry.read_u16_vec(bytes, end)?.into_iter().next();
            }
            273 => {
                strip_offsets = entry.read_u32_vec(bytes, end)?;
            }
            277 => {
                samples_per_pixel = *entry
                    .read_u16_vec(bytes, end)?
                    .first()
                    .unwrap_or(&1);
            }
            278 => {
                rows_per_strip = entry.read_u32_vec(bytes, end)?.into_iter().next();
            }
            279 => {
                strip_byte_counts = entry.read_u32_vec(bytes, end)?;
            }
            282 => {
                x_resolution = entry.read_rational_f32(bytes, end)?;
            }
            283 => {
                y_resolution = entry.read_rational_f32(bytes, end)?;
            }
            284 => {
                planar_configuration = *entry
                    .read_u16_vec(bytes, end)?
                    .first()
                    .unwrap_or(&1);
            }
            296 => {
                resolution_unit = *entry
                    .read_u16_vec(bytes, end)?
                    .first()
                    .unwrap_or(&2);
            }
            317 => {
                predictor = *entry
                    .read_u16_vec(bytes, end)?
                    .first()
                    .unwrap_or(&1);
            }
            320 => {
                color_map = entry.read_u16_vec(bytes, end)?;
            }
            322 => {
                tile_width = entry.read_u32_vec(bytes, end)?.into_iter().next();
            }
            323 => {
                tile_length = entry.read_u32_vec(bytes, end)?.into_iter().next();
            }
            324 => {
                tile_offsets = entry.read_u32_vec(bytes, end)?;
            }
            325 => {
                tile_byte_counts = entry.read_u32_vec(bytes, end)?;
            }
            338 => {
                extra_samples = entry.read_u16_vec(bytes, end)?;
            }
            34675 => {
                let raw_icc = entry.read_bytes(bytes, end)?;
                if !raw_icc.is_empty() {
                    icc_profile = Some(raw_icc);
                }
            }
            _ => {}
        }
    }

    if width == 0 || height == 0 {
        return Err(ImageDecodeError::InvalidTiff(
            "TIFF IFD missing non-zero ImageWidth/ImageLength".into(),
        ));
    }
    let photometric = photometric.ok_or_else(|| {
        ImageDecodeError::InvalidTiff("TIFF IFD missing PhotometricInterpretation".into())
    })?;
    let bits_per_sample = bits_per_sample.unwrap_or_else(|| vec![1; samples_per_pixel as usize]);
    let rows_per_strip = rows_per_strip.unwrap_or(height).max(1);
    let next_ifd = read_u32(bytes, entries_end, end)?;

    Ok((
        ParsedIfd {
            width,
            height,
            bits_per_sample,
            compression,
            photometric,
            strip_offsets,
            samples_per_pixel,
            rows_per_strip,
            strip_byte_counts,
            x_resolution,
            y_resolution,
            resolution_unit,
            planar_configuration,
            predictor,
            color_map,
            tile_width,
            tile_length,
            tile_offsets,
            tile_byte_counts,
            extra_samples,
            icc_profile,
        },
        next_ifd,
    ))
}

fn collect_ifds(bytes: &[u8]) -> Result<(TiffEndianness, Vec<ParsedIfd>), ImageDecodeError> {
    let (end, mut next_ifd) = parse_header(bytes)?;
    let mut visited = BTreeSet::new();
    let mut ifds = Vec::new();
    while next_ifd != 0 {
        if !visited.insert(next_ifd) {
            return Err(ImageDecodeError::InvalidTiff(
                "cyclic IFD offset detected".into(),
            ));
        }
        let (ifd, next) = parse_ifd_at(bytes, next_ifd, end)?;
        ifds.push(ifd);
        next_ifd = next;
    }
    if ifds.is_empty() {
        return Err(ImageDecodeError::InvalidTiff(
            "TIFF file contains no IFD pages".into(),
        ));
    }
    Ok((end, ifds))
}

/// Inspects a TIFF stream and returns page count and per-page metadata without decoding pixels.
pub fn inspect_tiff_info(bytes: &[u8]) -> Result<TiffInfo, ImageDecodeError> {
    let (endianness, ifds) = collect_ifds(bytes)?;
    let pages: Vec<TiffPageInfo> = ifds
        .iter()
        .enumerate()
        .map(|(idx, ifd)| ifd.to_page_info(idx))
        .collect();
    Ok(TiffInfo {
        endianness,
        page_count: pages.len(),
        pages,
    })
}

/// Decodes page 0 of a TIFF image.
pub fn decode_tiff(bytes: &[u8]) -> Result<DecodedImage, ImageDecodeError> {
    decode_tiff_page(bytes, 0)
}

/// Decodes a specific 0-based `page_index` from a single-page or multi-page TIFF image.
pub fn decode_tiff_page(bytes: &[u8], page_index: usize) -> Result<DecodedImage, ImageDecodeError> {
    let (end, ifds) = collect_ifds(bytes)?;
    let total = ifds.len();
    let ifd = ifds
        .into_iter()
        .nth(page_index)
        .ok_or(ImageDecodeError::TiffPageOutOfRange {
            page: page_index,
            total,
        })?;
    decode_ifd(bytes, end, &ifd, total)
}

/// TIFF 6.0 MSB-first variable-width (9..12-bit) LZW decompressor with early-change bit growth.
fn decompress_lzw(src: &[u8], expected_len: usize) -> Result<Vec<u8>, ImageDecodeError> {
    const CLEAR: u16 = 256;
    const EOI: u16 = 257;

    let mut out = Vec::with_capacity(expected_len);
    let mut prefix = [u16::MAX; 4096];
    let mut suffix = [0u8; 4096];
    let mut first_char = [0u8; 4096];
    for i in 0..256usize {
        suffix[i] = i as u8;
        first_char[i] = i as u8;
    }

    let mut next_code: u16 = 258;
    let mut code_bits: u8 = 9;
    let mut prev_code: Option<u16> = None;

    let mut bit_buf: u32 = 0;
    let mut bit_count: u8 = 0;
    let mut byte_pos: usize = 0;
    let mut stack = Vec::with_capacity(256);

    let read_code = |bit_buf: &mut u32,
                     bit_count: &mut u8,
                     byte_pos: &mut usize,
                     bits: u8|
     -> Option<u16> {
        while *bit_count < bits {
            if *byte_pos >= src.len() {
                return None;
            }
            *bit_buf = (*bit_buf << 8) | (src[*byte_pos] as u32);
            *byte_pos += 1;
            *bit_count += 8;
        }
        let shift = *bit_count - bits;
        let mask = (1u32 << bits) - 1;
        let code = ((*bit_buf >> shift) & mask) as u16;
        *bit_count -= bits;
        *bit_buf &= (1u32 << *bit_count).wrapping_sub(1);
        Some(code)
    };

    while let Some(code) = read_code(&mut bit_buf, &mut bit_count, &mut byte_pos, code_bits) {
        if code == EOI {
            break;
        }
        if code == CLEAR {
            next_code = 258;
            code_bits = 9;
            prev_code = None;
            continue;
        }

        if code < next_code {
            stack.clear();
            let mut cur = code;
            let mut steps = 0usize;
            while cur != u16::MAX {
                if steps >= 4096 {
                    return Err(ImageDecodeError::InvalidTiff(
                        "LZW dictionary cycle".into(),
                    ));
                }
                stack.push(suffix[cur as usize]);
                cur = prefix[cur as usize];
                steps += 1;
            }
            for &b in stack.iter().rev() {
                out.push(b);
            }
            if let Some(prev) = prev_code {
                if (next_code as usize) < 4096 {
                    prefix[next_code as usize] = prev;
                    suffix[next_code as usize] = first_char[code as usize];
                    first_char[next_code as usize] = first_char[prev as usize];
                    next_code += 1;
                    if next_code == (1u16 << code_bits) - 1 && code_bits < 12 {
                        code_bits += 1;
                    }
                }
            }
            prev_code = Some(code);
        } else if code == next_code {
            let prev = prev_code.ok_or_else(|| {
                ImageDecodeError::InvalidTiff("LZW KwKwK code without previous code".into())
            })?;
            let fc = first_char[prev as usize];
            stack.clear();
            let mut cur = prev;
            let mut steps = 0usize;
            while cur != u16::MAX {
                if steps >= 4096 {
                    return Err(ImageDecodeError::InvalidTiff(
                        "LZW dictionary cycle".into(),
                    ));
                }
                stack.push(suffix[cur as usize]);
                cur = prefix[cur as usize];
                steps += 1;
            }
            for &b in stack.iter().rev() {
                out.push(b);
            }
            out.push(fc);
            if (next_code as usize) < 4096 {
                prefix[next_code as usize] = prev;
                suffix[next_code as usize] = fc;
                first_char[next_code as usize] = fc;
                next_code += 1;
                if next_code == (1u16 << code_bits) - 1 && code_bits < 12 {
                    code_bits += 1;
                }
            }
            prev_code = Some(code);
        } else {
            return Err(ImageDecodeError::InvalidTiff(format!(
                "invalid LZW code {} (next_code={})",
                code, next_code
            )));
        }

        if expected_len > 0 && out.len() >= expected_len {
            break;
        }
    }

    Ok(out)
}

fn decompress_packbits(src: &[u8], expected_len: usize) -> Result<Vec<u8>, ImageDecodeError> {
    let mut out = Vec::with_capacity(expected_len);
    let mut i = 0usize;
    while i < src.len() {
        let n = src[i] as i8;
        i += 1;
        if n >= 0 {
            let count = (n as usize) + 1;
            let end = i + count;
            if end > src.len() {
                return Err(ImageDecodeError::InvalidTiff(
                    "PackBits literal run truncated".into(),
                ));
            }
            out.extend_from_slice(&src[i..end]);
            i = end;
        } else if n != -128 {
            let count = ((-n) as usize) + 1;
            let val = *src.get(i).ok_or_else(|| {
                ImageDecodeError::InvalidTiff("PackBits repeat run truncated".into())
            })?;
            i += 1;
            out.resize(out.len() + count, val);
        }
        if expected_len > 0 && out.len() >= expected_len {
            break;
        }
    }
    Ok(out)
}

fn decompress_chunk(
    src: &[u8],
    compression: u16,
    expected_len: usize,
) -> Result<Vec<u8>, ImageDecodeError> {
    match compression {
        1 => Ok(src.to_vec()),
        5 => decompress_lzw(src, expected_len),
        8 | 32946 => miniz_oxide::inflate::decompress_to_vec_zlib(src)
            .or_else(|_| miniz_oxide::inflate::decompress_to_vec(src))
            .map_err(|e| ImageDecodeError::InvalidTiff(format!("ZIP/Deflate error: {:?}", e))),
        32773 => decompress_packbits(src, expected_len),
        other => Err(ImageDecodeError::InvalidTiff(format!(
            "unsupported TIFF compression {}",
            other
        ))),
    }
}

fn undo_horizontal_predictor(
    buf: &mut [u8],
    row_width: usize,
    rows: usize,
    samples_per_pixel: usize,
    bits_per_sample: u16,
    end: TiffEndianness,
) -> Result<(), ImageDecodeError> {
    match bits_per_sample {
        8 => {
            let row_bytes = row_width * samples_per_pixel;
            for r in 0..rows {
                let start = r * row_bytes;
                let end_off = (start + row_bytes).min(buf.len());
                if start + samples_per_pixel >= end_off {
                    continue;
                }
                for i in (start + samples_per_pixel)..end_off {
                    buf[i] = buf[i].wrapping_add(buf[i - samples_per_pixel]);
                }
            }
            Ok(())
        }
        16 => {
            let row_bytes = row_width * samples_per_pixel * 2;
            let stride = samples_per_pixel * 2;
            for r in 0..rows {
                let start = r * row_bytes;
                let end_off = (start + row_bytes).min(buf.len());
                let mut i = start + stride;
                while i + 1 < end_off {
                    let prev = match end {
                        TiffEndianness::Little => u16::from_le_bytes([buf[i - stride], buf[i - stride + 1]]),
                        TiffEndianness::Big => u16::from_be_bytes([buf[i - stride], buf[i - stride + 1]]),
                    };
                    let cur = match end {
                        TiffEndianness::Little => u16::from_le_bytes([buf[i], buf[i + 1]]),
                        TiffEndianness::Big => u16::from_be_bytes([buf[i], buf[i + 1]]),
                    };
                    let sum = cur.wrapping_add(prev);
                    let bytes = match end {
                        TiffEndianness::Little => sum.to_le_bytes(),
                        TiffEndianness::Big => sum.to_be_bytes(),
                    };
                    buf[i] = bytes[0];
                    buf[i + 1] = bytes[1];
                    i += 2;
                }
            }
            Ok(())
        }
        other => Err(ImageDecodeError::InvalidTiff(format!(
            "horizontal predictor unsupported for {}-bit samples",
            other
        ))),
    }
}

fn decode_ifd(
    bytes: &[u8],
    end: TiffEndianness,
    ifd: &ParsedIfd,
    page_count: usize,
) -> Result<DecodedImage, ImageDecodeError> {
    let w = ifd.width as usize;
    let h = ifd.height as usize;
    let spp = ifd.samples_per_pixel as usize;
    if spp == 0 {
        return Err(ImageDecodeError::InvalidTiff(
            "SamplesPerPixel cannot be 0".into(),
        ));
    }

    let bps = *ifd.bits_per_sample.first().unwrap_or(&8);
    if !ifd.bits_per_sample.iter().all(|&b| b == bps) {
        return Err(ImageDecodeError::InvalidTiff(format!(
            "mixed BitsPerSample {:?} is not supported",
            ifd.bits_per_sample
        )));
    }
    if bps != 1 && bps != 8 && bps != 16 {
        return Err(ImageDecodeError::InvalidTiff(format!(
            "unsupported BitsPerSample {}",
            bps
        )));
    }
    if ifd.predictor != 1 && ifd.predictor != 2 {
        return Err(ImageDecodeError::InvalidTiff(format!(
            "unsupported Predictor {}",
            ifd.predictor
        )));
    }

    let samples_per_chunk_pixel = if ifd.planar_configuration == 2 { 1 } else { spp };
    let is_tiled = ifd.tile_width.is_some() && !ifd.tile_offsets.is_empty();

    // Decode into chunky u8-per-sample (or u16->u8 downscaled) buffer of len w * h * spp
    let mut chunky_samples = vec![0u8; w * h * spp];

    if is_tiled {
        let tw = ifd.tile_width.unwrap_or(0) as usize;
        let th = ifd.tile_length.unwrap_or(0) as usize;
        if tw == 0 || th == 0 {
            return Err(ImageDecodeError::InvalidTiff(
                "invalid TileWidth/TileLength".into(),
            ));
        }
        if ifd.tile_offsets.len() != ifd.tile_byte_counts.len() {
            return Err(ImageDecodeError::InvalidTiff(
                "TileOffsets/TileByteCounts length mismatch".into(),
            ));
        }
        let tiles_across = w.div_ceil(tw);
        let tiles_down = h.div_ceil(th);
        let tiles_per_plane = tiles_across * tiles_down;
        let expected_tiles = if ifd.planar_configuration == 2 {
            tiles_per_plane * spp
        } else {
            tiles_per_plane
        };
        if ifd.tile_offsets.len() < expected_tiles {
            return Err(ImageDecodeError::InvalidTiff(
                "insufficient tiles for image dimensions".into(),
            ));
        }

        let row_bytes_tile = match bps {
            1 => (tw * samples_per_chunk_pixel).div_ceil(8),
            8 => tw * samples_per_chunk_pixel,
            16 => tw * samples_per_chunk_pixel * 2,
            _ => unreachable!(),
        };
        let expected_tile_bytes = row_bytes_tile * th;

        for (tile_idx, (&off, &cnt)) in ifd
            .tile_offsets
            .iter()
            .zip(ifd.tile_byte_counts.iter())
            .take(expected_tiles)
            .enumerate()
        {
            let start = off as usize;
            let end_off = start + cnt as usize;
            let compressed = bytes.get(start..end_off).ok_or_else(|| {
                ImageDecodeError::InvalidTiff("tile data out of bounds".into())
            })?;
            let mut raw = decompress_chunk(compressed, ifd.compression, expected_tile_bytes)?;
            if ifd.predictor == 2 {
                undo_horizontal_predictor(
                    &mut raw,
                    tw,
                    th,
                    samples_per_chunk_pixel,
                    bps,
                    end,
                )?;
            }

            let (plane, local_tile_idx) = if ifd.planar_configuration == 2 {
                (tile_idx / tiles_per_plane, tile_idx % tiles_per_plane)
            } else {
                (0, tile_idx)
            };
            let tx = (local_tile_idx % tiles_across) * tw;
            let ty = (local_tile_idx / tiles_across) * th;
            let copy_w = tw.min(w.saturating_sub(tx));
            let copy_h = th.min(h.saturating_sub(ty));

            for r in 0..copy_h {
                let src_row_off = r * row_bytes_tile;
                for c in 0..copy_w {
                    let dst_px = (ty + r) * w + (tx + c);
                    if ifd.planar_configuration == 2 {
                        let sample = extract_sample(&raw, src_row_off, c, bps, end)?;
                        chunky_samples[dst_px * spp + plane] = sample;
                    } else {
                        for s in 0..spp {
                            let sample = extract_sample(&raw, src_row_off, c * spp + s, bps, end)?;
                            chunky_samples[dst_px * spp + s] = sample;
                        }
                    }
                }
            }
        }
    } else {
        if ifd.strip_offsets.len() != ifd.strip_byte_counts.len() || ifd.strip_offsets.is_empty() {
            return Err(ImageDecodeError::InvalidTiff(
                "invalid StripOffsets/StripByteCounts".into(),
            ));
        }
        let rps = ifd.rows_per_strip as usize;
        let strips_per_plane = h.div_ceil(rps);
        let expected_strips = if ifd.planar_configuration == 2 {
            strips_per_plane * spp
        } else {
            strips_per_plane
        };
        if ifd.strip_offsets.len() < expected_strips {
            return Err(ImageDecodeError::InvalidTiff(
                "insufficient strips for image dimensions".into(),
            ));
        }

        let row_bytes = match bps {
            1 => (w * samples_per_chunk_pixel).div_ceil(8),
            8 => w * samples_per_chunk_pixel,
            16 => w * samples_per_chunk_pixel * 2,
            _ => unreachable!(),
        };

        for (strip_idx, (&off, &cnt)) in ifd
            .strip_offsets
            .iter()
            .zip(ifd.strip_byte_counts.iter())
            .take(expected_strips)
            .enumerate()
        {
            let (plane, local_strip) = if ifd.planar_configuration == 2 {
                (strip_idx / strips_per_plane, strip_idx % strips_per_plane)
            } else {
                (0, strip_idx)
            };
            let y0 = local_strip * rps;
            let rows_in_strip = rps.min(h.saturating_sub(y0));
            let expected_strip_bytes = row_bytes * rows_in_strip;

            let start = off as usize;
            let end_off = start + cnt as usize;
            let compressed = bytes.get(start..end_off).ok_or_else(|| {
                ImageDecodeError::InvalidTiff("strip data out of bounds".into())
            })?;
            let mut raw = decompress_chunk(compressed, ifd.compression, expected_strip_bytes)?;
            if ifd.predictor == 2 {
                undo_horizontal_predictor(
                    &mut raw,
                    w,
                    rows_in_strip,
                    samples_per_chunk_pixel,
                    bps,
                    end,
                )?;
            }

            for r in 0..rows_in_strip {
                let src_row_off = r * row_bytes;
                for c in 0..w {
                    let dst_px = (y0 + r) * w + c;
                    if ifd.planar_configuration == 2 {
                        let sample = extract_sample(&raw, src_row_off, c, bps, end)?;
                        chunky_samples[dst_px * spp + plane] = sample;
                    } else {
                        for s in 0..spp {
                            let sample = extract_sample(&raw, src_row_off, c * spp + s, bps, end)?;
                            chunky_samples[dst_px * spp + s] = sample;
                        }
                    }
                }
            }
        }
    }

    let (dpi_x, dpi_y) = ifd.dpi();
    let icc_profile = ifd.icc_profile.clone();

    match ifd.photometric {
        // 0 = WhiteIsZero, 1 = BlackIsZero
        0 | 1 => {
            let invert = ifd.photometric == 0;
            let mut pixels = Vec::with_capacity(w * h);
            let mut alpha = if spp >= 2 {
                Some(Vec::with_capacity(w * h))
            } else {
                None
            };
            for px in 0..(w * h) {
                let g = chunky_samples[px * spp];
                pixels.push(if invert { 255 - g } else { g });
                if let Some(ref mut a) = alpha {
                    a.push(chunky_samples[px * spp + 1]);
                }
            }
            Ok(DecodedImage {
                width: ifd.width,
                height: ifd.height,
                pixels,
                color_model: ImageColorModel::Gray8,
                alpha,
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
                dpi_x,
                dpi_y,
                icc_profile,
                page_count: Some(page_count),
            })
        }
        // 2 = RGB (optionally with ExtraSamples alpha)
        2 => {
            if spp < 3 {
                return Err(ImageDecodeError::InvalidTiff(format!(
                    "RGB TIFF requires >= 3 samples/pixel, got {}",
                    spp
                )));
            }
            let has_alpha = spp >= 4;
            let is_premul = ifd.extra_samples.first().copied() == Some(1);
            let mut pixels = Vec::with_capacity(w * h * 3);
            let mut alpha = if has_alpha {
                Some(Vec::with_capacity(w * h))
            } else {
                None
            };
            for px in 0..(w * h) {
                let base = px * spp;
                let mut r = chunky_samples[base];
                let mut g = chunky_samples[base + 1];
                let mut b = chunky_samples[base + 2];
                if let Some(ref mut al) = alpha {
                    let a = chunky_samples[base + 3];
                    if is_premul && a > 0 && a < 255 {
                        let af = a as f32 / 255.0;
                        r = ((r as f32 / af).round()).clamp(0.0, 255.0) as u8;
                        g = ((g as f32 / af).round()).clamp(0.0, 255.0) as u8;
                        b = ((b as f32 / af).round()).clamp(0.0, 255.0) as u8;
                    }
                    al.push(a);
                }
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }
            Ok(DecodedImage {
                width: ifd.width,
                height: ifd.height,
                pixels,
                color_model: ImageColorModel::Rgb8,
                alpha,
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
                dpi_x,
                dpi_y,
                icc_profile,
                page_count: Some(page_count),
            })
        }
        // 3 = Palette (ColorMap)
        3 => {
            let entries = 1usize << bps.min(8);
            if ifd.color_map.len() < entries * 3 {
                return Err(ImageDecodeError::InvalidTiff(
                    "Palette TIFF missing ColorMap entries".into(),
                ));
            }
            let mut pixels = Vec::with_capacity(w * h * 3);
            for px in 0..(w * h) {
                let idx = chunky_samples[px * spp] as usize;
                if idx >= entries {
                    return Err(ImageDecodeError::InvalidTiff(
                        "Palette index out of range".into(),
                    ));
                }
                pixels.push((ifd.color_map[idx] >> 8) as u8);
                pixels.push((ifd.color_map[entries + idx] >> 8) as u8);
                pixels.push((ifd.color_map[entries * 2 + idx] >> 8) as u8);
            }
            Ok(DecodedImage {
                width: ifd.width,
                height: ifd.height,
                pixels,
                color_model: ImageColorModel::Rgb8,
                alpha: None,
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
                dpi_x,
                dpi_y,
                icc_profile,
                page_count: Some(page_count),
            })
        }
        // 5 = Separated (CMYK, optionally with ExtraSamples alpha)
        5 => {
            if spp < 4 {
                return Err(ImageDecodeError::InvalidTiff(format!(
                    "CMYK TIFF requires >= 4 samples/pixel, got {}",
                    spp
                )));
            }
            let has_alpha = spp >= 5;
            let mut pixels = Vec::with_capacity(w * h * 4);
            let mut alpha = if has_alpha {
                Some(Vec::with_capacity(w * h))
            } else {
                None
            };
            for px in 0..(w * h) {
                let base = px * spp;
                pixels.push(chunky_samples[base]);
                pixels.push(chunky_samples[base + 1]);
                pixels.push(chunky_samples[base + 2]);
                pixels.push(chunky_samples[base + 3]);
                if let Some(ref mut al) = alpha {
                    al.push(chunky_samples[base + 4]);
                }
            }
            Ok(DecodedImage {
                width: ifd.width,
                height: ifd.height,
                pixels,
                color_model: ImageColorModel::Cmyk8,
                alpha,
                raw_jpeg_bytes: None,
                jpeg_cmyk_inverted: false,
                dpi_x,
                dpi_y,
                icc_profile,
                page_count: Some(page_count),
            })
        }
        other => Err(ImageDecodeError::InvalidTiff(format!(
            "unsupported TIFF PhotometricInterpretation {}",
            other
        ))),
    }
}

#[inline]
fn extract_sample(
    raw: &[u8],
    row_off: usize,
    sample_idx: usize,
    bps: u16,
    end: TiffEndianness,
) -> Result<u8, ImageDecodeError> {
    match bps {
        1 => {
            let byte_idx = row_off + (sample_idx / 8);
            let bit = 7 - (sample_idx % 8);
            let b = *raw.get(byte_idx).ok_or_else(|| {
                ImageDecodeError::InvalidTiff("1-bit TIFF strip/tile truncated".into())
            })?;
            Ok(if ((b >> bit) & 1) != 0 { 255 } else { 0 })
        }
        8 => raw.get(row_off + sample_idx).copied().ok_or_else(|| {
            ImageDecodeError::InvalidTiff("8-bit TIFF strip/tile truncated".into())
        }),
        16 => {
            let off = row_off + sample_idx * 2;
            let val = read_u16(raw, off, end)?;
            Ok((val >> 8) as u8)
        }
        _ => unreachable!(),
    }
}

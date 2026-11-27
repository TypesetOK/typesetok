//! ISO 15076-1 (ICC v2 & v4) Profile Parser, Validator, and Loader.
//!
//! Parses binary `.icc` / `.icm` profiles, validates headers and tag tables,
//! decodes TRC curves (`curv`, `para`), XYZ primaries (`XYZ `), chromatic
//! adaptation matrices (`chad`), and multi-dimensional LUTs (`mft1`, `mft2`,
//! `mAB `, `mBA `, `gamt`).

use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

/// Errors returned when parsing or loading an ICC profile.
#[derive(Debug, Error)]
pub enum IccError {
    #[error("ICC profile too short: {len} bytes (minimum 132 bytes required)")]
    TooShort { len: usize },

    #[error("Invalid ICC signature: expected 'acsp', got {0:?}")]
    InvalidSignature([u8; 4]),

    #[error("ICC profile size mismatch: header declares {declared} bytes, actual {actual} bytes")]
    SizeMismatch { declared: usize, actual: usize },

    #[error("Invalid ICC tag table")]
    InvalidTagTable,

    #[error("Malformed ICC tag {tag:?}: {reason}")]
    MalformedTag { tag: [u8; 4], reason: &'static str },

    #[error("I/O error reading ICC profile: {0}")]
    Io(#[from] std::io::Error),
}

/// ICC Rendering Intent (ISO 15076-1 Section 7.2.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum RenderingIntent {
    /// Photographic / gamut-compression intent (`A2B0` / `B2A0`).
    Perceptual = 0,
    /// Media-relative colorimetric intent (`A2B1` / `B2A1`), scales white point to media white.
    #[default]
    RelativeColorimetric = 1,
    /// Vivid graphics / saturation-preserving intent (`A2B2` / `B2A2`).
    Saturation = 2,
    /// ICC-absolute colorimetric intent (`A2B1` / `B2A1` + white-point adaptation).
    AbsoluteColorimetric = 3,
}

impl RenderingIntent {
    /// Parses an ICC header u32 rendering intent value.
    pub fn from_icc_u32(v: u32) -> Self {
        match v {
            0 => Self::Perceptual,
            2 => Self::Saturation,
            3 => Self::AbsoluteColorimetric,
            _ => Self::RelativeColorimetric,
        }
    }

    /// Returns the ICC header u32 value (`0..=3`).
    pub fn to_icc_u32(self) -> u32 {
        self as u32
    }

    /// Parses a CLI / UI string into a [`RenderingIntent`].
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "perceptual" | "0" => Some(Self::Perceptual),
            "relative" | "relative-colorimetric" | "relative_colorimetric" | "1" => {
                Some(Self::RelativeColorimetric)
            }
            "saturation" | "2" => Some(Self::Saturation),
            "absolute" | "absolute-colorimetric" | "absolute_colorimetric" | "3" => {
                Some(Self::AbsoluteColorimetric)
            }
            _ => None,
        }
    }

    /// Returns the PDF `/RI` name (without leading slash).
    pub fn pdf_name(self) -> &'static str {
        match self {
            Self::Perceptual => "Perceptual",
            Self::RelativeColorimetric => "RelativeColorimetric",
            Self::Saturation => "Saturation",
            Self::AbsoluteColorimetric => "AbsoluteColorimetric",
        }
    }

    /// Returns the LUT index (`0` = Perceptual, `1` = Colorimetric, `2` = Saturation).
    pub(crate) fn lut_index(self) -> usize {
        match self {
            Self::Perceptual => 0,
            Self::RelativeColorimetric | Self::AbsoluteColorimetric => 1,
            Self::Saturation => 2,
        }
    }
}

/// ICC Profile Device Class (`header[12..16]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IccDeviceClass {
    Input,
    Display,
    Output,
    Link,
    ColorSpace,
    Abstract,
    NamedColor,
    Other([u8; 4]),
}

impl IccDeviceClass {
    fn from_bytes(b: [u8; 4]) -> Self {
        match &b {
            b"scnr" => Self::Input,
            b"mntr" => Self::Display,
            b"prtr" => Self::Output,
            b"link" => Self::Link,
            b"spac" => Self::ColorSpace,
            b"abst" => Self::Abstract,
            b"nmcl" => Self::NamedColor,
            _ => Self::Other(b),
        }
    }
}

/// ICC Color Space signature (`header[16..20]` and PCS `header[20..24]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IccColorSpace {
    Rgb,
    Cmyk,
    Gray,
    Lab,
    Xyz,
    Other([u8; 4]),
}

impl IccColorSpace {
    fn from_bytes(b: [u8; 4]) -> Self {
        match &b {
            b"RGB " => Self::Rgb,
            b"CMYK" => Self::Cmyk,
            b"GRAY" => Self::Gray,
            b"Lab " => Self::Lab,
            b"XYZ " => Self::Xyz,
            _ => Self::Other(b),
        }
    }

    /// Number of color channels for known spaces.
    pub fn channels(self) -> Option<usize> {
        match self {
            Self::Gray => Some(1),
            Self::Rgb | Self::Lab | Self::Xyz => Some(3),
            Self::Cmyk => Some(4),
            Self::Other(_) => None,
        }
    }
}

/// Parsed 128-byte ICC profile header.
#[derive(Debug, Clone, PartialEq)]
pub struct IccHeader {
    pub size: u32,
    pub cmm_type: [u8; 4],
    pub version_major: u8,
    pub version_minor: u8,
    pub version_bugfix: u8,
    pub device_class: IccDeviceClass,
    pub color_space: IccColorSpace,
    pub pcs: IccColorSpace,
    pub rendering_intent: RenderingIntent,
    pub illuminant: [f32; 3],
    pub creator: [u8; 4],
}

/// 1D Tone Reproduction Curve (`curv` or `para` tag).
#[derive(Debug, Clone, PartialEq)]
pub enum ToneCurve {
    Identity,
    Gamma(f32),
    Lut16(Vec<f32>),
    Parametric { func_type: u16, params: [f32; 7] },
}

impl ToneCurve {
    /// Evaluates the forward TRC mapping `x in [0, 1] -> y in [0, 1]`.
    pub fn eval(&self, x: f32) -> f32 {
        let x = if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
        match self {
            Self::Identity => x,
            Self::Gamma(g) => x.powf(*g),
            Self::Lut16(table) => {
                if table.is_empty() {
                    return x;
                }
                if table.len() == 1 {
                    return x.powf(table[0]);
                }
                let max_idx = (table.len() - 1) as f32;
                let pos = x * max_idx;
                let i0 = (pos.floor() as usize).min(table.len() - 1);
                let i1 = (i0 + 1).min(table.len() - 1);
                let frac = pos - i0 as f32;
                table[i0] * (1.0 - frac) + table[i1] * frac
            }
            Self::Parametric { func_type, params } => {
                let [g, a, b, c, d, e, f] = *params;
                match func_type {
                    0 => x.powf(g),
                    1 => {
                        if x >= -b / a {
                            (a * x + b).powf(g)
                        } else {
                            0.0
                        }
                    }
                    2 => {
                        if x >= -b / a {
                            (a * x + b).powf(g) + c
                        } else {
                            c
                        }
                    }
                    3 => {
                        if x >= d {
                            (a * x + b).powf(g)
                        } else {
                            c * x
                        }
                    }
                    4 => {
                        if x >= d {
                            (a * x + b).powf(g) + e
                        } else {
                            c * x + f
                        }
                    }
                    _ => x,
                }
                .clamp(0.0, 1.0)
            }
        }
    }

    /// Evaluates the inverse TRC mapping `y in [0, 1] -> x in [0, 1]`.
    pub fn eval_inverse(&self, y: f32) -> f32 {
        let y = if y.is_nan() { 0.0 } else { y.clamp(0.0, 1.0) };
        match self {
            Self::Identity => y,
            Self::Gamma(g) => {
                if *g <= 1e-6 {
                    y
                } else {
                    y.powf(1.0 / *g)
                }
            }
            Self::Parametric { func_type: 3, params } => {
                let [g, a, b, c, d, _, _] = *params;
                let threshold = c * d;
                if y >= threshold && a.abs() > 1e-7 && g.abs() > 1e-7 {
                    ((y.powf(1.0 / g) - b) / a).clamp(0.0, 1.0)
                } else if c.abs() > 1e-7 {
                    (y / c).clamp(0.0, 1.0)
                } else {
                    y
                }
            }
            _ => {
                // Monotonic binary search on [0, 1]
                let mut lo = 0.0_f32;
                let mut hi = 1.0_f32;
                for _ in 0..24 {
                    let mid = 0.5 * (lo + hi);
                    if self.eval(mid) < y {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                0.5 * (lo + hi)
            }
        }
    }
}

/// Decoded multi-dimensional ICC LUT (`mft1`, `mft2`, `mAB `, `mBA `).
#[derive(Debug, Clone, PartialEq)]
pub struct IccLut {
    pub in_channels: usize,
    pub out_channels: usize,
    pub grid_points: [usize; 4],
    pub matrix: Option<[f32; 12]>,
    pub a_curves: Vec<ToneCurve>,
    pub m_curves: Vec<ToneCurve>,
    pub b_curves: Vec<ToneCurve>,
    pub clut: Option<Vec<f32>>,
    pub is_mba: bool,
}

impl IccLut {
    /// Evaluates the LUT for an `in_channels`-element slice in `[0.0, 1.0]`.
    pub fn eval(&self, input: &[f32]) -> Vec<f32> {
        let mut vals: Vec<f32> = input
            .iter()
            .take(self.in_channels)
            .map(|&v| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) })
            .collect();
        while vals.len() < self.in_channels {
            vals.push(0.0);
        }

        if self.is_mba {
            // mBA pipeline: B -> Matrix -> M -> CLUT -> A
            for (v, curve) in vals.iter_mut().zip(self.b_curves.iter()) {
                *v = curve.eval(*v);
            }
            if let Some(m) = self.matrix {
                vals = apply_matrix_3x3(&m, &vals);
            }
            for (v, curve) in vals.iter_mut().zip(self.m_curves.iter()) {
                *v = curve.eval(*v);
            }
            if let Some(clut) = &self.clut {
                vals = self.interp_clut(clut, &vals);
            }
            for (v, curve) in vals.iter_mut().zip(self.a_curves.iter()) {
                *v = curve.eval(*v);
            }
        } else {
            // mft1 / mft2 / mAB pipeline:
            // For mft1/mft2: Matrix (if PCS XYZ input) -> A (input) -> CLUT -> B (output)
            // For mAB: A -> CLUT -> M -> Matrix -> B
            if !self.a_curves.is_empty() {
                for (v, curve) in vals.iter_mut().zip(self.a_curves.iter()) {
                    *v = curve.eval(*v);
                }
            }
            if let Some(clut) = &self.clut {
                vals = self.interp_clut(clut, &vals);
            }
            if !self.m_curves.is_empty() {
                for (v, curve) in vals.iter_mut().zip(self.m_curves.iter()) {
                    *v = curve.eval(*v);
                }
            }
            if let Some(m) = self.matrix {
                if vals.len() >= 3 {
                    vals = apply_matrix_3x3(&m, &vals);
                }
            }
            if !self.b_curves.is_empty() {
                for (v, curve) in vals.iter_mut().zip(self.b_curves.iter()) {
                    *v = curve.eval(*v);
                }
            }
        }

        vals.truncate(self.out_channels);
        while vals.len() < self.out_channels {
            vals.push(0.0);
        }
        vals
    }

    fn interp_clut(&self, clut: &[f32], x: &[f32]) -> Vec<f32> {
        match self.in_channels {
            3 => interp_nd::<3>(
                [self.grid_points[0], self.grid_points[1], self.grid_points[2]],
                self.out_channels,
                clut,
                [x[0], x[1], x[2]],
            ),
            4 => interp_nd::<4>(
                self.grid_points,
                self.out_channels,
                clut,
                [x[0], x[1], x[2], x[3]],
            ),
            _ => x.to_vec(),
        }
    }
}

fn apply_matrix_3x3(m: &[f32; 12], v: &[f32]) -> Vec<f32> {
    if v.len() < 3 {
        return v.to_vec();
    }
    let x = v[0];
    let y = v[1];
    let z = v[2];
    vec![
        (m[0] * x + m[1] * y + m[2] * z + m[9]).clamp(0.0, 1.0),
        (m[3] * x + m[4] * y + m[5] * z + m[10]).clamp(0.0, 1.0),
        (m[6] * x + m[7] * y + m[8] * z + m[11]).clamp(0.0, 1.0),
    ]
}

fn interp_nd<const N: usize>(
    grids: [usize; N],
    out_ch: usize,
    clut: &[f32],
    x: [f32; N],
) -> Vec<f32> {
    let mut i0 = [0usize; N];
    let mut i1 = [0usize; N];
    let mut d = [0.0f32; N];
    for axis in 0..N {
        let g = grids[axis].max(2);
        let n = (g - 1) as f32;
        let fx = (x[axis].clamp(0.0, 1.0) * n).min(n - 1e-6);
        i0[axis] = fx.floor() as usize;
        i1[axis] = (i0[axis] + 1).min(g - 1);
        d[axis] = fx - i0[axis] as f32;
    }

    let mut out = vec![0.0_f32; out_ch];
    let corners = 1usize << N;
    for mask in 0..corners {
        let mut w = 1.0_f32;
        let mut idx = 0usize;
        for axis in 0..N {
            let bit = (mask >> axis) & 1;
            let coord = if bit == 0 {
                w *= 1.0 - d[axis];
                i0[axis]
            } else {
                w *= d[axis];
                i1[axis]
            };
            idx = idx * grids[axis].max(2) + coord;
        }
        let base = idx * out_ch;
        if base + out_ch <= clut.len() {
            for c in 0..out_ch {
                out[c] += w * clut[base + c];
            }
        }
    }
    out
}

/// Parsed ISO 15076-1 ICC color profile.
#[derive(Debug, Clone)]
pub struct IccProfile {
    raw_bytes: Vec<u8>,
    header: IccHeader,
    description: String,
    pub media_white_point: Option<[f32; 3]>,
    pub chromatic_adaptation: Option<[f32; 9]>,
    pub rgb_primaries: Option<[[f32; 3]; 3]>,
    pub rgb_trc: Option<[ToneCurve; 3]>,
    pub gray_trc: Option<ToneCurve>,
    a2b: [Option<IccLut>; 3],
    b2a: [Option<IccLut>; 3],
    gamut: Option<IccLut>,
}

impl IccProfile {
    /// Parses and validates an ICC profile from raw bytes.
    pub fn from_bytes(raw_bytes: Vec<u8>) -> Result<Self, IccError> {
        if raw_bytes.len() < 132 {
            return Err(IccError::TooShort {
                len: raw_bytes.len(),
            });
        }
        let declared_size = read_u32(&raw_bytes, 0) as usize;
        if declared_size > raw_bytes.len() || declared_size < 132 {
            return Err(IccError::SizeMismatch {
                declared: declared_size,
                actual: raw_bytes.len(),
            });
        }
        let sig: [u8; 4] = raw_bytes[36..40].try_into().unwrap();
        if &sig != b"acsp" {
            return Err(IccError::InvalidSignature(sig));
        }

        let header = IccHeader {
            size: declared_size as u32,
            cmm_type: raw_bytes[4..8].try_into().unwrap(),
            version_major: raw_bytes[8],
            version_minor: (raw_bytes[9] >> 4) & 0x0F,
            version_bugfix: raw_bytes[9] & 0x0F,
            device_class: IccDeviceClass::from_bytes(raw_bytes[12..16].try_into().unwrap()),
            color_space: IccColorSpace::from_bytes(raw_bytes[16..20].try_into().unwrap()),
            pcs: IccColorSpace::from_bytes(raw_bytes[20..24].try_into().unwrap()),
            rendering_intent: RenderingIntent::from_icc_u32(read_u32(&raw_bytes, 64)),
            illuminant: [
                read_s15fixed16(&raw_bytes, 68),
                read_s15fixed16(&raw_bytes, 72),
                read_s15fixed16(&raw_bytes, 76),
            ],
            creator: raw_bytes[80..84].try_into().unwrap(),
        };

        let tag_count = read_u32(&raw_bytes, 128) as usize;
        let table_end = 132usize
            .checked_add(tag_count.saturating_mul(12))
            .ok_or(IccError::InvalidTagTable)?;
        if table_end > raw_bytes.len() {
            return Err(IccError::InvalidTagTable);
        }

        let mut tags: BTreeMap<[u8; 4], (usize, usize)> = BTreeMap::new();
        for i in 0..tag_count {
            let entry = 132 + i * 12;
            let tag_sig: [u8; 4] = raw_bytes[entry..entry + 4].try_into().unwrap();
            let offset = read_u32(&raw_bytes, entry + 4) as usize;
            let size = read_u32(&raw_bytes, entry + 8) as usize;
            if offset < 132 || offset.saturating_add(size) > raw_bytes.len() {
                return Err(IccError::InvalidTagTable);
            }
            tags.insert(tag_sig, (offset, size));
        }

        let slice_for = |sig: &[u8; 4]| -> Option<&[u8]> {
            tags.get(sig).map(|&(off, len)| &raw_bytes[off..off + len])
        };

        let description = slice_for(b"desc")
            .and_then(parse_description_tag)
            .unwrap_or_else(|| "Custom ICC Profile".to_string());

        let media_white_point = slice_for(b"wtpt").and_then(parse_xyz_tag);

        let chromatic_adaptation = slice_for(b"chad").and_then(parse_chad_tag);

        let rgb_primaries = match (
            slice_for(b"rXYZ").and_then(parse_xyz_tag),
            slice_for(b"gXYZ").and_then(parse_xyz_tag),
            slice_for(b"bXYZ").and_then(parse_xyz_tag),
        ) {
            (Some(r), Some(g), Some(b)) => Some([r, g, b]),
            _ => None,
        };

        let rgb_trc = match (
            slice_for(b"rTRC").and_then(parse_trc_tag),
            slice_for(b"gTRC").and_then(parse_trc_tag),
            slice_for(b"bTRC").and_then(parse_trc_tag),
        ) {
            (Some(r), Some(g), Some(b)) => Some([r, g, b]),
            _ => None,
        };

        let gray_trc = slice_for(b"kTRC").and_then(parse_trc_tag);

        let a2b = [
            slice_for(b"A2B0").and_then(|d| parse_lut_tag(*b"A2B0", d).ok()),
            slice_for(b"A2B1").and_then(|d| parse_lut_tag(*b"A2B1", d).ok()),
            slice_for(b"A2B2").and_then(|d| parse_lut_tag(*b"A2B2", d).ok()),
        ];

        let b2a = [
            slice_for(b"B2A0").and_then(|d| parse_lut_tag(*b"B2A0", d).ok()),
            slice_for(b"B2A1").and_then(|d| parse_lut_tag(*b"B2A1", d).ok()),
            slice_for(b"B2A2").and_then(|d| parse_lut_tag(*b"B2A2", d).ok()),
        ];

        let gamut = slice_for(b"gamt").and_then(|d| parse_lut_tag(*b"gamt", d).ok());

        Ok(Self {
            raw_bytes,
            header,
            description,
            media_white_point,
            chromatic_adaptation,
            rgb_primaries,
            rgb_trc,
            gray_trc,
            a2b,
            b2a,
            gamut,
        })
    }

    /// Loads and parses an `.icc` / `.icm` profile from disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, IccError> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(bytes)
    }

    /// Loads one of the built-in CMYK output profiles.
    pub fn from_builtin(id: crate::color::IccProfileId) -> Result<Self, IccError> {
        let bytes = crate::color::ColorManagementEngine::generate_icc_profile_bytes(id);
        Self::from_bytes(bytes)
    }

    /// Constructs a standard D50-adapted sRGB display profile (IEC 61966-2-1).
    pub fn srgb_builtin() -> Self {
        Self {
            raw_bytes: Vec::new(),
            header: IccHeader {
                size: 0,
                cmm_type: *b"TOK ",
                version_major: 2,
                version_minor: 4,
                version_bugfix: 0,
                device_class: IccDeviceClass::Display,
                color_space: IccColorSpace::Rgb,
                pcs: IccColorSpace::Xyz,
                rendering_intent: RenderingIntent::RelativeColorimetric,
                illuminant: [0.9642, 1.0, 0.8249],
                creator: *b"TOK ",
            },
            description: "sRGB IEC61966-2.1 (D50 Adapted)".to_string(),
            media_white_point: Some([0.9642, 1.0, 0.8249]),
            chromatic_adaptation: None,
            // Bradford D65 -> D50 adapted sRGB primaries
            rgb_primaries: Some([
                [0.4360747, 0.2225045, 0.0139322],
                [0.3850649, 0.7168786, 0.0971045],
                [0.1430804, 0.0606169, 0.7141733],
            ]),
            rgb_trc: Some([
                ToneCurve::Parametric {
                    func_type: 3,
                    params: [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045, 0.0, 0.0],
                },
                ToneCurve::Parametric {
                    func_type: 3,
                    params: [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045, 0.0, 0.0],
                },
                ToneCurve::Parametric {
                    func_type: 3,
                    params: [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045, 0.0, 0.0],
                },
            ]),
            gray_trc: None,
            a2b: [None, None, None],
            b2a: [None, None, None],
            gamut: None,
        }
    }

    /// Raw ICC profile bytes suitable for embedding in PDF/X `DestOutputProfile`.
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub fn header(&self) -> &IccHeader {
        &self.header
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn is_cmyk_output(&self) -> bool {
        self.header.color_space == IccColorSpace::Cmyk
    }

    pub fn is_rgb(&self) -> bool {
        self.header.color_space == IccColorSpace::Rgb
    }

    /// Selects the best matching `A2B` LUT for the requested rendering intent,
    /// falling back to `A2B0` if the intent-specific tag is absent.
    pub fn select_a2b(&self, intent: RenderingIntent) -> Option<&IccLut> {
        let idx = intent.lut_index();
        self.a2b[idx].as_ref().or_else(|| self.a2b[0].as_ref())
    }

    /// Selects the best matching `B2A` LUT for the requested rendering intent,
    /// falling back to `B2A0` if the intent-specific tag is absent.
    pub fn select_b2a(&self, intent: RenderingIntent) -> Option<&IccLut> {
        let idx = intent.lut_index();
        self.b2a[idx].as_ref().or_else(|| self.b2a[0].as_ref())
    }

    /// Returns the `gamt` (out-of-gamut) LUT if present in the profile.
    pub fn gamut_lut(&self) -> Option<&IccLut> {
        self.gamut.as_ref()
    }
}

fn read_u16(b: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([b[off], b[off + 1]])
}

fn read_u32(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn read_s15fixed16(b: &[u8], off: usize) -> f32 {
    i32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]) as f32 / 65536.0
}

fn parse_description_tag(data: &[u8]) -> Option<String> {
    if data.len() < 12 {
        return None;
    }
    match &data[0..4] {
        b"desc" => {
            let ascii_len = read_u32(data, 8) as usize;
            let end = (12 + ascii_len).min(data.len());
            let raw = &data[12..end];
            let trimmed = raw.split(|&b| b == 0).next().unwrap_or(raw);
            Some(String::from_utf8_lossy(trimmed).trim().to_string())
        }
        b"mluc" if data.len() >= 28 => {
            let count = read_u32(data, 8) as usize;
            if count == 0 {
                return None;
            }
            let str_len = read_u32(data, 20) as usize;
            let str_off = read_u32(data, 24) as usize;
            if str_off + str_len > data.len() || str_len < 2 {
                return None;
            }
            let u16s: Vec<u16> = data[str_off..str_off + str_len]
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            Some(String::from_utf16_lossy(&u16s).trim_end_matches('\0').to_string())
        }
        _ => None,
    }
}

fn parse_xyz_tag(data: &[u8]) -> Option<[f32; 3]> {
    if data.len() < 20 || &data[0..4] != b"XYZ " {
        return None;
    }
    Some([
        read_s15fixed16(data, 8),
        read_s15fixed16(data, 12),
        read_s15fixed16(data, 16),
    ])
}

fn parse_chad_tag(data: &[u8]) -> Option<[f32; 9]> {
    if data.len() < 44 || &data[0..4] != b"sf32" {
        return None;
    }
    let mut m = [0.0f32; 9];
    for (i, slot) in m.iter_mut().enumerate() {
        *slot = read_s15fixed16(data, 8 + i * 4);
    }
    Some(m)
}

fn parse_trc_tag(data: &[u8]) -> Option<ToneCurve> {
    if data.len() < 12 {
        return None;
    }
    match &data[0..4] {
        b"curv" => {
            let count = read_u32(data, 8) as usize;
            if count == 0 {
                Some(ToneCurve::Identity)
            } else if count == 1 && data.len() >= 14 {
                // u8Fixed8Number
                let gamma = read_u16(data, 12) as f32 / 256.0;
                Some(ToneCurve::Gamma(gamma))
            } else if data.len() >= 12 + count * 2 {
                let mut table = Vec::with_capacity(count);
                for i in 0..count {
                    table.push(read_u16(data, 12 + i * 2) as f32 / 65535.0);
                }
                Some(ToneCurve::Lut16(table))
            } else {
                None
            }
        }
        b"para" if data.len() >= 16 => {
            let func_type = read_u16(data, 8);
            let param_count = match func_type {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => return None,
            };
            if data.len() < 12 + param_count * 4 {
                return None;
            }
            let mut params = [0.0f32; 7];
            for (i, slot) in params.iter_mut().take(param_count).enumerate() {
                *slot = read_s15fixed16(data, 12 + i * 4);
            }
            Some(ToneCurve::Parametric { func_type, params })
        }
        _ => None,
    }
}

fn parse_lut_tag(tag: [u8; 4], data: &[u8]) -> Result<IccLut, IccError> {
    if data.len() < 32 {
        return Err(IccError::MalformedTag {
            tag,
            reason: "LUT tag shorter than 32 bytes",
        });
    }
    match &data[0..4] {
        b"mft1" => parse_mft1(tag, data),
        b"mft2" => parse_mft2(tag, data),
        b"mAB " => parse_mab_mba(tag, data, false),
        b"mBA " => parse_mab_mba(tag, data, true),
        _ => Err(IccError::MalformedTag {
            tag,
            reason: "unsupported LUT tag signature",
        }),
    }
}

fn parse_mft1(tag: [u8; 4], data: &[u8]) -> Result<IccLut, IccError> {
    if data.len() < 48 {
        return Err(IccError::MalformedTag {
            tag,
            reason: "mft1 header truncated",
        });
    }
    let in_ch = data[8] as usize;
    let out_ch = data[9] as usize;
    let grid = data[10] as usize;
    if in_ch == 0 || out_ch == 0 || grid < 2 {
        return Err(IccError::MalformedTag {
            tag,
            reason: "invalid mft1 dimensions",
        });
    }
    let in_len = in_ch * 256;
    let clut_len = grid.pow(in_ch as u32) * out_ch;
    let out_len = out_ch * 256;
    if data.len() < 48 + in_len + clut_len + out_len {
        return Err(IccError::MalformedTag {
            tag,
            reason: "mft1 data truncated",
        });
    }
    let mut off = 48;
    let mut a_curves = Vec::with_capacity(in_ch);
    for _ in 0..in_ch {
        let t: Vec<f32> = data[off..off + 256].iter().map(|&v| v as f32 / 255.0).collect();
        a_curves.push(ToneCurve::Lut16(t));
        off += 256;
    }
    let clut: Vec<f32> = data[off..off + clut_len]
        .iter()
        .map(|&v| v as f32 / 255.0)
        .collect();
    off += clut_len;
    let mut b_curves = Vec::with_capacity(out_ch);
    for _ in 0..out_ch {
        let t: Vec<f32> = data[off..off + 256].iter().map(|&v| v as f32 / 255.0).collect();
        b_curves.push(ToneCurve::Lut16(t));
        off += 256;
    }

    Ok(IccLut {
        in_channels: in_ch,
        out_channels: out_ch,
        grid_points: [grid, grid, grid, grid],
        matrix: None,
        a_curves,
        m_curves: Vec::new(),
        b_curves,
        clut: Some(clut),
        is_mba: false,
    })
}

fn parse_mft2(tag: [u8; 4], data: &[u8]) -> Result<IccLut, IccError> {
    if data.len() < 52 {
        return Err(IccError::MalformedTag {
            tag,
            reason: "mft2 header truncated",
        });
    }
    let in_ch = data[8] as usize;
    let out_ch = data[9] as usize;
    let grid = data[10] as usize;
    let in_entries = read_u16(data, 48) as usize;
    let out_entries = read_u16(data, 50) as usize;
    if in_ch == 0 || out_ch == 0 || grid < 2 {
        return Err(IccError::MalformedTag {
            tag,
            reason: "invalid mft2 dimensions",
        });
    }
    let clut_nodes = grid
        .checked_pow(in_ch as u32)
        .and_then(|n| n.checked_mul(out_ch))
        .ok_or(IccError::MalformedTag {
            tag,
            reason: "mft2 CLUT size overflow",
        })?;
    let needed = 52 + (in_ch * in_entries + clut_nodes + out_ch * out_entries) * 2;
    if data.len() < needed {
        return Err(IccError::MalformedTag {
            tag,
            reason: "mft2 data truncated",
        });
    }

    let mut off = 52;
    let mut a_curves = Vec::with_capacity(in_ch);
    for _ in 0..in_ch {
        let mut t = Vec::with_capacity(in_entries);
        for _ in 0..in_entries {
            t.push(read_u16(data, off) as f32 / 65535.0);
            off += 2;
        }
        a_curves.push(ToneCurve::Lut16(t));
    }
    let mut clut = Vec::with_capacity(clut_nodes);
    for _ in 0..clut_nodes {
        clut.push(read_u16(data, off) as f32 / 65535.0);
        off += 2;
    }
    let mut b_curves = Vec::with_capacity(out_ch);
    for _ in 0..out_ch {
        let mut t = Vec::with_capacity(out_entries);
        for _ in 0..out_entries {
            t.push(read_u16(data, off) as f32 / 65535.0);
            off += 2;
        }
        b_curves.push(ToneCurve::Lut16(t));
    }

    Ok(IccLut {
        in_channels: in_ch,
        out_channels: out_ch,
        grid_points: [grid, grid, grid, grid],
        matrix: None,
        a_curves,
        m_curves: Vec::new(),
        b_curves,
        clut: Some(clut),
        is_mba: false,
    })
}

fn parse_mab_mba(tag: [u8; 4], data: &[u8], is_mba: bool) -> Result<IccLut, IccError> {
    if data.len() < 32 {
        return Err(IccError::MalformedTag {
            tag,
            reason: "mAB/mBA header truncated",
        });
    }
    let in_ch = data[8] as usize;
    let out_ch = data[9] as usize;
    let b_off = read_u32(data, 12) as usize;
    let mat_off = read_u32(data, 16) as usize;
    let m_off = read_u32(data, 20) as usize;
    let clut_off = read_u32(data, 24) as usize;
    let a_off = read_u32(data, 28) as usize;

    let b_count = if is_mba { in_ch } else { out_ch };
    let a_count = if is_mba { out_ch } else { in_ch };

    let b_curves = if b_off > 0 {
        parse_curve_sequence(data, b_off, b_count)
    } else {
        Vec::new()
    };
    let m_curves = if m_off > 0 {
        parse_curve_sequence(data, m_off, b_count)
    } else {
        Vec::new()
    };
    let a_curves = if a_off > 0 {
        parse_curve_sequence(data, a_off, a_count)
    } else {
        Vec::new()
    };

    let matrix = if mat_off > 0 && mat_off + 48 <= data.len() {
        let mut m = [0.0f32; 12];
        for (i, slot) in m.iter_mut().enumerate() {
            *slot = read_s15fixed16(data, mat_off + i * 4);
        }
        Some(m)
    } else {
        None
    };

    let (grid_points, clut) = if clut_off > 0 && clut_off + 20 <= data.len() {
        let mut gp = [1usize; 4];
        let mut total_nodes = 1usize;
        for axis in 0..in_ch.min(4) {
            let g = (data[clut_off + axis] as usize).max(2);
            gp[axis] = g;
            total_nodes = total_nodes.saturating_mul(g);
        }
        let precision = data[clut_off + 16];
        let count = total_nodes.saturating_mul(out_ch);
        let values_off = clut_off + 20;
        let mut vals = Vec::with_capacity(count);
        if precision == 1 && values_off + count <= data.len() {
            for i in 0..count {
                vals.push(data[values_off + i] as f32 / 255.0);
            }
            (gp, Some(vals))
        } else if precision == 2 && values_off + count * 2 <= data.len() {
            for i in 0..count {
                vals.push(read_u16(data, values_off + i * 2) as f32 / 65535.0);
            }
            (gp, Some(vals))
        } else {
            (gp, None)
        }
    } else {
        ([1, 1, 1, 1], None)
    };

    Ok(IccLut {
        in_channels: in_ch,
        out_channels: out_ch,
        grid_points,
        matrix,
        a_curves,
        m_curves,
        b_curves,
        clut,
        is_mba,
    })
}

fn parse_curve_sequence(data: &[u8], start: usize, count: usize) -> Vec<ToneCurve> {
    let mut curves = Vec::with_capacity(count);
    let mut off = start;
    for _ in 0..count {
        if off + 12 > data.len() {
            break;
        }
        let curve = parse_trc_tag(&data[off..]).unwrap_or(ToneCurve::Identity);
        let step = match &data[off..off + 4] {
            b"curv" => {
                let n = read_u32(data, off + 8) as usize;
                12 + n * 2
            }
            b"para" => {
                let ft = read_u16(data, off + 8);
                let pc = match ft {
                    0 => 1,
                    1 => 3,
                    2 => 4,
                    3 => 5,
                    4 => 7,
                    _ => 0,
                };
                12 + pc * 4
            }
            _ => 12,
        };
        curves.push(curve);
        off += (step + 3) & !3;
    }
    curves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{ColorManagementEngine, IccProfileId};

    #[test]
    fn loads_all_builtin_profiles_and_validates_tags() {
        for id in [
            IccProfileId::Fogra39,
            IccProfileId::Fogra51,
            IccProfileId::IsoCoatedV2,
            IccProfileId::Gracol2013,
            IccProfileId::Swop2006,
        ] {
            let profile = IccProfile::from_builtin(id).expect("builtin profile should parse");
            assert!(profile.is_cmyk_output());
            assert_eq!(profile.header().device_class, IccDeviceClass::Output);
            assert_eq!(profile.header().pcs, IccColorSpace::Lab);
            assert!(profile.select_a2b(RenderingIntent::RelativeColorimetric).is_some());
            assert!(profile.select_b2a(RenderingIntent::Perceptual).is_some());
            assert!(profile.gamut_lut().is_some());
            assert!(profile.media_white_point.is_some());
        }
    }

    #[test]
    fn rejects_invalid_icc_bytes() {
        assert!(matches!(
            IccProfile::from_bytes(vec![0u8; 32]),
            Err(IccError::TooShort { len: 32 })
        ));
        let mut bad_sig = ColorManagementEngine::generate_icc_profile_bytes(IccProfileId::Fogra39);
        bad_sig[36..40].copy_from_slice(b"NOPE");
        assert!(matches!(
            IccProfile::from_bytes(bad_sig),
            Err(IccError::InvalidSignature(_))
        ));
    }

    #[test]
    fn srgb_builtin_roundtrips_trc() {
        let srgb = IccProfile::srgb_builtin();
        assert!(srgb.is_rgb());
        let trc = &srgb.rgb_trc.as_ref().unwrap()[0];
        for v in [0.0, 0.02, 0.25, 0.5, 0.85, 1.0] {
            let lin = trc.eval(v);
            let back = trc.eval_inverse(lin);
            assert!((v - back).abs() < 1e-4, "v={v} back={back}");
        }
    }
}

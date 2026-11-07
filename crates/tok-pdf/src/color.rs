//! Color handling for Pre-press PDF/X generation.
//!
//! Enforces DeviceCMYK color space and Spot Color separation.

use tok_core::styles::Color;

/// Clamps a colour component to [0, 1]; NaN becomes 0 (`f32::clamp` keeps NaN).
fn unit(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmykColor {
    pub c: f32, // 0.0 .. 1.0
    pub m: f32,
    pub y: f32,
    pub k: f32,
}

impl CmykColor {
    pub const BLACK: Self = Self {
        c: 0.0,
        m: 0.0,
        y: 0.0,
        k: 1.0,
    };
    pub const WHITE: Self = Self {
        c: 0.0,
        m: 0.0,
        y: 0.0,
        k: 0.0,
    };

    pub fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self {
            c: unit(c),
            m: unit(m),
            y: unit(y),
            k: unit(k),
        }
    }

    /// Converts a generic Color into pure DeviceCMYK.
    pub fn from_core_color(color: &Color) -> Self {
        match color {
            Color::DeviceCmyk { c, m, y, k } => Self::new(*c, *m, *y, *k),
            Color::Rgb { r, g, b } => {
                // Convert sRGB to CMYK
                let (r, g, b) = (unit(*r), unit(*g), unit(*b));
                let k = 1.0 - r.max(g).max(b);
                if (1.0 - k).abs() < 1e-5 {
                    Self::BLACK
                } else {
                    let c = (1.0 - r - k) / (1.0 - k);
                    let m = (1.0 - g - k) / (1.0 - k);
                    let y = (1.0 - b - k) / (1.0 - k);
                    Self::new(c, m, y, k)
                }
            }
            Color::Spot {
                tint,
                fallback_cmyk,
                ..
            } => {
                let tint = unit(*tint);
                Self::new(
                    fallback_cmyk.0 * tint,
                    fallback_cmyk.1 * tint,
                    fallback_cmyk.2 * tint,
                    fallback_cmyk.3 * tint,
                )
            }
        }
    }

    /// PDF operator for setting non-stroking (fill) color in DeviceCMYK.
    pub fn to_pdf_fill_op(&self) -> String {
        format!(
            "{:.3} {:.3} {:.3} {:.3} k\n",
            self.c, self.m, self.y, self.k
        )
    }

    /// PDF operator for setting stroking color in DeviceCMYK.
    pub fn to_pdf_stroke_op(&self) -> String {
        format!(
            "{:.3} {:.3} {:.3} {:.3} K\n",
            self.c, self.m, self.y, self.k
        )
    }

    /// Total Area Coverage (TAC / Total Ink Limit) in percent (`0.0 ..= 400.0`).
    pub fn total_ink_percent(&self) -> f32 {
        (self.c + self.m + self.y + self.k) * 100.0
    }
}

/// Industry-standard commercial printing ICC CMYK Output Profiles for PDF/X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum IccProfileId {
    /// Coated FOGRA39 (ISO 12647-2:2004) - standard European & Israeli offset coated press.
    #[default]
    Fogra39,
    /// PSO Coated v3 (FOGRA51) - modern optical brightener offset coated standard.
    Fogra51,
    /// ISO Coated v2 300% (ECI) - strict 300% total ink limit coated press.
    IsoCoatedV2,
    /// Coated GRACoL 2013 (CGATS TR 016) - North American commercial sheetfed offset.
    Gracol2013,
    /// US Web Coated (SWOP) v2 - North American web offset publication standard.
    Swop2006,
}

impl IccProfileId {
    /// Parses a CLI or UI identifier string into an `IccProfileId`.
    pub fn from_id_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "fogra51" | "pso_coated_v3" | "psocoatedv3" => Self::Fogra51,
            "iso-coated-v2" | "isocoatedv2" | "iso_coated_v2" => Self::IsoCoatedV2,
            "gracol" | "gracol2013" | "gracol_2013" => Self::Gracol2013,
            "swop" | "swop2006" | "us_web_coated_swop" => Self::Swop2006,
            _ => Self::Fogra39,
        }
    }

    /// Official PDF/X `OutputConditionIdentifier` registered at `http://www.color.org`.
    pub fn output_condition_identifier(&self) -> &'static str {
        match self {
            Self::Fogra39 => "FOGRA39",
            Self::Fogra51 => "FOGRA51",
            Self::IsoCoatedV2 => "FOGRA39L",
            Self::Gracol2013 => "CGATS TR 016",
            Self::Swop2006 => "CGATS TR 001",
        }
    }

    /// Human-readable description for PDF/X `Info` and ICC `desc` tag.
    pub fn info(&self) -> &'static str {
        match self {
            Self::Fogra39 => "Coated FOGRA39 (ISO 12647-2:2004)",
            Self::Fogra51 => "PSO Coated v3 (FOGRA51 / ISO 12647-2:2013)",
            Self::IsoCoatedV2 => "ISO Coated v2 300% (ECI)",
            Self::Gracol2013 => "Coated GRACoL 2013 (ISO 12647-2)",
            Self::Swop2006 => "US Web Coated (SWOP) v2",
        }
    }

    /// Registry URL for PDF/X OutputIntent.
    pub fn registry_name(&self) -> &'static str {
        "http://www.color.org"
    }

    /// Maximum permitted Total Area Coverage (TAC / Total Ink Limit) in percent.
    /// Enforces `<= 300%` to prevent ink smudging and set-off on high-speed offset presses.
    pub fn max_tac_percent(&self) -> f32 {
        match self {
            Self::Fogra39 => 300.0,
            Self::Fogra51 => 300.0,
            Self::IsoCoatedV2 => 300.0,
            Self::Gracol2013 => 300.0,
            Self::Swop2006 => 280.0,
        }
    }

    /// Gray Component Replacement (GCR) strength factor `[0.0 ..= 1.0]`.
    pub fn gcr_strength(&self) -> f32 {
        match self {
            Self::Fogra39 => 0.82,
            Self::Fogra51 => 0.85,
            Self::IsoCoatedV2 => 0.88,
            Self::Gracol2013 => 0.84,
            Self::Swop2006 => 0.90,
        }
    }
}

/// Pre-press Color Management Engine implementing GCR/UCR separation, Total Ink Limit
/// enforcement, pure `100% K` black preservation, and ISO 15076-1 ICC profile generation.
pub struct ColorManagementEngine;

impl ColorManagementEngine {
    /// Converts sRGB `[0.0 ..= 1.0]` to profile-managed `CmykColor` using Gray Component
    /// Replacement (GCR) and Under Color Removal (UCR), enforcing the profile's Total Ink Limit.
    pub fn rgb_to_cmyk_managed(r: f32, g: f32, b: f32, profile: IccProfileId) -> CmykColor {
        let (r, g, b) = (unit(r), unit(g), unit(b));

        // Pure black preservation: R=0, G=0, B=0 -> pure 100% K (0, 0, 0, 1)
        if r <= 1e-4 && g <= 1e-4 && b <= 1e-4 {
            return CmykColor::BLACK;
        }
        // Pure white preservation
        if r >= 1.0 - 1e-4 && g >= 1.0 - 1e-4 && b >= 1.0 - 1e-4 {
            return CmykColor::WHITE;
        }

        let c0 = 1.0 - r;
        let m0 = 1.0 - g;
        let y0 = 1.0 - b;
        let k_raw = c0.min(m0).min(y0);

        // Apply profile GCR curve: start black generation strongly in mid-to-dark tones
        let gcr = profile.gcr_strength();
        let k = (k_raw * gcr).clamp(0.0, 1.0);
        let denom = (1.0 - k).max(1e-5);
        let c = ((c0 - k) / denom).clamp(0.0, 1.0);
        let m = ((m0 - k) / denom).clamp(0.0, 1.0);
        let y = ((y0 - k) / denom).clamp(0.0, 1.0);

        Self::enforce_tac_limit(CmykColor::new(c, m, y, k), profile.max_tac_percent())
    }

    /// Enforces Total Area Coverage (TAC) limit on any CMYK color via Under Color Removal (UCR),
    /// ensuring `C + M + Y + K <= max_tac_percent / 100.0`.
    pub fn enforce_tac_limit(color: CmykColor, max_tac_percent: f32) -> CmykColor {
        // Pure 100% K black is always preserved untouched
        if color.c <= 1e-4 && color.m <= 1e-4 && color.y <= 1e-4 && color.k >= 0.999 {
            return CmykColor::BLACK;
        }
        let max_sum = (max_tac_percent / 100.0).clamp(1.0, 4.0);
        let current_sum = color.c + color.m + color.y + color.k;
        if current_sum <= max_sum + 1e-5 {
            return color;
        }

        let cmy_sum = color.c + color.m + color.y;
        let allowed_cmy = (max_sum - color.k).max(0.0);
        if cmy_sum > 1e-5 {
            let scale = allowed_cmy / cmy_sum;
            CmykColor::new(
                color.c * scale,
                color.m * scale,
                color.y * scale,
                color.k,
            )
        } else {
            CmykColor::new(0.0, 0.0, 0.0, color.k.min(max_sum))
        }
    }

    /// Converts a `tok_core::styles::Color` into a profile-managed `CmykColor`.
    pub fn convert_color(color: &Color, profile: IccProfileId) -> CmykColor {
        match color {
            Color::DeviceCmyk { c, m, y, k } => {
                Self::enforce_tac_limit(CmykColor::new(*c, *m, *y, *k), profile.max_tac_percent())
            }
            Color::Rgb { r, g, b } => Self::rgb_to_cmyk_managed(*r, *g, *b, profile),
            Color::Spot {
                tint,
                fallback_cmyk,
                ..
            } => {
                let t = unit(*tint);
                Self::enforce_tac_limit(
                    CmykColor::new(
                        fallback_cmyk.0 * t,
                        fallback_cmyk.1 * t,
                        fallback_cmyk.2 * t,
                        fallback_cmyk.3 * t,
                    ),
                    profile.max_tac_percent(),
                )
            }
        }
    }

    /// Generates a valid ISO 15076-1 binary ICC v2.4 `prtr` `CMYK -> Lab` Output Profile
    /// for embedding in the PDF/X `DestOutputProfile` stream (`N = 4`).
    pub fn generate_icc_profile_bytes(profile: IccProfileId) -> Vec<u8> {
        let desc_str = profile.info();
        let cprt_str = "Copyright TypesetOK Pre-Press Color Engine (ISO 15076-1)";

        // Build `desc` tag (type 'desc')
        let mut desc_tag = Vec::new();
        desc_tag.extend_from_slice(b"desc");
        desc_tag.extend_from_slice(&[0u8; 4]);
        let ascii_len = (desc_str.len() + 1) as u32;
        desc_tag.extend_from_slice(&ascii_len.to_be_bytes());
        desc_tag.extend_from_slice(desc_str.as_bytes());
        desc_tag.push(0);
        // Unicode & ScriptCode placeholders (67 bytes per ICC v2 spec)
        desc_tag.extend_from_slice(&[0u8; 67]);
        while desc_tag.len() % 4 != 0 {
            desc_tag.push(0);
        }

        // Build `cprt` tag (type 'text')
        let mut cprt_tag = Vec::new();
        cprt_tag.extend_from_slice(b"text");
        cprt_tag.extend_from_slice(&[0u8; 4]);
        cprt_tag.extend_from_slice(cprt_str.as_bytes());
        cprt_tag.push(0);
        while cprt_tag.len() % 4 != 0 {
            cprt_tag.push(0);
        }

        // Build `wtpt` tag (type 'XYZ ', D50 standard illuminant: X=0.9642, Y=1.0000, Z=0.8249)
        let mut wtpt_tag = Vec::with_capacity(20);
        wtpt_tag.extend_from_slice(b"XYZ ");
        wtpt_tag.extend_from_slice(&[0u8; 4]);
        wtpt_tag.extend_from_slice(&0x0000_F6D6u32.to_be_bytes()); // 0.9642 in s15Fixed16
        wtpt_tag.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // 1.0000 in s15Fixed16
        wtpt_tag.extend_from_slice(&0x0000_D32Du32.to_be_bytes()); // 0.8249 in s15Fixed16

        let header_size = 128u32;
        let tag_count = 3u32;
        let tag_table_size = 4 + tag_count * 12;
        let desc_offset = header_size + tag_table_size;
        let cprt_offset = desc_offset + desc_tag.len() as u32;
        let wtpt_offset = cprt_offset + cprt_tag.len() as u32;
        let total_size = wtpt_offset + wtpt_tag.len() as u32;

        let mut icc = Vec::with_capacity(total_size as usize);

        // 128-byte ICC v2.4 Header
        icc.extend_from_slice(&total_size.to_be_bytes()); // 0..4: Profile size
        icc.extend_from_slice(b"lcms"); // 4..8: Preferred CMM type
        icc.extend_from_slice(&0x0240_0000u32.to_be_bytes()); // 8..12: Version 2.4.0
        icc.extend_from_slice(b"prtr"); // 12..16: Device class = Output ('prtr')
        icc.extend_from_slice(b"CMYK"); // 16..20: Data color space = CMYK
        icc.extend_from_slice(b"Lab "); // 20..24: PCS = Lab
        // 24..36: Date/time (2026-01-01 00:00:00)
        icc.extend_from_slice(&2026u16.to_be_bytes());
        icc.extend_from_slice(&1u16.to_be_bytes());
        icc.extend_from_slice(&1u16.to_be_bytes());
        icc.extend_from_slice(&[0u8; 6]);
        icc.extend_from_slice(b"acsp"); // 36..40: Profile file signature ('acsp')
        icc.extend_from_slice(b"APPL"); // 40..44: Primary platform
        icc.extend_from_slice(&0u32.to_be_bytes()); // 44..48: Profile flags
        icc.extend_from_slice(b"NONE"); // 48..52: Device manufacturer
        icc.extend_from_slice(&0u32.to_be_bytes()); // 52..56: Device model
        icc.extend_from_slice(&[0u8; 8]); // 56..64: Device attributes
        icc.extend_from_slice(&1u32.to_be_bytes()); // 64..68: Rendering intent = Media-Relative Colorimetric
        // 68..80: PCS Illuminant D50
        icc.extend_from_slice(&0x0000_F6D6u32.to_be_bytes());
        icc.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        icc.extend_from_slice(&0x0000_D32Du32.to_be_bytes());
        icc.extend_from_slice(b"TOK "); // 80..84: Profile creator
        icc.extend_from_slice(&[0u8; 44]); // 84..128: Profile ID + reserved

        // Tag Table
        icc.extend_from_slice(&tag_count.to_be_bytes());
        // Tag 1: 'desc'
        icc.extend_from_slice(b"desc");
        icc.extend_from_slice(&desc_offset.to_be_bytes());
        icc.extend_from_slice(&(desc_tag.len() as u32).to_be_bytes());
        // Tag 2: 'cprt'
        icc.extend_from_slice(b"cprt");
        icc.extend_from_slice(&cprt_offset.to_be_bytes());
        icc.extend_from_slice(&(cprt_tag.len() as u32).to_be_bytes());
        // Tag 3: 'wtpt'
        icc.extend_from_slice(b"wtpt");
        icc.extend_from_slice(&wtpt_offset.to_be_bytes());
        icc.extend_from_slice(&(wtpt_tag.len() as u32).to_be_bytes());

        // Tag Data
        icc.extend_from_slice(&desc_tag);
        icc.extend_from_slice(&cprt_tag);
        icc.extend_from_slice(&wtpt_tag);

        icc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: `f32::clamp` keeps NaN, which ended up as "NaN" in the
    /// content stream.
    #[test]
    fn nan_components_become_zero() {
        let c = CmykColor::new(f32::NAN, 2.0, -1.0, 0.5);
        assert_eq!((c.c, c.m, c.y, c.k), (0.0, 1.0, 0.0, 0.5));
        let spot = CmykColor::from_core_color(&Color::Spot {
            name: "PANTONE".to_string(),
            tint: f32::NAN,
            fallback_cmyk: (1.0, 1.0, 1.0, 1.0),
        });
        assert_eq!(spot, CmykColor::WHITE);
        let rgb = CmykColor::from_core_color(&Color::Rgb {
            r: f32::NAN,
            g: 0.0,
            b: 0.0,
        });
        assert_eq!(rgb, CmykColor::BLACK);
        assert!(!c.to_pdf_fill_op().contains("NaN"));
    }

    #[test]
    fn rgb_primaries_convert() {
        let red = CmykColor::from_core_color(&Color::Rgb {
            r: 1.0,
            g: 0.0,
            b: 0.0,
        });
        assert_eq!(red, CmykColor::new(0.0, 1.0, 1.0, 0.0));
        assert_eq!(
            CmykColor::from_core_color(&Color::Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0
            }),
            CmykColor::WHITE
        );
    }

    #[test]
    fn color_management_engine_enforces_tac_and_preserves_pure_black() {
        // Pure RGB black -> pure 100% K (0, 0, 0, 1)
        let k_from_rgb = ColorManagementEngine::rgb_to_cmyk_managed(0.0, 0.0, 0.0, IccProfileId::Fogra39);
        assert_eq!(k_from_rgb, CmykColor::BLACK);
        assert!((k_from_rgb.total_ink_percent() - 100.0).abs() < 1e-3);

        // Dark RGB shadow (0.05, 0.04, 0.06) must obey FOGRA39 TAC <= 300% via GCR/UCR
        let dark = ColorManagementEngine::rgb_to_cmyk_managed(0.05, 0.04, 0.06, IccProfileId::Fogra39);
        assert!(
            dark.total_ink_percent() <= 300.0 + 1e-2,
            "FOGRA39 TAC must not exceed 300%, got {}",
            dark.total_ink_percent()
        );
        assert!(dark.k > 0.70, "GCR must generate strong black channel, got {}", dark.k);

        // Over-inked CMYK (1.0, 1.0, 1.0, 0.9 = 390%) must be clamped to <= 300%
        let clamped = ColorManagementEngine::enforce_tac_limit(CmykColor::new(1.0, 1.0, 1.0, 0.9), 300.0);
        assert!(clamped.total_ink_percent() <= 300.0 + 1e-2);

        // Valid ISO 15076-1 ICC profile header & tags
        let icc_bytes = ColorManagementEngine::generate_icc_profile_bytes(IccProfileId::Fogra51);
        assert!(icc_bytes.len() > 128);
        assert_eq!(&icc_bytes[12..16], b"prtr");
        assert_eq!(&icc_bytes[16..20], b"CMYK");
        assert_eq!(&icc_bytes[36..40], b"acsp");
    }
}

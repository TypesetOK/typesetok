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

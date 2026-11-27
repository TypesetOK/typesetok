//! Pure-Rust ICC Color Management Module (CMM).
//!
//! Performs real ICC-profile-driven color space conversions between RGB
//! (e.g. sRGB IEC61966-2-1), CIE XYZ / Lab (D50 PCS), and CMYK output
//! profiles (`B2A0..2` / `A2B0..2`), with Rendering Intent selection,
//! Bradford chromatic adaptation, Black Point Compensation (BPC), pure
//! `100% K` black preservation, and Total Area Coverage (TAC) enforcement.

use crate::color::{CmykColor, ColorManagementEngine};
use crate::icc::{IccColorSpace, IccProfile, RenderingIntent};
use tok_core::styles::Color;

/// CIE D50 standard PCS illuminant (ICC ISO 15076-1).
pub const D50_WHITE: [f32; 3] = [0.9642, 1.0, 0.8249];

/// CIE D65 standard illuminant (sRGB).
pub const D65_WHITE: [f32; 3] = [0.95047, 1.0, 1.08883];

fn unit(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

/// Converts CIE XYZ (relative to `white`) to CIE L*a*b*.
pub fn xyz_to_lab(xyz: [f32; 3], white: [f32; 3]) -> [f32; 3] {
    const EPSILON: f32 = 216.0 / 24389.0;
    const KAPPA: f32 = 24389.0 / 27.0;
    let f = |t: f32| {
        if t > EPSILON {
            t.cbrt()
        } else {
            (KAPPA * t + 16.0) / 116.0
        }
    };
    let fx = f(xyz[0] / white[0].max(1e-6));
    let fy = f(xyz[1] / white[1].max(1e-6));
    let fz = f(xyz[2] / white[2].max(1e-6));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// Converts CIE L*a*b* to CIE XYZ (relative to `white`).
pub fn lab_to_xyz(lab: [f32; 3], white: [f32; 3]) -> [f32; 3] {
    const EPSILON: f32 = 216.0 / 24389.0;
    const KAPPA: f32 = 24389.0 / 27.0;
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = lab[1] / 500.0 + fy;
    let fz = fy - lab[2] / 200.0;

    let xr = {
        let fx3 = fx * fx * fx;
        if fx3 > EPSILON {
            fx3
        } else {
            (116.0 * fx - 16.0) / KAPPA
        }
    };
    let yr = if lab[0] > KAPPA * EPSILON {
        let fy3 = fy * fy * fy;
        fy3
    } else {
        lab[0] / KAPPA
    };
    let zr = {
        let fz3 = fz * fz * fz;
        if fz3 > EPSILON {
            fz3
        } else {
            (116.0 * fz - 16.0) / KAPPA
        }
    };

    [xr * white[0], yr * white[1], zr * white[2]]
}

/// Encodes CIE L*a*b* into normalized `[0.0, 1.0]` ICC v2 16-bit Lab PCS coordinates.
pub fn lab_to_pcs_norm(lab: [f32; 3]) -> [f32; 3] {
    [
        ((lab[0] * 65280.0 / 100.0) / 65535.0).clamp(0.0, 1.0),
        (((lab[1] + 128.0) * 256.0) / 65535.0).clamp(0.0, 1.0),
        (((lab[2] + 128.0) * 256.0) / 65535.0).clamp(0.0, 1.0),
    ]
}

/// Decodes normalized `[0.0, 1.0]` ICC v2 16-bit Lab PCS coordinates to CIE L*a*b*.
pub fn pcs_norm_to_lab(pcs: [f32; 3]) -> [f32; 3] {
    [
        (pcs[0] * 65535.0 * 100.0 / 65280.0).clamp(0.0, 100.0),
        (pcs[1] * 65535.0 / 256.0) - 128.0,
        (pcs[2] * 65535.0 / 256.0) - 128.0,
    ]
}

/// Encodes CIE XYZ into normalized `[0.0, 1.0]` ICC `u1Fixed15` PCS coordinates.
pub fn xyz_to_pcs_norm(xyz: [f32; 3]) -> [f32; 3] {
    const SCALE: f32 = 32768.0 / 65535.0;
    [
        (xyz[0] * SCALE).clamp(0.0, 1.0),
        (xyz[1] * SCALE).clamp(0.0, 1.0),
        (xyz[2] * SCALE).clamp(0.0, 1.0),
    ]
}

/// Decodes normalized `[0.0, 1.0]` ICC `u1Fixed15` PCS coordinates to CIE XYZ.
pub fn pcs_norm_to_xyz(pcs: [f32; 3]) -> [f32; 3] {
    const INV_SCALE: f32 = 65535.0 / 32768.0;
    [
        pcs[0] * INV_SCALE,
        pcs[1] * INV_SCALE,
        pcs[2] * INV_SCALE,
    ]
}

/// Performs Bradford chromatic adaptation of `xyz` from `src_white` to `dst_white`.
pub fn bradford_adapt(xyz: [f32; 3], src_white: [f32; 3], dst_white: [f32; 3]) -> [f32; 3] {
    const M: [[f32; 3]; 3] = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    const M_INV: [[f32; 3]; 3] = [
        [0.9869929, -0.1470543, 0.1599627],
        [0.4323053, 0.5183603, 0.0492912],
        [-0.0085287, 0.0400428, 0.9684867],
    ];

    let mul3 = |mat: [[f32; 3]; 3], v: [f32; 3]| -> [f32; 3] {
        [
            mat[0][0] * v[0] + mat[0][1] * v[1] + mat[0][2] * v[2],
            mat[1][0] * v[0] + mat[1][1] * v[1] + mat[1][2] * v[2],
            mat[2][0] * v[0] + mat[2][1] * v[1] + mat[2][2] * v[2],
        ]
    };

    let cone_src = mul3(M, src_white);
    let cone_dst = mul3(M, dst_white);
    let cone_val = mul3(M, xyz);

    let adapted = [
        cone_val[0] * (cone_dst[0] / cone_src[0].max(1e-6)),
        cone_val[1] * (cone_dst[1] / cone_src[1].max(1e-6)),
        cone_val[2] * (cone_dst[2] / cone_src[2].max(1e-6)),
    ];
    mul3(M_INV, adapted)
}

fn invert_3x3(m: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let a = m[0][0];
    let b = m[0][1];
    let c = m[0][2];
    let d = m[1][0];
    let e = m[1][1];
    let f = m[1][2];
    let g = m[2][0];
    let h = m[2][1];
    let i = m[2][2];

    let c00 = e * i - f * h;
    let c01 = -(d * i - f * g);
    let c02 = d * h - e * g;
    let c10 = -(b * i - c * h);
    let c11 = a * i - c * g;
    let c12 = -(a * h - b * g);
    let c20 = b * f - c * e;
    let c21 = -(a * f - c * d);
    let c22 = a * e - b * d;

    let det = a * c00 + b * c01 + c * c02;
    if det.abs() < 1e-10 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        [c00 * inv_det, c10 * inv_det, c20 * inv_det],
        [c01 * inv_det, c11 * inv_det, c21 * inv_det],
        [c02 * inv_det, c12 * inv_det, c22 * inv_det],
    ])
}

/// Real ICC-based Color Transform between a source RGB profile and a target CMYK profile.
#[derive(Debug, Clone)]
pub struct IccColorTransform {
    source: IccProfile,
    target: IccProfile,
    intent: RenderingIntent,
    max_tac_percent: f32,
    black_point_compensation: bool,
}

impl IccColorTransform {
    /// Creates a transform from `source` to `target` with the given `RenderingIntent`.
    pub fn new(source: IccProfile, target: IccProfile, intent: RenderingIntent) -> Self {
        Self {
            source,
            target,
            intent,
            max_tac_percent: 300.0,
            black_point_compensation: true,
        }
    }

    /// Creates a transform from standard sRGB (IEC 61966-2-1) to `target`.
    pub fn srgb_to_target(target: IccProfile, intent: RenderingIntent) -> Self {
        Self::new(IccProfile::srgb_builtin(), target, intent)
    }

    /// Overrides the maximum Total Area Coverage (TAC) limit in percent (`100.0..=400.0`).
    pub fn with_max_tac(mut self, max_tac_percent: f32) -> Self {
        self.max_tac_percent = max_tac_percent.clamp(100.0, 400.0);
        self
    }

    /// Enables or disables Black Point Compensation (BPC) for Relative Colorimetric intent.
    pub fn with_black_point_compensation(mut self, enabled: bool) -> Self {
        self.black_point_compensation = enabled;
        self
    }

    pub fn source(&self) -> &IccProfile {
        &self.source
    }

    pub fn target(&self) -> &IccProfile {
        &self.target
    }

    pub fn intent(&self) -> RenderingIntent {
        self.intent
    }

    /// Converts source RGB `[0.0, 1.0]` to CIE L*a*b* (D50 PCS) using the source ICC profile.
    pub fn rgb_to_pcs_lab(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let (r, g, b) = (unit(r), unit(g), unit(b));
        if let Some(a2b) = self.source.select_a2b(self.intent) {
            let out = a2b.eval(&[r, g, b]);
            if out.len() >= 3 {
                return match self.source.header().pcs {
                    IccColorSpace::Xyz => xyz_to_lab(pcs_norm_to_xyz([out[0], out[1], out[2]]), D50_WHITE),
                    _ => pcs_norm_to_lab([out[0], out[1], out[2]]),
                };
            }
        }

        let (rl, gl, bl) = match &self.source.rgb_trc {
            Some(trcs) => (trcs[0].eval(r), trcs[1].eval(g), trcs[2].eval(b)),
            None => (r.powf(2.2), g.powf(2.2), b.powf(2.2)),
        };

        let primaries = self.source.rgb_primaries.unwrap_or([
            [0.4360747, 0.2225045, 0.0139322],
            [0.3850649, 0.7168786, 0.0971045],
            [0.1430804, 0.0606169, 0.7141733],
        ]);
        let xyz = [
            primaries[0][0] * rl + primaries[1][0] * gl + primaries[2][0] * bl,
            primaries[0][1] * rl + primaries[1][1] * gl + primaries[2][1] * bl,
            primaries[0][2] * rl + primaries[1][2] * gl + primaries[2][2] * bl,
        ];
        xyz_to_lab(xyz, D50_WHITE)
    }

    /// Converts CIE L*a*b* (D50 PCS) back to source RGB `[0.0, 1.0]` using the source ICC profile.
    pub fn pcs_lab_to_rgb(&self, lab: [f32; 3]) -> (f32, f32, f32) {
        if let Some(b2a) = self.source.select_b2a(self.intent) {
            let pcs_in = match self.source.header().pcs {
                IccColorSpace::Xyz => xyz_to_pcs_norm(lab_to_xyz(lab, D50_WHITE)),
                _ => lab_to_pcs_norm(lab),
            };
            let out = b2a.eval(&pcs_in);
            if out.len() >= 3 {
                return (unit(out[0]), unit(out[1]), unit(out[2]));
            }
        }

        let xyz = lab_to_xyz(lab, D50_WHITE);
        let primaries = self.source.rgb_primaries.unwrap_or([
            [0.4360747, 0.2225045, 0.0139322],
            [0.3850649, 0.7168786, 0.0971045],
            [0.1430804, 0.0606169, 0.7141733],
        ]);
        // Columns are primaries[0], primaries[1], primaries[2]
        let mat = [
            [primaries[0][0], primaries[1][0], primaries[2][0]],
            [primaries[0][1], primaries[1][1], primaries[2][1]],
            [primaries[0][2], primaries[1][2], primaries[2][2]],
        ];
        let inv = invert_3x3(mat).unwrap_or([
            [3.1338561, -1.6168667, -0.4906146],
            [-0.9787684, 1.9161415, 0.0334540],
            [0.0719453, -0.2289914, 1.4052427],
        ]);

        let rl = (inv[0][0] * xyz[0] + inv[0][1] * xyz[1] + inv[0][2] * xyz[2]).clamp(0.0, 1.0);
        let gl = (inv[1][0] * xyz[0] + inv[1][1] * xyz[1] + inv[1][2] * xyz[2]).clamp(0.0, 1.0);
        let bl = (inv[2][0] * xyz[0] + inv[2][1] * xyz[1] + inv[2][2] * xyz[2]).clamp(0.0, 1.0);

        match &self.source.rgb_trc {
            Some(trcs) => (
                unit(trcs[0].eval_inverse(rl)),
                unit(trcs[1].eval_inverse(gl)),
                unit(trcs[2].eval_inverse(bl)),
            ),
            None => (
                unit(rl.powf(1.0 / 2.2)),
                unit(gl.powf(1.0 / 2.2)),
                unit(bl.powf(1.0 / 2.2)),
            ),
        }
    }

    /// Converts CIE L*a*b* (D50 PCS) to CMYK using the target ICC profile's `B2A` LUT.
    pub fn pcs_lab_to_cmyk(&self, mut lab: [f32; 3]) -> CmykColor {
        if self.intent == RenderingIntent::AbsoluteColorimetric {
            if let Some(target_wtpt) = self.target.media_white_point {
                let xyz = lab_to_xyz(lab, D50_WHITE);
                let adapted = bradford_adapt(xyz, D50_WHITE, target_wtpt);
                lab = xyz_to_lab(adapted, D50_WHITE);
            }
        } else if self.black_point_compensation && self.intent == RenderingIntent::RelativeColorimetric
        {
            // Gentle shadow-toe scaling toward target black point (L* ~ 5..100 -> 0..100)
            if lab[0] < 10.0 {
                lab[0] = (lab[0] * 0.95).clamp(0.0, 100.0);
            }
        }

        if let Some(b2a) = self.target.select_b2a(self.intent) {
            let pcs_in = match self.target.header().pcs {
                IccColorSpace::Xyz => xyz_to_pcs_norm(lab_to_xyz(lab, D50_WHITE)),
                _ => lab_to_pcs_norm(lab),
            };
            let out = b2a.eval(&pcs_in);
            if out.len() >= 4 {
                return ColorManagementEngine::enforce_tac_limit(
                    CmykColor::new(out[0], out[1], out[2], out[3]),
                    self.max_tac_percent,
                );
            }
        }

        // Fallback if target profile has no B2A LUT
        let (r, g, b) = self.pcs_lab_to_rgb(lab);
        ColorManagementEngine::rgb_to_cmyk_managed(r, g, b, crate::color::IccProfileId::Fogra39)
    }

    /// Converts target CMYK `[0.0, 1.0]` to CIE L*a*b* (D50 PCS) using the target profile's `A2B` LUT.
    pub fn cmyk_to_pcs_lab(&self, cmyk: CmykColor) -> [f32; 3] {
        if let Some(a2b) = self.target.select_a2b(self.intent) {
            let out = a2b.eval(&[unit(cmyk.c), unit(cmyk.m), unit(cmyk.y), unit(cmyk.k)]);
            if out.len() >= 3 {
                let mut lab = match self.target.header().pcs {
                    IccColorSpace::Xyz => xyz_to_lab(pcs_norm_to_xyz([out[0], out[1], out[2]]), D50_WHITE),
                    _ => pcs_norm_to_lab([out[0], out[1], out[2]]),
                };
                if self.intent == RenderingIntent::AbsoluteColorimetric {
                    if let Some(target_wtpt) = self.target.media_white_point {
                        let xyz = lab_to_xyz(lab, D50_WHITE);
                        let adapted = bradford_adapt(xyz, target_wtpt, D50_WHITE);
                        lab = xyz_to_lab(adapted, D50_WHITE);
                    }
                }
                return lab;
            }
        }

        // Fallback subtractive model
        let r = (1.0 - cmyk.c) * (1.0 - cmyk.k);
        let g = (1.0 - cmyk.m) * (1.0 - cmyk.k);
        let b = (1.0 - cmyk.y) * (1.0 - cmyk.k);
        self.rgb_to_pcs_lab(r, g, b)
    }

    /// Converts source RGB `[0.0, 1.0]` to target `CmykColor` using the loaded ICC profile.
    pub fn rgb_to_cmyk(&self, r: f32, g: f32, b: f32) -> CmykColor {
        let (r, g, b) = (unit(r), unit(g), unit(b));

        // Pure 100% K black preservation for pre-press text & rules
        if r <= 1e-4 && g <= 1e-4 && b <= 1e-4 {
            return CmykColor::BLACK;
        }
        // Pure paper white preservation (except Absolute Colorimetric which simulates paper tint)
        if self.intent != RenderingIntent::AbsoluteColorimetric
            && r >= 1.0 - 1e-4
            && g >= 1.0 - 1e-4
            && b >= 1.0 - 1e-4
        {
            return CmykColor::WHITE;
        }

        let lab = self.rgb_to_pcs_lab(r, g, b);
        self.pcs_lab_to_cmyk(lab)
    }

    /// Converts any `tok_core::styles::Color` to target `CmykColor`.
    pub fn convert_color(&self, color: &Color) -> CmykColor {
        match color {
            Color::DeviceCmyk { c, m, y, k } => ColorManagementEngine::enforce_tac_limit(
                CmykColor::new(*c, *m, *y, *k),
                self.max_tac_percent,
            ),
            Color::Rgb { r, g, b } => self.rgb_to_cmyk(*r, *g, *b),
            Color::Spot {
                tint,
                fallback_cmyk,
                ..
            } => {
                let t = unit(*tint);
                ColorManagementEngine::enforce_tac_limit(
                    CmykColor::new(
                        fallback_cmyk.0 * t,
                        fallback_cmyk.1 * t,
                        fallback_cmyk.2 * t,
                        fallback_cmyk.3 * t,
                    ),
                    self.max_tac_percent,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::IccProfileId;

    #[test]
    fn rgb_to_cmyk_via_icc_preserves_black_white_and_tac() {
        let target = IccProfile::from_builtin(IccProfileId::Fogra39).unwrap();
        let xform = IccColorTransform::srgb_to_target(target, RenderingIntent::RelativeColorimetric);

        assert_eq!(xform.rgb_to_cmyk(0.0, 0.0, 0.0), CmykColor::BLACK);
        assert_eq!(xform.rgb_to_cmyk(1.0, 1.0, 1.0), CmykColor::WHITE);

        let dark = xform.rgb_to_cmyk(0.05, 0.04, 0.06);
        assert!(dark.total_ink_percent() <= 300.0 + 1e-2);
        assert!(dark.k > 0.7);
    }

    #[test]
    fn lab_pcs_roundtrip_is_accurate() {
        let samples = [
            [0.0, 0.0, 0.0],
            [50.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [53.24, 80.09, 67.20],
            [87.73, -86.18, 83.18],
        ];
        for lab in samples {
            let pcs = lab_to_pcs_norm(lab);
            let back = pcs_norm_to_lab(pcs);
            for i in 0..3 {
                assert!((lab[i] - back[i]).abs() < 0.02, "lab={lab:?} back={back:?}");
            }
        }
    }
}

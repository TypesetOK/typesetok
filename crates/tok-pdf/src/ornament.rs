//! Sacred built-in vector ornaments and zero-dependency SVG-to-PDF compiler.
//!
//! Compiles pure SVG geometry (`<svg>`, `<g>`, `<path>`, `<rect>`, `<circle>`,
//! `<ellipse>`, `<line>`, `<polygon>`, `<polyline>`) into exact PDF path
//! operators with quadratic-to-cubic Bézier elevation, Y-axis flipping, and
//! ICC-managed CMYK or Spot `/Separation` colors.

use crate::color::{CmykColor, ColorManagementEngine, IccProfileId};
use pdf_writer::types::ColorSpaceOperand;
use pdf_writer::{Content, Name, Ref};
use std::collections::BTreeMap;
use tok_core::styles::Color;

#[derive(Debug, Clone, Copy)]
pub struct BuiltinOrnamentSpec {
    pub id: &'static str,
    pub label_he: &'static str,
    pub label_en: &'static str,
    pub default_width_pt: f32,
    pub default_height_pt: f32,
    pub svg: &'static str,
}

pub struct BuiltinOrnaments;

impl BuiltinOrnaments {
    pub const ALL: &'static [BuiltinOrnamentSpec] = &[
        BuiltinOrnamentSpec {
            id: "crown_torah",
            label_he: "כתר תורה",
            label_en: "Torah Crown",
            default_width_pt: 96.0,
            default_height_pt: 32.0,
            svg: r#"<svg viewBox="0 0 120 40"><path d="M18 32 L10 14 L34 22 L60 6 L86 22 L110 14 L102 32 Z" fill="currentColor"/><rect x="20" y="34" width="80" height="3" rx="1.5" fill="currentColor"/><circle cx="60" cy="5" r="3" fill="currentColor"/><circle cx="10" cy="12" r="2.2" fill="currentColor"/><circle cx="110" cy="12" r="2.2" fill="currentColor"/></svg>"#,
        },
        BuiltinOrnamentSpec {
            id: "menorah_divider",
            label_he: "מנורה ומפריד",
            label_en: "Menorah Divider",
            default_width_pt: 120.0,
            default_height_pt: 28.0,
            svg: r#"<svg viewBox="0 0 120 32"><line x1="6" y1="16" x2="42" y2="16" stroke="currentColor" stroke-width="1.4"/><line x1="78" y1="16" x2="114" y2="16" stroke="currentColor" stroke-width="1.4"/><path d="M60 4 L60 26 M48 26 L72 26 M52 8 Q52 17 60 17 Q68 17 68 8 M45 8 Q45 21 60 21 Q75 21 75 8" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>"#,
        },
        BuiltinOrnamentSpec {
            id: "floral_parasha",
            label_he: "עיטור פרשה",
            label_en: "Floral Parasha",
            default_width_pt: 100.0,
            default_height_pt: 26.0,
            svg: r#"<svg viewBox="0 0 100 32"><path d="M50 4 C56 10 56 22 50 28 C44 22 44 10 50 4 Z M38 16 C44 10 56 10 62 16 C56 22 44 22 38 16 Z" fill="currentColor"/><path d="M12 16 Q24 9 34 16 Q24 23 12 16 Z M88 16 Q76 9 66 16 Q76 23 88 16 Z" fill="none" stroke="currentColor" stroke-width="1.4"/><circle cx="50" cy="16" r="2.5" fill="currentColor"/></svg>"#,
        },
        BuiltinOrnamentSpec {
            id: "star_tailpiece",
            label_he: "חותם מגן דוד",
            label_en: "Star Tailpiece",
            default_width_pt: 48.0,
            default_height_pt: 48.0,
            svg: r#"<svg viewBox="0 0 60 60"><polygon points="30,6 51,42 9,42" fill="none" stroke="currentColor" stroke-width="2"/><polygon points="30,54 9,18 51,18" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="30" cy="30" r="3.5" fill="currentColor"/></svg>"#,
        },
        BuiltinOrnamentSpec {
            id: "folio_cartouche",
            label_he: "מסגרת כרטוש למספור",
            label_en: "Folio Cartouche",
            default_width_pt: 72.0,
            default_height_pt: 22.0,
            svg: r#"<svg viewBox="0 0 120 36"><path d="M12 18 Q20 4 36 6 L84 6 Q100 4 108 18 Q100 32 84 30 L36 30 Q20 32 12 18 Z" fill="none" stroke="currentColor" stroke-width="1.3"/><circle cx="8" cy="18" r="2.2" fill="currentColor"/><circle cx="112" cy="18" r="2.2" fill="currentColor"/></svg>"#,
        },
    ];

    pub fn find(id: &str) -> Option<&'static BuiltinOrnamentSpec> {
        Self::ALL.iter().find(|s| s.id == id)
    }

    pub fn get_svg(id: &str) -> Option<&'static str> {
        Self::find(id).map(|s| s.svg)
    }
}

#[derive(Debug, Clone, Copy)]
struct Matrix2D {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Matrix2D {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn mul(self, rhs: Self) -> Self {
        Self {
            a: self.a * rhs.a + self.c * rhs.b,
            b: self.b * rhs.a + self.d * rhs.b,
            c: self.a * rhs.c + self.c * rhs.d,
            d: self.b * rhs.c + self.d * rhs.d,
            e: self.a * rhs.e + self.c * rhs.f + self.e,
            f: self.b * rhs.e + self.d * rhs.f + self.f,
        }
    }

    #[inline]
    fn apply(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
}

#[derive(Debug, Clone)]
enum PaintSpec {
    None,
    CurrentColor,
    ExplicitRgb(f32, f32, f32),
}

fn parse_attr(attrs: &str, name: &str) -> Option<String> {
    let bytes = attrs.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !(bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        let key = &attrs[key_start..i];
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            while i < bytes.len() && !(bytes[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            continue;
        }
        i += 1; // skip '='
        while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let quote = bytes[i];
        let val = if quote == b'"' || quote == b'\'' {
            i += 1;
            let v_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let s = &attrs[v_start..i];
            if i < bytes.len() {
                i += 1;
            }
            s
        } else {
            let v_start = i;
            while i < bytes.len() && !(bytes[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            &attrs[v_start..i]
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(val.trim().to_string());
        }
    }
    None
}

fn parse_f32_attr(attrs: &str, name: &str, default_val: f32) -> f32 {
    parse_attr(attrs, name)
        .and_then(|s| {
            s.trim_end_matches("px")
                .trim_end_matches("pt")
                .parse::<f32>()
                .ok()
        })
        .filter(|v| v.is_finite())
        .unwrap_or(default_val)
}

fn parse_paint(val: Option<&str>, default_spec: PaintSpec) -> PaintSpec {
    let Some(raw) = val.map(str::trim) else {
        return default_spec;
    };
    if raw.eq_ignore_ascii_case("none") || raw.eq_ignore_ascii_case("transparent") {
        return PaintSpec::None;
    }
    if raw.eq_ignore_ascii_case("currentcolor") || raw.eq_ignore_ascii_case("inherit") {
        return PaintSpec::CurrentColor;
    }
    if let Some(hex) = raw.strip_prefix('#') {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return PaintSpec::ExplicitRgb(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                );
            }
        } else if hex.len() == 3 {
            let nib = |c: &str| {
                u8::from_str_radix(c, 16)
                    .ok()
                    .map(|v| (v * 17) as f32 / 255.0)
            };
            if let (Some(r), Some(g), Some(b)) = (nib(&hex[0..1]), nib(&hex[1..2]), nib(&hex[2..3]))
            {
                return PaintSpec::ExplicitRgb(r, g, b);
            }
        }
    }
    if raw.eq_ignore_ascii_case("black") {
        return PaintSpec::ExplicitRgb(0.0, 0.0, 0.0);
    }
    PaintSpec::CurrentColor
}

fn parse_transform(attr: Option<&str>) -> Matrix2D {
    let Some(raw) = attr else {
        return Matrix2D::IDENTITY;
    };
    let mut cur = Matrix2D::IDENTITY;
    let mut rest = raw.trim();
    while !rest.is_empty() {
        let Some(open) = rest.find('(') else { break };
        let Some(close) = rest[open + 1..].find(')') else {
            break;
        };
        let op = rest[..open].trim();
        let args_str = &rest[open + 1..open + 1 + close];
        let nums: Vec<f32> = args_str
            .split(|c: char| c == ',' || c.is_ascii_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f32>().ok())
            .collect();
        let next = if op.eq_ignore_ascii_case("translate") && !nums.is_empty() {
            Matrix2D {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: nums[0],
                f: *nums.get(1).unwrap_or(&0.0),
            }
        } else if op.eq_ignore_ascii_case("scale") && !nums.is_empty() {
            let sx = nums[0];
            let sy = *nums.get(1).unwrap_or(&sx);
            Matrix2D {
                a: sx,
                b: 0.0,
                c: 0.0,
                d: sy,
                e: 0.0,
                f: 0.0,
            }
        } else if op.eq_ignore_ascii_case("matrix") && nums.len() >= 6 {
            Matrix2D {
                a: nums[0],
                b: nums[1],
                c: nums[2],
                d: nums[3],
                e: nums[4],
                f: nums[5],
            }
        } else {
            Matrix2D::IDENTITY
        };
        cur = cur.mul(next);
        rest = rest[open + 1 + close + 1..].trim();
    }
    cur
}

fn parse_number_list(raw: &str) -> Vec<f32> {
    let mut nums = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b',' || (b as char).is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if b == b'+' || b == b'-' {
            i += 1;
        }
        let mut seen_dot = false;
        while i < bytes.len() {
            let ch = bytes[i];
            if ch.is_ascii_digit() {
                i += 1;
            } else if ch == b'.' && !seen_dot {
                seen_dot = true;
                i += 1;
            } else if (ch == b'e' || ch == b'E') && i + 1 < bytes.len() {
                i += 1;
                if bytes[i] == b'+' || bytes[i] == b'-' {
                    i += 1;
                }
            } else {
                break;
            }
        }
        if i > start {
            if let Ok(v) = raw[start..i].parse::<f32>() {
                if v.is_finite() {
                    nums.push(v);
                }
            }
        } else {
            i += 1;
        }
    }
    nums
}

#[derive(Debug, Clone)]
enum PathToken {
    Cmd(char),
    Num(f32),
}

fn tokenize_path_d(d: &str) -> Vec<PathToken> {
    let mut out = Vec::new();
    let bytes = d.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if (b as char).is_ascii_whitespace() || b == b',' {
            i += 1;
            continue;
        }
        if (b as char).is_ascii_alphabetic() && b != b'e' && b != b'E' {
            out.push(PathToken::Cmd(b as char));
            i += 1;
            continue;
        }
        let start = i;
        if b == b'+' || b == b'-' {
            i += 1;
        }
        let mut seen_dot = false;
        while i < bytes.len() {
            let ch = bytes[i];
            if ch.is_ascii_digit() {
                i += 1;
            } else if ch == b'.' && !seen_dot {
                seen_dot = true;
                i += 1;
            } else if (ch == b'e' || ch == b'E') && i > start && i + 1 < bytes.len() {
                i += 1;
                if bytes[i] == b'+' || bytes[i] == b'-' {
                    i += 1;
                }
            } else {
                break;
            }
        }
        if i > start {
            if let Ok(n) = d[start..i].parse::<f32>() {
                if n.is_finite() {
                    out.push(PathToken::Num(n));
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

pub struct SvgVectorCompiler;

impl SvgVectorCompiler {
    /// Emits a fill color matching `PaintSpec` (Spot or ICC-managed CMYK; never DeviceRGB).
    fn set_fill_paint(
        content: &mut Content,
        spec: &PaintSpec,
        default_color: &Color,
        profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
    ) {
        match spec {
            PaintSpec::None => {}
            PaintSpec::CurrentColor => {
                if let Color::Spot { name, tint, .. } = default_color {
                    if let Some((res_name, _, _)) = spot_colors.get(name) {
                        content.set_fill_color_space(ColorSpaceOperand::Named(Name(
                            res_name.as_bytes(),
                        )));
                        content.set_fill_color([tint.clamp(0.0, 1.0)]);
                        return;
                    }
                }
                let cmyk = ColorManagementEngine::convert_color(default_color, profile);
                content.set_fill_cmyk(cmyk.c, cmyk.m, cmyk.y, cmyk.k);
            }
            PaintSpec::ExplicitRgb(r, g, b) => {
                let cmyk = ColorManagementEngine::rgb_to_cmyk_managed(*r, *g, *b, profile);
                content.set_fill_cmyk(cmyk.c, cmyk.m, cmyk.y, cmyk.k);
            }
        }
    }

    /// Emits a stroke color matching `PaintSpec` (Spot or ICC-managed CMYK; never DeviceRGB).
    fn set_stroke_paint(
        content: &mut Content,
        spec: &PaintSpec,
        default_color: &Color,
        profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
    ) {
        match spec {
            PaintSpec::None => {}
            PaintSpec::CurrentColor => {
                if let Color::Spot { name, tint, .. } = default_color {
                    if let Some((res_name, _, _)) = spot_colors.get(name) {
                        content.set_stroke_color_space(ColorSpaceOperand::Named(Name(
                            res_name.as_bytes(),
                        )));
                        content.set_stroke_color([tint.clamp(0.0, 1.0)]);
                        return;
                    }
                }
                let cmyk = ColorManagementEngine::convert_color(default_color, profile);
                content.set_stroke_cmyk(cmyk.c, cmyk.m, cmyk.y, cmyk.k);
            }
            PaintSpec::ExplicitRgb(r, g, b) => {
                let cmyk = ColorManagementEngine::rgb_to_cmyk_managed(*r, *g, *b, profile);
                content.set_stroke_cmyk(cmyk.c, cmyk.m, cmyk.y, cmyk.k);
            }
        }
    }

    fn emit_ellipse_path(
        content: &mut Content,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        map_pt: &dyn Fn(f32, f32) -> (f32, f32),
    ) {
        const KAPPA: f32 = 0.5522848;
        let ox = rx * KAPPA;
        let oy = ry * KAPPA;
        let (p0x, p0y) = map_pt(cx + rx, cy);
        content.move_to(p0x, p0y);

        let (c1x, c1y) = map_pt(cx + rx, cy + oy);
        let (c2x, c2y) = map_pt(cx + ox, cy + ry);
        let (p1x, p1y) = map_pt(cx, cy + ry);
        content.cubic_to(c1x, c1y, c2x, c2y, p1x, p1y);

        let (c3x, c3y) = map_pt(cx - ox, cy + ry);
        let (c4x, c4y) = map_pt(cx - rx, cy + oy);
        let (p2x, p2y) = map_pt(cx - rx, cy);
        content.cubic_to(c3x, c3y, c4x, c4y, p2x, p2y);

        let (c5x, c5y) = map_pt(cx - rx, cy - oy);
        let (c6x, c6y) = map_pt(cx - ox, cy - ry);
        let (p3x, p3y) = map_pt(cx, cy - ry);
        content.cubic_to(c5x, c5y, c6x, c6y, p3x, p3y);

        let (c7x, c7y) = map_pt(cx + ox, cy - ry);
        let (c8x, c8y) = map_pt(cx + rx, cy - oy);
        content.cubic_to(c7x, c7y, c8x, c8y, p0x, p0y);
        content.close_path();
    }

    fn emit_path_d(content: &mut Content, d: &str, map_pt: &dyn Fn(f32, f32) -> (f32, f32)) {
        let tokens = tokenize_path_d(d);
        let mut ti = 0usize;
        let mut cmd = '\0';
        let mut cur_x = 0.0_f32;
        let mut cur_y = 0.0_f32;
        let mut sub_x = 0.0_f32;
        let mut sub_y = 0.0_f32;
        let mut prev_c2: Option<(f32, f32)> = None;
        let mut prev_qcp: Option<(f32, f32)> = None;

        let next_num = |tokens: &[PathToken], idx: &mut usize| -> Option<f32> {
            match tokens.get(*idx) {
                Some(PathToken::Num(n)) => {
                    *idx += 1;
                    Some(*n)
                }
                _ => None,
            }
        };

        while ti < tokens.len() {
            if let PathToken::Cmd(c) = tokens[ti] {
                cmd = c;
                ti += 1;
                if matches!(cmd, 'Z' | 'z') {
                    content.close_path();
                    cur_x = sub_x;
                    cur_y = sub_y;
                    prev_c2 = None;
                    prev_qcp = None;
                    continue;
                }
            }
            if cmd == '\0' {
                ti += 1;
                continue;
            }
            let is_rel = cmd.is_ascii_lowercase();
            match cmd {
                'M' | 'm' => {
                    let (Some(mut x), Some(mut y)) =
                        (next_num(&tokens, &mut ti), next_num(&tokens, &mut ti))
                    else {
                        break;
                    };
                    if is_rel {
                        x += cur_x;
                        y += cur_y;
                    }
                    let (px, py) = map_pt(x, y);
                    content.move_to(px, py);
                    cur_x = x;
                    cur_y = y;
                    sub_x = x;
                    sub_y = y;
                    cmd = if is_rel { 'l' } else { 'L' };
                    prev_c2 = None;
                    prev_qcp = None;
                }
                'L' | 'l' => {
                    let (Some(mut x), Some(mut y)) =
                        (next_num(&tokens, &mut ti), next_num(&tokens, &mut ti))
                    else {
                        break;
                    };
                    if is_rel {
                        x += cur_x;
                        y += cur_y;
                    }
                    let (px, py) = map_pt(x, y);
                    content.line_to(px, py);
                    cur_x = x;
                    cur_y = y;
                    prev_c2 = None;
                    prev_qcp = None;
                }
                'H' | 'h' => {
                    let Some(mut x) = next_num(&tokens, &mut ti) else {
                        break;
                    };
                    if is_rel {
                        x += cur_x;
                    }
                    let (px, py) = map_pt(x, cur_y);
                    content.line_to(px, py);
                    cur_x = x;
                    prev_c2 = None;
                    prev_qcp = None;
                }
                'V' | 'v' => {
                    let Some(mut y) = next_num(&tokens, &mut ti) else {
                        break;
                    };
                    if is_rel {
                        y += cur_y;
                    }
                    let (px, py) = map_pt(cur_x, y);
                    content.line_to(px, py);
                    cur_y = y;
                    prev_c2 = None;
                    prev_qcp = None;
                }
                'C' | 'c' => {
                    let (
                        Some(mut x1),
                        Some(mut y1),
                        Some(mut x2),
                        Some(mut y2),
                        Some(mut x),
                        Some(mut y),
                    ) = (
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                    )
                    else {
                        break;
                    };
                    if is_rel {
                        x1 += cur_x;
                        y1 += cur_y;
                        x2 += cur_x;
                        y2 += cur_y;
                        x += cur_x;
                        y += cur_y;
                    }
                    let (px1, py1) = map_pt(x1, y1);
                    let (px2, py2) = map_pt(x2, y2);
                    let (px, py) = map_pt(x, y);
                    content.cubic_to(px1, py1, px2, py2, px, py);
                    prev_c2 = Some((x2, y2));
                    prev_qcp = None;
                    cur_x = x;
                    cur_y = y;
                }
                'S' | 's' => {
                    let (Some(mut x2), Some(mut y2), Some(mut x), Some(mut y)) = (
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                    ) else {
                        break;
                    };
                    if is_rel {
                        x2 += cur_x;
                        y2 += cur_y;
                        x += cur_x;
                        y += cur_y;
                    }
                    let (x1, y1) = match prev_c2 {
                        Some((cx, cy)) => (2.0 * cur_x - cx, 2.0 * cur_y - cy),
                        None => (cur_x, cur_y),
                    };
                    let (px1, py1) = map_pt(x1, y1);
                    let (px2, py2) = map_pt(x2, y2);
                    let (px, py) = map_pt(x, y);
                    content.cubic_to(px1, py1, px2, py2, px, py);
                    prev_c2 = Some((x2, y2));
                    prev_qcp = None;
                    cur_x = x;
                    cur_y = y;
                }
                'Q' | 'q' => {
                    // Exact quadratic to cubic degree elevation:
                    // C1 = P0 + 2/3 * (Q1 - P0), C2 = P1 + 2/3 * (Q1 - P1)
                    let (Some(mut qx), Some(mut qy), Some(mut x), Some(mut y)) = (
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                    ) else {
                        break;
                    };
                    if is_rel {
                        qx += cur_x;
                        qy += cur_y;
                        x += cur_x;
                        y += cur_y;
                    }
                    let c1x = cur_x + (2.0 / 3.0) * (qx - cur_x);
                    let c1y = cur_y + (2.0 / 3.0) * (qy - cur_y);
                    let c2x = x + (2.0 / 3.0) * (qx - x);
                    let c2y = y + (2.0 / 3.0) * (qy - y);
                    let (px1, py1) = map_pt(c1x, c1y);
                    let (px2, py2) = map_pt(c2x, c2y);
                    let (px, py) = map_pt(x, y);
                    content.cubic_to(px1, py1, px2, py2, px, py);
                    prev_qcp = Some((qx, qy));
                    prev_c2 = None;
                    cur_x = x;
                    cur_y = y;
                }
                'T' | 't' => {
                    let (Some(mut x), Some(mut y)) =
                        (next_num(&tokens, &mut ti), next_num(&tokens, &mut ti))
                    else {
                        break;
                    };
                    if is_rel {
                        x += cur_x;
                        y += cur_y;
                    }
                    let (qx, qy) = match prev_qcp {
                        Some((px, py)) => (2.0 * cur_x - px, 2.0 * cur_y - py),
                        None => (cur_x, cur_y),
                    };
                    let c1x = cur_x + (2.0 / 3.0) * (qx - cur_x);
                    let c1y = cur_y + (2.0 / 3.0) * (qy - cur_y);
                    let c2x = x + (2.0 / 3.0) * (qx - x);
                    let c2y = y + (2.0 / 3.0) * (qy - y);
                    let (px1, py1) = map_pt(c1x, c1y);
                    let (px2, py2) = map_pt(c2x, c2y);
                    let (px, py) = map_pt(x, y);
                    content.cubic_to(px1, py1, px2, py2, px, py);
                    prev_qcp = Some((qx, qy));
                    prev_c2 = None;
                    cur_x = x;
                    cur_y = y;
                }
                'A' | 'a' => {
                    // Consume 7 elliptical arc parameters cleanly and approximate segment end with line_to
                    let vals = [
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                        next_num(&tokens, &mut ti),
                    ];
                    if let (Some(mut x), Some(mut y)) = (vals[5], vals[6]) {
                        if is_rel {
                            x += cur_x;
                            y += cur_y;
                        }
                        let (px, py) = map_pt(x, y);
                        content.line_to(px, py);
                        cur_x = x;
                        cur_y = y;
                        prev_c2 = None;
                        prev_qcp = None;
                    } else {
                        break;
                    }
                }
                _ => {
                    ti += 1;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_paint(
        content: &mut Content,
        fill: &PaintSpec,
        stroke: &PaintSpec,
        stroke_width: f32,
        scale: f32,
        default_color: &Color,
        profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
    ) -> (bool, bool) {
        let do_fill = !matches!(fill, PaintSpec::None);
        let do_stroke = !matches!(stroke, PaintSpec::None);
        if do_fill {
            Self::set_fill_paint(content, fill, default_color, profile, spot_colors);
        }
        if do_stroke {
            Self::set_stroke_paint(content, stroke, default_color, profile, spot_colors);
            content.set_line_width((stroke_width * scale).max(0.25));
        }
        (do_fill, do_stroke)
    }

    fn finish_paint(content: &mut Content, do_fill: bool, do_stroke: bool, fill_even_odd: bool) {
        match (do_fill, do_stroke, fill_even_odd) {
            (true, true, true) => {
                content.fill_even_odd_and_stroke();
            }
            (true, true, false) => {
                content.fill_nonzero_and_stroke();
            }
            (true, false, true) => {
                content.fill_even_odd();
            }
            (true, false, false) => {
                content.fill_nonzero();
            }
            (false, true, _) => {
                content.stroke();
            }
            (false, false, _) => {}
        }
    }

    /// Compiles an SVG document into PDF vector graphics operators inside `(pdf_x, pdf_y, width_pt, height_pt)`
    /// (where `pdf_y` is bottom-left in PDF points). Returns number of drawn vector shapes.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_into_content(
        svg: &str,
        pdf_x: f32,
        pdf_y: f32,
        width_pt: f32,
        height_pt: f32,
        default_color: &Color,
        profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
        content: &mut Content,
    ) -> Result<usize, String> {
        let mut vb_x = 0.0_f32;
        let mut vb_y = 0.0_f32;
        let mut vb_w = 100.0_f32;
        let mut vb_h = 100.0_f32;

        if let Some(svg_start) = svg.find("<svg") {
            if let Some(tag_end) = svg[svg_start..].find('>') {
                let attrs = &svg[svg_start + 4..svg_start + tag_end];
                if let Some(vb) = parse_attr(attrs, "viewBox") {
                    let nums = parse_number_list(&vb);
                    if nums.len() == 4 && nums[2] > 0.0 && nums[3] > 0.0 {
                        vb_x = nums[0];
                        vb_y = nums[1];
                        vb_w = nums[2];
                        vb_h = nums[3];
                    }
                } else {
                    let w = parse_f32_attr(attrs, "width", 100.0);
                    let h = parse_f32_attr(attrs, "height", 100.0);
                    if w > 0.0 && h > 0.0 {
                        vb_w = w;
                        vb_h = h;
                    }
                }
            }
        }

        let scale = (width_pt / vb_w).min(height_pt / vb_h).max(1e-4);
        let pad_x = (width_pt - vb_w * scale) * 0.5;
        let pad_y = (height_pt - vb_h * scale) * 0.5;

        let mut stack: Vec<Matrix2D> = vec![Matrix2D::IDENTITY];
        let mut drawn_shapes = 0usize;

        content.save_state();

        let mut pos = 0usize;
        while let Some(rel_lt) = svg[pos..].find('<') {
            let tag_start = pos + rel_lt + 1;
            let Some(rel_gt) = svg[tag_start..].find('>') else {
                break;
            };
            let raw_tag = svg[tag_start..tag_start + rel_gt].trim();
            pos = tag_start + rel_gt + 1;

            if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
                continue;
            }
            if let Some(rest_tag) = raw_tag.strip_prefix('/') {
                let name = rest_tag.trim();
                if name.eq_ignore_ascii_case("g") && stack.len() > 1 {
                    stack.pop();
                }
                continue;
            }
            let self_closing = raw_tag.ends_with('/');
            let inner = if self_closing {
                raw_tag[..raw_tag.len() - 1].trim()
            } else {
                raw_tag
            };
            let split_idx = inner
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(inner.len());
            let tag_name = inner[..split_idx].trim();
            let attrs = inner[split_idx..].trim();

            if tag_name.eq_ignore_ascii_case("g") {
                let top = *stack.last().unwrap_or(&Matrix2D::IDENTITY);
                let xf = parse_transform(parse_attr(attrs, "transform").as_deref());
                if !self_closing {
                    stack.push(top.mul(xf));
                }
                continue;
            }

            let cur_xf = stack
                .last()
                .unwrap_or(&Matrix2D::IDENTITY)
                .mul(parse_transform(parse_attr(attrs, "transform").as_deref()));

            let map_pt = |sx: f32, sy: f32| -> (f32, f32) {
                let (ux, uy) = cur_xf.apply(sx, sy);
                let x = pdf_x + pad_x + (ux - vb_x) * scale;
                let y = pdf_y + height_pt - pad_y - (uy - vb_y) * scale;
                (x, y)
            };

            let fill_attr = parse_attr(attrs, "fill");
            let stroke_attr = parse_attr(attrs, "stroke");
            let fill = parse_paint(fill_attr.as_deref(), PaintSpec::CurrentColor);
            let stroke = parse_paint(stroke_attr.as_deref(), PaintSpec::None);
            let fill_even_odd =
                parse_attr(attrs, "fill-rule").is_some_and(|r| r.eq_ignore_ascii_case("evenodd"));
            let stroke_width = parse_f32_attr(attrs, "stroke-width", 1.0);

            if tag_name.eq_ignore_ascii_case("path") {
                if let Some(d) = parse_attr(attrs, "d") {
                    let (do_fill, do_stroke) = Self::prepare_paint(
                        content,
                        &fill,
                        &stroke,
                        stroke_width,
                        scale,
                        default_color,
                        profile,
                        spot_colors,
                    );
                    if do_fill || do_stroke {
                        Self::emit_path_d(content, &d, &map_pt);
                        Self::finish_paint(content, do_fill, do_stroke, fill_even_odd);
                        drawn_shapes += 1;
                    }
                }
            } else if tag_name.eq_ignore_ascii_case("rect") {
                let x = parse_f32_attr(attrs, "x", 0.0);
                let y = parse_f32_attr(attrs, "y", 0.0);
                let w = parse_f32_attr(attrs, "width", 0.0);
                let h = parse_f32_attr(attrs, "height", 0.0);
                if w > 0.0 && h > 0.0 {
                    let (do_fill, do_stroke) = Self::prepare_paint(
                        content,
                        &fill,
                        &stroke,
                        stroke_width,
                        scale,
                        default_color,
                        profile,
                        spot_colors,
                    );
                    if do_fill || do_stroke {
                        let (p0x, p0y) = map_pt(x, y);
                        let (p1x, p1y) = map_pt(x + w, y);
                        let (p2x, p2y) = map_pt(x + w, y + h);
                        let (p3x, p3y) = map_pt(x, y + h);
                        content
                            .move_to(p0x, p0y)
                            .line_to(p1x, p1y)
                            .line_to(p2x, p2y)
                            .line_to(p3x, p3y)
                            .close_path();
                        Self::finish_paint(content, do_fill, do_stroke, fill_even_odd);
                        drawn_shapes += 1;
                    }
                }
            } else if tag_name.eq_ignore_ascii_case("circle") {
                let cx = parse_f32_attr(attrs, "cx", 0.0);
                let cy = parse_f32_attr(attrs, "cy", 0.0);
                let r = parse_f32_attr(attrs, "r", 0.0);
                if r > 0.0 {
                    let (do_fill, do_stroke) = Self::prepare_paint(
                        content,
                        &fill,
                        &stroke,
                        stroke_width,
                        scale,
                        default_color,
                        profile,
                        spot_colors,
                    );
                    if do_fill || do_stroke {
                        Self::emit_ellipse_path(content, cx, cy, r, r, &map_pt);
                        Self::finish_paint(content, do_fill, do_stroke, fill_even_odd);
                        drawn_shapes += 1;
                    }
                }
            } else if tag_name.eq_ignore_ascii_case("ellipse") {
                let cx = parse_f32_attr(attrs, "cx", 0.0);
                let cy = parse_f32_attr(attrs, "cy", 0.0);
                let rx = parse_f32_attr(attrs, "rx", 0.0);
                let ry = parse_f32_attr(attrs, "ry", 0.0);
                if rx > 0.0 && ry > 0.0 {
                    let (do_fill, do_stroke) = Self::prepare_paint(
                        content,
                        &fill,
                        &stroke,
                        stroke_width,
                        scale,
                        default_color,
                        profile,
                        spot_colors,
                    );
                    if do_fill || do_stroke {
                        Self::emit_ellipse_path(content, cx, cy, rx, ry, &map_pt);
                        Self::finish_paint(content, do_fill, do_stroke, fill_even_odd);
                        drawn_shapes += 1;
                    }
                }
            } else if tag_name.eq_ignore_ascii_case("line") {
                let x1 = parse_f32_attr(attrs, "x1", 0.0);
                let y1 = parse_f32_attr(attrs, "y1", 0.0);
                let x2 = parse_f32_attr(attrs, "x2", 0.0);
                let y2 = parse_f32_attr(attrs, "y2", 0.0);
                let eff_stroke = parse_paint(stroke_attr.as_deref(), PaintSpec::CurrentColor);
                let (do_fill, do_stroke) = Self::prepare_paint(
                    content,
                    &PaintSpec::None,
                    &eff_stroke,
                    stroke_width,
                    scale,
                    default_color,
                    profile,
                    spot_colors,
                );
                if do_fill || do_stroke {
                    let (p1x, p1y) = map_pt(x1, y1);
                    let (p2x, p2y) = map_pt(x2, y2);
                    content.move_to(p1x, p1y).line_to(p2x, p2y);
                    Self::finish_paint(content, do_fill, do_stroke, false);
                    drawn_shapes += 1;
                }
            } else if tag_name.eq_ignore_ascii_case("polygon")
                || tag_name.eq_ignore_ascii_case("polyline")
            {
                let close = tag_name.eq_ignore_ascii_case("polygon");
                if let Some(pts_str) = parse_attr(attrs, "points") {
                    let nums = parse_number_list(&pts_str);
                    if nums.len() >= 4 {
                        let eff_fill = if close {
                            fill
                        } else {
                            parse_paint(fill_attr.as_deref(), PaintSpec::None)
                        };
                        let eff_stroke = if !close && stroke_attr.is_none() {
                            PaintSpec::CurrentColor
                        } else {
                            stroke
                        };
                        let (do_fill, do_stroke) = Self::prepare_paint(
                            content,
                            &eff_fill,
                            &eff_stroke,
                            stroke_width,
                            scale,
                            default_color,
                            profile,
                            spot_colors,
                        );
                        if do_fill || do_stroke {
                            let (x0, y0) = map_pt(nums[0], nums[1]);
                            content.move_to(x0, y0);
                            for pair in nums[2..].as_chunks::<2>().0 {
                                let (px, py) = map_pt(pair[0], pair[1]);
                                content.line_to(px, py);
                            }
                            if close {
                                content.close_path();
                            }
                            Self::finish_paint(content, do_fill, do_stroke, fill_even_odd);
                            drawn_shapes += 1;
                        }
                    }
                }
            }
        }

        content.restore_state();
        Ok(drawn_shapes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_all_four_builtin_ornaments_into_cmyk_vectors() {
        let spots = BTreeMap::new();
        for spec in BuiltinOrnaments::ALL {
            let mut content = Content::new();
            let count = SvgVectorCompiler::compile_into_content(
                spec.svg,
                100.0,
                500.0,
                spec.default_width_pt,
                spec.default_height_pt,
                &Color::default(),
                IccProfileId::Fogra39,
                &spots,
                &mut content,
            )
            .unwrap();
            assert!(
                count >= 2,
                "Ornament {} should emit multiple shapes",
                spec.id
            );
            let finished = content.finish();
            let s = String::from_utf8_lossy(&finished);
            assert!(s.contains(" k\n") || s.contains(" K\n"));
            assert!(!s.contains(" rg\n") && !s.contains(" RG\n"));
        }
    }

    #[test]
    fn converts_quadratic_bezier_q_to_cubic_c_with_y_flip() {
        let svg = r##"<svg viewBox="0 0 100 100"><g transform="translate(10,0)"><path d="M 0 0 Q 50 100 80 0" fill="#D4AF37"/></g></svg>"##;
        let spots = BTreeMap::new();
        let mut content = Content::new();
        let count = SvgVectorCompiler::compile_into_content(
            svg,
            0.0,
            200.0,
            100.0,
            100.0,
            &Color::default(),
            IccProfileId::Fogra39,
            &spots,
            &mut content,
        )
        .unwrap();
        assert_eq!(count, 1);
        let finished = content.finish();
        let s = String::from_utf8_lossy(&finished);
        assert!(
            s.contains("10 300 m\n"),
            "Start M(0,0) with translate(10,0) maps to (10,300)"
        );
        assert!(
            s.contains("90 300 c\n"),
            "Quadratic Q must elevate to cubic Bézier 'c' ending at (90,300)"
        );
    }
}

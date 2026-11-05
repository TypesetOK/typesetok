//! Professional Optical Margin Alignment (Hanging Punctuation & Glyph Overhang Protrusion).
//!
//! Implements optical margin alignment for Hebrew (RTL), Latin (LTR), and bidirectional text:
//! - Punctuation protrusion: hyphens, Hebrew maqaf (`־`), geresh (`׳`), gershayim (`״`),
//!   sof pasuk (`׃`), periods, commas, colons, semicolons, quotation marks, and brackets
//!   hang into the margin so the perceived optical edge of the column remains straight.
//! - Glyph overhang protrusion: curved and diagonal letterforms (such as Hebrew `א`, `ד`, `י`, `ל`, `ס`, `ע`, `ר`, `ת`
//!   and Latin `A`, `C`, `O`, `T`, `V`, `W`, `Y`) slightly overhang the margin boundary according to their
//!   stroke geometry and optical density.
//! - Combining mark awareness: zero-advance Hebrew niqqud and cantillation (ta'amim) marks attached to
//!   an edge consonant never obscure the base character's optical protrusion, and move in lockstep with
//!   their base glyph.

use crate::shaper::PositionedGlyph;
use tok_core::styles::{OpticalMarginConfig, TextAlignment};

/// Computed optical overhang protrusion (in points and normalized factors) for a single line.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OpticalProtrusion {
    /// Distance (in pt) by which the visual leftmost glyph overhangs left of the left margin (`x_offset`).
    pub left_pt: f32,
    /// Distance (in pt) by which the visual rightmost glyph overhangs right of the right margin (`x_offset + target_width`).
    pub right_pt: f32,
    /// Normalized left overhang factor relative to the leftmost glyph's width `[0.0, 1.0]`.
    pub left_factor: f32,
    /// Normalized right overhang factor relative to the rightmost glyph's width `[0.0, 1.0]`.
    pub right_factor: f32,
}

pub struct OpticalMarginAligner;

impl OpticalMarginAligner {
    /// Returns true if `c` is a zero-width combining mark (Hebrew Niqqud, Ta'amim, or Unicode combining diacritic).
    #[inline]
    pub fn is_combining_mark(c: char) -> bool {
        matches!(
            c,
            '\u{0591}'..='\u{05BD}'
                | '\u{05BF}'
                | '\u{05C1}'..='\u{05C2}'
                | '\u{05C4}'..='\u{05C5}'
                | '\u{05C7}'
                | '\u{0300}'..='\u{036F}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
    }

    /// Returns true if `c` is a punctuation character eligible for hanging punctuation protrusion.
    #[inline]
    pub fn is_protrudable_punctuation(c: char) -> bool {
        matches!(
            c,
            '.' | ','
                | '-'
                | '\u{05BE}' // Hebrew Maqaf (־)
                | '\u{05F3}' // Hebrew Geresh (׳)
                | '\u{05F4}' // Hebrew Gershayim (״)
                | '\u{05C3}' // Hebrew Sof Pasuk (׃)
                | '\u{05C0}' // Hebrew Paseq (׀)
                | '\u{2010}' // Hyphen
                | '\u{2011}' // Non-breaking hyphen
                | '\u{00AD}' // Soft hyphen
                | '\u{2013}' // En-dash (–)
                | '\u{2014}' // Em-dash (—)
                | '\''
                | '"'
                | '\u{2018}' // ‘
                | '\u{2019}' // ’
                | '\u{201C}' // “
                | '\u{201D}' // ”
                | '\u{2039}' // ‹
                | '\u{203A}' // ›
                | '\u{00AB}' // «
                | '\u{00BB}' // »
                | ':'
                | ';'
                | '!'
                | '?'
                | '\u{2026}' // …
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
        )
    }

    /// Returns `(left_factor, right_factor)` representing the fraction of the character's advance width
    /// that should protrude beyond the left or right margin boundary.
    pub fn protrusion_factors_for_char(c: char, config: &OpticalMarginConfig) -> (f32, f32) {
        if !config.enabled || config.factor <= 0.0 || !config.factor.is_finite() {
            return (0.0, 0.0);
        }

        let scale = config.factor.clamp(0.0, 2.0);

        if Self::is_protrudable_punctuation(c) {
            if !config.protrude_punctuation {
                return (0.0, 0.0);
            }
            let (lf, rf) = match c {
                // High-protrusion small punctuation
                '.' | ',' => (0.60, 0.70),
                // Hyphens and Hebrew Maqaf
                '-' | '\u{05BE}' | '\u{2010}' | '\u{2011}' | '\u{00AD}' => (0.65, 0.70),
                // Hebrew Geresh & Gershayim, single & double quotation marks
                '\u{05F3}' | '\u{05F4}' | '\'' | '"' | '\u{2018}' | '\u{2019}' | '\u{201C}'
                | '\u{201D}' | '\u{2039}' | '\u{203A}' => (0.55, 0.55),
                // Guillemets
                '\u{00AB}' | '\u{00BB}' => (0.40, 0.40),
                // Colons, semicolons, Hebrew Sof Pasuk and Paseq
                ':' | ';' | '\u{05C3}' | '\u{05C0}' => (0.35, 0.45),
                // Dashes and ellipsis
                '\u{2013}' => (0.35, 0.35),
                '\u{2014}' | '\u{2026}' => (0.25, 0.25),
                // Exclamation and question marks
                '!' | '?' => (0.20, 0.25),
                // Parentheses and brackets (curved outer edges)
                '(' | '[' | '{' | ')' | ']' | '}' => (0.30, 0.30),
                _ => (0.0, 0.0),
            };
            return ((lf * scale).clamp(0.0, 0.85), (rf * scale).clamp(0.0, 0.85));
        }

        if !config.protrude_glyphs {
            return (0.0, 0.0);
        }

        let (lf, rf) = match c {
            // Hebrew letterforms: (left_edge_factor, right_edge_factor)
            // In RTL, right edge is line start and left edge is line end.
            'א' => (0.06, 0.06), // Diagonal upper-right arm and lower-left leg
            'ב' => (0.02, 0.04), // Base heel on right
            'ג' => (0.06, 0.05), // Arching foot
            'ד' => (0.04, 0.08), // Pronounced top-right roof overhang (serif)
            'ה' => (0.03, 0.03),
            'ו' => (0.05, 0.05), // Narrow vertical stem with top hook
            'ז' => (0.07, 0.07), // Top bar overhangs both sides of stem
            'ח' => (0.02, 0.02),
            'ט' => (0.04, 0.05),       // Rounded belly
            'י' => (0.14, 0.14),       // Small suspended glyph with large lower whitespace
            'כ' | 'ך' => (0.04, 0.06), // Curved right shoulder
            'ל' => (0.06, 0.05),       // Ascender and curved body
            'מ' => (0.03, 0.04),
            'ם' => (0.02, 0.03),
            'נ' | 'ן' => (0.03, 0.05),
            'ס' => (0.07, 0.07),       // Round bowl on both left and right edges
            'ע' => (0.07, 0.06),       // Diagonal strokes
            'פ' | 'ף' => (0.05, 0.06), // Curved right bowl
            'צ' | 'ץ' => (0.06, 0.06), // Diagonal upper arms
            'ק' => (0.06, 0.04),       // Open lower-right, curved upper-right
            'ר' => (0.07, 0.06),       // Curved top-right shoulder, open left whitespace
            'ש' => (0.05, 0.05),       // Angled outer arms
            'ת' => (0.08, 0.03),       // Pronounced left foot protrusion (עקב של ת)

            // Latin letterforms: curved bowls and diagonals
            'C' | 'G' | 'O' | 'Q' | 'c' | 'e' | 'o' => (0.06, 0.06),
            'S' | 's' => (0.05, 0.05),
            'A' | 'V' | 'W' | 'X' | 'Y' | 'v' | 'w' | 'x' | 'y' => (0.07, 0.07),
            'T' => (0.07, 0.07),
            'J' | 'j' => (0.05, 0.03),
            'f' | 'r' | 't' => (0.03, 0.05),
            _ => (0.0, 0.0),
        };

        ((lf * scale).clamp(0.0, 0.50), (rf * scale).clamp(0.0, 0.50))
    }

    /// Computes the left and right optical protrusion (in pt) for a visually ordered sequence of glyphs.
    pub fn compute_line_protrusion(
        glyphs: &[PositionedGlyph],
        font_size_pt: f32,
        config: &OpticalMarginConfig,
    ) -> OpticalProtrusion {
        if !config.enabled || glyphs.is_empty() {
            return OpticalProtrusion::default();
        }

        let fallback_ref_w = (font_size_pt * 0.35).max(2.0);

        // 1. Find leftmost non-space base/spacing glyph in visual order
        let mut left_pt = 0.0;
        let mut left_factor = 0.0;
        for g in glyphs.iter() {
            let Some(ch) = g.character else {
                continue;
            };
            if ch.is_whitespace() {
                continue;
            }
            if Self::is_combining_mark(ch) && g.x_advance <= 0.5 {
                continue;
            }
            let (lf, _) = Self::protrusion_factors_for_char(ch, config);
            let ref_w = if g.x_advance > 0.5 {
                g.x_advance
            } else {
                fallback_ref_w
            };
            left_factor = lf;
            left_pt = lf * ref_w;
            break;
        }

        // 2. Find rightmost non-space base/spacing glyph in visual order
        let mut right_pt = 0.0;
        let mut right_factor = 0.0;
        for g in glyphs.iter().rev() {
            let Some(ch) = g.character else {
                continue;
            };
            if ch.is_whitespace() {
                continue;
            }
            if Self::is_combining_mark(ch) && g.x_advance <= 0.5 {
                continue;
            }
            let (_, rf) = Self::protrusion_factors_for_char(ch, config);
            let ref_w = if g.x_advance > 0.5 {
                g.x_advance
            } else {
                fallback_ref_w
            };
            right_factor = rf;
            right_pt = rf * ref_w;
            break;
        }

        OpticalProtrusion {
            left_pt,
            right_pt,
            left_factor,
            right_factor,
        }
    }

    /// Estimates the optical protrusion (in pt) of a line's logical start and end text
    /// for use during Knuth-Plass line breaking before visual reordering.
    pub fn estimate_span_protrusion_bonus(
        first_box_text: &str,
        last_box_text: &str,
        break_is_flagged_hyphen: bool,
        font_size_pt: f32,
        is_rtl: bool,
        config: &OpticalMarginConfig,
    ) -> f32 {
        if !config.enabled {
            return 0.0;
        }

        let first_char = first_box_text
            .chars()
            .find(|&c| !c.is_whitespace() && !Self::is_combining_mark(c));
        let last_char = if break_is_flagged_hyphen {
            Some(if is_rtl { '\u{05BE}' } else { '-' })
        } else {
            last_box_text
                .chars()
                .rev()
                .find(|&c| !c.is_whitespace() && !Self::is_combining_mark(c))
        };

        let avg_char_w = (font_size_pt * 0.45).max(2.0);
        let start_factor = first_char
            .map(|c| {
                let (lf, rf) = Self::protrusion_factors_for_char(c, config);
                if is_rtl {
                    rf
                } else {
                    lf
                }
            })
            .unwrap_or(0.0);
        let end_factor = last_char
            .map(|c| {
                let (lf, rf) = Self::protrusion_factors_for_char(c, config);
                if is_rtl {
                    lf
                } else {
                    rf
                }
            })
            .unwrap_or(0.0);

        (start_factor + end_factor) * avg_char_w
    }

    /// Calculates the effective justification target width and starting X coordinate for a line
    /// with optical margin alignment applied.
    ///
    /// Returns `(effective_target_width, line_start_x_adjustment)` where:
    /// - For a justified line (`!is_last` and `alignment == TextAlignment::Justified`),
    ///   `effective_target_width = target_width + left_pt + right_pt`, and the line starts at
    ///   `x_offset - left_pt` so the left glyph hangs left by `left_pt` and the right glyph hangs
    ///   right by `right_pt`.
    /// - For a ragged-left RTL line (`is_last` in RTL or `TextAlignment::Right`),
    ///   the right edge protrudes by `+right_pt`.
    /// - For a ragged-right LTR line (`is_last` in LTR or `TextAlignment::Left`),
    ///   the left edge protrudes by `-left_pt`.
    pub fn adjust_line_geometry(
        target_width: f32,
        x_offset: f32,
        is_rtl: bool,
        is_last: bool,
        alignment: TextAlignment,
        protrusion: OpticalProtrusion,
    ) -> (f32, f32, f32) {
        let is_justified = alignment == TextAlignment::Justified && !is_last;
        if is_justified {
            let eff_target = (target_width + protrusion.left_pt + protrusion.right_pt).max(1.0);
            let eff_x_offset = x_offset - protrusion.left_pt;
            (eff_target, eff_x_offset, protrusion.right_pt)
        } else {
            match alignment {
                TextAlignment::Right => (
                    target_width,
                    x_offset + protrusion.right_pt,
                    protrusion.right_pt,
                ),
                TextAlignment::Left => (target_width, x_offset - protrusion.left_pt, 0.0),
                TextAlignment::Center => (
                    target_width,
                    x_offset + (protrusion.right_pt - protrusion.left_pt) * 0.5,
                    0.0,
                ),
                TextAlignment::Justified => {
                    // Last line of a justified paragraph aligns to the start edge
                    if is_rtl {
                        (
                            target_width,
                            x_offset + protrusion.right_pt,
                            protrusion.right_pt,
                        )
                    } else {
                        (target_width, x_offset - protrusion.left_pt, 0.0)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(ch: char, width: f32) -> PositionedGlyph {
        PositionedGlyph {
            glyph_id: ch as u32,
            cluster: 0,
            x_advance: width,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some(ch),
        }
    }

    #[test]
    fn test_hebrew_punctuation_and_maqaf_protrusion() {
        let cfg = OpticalMarginConfig::default();
        // Visual order in RTL for "בראשית־" (where maqaf is at left edge and ב is at right edge):
        // [ '־', 'ת', 'י', 'ש', 'א', 'ר', 'ב' ]
        let glyphs = vec![
            glyph('\u{05BE}', 4.0), // Maqaf on visual left
            glyph('ת', 8.0),
            glyph('י', 4.0),
            glyph('ש', 9.0),
            glyph('א', 8.0),
            glyph('ר', 7.0),
            glyph('ב', 8.0), // Bet on visual right
        ];

        let prot = OpticalMarginAligner::compute_line_protrusion(&glyphs, 12.0, &cfg);
        // Maqaf on left has 0.65 factor -> 4.0 * 0.65 = 2.60pt
        assert!(
            (prot.left_pt - 2.60).abs() < 1e-2,
            "left_pt={}",
            prot.left_pt
        );
        // Bet on right has 0.04 factor -> 8.0 * 0.04 = 0.32pt
        assert!(
            prot.right_pt > 0.1 && prot.right_pt < 1.0,
            "right_pt={}",
            prot.right_pt
        );
    }

    #[test]
    fn test_combining_marks_do_not_mask_edge_glyph_protrusion() {
        let cfg = OpticalMarginConfig::default();
        // Visual order where rightmost base letter 'ד' is followed by zero-advance niqqud/taamim
        let glyphs = vec![
            glyph('.', 3.0),
            glyph('א', 8.0),
            glyph('ד', 8.0),
            glyph('\u{05B8}', 0.0), // Qamats (zero advance)
            glyph('\u{0596}', 0.0), // Tifcha (zero advance)
        ];

        let prot = OpticalMarginAligner::compute_line_protrusion(&glyphs, 12.0, &cfg);
        assert!((prot.left_pt - 3.0 * 0.60).abs() < 1e-2);
        // Rightmost spacing glyph is 'ד' (0.08 * 8.0 = 0.64pt), NOT masked by Qamats/Tifcha
        assert!((prot.right_pt - 8.0 * 0.08).abs() < 1e-2);
    }

    #[test]
    fn test_adjust_line_geometry_justified_and_ragged_rtl() {
        let prot = OpticalProtrusion {
            left_pt: 2.5,
            right_pt: 1.5,
            left_factor: 0.6,
            right_factor: 0.4,
        };

        // Justified line on 200pt column at x_offset = 0:
        // Effective target = 200 + 2.5 + 1.5 = 204.0, starting at -2.5 (so ends at -2.5 + 204.0 = 201.5)
        let (eff_w, eff_x, _) = OpticalMarginAligner::adjust_line_geometry(
            200.0,
            0.0,
            true,
            false,
            TextAlignment::Justified,
            prot,
        );
        assert!((eff_w - 204.0).abs() < 1e-3);
        assert!((eff_x - (-2.5)).abs() < 1e-3);
        assert!((eff_x + eff_w - 201.5).abs() < 1e-3);

        // Ragged last line in RTL on 200pt column at x_offset = 0:
        // Right edge overhangs by +1.5pt
        let (eff_w_last, eff_x_last, _) = OpticalMarginAligner::adjust_line_geometry(
            200.0,
            0.0,
            true,
            true,
            TextAlignment::Justified,
            prot,
        );
        assert!((eff_w_last - 200.0).abs() < 1e-3);
        assert!((eff_x_last - 1.5).abs() < 1e-3);
    }
}

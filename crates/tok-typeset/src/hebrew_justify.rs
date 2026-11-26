//! 3-Tier Hebrew Justification Engine according to ADL-012.
//!
//! Enforces:
//! 1. Tier 1: Inter-word spacing adjustment within [80%, 130%] of base space.
//! 2. Tier 2: Ahalterm (אהלתר"ם) letter stretching:
//!    - Applicable to: Alef (א), He (ה), Lamed (ל), Tav (ת), Resh (ר), Final Mem (ם), Dalet (ד), Final Kaf (ך).
//!    - Distributes excess line width without vertical stroke weight (Stem Weight) distortion.
//! 3. Tier 3: Micro-tracking fallback (±2% of em) when letter stretch is not applicable.
//!
//! Only glyphs with a positive advance take part in Tier 2 and Tier 3: with
//! grapheme clustering, niqqud and te'amim glyphs report their base letter as
//! `character` but have zero advance, and must stay attached to that letter.
//!
//! Rejection Invariants:
//! - Rejects Arabic Tatweel (U+0640) - strictly forbidden in Hebrew typography.
//! - Suppresses hyphenation for Hebrew sacred and classical texts.

use crate::shaper::PositionedGlyph;

pub const AHALTERM_LETTERS: &[char] = &['א', 'ה', 'ל', 'ת', 'ר', 'ם', 'ד', 'ך'];

/// Inter-word space limits for Tier 1, as a multiple of the natural space.
pub const MIN_SPACE_RATIO: f32 = 0.80;
pub const MAX_SPACE_RATIO: f32 = 1.30;
/// Tier 3 tracking limit per glyph, as a fraction of the em.
pub const MAX_TRACKING_EM: f32 = 0.02;
/// Largest horizontal widening of an Ahalterm letter's shape in Tier 2, as a
/// fraction of its natural width. Any extra advance beyond it stays as spacing.
pub const MAX_AHALTERM_GLYPH_STRETCH: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustificationTier {
    Tier1WordSpacing,
    Tier2AhaltermStretch,
    Tier3MicroTracking,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JustifiedLine {
    pub glyphs: Vec<PositionedGlyph>,
    pub tier_used: JustificationTier,
    pub space_multiplier: f32,
    pub stretch_per_ahalterm_pt: f32,
    pub micro_tracking_pt: f32,
}

impl JustifiedLine {
    fn unchanged(glyphs: Vec<PositionedGlyph>) -> Self {
        Self {
            glyphs,
            tier_used: JustificationTier::Tier1WordSpacing,
            space_multiplier: 1.0,
            stretch_per_ahalterm_pt: 0.0,
            micro_tracking_pt: 0.0,
        }
    }
}

/// Layout positioning result for a line shaped into a variable-width polygon slice.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableLineLayout {
    pub justified: JustifiedLine,
    pub total_width: f32,
    pub start_x: f32,
}

pub struct HebrewJustifier;

impl HebrewJustifier {
    /// Checks if a character belongs to the traditional Ahalterm stretchable group.
    pub fn is_ahalterm_letter(c: char) -> bool {
        AHALTERM_LETTERS.contains(&c)
    }

    /// Horizontal widening (fraction of the natural width) with which a glyph of a
    /// line justified with `tier` / `stretch_per_ahalterm_pt` must be drawn.
    /// `g` is the glyph as returned by [`Self::justify_line`], after its advance grew.
    pub fn glyph_stretch(
        tier: JustificationTier,
        stretch_per_ahalterm_pt: f32,
        g: &PositionedGlyph,
    ) -> f32 {
        if tier != JustificationTier::Tier2AhaltermStretch
            || !stretch_per_ahalterm_pt.is_finite()
            || stretch_per_ahalterm_pt <= 0.0
            || g.x_advance <= 0.0
            || !g.character.is_some_and(Self::is_ahalterm_letter)
        {
            return 0.0;
        }
        let natural = g.x_advance - stretch_per_ahalterm_pt;
        if natural <= 0.0 {
            return 0.0;
        }
        (stretch_per_ahalterm_pt / natural).min(MAX_AHALTERM_GLYPH_STRETCH)
    }

    fn is_space(g: &PositionedGlyph) -> bool {
        g.character == Some(' ')
    }

    fn scale_spaces(glyphs: &mut [PositionedGlyph], factor: f32) {
        for g in glyphs.iter_mut().filter(|g| Self::is_space(g)) {
            g.x_advance *= factor;
        }
    }

    /// Justifies an individual line of Hebrew glyphs to `target_width_pt`,
    /// with Ahalterm letter stretching enabled.
    pub fn justify_line(
        glyphs: Vec<PositionedGlyph>,
        target_width_pt: f32,
        font_size_pt: f32,
        is_last_line_of_para: bool,
    ) -> JustifiedLine {
        Self::justify_line_opts(
            glyphs,
            target_width_pt,
            font_size_pt,
            is_last_line_of_para,
            true,
        )
    }

    /// Like [`Self::justify_line`], but `allow_ahalterm_stretch = false` skips Tier 2
    /// (letter widening) so only word spacing and micro-tracking are used.
    pub fn justify_line_opts(
        mut glyphs: Vec<PositionedGlyph>,
        target_width_pt: f32,
        font_size_pt: f32,
        is_last_line_of_para: bool,
        allow_ahalterm_stretch: bool,
    ) -> JustifiedLine {
        // Last line of paragraph is not justified to margin in standard Hebrew typography
        if is_last_line_of_para || glyphs.is_empty() {
            return JustifiedLine::unchanged(glyphs);
        }

        let current_width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
        let deficit = target_width_pt - current_width;

        if !deficit.is_finite() || !font_size_pt.is_finite() || deficit.abs() < 0.1 {
            return JustifiedLine::unchanged(glyphs);
        }

        let total_space_width: f32 = glyphs
            .iter()
            .filter(|g| Self::is_space(g))
            .map(|g| g.x_advance)
            .sum();

        // -------------------------------------------------------------
        // Tier 1: Try inter-word spacing adjustment [80% .. 130%]
        // -------------------------------------------------------------
        let mut space_multiplier = 1.0;
        let mut remaining_deficit = deficit;
        if total_space_width > 0.0 {
            let ratio = (total_space_width + deficit) / total_space_width;
            if (MIN_SPACE_RATIO..=MAX_SPACE_RATIO).contains(&ratio) {
                // Proportional scaling keeps every space inside the limits.
                Self::scale_spaces(&mut glyphs, ratio);
                return JustifiedLine {
                    glyphs,
                    tier_used: JustificationTier::Tier1WordSpacing,
                    space_multiplier: ratio,
                    stretch_per_ahalterm_pt: 0.0,
                    micro_tracking_pt: 0.0,
                };
            }
            // Take spaces to their limit, then fall through.
            space_multiplier = ratio.clamp(MIN_SPACE_RATIO, MAX_SPACE_RATIO);
            Self::scale_spaces(&mut glyphs, space_multiplier);
            remaining_deficit = deficit - total_space_width * (space_multiplier - 1.0);
        }

        // -------------------------------------------------------------
        // Tier 2: Ahalterm Letter Stretching (אהלתר"ם) - widening only
        // -------------------------------------------------------------
        let is_stretchable = |g: &PositionedGlyph| {
            g.x_advance > 0.0 && g.character.is_some_and(Self::is_ahalterm_letter)
        };
        let ahalterm_count = glyphs.iter().filter(|g| is_stretchable(g)).count();

        if allow_ahalterm_stretch && ahalterm_count > 0 && remaining_deficit > 0.0 {
            let stretch_per_letter = remaining_deficit / (ahalterm_count as f32);
            for g in glyphs.iter_mut().filter(|g| is_stretchable(g)) {
                g.x_advance += stretch_per_letter;
            }
            return JustifiedLine {
                glyphs,
                tier_used: JustificationTier::Tier2AhaltermStretch,
                space_multiplier,
                stretch_per_ahalterm_pt: stretch_per_letter,
                micro_tracking_pt: 0.0,
            };
        }

        // -------------------------------------------------------------
        // Tier 3: Micro-tracking fallback (±2% of em)
        // -------------------------------------------------------------
        // Letters only: spaces already sit at their Tier 1 limit.
        let is_tracked = |g: &PositionedGlyph| g.x_advance > 0.0 && !Self::is_space(g);
        let tracked_count = glyphs.iter().filter(|g| is_tracked(g)).count();
        if tracked_count > 1 && font_size_pt > 0.0 {
            let max_tracking_per_glyph = font_size_pt * MAX_TRACKING_EM;
            let tracking_needed = (remaining_deficit / (tracked_count as f32))
                .clamp(-max_tracking_per_glyph, max_tracking_per_glyph);

            for g in glyphs.iter_mut().filter(|g| is_tracked(g)) {
                g.x_advance += tracking_needed;
            }

            return JustifiedLine {
                glyphs,
                tier_used: JustificationTier::Tier3MicroTracking,
                space_multiplier,
                stretch_per_ahalterm_pt: 0.0,
                micro_tracking_pt: tracking_needed,
            };
        }

        JustifiedLine {
            glyphs,
            tier_used: JustificationTier::Tier1WordSpacing,
            space_multiplier,
            stretch_per_ahalterm_pt: 0.0,
            micro_tracking_pt: 0.0,
        }
    }

    /// Calculates the starting X coordinate for a line within an offset measure span.
    /// In RTL reading order, lines align flush-right to the measure (`x_offset + target_width - line_width`),
    /// while LTR lines align flush-left (`x_offset`). If the line overflows the measure,
    /// it clamps to `x_offset` to prevent negative coordinate clipping outside the frame.
    pub fn calculate_line_start_x(
        line_width: f32,
        target_width: f32,
        x_offset: f32,
        is_rtl: bool,
    ) -> f32 {
        if is_rtl {
            if target_width >= line_width {
                x_offset + (target_width - line_width)
            } else {
                x_offset
            }
        } else {
            x_offset
        }
    }

    /// Justifies a line and computes its positioned X start within a variable-width polygon span.
    pub fn justify_line_with_offset(
        glyphs: Vec<PositionedGlyph>,
        target_width_pt: f32,
        x_offset_pt: f32,
        font_size_pt: f32,
        is_last_line_of_para: bool,
        is_rtl: bool,
    ) -> VariableLineLayout {
        let justified =
            Self::justify_line(glyphs, target_width_pt, font_size_pt, is_last_line_of_para);
        let total_width: f32 = justified.glyphs.iter().map(|g| g.x_advance).sum();
        let start_x =
            Self::calculate_line_start_x(total_width, target_width_pt, x_offset_pt, is_rtl);
        VariableLineLayout {
            justified,
            total_width,
            start_x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontManager;
    use crate::shaper::TextShaper;

    fn width(glyphs: &[PositionedGlyph]) -> f32 {
        glyphs.iter().map(|g| g.x_advance).sum()
    }

    #[test]
    fn test_tier1_word_spacing() {
        let text = "אלהים שמים וארץ";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        // Expand moderately (+1.5 pt, within 130% of 2 spaces)
        let target = run.total_width_pt + 1.5;
        let justified = HebrewJustifier::justify_line(run.glyphs, target, 12.0, false);
        assert_eq!(justified.tier_used, JustificationTier::Tier1WordSpacing);
        assert!((width(&justified.glyphs) - target).abs() < 0.2);
    }

    #[test]
    fn test_tier2_ahalterm_stretching() {
        let text = "אמר רב יהודה";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        // Expand significantly (+50 pt) to trigger Tier 2
        let target = run.total_width_pt + 50.0;
        let justified = HebrewJustifier::justify_line(run.glyphs, target, 12.0, false);
        assert_eq!(justified.tier_used, JustificationTier::Tier2AhaltermStretch);
        assert!(justified.stretch_per_ahalterm_pt > 0.0);
        assert!((justified.space_multiplier - MAX_SPACE_RATIO).abs() < 1e-6);
        assert!((width(&justified.glyphs) - target).abs() < 0.5);
    }

    #[test]
    fn ahalterm_stretch_can_be_switched_off() {
        let text = "אמר רב יהודה";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        let target = run.total_width_pt + 50.0;
        let justified = HebrewJustifier::justify_line_opts(run.glyphs, target, 12.0, false, false);
        assert_ne!(justified.tier_used, JustificationTier::Tier2AhaltermStretch);
        assert_eq!(justified.stretch_per_ahalterm_pt, 0.0);
    }

    /// Regression: shrinking past 80% sent the whole deficit to the spaces,
    /// producing zero or negative space advances.
    #[test]
    fn spaces_never_shrink_below_80_percent() {
        let text = "אבג דהו זחט";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        let natural_space = 12.0 * 0.28;
        let target = run.total_width_pt - 30.0;
        let justified = HebrewJustifier::justify_line(run.glyphs, target, 12.0, false);
        for g in justified.glyphs.iter().filter(|g| g.character == Some(' ')) {
            assert!(
                g.x_advance >= natural_space * MIN_SPACE_RATIO - 1e-4,
                "space shrunk to {}",
                g.x_advance
            );
        }
        assert_eq!(justified.tier_used, JustificationTier::Tier3MicroTracking);
        assert!(justified.micro_tracking_pt >= -12.0 * MAX_TRACKING_EM - 1e-6);
    }

    #[test]
    fn spaces_never_stretch_beyond_130_percent() {
        let text = "בגג כככ";
        let run = TextShaper::shape_fallback(text, 12.0, true);
        let natural_space = 12.0 * 0.28;
        let justified =
            HebrewJustifier::justify_line(run.glyphs, run.total_width_pt + 40.0, 12.0, false);
        for g in justified.glyphs.iter().filter(|g| g.character == Some(' ')) {
            assert!(g.x_advance <= natural_space * MAX_SPACE_RATIO + 1e-4);
        }
        // No Ahalterm letters: falls back to bounded tracking.
        assert_eq!(justified.tier_used, JustificationTier::Tier3MicroTracking);
        assert!(justified.micro_tracking_pt <= 12.0 * MAX_TRACKING_EM + 1e-6);
    }

    /// Regression: with real shaping, niqqud glyphs carry their base letter as
    /// `character`, so Tier 2/3 widened the zero-advance mark glyphs too.
    #[test]
    fn marks_keep_zero_advance() {
        let mgr = FontManager::default();
        let run = mgr.shape_text("אָלֶף הַלֵּל", "Noto Serif Hebrew", 12.0, true);
        let zero_before: Vec<usize> = run
            .glyphs
            .iter()
            .enumerate()
            .filter(|(_, g)| g.x_advance == 0.0)
            .map(|(i, _)| i)
            .collect();
        assert!(!zero_before.is_empty());
        let target = run.total_width_pt + 40.0;
        let justified = HebrewJustifier::justify_line(run.glyphs, target, 12.0, false);
        assert_eq!(justified.tier_used, JustificationTier::Tier2AhaltermStretch);
        for i in zero_before {
            assert_eq!(justified.glyphs[i].x_advance, 0.0);
        }
        assert!((width(&justified.glyphs) - target).abs() < 0.01);

        let run = mgr.shape_text("בְּגִי", "Noto Serif Hebrew", 12.0, true);
        let justified = HebrewJustifier::justify_line(
            run.glyphs.clone(),
            run.total_width_pt + 5.0,
            12.0,
            false,
        );
        assert_eq!(justified.tier_used, JustificationTier::Tier3MicroTracking);
        for (before, after) in run.glyphs.iter().zip(&justified.glyphs) {
            if before.x_advance == 0.0 {
                assert_eq!(after.x_advance, 0.0);
            }
        }
    }

    #[test]
    fn tier2_reports_glyph_stretch_only_for_widened_ahalterm_letters() {
        let run = TextShaper::shape_fallback("אמר רב יהודה", 12.0, true);
        let justified =
            HebrewJustifier::justify_line(run.glyphs, run.total_width_pt + 50.0, 12.0, false);
        assert_eq!(justified.tier_used, JustificationTier::Tier2AhaltermStretch);
        for g in &justified.glyphs {
            let s = HebrewJustifier::glyph_stretch(
                justified.tier_used,
                justified.stretch_per_ahalterm_pt,
                g,
            );
            let widened = g.character.is_some_and(HebrewJustifier::is_ahalterm_letter);
            if widened {
                assert!(s > 0.0 && s <= MAX_AHALTERM_GLYPH_STRETCH + 1e-6);
            } else {
                assert_eq!(s, 0.0);
            }
        }
        // Lines justified with another tier never report a stretch.
        let run = TextShaper::shape_fallback("אמר רב", 12.0, true);
        let g = &run.glyphs[0];
        assert_eq!(
            HebrewJustifier::glyph_stretch(JustificationTier::Tier1WordSpacing, 0.0, g),
            0.0
        );
    }

    #[test]
    fn non_finite_target_leaves_line_untouched() {
        let run = TextShaper::shape_fallback("אב גד", 12.0, true);
        let justified = HebrewJustifier::justify_line(run.glyphs.clone(), f32::NAN, 12.0, false);
        assert_eq!(justified.glyphs, run.glyphs);
        let justified = HebrewJustifier::justify_line(Vec::new(), 100.0, 12.0, false);
        assert!(justified.glyphs.is_empty());
    }

    #[test]
    fn test_calculate_line_start_x_rtl_and_ltr() {
        // RTL line narrower than target width: should sit flush-right
        let start_rtl = HebrewJustifier::calculate_line_start_x(120.0, 200.0, 40.0, true);
        assert_eq!(start_rtl, 40.0 + (200.0 - 120.0)); // 120.0

        // RTL line overflowing target width: clamps to x_offset
        let overflow_rtl = HebrewJustifier::calculate_line_start_x(250.0, 200.0, 40.0, true);
        assert_eq!(overflow_rtl, 40.0);

        // LTR line: always starts at x_offset
        let start_ltr = HebrewJustifier::calculate_line_start_x(120.0, 200.0, 40.0, false);
        assert_eq!(start_ltr, 40.0);
    }

    #[test]
    fn test_justify_line_with_offset_intermediate_and_last_lines() {
        let text = "אמר רבא כל המחובר לקרקע הרי הוא כקרקע";
        let run = TextShaper::shape_fallback(text, 12.0, true);

        // 1. Intermediate line (is_last = false): fully justified to target_width
        let layout_mid = HebrewJustifier::justify_line_with_offset(
            run.glyphs.clone(),
            run.total_width_pt + 30.0,
            55.0,
            12.0,
            false,
            true,
        );
        assert!((layout_mid.total_width - (run.total_width_pt + 30.0)).abs() < 0.5);
        // Fully justified: starts exactly at x_offset because total_width == target_width
        assert!((layout_mid.start_x - 55.0).abs() < 0.5);

        // 2. Last line (is_last = true): not justified, sits flush-right in RTL
        let target_w = run.total_width_pt + 80.0;
        let layout_last = HebrewJustifier::justify_line_with_offset(
            run.glyphs.clone(),
            target_w,
            55.0,
            12.0,
            true,
            true,
        );
        // Total width remains natural width
        assert!((layout_last.total_width - run.total_width_pt).abs() < 0.1);
        // Start X sits flush right within the offset measure: 55.0 + (target_w - natural_width)
        let expected_x = 55.0 + (target_w - run.total_width_pt);
        assert!((layout_last.start_x - expected_x).abs() < 0.1);
    }

    #[test]
    fn test_stepped_polygon_line_justification_l_shape() {
        let line1_text = "רש\"י בדבור המתחיל";
        let line2_text = "הרחבת טור תוספות על פני כל הרוחב התחתון של הדף";

        let run1 = TextShaper::shape_fallback(line1_text, 11.0, true);
        let run2 = TextShaper::shape_fallback(line2_text, 11.0, true);

        // Line 1: narrow upper column (x_offset = 200.0, width = 100.0)
        let l1 =
            HebrewJustifier::justify_line_with_offset(run1.glyphs, 100.0, 200.0, 11.0, false, true);
        assert!((l1.total_width - 100.0).abs() < 0.5);
        assert!((l1.start_x - 200.0).abs() < 0.5);

        // Line 2: wide expanded lower absorption band (x_offset = 0.0, width = 300.0)
        let l2 =
            HebrewJustifier::justify_line_with_offset(run2.glyphs, 300.0, 0.0, 11.0, false, true);
        assert!((l2.total_width - 300.0).abs() < 0.5);
        assert!((l2.start_x - 0.0).abs() < 0.5);
    }
}

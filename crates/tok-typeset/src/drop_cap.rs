//! Hebrew Drop Cap and Opening Word Shaper ("חלון במילת פתיחה").
//!
//! Shapes large, bold opening words (including Niqqud and Ta'amim),
//! calculates their exact visual bounding box, and synthesizes the
//! rectangular/polygonal exclusion zone around them.

use crate::font::{FontManager, DEFAULT_FAMILY};
use crate::geometry::{GlyphBox, PhysicalRect};
use crate::polygon::ExclusionZone;
use tok_core::styles::{DropCapConfig, FONT_WEIGHT_BOLD};

/// Layout and geometry result for an opening word / drop cap window.
#[derive(Debug, Clone, PartialEq)]
pub struct DropCapLayout {
    /// Opening text extracted (e.g. "בְּרֵאשִׁית", "מֵאֵימָתַי").
    pub text: String,
    /// UTF-8 byte offset of `text` within the original paragraph string.
    pub opening_byte_offset: usize,
    /// Remainder of the paragraph text following the opening word.
    pub remaining_text: String,
    /// UTF-8 byte offset of `remaining_text` within the original paragraph string.
    pub remaining_byte_offset: usize,
    /// Positioned glyphs of the drop cap word within the window (`y` is `y_offset` relative to `baseline_y`).
    pub glyphs: Vec<GlyphBox>,
    /// Calculated font size in points used for the drop cap.
    pub font_size_pt: f32,
    /// Baseline Y coordinate of the drop cap inside `window_rect`.
    pub baseline_y: f32,
    /// Registered font family name that shaped `glyphs` (after weight and fallback resolution).
    pub font_family: String,
    /// Exact physical ink bounding rectangle enclosing all glyph outlines (base letters + niqqud + ta'amim).
    pub ink_bounds: PhysicalRect,
    /// Bounding rectangle of the window (in frame coordinates).
    pub window_rect: PhysicalRect,
    /// Geometric exclusion zone used to notch indented body lines.
    pub exclusion: ExclusionZone,
}

pub struct HebrewDropCapShaper;

impl HebrewDropCapShaper {
    #[inline]
    fn is_hebrew_or_unicode_combining(c: char) -> bool {
        ('\u{0591}'..='\u{05BD}').contains(&c)
            || ('\u{05BF}'..='\u{05C7}').contains(&c)
            || c == '\u{FB1E}'
            || ('\u{0300}'..='\u{036F}').contains(&c)
    }

    /// Splits paragraph text into `(opening_text, opening_byte_offset, remaining_text, remaining_byte_offset)`.
    pub fn split_opening_text_with_offsets(
        text: &str,
        config: &DropCapConfig,
    ) -> (String, usize, String, usize) {
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            return (String::new(), 0, String::new(), text.len());
        }
        let leading_ws = text.len() - trimmed.len();

        let raw_split_pos = if config.whole_word {
            trimmed
                .find(|c: char| c.is_whitespace())
                .unwrap_or(trimmed.len())
        } else {
            let target_chars = config.characters_count.max(1);
            let mut count = 0;
            let mut split_pos = trimmed.len();

            for (byte_offset, c) in trimmed.char_indices() {
                let is_combining = Self::is_hebrew_or_unicode_combining(c);
                if c.is_whitespace() && count > 0 {
                    split_pos = byte_offset;
                    break;
                }
                if !is_combining && !c.is_whitespace() {
                    count += 1;
                    if count > target_chars {
                        split_pos = byte_offset;
                        break;
                    }
                }
            }
            split_pos
        };

        let opening_slice = trimmed[..raw_split_pos].trim();
        let after_slice = &trimmed[raw_split_pos..];
        let remaining_trimmed = after_slice.trim_start();
        let rem_ws = after_slice.len() - remaining_trimmed.len();
        let remaining_byte_offset = leading_ws + raw_split_pos + rem_ws;

        (
            opening_slice.to_string(),
            leading_ws,
            remaining_trimmed.to_string(),
            remaining_byte_offset,
        )
    }

    /// Splits paragraph text into the opening word/letters and the remaining body text.
    pub fn split_opening_text(text: &str, config: &DropCapConfig) -> (String, String) {
        let (opening, _, remaining, _) = Self::split_opening_text_with_offsets(text, config);
        (opening, remaining)
    }

    /// Computes raw glyph outline extrema `(min_x, max_x, top_rel_baseline, bottom_rel_baseline)`
    /// in points relative to `(pen_start_x = 0, baseline_y = 0)`.
    fn compute_outline_extrema(
        glyphs: &[crate::shaper::PositionedGlyph],
        total_advance_pt: f32,
        font_size_pt: f32,
        font_bytes: Option<&[u8]>,
    ) -> (f32, f32, f32, f32) {
        let face = font_bytes.and_then(|bytes| ttf_parser::Face::parse(bytes, 0).ok());
        let upem = face
            .as_ref()
            .map(|f| f.units_per_em().max(1) as f32)
            .unwrap_or(1000.0);
        let scale = font_size_pt / upem;

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y_rel = f32::INFINITY; // negative = above baseline
        let mut max_y_rel = f32::NEG_INFINITY; // positive = below baseline

        let mut pen_x = 0.0f32;
        for g in glyphs {
            let gx = pen_x + g.x_offset;
            if let Some(bbox) = face
                .as_ref()
                .and_then(|f| f.glyph_bounding_box(ttf_parser::GlyphId(g.glyph_id as u16)))
            {
                let x0 = gx + (bbox.x_min as f32) * scale;
                let x1 = gx + (bbox.x_max as f32) * scale;
                // In OpenType/TrueType, +Y is up from baseline; in frame space, +Y is down.
                let y_top = -g.y_offset - (bbox.y_max as f32) * scale;
                let y_bot = -g.y_offset - (bbox.y_min as f32) * scale;
                min_x = min_x.min(x0);
                max_x = max_x.max(x1);
                min_y_rel = min_y_rel.min(y_top);
                max_y_rel = max_y_rel.max(y_bot);
            } else {
                let x0 = gx;
                let x1 = gx + g.x_advance.max(font_size_pt * 0.25);
                let y_top = -g.y_offset - font_size_pt * 0.78;
                let y_bot = -g.y_offset + font_size_pt * 0.22;
                min_x = min_x.min(x0);
                max_x = max_x.max(x1);
                min_y_rel = min_y_rel.min(y_top);
                max_y_rel = max_y_rel.max(y_bot);
            }
            pen_x += g.x_advance;
        }

        if !min_x.is_finite() {
            min_x = 0.0;
        }
        if !max_x.is_finite() {
            max_x = total_advance_pt;
        }
        if !min_y_rel.is_finite() {
            min_y_rel = -font_size_pt * 0.78;
        }
        if !max_y_rel.is_finite() {
            max_y_rel = font_size_pt * 0.22;
        }

        (min_x, max_x, min_y_rel, max_y_rel)
    }

    /// Shapes the opening word and computes its exact window geometry and exclusion zone.
    pub fn shape_drop_cap(
        text: &str,
        config: &DropCapConfig,
        font_manager: &FontManager,
        column_width_pt: f32,
        line_height_pt: f32,
        is_rtl: bool,
    ) -> Option<DropCapLayout> {
        if !config.enabled || text.trim().is_empty() || column_width_pt <= 0.0 {
            return None;
        }

        let (opening_text, opening_byte_offset, remaining_text, remaining_byte_offset) =
            Self::split_opening_text_with_offsets(text, config);
        if opening_text.is_empty() {
            return None;
        }

        let requested_family = config.font_family.as_deref().unwrap_or(DEFAULT_FAMILY);
        let lines_span = config.lines.max(1);
        let window_height = (lines_span as f32) * line_height_pt.max(1.0);

        let mut font_size = config
            .font_size_pt
            .unwrap_or_else(|| (line_height_pt * (lines_span as f32 - 0.15)).max(16.0));

        // Resolve requested weight (default 700 bold)
        let font_face_name =
            font_manager.face_for(requested_family, config.font_weight.max(FONT_WEIGHT_BOLD));
        let session = font_manager.session(&font_face_name);

        let (mut run, mut choice) = session.shape(&opening_text, font_size, is_rtl);
        let mut shaped_family = session
            .family_of(choice)
            .unwrap_or(&font_face_name)
            .to_string();
        let mut font_bytes = font_manager
            .get_font(&shaped_family)
            .map(|f| f.raw_bytes.clone());

        let (mut raw_min_x, mut raw_max_x, mut raw_top_rel, mut raw_bot_rel) =
            Self::compute_outline_extrema(
                &run.glyphs,
                run.total_width_pt,
                font_size,
                font_bytes.as_deref().map(|v| v.as_slice()),
            );

        // If font_size was auto-computed and the vertical outline span (including upper/lower
        // niqqud and ta'amim) exceeds 94% of window_height, or horizontal ink exceeds 75% of column,
        // scale font_size down proportionally so glyph outlines never clip outside the window.
        let vertical_span = (raw_bot_rel - raw_top_rel).max(1.0);
        let horizontal_span = (raw_max_x.max(run.total_width_pt) - raw_min_x.min(0.0)).max(1.0);
        let max_allowed_h = window_height * 0.94;
        let max_allowed_w = (column_width_pt - config.margin_pt.max(0.0) - 24.0).max(20.0);

        let v_scale = if config.font_size_pt.is_none() && vertical_span > max_allowed_h {
            max_allowed_h / vertical_span
        } else {
            1.0
        };
        let h_scale = if horizontal_span > max_allowed_w {
            max_allowed_w / horizontal_span
        } else {
            1.0
        };
        let fit_scale = v_scale.min(h_scale);
        if fit_scale < 0.995 {
            font_size = (font_size * fit_scale).max(8.0);
            let reshaped = session.shape(&opening_text, font_size, is_rtl);
            run = reshaped.0;
            choice = reshaped.1;
            shaped_family = session
                .family_of(choice)
                .unwrap_or(&font_face_name)
                .to_string();
            font_bytes = font_manager
                .get_font(&shaped_family)
                .map(|f| f.raw_bytes.clone());
            let ext = Self::compute_outline_extrema(
                &run.glyphs,
                run.total_width_pt,
                font_size,
                font_bytes.as_deref().map(|v| v.as_slice()),
            );
            raw_min_x = ext.0;
            raw_max_x = ext.1;
            raw_top_rel = ext.2;
            raw_bot_rel = ext.3;
        }

        let left_overhang = (-raw_min_x).max(0.0);
        let right_extent = raw_max_x.max(run.total_width_pt);
        let ink_span_x = (right_extent + left_overhang).max(run.total_width_pt);

        let margin = config.margin_pt.max(0.0);
        let window_width = (ink_span_x + margin).min(column_width_pt);

        let (window_x, glyph_start_x) = if is_rtl {
            let win_x = (column_width_pt - window_width).max(0.0);
            let gl_x = win_x + margin + left_overhang;
            (win_x, gl_x)
        } else {
            let win_x = 0.0;
            let gl_x = left_overhang;
            (win_x, gl_x)
        };

        // Nominal baseline aligns with the N-th body line's baseline (`window_height - line_height_pt * 0.22`),
        // clamped so neither upper marks (`baseline_y + raw_top_rel >= 0`) nor lower marks (`baseline_y + raw_bot_rel <= window_height`) clip.
        let nominal_baseline = window_height - (line_height_pt * 0.22);
        let max_baseline = (window_height - raw_bot_rel.max(0.0)).max(0.0);
        let min_baseline = (-raw_top_rel).max(0.0).min(max_baseline);
        let baseline_y = nominal_baseline.clamp(min_baseline, max_baseline);

        let mut curr_x = glyph_start_x;
        let glyph_boxes: Vec<GlyphBox> = run
            .glyphs
            .into_iter()
            .map(|g| {
                let gb = GlyphBox {
                    glyph_id: g.glyph_id,
                    cluster: g.cluster + opening_byte_offset as u32,
                    x: curr_x + g.x_offset,
                    y: g.y_offset,
                    width: g.x_advance,
                    height: font_size,
                    character: g.character,
                    font_index: 0,
                };
                curr_x += g.x_advance;
                gb
            })
            .collect();

        let ink_x = (glyph_start_x + raw_min_x).clamp(window_x, window_x + window_width);
        let ink_right = (glyph_start_x + raw_max_x.max(run.total_width_pt))
            .clamp(ink_x, window_x + window_width);
        let ink_y = (baseline_y + raw_top_rel).clamp(0.0, window_height);
        let ink_bottom = (baseline_y + raw_bot_rel).clamp(ink_y, window_height);
        let ink_bounds = PhysicalRect::new(
            ink_x,
            ink_y,
            (ink_right - ink_x).max(0.0),
            (ink_bottom - ink_y).max(0.0),
        );
        let window_rect = PhysicalRect::new(window_x, 0.0, window_width, window_height);
        let exclusion = ExclusionZone::new(window_rect, 0.0);

        Some(DropCapLayout {
            text: opening_text,
            opening_byte_offset,
            remaining_text,
            remaining_byte_offset,
            glyphs: glyph_boxes,
            font_size_pt: font_size,
            baseline_y,
            font_family: shaped_family,
            ink_bounds,
            window_rect,
            exclusion,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_opening_text_whole_word() {
        let config = DropCapConfig {
            whole_word: true,
            ..Default::default()
        };

        let (first, rem) =
            HebrewDropCapShaper::split_opening_text("בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם", &config);
        assert_eq!(first, "בְּרֵאשִׁ֖ית");
        assert_eq!(rem, "בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם");
    }

    #[test]
    fn test_split_opening_text_chars_count_with_niqqud() {
        let config = DropCapConfig {
            whole_word: false,
            characters_count: 2,
            ..Default::default()
        };

        // First 2 base letters: מ and א, keeping their vowels/dagesh attached
        let (first, rem) = HebrewDropCapShaper::split_opening_text("מֵאֵימָתַי קוֹרִין אֶת שְׁמַע", &config);
        assert_eq!(first, "מֵאֵ");
        assert_eq!(rem, "ימָתַי קוֹרִין אֶת שְׁמַע");
    }

    #[test]
    fn test_shape_drop_cap_talmud_opening_word() {
        let fm = FontManager::default();
        let config = DropCapConfig {
            enabled: true,
            lines: 3,
            whole_word: true,
            margin_pt: 7.0,
            ..Default::default()
        };

        let layout = HebrewDropCapShaper::shape_drop_cap(
            "מֵאֵימָתַ֛י קוֹרִין אֶת שְׁמַע בְּעַרְבִית",
            &config,
            &fm,
            350.0,
            16.0,
            true, // RTL
        )
        .expect("Should shape drop cap");

        assert_eq!(layout.text, "מֵאֵימָתַ֛י");
        assert_eq!(layout.remaining_text, "קוֹרִין אֶת שְׁמַע בְּעַרְבִית");
        assert!(!layout.glyphs.is_empty());

        // 3 lines * 16.0 = 48.0 window height
        assert_eq!(layout.window_rect.height, 48.0);
        // Positive ink width + margin
        assert!(layout.window_rect.width > 20.0);
        // RTL positioning: window x starts at 350.0 - window_width
        assert!((layout.window_rect.x + layout.window_rect.width - 350.0).abs() < 1e-2);
        // Exclusion zone bounds match window rect
        assert_eq!(layout.exclusion.rect, layout.window_rect);
    }

    #[test]
    fn test_shape_drop_cap_disabled_returns_none() {
        let fm = FontManager::default();
        let config = DropCapConfig {
            enabled: false,
            ..Default::default()
        };

        let result = HebrewDropCapShaper::shape_drop_cap("בְּרֵאשִׁית", &config, &fm, 300.0, 15.0, true);
        assert!(result.is_none());
    }

    #[test]
    fn test_drop_cap_outline_ink_bounds_and_cantillation_font_fallback() {
        let fm = FontManager::default();
        let config = DropCapConfig {
            enabled: true,
            lines: 3,
            whole_word: true,
            margin_pt: 6.0,
            ..Default::default()
        };

        let text = "  בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים";
        let layout = HebrewDropCapShaper::shape_drop_cap(text, &config, &fm, 300.0, 15.0, true)
            .expect("Should shape drop cap with cantillation");

        // Cantillation mark (Tipeha \u{0596}) triggers fallback from Frank Ruhl Libre Bold to Noto Serif Hebrew
        assert_eq!(layout.font_family, "Noto Serif Hebrew");
        assert_eq!(layout.opening_byte_offset, 2);
        assert_eq!(
            &text[layout.opening_byte_offset..layout.opening_byte_offset + layout.text.len()],
            "בְּרֵאשִׁ֖ית"
        );
        assert_eq!(&text[layout.remaining_byte_offset..], "בָּרָ֣א אֱלֹהִ֑ים");

        // Ink bounds must be strictly contained inside window_rect
        assert!(layout.ink_bounds.x >= layout.window_rect.x - 1e-3);
        assert!(
            layout.ink_bounds.x + layout.ink_bounds.width
                <= layout.window_rect.x + layout.window_rect.width + 1e-3
        );
        assert!(layout.ink_bounds.y >= layout.window_rect.y - 1e-3);
        assert!(
            layout.ink_bounds.y + layout.ink_bounds.height
                <= layout.window_rect.y + layout.window_rect.height + 1e-3
        );
        assert!(layout.baseline_y > 0.0 && layout.baseline_y < layout.window_rect.height);

        // Glyph y values store relative GPOS y_offset (not baseline_y + y_offset)
        for g in &layout.glyphs {
            assert!(
                g.y.abs() < layout.font_size_pt,
                "GlyphBox.y ({}) must be relative y_offset, not pre-added to baseline_y ({})",
                g.y,
                layout.baseline_y
            );
            assert!(g.cluster >= layout.opening_byte_offset as u32);
        }
    }
}

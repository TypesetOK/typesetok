//! Central Typesetting Engine for TypesetOK.
//!
//! Orchestrates text shaping, Bidi decomposition, Knuth-Plass line breaking,
//! 3-tier Hebrew justification, and multi-page convergence pagination.

use crate::font::{FontManager, ShapingSession};
use crate::footnote_paginator::{DynamicFootnotePaginator, FootnotePaginatorConfig};
use crate::gematria::GematriaEngine;
use crate::geometry::{
    BreakToken, GlyphBox, LineBox, PageLayoutBox, PhysicalRect, TextFrameBox, UNKNOWN_FONT,
};
use crate::hebrew_justify::HebrewJustifier;
use crate::knuth_plass::{KnuthPlassBreaker, LayoutItem, MeasureProfile};
use crate::layout_classifier::DocumentClassifier;
use crate::layout_family::LayoutFamily;
use crate::layout_features::DocumentLayoutFeatures;
use crate::layout_template::LayoutTemplate;
use crate::multi_flow::{FlowGeometrySpec, FlowPlacementRole, MultiFlowSolver, SpreadSide};
use crate::page_break::{GlobalPageBreaker, PaginationLineInput};
use crate::shaper::PositionedGlyph;
use crate::solver::{GeometricConstraintSolver, PageConstraintModel, PageGeometryConfig};
use crate::template_synthesizer::TemplateSynthesizer;
use rayon::prelude::*;
use std::collections::HashMap;
use tok_core::model::{DocumentRoot, Flow, FlowType, ParagraphNode};
use tok_core::FlowId;
use unicode_bidi::BidiInfo;

const DEFAULT_FONT_FAMILY: &str = crate::font::DEFAULT_FAMILY;
const DEFAULT_FONT_SIZE_PT: f32 = 11.0;
const DEFAULT_LINE_HEIGHT_PT: f32 = 14.5;
/// Natural inter-word space, in em.
const SPACE_EM: f32 = 0.28;
/// Inter-word glue may stretch to 130% and shrink to 80% (see `hebrew_justify`).
const SPACE_STRETCH: f32 = 0.30;
const SPACE_SHRINK: f32 = 0.20;
const LINE_BREAK_TOLERANCE: f32 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct TypesettingEngineConfig {
    pub page_width_pt: f32,
    pub page_height_pt: f32,
    pub margin_top_pt: f32,
    pub margin_bottom_pt: f32,
    pub margin_inner_pt: f32, // Gutter
    pub margin_outer_pt: f32,
    pub masoretic_typography: crate::masoretic::MasoreticTypographyConfig,
}

impl Default for TypesettingEngineConfig {
    fn default() -> Self {
        // Standard A4 with 25mm margins
        Self {
            page_width_pt: 595.28,
            page_height_pt: 841.89,
            margin_top_pt: 70.87,
            margin_bottom_pt: 56.69,
            margin_inner_pt: 85.04, // 30mm for binding
            margin_outer_pt: 56.69, // 20mm outer
            masoretic_typography: crate::masoretic::MasoreticTypographyConfig::default(),
        }
    }
}

impl TypesettingEngineConfig {
    pub fn with_masoretic_typography(
        mut self,
        config: crate::masoretic::MasoreticTypographyConfig,
    ) -> Self {
        self.masoretic_typography = config;
        self
    }
}

pub struct TypesettingEngine {
    config: TypesettingEngineConfig,
    pub font_manager: FontManager,
    pub paragraph_cache: std::sync::RwLock<crate::layout_cache::ParagraphLayoutCache>,
}

/// Whitespace that separates words. No-break spaces stay inside words.
fn is_break_space(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\u{00A0}' | '\u{2007}' | '\u{202F}')
}

/// Index of `family` in a line font table, appending it when new.
fn font_slot(fonts: &mut Vec<String>, family: Option<&str>) -> u16 {
    let Some(family) = family else {
        return UNKNOWN_FONT;
    };
    let idx = match fonts.iter().position(|f| f == family) {
        Some(idx) => idx,
        None => {
            fonts.push(family.to_string());
            fonts.len() - 1
        }
    };
    u16::try_from(idx).unwrap_or(UNKNOWN_FONT)
}

/// UAX #9 rule L2 at item granularity: reverses every maximal run at or above
/// each level, from the highest level down to the lowest odd level.
fn visual_order(levels: &[(u8, usize)]) -> Vec<usize> {
    let mut seq = levels.to_vec();
    let max = seq.iter().map(|s| s.0).max().unwrap_or(0);
    let min = seq.iter().map(|s| s.0).min().unwrap_or(0);
    let lowest_odd = min | 1;
    for level in (lowest_odd..=max).rev() {
        let mut k = 0;
        while k < seq.len() {
            if seq[k].0 >= level {
                let start = k;
                while k < seq.len() && seq[k].0 >= level {
                    k += 1;
                }
                seq[start..k].reverse();
            } else {
                k += 1;
            }
        }
    }
    seq.into_iter().map(|s| s.1).collect()
}

/// Per-item data the line breaker does not need to know about.
#[derive(Clone, Copy)]
struct ItemMeta {
    level: u8,
    font: u16,
    /// Byte offset of the item in the paragraph text.
    offset: u32,
    /// Effective font size (pt) of this item (accounts for character span / GREP scaling).
    font_size: f32,
}

impl TypesettingEngine {
    pub fn new(config: TypesettingEngineConfig) -> Self {
        Self {
            config,
            font_manager: FontManager::default(),
            paragraph_cache: std::sync::RwLock::new(
                crate::layout_cache::ParagraphLayoutCache::new(),
            ),
        }
    }

    pub fn with_font_manager(config: TypesettingEngineConfig, font_manager: FontManager) -> Self {
        Self {
            config,
            font_manager,
            paragraph_cache: std::sync::RwLock::new(
                crate::layout_cache::ParagraphLayoutCache::new(),
            ),
        }
    }

    /// Invalidates cached layout lines for a specific paragraph.
    pub fn invalidate_paragraph(&self, node_id: tok_core::id::NodeId) -> bool {
        self.paragraph_cache.write().unwrap().invalidate(node_id)
    }

    /// Invalidates cached layout lines for a collection of paragraph IDs.
    pub fn invalidate_paragraphs<I: IntoIterator<Item = tok_core::id::NodeId>>(
        &self,
        ids: I,
    ) -> usize {
        let mut cache = self.paragraph_cache.write().unwrap();
        let mut count = 0;
        for id in ids {
            if cache.invalidate(id) {
                count += 1;
            }
        }
        count
    }

    /// Clears the entire paragraph layout cache.
    pub fn clear_cache(&self) {
        self.paragraph_cache.write().unwrap().clear();
    }

    /// Returns layout cache metrics (hits, misses, entry count, memory bytes).
    pub fn cache_metrics(&self) -> crate::layout_cache::CacheMetricsSnapshot {
        self.paragraph_cache.read().unwrap().snapshot()
    }

    /// Typesets a single paragraph into formatted, justified LineBoxes using default font.
    pub fn typeset_paragraph(
        &self,
        paragraph: &ParagraphNode,
        column_width_pt: f32,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        self.typeset_paragraph_with_font(
            paragraph,
            DEFAULT_FONT_FAMILY,
            column_width_pt,
            font_size_pt,
            line_height_pt,
        )
    }

    /// Typesets a single paragraph into formatted, justified LineBoxes with a specified font family.
    ///
    /// Line baselines are relative to the top of the paragraph. Glyphs are in
    /// visual order; RTL lines that do not fill the measure are right-aligned.
    pub fn typeset_paragraph_with_font(
        &self,
        paragraph: &ParagraphNode,
        font_family: &str,
        column_width_pt: f32,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        if !column_width_pt.is_finite() || column_width_pt <= 0.0 {
            return Vec::new();
        }
        let has_special_patch = paragraph.style_patches.iter().any(|p| {
            p.drop_cap.as_ref().is_some_and(|dc| dc.enabled)
                || p.optical_margin.as_ref().is_some_and(|om| om.enabled)
                || p.last_line_constraint.is_some()
                || p.alignment.is_some()
                || p.first_line_indent_pt.is_some()
                || p.indent_start_pt.is_some()
                || p.indent_end_pt.is_some()
        });
        if has_special_patch {
            let profile = MeasureProfile::uniform(column_width_pt);
            return self.typeset_paragraph_shaped(
                paragraph,
                font_family,
                &profile,
                font_size_pt,
                line_height_pt,
            );
        }
        let session = self.font_manager.session(font_family);
        Self::typeset_with_session(
            &session,
            paragraph,
            column_width_pt,
            font_size_pt,
            line_height_pt,
        )
    }

    /// Typesets a single paragraph into formatted, justified LineBoxes matching a variable [`MeasureProfile`] (TeX `\parshape`).
    pub fn typeset_paragraph_shaped(
        &self,
        paragraph: &ParagraphNode,
        font_family: &str,
        profile: &MeasureProfile,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        self.typeset_paragraph_shaped_with_style(
            paragraph,
            None,
            font_family,
            profile,
            font_size_pt,
            line_height_pt,
        )
    }

    /// Typesets a single paragraph with an optional base [`tok_core::styles::ParagraphStyle`] and a [`MeasureProfile`].
    pub fn typeset_paragraph_shaped_with_style(
        &self,
        paragraph: &ParagraphNode,
        base_style: Option<&tok_core::styles::ParagraphStyle>,
        font_family: &str,
        profile: &MeasureProfile,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        let dc_opt = paragraph
            .style_patches
            .iter()
            .filter_map(|p| p.drop_cap.as_ref())
            .next_back()
            .cloned()
            .or_else(|| base_style.and_then(|s| s.drop_cap.clone()));

        let llc_opt = paragraph
            .style_patches
            .iter()
            .filter_map(|p| p.last_line_constraint.as_ref())
            .next_back()
            .cloned()
            .or_else(|| base_style.and_then(|s| s.last_line_constraint.clone()));

        let mut effective_profile = profile.clone();
        if !effective_profile.last_line_constraint.is_active() {
            if let Some(ref llc_cfg) = llc_opt {
                effective_profile.last_line_constraint =
                    crate::knuth_plass::LastLineConstraint::from(llc_cfg);
            }
        }

        let line_height = if line_height_pt.is_finite() && line_height_pt > 0.0 {
            line_height_pt
        } else {
            (if font_size_pt.is_finite() && font_size_pt > 0.0 {
                font_size_pt
            } else {
                DEFAULT_FONT_SIZE_PT
            }) * 1.3
        };

        let bidi = BidiInfo::new(&paragraph.text, None);
        let is_rtl = bidi.paragraphs.first().is_some_and(|p| p.level.is_rtl());

        if let Some(ref dc) = dc_opt {
            if dc.enabled {
                let col_w = effective_profile.width_for_line(0);
                let mut dc_resolved = dc.clone();
                if dc_resolved.font_family.is_none() {
                    dc_resolved.font_family = Some(font_family.to_string());
                }
                if let Some(drop_layout) = crate::drop_cap::HebrewDropCapShaper::shape_drop_cap(
                    &paragraph.text,
                    &dc_resolved,
                    &self.font_manager,
                    col_w,
                    line_height,
                    is_rtl,
                ) {
                    let notched_profile = effective_profile.with_drop_cap_exclusion(
                        line_height,
                        &drop_layout.exclusion,
                        is_rtl,
                    );
                    let mut rem_node = paragraph.clone();
                    rem_node.text = drop_layout.remaining_text;
                    rem_node.style_patches.retain(|p| p.drop_cap.is_none());
                    let mut base_style_no_dc = base_style.cloned();
                    if let Some(ref mut s) = base_style_no_dc {
                        s.drop_cap = None;
                    }

                    let session = self.font_manager.session(font_family);
                    let body_lines = Self::typeset_with_session_shaped_opts(
                        &session,
                        &rem_node,
                        base_style_no_dc.as_ref(),
                        &notched_profile,
                        font_size_pt,
                        line_height_pt,
                    );

                    let rem_byte_offset = drop_layout.remaining_byte_offset as u32;
                    let mut all_lines = Vec::with_capacity(body_lines.len() + 1);
                    all_lines.push(LineBox {
                        line_index: 0,
                        paragraph_id: Some(paragraph.id),
                        baseline_y: drop_layout.baseline_y,
                        height: drop_layout.window_rect.height,
                        width: drop_layout.window_rect.width,
                        glyphs: drop_layout.glyphs,
                        text: drop_layout.text,
                        is_rtl,
                        fonts: vec![drop_layout.font_family],
                        is_drop_cap: true,
                        x_offset: drop_layout.window_rect.x,
                    });

                    for (i, mut bl) in body_lines.into_iter().enumerate() {
                        bl.line_index = i + 1;
                        bl.paragraph_id = Some(paragraph.id);
                        for g in &mut bl.glyphs {
                            g.cluster += rem_byte_offset;
                        }
                        all_lines.push(bl);
                    }

                    return all_lines;
                }
            }
        }

        let session = self.font_manager.session(font_family);
        Self::typeset_with_session_shaped_full(
            &session,
            paragraph,
            base_style,
            &effective_profile,
            font_size_pt,
            line_height_pt,
            Some(&self.config.masoretic_typography),
        )
    }

    fn typeset_with_session(
        session: &ShapingSession<'_>,
        paragraph: &ParagraphNode,
        column_width_pt: f32,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        if !column_width_pt.is_finite() || column_width_pt <= 0.0 {
            return Vec::new();
        }
        let profile = MeasureProfile::uniform(column_width_pt);
        Self::typeset_with_session_shaped(
            session,
            paragraph,
            &profile,
            font_size_pt,
            line_height_pt,
        )
    }

    fn typeset_with_session_shaped(
        session: &ShapingSession<'_>,
        paragraph: &ParagraphNode,
        profile: &MeasureProfile,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        Self::typeset_with_session_shaped_opts(
            session,
            paragraph,
            None,
            profile,
            font_size_pt,
            line_height_pt,
        )
    }

    fn typeset_with_session_shaped_opts(
        session: &ShapingSession<'_>,
        paragraph: &ParagraphNode,
        base_style: Option<&tok_core::styles::ParagraphStyle>,
        profile: &MeasureProfile,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> Vec<LineBox> {
        Self::typeset_with_session_shaped_full(
            session,
            paragraph,
            base_style,
            profile,
            font_size_pt,
            line_height_pt,
            None,
        )
    }

    fn typeset_with_session_shaped_full(
        session: &ShapingSession<'_>,
        paragraph: &ParagraphNode,
        base_style: Option<&tok_core::styles::ParagraphStyle>,
        profile: &MeasureProfile,
        font_size_pt: f32,
        line_height_pt: f32,
        masoretic_config: Option<&crate::masoretic::MasoreticTypographyConfig>,
    ) -> Vec<LineBox> {
        let text = paragraph.text.as_str();
        if text.trim().is_empty() || !profile.is_valid() {
            return Vec::new();
        }
        let default_masoretic_cfg = crate::masoretic::MasoreticTypographyConfig::default();
        let masoretic_cfg = masoretic_config.unwrap_or(&default_masoretic_cfg);
        let font_size = if font_size_pt.is_finite() && font_size_pt > 0.0 {
            font_size_pt
        } else {
            DEFAULT_FONT_SIZE_PT
        };
        let line_height = if line_height_pt.is_finite() && line_height_pt > 0.0 {
            line_height_pt
        } else {
            font_size * 1.3
        };

        let om_opt = paragraph
            .style_patches
            .iter()
            .filter_map(|p| p.optical_margin.as_ref())
            .next_back()
            .cloned()
            .or_else(|| base_style.and_then(|s| s.optical_margin.clone()));

        let align = paragraph
            .style_patches
            .iter()
            .filter_map(|p| p.alignment)
            .next_back()
            .or_else(|| base_style.map(|s| s.alignment))
            .unwrap_or(tok_core::styles::TextAlignment::Justified);

        let llc_opt = paragraph
            .style_patches
            .iter()
            .filter_map(|p| p.last_line_constraint.as_ref())
            .next_back()
            .cloned()
            .or_else(|| base_style.and_then(|s| s.last_line_constraint.clone()));

        let mut effective_profile = profile.clone();
        if !effective_profile.last_line_constraint.is_active() {
            if let Some(ref llc_cfg) = llc_opt {
                effective_profile.last_line_constraint =
                    crate::knuth_plass::LastLineConstraint::from(llc_cfg);
            }
        }

        // 1. Resolve bidi levels once for the whole paragraph.
        let bidi = BidiInfo::new(text, None);
        let is_rtl = bidi.paragraphs.first().is_some_and(|p| p.level.is_rtl());
        let level_at = |byte: usize| -> u8 {
            bidi.levels
                .get(byte)
                .map(|l| l.number())
                .unwrap_or(u8::from(is_rtl))
        };

        // 2. Build the Knuth-Plass stream: words split into same-level & same-span runs,
        //    each shaped in its own direction and character format, separated by glue.
        let mut fonts: Vec<String> = Vec::new();
        let space_font = font_slot(&mut fonts, session.family_of(session.space_choice()));
        let space_gid = session.space_glyph_id().unwrap_or(3);
        let base_space = font_size * SPACE_EM;

        // Map byte offsets to active span index (if paragraph.spans is non-empty)
        let span_at_byte: Vec<Option<usize>> = if paragraph.spans.is_empty() {
            Vec::new()
        } else {
            let mut map = vec![None; text.len() + 1];
            for (char_idx, (byte_idx, ch)) in text.char_indices().enumerate() {
                let active_span = paragraph
                    .spans
                    .iter()
                    .position(|s| char_idx >= s.start && char_idx < s.end);
                map[byte_idx..byte_idx + ch.len_utf8()].fill(active_span);
            }
            map
        };
        let span_idx_at =
            |byte: usize| -> Option<usize> { span_at_byte.get(byte).copied().flatten() };

        let est_items = (text.len() / 5).max(8) * 2;
        let mut items: Vec<LayoutItem> = Vec::with_capacity(est_items);
        let mut meta: Vec<ItemMeta> = Vec::with_capacity(est_items);
        let mut word_start: Option<usize> = None;
        let mut gap_offset: Option<usize> = None;

        let mut push_word = |start: usize, end: usize, gap: Option<usize>| {
            if let Some(gap) = gap {
                items.push(LayoutItem::Glue {
                    width: base_space,
                    stretch: base_space * SPACE_STRETCH,
                    shrink: base_space * SPACE_SHRINK,
                });
                meta.push(ItemMeta {
                    level: level_at(gap),
                    font: space_font,
                    offset: gap as u32,
                    font_size,
                });
            }
            let mut run_start = start;
            let word = &text[start..end];
            let boundaries = word
                .char_indices()
                .map(|(i, _)| start + i)
                .skip(1)
                .filter(|&b| level_at(b) != level_at(b - 1) || span_idx_at(b) != span_idx_at(b - 1))
                .chain(std::iter::once(end));
            for run_end in boundaries {
                let level = level_at(run_start);
                let span_opt = span_idx_at(run_start).and_then(|idx| paragraph.spans.get(idx));
                let mut run_font_size = font_size;
                let mut baseline_shift = 0.0_f32;
                if let Some(sp) = span_opt {
                    if let Some(sz) = sp.format.font_size_pt {
                        if sz.is_finite() && sz > 0.0 {
                            run_font_size = sz;
                        }
                    }
                    if let Some(scale) = sp.format.font_scale {
                        if scale.is_finite() && scale > 0.0 {
                            run_font_size *= scale;
                        }
                    }
                    if let Some(mv) = sp.format.masoretic_variant {
                        if sp.format.font_scale.is_none() {
                            run_font_size *= mv.font_size_scale();
                        }
                        if let Some(bs) = sp.format.baseline_shift_pt {
                            if bs.is_finite() {
                                baseline_shift += bs;
                            }
                        } else {
                            baseline_shift += mv.baseline_shift_em() * font_size;
                        }
                    } else if let Some(bs) = sp.format.baseline_shift_pt {
                        if bs.is_finite() {
                            baseline_shift += bs;
                        }
                    }
                }
                let run_text = &text[run_start..run_end];
                let mut extra_features = masoretic_cfg.resolve_features_for_text(run_text);
                if let Some(para_features) = base_style.and_then(|s| s.opentype_features.as_ref()) {
                    extra_features.extend(crate::masoretic::convert_opentype_settings(para_features));
                }
                for patch in &paragraph.style_patches {
                    if let Some(patch_features) = &patch.opentype_features {
                        extra_features.extend(crate::masoretic::convert_opentype_settings(patch_features));
                    }
                }
                if let Some(sp) = span_opt {
                    if let Some(mv) = sp.format.masoretic_variant {
                        if let Some(tag) = mv.recommended_opentype_tag() {
                            if let Ok(bytes) = tag.as_bytes().try_into() {
                                extra_features.push(rustybuzz::Feature::new(
                                    rustybuzz::ttf_parser::Tag::from_bytes(bytes),
                                    1,
                                    ..,
                                ));
                            }
                        }
                    }
                    if let Some(span_features) = &sp.format.opentype_features {
                        extra_features.extend(crate::masoretic::convert_opentype_settings(span_features));
                    }
                }
                let (mut run, choice) = session.shape_with_features(
                    run_text,
                    run_font_size,
                    level % 2 == 1,
                    &extra_features,
                );
                for g in &mut run.glyphs {
                    g.cluster += run_start as u32;
                    g.y_offset += baseline_shift;
                }
                let resolved_family_owned: Option<String> = if let Some(sp) = span_opt {
                    let is_bold_span = sp.format.font_weight.is_some_and(|w| w >= 600);
                    if let Some(ref fam) = sp.format.font_family {
                        if is_bold_span
                            && !fam.ends_with(" Bold")
                            && (fam == "Frank Ruhl Libre" || fam == "David Libre")
                        {
                            Some(format!("{} Bold", fam))
                        } else {
                            Some(fam.clone())
                        }
                    } else if let Some(base_fam) = session.family_of(choice) {
                        if is_bold_span
                            && !base_fam.ends_with(" Bold")
                            && (base_fam == "Frank Ruhl Libre" || base_fam == "David Libre")
                        {
                            Some(format!("{} Bold", base_fam))
                        } else {
                            Some(base_fam.to_string())
                        }
                    } else {
                        None
                    }
                } else {
                    session.family_of(choice).map(ToString::to_string)
                };
                items.push(LayoutItem::Box {
                    width: run.total_width_pt,
                    text: text[run_start..run_end].to_string(),
                    glyphs: run.glyphs,
                });
                meta.push(ItemMeta {
                    level,
                    font: font_slot(&mut fonts, resolved_family_owned.as_deref()),
                    offset: run_start as u32,
                    font_size: run_font_size,
                });
                run_start = run_end;
            }
        };

        for (i, c) in text.char_indices() {
            if is_break_space(c) {
                if let Some(start) = word_start.take() {
                    push_word(start, i, gap_offset.take());
                    gap_offset = Some(i);
                }
            } else if word_start.is_none() {
                word_start = Some(i);
            }
        }
        if let Some(start) = word_start {
            push_word(start, text.len(), gap_offset);
        }

        // Forced break at end of paragraph
        items.push(LayoutItem::Penalty {
            width: 0.0,
            penalty: KnuthPlassBreaker::FORCED_BREAK_PENALTY,
            flagged: false,
        });
        meta.push(ItemMeta {
            level: u8::from(is_rtl),
            font: space_font,
            offset: text.len() as u32,
            font_size,
        });

        // 3. Run Knuth-Plass global line breaker matching the profile
        let spans = KnuthPlassBreaker::break_paragraph_spans_with_profile(
            &items,
            &effective_profile,
            LINE_BREAK_TOLERANCE,
        );

        // 4. Reorder, justify and position each line.
        let total_lines = spans.len();
        let mut result_lines = Vec::with_capacity(total_lines);

        for (idx, span) in spans.into_iter().enumerate() {
            let is_last = idx + 1 == total_lines;
            let logical: Vec<(u8, usize)> = (span.start..span.end)
                .filter(|&i| !matches!(items[i], LayoutItem::Penalty { .. }))
                .map(|i| (meta[i].level, i))
                .collect();

            let mut line_text = String::new();
            for &(_, i) in &logical {
                match &items[i] {
                    LayoutItem::Box { text, .. } => line_text.push_str(text),
                    LayoutItem::Glue { .. } => line_text.push(' '),
                    LayoutItem::Penalty { .. } => {}
                }
            }

            let mut line_glyphs: Vec<PositionedGlyph> = Vec::new();
            let mut glyph_fonts: Vec<u16> = Vec::new();
            let mut glyph_sizes: Vec<f32> = Vec::new();
            for i in visual_order(&logical) {
                match &items[i] {
                    LayoutItem::Box { glyphs, .. } => {
                        line_glyphs.extend(glyphs.iter().cloned());
                        glyph_fonts.extend(std::iter::repeat_n(meta[i].font, glyphs.len()));
                        glyph_sizes.extend(std::iter::repeat_n(meta[i].font_size, glyphs.len()));
                    }
                    LayoutItem::Glue { .. } => {
                        line_glyphs.push(PositionedGlyph {
                            glyph_id: space_gid,
                            cluster: meta[i].offset,
                            x_advance: base_space,
                            y_advance: 0.0,
                            x_offset: 0.0,
                            y_offset: 0.0,
                            character: Some(' '),
                        });
                        glyph_fonts.push(meta[i].font);
                        glyph_sizes.push(meta[i].font_size);
                    }
                    LayoutItem::Penalty { .. } => {}
                }
            }

            let target_w = span.target_width;
            let is_justified_line = !is_last && align == tok_core::styles::TextAlignment::Justified;

            // Compute optical margin protrusion for the line's visual edge glyphs when enabled
            let protrusion = om_opt
                .as_ref()
                .filter(|cfg| cfg.enabled)
                .map(|cfg| {
                    crate::optical_margin::OpticalMarginAligner::compute_line_protrusion(
                        &line_glyphs,
                        font_size,
                        cfg,
                    )
                })
                .unwrap_or_default();

            let (justified_target, line_x_shift, _) =
                crate::optical_margin::OpticalMarginAligner::adjust_line_geometry(
                    target_w,
                    span.x_offset,
                    is_rtl,
                    is_last,
                    align,
                    protrusion,
                );

            let justified = if is_justified_line {
                HebrewJustifier::justify_line(line_glyphs, justified_target, font_size, false)
            } else {
                HebrewJustifier::justify_line(line_glyphs, target_w, font_size, true)
            };

            let line_width: f32 = justified.glyphs.iter().map(|g| g.x_advance).sum();
            let base_start_x = match align {
                tok_core::styles::TextAlignment::Center => {
                    span.x_offset + (target_w - line_width) * 0.5
                }
                tok_core::styles::TextAlignment::Left => span.x_offset,
                tok_core::styles::TextAlignment::Right => {
                    span.x_offset + (target_w - line_width).max(0.0)
                }
                tok_core::styles::TextAlignment::Justified => {
                    if is_justified_line {
                        span.x_offset
                    } else {
                        HebrewJustifier::calculate_line_start_x(
                            line_width,
                            target_w,
                            span.x_offset,
                            is_rtl,
                        )
                    }
                }
            };
            let mut current_x = base_start_x + (line_x_shift - span.x_offset);
            let glyph_boxes: Vec<GlyphBox> = justified
                .glyphs
                .into_iter()
                .zip(glyph_fonts.into_iter().zip(glyph_sizes))
                .map(|(g, (font_index, glyph_font_size))| {
                    let b = GlyphBox {
                        glyph_id: g.glyph_id,
                        cluster: g.cluster,
                        x: current_x + g.x_offset,
                        y: g.y_offset,
                        width: g.x_advance,
                        height: glyph_font_size,
                        character: g.character,
                        font_index,
                    };
                    current_x += g.x_advance;
                    b
                })
                .collect();

            result_lines.push(LineBox {
                line_index: idx,
                paragraph_id: Some(paragraph.id),
                baseline_y: (idx as f32 + 1.0) * line_height,
                height: line_height,
                width: line_width,
                glyphs: glyph_boxes,
                text: line_text,
                is_rtl,
                fonts: fonts.clone(),
                is_drop_cap: false,
                x_offset: span.x_offset,
            });
        }

        result_lines
    }

    /// Solves or calculates the content rectangle for a given spread side (Recto / Verso).
    pub fn content_rect_for_side(&self, side: SpreadSide) -> PhysicalRect {
        let page_config = PageGeometryConfig::new(
            self.config.page_width_pt,
            self.config.page_height_pt,
            self.config.margin_inner_pt,
            self.config.margin_outer_pt,
            self.config.margin_top_pt,
            self.config.margin_bottom_pt,
            side,
        );
        let mut solver = GeometricConstraintSolver::new();
        let page_model = PageConstraintModel::new();
        if page_model.apply(&mut solver, &page_config).is_ok() {
            let r = page_model.get_content_rect(&solver);
            PhysicalRect::new(r.x, r.y, r.width, r.height)
        } else {
            let (margin_left, margin_right) = match side {
                SpreadSide::Recto => (self.config.margin_inner_pt, self.config.margin_outer_pt),
                SpreadSide::Verso => (self.config.margin_outer_pt, self.config.margin_inner_pt),
            };
            PhysicalRect::new(
                margin_left,
                self.config.margin_top_pt,
                (self.config.page_width_pt - margin_left - margin_right).max(0.0),
                (self.config.page_height_pt
                    - self.config.margin_top_pt
                    - self.config.margin_bottom_pt)
                    .max(0.0),
            )
        }
    }

    fn new_page(
        &self,
        page_num: usize,
        _content_width: f32,
        content_height: f32,
        lines: Vec<LineBox>,
        break_token: Option<BreakToken>,
        flow_id: &str,
    ) -> PageLayoutBox {
        let side = if page_num % 2 == 1 {
            SpreadSide::Recto
        } else {
            SpreadSide::Verso
        };
        let content = self.content_rect_for_side(side);

        let frame = TextFrameBox {
            frame_id: format!("frame_{}_{}", page_num, flow_id),
            flow_id: flow_id.to_string(),
            rect: PhysicalRect::new(content.x, content.y, content.width, content_height),
            lines,
        };
        PageLayoutBox {
            page_index: page_num - 1,
            page_number_gematria: GematriaEngine::to_hebrew_numeral(page_num),
            dimensions: PhysicalRect::new(
                0.0,
                0.0,
                self.config.page_width_pt,
                self.config.page_height_pt,
            ),
            frames: vec![frame],
            break_token,
        }
    }

    /// Typesets an entire DocumentRoot into multi-page layout boxes,
    /// respecting custom page dimensions and margins if present in document settings.
    pub fn typeset_document(&self, doc: &DocumentRoot) -> Vec<PageLayoutBox> {
        let mut active_config = self.config.clone();
        if let Some(settings) = &doc.metadata.document_settings {
            let mm_to_pt = |val: Option<f64>| val.map(|v| (v * 72.0 / 25.4) as f32);

            if let Some(w) = mm_to_pt(settings.get("pageWidthMm").and_then(|v| v.as_f64())) {
                if w > 10.0 {
                    active_config.page_width_pt = w;
                }
            }
            if let Some(h) = mm_to_pt(settings.get("pageHeightMm").and_then(|v| v.as_f64())) {
                if h > 10.0 {
                    active_config.page_height_pt = h;
                }
            }
            if let Some(mt) = mm_to_pt(settings.get("marginTopMm").and_then(|v| v.as_f64())) {
                active_config.margin_top_pt = mt;
            }
            if let Some(mb) = mm_to_pt(settings.get("marginBottomMm").and_then(|v| v.as_f64())) {
                active_config.margin_bottom_pt = mb;
            }
            if let Some(mi) = mm_to_pt(settings.get("marginInsideMm").and_then(|v| v.as_f64())) {
                active_config.margin_inner_pt = mi;
            }
            if let Some(mo) = mm_to_pt(settings.get("marginOutsideMm").and_then(|v| v.as_f64())) {
                active_config.margin_outer_pt = mo;
            }
        }

        if active_config != self.config {
            let engine = TypesettingEngine::new(active_config);
            return engine.typeset_document_internal(doc);
        }

        self.typeset_document_internal(doc)
    }

    /// Internal implementation of document typesetting.
    pub fn typeset_document_internal(&self, doc: &DocumentRoot) -> Vec<PageLayoutBox> {
        let has_multi_flow = doc.sections.iter().any(|s| s.flows.len() > 1);
        if has_multi_flow {
            return self.typeset_document_multi_flow(doc);
        }

        let content = self.content_rect_for_side(SpreadSide::Recto);
        let content_width = content.width;
        let content_height = content.height;
        let flow_id = doc
            .sections
            .first()
            .and_then(|s| s.main_flow().or_else(|| s.flows.first()))
            .map(|f| f.id.0.as_str())
            .unwrap_or("gemara");

        // Paragraphs are laid out independently, so break them in parallel;
        // results come back in document order, keeping output deterministic.
        let jobs: Vec<(usize, usize, &ParagraphNode)> = doc
            .sections
            .iter()
            .enumerate()
            .filter_map(|(sec_idx, sec)| sec.main_flow().map(|flow| (sec_idx, flow)))
            .flat_map(|(sec_idx, flow)| {
                flow.paragraphs
                    .iter()
                    .enumerate()
                    .map(move |(p_idx, p)| (sec_idx, p_idx, p))
            })
            .collect();

        let default_style = tok_core::styles::ParagraphStyle::default();
        let has_spans_to_resolve = !doc.grep_rules.is_empty() || !doc.character_styles.is_empty();
        let cached_lookup: Vec<(crate::layout_cache::ParagraphCacheKey, Option<Vec<LineBox>>)> = {
            let cache = self.paragraph_cache.read().unwrap();
            jobs.iter()
                .map(|&(_, _, p)| {
                    let style = doc
                        .paragraph_styles
                        .iter()
                        .find(|s| s.id == p.style_id)
                        .unwrap_or(&default_style);
                    let key = if has_spans_to_resolve {
                        let mut p_eff = p.clone();
                        p_eff.spans = doc.effective_spans_for_paragraph(p, Some(flow_id));
                        crate::layout_cache::ParagraphLayoutCache::make_key(
                            &p_eff,
                            style,
                            content_width,
                        )
                    } else {
                        crate::layout_cache::ParagraphLayoutCache::make_key(p, style, content_width)
                    };
                    let hit = cache.get(&key).map(|c| c.lines.clone());
                    (key, hit)
                })
                .collect()
        };

        let misses: Vec<(
            usize,
            &ParagraphNode,
            crate::layout_cache::ParagraphCacheKey,
        )> = jobs
            .iter()
            .zip(&cached_lookup)
            .enumerate()
            .filter_map(|(idx, (&(_, _, p), (key, cached)))| {
                if cached.is_none() {
                    Some((idx, p, key.clone()))
                } else {
                    None
                }
            })
            .collect();

        let computed: Vec<(usize, crate::layout_cache::ParagraphCacheKey, Vec<LineBox>)> = misses
            .par_iter()
            .map_init(
                HashMap::<String, ShapingSession<'_>>::new,
                |sessions, &(idx, p, ref key)| {
                    let mut p_eff;
                    let p_ref = if has_spans_to_resolve {
                        p_eff = p.clone();
                        p_eff.spans = doc.effective_spans_for_paragraph(p, Some(flow_id));
                        &p_eff
                    } else {
                        p
                    };
                    let base_style = doc.paragraph_styles.iter().find(|s| s.id == p.style_id);
                    let resolved_style = p.effective_style(&doc.paragraph_styles);
                    let (font_family, font_size, line_height) = if base_style.is_some()
                        || !p.style_patches.is_empty()
                    {
                        (
                            self.font_manager
                                .face_for(&resolved_style.font_family, resolved_style.font_weight),
                            resolved_style.font_size_pt,
                            resolved_style.line_height_pt,
                        )
                    } else {
                        (
                            DEFAULT_FONT_FAMILY.to_string(),
                            DEFAULT_FONT_SIZE_PT,
                            DEFAULT_LINE_HEIGHT_PT,
                        )
                    };
                    let lines = if resolved_style
                        .drop_cap
                        .as_ref()
                        .is_some_and(|dc| dc.enabled)
                    {
                        let profile = MeasureProfile::uniform(content_width);
                        self.typeset_paragraph_shaped_with_style(
                            p_ref,
                            Some(&resolved_style),
                            &font_family,
                            &profile,
                            font_size,
                            line_height,
                        )
                    } else {
                        let session = sessions
                            .entry(font_family.clone())
                            .or_insert_with(|| self.font_manager.session(&font_family));
                        let profile = MeasureProfile::uniform(content_width);
                        Self::typeset_with_session_shaped_full(
                            session,
                            p_ref,
                            Some(&resolved_style),
                            &profile,
                            font_size,
                            line_height,
                            Some(&self.config.masoretic_typography),
                        )
                    };
                    (idx, key.clone(), lines)
                },
            )
            .collect();

        if !computed.is_empty() {
            let mut cache = self.paragraph_cache.write().unwrap();
            for (_, key, lines) in &computed {
                cache.insert(key.clone(), lines.clone());
            }
        }

        let mut computed_map: HashMap<usize, Vec<LineBox>> = computed
            .into_iter()
            .map(|(idx, _, lines)| (idx, lines))
            .collect();

        let laid_out: Vec<Vec<LineBox>> = cached_lookup
            .into_iter()
            .enumerate()
            .map(|(idx, (_, hit))| {
                hit.unwrap_or_else(|| computed_map.remove(&idx).unwrap_or_default())
            })
            .collect();

        let mut pagination_inputs = Vec::new();
        let mut prev_space_after: f32 = 0.0;
        let mut prev_section_idx: Option<usize> = None;

        for (&(sec_idx, p_idx, p), p_lines) in jobs.iter().zip(laid_out) {
            if prev_section_idx != Some(sec_idx) {
                prev_section_idx = Some(sec_idx);
                prev_space_after = 0.0;
            }

            let (keep_with_next, orphan_control, widow_control, space_before, space_after) = doc
                .paragraph_styles
                .iter()
                .find(|s| s.id == p.style_id)
                .map(|style| {
                    (
                        style.keep_with_next,
                        style.orphan_control,
                        style.widow_control,
                        style.space_before_pt.max(0.0),
                        style.space_after_pt.max(0.0),
                    )
                })
                .unwrap_or((false, 2, 2, 0.0, 0.0));

            if p_lines.is_empty() {
                prev_space_after = prev_space_after.max(space_before).max(space_after);
                continue;
            }

            let effective_space_before = prev_space_after.max(space_before);
            let total_lines = p_lines.len();
            for (l_idx, line) in p_lines.into_iter().enumerate() {
                let char_offset = line
                    .glyphs
                    .iter()
                    .map(|g| g.cluster as usize)
                    .min()
                    .and_then(|byte| p.text.get(..byte))
                    .map_or(0, |s| s.chars().count());

                pagination_inputs.push(PaginationLineInput {
                    line,
                    section_idx: sec_idx,
                    paragraph_idx: p_idx,
                    paragraph_id: Some(p.id),
                    line_in_paragraph: l_idx,
                    total_lines_in_paragraph: total_lines,
                    keep_with_next,
                    orphan_control,
                    widow_control,
                    space_before: if l_idx == 0 {
                        effective_space_before
                    } else {
                        0.0
                    },
                    space_after,
                    char_offset,
                    force_page_break_before: p.has_forced_page_break_before(),
                    force_page_break_after: p.has_forced_page_break_after(),
                    keep_lines_together: p
                        .overrides
                        .as_ref()
                        .is_some_and(|o| o.keep_lines_together),
                });
            }
            prev_space_after = space_after;
        }

        let breaker = GlobalPageBreaker::new(content_height);
        let slices = breaker.break_pages(&pagination_inputs);

        let mut pages = Vec::new();
        for (page_idx, slice) in slices.into_iter().enumerate() {
            let page = self.new_page(
                page_idx + 1,
                content_width,
                slice.total_height,
                slice.lines,
                slice.break_token,
                flow_id,
            );
            pages.push(page);
        }

        if pages.is_empty() {
            pages.push(self.new_page(1, content_width, content_height, Vec::new(), None, flow_id));
        }

        pages
    }

    /// Incrementally typesets a document given the previous layout outcome.
    ///
    /// Exploits:
    /// 1. `ParagraphLayoutCache`: re-breaks only paragraphs whose text, styles, or width changed.
    /// 2. Zero-Cascade Optimization: when an edit does not alter paragraph line count and heights,
    ///    only the containing page is updated, completing in sub-millisecond latency.
    /// 3. Incremental Page Breaking with Convergence: repaginates downstream only until
    ///    break tokens converge with the previous page boundaries.
    pub fn typeset_document_incremental(
        &self,
        doc: &DocumentRoot,
        previous_pages: &[PageLayoutBox],
        modified_node_id: Option<tok_core::id::NodeId>,
    ) -> (
        Vec<PageLayoutBox>,
        crate::incremental::IncrementalTypesetDiagnostics,
    ) {
        let start_time = std::time::Instant::now();

        // If no previous pages exist, perform full typeset
        if previous_pages.is_empty() {
            let pages = self.typeset_document(doc);
            let total = pages.len();
            let total_paras: usize = doc
                .sections
                .iter()
                .flat_map(|s| &s.flows)
                .map(|f| f.paragraphs.len())
                .sum();
            let diag = crate::incremental::IncrementalTypesetDiagnostics {
                total_pages: total,
                first_affected_page: 0,
                recomputed_pages: total,
                reused_pages: 0,
                recomputed_paragraphs: total_paras,
                reused_paragraphs: 0,
                zero_cascade: false,
                converged_at_page: None,
                duration: start_time.elapsed(),
            };
            return (pages, diag);
        }

        let content = self.content_rect_for_side(SpreadSide::Recto);
        let content_width = content.width;
        let content_height = content.height;
        let flow_id = doc
            .sections
            .first()
            .and_then(|s| s.main_flow().or_else(|| s.flows.first()))
            .map(|f| f.id.0.as_str())
            .unwrap_or("gemara");

        // Collect all paragraphs and check cache
        let jobs: Vec<(usize, usize, &ParagraphNode)> = doc
            .sections
            .iter()
            .enumerate()
            .filter_map(|(sec_idx, sec)| sec.main_flow().map(|flow| (sec_idx, flow)))
            .flat_map(|(sec_idx, flow)| {
                flow.paragraphs
                    .iter()
                    .enumerate()
                    .map(move |(p_idx, p)| (sec_idx, p_idx, p))
            })
            .collect();

        let default_style = tok_core::styles::ParagraphStyle::default();
        let cached_lookup: Vec<(crate::layout_cache::ParagraphCacheKey, Option<Vec<LineBox>>)> = {
            let cache = self.paragraph_cache.read().unwrap();
            jobs.iter()
                .map(|&(_, _, p)| {
                    let style = doc
                        .paragraph_styles
                        .iter()
                        .find(|s| s.id == p.style_id)
                        .unwrap_or(&default_style);
                    let key = crate::layout_cache::ParagraphLayoutCache::make_key(
                        p,
                        style,
                        content_width,
                    );
                    let hit = cache.get(&key).map(|c| c.lines.clone());
                    (key, hit)
                })
                .collect()
        };

        let misses: Vec<(
            usize,
            &ParagraphNode,
            crate::layout_cache::ParagraphCacheKey,
        )> = jobs
            .iter()
            .zip(&cached_lookup)
            .enumerate()
            .filter_map(|(idx, (&(_, _, p), (key, cached)))| {
                if cached.is_none() {
                    Some((idx, p, key.clone()))
                } else {
                    None
                }
            })
            .collect();

        let recomputed_paras_count = misses.len();
        let reused_paras_count = jobs.len().saturating_sub(recomputed_paras_count);

        let computed: Vec<(usize, crate::layout_cache::ParagraphCacheKey, Vec<LineBox>)> = misses
            .par_iter()
            .map_init(
                HashMap::<String, ShapingSession<'_>>::new,
                |sessions, &(idx, p, ref key)| {
                    let base_style = doc.paragraph_styles.iter().find(|s| s.id == p.style_id);
                    let resolved_style = p.effective_style(&doc.paragraph_styles);
                    let (font_family, font_size, line_height) = if base_style.is_some()
                        || !p.style_patches.is_empty()
                    {
                        (
                            self.font_manager
                                .face_for(&resolved_style.font_family, resolved_style.font_weight),
                            resolved_style.font_size_pt,
                            resolved_style.line_height_pt,
                        )
                    } else {
                        (
                            DEFAULT_FONT_FAMILY.to_string(),
                            DEFAULT_FONT_SIZE_PT,
                            DEFAULT_LINE_HEIGHT_PT,
                        )
                    };
                    let lines = if resolved_style
                        .drop_cap
                        .as_ref()
                        .is_some_and(|dc| dc.enabled)
                    {
                        let profile = MeasureProfile::uniform(content_width);
                        self.typeset_paragraph_shaped_with_style(
                            p,
                            Some(&resolved_style),
                            &font_family,
                            &profile,
                            font_size,
                            line_height,
                        )
                    } else {
                        let session = sessions
                            .entry(font_family.clone())
                            .or_insert_with(|| self.font_manager.session(&font_family));
                        let profile = MeasureProfile::uniform(content_width);
                        Self::typeset_with_session_shaped_opts(
                            session,
                            p,
                            Some(&resolved_style),
                            &profile,
                            font_size,
                            line_height,
                        )
                    };
                    (idx, key.clone(), lines)
                },
            )
            .collect();

        if !computed.is_empty() {
            let mut cache = self.paragraph_cache.write().unwrap();
            for (_, key, lines) in &computed {
                cache.insert(key.clone(), lines.clone());
            }
        }

        let mut computed_map: HashMap<usize, Vec<LineBox>> = computed
            .into_iter()
            .map(|(idx, _, lines)| (idx, lines))
            .collect();

        let laid_out: Vec<Vec<LineBox>> = cached_lookup
            .into_iter()
            .enumerate()
            .map(|(idx, (_, hit))| {
                hit.unwrap_or_else(|| computed_map.remove(&idx).unwrap_or_default())
            })
            .collect();

        // Check Zero-Cascade condition:
        // If exactly 1 paragraph was modified, and it is located completely within 1 page in previous_pages,
        // and its new line count equals its old line count on that page, and line heights match:
        if recomputed_paras_count == 1 {
            let target_node_id = modified_node_id.or_else(|| misses.first().map(|(_, p, _)| p.id));

            if let Some(target_id) = target_node_id {
                let pages_containing: Vec<usize> = previous_pages
                    .iter()
                    .enumerate()
                    .filter(|(_, page)| {
                        page.frames
                            .iter()
                            .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(target_id)))
                    })
                    .map(|(idx, _)| idx)
                    .collect();

                if pages_containing.len() == 1 {
                    let page_idx = pages_containing[0];
                    let old_page = &previous_pages[page_idx];
                    let old_lines_count = old_page
                        .frames
                        .iter()
                        .flat_map(|f| &f.lines)
                        .filter(|l| l.paragraph_id == Some(target_id))
                        .count();

                    if let Some((job_idx, _)) = jobs
                        .iter()
                        .enumerate()
                        .find(|(_, (_, _, p))| p.id == target_id)
                    {
                        let new_lines = &laid_out[job_idx];
                        if new_lines.len() == old_lines_count {
                            // Check line heights match
                            let old_lines: Vec<&LineBox> = old_page
                                .frames
                                .iter()
                                .flat_map(|f| &f.lines)
                                .filter(|l| l.paragraph_id == Some(target_id))
                                .collect();

                            let heights_match = new_lines
                                .iter()
                                .zip(&old_lines)
                                .all(|(n, o)| (n.height - o.height).abs() < 1e-3);

                            if heights_match {
                                // ZERO-CASCADE TRIGGERED!
                                let mut new_pages = previous_pages.to_vec();
                                let page = &mut new_pages[page_idx];
                                for frame in &mut page.frames {
                                    let mut new_frame_lines = Vec::with_capacity(frame.lines.len());
                                    let mut new_lines_iter = new_lines.iter().cloned();

                                    for old_line in &frame.lines {
                                        let line = if old_line.paragraph_id == Some(target_id) {
                                            let mut n = new_lines_iter
                                                .next()
                                                .unwrap_or_else(|| old_line.clone());
                                            n.baseline_y = old_line.baseline_y;
                                            n
                                        } else {
                                            old_line.clone()
                                        };
                                        new_frame_lines.push(line);
                                    }
                                    frame.lines = new_frame_lines;
                                }

                                let total = new_pages.len();
                                return (
                                    new_pages,
                                    crate::incremental::IncrementalTypesetDiagnostics {
                                        total_pages: total,
                                        first_affected_page: page_idx,
                                        recomputed_pages: 1,
                                        reused_pages: total - 1,
                                        recomputed_paragraphs: 1,
                                        reused_paragraphs: reused_paras_count,
                                        zero_cascade: true,
                                        converged_at_page: Some(page_idx),
                                        duration: start_time.elapsed(),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        // Otherwise, build pagination inputs and run IncrementalPageBreaker
        let mut pagination_inputs = Vec::new();
        let mut prev_space_after: f32 = 0.0;
        let mut prev_section_idx: Option<usize> = None;
        let mut first_affected_line_idx: Option<usize> = None;

        for (job_idx, (&(sec_idx, p_idx, p), p_lines)) in jobs.iter().zip(&laid_out).enumerate() {
            let is_modified = misses.iter().any(|(m_idx, _, _)| *m_idx == job_idx);
            if is_modified && first_affected_line_idx.is_none() {
                first_affected_line_idx = Some(pagination_inputs.len());
            }

            if prev_section_idx != Some(sec_idx) {
                prev_section_idx = Some(sec_idx);
                prev_space_after = 0.0;
            }

            let (keep_with_next, orphan_control, widow_control, space_before, space_after) = doc
                .paragraph_styles
                .iter()
                .find(|s| s.id == p.style_id)
                .map(|style| {
                    (
                        style.keep_with_next,
                        style.orphan_control,
                        style.widow_control,
                        style.space_before_pt.max(0.0),
                        style.space_after_pt.max(0.0),
                    )
                })
                .unwrap_or((false, 2, 2, 0.0, 0.0));

            if p_lines.is_empty() {
                prev_space_after = prev_space_after.max(space_before).max(space_after);
                continue;
            }

            let effective_space_before = prev_space_after.max(space_before);
            let total_lines = p_lines.len();
            for (l_idx, line) in p_lines.iter().enumerate() {
                let char_offset = line
                    .glyphs
                    .iter()
                    .map(|g| g.cluster as usize)
                    .min()
                    .and_then(|byte| p.text.get(..byte))
                    .map_or(0, |s| s.chars().count());

                pagination_inputs.push(PaginationLineInput {
                    line: line.clone(),
                    section_idx: sec_idx,
                    paragraph_idx: p_idx,
                    paragraph_id: Some(p.id),
                    line_in_paragraph: l_idx,
                    total_lines_in_paragraph: total_lines,
                    keep_with_next,
                    orphan_control,
                    widow_control,
                    space_before: if l_idx == 0 {
                        effective_space_before
                    } else {
                        0.0
                    },
                    space_after,
                    char_offset,
                    force_page_break_before: p.has_forced_page_break_before(),
                    force_page_break_after: p.has_forced_page_break_after(),
                    keep_lines_together: p
                        .overrides
                        .as_ref()
                        .is_some_and(|o| o.keep_lines_together),
                });
            }
            prev_space_after = space_after;
        }

        let previous_slices: Vec<crate::page_break::PageSlice> = previous_pages
            .iter()
            .map(|p| crate::page_break::PageSlice {
                lines: p.frames.iter().flat_map(|f| f.lines.clone()).collect(),
                total_height: p.frames.iter().map(|f| f.rect.height).sum(),
                break_token: p.break_token.clone(),
            })
            .collect();

        let inc_breaker = crate::incremental::IncrementalPageBreaker::new(content_height);
        let (slices, break_diag) = inc_breaker.break_pages_incremental(
            &pagination_inputs,
            &previous_slices,
            first_affected_line_idx.unwrap_or(0),
        );

        let mut pages = Vec::new();
        for (page_idx, slice) in slices.into_iter().enumerate() {
            let page = self.new_page(
                page_idx + 1,
                content_width,
                slice.total_height,
                slice.lines,
                slice.break_token,
                flow_id,
            );
            pages.push(page);
        }

        if pages.is_empty() {
            pages.push(self.new_page(1, content_width, content_height, Vec::new(), None, flow_id));
        }

        let total = pages.len();
        let diag = crate::incremental::IncrementalTypesetDiagnostics {
            total_pages: total,
            first_affected_page: break_diag.first_affected_page,
            recomputed_pages: break_diag.recomputed_pages_count,
            reused_pages: total.saturating_sub(break_diag.recomputed_pages_count),
            recomputed_paragraphs: recomputed_paras_count,
            reused_paragraphs: reused_paras_count,
            zero_cascade: break_diag.zero_cascade,
            converged_at_page: break_diag.converged_at_page,
            duration: start_time.elapsed(),
        };

        (pages, diag)
    }

    /// Synchronizes bidirectional dependency tracking between document nodes and physical pages,
    /// and resolves cross-references to their target physical pages.
    /// Returns the number of updated cross-references.
    pub fn update_cross_references_and_dependencies(
        &self,
        doc: &mut DocumentRoot,
        pages: &[PageLayoutBox],
    ) -> usize {
        let mut updated_refs = 0;

        // 1. Update DocumentDependencyTracker if present
        if let Some(tracker) = &mut doc.dependency_tracker {
            tracker.node_to_pages.clear();
            tracker.page_to_nodes.clear();
            tracker.anchor_to_page.clear();

            for (page_idx, page) in pages.iter().enumerate() {
                for frame in &page.frames {
                    for line in &frame.lines {
                        if let Some(node_id) = line.paragraph_id {
                            tracker.record_placement(node_id, page_idx);
                        }
                    }
                }
            }

            // Record anchors
            if let Some(registry) = &doc.cross_references {
                for (key, target) in &registry.targets {
                    if let Some(pages) = tracker.pages_for_node(target.anchor.node_id) {
                        if let Some(&first_page) = pages.iter().next() {
                            tracker.record_anchor_page(key, first_page);
                        }
                    }
                }
            }

            tracker.clean_after_recompute();
            tracker.advance_revision();
        }

        // 2. Resolve cross-references if present
        let mut target_pages: HashMap<String, usize> = HashMap::new();
        for (page_idx, page) in pages.iter().enumerate() {
            for frame in &page.frames {
                for line in &frame.lines {
                    if let Some(node_id) = line.paragraph_id {
                        if let Some(registry) = &doc.cross_references {
                            for (key, target) in &registry.targets {
                                if target.anchor.node_id == node_id
                                    && !target_pages.contains_key(key)
                                {
                                    target_pages.insert(key.clone(), page_idx);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(registry) = &mut doc.cross_references {
            for xref in &mut registry.references {
                if let Some(&page_idx) = target_pages.get(&xref.target_key) {
                    let page_num = page_idx + 1;
                    let gematria = GematriaEngine::to_hebrew_numeral(page_num);
                    let target_label = registry
                        .targets
                        .get(&xref.target_key)
                        .and_then(|t| t.label.as_deref());
                    let resolved =
                        xref.format_resolved(Some(page_num), Some(&gematria), target_label, None);
                    if xref.cached_resolved_text.as_deref() != Some(&resolved) {
                        xref.resolve_text(resolved);
                        updated_refs += 1;
                    }
                }
            }
        }

        updated_refs
    }

    /// Typesets a multi-flow document (Talmud / Mikraot / commentaries) using MultiFlowSolver.
    pub fn typeset_document_multi_flow(&self, doc: &DocumentRoot) -> Vec<PageLayoutBox> {
        let content = self.content_rect_for_side(SpreadSide::Recto);
        let content_width = content.width;
        let content_height = content.height;

        let Some(first_sec) = doc.sections.first() else {
            return vec![self.new_page(
                1,
                content_width,
                content_height,
                Vec::new(),
                None,
                "gemara",
            )];
        };

        let margin_x = self.config.margin_inner_pt + self.config.margin_outer_pt;
        let margin_y = self.config.margin_top_pt + self.config.margin_bottom_pt;

        // Merge flows and section-level anchors across all sections in the document
        let mut merged_sec = first_sec.clone();
        for sec in doc.sections.iter().skip(1) {
            for flow in &sec.flows {
                if let Some(existing) = merged_sec.flows.iter_mut().find(|f| f.id == flow.id) {
                    existing.paragraphs.extend(flow.paragraphs.iter().cloned());
                } else {
                    merged_sec.flows.push(flow.clone());
                }
            }
            for anchor_def in sec.anchors.anchors.values() {
                merged_sec.anchors.register_anchor(anchor_def.clone());
            }
            for links in sec.anchors.anchor_to_commentaries.values() {
                for link in links {
                    merged_sec.anchors.link_commentary(link.clone());
                }
            }
        }
        let first_sec = &merged_sec;

        // 1. Synthesize concrete LayoutTemplate via DocumentClassifier and TemplateSynthesizer
        let features = DocumentLayoutFeatures::from_section(first_sec);
        let classification = DocumentClassifier::classify(&features);
        let template = TemplateSynthesizer::synthesize(
            doc,
            first_sec,
            &classification.family,
            self.config.page_width_pt,
            self.config.page_height_pt,
            margin_x,
            margin_y,
        )
        .unwrap_or_else(|_| {
            let fallback_family = LayoutFamily::CustomConstraints {
                flow_ids: first_sec.flows.iter().map(|f| f.id.clone()).collect(),
            };
            TemplateSynthesizer::synthesize(
                doc,
                first_sec,
                &fallback_family,
                self.config.page_width_pt,
                self.config.page_height_pt,
                margin_x,
                margin_y,
            )
            .unwrap_or_else(|_| {
                let mut fallback_tmpl = LayoutTemplate::new(LayoutFamily::SingleFlow {
                    flow_id: FlowId::main(),
                });
                fallback_tmpl.flow_specs = first_sec
                    .flows
                    .iter()
                    .enumerate()
                    .map(|(idx, f)| FlowGeometrySpec::new(f.id.clone(), (idx + 1) as u8))
                    .collect();
                fallback_tmpl
            })
        });

        let flow_specs: Vec<FlowGeometrySpec> = template.flow_specs.clone();

        // Solve nominal column allocations to determine column width for each flow
        let nominal_allocations = MultiFlowSolver::solve_generic_spread(
            self.config.page_width_pt,
            self.config.page_height_pt,
            self.config.margin_inner_pt,
            self.config.margin_outer_pt,
            self.config.margin_top_pt,
            &flow_specs,
            SpreadSide::Recto,
        );

        // Layout lines for each flow and retain paragraph metadata for dynamic width re-breaking
        let mut flow_lines_map: std::collections::BTreeMap<String, Vec<LineBox>> =
            std::collections::BTreeMap::new();
        let mut flow_nominal_width: std::collections::BTreeMap<String, f32> =
            std::collections::BTreeMap::new();
        let mut para_lookup: std::collections::BTreeMap<
            tok_core::id::NodeId,
            (
                ParagraphNode,
                tok_core::styles::ParagraphStyle,
                String,
                f32,
                f32,
            ),
        > = std::collections::BTreeMap::new();
        let mut sessions: std::collections::BTreeMap<String, ShapingSession<'_>> =
            std::collections::BTreeMap::new();

        let has_grep_rules = !doc.grep_rules.is_empty();
        for flow in &first_sec.flows {
            let base_flow_width = nominal_allocations
                .iter()
                .find(|a| a.flow_id == flow.id)
                .map(|a| a.allocated_width_pt)
                .unwrap_or(content_width);

            let app_cfg = doc.find_apparatus(&flow.id.0);
            let flow_width = if let Some(app) = app_cfg {
                let cols = app.columns.max(1) as f32;
                if cols > 1.0 {
                    let total_gaps = (cols - 1.0) * app.column_gap_pt;
                    ((base_flow_width - total_gaps) / cols).max(20.0)
                } else {
                    base_flow_width
                }
            } else {
                base_flow_width
            };
            flow_nominal_width.insert(flow.id.0.clone(), flow_width);

            let is_primary = flow_specs
                .iter()
                .find(|s| s.flow_id == flow.id)
                .map(|s| s.role == FlowPlacementRole::Primary)
                .unwrap_or(false);

            let mut lines = Vec::new();
            for p in &flow.paragraphs {
                let mut p_eff;
                let p_ref = if has_grep_rules {
                    p_eff = p.clone();
                    p_eff.spans = doc.effective_spans_for_paragraph(p, Some(&flow.id.0));
                    &p_eff
                } else {
                    p
                };
                let style = p.effective_style(&doc.paragraph_styles);

                let (mut font_family, mut font_size, mut line_height) = {
                    let (font, sz, lh) = (
                        self.font_manager
                            .face_for(&style.font_family, style.font_weight),
                        style.font_size_pt,
                        style.line_height_pt,
                    );
                    if doc.paragraph_styles.iter().any(|s| s.id == p.style_id)
                        || !p.style_patches.is_empty()
                    {
                        (font, sz, lh)
                    } else {
                        let font = if is_primary {
                            "Frank Ruhl Libre"
                        } else {
                            "Noto Rashi Hebrew"
                        };
                        let sz = if is_primary { 13.5 } else { 10.5 };
                        (font.to_string(), sz, sz * 1.45)
                    }
                };

                if let Some(app) = app_cfg {
                    if let Some(ref fam) = app.font_family {
                        font_family = self.font_manager.face_for(fam, style.font_weight);
                    }
                    if let Some(sz) = app.font_size_pt {
                        if sz > 0.0 {
                            font_size = sz;
                        }
                    }
                    if let Some(lh) = app.line_height_pt {
                        if lh > 0.0 {
                            line_height = lh;
                        }
                    }
                }

                para_lookup.insert(
                    p.id,
                    (
                        p_ref.clone(),
                        style.clone(),
                        font_family.clone(),
                        font_size,
                        line_height,
                    ),
                );

                let key =
                    crate::layout_cache::ParagraphLayoutCache::make_key(p_ref, &style, flow_width);
                let cached_lines = {
                    let cache = self.paragraph_cache.read().unwrap();
                    cache.get(&key).map(|c| c.lines.clone())
                };

                let p_lines = if let Some(hit) = cached_lines {
                    hit
                } else if style.drop_cap.as_ref().is_some_and(|dc| dc.enabled) {
                    let profile = MeasureProfile::uniform(flow_width);
                    let computed = self.typeset_paragraph_shaped_with_style(
                        p_ref,
                        Some(&style),
                        &font_family,
                        &profile,
                        font_size,
                        line_height,
                    );
                    let mut cache = self.paragraph_cache.write().unwrap();
                    cache.insert(key, computed.clone());
                    computed
                } else {
                    let session = sessions
                        .entry(font_family.clone())
                        .or_insert_with(|| self.font_manager.session(&font_family));
                    let profile = MeasureProfile::uniform(flow_width);
                    let computed = Self::typeset_with_session_shaped_full(
                        session,
                        p_ref,
                        Some(&style),
                        &profile,
                        font_size,
                        line_height,
                        Some(&self.config.masoretic_typography),
                    );
                    let mut cache = self.paragraph_cache.write().unwrap();
                    cache.insert(key, computed.clone());
                    computed
                };
                lines.extend(p_lines);
            }
            flow_lines_map.insert(flow.id.0.clone(), lines);
        }

        let _has_notes = flow_specs
            .iter()
            .any(|s| s.role == FlowPlacementRole::BottomBand);

        let primary_spec_opt = flow_specs
            .iter()
            .find(|s| s.role == FlowPlacementRole::Primary);
        let primary_flow_id: FlowId = primary_spec_opt
            .map(|s| s.flow_id.clone())
            .or_else(|| {
                first_sec
                    .flows
                    .iter()
                    .find(|f| f.flow_type == FlowType::Main)
                    .map(|f| f.id.clone())
            })
            .unwrap_or_else(|| FlowId::new("gemara"));

        let mut anchor_map = tok_core::multi_flow_anchor::MultiFlowAnchorMap::default();
        if let Some(ref map) = doc.multi_flow_anchors {
            for a in map.anchors.values() {
                anchor_map.register_anchor(a.clone());
            }
            for links in map.anchor_to_commentaries.values() {
                for link in links {
                    anchor_map.link_commentary(link.clone());
                }
            }
        }
        for a in first_sec.anchors.anchors.values() {
            anchor_map.register_anchor(a.clone());
        }
        for links in first_sec.anchors.anchor_to_commentaries.values() {
            for link in links {
                anchor_map.link_commentary(link.clone());
            }
        }
        if anchor_map.anchors.is_empty() {
            if let Some(pf) = first_sec.flows.iter().find(|f| f.id == primary_flow_id) {
                let comms: Vec<&Flow> = first_sec
                    .flows
                    .iter()
                    .filter(|f| f.id != primary_flow_id)
                    .collect();
                anchor_map = tok_core::multi_flow_anchor::MultiFlowAnchorMap::synthesize_or_default(
                    pf, &comms,
                );
            }
        }

        let mut cumulative_placed_primary_paragraphs: std::collections::BTreeSet<
            tok_core::id::NodeId,
        > = std::collections::BTreeSet::new();

        let mut pages = Vec::new();
        let mut current_page_num = 1;

        let mut flow_cursors: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for flow in &first_sec.flows {
            flow_cursors.insert(flow.id.0.clone(), 0);
        }

        loop {
            let side = if current_page_num % 2 == 1 {
                SpreadSide::Recto
            } else {
                SpreadSide::Verso
            };

            let fn_flow_specs: Vec<&FlowGeometrySpec> = flow_specs
                .iter()
                .filter(|s| {
                    s.role == FlowPlacementRole::BottomBand
                        || s.flow_id.0 == "footnote"
                        || s.flow_id.0.contains("note")
                        || doc.find_apparatus(&s.flow_id.0).is_some()
                })
                .collect();

            // Prepare paginator configs for all apparatuses
            let mut apparatus_demands: Vec<(&str, &[LineBox], FootnotePaginatorConfig)> =
                Vec::new();
            for fn_s in &fn_flow_specs {
                let f_id = fn_s.flow_id.0.as_str();
                let cur = flow_cursors.get(f_id).copied().unwrap_or(0);
                if let Some(lines) = flow_lines_map.get(f_id) {
                    let remaining = &lines[cur..];
                    if !remaining.is_empty() {
                        let p_cfg = doc
                            .find_apparatus(f_id)
                            .map(FootnotePaginatorConfig::from_apparatus_config)
                            .unwrap_or_else(|| FootnotePaginatorConfig {
                                apparatus_id: f_id.to_string(),
                                ..Default::default()
                            });
                        apparatus_demands.push((f_id, remaining, p_cfg));
                    }
                }
            }

            let app_tuples: Vec<(&str, &[LineBox], &FootnotePaginatorConfig)> = apparatus_demands
                .iter()
                .map(|(id, l, c)| (*id, *l, c))
                .collect();

            let budgeted_fn_heights = DynamicFootnotePaginator::calculate_multi_apparatus_heights(
                &app_tuples,
                content_height,
            );

            let footnote_target = if budgeted_fn_heights.is_empty() {
                None
            } else {
                Some(budgeted_fn_heights.iter().map(|(_, h)| *h).sum())
            };

            // Determine active flows on this page (flows with unplaced lines)
            let mut active_specs: Vec<FlowGeometrySpec> = flow_specs
                .iter()
                .filter(|s| {
                    let cur = flow_cursors.get(&s.flow_id.0).copied().unwrap_or(0);
                    let total = flow_lines_map.get(&s.flow_id.0).map_or(0, |v| v.len());
                    cur < total
                })
                .cloned()
                .collect();

            // Set each footnote flow's target_height_pt from its budgeted apparatus height
            for s in &mut active_specs {
                if let Some((_, h)) = budgeted_fn_heights
                    .iter()
                    .find(|(id, _)| *id == s.flow_id.0)
                {
                    s.target_height_pt = *h;
                }
            }

            let current_specs = if active_specs.is_empty() {
                &flow_specs[..]
            } else {
                &active_specs[..]
            };

            // Determine if primary stream finishes early on this page
            let primary_spec = current_specs
                .iter()
                .find(|s| s.role == FlowPlacementRole::Primary);
            let primary_target_h = if let Some(p_spec) = primary_spec {
                let p_cursor = flow_cursors.get(&p_spec.flow_id.0).copied().unwrap_or(0);
                if let Some(lines) = flow_lines_map.get(&p_spec.flow_id.0) {
                    let remaining_lines = &lines[p_cursor..];
                    let rem_h: f32 = remaining_lines
                        .iter()
                        .filter(|l| !l.is_drop_cap)
                        .map(|l| l.height)
                        .sum();
                    let max_h = self.config.page_height_pt - 2.0 * self.config.margin_top_pt;
                    if rem_h > 0.0 && rem_h < max_h - MultiFlowSolver::L_SHAPE_MIN_GAP_PT {
                        Some(rem_h)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let comm_termination = if primary_target_h.is_none() {
                current_specs
                    .iter()
                    .filter(|s| {
                        s.role != FlowPlacementRole::Primary
                            && s.role != FlowPlacementRole::BottomBand
                    })
                    .find_map(|s| {
                        let cur = flow_cursors.get(&s.flow_id.0).copied().unwrap_or(0);
                        if let Some(lines) = flow_lines_map.get(&s.flow_id.0) {
                            let rem_lines = &lines[cur..];
                            let rem_h: f32 = rem_lines
                                .iter()
                                .filter(|l| !l.is_drop_cap)
                                .map(|l| l.height)
                                .sum();
                            let max_h =
                                self.config.page_height_pt - 2.0 * self.config.margin_top_pt;
                            if rem_h > 0.0 && rem_h < max_h - MultiFlowSolver::L_SHAPE_MIN_GAP_PT {
                                Some((s.flow_id.clone(), rem_h))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
            } else {
                None
            };

            let expansion_id_derived = current_specs
                .iter()
                .filter(|s| {
                    s.role != FlowPlacementRole::Primary && s.role != FlowPlacementRole::BottomBand
                })
                .max_by_key(|s| {
                    let cur = flow_cursors.get(&s.flow_id.0).copied().unwrap_or(0);
                    let total = flow_lines_map.get(&s.flow_id.0).map_or(0, |v| v.len());
                    total.saturating_sub(cur)
                })
                .map(|s| &s.flow_id);
            let expansion_id = template.expansion_flow_id.as_ref().or(expansion_id_derived);
            let dynamic_result = MultiFlowSolver::solve_dynamic_spread_with_commentary_absorption(
                self.config.page_width_pt,
                self.config.page_height_pt,
                self.config.margin_inner_pt,
                self.config.margin_outer_pt,
                self.config.margin_top_pt,
                current_specs,
                side,
                primary_target_h,
                footnote_target,
                expansion_id,
                comm_termination,
            );

            let mut page_frames = Vec::new();
            let mut any_lines_placed = false;
            let mut page_primary_para_top_y: std::collections::BTreeMap<tok_core::id::NodeId, f32> =
                std::collections::BTreeMap::new();

            // Place primary column first, then primary expansion, then commentary columns, then commentary expansions
            let mut sorted_allocations = dynamic_result.allocations.clone();
            sorted_allocations.sort_by_key(|alloc| {
                let is_exp = alloc.flow_id.0.ends_with("_expansion");
                let is_primary = alloc.flow_id == primary_flow_id
                    || alloc.flow_id.0 == format!("{}_expansion", primary_flow_id.0);
                if is_primary && !is_exp {
                    0
                } else if is_primary && is_exp {
                    1
                } else if !is_exp {
                    2
                } else {
                    3
                }
            });

            for alloc in &sorted_allocations {
                let (is_expansion, source_id) =
                    if let Some(base) = alloc.flow_id.0.strip_suffix("_expansion") {
                        (true, base)
                    } else {
                        (false, alloc.flow_id.0.as_str())
                    };

                let is_primary_flow = source_id == primary_flow_id.0.as_str();
                let cursor = flow_cursors.get(source_id).copied().unwrap_or(0);
                let nominal_w = flow_nominal_width
                    .get(source_id)
                    .copied()
                    .unwrap_or(alloc.allocated_width_pt);

                let mut frame_lines: Vec<LineBox> = Vec::new();
                let mut current_h = 0.0_f32;
                let mut active_drop_cap_bottom = 0.0_f32;

                // Snapshot commentary pools for primary overflow holdback checks
                let comm_heights_snapshot: std::collections::BTreeMap<
                    String,
                    Vec<(Option<tok_core::id::NodeId>, f32)>,
                > = if is_primary_flow {
                    flow_lines_map
                        .iter()
                        .filter(|(k, _)| k.as_str() != source_id)
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                v.iter()
                                    .map(|l| {
                                        (l.paragraph_id, if l.is_drop_cap { 0.0 } else { l.height })
                                    })
                                    .collect(),
                            )
                        })
                        .collect()
                } else {
                    std::collections::BTreeMap::new()
                };

                if let Some(pool) = flow_lines_map.get_mut(source_id) {
                    let mut idx = cursor;
                    while idx < pool.len() {
                        let prev_pid = if idx > cursor {
                            pool[idx - 1].paragraph_id
                        } else {
                            None
                        };
                        let curr_pid = pool[idx].paragraph_id;
                        let is_para_start = curr_pid.is_some() && prev_pid != curr_pid;

                        // If we are in a wider expansion frame at the start of a paragraph (or continuation slice),
                        // dynamically re-break the paragraph's remaining text for alloc.allocated_width_pt using Knuth-Plass.
                        if is_expansion && alloc.allocated_width_pt > nominal_w + 5.0 {
                            if let Some(pid) = curr_pid {
                                let para_end = idx
                                    + pool[idx..]
                                        .iter()
                                        .take_while(|line| line.paragraph_id == Some(pid))
                                        .count();
                                let needs_rebreak = pool[idx..para_end]
                                    .iter()
                                    .any(|line| !line.is_drop_cap && line.width <= nominal_w + 2.0);
                                if needs_rebreak {
                                    if let Some((p_node, p_style, font_fam, font_sz, line_h)) =
                                        para_lookup.get(&pid)
                                    {
                                        let is_full_para =
                                            idx == 0 || pool[idx - 1].paragraph_id != Some(pid);
                                        let full_text = p_node.text.as_str();
                                        let min_cluster = if is_full_para {
                                            0usize
                                        } else {
                                            pool[idx..para_end]
                                                .iter()
                                                .flat_map(|l| l.glyphs.iter())
                                                .map(|g| g.cluster as usize)
                                                .min()
                                                .unwrap_or(0)
                                        };
                                        if min_cluster < full_text.len()
                                            && full_text.is_char_boundary(min_cluster)
                                        {
                                            let sub_text = &full_text[min_cluster..];
                                            let mut sub_para = ParagraphNode::new(
                                                p_node.index.clone(),
                                                p_node.style_id.clone(),
                                                sub_text,
                                            );
                                            sub_para.id = pid;
                                            sub_para.style_patches = p_node.style_patches.clone();
                                            let profile =
                                                MeasureProfile::uniform(alloc.allocated_width_pt);
                                            let mut rebroken = if is_full_para
                                                && p_style
                                                    .drop_cap
                                                    .as_ref()
                                                    .is_some_and(|dc| dc.enabled)
                                            {
                                                self.typeset_paragraph_shaped_with_style(
                                                    &sub_para,
                                                    Some(p_style),
                                                    font_fam,
                                                    &profile,
                                                    *font_sz,
                                                    *line_h,
                                                )
                                            } else {
                                                let session = sessions
                                                    .entry(font_fam.clone())
                                                    .or_insert_with(|| {
                                                        self.font_manager.session(font_fam)
                                                    });
                                                Self::typeset_with_session_shaped_opts(
                                                    session,
                                                    &sub_para,
                                                    Some(p_style),
                                                    &profile,
                                                    *font_sz,
                                                    *line_h,
                                                )
                                            };
                                            if min_cluster > 0 {
                                                for rb_line in &mut rebroken {
                                                    for g in &mut rb_line.glyphs {
                                                        g.cluster += min_cluster as u32;
                                                    }
                                                }
                                            }
                                            if !rebroken.is_empty() {
                                                pool.splice(idx..para_end, rebroken);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let l = &pool[idx];

                        // Invariant 1 (Living Tzurat HaDaf): Commentary paragraph cannot appear before its primary anchor paragraph
                        if !is_primary_flow {
                            if let Some(pid) = l.paragraph_id {
                                if let Some(anchor_def) = anchor_map.get_anchor_for_commentary(pid)
                                {
                                    if !cumulative_placed_primary_paragraphs
                                        .contains(&anchor_def.primary_node_id)
                                    {
                                        break;
                                    }
                                }
                            }
                        }

                        // Cross-page commentary overflow holdback: if at least one primary paragraph is already placed on
                        // this page and placing a new primary paragraph `pid` would cause its anchored commentary to overflow
                        // an already-full commentary column on this page, hold `pid` back to the next page.
                        if is_primary_flow
                            && !is_expansion
                            && is_para_start
                            && !frame_lines.is_empty()
                        {
                            if let Some(pid) = l.paragraph_id {
                                let linked_anchors =
                                    anchor_map.get_commentaries_for_primary_node(pid);
                                let mut should_hold_back = false;
                                for anchor in linked_anchors {
                                    for (comm_fid, comm_lines) in &comm_heights_snapshot {
                                        let comm_cur =
                                            flow_cursors.get(comm_fid).copied().unwrap_or(0);
                                        if let Some(rel_pos) =
                                            comm_lines[comm_cur..].iter().position(|(cid, _)| {
                                                *cid == Some(anchor.commentary_node_id)
                                            })
                                        {
                                            let target_start = comm_cur + rel_pos;
                                            let prior_comm_h: f32 = comm_lines
                                                [comm_cur..target_start]
                                                .iter()
                                                .map(|(_, h)| *h)
                                                .sum();
                                            let target_comm_h: f32 = comm_lines[target_start..]
                                                .iter()
                                                .take_while(|(cid, _)| {
                                                    *cid == Some(anchor.commentary_node_id)
                                                })
                                                .take(2)
                                                .map(|(_, h)| *h)
                                                .sum();
                                            let comm_avail_h = sorted_allocations
                                                .iter()
                                                .filter(|a| {
                                                    a.flow_id.0 == *comm_fid
                                                        || a.flow_id.0
                                                            == format!("{}_expansion", comm_fid)
                                                })
                                                .map(|a| a.allocated_height_pt)
                                                .sum::<f32>()
                                                .max(alloc.allocated_height_pt);
                                            if prior_comm_h > 0.0
                                                && prior_comm_h + target_comm_h > comm_avail_h
                                            {
                                                should_hold_back = true;
                                                break;
                                            }
                                        }
                                    }
                                    if should_hold_back {
                                        break;
                                    }
                                }
                                if should_hold_back {
                                    break;
                                }
                            }
                        }

                        // Intra-page vertical spring alignment: advance commentary paragraph start toward its anchored
                        // primary paragraph's top Y on the same page within available column slack.
                        if !is_primary_flow && !is_expansion && is_para_start {
                            if let Some(cid) = l.paragraph_id {
                                if let Some(anchor_def) = anchor_map.get_anchor_for_commentary(cid)
                                {
                                    if let Some(&primary_top_y) =
                                        page_primary_para_top_y.get(&anchor_def.primary_node_id)
                                    {
                                        if primary_top_y > current_h {
                                            let rem_eligible_h: f32 = pool[idx..]
                                                .iter()
                                                .take_while(|line| {
                                                    line.paragraph_id
                                                        .and_then(|id| {
                                                            anchor_map.get_anchor_for_commentary(id)
                                                        })
                                                        .is_none_or(|a| {
                                                            cumulative_placed_primary_paragraphs
                                                                .contains(&a.primary_node_id)
                                                        })
                                                })
                                                .filter(|line| !line.is_drop_cap)
                                                .map(|line| line.height)
                                                .sum();
                                            let slack = (alloc.allocated_height_pt
                                                - current_h
                                                - rem_eligible_h)
                                                .max(0.0);
                                            let shift = (primary_top_y - current_h).min(slack);
                                            current_h += shift;
                                        }
                                    }
                                }
                            }
                        }

                        // Capacity check and widow protection
                        if current_h + l.height > alloc.allocated_height_pt + 0.05
                            && !frame_lines.is_empty()
                        {
                            if let Some(pid) = l.paragraph_id {
                                let para_lines_rem = pool[idx..]
                                    .iter()
                                    .take_while(|line| line.paragraph_id == Some(pid))
                                    .filter(|line| !line.is_drop_cap)
                                    .count();
                                if para_lines_rem == 1
                                    && frame_lines.len() >= 2
                                    && current_h > active_drop_cap_bottom
                                {
                                    let _ = frame_lines.pop().unwrap();
                                    idx -= 1;
                                }
                            }
                            break;
                        }

                        // Orphan protection at paragraph start
                        if let Some(pid) = l.paragraph_id {
                            if is_para_start && !frame_lines.is_empty() {
                                let avail_h = alloc.allocated_height_pt - current_h;
                                let para_slice: Vec<&LineBox> = pool[idx..]
                                    .iter()
                                    .take_while(|line| line.paragraph_id == Some(pid))
                                    .filter(|line| !line.is_drop_cap)
                                    .collect();
                                let para_total = para_slice.len();
                                let fit_count = para_slice
                                    .iter()
                                    .scan(0.0_f32, |acc, line| {
                                        *acc += line.height;
                                        Some(*acc)
                                    })
                                    .take_while(|&h| h <= avail_h + 0.05)
                                    .count();
                                if para_total >= 2 && fit_count < 2 {
                                    break;
                                }
                            }
                        }

                        let mut positioned_line = l.clone();
                        // Fallback single-line re-justification if para_lookup was unavailable
                        if is_expansion
                            && !positioned_line.is_drop_cap
                            && alloc.allocated_width_pt > positioned_line.width + 5.0
                            && positioned_line
                                .paragraph_id
                                .is_none_or(|pid| !para_lookup.contains_key(&pid))
                        {
                            let font_sz = positioned_line
                                .glyphs
                                .first()
                                .map(|g| g.height)
                                .unwrap_or(DEFAULT_FONT_SIZE_PT);
                            let p_glyphs: Vec<PositionedGlyph> = positioned_line
                                .glyphs
                                .iter()
                                .map(|g| PositionedGlyph {
                                    glyph_id: g.glyph_id,
                                    cluster: g.cluster,
                                    x_advance: g.width,
                                    y_advance: 0.0,
                                    x_offset: 0.0,
                                    y_offset: g.y,
                                    character: g.character,
                                })
                                .collect();
                            let justified = HebrewJustifier::justify_line(
                                p_glyphs,
                                alloc.allocated_width_pt,
                                font_sz,
                                false,
                            );
                            let new_w: f32 = justified.glyphs.iter().map(|g| g.x_advance).sum();
                            let mut curr_x = HebrewJustifier::calculate_line_start_x(
                                new_w,
                                alloc.allocated_width_pt,
                                0.0,
                                positioned_line.is_rtl,
                            );
                            positioned_line.width = new_w;
                            positioned_line.glyphs = justified
                                .glyphs
                                .into_iter()
                                .zip(positioned_line.glyphs.iter())
                                .map(|(jg, orig_g)| {
                                    let b = GlyphBox {
                                        glyph_id: jg.glyph_id,
                                        cluster: jg.cluster,
                                        x: curr_x + jg.x_offset,
                                        y: jg.y_offset,
                                        width: jg.x_advance,
                                        height: orig_g.height,
                                        character: jg.character,
                                        font_index: orig_g.font_index,
                                    };
                                    curr_x += jg.x_advance;
                                    b
                                })
                                .collect();
                        }

                        if is_primary_flow {
                            if let Some(pid) = positioned_line.paragraph_id {
                                page_primary_para_top_y.entry(pid).or_insert(current_h);
                                cumulative_placed_primary_paragraphs.insert(pid);
                            }
                        }

                        if positioned_line.is_drop_cap {
                            let local_base = if positioned_line.baseline_y > 0.0 {
                                positioned_line.baseline_y
                            } else {
                                positioned_line.height * 0.82
                            };
                            positioned_line.baseline_y = current_h + local_base;
                            positioned_line.line_index = frame_lines.len();
                            active_drop_cap_bottom =
                                active_drop_cap_bottom.max(current_h + positioned_line.height);
                        } else {
                            positioned_line.baseline_y = current_h + positioned_line.height;
                            positioned_line.line_index = frame_lines.len();
                            current_h += positioned_line.height;
                            let next_same_para = pool
                                .get(idx + 1)
                                .is_some_and(|nl| nl.paragraph_id == positioned_line.paragraph_id);
                            if !next_same_para && current_h < active_drop_cap_bottom {
                                current_h = active_drop_cap_bottom;
                            }
                        }

                        frame_lines.push(positioned_line);
                        idx += 1;
                    }

                    // If we re-broke a paragraph for a wider expansion frame and it partially overflowed the
                    // expansion frame onto the next page, re-break the unplaced remainder back to nominal_w.
                    if is_expansion
                        && alloc.allocated_width_pt > nominal_w + 5.0
                        && idx < pool.len()
                    {
                        if let Some(pid) = pool[idx].paragraph_id {
                            let para_end = idx
                                + pool[idx..]
                                    .iter()
                                    .take_while(|line| line.paragraph_id == Some(pid))
                                    .count();
                            let has_wide_lines = pool[idx..para_end]
                                .iter()
                                .any(|line| line.width > nominal_w + 2.0);
                            if has_wide_lines {
                                if let Some((p_node, p_style, font_fam, font_sz, line_h)) =
                                    para_lookup.get(&pid)
                                {
                                    let full_text = p_node.text.as_str();
                                    let min_cluster = pool[idx..para_end]
                                        .iter()
                                        .flat_map(|l| l.glyphs.iter())
                                        .map(|g| g.cluster as usize)
                                        .min()
                                        .unwrap_or(0);
                                    if min_cluster < full_text.len()
                                        && full_text.is_char_boundary(min_cluster)
                                    {
                                        let sub_text = &full_text[min_cluster..];
                                        let mut sub_para = ParagraphNode::new(
                                            p_node.index.clone(),
                                            p_node.style_id.clone(),
                                            sub_text,
                                        );
                                        sub_para.id = pid;
                                        sub_para.style_patches = p_node.style_patches.clone();
                                        let session = sessions
                                            .entry(font_fam.clone())
                                            .or_insert_with(|| self.font_manager.session(font_fam));
                                        let profile = MeasureProfile::uniform(nominal_w);
                                        let mut rebroken = Self::typeset_with_session_shaped_opts(
                                            session,
                                            &sub_para,
                                            Some(p_style),
                                            &profile,
                                            *font_sz,
                                            *line_h,
                                        );
                                        if min_cluster > 0 {
                                            for rb_line in &mut rebroken {
                                                for g in &mut rb_line.glyphs {
                                                    g.cluster += min_cluster as u32;
                                                }
                                            }
                                        }
                                        if !rebroken.is_empty() {
                                            pool.splice(idx..para_end, rebroken);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let lines_placed = idx - cursor;
                    if lines_placed > 0 {
                        flow_cursors.insert(source_id.to_string(), cursor + lines_placed);
                        any_lines_placed = true;
                    }
                }

                if is_expansion && frame_lines.is_empty() {
                    continue;
                }

                page_frames.push(TextFrameBox {
                    frame_id: format!("frame_{}_{}", current_page_num, alloc.flow_id.0),
                    flow_id: alloc.flow_id.0.clone(),
                    rect: PhysicalRect::new(
                        alloc.allocated_x_pt,
                        alloc.allocated_y_pt,
                        alloc.allocated_width_pt,
                        alloc.allocated_height_pt,
                    ),
                    lines: frame_lines,
                });
            }

            // Place footnotes if present (supports multiple independent parallel apparatuses)
            let fn_allocs = if !dynamic_result.footnote_allocations.is_empty() {
                dynamic_result.footnote_allocations.clone()
            } else if let Some(fn_alloc) = &dynamic_result.footnote_allocation {
                vec![fn_alloc.clone()]
            } else {
                Vec::new()
            };

            for fn_alloc in fn_allocs {
                let notes_id = fn_alloc.flow_id.0.clone();
                let p_cfg = doc
                    .find_apparatus(&notes_id)
                    .map(FootnotePaginatorConfig::from_apparatus_config)
                    .unwrap_or_else(|| FootnotePaginatorConfig {
                        apparatus_id: notes_id.clone(),
                        ..Default::default()
                    });

                let lines_pool = flow_lines_map.get(&notes_id);
                let cursor = flow_cursors.get(&notes_id).copied().unwrap_or(0);
                let mut frame_lines: Vec<LineBox> = Vec::new();

                if let Some(pool) = lines_pool {
                    let remaining = &pool[cursor..];
                    let fn_slice = DynamicFootnotePaginator::slice_for_allocated_band_with_width(
                        remaining,
                        fn_alloc.allocated_width_pt,
                        fn_alloc.allocated_height_pt,
                        &p_cfg,
                        true,
                    );
                    if fn_slice.lines_consumed > 0 {
                        flow_cursors.insert(notes_id.clone(), cursor + fn_slice.lines_consumed);
                        any_lines_placed = true;
                    }
                    frame_lines = fn_slice.placed_lines;
                }

                page_frames.push(TextFrameBox {
                    frame_id: format!("frame_{}_{}", current_page_num, notes_id),
                    flow_id: notes_id,
                    rect: PhysicalRect::new(
                        fn_alloc.allocated_x_pt,
                        fn_alloc.allocated_y_pt,
                        fn_alloc.allocated_width_pt,
                        fn_alloc.allocated_height_pt,
                    ),
                    lines: frame_lines,
                });
            }

            // Restore canonical allocation ordering
            let orig_order: Vec<String> = dynamic_result
                .allocations
                .iter()
                .map(|a| a.flow_id.0.clone())
                .collect();
            page_frames.sort_by_key(|f| {
                orig_order
                    .iter()
                    .position(|id| id == &f.flow_id)
                    .unwrap_or(999)
            });

            // Apply Shared Column Height Balancing across parallel column and expansion frames
            let height_balance_config = crate::co_pagination::HeightBalanceConfig {
                max_line_expansion_pt: 3.5,
                max_paragraph_expansion_pt: 16.0,
                min_lines_for_balance: 3,
                target_mode: crate::co_pagination::BalanceTargetMode::TallestColumn,
            };
            let mut bal_frames: Vec<TextFrameBox> = std::mem::take(&mut page_frames);
            let col_indices: Vec<usize> = bal_frames
                .iter()
                .enumerate()
                .filter(|(_, f)| {
                    !f.flow_id.contains("note")
                        && f.lines.len() >= height_balance_config.min_lines_for_balance
                })
                .map(|(i, _)| i)
                .collect();

            if col_indices.len() >= 2 {
                let mut cols_subset: Vec<TextFrameBox> =
                    col_indices.iter().map(|&i| bal_frames[i].clone()).collect();
                crate::co_pagination::SharedColumnHeightBalancer::balance_page_frames(
                    &mut cols_subset,
                    &height_balance_config,
                );
                for (subset_idx, &orig_idx) in col_indices.iter().enumerate() {
                    bal_frames[orig_idx] = cols_subset[subset_idx].clone();
                }
            }
            page_frames = bal_frames;

            let all_exhausted = flow_cursors.iter().all(|(fid, &idx)| {
                let total = flow_lines_map.get(fid).map_or(0, |v| v.len());
                idx >= total
            });

            let break_token = if !all_exhausted {
                let continuing_flow = first_sec
                    .flows
                    .iter()
                    .find(|f| {
                        let cur = flow_cursors.get(&f.id.0).copied().unwrap_or(0);
                        let total = flow_lines_map.get(&f.id.0).map_or(0, |v| v.len());
                        cur < total
                    })
                    .or_else(|| first_sec.flows.first());

                continuing_flow.and_then(|f| {
                    let cursor = flow_cursors.get(&f.id.0).copied().unwrap_or(0);
                    let lines = flow_lines_map.get(&f.id.0)?;
                    lines.get(cursor).map(|line| BreakToken {
                        section_index: 0,
                        paragraph_index: cursor,
                        char_offset: line
                            .glyphs
                            .iter()
                            .map(|g| g.cluster as usize)
                            .min()
                            .unwrap_or(0),
                        paragraph_id: line.paragraph_id,
                    })
                })
            } else {
                None
            };

            pages.push(PageLayoutBox {
                page_index: current_page_num - 1,
                page_number_gematria: GematriaEngine::to_hebrew_numeral(current_page_num),
                dimensions: PhysicalRect::new(
                    0.0,
                    0.0,
                    self.config.page_width_pt,
                    self.config.page_height_pt,
                ),
                frames: page_frames,
                break_token,
            });

            current_page_num += 1;

            if all_exhausted || !any_lines_placed || current_page_num > 500 {
                break;
            }
        }

        pages
    }

    /// Incrementally typesets a multi-flow document with precise paragraph layout caching
    /// across all streams (Gemara, commentaries, footnotes) and zero-cascade fast path.
    pub fn typeset_document_multi_flow_incremental(
        &self,
        doc: &DocumentRoot,
        previous_pages: &[PageLayoutBox],
        modified_node_id: Option<tok_core::id::NodeId>,
    ) -> (
        Vec<PageLayoutBox>,
        crate::incremental::IncrementalTypesetDiagnostics,
    ) {
        let start_time = std::time::Instant::now();
        if let Some(target_id) = modified_node_id {
            self.invalidate_paragraph(target_id);
        }

        // Fast path: Zero cascade if single paragraph modified and fits in existing frame lines count & heights
        if let Some(target_id) = modified_node_id {
            let mut page_indices = Vec::new();
            for (p_idx, page) in previous_pages.iter().enumerate() {
                for frame in &page.frames {
                    if frame
                        .lines
                        .iter()
                        .any(|l| l.paragraph_id == Some(target_id))
                    {
                        page_indices.push(p_idx);
                        break;
                    }
                }
            }

            if page_indices.len() == 1 {
                let page_idx = page_indices[0];
                let old_page = &previous_pages[page_idx];

                if let Some(first_sec) = doc.sections.first() {
                    for flow in &first_sec.flows {
                        if let Some(p) = flow.paragraphs.iter().find(|p| p.id == target_id) {
                            let prev_frame =
                                old_page.frames.iter().find(|f| f.flow_id == flow.id.0);
                            if let Some(p_frame) = prev_frame {
                                let flow_width = p_frame.rect.width;
                                let old_lines: Vec<&LineBox> = p_frame
                                    .lines
                                    .iter()
                                    .filter(|l| l.paragraph_id == Some(target_id))
                                    .collect();

                                let default_style = tok_core::styles::ParagraphStyle::default();
                                let style = doc
                                    .paragraph_styles
                                    .iter()
                                    .find(|s| s.id == p.style_id)
                                    .unwrap_or(&default_style);

                                let is_primary = flow.flow_type == FlowType::Main;
                                let (font_family, font_size, line_height) = {
                                    let (font, sz, lh) = (
                                        self.font_manager
                                            .face_for(&style.font_family, style.font_weight),
                                        style.font_size_pt,
                                        style.line_height_pt,
                                    );
                                    if doc.paragraph_styles.iter().any(|s| s.id == p.style_id) {
                                        (font, sz, lh)
                                    } else {
                                        let font = if is_primary {
                                            "Frank Ruhl Libre"
                                        } else {
                                            "Noto Rashi Hebrew"
                                        };
                                        let sz = if is_primary { 13.5 } else { 10.5 };
                                        (font.to_string(), sz, sz * 1.45)
                                    }
                                };

                                let session = self.font_manager.session(&font_family);
                                let new_lines = Self::typeset_with_session(
                                    &session,
                                    p,
                                    flow_width,
                                    font_size,
                                    line_height,
                                );

                                if new_lines.len() == old_lines.len() {
                                    let heights_match = new_lines
                                        .iter()
                                        .zip(&old_lines)
                                        .all(|(n, o)| (n.height - o.height).abs() < 1e-3);

                                    if heights_match {
                                        // Cache new lines
                                        let key =
                                            crate::layout_cache::ParagraphLayoutCache::make_key(
                                                p, style, flow_width,
                                            );
                                        self.paragraph_cache
                                            .write()
                                            .unwrap()
                                            .insert(key, new_lines.clone());

                                        let mut new_pages = previous_pages.to_vec();
                                        let page = &mut new_pages[page_idx];
                                        for frame in &mut page.frames {
                                            if frame.flow_id == flow.id.0 {
                                                let mut new_frame_lines =
                                                    Vec::with_capacity(frame.lines.len());
                                                let mut new_lines_iter =
                                                    new_lines.clone().into_iter();
                                                for old_line in &frame.lines {
                                                    let line = if old_line.paragraph_id
                                                        == Some(target_id)
                                                    {
                                                        let mut n = new_lines_iter
                                                            .next()
                                                            .unwrap_or_else(|| old_line.clone());
                                                        n.baseline_y = old_line.baseline_y;
                                                        n
                                                    } else {
                                                        old_line.clone()
                                                    };
                                                    new_frame_lines.push(line);
                                                }
                                                frame.lines = new_frame_lines;
                                            }
                                        }

                                        let total = new_pages.len();
                                        return (
                                            new_pages,
                                            crate::incremental::IncrementalTypesetDiagnostics {
                                                total_pages: total,
                                                first_affected_page: page_idx,
                                                recomputed_pages: 1,
                                                reused_pages: total.saturating_sub(1),
                                                recomputed_paragraphs: 1,
                                                reused_paragraphs: flow
                                                    .paragraphs
                                                    .len()
                                                    .saturating_sub(1),
                                                zero_cascade: true,
                                                converged_at_page: Some(page_idx),
                                                duration: start_time.elapsed(),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Full multi-flow layout reusing cached paragraphs
        let pages = self.typeset_document_multi_flow(doc);
        let total = pages.len();
        (
            pages,
            crate::incremental::IncrementalTypesetDiagnostics {
                total_pages: total,
                first_affected_page: 0,
                recomputed_pages: total,
                reused_pages: 0,
                recomputed_paragraphs: 1,
                reused_paragraphs: 0,
                zero_cascade: false,
                converged_at_page: None,
                duration: start_time.elapsed(),
            },
        )
    }

    /// Co-paginates a multi-stream commentary document (Talmud / Mikraot Gedolot / academic)
    /// across facing pages with active anchor synchronization, automatic overflow splitting,
    /// dynamic L-shape synthesis, and spine-relative margins.
    pub fn typeset_co_paginated_document(
        &self,
        template: &crate::co_pagination::TemplateConfig,
        sync_config: &crate::co_pagination::SynchronizerConfig,
        doc: &crate::co_pagination::CoPaginationDocument,
    ) -> crate::co_pagination::CoPaginationResult {
        crate::co_pagination::CoPaginationEngine::co_paginate(template, sync_config, doc, self)
    }

    /// Spawns a background worker on an `Arc<TypesettingEngine>` to co-paginate a multi-stream
    /// document asynchronously without blocking the UI thread.
    ///
    /// Pages and progress updates are streamed incrementally through the returned `Receiver<PaginationEvent>`,
    /// and the job can be cancelled at any time using the `CancellationToken`.
    pub fn paginate_co_paginated_document_async(
        self: &std::sync::Arc<Self>,
        template: crate::co_pagination::TemplateConfig,
        sync_config: crate::co_pagination::SynchronizerConfig,
        doc: crate::co_pagination::CoPaginationDocument,
    ) -> (
        std::sync::mpsc::Receiver<crate::co_pagination::PaginationEvent>,
        crate::co_pagination::CancellationToken,
    ) {
        crate::co_pagination::NonBlockingPaginator::paginate_async(
            template,
            sync_config,
            doc,
            self.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tok_core::id::FractionalIndex;

    fn engine() -> TypesettingEngine {
        TypesettingEngine::new(TypesettingEngineConfig::default())
    }

    fn para(text: &str) -> ParagraphNode {
        ParagraphNode::new(FractionalIndex::initial(), "default-body", text)
    }

    /// Base characters of a line, left to right (marks skipped).
    fn visual_chars(line: &LineBox) -> String {
        line.glyphs
            .iter()
            .filter(|g| g.width > 0.0)
            .filter_map(|g| g.character)
            .collect()
    }

    #[test]
    fn test_typesetting_document_pages() {
        let mut doc = DocumentRoot::new("ספר בדיקה לעימוד");
        let flow = doc.sections[0].main_flow_mut().unwrap();

        // Add 5 long Hebrew paragraphs
        for i in 0..5 {
            let text = "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ. \
                        וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ וְחֹשֶׁךְ עַל פְּנֵי תְהוֹם וְרוּחַ אֱלֹהִים מְרַחֶפֶת עַל פְּנֵי הַמָּיִם. \
                        וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר וַיְהִי אוֹר. וַיַּרְא אֱלֹהִים אֶת הָאוֹר כִּי טוֹב וַיַּבְדֵּל אֱלֹהִים בֵּין הָאוֹר וּבֵין הַחֹשֶׁךְ.";
            let p = ParagraphNode::new(
                FractionalIndex::new(format!("p{}", i)),
                "default-body",
                text,
            );
            flow.add_paragraph(p);
        }

        let pages = engine().typeset_document(&doc);

        assert!(!pages.is_empty());
        assert_eq!(
            pages[0].page_number_gematria,
            format!("א{}", crate::gematria::HEBREW_GERESH)
        );
        assert!(!pages[0].frames[0].lines.is_empty());
    }

    /// Regression: from the second line of a paragraph on, the paragraph-relative
    /// baseline was added to the page height that already included the earlier
    /// lines, so lines overlapped or left gaps.
    #[test]
    fn baselines_advance_by_one_line_height() {
        let mut doc = DocumentRoot::new("t");
        doc.paragraph_styles[0].space_before_pt = 0.0;
        doc.paragraph_styles[0].space_after_pt = 0.0;
        let flow = doc.sections[0].main_flow_mut().unwrap();
        let long = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה דִּבְרֵי רַבִּי אֱלִיעֶזֶר וַחֲכָמִים אוֹמְרִים עַד חֲצוֹת ".repeat(6);
        for i in 0..40 {
            flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("p{:03}", i)),
                "default-body",
                &long,
            ));
        }
        let pages = engine().typeset_document(&doc);
        assert!(pages.len() > 1);
        for page in &pages {
            let frame = &page.frames[0];
            let mut expected = 0.0;
            for (i, line) in frame.lines.iter().enumerate() {
                expected += line.height;
                assert!(
                    (line.baseline_y - expected).abs() < 1e-3,
                    "page {} line {}: baseline {} expected {}",
                    page.page_index,
                    i,
                    line.baseline_y,
                    expected
                );
                assert_eq!(line.line_index, i);
            }
            assert!(expected <= frame.rect.height + 1e-3);
        }
    }

    /// Regression: words of an RTL paragraph were laid out left to right.
    #[test]
    fn rtl_words_are_in_visual_order() {
        let lines = engine().typeset_paragraph(&para("אב גד"), 300.0, 12.0, 16.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(visual_chars(&lines[0]), "דג בא");
        assert_eq!(lines[0].text, "אב גד");
    }

    /// Regression: Latin words and numbers inside Hebrew were shaped
    /// right-to-left, i.e. printed backwards.
    #[test]
    fn embedded_ltr_runs_keep_their_direction() {
        let lines = engine().typeset_paragraph(&para("שלום abc עולם"), 400.0, 12.0, 16.0);
        assert_eq!(visual_chars(&lines[0]), "םלוע abc םולש");
        let lines = engine().typeset_paragraph(&para("שנת 2026 היא"), 400.0, 12.0, 16.0);
        assert_eq!(visual_chars(&lines[0]), "איה 2026 תנש");
        let lines = engine().typeset_paragraph(&para("ב-2026"), 400.0, 12.0, 16.0);
        assert_eq!(visual_chars(&lines[0]), "2026-ב");
        // LTR paragraph with a Hebrew word.
        let lines = engine().typeset_paragraph(&para("see שלום now"), 400.0, 12.0, 16.0);
        assert!(!lines[0].is_rtl);
        assert_eq!(visual_chars(&lines[0]), "see םולש now");
    }

    #[test]
    fn rtl_short_lines_are_right_aligned_and_lines_never_start_with_space() {
        let text = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן, עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה, דִּבְרֵי רַבִּי אֱלִיעֶזֶר.";
        let lines = engine().typeset_paragraph(&para(text), 200.0, 11.0, 14.5);
        assert!(lines.len() > 2);
        for line in &lines {
            let first = line.glyphs.first().unwrap();
            let last = line.glyphs.last().unwrap();
            assert_ne!(first.character, Some(' '), "line {:?}", line.text);
            assert_ne!(last.character, Some(' '), "line {:?}", line.text);
            let (_, right) = line.horizontal_extent();
            assert!((right - 200.0).abs() < 0.5, "right edge {right}");
        }
        for line in &lines[..lines.len() - 1] {
            let (left, _) = line.horizontal_extent();
            assert!(left.abs() < 0.5, "justified line starts at {left}");
            assert!((line.width - 200.0).abs() < 0.5);
        }
        let last = lines.last().unwrap();
        assert!(last.horizontal_extent().0 > 1.0, "last line must be ragged");
    }

    /// Regression: glyph ids came from whichever font shaped the word (e.g.
    /// David Libre for a word with '?'), with nothing recording that font.
    #[test]
    fn every_glyph_records_the_font_it_belongs_to() {
        let engine = engine();
        let text = "שלום עולם? Typeset 2026";
        for family in [
            "Noto Serif Hebrew",
            "David CLM",
            "Frank Ruhl Libre",
            "Rashi",
        ] {
            let lines = engine.typeset_paragraph_with_font(&para(text), family, 400.0, 12.0, 16.0);
            for line in &lines {
                for g in &line.glyphs {
                    let name = line.font_of(g).expect("glyph without font");
                    let font = engine.font_manager.get_font(name).unwrap();
                    let face = ttf_parser::Face::parse(&font.raw_bytes, 0).unwrap();
                    let ch = g.character.unwrap();
                    if ch.is_alphanumeric() || ch == '?' || ch == ' ' {
                        assert_eq!(
                            face.glyph_index(ch).map(|id| u32::from(id.0)),
                            Some(g.glyph_id),
                            "{ch:?} in {name}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn clusters_are_paragraph_byte_offsets() {
        let text = "אָב גד  abc";
        let lines = engine().typeset_paragraph(&para(text), 400.0, 12.0, 16.0);
        let p = para(text);
        for g in &lines[0].glyphs {
            // The cluster starts at the glyph's grapheme; marks share it.
            let grapheme: String = p.text[g.cluster as usize..].chars().take(2).collect();
            assert!(
                grapheme.contains(g.character.unwrap()),
                "{:?} not at cluster {}",
                g.character,
                g.cluster
            );
        }
    }

    #[test]
    fn whitespace_runs_collapse_and_nbsp_binds() {
        let lines = engine().typeset_paragraph(&para("  אב \t\n גד  "), 300.0, 12.0, 16.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "אב גד");
        let lines = engine().typeset_paragraph(&para("אב\u{00A0}גד"), 300.0, 12.0, 16.0);
        assert_eq!(lines[0].text, "אב\u{00A0}גד");
    }

    #[test]
    fn degenerate_inputs_do_not_panic() {
        let e = engine();
        assert!(e.typeset_paragraph(&para(""), 300.0, 12.0, 16.0).is_empty());
        assert!(e
            .typeset_paragraph(&para("   "), 300.0, 12.0, 16.0)
            .is_empty());
        assert!(e.typeset_paragraph(&para("אב"), 0.0, 12.0, 16.0).is_empty());
        assert!(e
            .typeset_paragraph(&para("אב"), f32::NAN, 12.0, 16.0)
            .is_empty());
        let lines = e.typeset_paragraph(&para("אב גד"), 300.0, f32::NAN, -1.0);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].height > 0.0);
        // A single word wider than the column still gets laid out.
        let lines = e.typeset_paragraph(&para(&"א".repeat(200)), 50.0, 12.0, 16.0);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].width > 50.0);
    }

    #[test]
    fn visual_order_follows_uax9_l2() {
        let order = visual_order(&[(1, 0), (1, 1), (2, 2), (2, 3), (1, 4)]);
        assert_eq!(order, vec![4, 2, 3, 1, 0]);
        let order = visual_order(&[(0, 0), (1, 1), (1, 2), (0, 3)]);
        assert_eq!(order, vec![0, 2, 1, 3]);
        assert!(visual_order(&[]).is_empty());
    }

    #[test]
    fn test_paragraph_space_before_after_margin_collapsing() {
        use tok_core::styles::ParagraphStyle;

        let mut doc = DocumentRoot::new("test");
        let s1 = ParagraphStyle {
            id: "s1".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 0.0,
            space_after_pt: 10.0,
            ..ParagraphStyle::default()
        };
        let s2 = ParagraphStyle {
            id: "s2".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 6.0,
            space_after_pt: 4.0,
            ..ParagraphStyle::default()
        };
        let s3 = ParagraphStyle {
            id: "s3".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 15.0,
            space_after_pt: 0.0,
            ..ParagraphStyle::default()
        };
        doc.paragraph_styles = vec![s1, s2, s3];

        let flow = doc.sections[0].main_flow_mut().unwrap();
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p1"),
            "s1",
            "פסקה אחת",
        ));
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p2"),
            "s2",
            "פסקה שתיים",
        ));
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p3"),
            "s3",
            "פסקה שלוש",
        ));

        let pages = engine().typeset_document(&doc);
        assert_eq!(pages.len(), 1);
        let lines = &pages[0].frames[0].lines;
        assert_eq!(lines.len(), 3);

        // Top of page: space_before collapses to 0.0
        assert_eq!(lines[0].baseline_y, 14.0);

        // Collapsed margin between s1.after (10.0) and s2.before (6.0) is 10.0.max(6.0) = 10.0
        // baseline = 14.0 + 10.0 + 14.0 = 38.0
        assert_eq!(lines[1].baseline_y, 38.0);

        // Collapsed margin between s2.after (4.0) and s3.before (15.0) is 4.0.max(15.0) = 15.0
        // baseline = 38.0 + 15.0 + 14.0 = 67.0
        assert_eq!(lines[2].baseline_y, 67.0);
    }

    #[test]
    fn bold_styles_are_measured_with_the_bold_face() {
        use tok_core::styles::ParagraphStyle;

        let text = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע";
        let mut doc = DocumentRoot::new("bold");
        doc.paragraph_styles = [
            ("regular", "Frank Ruhl Libre", 400),
            ("bold", "Frank Ruhl Libre", 700),
            ("rashi-bold", "Rashi", 700),
            ("david-bold", "David CLM", 700),
        ]
        .into_iter()
        .map(|(id, family, font_weight)| ParagraphStyle {
            id: id.to_string(),
            font_family: family.to_string(),
            font_weight,
            ..ParagraphStyle::default()
        })
        .collect();
        let flow = doc.sections[0].main_flow_mut().unwrap();
        for (i, id) in ["regular", "bold", "rashi-bold", "david-bold"]
            .into_iter()
            .enumerate()
        {
            flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("p{i}")),
                id,
                text,
            ));
        }

        let pages = engine().typeset_document(&doc);
        let lines = &pages[0].frames[0].lines;
        let fonts: Vec<&str> = lines.iter().map(|l| l.fonts[0].as_str()).collect();
        assert_eq!(
            fonts,
            [
                "Frank Ruhl Libre",
                "Frank Ruhl Libre Bold",
                "Noto Rashi Hebrew Bold",
                // No bold David Libre is shipped: the regular face is used.
                "David Libre",
            ]
        );
        let ink = |l: &LineBox| l.glyphs.iter().map(|g| g.width).sum::<f32>();
        assert!(ink(&lines[1]) > ink(&lines[0]), "bold glyphs are wider");
    }

    #[test]
    fn styles_with_font_weight_700_are_measured_with_bold_face() {
        use tok_core::styles::ParagraphStyle;

        let text = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע";
        let mut doc = DocumentRoot::new("weight-700");
        doc.paragraph_styles = vec![
            ParagraphStyle {
                id: "regular".to_string(),
                font_family: "Frank Ruhl Libre".to_string(),
                font_weight: 400,
                ..ParagraphStyle::default()
            },
            ParagraphStyle {
                id: "weight-700".to_string(),
                font_family: "Frank Ruhl Libre".to_string(),
                font_weight: 700,
                ..ParagraphStyle::default()
            },
        ];
        let flow = doc.sections[0].main_flow_mut().unwrap();
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p0".to_string()),
            "regular",
            text,
        ));
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p1".to_string()),
            "weight-700",
            text,
        ));

        let pages = engine().typeset_document(&doc);
        let lines = &pages[0].frames[0].lines;
        assert_eq!(lines[0].fonts[0], "Frank Ruhl Libre");
        assert_eq!(lines[1].fonts[0], "Frank Ruhl Libre Bold");
        let ink0: f32 = lines[0].glyphs.iter().map(|g| g.width).sum();
        let ink1: f32 = lines[1].glyphs.iter().map(|g| g.width).sum();
        assert!(ink1 > ink0, "font_weight: 700 produces wider bold glyphs");
    }

    #[test]
    fn test_margin_collapsing_at_page_boundary() {
        use tok_core::styles::ParagraphStyle;

        let mut doc = DocumentRoot::new("boundary");
        let style = ParagraphStyle {
            id: "styled".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 30.0,
            space_after_pt: 30.0,
            ..ParagraphStyle::default()
        };
        doc.paragraph_styles = vec![style];

        let flow = doc.sections[0].main_flow_mut().unwrap();
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p1"),
            "styled",
            "פסקה ראשונה בעמוד",
        ));

        let pages = engine().typeset_document(&doc);
        assert_eq!(pages.len(), 1);
        let lines = &pages[0].frames[0].lines;
        // Top of page: space_before must collapse to 0.0
        assert_eq!(lines[0].baseline_y, 14.0);
    }

    #[test]
    fn test_empty_paragraph_collapses_margins() {
        use tok_core::styles::ParagraphStyle;

        let mut doc = DocumentRoot::new("empty_test");
        let s1 = ParagraphStyle {
            id: "s1".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 0.0,
            space_after_pt: 5.0,
            ..ParagraphStyle::default()
        };
        let s_empty = ParagraphStyle {
            id: "s_empty".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 12.0,
            space_after_pt: 8.0,
            ..ParagraphStyle::default()
        };
        let s2 = ParagraphStyle {
            id: "s2".to_string(),
            line_height_pt: 14.0,
            space_before_pt: 6.0,
            space_after_pt: 0.0,
            ..ParagraphStyle::default()
        };
        doc.paragraph_styles = vec![s1, s_empty, s2];

        let flow = doc.sections[0].main_flow_mut().unwrap();
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p01"),
            "s1",
            "פסקה ראשונה",
        ));
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p02"),
            "s_empty",
            "",
        ));
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p03"),
            "s2",
            "פסקה שנייה",
        ));

        let pages = engine().typeset_document(&doc);
        let lines = &pages[0].frames[0].lines;
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].baseline_y, 14.0);
        // Margins collapsed: 5.0.max(12.0).max(8.0).max(6.0) = 12.0
        // baseline = 14.0 + 12.0 + 14.0 = 40.0
        assert_eq!(lines[1].baseline_y, 40.0);
    }

    #[test]
    fn test_typeset_document_multi_flow_talmud() {
        use tok_core::model::{Flow, FlowId, FlowType};

        let mut doc = DocumentRoot::new("תלמוד מסכת ברכות");
        let sec = &mut doc.sections[0];
        let mut rashi_flow = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
        rashi_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("r01"),
            "rashi",
            "רש\"י: מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין לאכול בתרומתן",
        ));
        let mut tosafot_flow = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
        tosafot_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("t01"),
            "tosafot",
            "תוספות: מאימתי קורין פירש הקונטרס דאקרא קאי ותימה",
        ));

        let main_flow = sec.main_flow_mut().unwrap();
        main_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("m01"),
            "main",
            "מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול בתרומתן עד סוף האשמורה הראשונה דברי רבי אליעזר",
        ));

        sec.flows.push(rashi_flow);
        sec.flows.push(tosafot_flow);

        let pages = engine().typeset_document(&doc);
        assert!(!pages.is_empty());
        let p0 = &pages[0];
        assert_eq!(p0.frames.len(), 3);
        let flow_ids: Vec<&str> = p0.frames.iter().map(|f| f.flow_id.as_str()).collect();
        assert!(flow_ids.contains(&"main"));
        assert!(flow_ids.contains(&"rashi"));
        assert!(flow_ids.contains(&"tosafot"));
    }

    #[test]
    fn test_typeset_document_multi_flow_commentary_absorption() {
        use tok_core::model::{Flow, FlowId, FlowType};

        let mut doc = DocumentRoot::new("תלמוד עם מסגרת בליעה");
        let sec = &mut doc.sections[0];

        // 1. Rashi: short flow that finishes early on the page
        let mut rashi_flow = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
        rashi_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("r01"),
            "rashi",
            "רש\"י: קצר בלבד כאן ומסתיים מיד בתחילת העמוד.",
        ));

        // 2. Main text: substantial text filling the column
        let main_flow = sec.main_flow_mut().unwrap();
        for i in 0..45 {
            main_flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("m{:02}", i)),
                "main",
                "מאימתי קורין את שמע בערבין משעה שהכהנים נכנסין לאכול בתרומתן עד סוף האשמורה הראשונה דברי רבי אליעזר וחכמים אומרים עד חצות.",
            ));
        }

        // 3. Tosafot: extensive text needing lots of space to absorb below Rashi
        let mut tosafot_flow = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
        for i in 0..20 {
            tosafot_flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("t{:02}", i)),
                "tosafot",
                "תוספות: פירש הקונטרס דאקרא קאי ותימה לר\"י דהא אמרינן לקמן מאימתי קורין את שמע בשחרית משיראה בין תכלת ללבן.",
            ));
        }

        sec.flows.push(rashi_flow);
        sec.flows.push(tosafot_flow);

        let pages = engine().typeset_document(&doc);
        assert!(!pages.is_empty());
        let p0 = &pages[0];

        let flow_ids: Vec<&str> = p0.frames.iter().map(|f| f.flow_id.as_str()).collect();
        assert!(flow_ids.contains(&"main"), "Page must contain main flow");
        assert!(flow_ids.contains(&"rashi"), "Page must contain rashi flow");
        assert!(
            flow_ids.contains(&"tosafot"),
            "Page must contain tosafot flow"
        );
        assert!(
            flow_ids.contains(&"tosafot_expansion"),
            "Tosafot must expand into absorption frame beneath terminating Rashi"
        );

        let rashi_frame = p0.frames.iter().find(|f| f.flow_id == "rashi").unwrap();
        let exp_frame = p0
            .frames
            .iter()
            .find(|f| f.flow_id == "tosafot_expansion")
            .unwrap();

        // The expansion frame must be located below the terminating Rashi frame
        assert!(
            exp_frame.rect.y >= rashi_frame.rect.y + rashi_frame.rect.height,
            "Expansion frame y ({}) must be below Rashi bottom ({})",
            exp_frame.rect.y,
            rashi_frame.rect.y + rashi_frame.rect.height
        );
        // The expansion frame must have placed lines
        assert!(
            !exp_frame.lines.is_empty(),
            "Expansion frame must contain placed lines"
        );
    }

    #[test]
    fn test_typeset_empty_document_produces_single_empty_page() {
        let doc = DocumentRoot::new("ריק");
        let pages = engine().typeset_document(&doc);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_index, 0);
        assert_eq!(pages[0].frames.len(), 1);
        assert!(pages[0].frames[0].lines.is_empty());
    }

    #[test]
    fn test_engine_typeset_co_paginated_document() {
        use crate::co_pagination::{
            AnchorKey, CoPaginatedChunk, CoPaginatedCommentary, CoPaginationDocument,
            SynchronizerConfig, TemplateConfig,
        };

        let eng = engine();
        let tmpl = TemplateConfig::default();
        let sync_cfg = SynchronizerConfig::default();

        let doc = CoPaginationDocument {
            main_chunks: vec![CoPaginatedChunk {
                text: "מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין".to_string(),
                anchors: vec![(AnchorKey("m1".to_string()), "קורין".to_string())],
            }],
            rashi_items: vec![CoPaginatedCommentary {
                target_anchor: AnchorKey("m1".to_string()),
                text: "קורין את שמע: פירש רש\"י".to_string(),
            }],
            tosafot_items: vec![],
            additional_streams: std::collections::HashMap::new(),
        };

        let result = eng.typeset_co_paginated_document(&tmpl, &sync_cfg, &doc);
        assert!(!result.pages.is_empty());
        assert!(result.total_gemara_lines > 0);
        assert!(result.total_rashi_lines > 0);
    }

    #[test]
    fn test_engine_paginate_co_paginated_document_async() {
        use crate::co_pagination::{
            AnchorKey, CoPaginatedChunk, CoPaginatedCommentary, CoPaginationDocument,
            PaginationEvent, SynchronizerConfig, TemplateConfig,
        };
        use std::sync::Arc;

        let eng = Arc::new(engine());
        let tmpl = TemplateConfig::default();
        let sync_cfg = SynchronizerConfig::default();

        let doc = CoPaginationDocument {
            main_chunks: vec![CoPaginatedChunk {
                text: "תנו רבנן מעשה ברבי אליעזר ורבי יהושע שהיו מסובין".to_string(),
                anchors: vec![(AnchorKey("a1".to_string()), "מסובין".to_string())],
            }],
            rashi_items: vec![CoPaginatedCommentary {
                target_anchor: AnchorKey("a1".to_string()),
                text: "מסובין: בהסבה של מצוה".to_string(),
            }],
            tosafot_items: vec![],
            additional_streams: std::collections::HashMap::new(),
        };

        let (rx, _token) = eng.paginate_co_paginated_document_async(tmpl, sync_cfg, doc);
        let mut got_page = false;
        let mut got_finished = false;

        while let Ok(event) = rx.recv() {
            match event {
                PaginationEvent::PageReady(_) => got_page = true,
                PaginationEvent::Finished(_) => got_finished = true,
                _ => {}
            }
        }

        assert!(
            got_page,
            "Worker should stream at least one PageReady event"
        );
        assert!(got_finished, "Worker should emit Finished event");
    }

    #[test]
    fn test_multi_flow_multi_page_cascading_and_completeness() {
        use tok_core::model::{Flow, FlowId};

        let mut doc = DocumentRoot::new("עימוד רב-עמודי");
        let sec = &mut doc.sections[0];
        sec.flows.clear();

        // Short primary text (fits on page 1)
        let mut main_flow = Flow::new(FlowId::new("main"), FlowType::Main);
        main_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("m01"),
            "style-main",
            "תלמוד בבלי מסכת ברכות דף ב עמוד א. מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין לאכול בתרומתן.",
        ));
        sec.flows.push(main_flow);

        // Long commentary text (overflows page 1 onto page 2 and page 3)
        let mut comm_flow = Flow::new(FlowId::new("commentary"), FlowType::CommentA);
        for i in 1..=45 {
            comm_flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("c{:02}", i)),
                "style-comm",
                &format!("פירוש והרחבה לקטע מספר {}: תנא אקרא קאי דכתיב בשכבך ובקומך והכי קתני זמן קריאת שמע של שכיבה אימת משעה שהכהנים נכנסין לאכול בתרומתן ואי בעית אימא יליף מברייתו של עולם דכתיב ויהי ערב ויהי בוקר יום אחד.", i),
            ));
        }
        sec.flows.push(comm_flow);

        let pages = engine().typeset_document(&doc);
        assert!(
            pages.len() >= 2,
            "Long commentary should cascade to at least 2 pages, got {}",
            pages.len()
        );

        // Verify page numbering & gematria
        assert_eq!(pages[0].page_number_gematria, "א׳");
        assert_eq!(pages[1].page_number_gematria, "ב׳");

        // Verify content completeness: all commentary paragraphs must appear in output lines
        let all_lines_text: Vec<String> = pages
            .iter()
            .flat_map(|p| p.frames.iter())
            .filter(|f| f.flow_id.contains("commentary"))
            .flat_map(|f| f.lines.iter().map(|l| l.text.clone()))
            .collect();

        assert!(
            !all_lines_text.is_empty(),
            "Commentary lines must be placed"
        );
        // Verify that paragraph 1 and paragraph 45 both appear in the text
        let combined_text = all_lines_text.join(" ");
        assert!(
            combined_text.contains("קטע מספר 1:"),
            "First commentary paragraph must appear"
        );
        assert!(
            combined_text.contains("קטע מספר 45:"),
            "Last commentary paragraph must appear"
        );

        // Verify break token on first page
        assert!(
            pages[0].break_token.is_some(),
            "First page should have a break token pointing to continuation"
        );
    }

    #[test]
    fn test_multi_flow_asymmetrical_three_streams_continuity() {
        use tok_core::model::{Flow, FlowId};

        let mut doc = DocumentRoot::new("שלוש זרימות אסימטריות");
        let sec = &mut doc.sections[0];
        sec.flows.clear();

        // Primary: 2 paragraphs
        let mut main_flow = Flow::new(FlowId::new("main"), FlowType::Main);
        main_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("m1"),
            "main",
            "פסקה ראשית ראשונה.",
        ));
        main_flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("m2"),
            "main",
            "פסקה ראשית שנייה.",
        ));
        sec.flows.push(main_flow);

        // Commentary 1 (Rashi): 8 paragraphs
        let mut rashi_flow = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
        for i in 1..=8 {
            rashi_flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("r{:02}", i)),
                "rashi",
                &format!(
                    "דיבור המתחיל רש\"י פסקה מספר {}: פירוש דברי הגמרא באריכות ובתוספת ביאור מקיף.",
                    i
                ),
            ));
        }
        sec.flows.push(rashi_flow);

        // Commentary 2 (Tosafot): 4 paragraphs
        let mut tosafot_flow = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
        for i in 1..=4 {
            tosafot_flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("t{:02}", i)),
                "tosafot",
                &format!(
                    "תוספות דיבור המתחיל {}: קושיא ופירוקא על דברי רש\"י והגמרא.",
                    i
                ),
            ));
        }
        sec.flows.push(tosafot_flow);

        let pages = engine().typeset_document(&doc);
        assert!(!pages.is_empty());

        // Check zero dropped paragraphs across all frames
        let rashi_lines: Vec<String> = pages
            .iter()
            .flat_map(|p| p.frames.iter())
            .filter(|f| f.flow_id.contains("rashi"))
            .flat_map(|f| f.lines.iter().map(|l| l.text.clone()))
            .collect();

        let rashi_combined = rashi_lines.join(" ");
        for i in 1..=8 {
            assert!(
                rashi_combined.contains(&format!("פסקה מספר {}:", i)),
                "Rashi paragraph {} must not be dropped",
                i
            );
        }
    }

    #[test]
    fn test_typeset_document_auto_classified_tzurat_hadaf_unfamiliar_names() {
        use tok_core::model::{Flow, FlowId};

        let mut doc = DocumentRoot::new("תבנית צורת הדף עם שמות מותאמים");
        let sec = &mut doc.sections[0];
        sec.flows.clear();

        // 3 flows with inner_spine and outer_margin roles
        let mut central = Flow::new(FlowId::new("text_core"), FlowType::Main);
        central.placement_role = Some("primary".to_string());
        central.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p1"),
            "style-main",
            "טקסט מרכזי עיקרי לבדיקת פריסת צורת הדף.",
        ));
        sec.flows.push(central);

        let mut inner = Flow::new(FlowId::new("annotation_a"), FlowType::CommentA);
        inner.placement_role = Some("inner_spine".to_string());
        inner.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p2"),
            "style-inner",
            "הערה פנימית לצד השדרה בטקסט מסורתי.",
        ));
        sec.flows.push(inner);

        let mut outer = Flow::new(FlowId::new("annotation_b"), FlowType::CommentB);
        outer.placement_role = Some("outer_margin".to_string());
        outer.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p3"),
            "style-outer",
            "הערה חיצונית בשוליים החיצוניים של הדף.",
        ));
        sec.flows.push(outer);

        let pages = engine().typeset_document(&doc);
        assert!(!pages.is_empty());
        let p0 = &pages[0];
        assert_eq!(p0.frames.len(), 3);

        let frame_ids: Vec<&str> = p0.frames.iter().map(|f| f.flow_id.as_str()).collect();
        assert!(frame_ids.contains(&"text_core"));
        assert!(frame_ids.contains(&"annotation_a"));
        assert!(frame_ids.contains(&"annotation_b"));

        // Frame widths and heights must be positive
        for frame in &p0.frames {
            assert!(frame.rect.width > 50.0);
            assert!(frame.rect.height > 10.0);
        }
    }

    #[test]
    fn test_typeset_document_auto_classified_parallel_columns() {
        use tok_core::model::{Flow, FlowId};

        let mut doc = DocumentRoot::new("עמודות מקבילות");
        let sec = &mut doc.sections[0];
        sec.flows.clear();

        let mut col1 = Flow::new(FlowId::new("hebrew_source"), FlowType::Main);
        col1.placement_role = Some("column_0".to_string());
        col1.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("c1"),
            "style-col",
            "עמודה ראשונה מקור עברי.",
        ));
        sec.flows.push(col1);

        let mut col2 = Flow::new(FlowId::new("aramaic_targum"), FlowType::Main);
        col2.placement_role = Some("column_1".to_string());
        col2.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("c2"),
            "style-col",
            "עמודה שנייה תרגום ארמי מקביל.",
        ));
        sec.flows.push(col2);

        let pages = engine().typeset_document(&doc);
        assert!(!pages.is_empty());
        let p0 = &pages[0];
        assert_eq!(p0.frames.len(), 2);

        let f0 = p0
            .frames
            .iter()
            .find(|f| f.flow_id == "hebrew_source")
            .unwrap();
        let f1 = p0
            .frames
            .iter()
            .find(|f| f.flow_id == "aramaic_targum")
            .unwrap();
        assert!(
            (f0.rect.width - f1.rect.width).abs() < 5.0,
            "Parallel columns should have equal widths by default"
        );
    }

    #[test]
    fn test_typeset_document_explicit_template_override() {
        use tok_core::model::{Flow, FlowId};

        let mut doc = DocumentRoot::new("עקיפת תבנית מפורשת");
        let sec = &mut doc.sections[0];
        sec.flows.clear();
        sec.layout_kind = Some("parallel_columns".to_string());
        sec.column_proportions = Some(vec![0.30, 0.70]);

        let mut stream_a = Flow::new(FlowId::new("stream_a"), FlowType::Main);
        stream_a.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("a1"),
            "style-a",
            "זרימה א צרה יחסית.",
        ));
        sec.flows.push(stream_a);

        let mut stream_b = Flow::new(FlowId::new("stream_b"), FlowType::Main);
        stream_b.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("b1"),
            "style-b",
            "זרימה ב רחבה יחסית התופסת את רוב רוחב העמוד.",
        ));
        sec.flows.push(stream_b);

        let pages = engine().typeset_document(&doc);
        assert!(!pages.is_empty());
        let p0 = &pages[0];
        assert_eq!(p0.frames.len(), 2);

        let fa = p0.frames.iter().find(|f| f.flow_id == "stream_a").unwrap();
        let fb = p0.frames.iter().find(|f| f.flow_id == "stream_b").unwrap();

        // Check ratio matches ~ 0.30 / 0.70
        let total_w = fa.rect.width + fb.rect.width;
        let ratio_a = fa.rect.width / total_w;
        let ratio_b = fb.rect.width / total_w;
        assert!(
            (ratio_a - 0.30).abs() < 0.05,
            "Ratio A should be approx 0.30, got {}",
            ratio_a
        );
        assert!(
            (ratio_b - 0.70).abs() < 0.05,
            "Ratio B should be approx 0.70, got {}",
            ratio_b
        );
    }

    #[test]
    fn test_typeset_document_prevents_stranded_headings() {
        use tok_core::styles::ParagraphStyle;

        let mut config = TypesettingEngineConfig::default();
        config.page_width_pt = 300.0;
        config.page_height_pt = 150.0;
        config.margin_top_pt = 20.0;
        config.margin_bottom_pt = 20.0;
        config.margin_inner_pt = 20.0;
        config.margin_outer_pt = 20.0;
        let eng = TypesettingEngine::new(config);

        let mut doc = DocumentRoot::new("בדיקת מניעת כותרת יתומה");
        let mut s_body = ParagraphStyle::default();
        s_body.id = "body".to_string();
        s_body.font_size_pt = 12.0;
        s_body.line_height_pt = 18.0;
        s_body.space_before_pt = 0.0;
        s_body.space_after_pt = 0.0;

        let mut s_heading = ParagraphStyle::default();
        s_heading.id = "heading".to_string();
        s_heading.font_size_pt = 14.0;
        s_heading.line_height_pt = 22.0;
        s_heading.space_before_pt = 10.0;
        s_heading.space_after_pt = 6.0;
        s_heading.keep_with_next = true;

        doc.paragraph_styles = vec![s_body, s_heading];

        let flow = doc.sections[0].main_flow_mut().unwrap();
        // 4 single-line body paragraphs (each 18pt = 72pt)
        for i in 0..4 {
            flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("a_{}", i)),
                "body",
                "פסקה קצרה",
            ));
        }
        // Heading with keep_with_next = true (22pt + 10pt space_before = 32pt -> total 104pt <= 110pt)
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("b_0"),
            "heading",
            "פרק ראשון: דיני פריסה גיאומטרית",
        ));
        // Next paragraph (18pt -> 104 + 18 = 122pt > 110pt)
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("c_0"),
            "body",
            "המשך גוף הטקסט שלא נכנס בעמוד הראשון",
        ));

        let pages = eng.typeset_document(&doc);
        assert!(
            pages.len() >= 2,
            "Document should split across at least 2 pages"
        );

        // The heading MUST NOT be on page 1!
        let page1_lines = &pages[0].frames[0].lines;
        let page2_lines = &pages[1].frames[0].lines;
        assert!(
            !page1_lines.iter().any(|l| l.text.contains("פרק ראשון")),
            "Heading must not be stranded on Page 1!"
        );
        assert!(
            page2_lines[0].text.contains("פרק ראשון"),
            "Heading should be pushed to Page 2 at top of page, got: {}",
            page2_lines[0].text
        );
    }

    #[test]
    fn test_typeset_document_orphan_and_widow_control() {
        use tok_core::styles::ParagraphStyle;

        let mut config = TypesettingEngineConfig::default();
        config.page_width_pt = 300.0;
        config.page_height_pt = 160.0;
        config.margin_top_pt = 20.0;
        config.margin_bottom_pt = 20.0;
        config.margin_inner_pt = 20.0;
        config.margin_outer_pt = 20.0;
        let eng = TypesettingEngine::new(config);

        let mut doc = DocumentRoot::new("בדיקת מניעת שורות יתומות");
        let mut s_body = ParagraphStyle::default();
        s_body.id = "body".to_string();
        s_body.font_size_pt = 12.0;
        s_body.line_height_pt = 20.0;
        s_body.space_before_pt = 0.0;
        s_body.space_after_pt = 0.0;
        s_body.orphan_control = 2;
        s_body.widow_control = 2;

        doc.paragraph_styles = vec![s_body];

        let flow = doc.sections[0].main_flow_mut().unwrap();
        // 5 single-line paragraphs fill 5 * 20 = 100pt (content height = 120pt, remaining = 20pt)
        for i in 0..5 {
            flow.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("p1_{}", i)),
                "body",
                "פסקה קצרה למילוי העמוד",
            ));
        }
        // Paragraph 2 is long (3 lines: 3 * 20 = 60pt). Line 0 (20pt) fits in remaining 20pt.
        // A naive greedy breaker would place Line 0 on Page 1 (leaving a 1-line orphan!).
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("p2"),
            "body",
            "פסקה שנייה ארוכה מאוד שמתפרסת על פני מספר שורות שלמות כדי לבדוק האם מנוע העימוד מונע היווצרות שורות יתומות בתחתית העמוד הראשון כפי שנדרש.",
        ));

        let p2_id = flow.paragraphs.last().unwrap().id;

        let pages = eng.typeset_document(&doc);
        assert!(pages.len() >= 2);

        // Check each page line counts for paragraph 2
        for (p_idx, page) in pages.iter().enumerate() {
            let lines = &page.frames[0].lines;
            let p2_lines_on_page: Vec<_> = lines
                .iter()
                .filter(|l| l.paragraph_id == Some(p2_id))
                .collect();
            if !p2_lines_on_page.is_empty() {
                assert!(
                    p2_lines_on_page.len() >= 2 || p_idx == pages.len() - 1,
                    "Page {} has only {} lines of paragraph 2 (violating orphan_control)",
                    p_idx + 1,
                    p2_lines_on_page.len()
                );
            }
        }
    }

    #[test]
    fn test_multi_flow_l_shape_expansion_multi_paragraph_completeness() {
        use tok_core::model::{Flow, FlowId};

        let mut config = TypesettingEngineConfig::default();
        config.page_width_pt = 500.0;
        config.page_height_pt = 300.0;
        config.margin_top_pt = 20.0;
        config.margin_bottom_pt = 20.0;
        config.margin_inner_pt = 30.0;
        config.margin_outer_pt = 20.0;
        let eng = TypesettingEngine::new(config);

        let mut doc = DocumentRoot::new("בדיקת שימור פסקאות בהרחבת צורת הדף");
        let sec = &mut doc.sections[0];
        sec.flows.clear();

        // 1. Central Gemara stream: finishes early (e.g. 50pt height out of 260pt)
        let mut central = Flow::new(FlowId::new("gemara"), FlowType::Main);
        central.placement_role = Some("primary".to_string());
        central.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("g1"),
            "default-body",
            "גמרא קצרה שמסתיימת מוקדם כדי להפעיל הרחבת צורת הדף.",
        ));
        sec.flows.push(central);

        // 2. Commentary stream (e.g. Rashi) with MULTIPLE paragraphs
        let mut rashi = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
        rashi.placement_role = Some("inner_spine".to_string());
        for i in 1..=4 {
            rashi.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("r{}", i)),
                "default-body",
                &format!("רש\"י פסקה מספר {}: פירוש דברי הגמרא בהרחבה רבה, כולל הסברים וביאורים ארוכים מאוד שמתפרסים על פני שורות רבות.", i),
            ));
        }
        sec.flows.push(rashi);

        // 3. Outer commentary (Tosafot)
        let mut tosafot = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
        tosafot.placement_role = Some("outer_margin".to_string());
        tosafot.add_paragraph(ParagraphNode::new(
            FractionalIndex::new("t1"),
            "default-body",
            "תוספות קצר בעמודה החיצונית.",
        ));
        sec.flows.push(tosafot);

        let pages = eng.typeset_document(&doc);
        assert!(!pages.is_empty());

        // Collect all lines for Rashi across all frames (both narrow and expansion frames)
        let rashi_texts: Vec<String> = pages
            .iter()
            .flat_map(|p| p.frames.iter())
            .filter(|f| f.flow_id.contains("rashi"))
            .flat_map(|f| f.lines.iter().map(|l| l.text.clone()))
            .collect();

        let combined = rashi_texts.join(" ");
        for i in 1..=4 {
            assert!(
                combined.contains(&format!("פסקה מספר {}:", i)),
                "Rashi paragraph {} must be completely preserved across frames, but was missing!",
                i
            );
        }
    }

    #[test]
    fn test_multi_flow_determinism_across_runs() {
        use tok_core::model::{Flow, FlowId};

        let mut doc = DocumentRoot::new("בדיקת דטרמיניזם רב-זרימתי");
        let sec = &mut doc.sections[0];
        sec.flows.clear();

        let mut central = Flow::new(FlowId::new("gemara"), FlowType::Main);
        central.placement_role = Some("primary".to_string());
        for i in 1..=5 {
            central.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("g{}", i)),
                "default-body",
                &format!("גמרא קטע מספר {}: מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול בתרומתן.", i),
            ));
        }
        sec.flows.push(central);

        let mut rashi = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
        rashi.placement_role = Some("inner_spine".to_string());
        for i in 1..=8 {
            rashi.add_paragraph(ParagraphNode::new(
                FractionalIndex::new(format!("r{}", i)),
                "default-body",
                &format!(
                    "רש\"י קטע {}: פירוש רש\"י הקדוש על דברי המשנה והגמרא במסכת ברכות.",
                    i
                ),
            ));
        }
        sec.flows.push(rashi);

        let eng = engine();
        let run1 = eng.typeset_document(&doc);
        let run2 = eng.typeset_document(&doc);

        assert_eq!(run1.len(), run2.len(), "Page count must be identical");
        for (idx, (p1, p2)) in run1.iter().zip(run2.iter()).enumerate() {
            assert_eq!(p1.page_index, p2.page_index);
            assert_eq!(p1.page_number_gematria, p2.page_number_gematria);
            assert_eq!(
                p1.frames.len(),
                p2.frames.len(),
                "Page {} frames count mismatch",
                idx
            );
            for (f1, f2) in p1.frames.iter().zip(p2.frames.iter()) {
                assert_eq!(f1.flow_id, f2.flow_id);
                assert_eq!(f1.rect, f2.rect);
                assert_eq!(f1.lines.len(), f2.lines.len());
                for (l1, l2) in f1.lines.iter().zip(f2.lines.iter()) {
                    assert_eq!(l1.text, l2.text);
                    assert_eq!(l1.width, l2.width);
                    assert_eq!(l1.baseline_y, l2.baseline_y);
                }
            }
        }
    }

    #[test]
    fn test_multi_flow_shaping_cache_performance() {
        let doc = crate::fixtures::create_realistic_talmud_document();
        let eng = engine();
        let start = std::time::Instant::now();
        let pages = eng.typeset_document_multi_flow(&doc);
        let elapsed = start.elapsed();
        assert!(!pages.is_empty(), "Must produce pages");
        assert!(
            elapsed.as_millis() < 4000,
            "Multi-flow typesetting took too long: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_paragraph_cache_integration_in_engine() {
        let mut doc = DocumentRoot::new("מבחן מטמון פסקאות");
        let flow = doc.sections[0].main_flow_mut().unwrap();
        let p1 = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("a"),
            "default-body",
            "פסקה ראשונה לצורך בדיקת מטמון",
        );
        let p2 = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("b"),
            "default-body",
            "פסקה שנייה לצורך בדיקת מטמון וחישוב אינקרמנטלי",
        );
        let p1_id = p1.id;
        flow.add_paragraph(p1);
        flow.add_paragraph(p2);

        let eng = engine();
        assert_eq!(eng.paragraph_cache.read().unwrap().len(), 0);

        // Run 1: Cold cache -> 2 misses
        let pages1 = eng.typeset_document(&doc);
        assert!(!pages1.is_empty());
        assert_eq!(eng.paragraph_cache.read().unwrap().len(), 2);
        let misses1 = eng.paragraph_cache.read().unwrap().metrics().miss_count();
        let hits1 = eng.paragraph_cache.read().unwrap().metrics().hit_count();
        assert_eq!(misses1, 2);
        assert_eq!(hits1, 0);

        // Run 2: Warm cache -> 2 hits, 0 new misses
        let pages2 = eng.typeset_document(&doc);
        assert_eq!(pages1.len(), pages2.len());
        let hits2 = eng.paragraph_cache.read().unwrap().metrics().hit_count();
        let misses2 = eng.paragraph_cache.read().unwrap().metrics().miss_count();
        assert_eq!(hits2, 2);
        assert_eq!(misses2, 2);

        // Invalidate p1
        assert!(eng.invalidate_paragraph(p1_id));
        assert_eq!(eng.paragraph_cache.read().unwrap().len(), 1);

        // Run 3: p1 is recomputed (1 miss), p2 is hit (1 hit)
        let _pages3 = eng.typeset_document(&doc);
        let hits3 = eng.paragraph_cache.read().unwrap().metrics().hit_count();
        let misses3 = eng.paragraph_cache.read().unwrap().metrics().miss_count();
        assert_eq!(hits3, 3);
        assert_eq!(misses3, 3);
    }

    #[test]
    fn test_zero_cascade_incremental_typesetting() {
        let mut doc = DocumentRoot::new("מבחן Zero-Cascade");
        let p1_id = {
            let flow = doc.sections[0].main_flow_mut().unwrap();
            for i in 0..100 {
                let p = ParagraphNode::new(
                    tok_core::id::FractionalIndex::new(format!("k{:03}", i)),
                    "default-body",
                    "זֶהוּ טֶקְסְט עִבְרִי שֶׁל פִּסְקָה תִּקְנִית לְצֹרֶךְ עִמּוּד מְדֻיָּק וּבְדִיקַת הִתְכַּנְּסוּת שֶׁל עִמּוּד אִינְקְרֶמֶנְטָלִי בְּלִי גְּלִישָׁה מְיֻתֶּרֶת.",
                );
                flow.add_paragraph(p);
            }
            flow.paragraphs[1].id
        };

        let eng = engine();
        let pages1 = eng.typeset_document(&doc);
        assert!(pages1.len() >= 2, "Must produce multiple pages");

        // Edit paragraph 1 with minor typo fix that preserves line count
        doc.sections[0].main_flow_mut().unwrap().paragraphs[1].text =
            "זֶהוּ טֶקְסְט עִבְרִי שֶׁל פִּסְקָה תִּקְנִית לְצֹרֶךְ עִמּוּד מְדֻיָּק וּבְדִיקַת הִתְכַּנְּסוּת שֶׁל עִמּוּד אִינְקְרֶמֶנְטָלִי בְּלִי שׁוּם גְּלִישָׁה.".to_string();

        let (pages2, diag) = eng.typeset_document_incremental(&doc, &pages1, Some(p1_id));

        assert_eq!(pages1.len(), pages2.len());
        assert!(diag.zero_cascade, "Must trigger zero-cascade optimization");
        assert_eq!(diag.recomputed_pages, 1);
        assert_eq!(diag.reused_pages, pages1.len() - 1);

        // Verify trailing pages are identical
        for page_idx in 1..pages1.len() {
            assert_eq!(
                pages1[page_idx].frames[0].lines.len(),
                pages2[page_idx].frames[0].lines.len()
            );
            for (l1, l2) in pages1[page_idx].frames[0]
                .lines
                .iter()
                .zip(&pages2[page_idx].frames[0].lines)
            {
                assert_eq!(l1.text, l2.text);
                assert_eq!(l1.baseline_y, l2.baseline_y);
            }
        }
    }

    #[test]
    fn test_incremental_page_number_and_gematria_consistency() {
        let mut doc = DocumentRoot::new("מבחן עקביות מספור עמודים וגמטריא");
        {
            let flow = doc.sections[0].main_flow_mut().unwrap();
            for i in 0..60 {
                let p = ParagraphNode::new(
                    tok_core::id::FractionalIndex::new(format!("k{:03}", i)),
                    "default-body",
                    "טֶקְסְט אָרֹךְ עִם כַּמָּה מִשְׁפָּטִים כְּדֵי לְמַלֵּא עַמּוּדִים בְּדִיּוּק וּלְאַפְשֵׁר בְּדִיקַת מַעֲבָרֵי עַמּוּד וּמִסְפּוּר עִבְרִי תַּקִּין.",
                );
                flow.add_paragraph(p);
            }
        }

        let eng = engine();
        let pages1 = eng.typeset_document(&doc);
        assert!(pages1.len() >= 2);

        // Verify initial page numbers and gematria
        for (idx, page) in pages1.iter().enumerate() {
            assert_eq!(page.page_index, idx);
            assert_eq!(
                page.page_number_gematria,
                GematriaEngine::to_hebrew_numeral(idx + 1)
            );
        }

        // Insert 80 new paragraphs at beginning to cause page expansion
        {
            let flow = doc.sections[0].main_flow_mut().unwrap();
            for i in 0..80 {
                let p = ParagraphNode::new(
                    tok_core::id::FractionalIndex::new(format!("a{:03}", i)),
                    "default-body",
                    "פִּסְקָה חֲדָשָׁה שֶׁנּוֹסְפָה בִּתְחִלַּת הַמִּסְמָךְ וְגּוֹרֶמֶת לִגְלִישַׁת עַמּוּדִים קָדִימָה.",
                );
                flow.add_paragraph(p);
            }
        }

        let (pages2, _diag) = eng.typeset_document_incremental(&doc, &pages1, None);
        assert!(pages2.len() > pages1.len(), "Page count must expand");

        // Verify that after incremental reflow, every page has correct monotonic numbering and gematria
        for (idx, page) in pages2.iter().enumerate() {
            assert_eq!(
                page.page_index, idx,
                "Page index must be strictly monotonic"
            );
            assert_eq!(
                page.page_number_gematria,
                GematriaEngine::to_hebrew_numeral(idx + 1),
                "Gematria folio must match 1-based page index"
            );
        }
    }

    #[test]
    fn test_anchor_and_cross_reference_consistency_across_incremental_reflow() {
        let mut doc = DocumentRoot::new("מבחן עוגנים והפניות צולבות בעימוד אינקרמנטלי");
        doc.dependency_tracker = Some(tok_core::dependency::DocumentDependencyTracker::new());
        let mut registry = tok_core::reference::CrossReferenceRegistry::new();

        let target_node_id;
        let source_node_id;

        {
            let flow = doc.sections[0].main_flow_mut().unwrap();

            let p_source = ParagraphNode::new(
                tok_core::id::FractionalIndex::new("a01"),
                "default-body",
                "פסקת הפניה מקורית המפנה לעוגן המטרה לקמן.",
            );
            source_node_id = p_source.id;
            flow.add_paragraph(p_source);

            let p_target = ParagraphNode::new(
                tok_core::id::FractionalIndex::new("b01"),
                "default-body",
                "פסקת יעד המכילה את עוגן המטרה שאליו מפנים.",
            );
            target_node_id = p_target.id;
            flow.add_paragraph(p_target);
        }

        // Register target anchor and reference
        let target = tok_core::reference::ReferenceTarget::new(
            "anchor-target-topic",
            tok_core::anchor::TextAnchor::start_of(target_node_id),
        )
        .with_label("סוגיא בפרק ראשון");
        registry.register_target(target);

        let xref = tok_core::reference::CrossReferenceNode::new(
            tok_core::anchor::TextAnchor::start_of(source_node_id),
            "anchor-target-topic",
            tok_core::reference::ReferenceKind::PageGematria,
            "??",
        );
        registry.register_reference(xref);
        doc.cross_references = Some(registry);

        let eng = engine();
        let pages1 = eng.typeset_document(&doc);
        eng.update_cross_references_and_dependencies(&mut doc, &pages1);

        // Initially both are on page 1 (Gematria א׳)
        let resolved_init = doc.cross_references.as_ref().unwrap().references[0]
            .resolved_or_fallback()
            .to_string();
        assert_eq!(
            resolved_init,
            GematriaEngine::to_hebrew_numeral(1),
            "Initial target should resolve to page 1 gematria"
        );
        assert_eq!(
            doc.dependency_tracker
                .as_ref()
                .unwrap()
                .page_for_anchor("anchor-target-topic"),
            Some(0)
        );

        // Now push target downstream by inserting many paragraphs between source and target
        {
            let flow = doc.sections[0].main_flow_mut().unwrap();
            for i in 0..70 {
                let p = ParagraphNode::new(
                    tok_core::id::FractionalIndex::new(format!("am{:03}", i)),
                    "default-body",
                    "טקסט מילוי המרחיב את המסמך ודוחף את עוגן היעד לעמודים הבאים.",
                );
                flow.add_paragraph(p);
            }
        }

        let (pages2, _diag) = eng.typeset_document_incremental(&doc, &pages1, None);
        assert!(pages2.len() >= 2);

        let updated_count = eng.update_cross_references_and_dependencies(&mut doc, &pages2);
        assert!(
            updated_count > 0,
            "Cross-reference should update due to page shift"
        );

        let target_page_idx = doc
            .dependency_tracker
            .as_ref()
            .unwrap()
            .page_for_anchor("anchor-target-topic")
            .unwrap();
        assert!(
            target_page_idx >= 1,
            "Target node must have shifted to downstream page"
        );

        let resolved_after = doc.cross_references.as_ref().unwrap().references[0]
            .resolved_or_fallback()
            .to_string();
        let expected_gematria = GematriaEngine::to_hebrew_numeral(target_page_idx + 1);
        assert_eq!(
            resolved_after, expected_gematria,
            "Cross reference must consistently reflect new target page in gematria"
        );
    }

    #[test]
    fn test_multi_flow_incremental_commentary_and_footnote_invalidation() {
        let mut doc = DocumentRoot::new("מבחן עימוד רב-זרימתי אינקרמנטלי");

        let sec = &mut doc.sections[0];
        sec.flows.clear();

        let mut gemara_flow =
            tok_core::Flow::new(tok_core::FlowId::new("gemara"), tok_core::FlowType::Main);
        let p_gemara = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("g01"),
            "default-body",
            "מַתְנִיתִין: מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית. מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.",
        );
        let _gemara_p_id = p_gemara.id;
        gemara_flow.add_paragraph(p_gemara);

        let mut rashi_flow =
            tok_core::Flow::new(tok_core::FlowId::new("rashi"), tok_core::FlowType::CommentA);
        let p_rashi = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("r01"),
            "default-body",
            "רש\"י: מאימתי - משעת יציאת הכוכבים.",
        );
        let _rashi_p_id = p_rashi.id;
        rashi_flow.add_paragraph(p_rashi);

        let mut notes_flow =
            tok_core::Flow::new(tok_core::FlowId::new("notes"), tok_core::FlowType::Footnote)
                .with_placement_role("bottom_band");
        let mut p_note = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("n01"),
            "default-body",
            "הערה א: עיין ברכות דף ב עמוד א.",
        );
        let note_p_id = p_note.id;
        notes_flow.add_paragraph(p_note.clone());

        sec.flows.push(gemara_flow);
        sec.flows.push(rashi_flow);
        sec.flows.push(notes_flow);

        let eng = engine();
        let pages1 = eng.typeset_document_multi_flow(&doc);
        assert!(!pages1.is_empty());

        // Invalidate ONLY the footnote paragraph
        p_note.text = "הערה א: עיין ברכות דף ב עמוד א ובהגהות הב\"ח אות א.".to_string();
        doc.sections[0].flows[2].paragraphs[0] = p_note;

        let (pages2, _diag) =
            eng.typeset_document_multi_flow_incremental(&doc, &pages1, Some(note_p_id));
        assert!(!pages2.is_empty());

        // Footnote frame must contain updated note
        let note_frame = pages2[0]
            .frames
            .iter()
            .find(|f| f.flow_id == "notes")
            .unwrap();
        assert!(note_frame
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(note_p_id)));
    }

    #[test]
    fn test_incremental_typeset_respects_layout_locks_and_page_break_overrides() {
        let mut doc = DocumentRoot::new("מבחן נעילת מעברי עמוד בעימוד אינקרמנטלי");
        let flow = doc.sections[0].main_flow_mut().unwrap();

        // P1: short paragraph (fits on page 1)
        let p1 = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("a01"),
            "default-body",
            "פסקה ראשונה קצרה המתאימה בקלות לחלק העליון של עמוד ראשון.",
        );
        let p1_id = p1.id;
        flow.add_paragraph(p1);

        // P2: forced page break before
        let p2 = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("b01"),
            "default-body",
            "פסקה שניה עם נעילת מעבר עמוד כפוי לפניה - חייבת להתחיל בעמוד חדש.",
        )
        .with_overrides(tok_core::overrides::LayoutLockOverrides::page_break_before());
        let p2_id = p2.id;
        flow.add_paragraph(p2);

        // P3: forced page break before
        let p3 = ParagraphNode::new(
            tok_core::id::FractionalIndex::new("c01"),
            "default-body",
            "פסקה שלישית כפויה להתחיל בעמוד הבא.",
        )
        .with_overrides(tok_core::overrides::LayoutLockOverrides::page_break_before());
        let p3_id = p3.id;
        flow.add_paragraph(p3);

        let eng = engine();
        let pages1 = eng.typeset_document(&doc);

        assert_eq!(
            pages1.len(),
            3,
            "Each paragraph with forced page break before must be on its own page"
        );
        assert!(pages1[0].frames[0]
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(p1_id)));
        assert!(pages1[1].frames[0]
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(p2_id)));
        assert!(pages1[2].frames[0]
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(p3_id)));

        // Incrementally edit P1 by adding more text
        doc.sections[0].flows[0].paragraphs[0].text =
            "פסקה ראשונה שנערכה באופן אינקרמנטלי ונוספו לה מילים נוספות.".to_string();

        let (pages2, _diag) = eng.typeset_document_incremental(&doc, &pages1, Some(p1_id));
        assert_eq!(
            pages2.len(),
            3,
            "Forced page break overrides must be strictly preserved during incremental reflow"
        );
        assert!(pages2[0].frames[0]
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(p1_id)));
        assert!(pages2[1].frames[0]
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(p2_id)));
        assert!(pages2[2].frames[0]
            .lines
            .iter()
            .any(|l| l.paragraph_id == Some(p3_id)));
    }

    #[test]
    fn test_typeset_paragraph_with_hebrew_drop_cap() {
        let eng = engine();
        let dc = tok_core::styles::DropCapConfig {
            enabled: true,
            lines: 3,
            whole_word: true,
            margin_pt: 8.0,
            ..Default::default()
        };

        let text = "בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ. \
                    וְהָאָ֗רֶץ הָיְתָ֥ה תֹ֙הוּ֙ וָבֹ֔הוּ וְחֹ֖שֶׁךְ עַל־פְּנֵ֣י תְה֑וֹם \
                    וְר֣וּחַ אֱלֹהִ֔ים מְרַחֶ֖פֶת עַל־פְּנֵ֥י הַמָּֽיִם. \
                    וַיֹּ֥אמֶר אֱלֹהִ֖ים יְהִ֣י א֑וֹר וַֽיְהִי־אֽוֹר. \
                    וַיַּ֧רְא אֱלֹהִ֛ים אֶת־הָא֖וֹר כִּי־ט֑וֹב וַיַּבְדֵּ֣ל אֱלֹהִ֔ים בֵּ֥ין הָא֖וֹר וּבֵ֥ין הַחֹֽשֶׁךְ.";
        let p = para(text).with_drop_cap(dc);

        let col_w = 320.0;
        let line_h = 16.0;
        let profile = MeasureProfile::uniform(col_w);

        let lines = eng.typeset_paragraph_shaped(&p, DEFAULT_FONT_FAMILY, &profile, 11.0, line_h);

        assert!(
            lines.len() >= 4,
            "Must produce drop cap plus multiple body lines"
        );

        // Line 0 is the drop cap window
        let drop_line = &lines[0];
        assert!(drop_line.is_drop_cap, "Line 0 must be marked as drop cap");
        assert_eq!(
            drop_line.text,
            tok_core::normalizer::HebrewNormalizer::normalize("בְּרֵאשִׁ֖ית")
        );
        assert_eq!(drop_line.height, 3.0 * line_h);
        assert!(drop_line.width > 30.0);
        let win_x = drop_line.x_offset;
        assert!(
            (win_x + drop_line.width - col_w).abs() < 1e-2,
            "Drop cap must align to right in RTL"
        );

        // Body lines 1, 2, 3 must be indented to the left of the drop cap window
        for bl in &lines[1..4] {
            assert!(!bl.is_drop_cap);
            assert!(
                bl.width <= col_w - drop_line.width + 1.0,
                "Body lines adjacent to drop cap must notch within available measure"
            );
            for g in &bl.glyphs {
                assert!(
                    g.x <= win_x + 1.0,
                    "Body glyphs must not overlap drop cap window horizontally"
                );
            }
        }

        // Lines 4+ (below the drop cap window) should have full width potential
        if lines.len() > 4 {
            let last_line = lines.last().unwrap();
            assert!(!last_line.is_drop_cap);
        }
    }

    #[test]
    fn test_typeset_paragraph_talmud_drop_cap() {
        let eng = engine();
        let dc = tok_core::styles::DropCapConfig {
            enabled: true,
            lines: 2,
            whole_word: true,
            margin_pt: 6.0,
            ..Default::default()
        };

        let text = "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן \
                    עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה דִּבְרֵי רַבִּי אֱלִיעֶזֶר.";
        let p = para(text).with_drop_cap(dc);

        let lines = eng.typeset_paragraph_shaped(
            &p,
            DEFAULT_FONT_FAMILY,
            &MeasureProfile::uniform(300.0),
            12.0,
            16.0,
        );
        assert!(lines.len() >= 3);
        assert!(lines[0].is_drop_cap);
        assert_eq!(lines[0].text, "מֵאֵימָתַי");
        assert_eq!(lines[0].height, 32.0); // 2 lines * 16.0
    }
}

//! Continuous multi-column pagination engine & 5-stage deterministic balance solver.
//!
//! Computes multi-column geometry (with RTL column 0 on the right side), handles
//! mixed `ColumnSpanMode::SpanAllColumns` bands, coordinates bottom footnotes
//! via `DynamicFootnotePaginator`, aligns column bottoms using a 5-stage `Mpt`
//! solver + `SharedColumnHeightBalancer`, emits `column_rules`, and reports
//! structured `LayoutDiagnostic` warnings when residual balance delta exceeds `2.0 pt`.

use crate::co_pagination::{BalanceTargetMode, HeightBalanceConfig, SharedColumnHeightBalancer};
use crate::diagnostics::{DiagnosticSeverity, LayoutDiagnostic};
use crate::footnote_paginator::{DynamicFootnotePaginator, FootnotePaginatorConfig};
use crate::gematria::GematriaEngine;
use crate::geometry::{
    LineBox, PageLayoutBox, PhysicalRect, PlacedImageBox, PlacedOrnamentBox, TextFrameBox,
};
use crate::mpt::Mpt;
use crate::SpreadSide;
use tok_core::id::NodeId;
use tok_core::model::{ImageAttachment, OrnamentRef};
use tok_core::overrides::PageBreakMode;
use tok_core::styles::{Color, ColumnDistributionMode, ColumnSpanMode, TextAlignment};

/// Minimum permitted physical width (pt) per column before column count is automatically reduced.
pub const MIN_COLUMN_WIDTH_PT: f32 = 36.0;

/// Tolerance threshold (2.0 pt = 2000 mpt) for column bottom balance verification.
pub const COLUMN_BALANCE_TOLERANCE_MPT: Mpt = Mpt(2_000);

/// Pure geometric helper for continuous multi-column grids.
pub struct ColumnGeometry;

impl ColumnGeometry {
    /// Returns effective column count in `1..=6` ensuring each column is at least
    /// [`MIN_COLUMN_WIDTH_PT`] wide (whenever `total_width_pt >= MIN_COLUMN_WIDTH_PT`).
    pub fn effective_column_count(
        total_width_pt: f32,
        requested_columns: u8,
        gutter_pt: f32,
    ) -> usize {
        let mut n = requested_columns.clamp(1, 6) as usize;
        let g = gutter_pt.max(0.0);
        while n > 1 {
            let col_w = (total_width_pt - g * (n - 1) as f32) / n as f32;
            if col_w >= MIN_COLUMN_WIDTH_PT {
                break;
            }
            n -= 1;
        }
        n.max(1)
    }

    /// Computes column rectangles inside `area`.
    ///
    /// - When `is_rtl == true`, column `0` is rightmost and column `n - 1` is leftmost.
    /// - When `is_rtl == false`, column `0` is leftmost and column `n - 1` is rightmost.
    pub fn compute(
        area: PhysicalRect,
        column_count: u8,
        gutter_pt: f32,
        is_rtl: bool,
    ) -> Vec<PhysicalRect> {
        let n = Self::effective_column_count(area.width, column_count, gutter_pt);
        let gutter = if n > 1 { gutter_pt.max(0.0) } else { 0.0 };
        let col_w = ((area.width - gutter * (n - 1) as f32) / n as f32).max(1.0);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let x = if is_rtl {
                area.x + area.width - ((i + 1) as f32 * col_w) - (i as f32 * gutter)
            } else {
                area.x + (i as f32) * (col_w + gutter)
            };
            out.push(PhysicalRect::new(x, area.y, col_w, area.height));
        }
        out
    }

    /// Computes `(x1, y1, x2, y2)` vertical separator rule lines centered in each gutter
    /// between consecutive columns `[0..active_cols]` spanning `[band_top, band_bottom]`.
    pub fn gutter_rules(
        col_rects: &[PhysicalRect],
        active_cols: usize,
        band_top: f32,
        band_bottom: f32,
    ) -> Vec<(f32, f32, f32, f32)> {
        if col_rects.len() < 2 || active_cols < 2 || band_bottom <= band_top {
            return Vec::new();
        }
        let pairs = active_cols.min(col_rects.len()) - 1;
        let mut rules = Vec::with_capacity(pairs);
        for i in 0..pairs {
            let a = col_rects[i];
            let b = col_rects[i + 1];
            let (left, right) = if a.x <= b.x { (a, b) } else { (b, a) };
            let x_mid = (left.x + left.width + right.x) * 0.5;
            rules.push((x_mid, band_top, x_mid, band_bottom));
        }
        rules
    }
}

/// Shaped paragraph input supplied by `TypesettingEngine` into `ContinuousColumnPaginator`.
#[derive(Debug, Clone)]
pub struct ColumnParagraphInput {
    pub section_idx: usize,
    pub paragraph_idx: usize,
    pub paragraph_id: Option<NodeId>,
    pub flow_id: String,
    pub lines: Vec<LineBox>,
    pub space_before_pt: f32,
    pub space_after_pt: f32,
    pub keep_with_next: bool,
    pub keep_together: bool,
    pub orphan_control: u8,
    pub widow_control: u8,
    pub break_before: PageBreakMode,
    pub force_odd_page: bool,
    pub column_span: ColumnSpanMode,
    pub is_rtl: bool,
    pub alignment: TextAlignment,
    pub image: Option<ImageAttachment>,
    pub ornament: Option<OrnamentRef>,
}

/// Flattened line item with integer `Mpt` metrics and attachment metadata.
#[derive(Debug, Clone)]
pub struct ColumnLineItem {
    pub line: LineBox,
    pub para_idx: usize,
    pub line_idx_in_para: usize,
    pub para_line_count: usize,
    pub base_height_mpt: Mpt,
    pub advance_before_mpt: Mpt,
    pub advance_after_mpt: Mpt,
    pub keep_with_next: bool,
    pub keep_together: bool,
    pub orphan_lines: usize,
    pub widow_lines: usize,
    pub break_before: PageBreakMode,
    pub force_odd_page: bool,
    pub column_span: ColumnSpanMode,
    pub alignment: TextAlignment,
    pub image: Option<ImageAttachment>,
    pub ornament: Option<OrnamentRef>,
}

/// Section-level configuration passed to `ContinuousColumnPaginator`.
#[derive(Debug, Clone)]
pub struct ColumnFlowConfig {
    pub section_idx: usize,
    pub start_page_num: usize,
    pub flow_id: String,
    pub page_dimensions: PhysicalRect,
    pub column_count: u8,
    pub column_gutter_pt: f32,
    pub column_distribution: ColumnDistributionMode,
    pub draw_column_rule: bool,
    pub is_rtl: bool,
    pub footnote_flow_id: String,
    pub footnote_config: FootnotePaginatorConfig,
}

impl Default for ColumnFlowConfig {
    fn default() -> Self {
        Self {
            section_idx: 0,
            start_page_num: 1,
            flow_id: "main".to_string(),
            page_dimensions: PhysicalRect::a4_portrait(),
            column_count: 2,
            column_gutter_pt: 12.0,
            column_distribution: ColumnDistributionMode::Balanced,
            draw_column_rule: false,
            is_rtl: true,
            footnote_flow_id: "notes".to_string(),
            footnote_config: FootnotePaginatorConfig::default(),
        }
    }
}

/// Prefix sums in integer millipoints enabling `O(1)` column height queries.
struct PrefixMetrics {
    cum_full: Vec<Mpt>,
    adv_before: Vec<Mpt>,
    adv_after: Vec<Mpt>,
}

impl PrefixMetrics {
    fn new(items: &[ColumnLineItem]) -> Self {
        let n = items.len();
        let mut cum_full = Vec::with_capacity(n + 1);
        let mut adv_before = Vec::with_capacity(n);
        let mut adv_after = Vec::with_capacity(n);
        cum_full.push(Mpt::ZERO);
        let mut acc = Mpt::ZERO;
        for item in items {
            adv_before.push(item.advance_before_mpt);
            adv_after.push(item.advance_after_mpt);
            acc += item.advance_before_mpt + item.base_height_mpt + item.advance_after_mpt;
            cum_full.push(acc);
        }
        Self {
            cum_full,
            adv_before,
            adv_after,
        }
    }

    /// Exact vertical height occupied by `items[start..end]` when placed as a contiguous column
    /// (top `space_before` collapses at column top; trailing `space_after` collapses at column bottom).
    #[inline]
    fn range_height(&self, start: usize, end: usize) -> Mpt {
        if start >= end {
            return Mpt::ZERO;
        }
        self.cum_full[end] - self.cum_full[start] - self.adv_before[start] - self.adv_after[end - 1]
    }
}

#[derive(Debug, Clone)]
pub struct ColumnSolveOutcome {
    pub frames: Vec<TextFrameBox>,
    pub placed_images: Vec<PlacedImageBox>,
    pub placed_ornaments: Vec<PlacedOrnamentBox>,
    pub diagnostics: Vec<LayoutDiagnostic>,
    pub band_height_pt: f32,
    pub active_cols: usize,
    pub residual_delta_mpt: Mpt,
}

/// 5-stage deterministic solver for balancing `SingleColumn` items across `N` columns on a page/band.
pub struct ColumnBalanceSolver;

#[derive(Clone, Debug)]
struct SplitCandidate {
    /// Length `num_cols + 1`, starts at `0`, ends at `items.len()`.
    bounds: Vec<usize>,
    delta: Mpt,
    penalty: Mpt,
    relaxed_wo: bool,
}

impl ColumnBalanceSolver {
    /// Evaluates penalty of splitting right before `idx` (`0 < idx < items.len()`).
    /// Returns `None` if forbidden under the current strictness mode.
    fn boundary_penalty(
        items: &[ColumnLineItem],
        idx: usize,
        allow_relaxed_wo: bool,
    ) -> Option<(Mpt, bool)> {
        if idx == 0 || idx >= items.len() {
            return Some((Mpt::ZERO, false));
        }
        let prev = &items[idx - 1];
        let next = &items[idx];
        let same_para = prev.para_idx == next.para_idx;

        if same_para {
            if prev.keep_together {
                return None;
            }
            let orphan_min = next.orphan_lines.min(next.para_line_count);
            let widow_min = next.widow_lines.min(next.para_line_count);
            let lines_before_split = next.line_idx_in_para;
            let lines_after_split = next.para_line_count.saturating_sub(next.line_idx_in_para);
            let wo_broken = (orphan_min > 1 && lines_before_split < orphan_min)
                || (widow_min > 1 && lines_after_split < widow_min);
            if wo_broken {
                if allow_relaxed_wo {
                    return Some((Mpt(8_000), true));
                } else {
                    return None;
                }
            }
            // Slight preference for paragraph boundaries over intra-paragraph splits when heights tie.
            Some((Mpt(200), false))
        } else if prev.keep_with_next {
            Some((Mpt(100_000), false))
        } else {
            Some((Mpt::ZERO, false))
        }
    }

    fn compute_split_delta(
        prefix: &PrefixMetrics,
        bounds: &[usize],
        total_items: usize,
        num_cols: usize,
    ) -> Mpt {
        let mut min_h = Mpt(i64::MAX);
        let mut max_h = Mpt::ZERO;
        let consider_empty = total_items >= num_cols;
        let mut counted = 0usize;
        for c in 0..num_cols {
            let a = bounds[c];
            let b = bounds[c + 1];
            if a == b && !consider_empty {
                continue;
            }
            let h = prefix.range_height(a, b);
            min_h = min_h.min(h);
            max_h = max_h.max(h);
            counted += 1;
        }
        if counted < 2 {
            Mpt::ZERO
        } else {
            max_h.saturating_sub(min_h)
        }
    }

    /// Stage 1: Greedy equal-height split (`total_mpt / num_cols`).
    fn stage1_greedy(
        items: &[ColumnLineItem],
        prefix: &PrefixMetrics,
        num_cols: usize,
        col_max_h: Mpt,
    ) -> Option<SplitCandidate> {
        let m = items.len();
        let total_h = prefix.range_height(0, m);
        let ideal = total_h / num_cols as i64;
        let mut bounds = Vec::with_capacity(num_cols + 1);
        bounds.push(0);
        let mut cur = 0usize;
        let mut total_penalty = Mpt::ZERO;

        for c in 0..(num_cols - 1) {
            let remaining_cols = num_cols - 1 - c;
            if cur >= m {
                bounds.push(m);
                continue;
            }
            // Pick split end >= cur maximizing proximity to ideal target while fitting col_max_h.
            let mut best_end: Option<(usize, Mpt, Mpt)> = None;
            let max_end = if m >= num_cols {
                m.saturating_sub(remaining_cols).max(cur)
            } else {
                m
            };
            for e in (cur + 1)..=max_end {
                let h = prefix.range_height(cur, e);
                if h > col_max_h {
                    break;
                }
                let Some((pen, _)) = Self::boundary_penalty(items, e, false) else {
                    continue;
                };
                let score = h.abs_diff(ideal) + pen;
                if best_end.is_none_or(|(_, b_score, _)| score < b_score) {
                    best_end = Some((e, score, pen));
                }
            }
            if let Some((e, _, pen)) = best_end {
                cur = e;
                total_penalty += pen;
                bounds.push(cur);
            } else {
                // Allow keeping all remaining in current col or empty trailing col
                let mut e = cur;
                while e < m && prefix.range_height(cur, e + 1) <= col_max_h {
                    e += 1;
                }
                cur = e;
                bounds.push(cur);
            }
        }
        if prefix.range_height(cur, m) > col_max_h {
            return None;
        }
        bounds.push(m);
        let delta = Self::compute_split_delta(prefix, &bounds, m, num_cols);
        Some(SplitCandidate {
            bounds,
            delta,
            penalty: total_penalty,
            relaxed_wo: false,
        })
    }

    /// Stage 2 & Stage 3: Bounded branch-and-bound split search over `items[0..m]`.
    fn bounded_search(
        items: &[ColumnLineItem],
        prefix: &PrefixMetrics,
        num_cols: usize,
        col_max_h: Mpt,
        allow_relaxed_wo: bool,
    ) -> Option<SplitCandidate> {
        let m = items.len();
        let total_h = prefix.range_height(0, m);

        #[allow(clippy::too_many_arguments)]
        fn dfs_exact(
            items: &[ColumnLineItem],
            prefix: &PrefixMetrics,
            col_idx: usize,
            start: usize,
            num_cols: usize,
            col_max_h: Mpt,
            total_h: Mpt,
            allow_relaxed_wo: bool,
            cur_min: Mpt,
            cur_max: Mpt,
            cur_pen: Mpt,
            cur_relaxed: bool,
            bounds: &mut Vec<usize>,
            best: &mut Option<SplitCandidate>,
            states: &mut usize,
        ) {
            const MAX_STATES: usize = 8192;
            const MAX_BRANCHES: usize = 12;

            if *states >= MAX_STATES {
                return;
            }
            *states += 1;

            let m = items.len();
            if col_idx + 1 == num_cols {
                let h = prefix.range_height(start, m);
                if h > col_max_h {
                    return;
                }
                bounds.push(m);
                let delta = ColumnBalanceSolver::compute_split_delta(prefix, bounds, m, num_cols);
                let cand = SplitCandidate {
                    bounds: bounds.clone(),
                    delta,
                    penalty: cur_pen,
                    relaxed_wo: cur_relaxed,
                };
                bounds.pop();
                let cost = cand.delta + cand.penalty;
                if best.as_ref().is_none_or(|b| cost < b.delta + b.penalty) {
                    *best = Some(cand);
                }
                return;
            }

            if cur_max.0 >= cur_min.0 {
                let lb = (cur_max - cur_min) + cur_pen;
                if let Some(ref b) = best {
                    if lb >= b.delta + b.penalty {
                        return;
                    }
                }
            }

            let ideal_cum = (total_h * (col_idx as i64 + 1)) / num_cols as i64;
            let target_h = ideal_cum.saturating_sub(prefix.range_height(0, start));

            let mut cands: Vec<(usize, Mpt, Mpt, Mpt, bool)> = Vec::new();
            for e in start..=m {
                if e == start && m >= num_cols {
                    continue;
                }
                let h = prefix.range_height(start, e);
                if h > col_max_h {
                    break;
                }
                let (pen, rel) = if e == m {
                    (Mpt::ZERO, false)
                } else {
                    match ColumnBalanceSolver::boundary_penalty(items, e, allow_relaxed_wo) {
                        Some(v) => v,
                        None => continue,
                    }
                };
                let sort_key = h.abs_diff(target_h) + pen;
                cands.push((e, h, pen, sort_key, rel));
            }
            cands.sort_by_key(|c| c.3);
            cands.truncate(MAX_BRANCHES);

            for (e, h, pen, _, rel) in cands {
                let (next_min, next_max) = if e == start && m < num_cols {
                    (cur_min, cur_max)
                } else {
                    (cur_min.min(h), cur_max.max(h))
                };
                bounds.push(e);
                dfs_exact(
                    items,
                    prefix,
                    col_idx + 1,
                    e,
                    num_cols,
                    col_max_h,
                    total_h,
                    allow_relaxed_wo,
                    next_min,
                    next_max,
                    cur_pen + pen,
                    cur_relaxed || rel,
                    bounds,
                    best,
                    states,
                );
                bounds.pop();
            }
        }

        let mut bounds = vec![0];
        let mut best = None;
        let mut states = 0usize;
        dfs_exact(
            items,
            prefix,
            0,
            0,
            num_cols,
            col_max_h,
            total_h,
            allow_relaxed_wo,
            Mpt(i64::MAX),
            Mpt::ZERO,
            Mpt::ZERO,
            false,
            &mut bounds,
            &mut best,
            &mut states,
        );
        best
    }

    /// Fallback greedy split ignoring strict constraints only when even Stage 3 cannot fit within `col_max_h`.
    fn fallback_fit_split(
        items: &[ColumnLineItem],
        prefix: &PrefixMetrics,
        num_cols: usize,
        col_max_h: Mpt,
    ) -> SplitCandidate {
        let m = items.len();
        let total_h = prefix.range_height(0, m);
        let ideal = total_h / num_cols as i64;
        let mut bounds = vec![0];
        let mut cur = 0usize;
        for c in 0..(num_cols - 1) {
            let remaining = m.saturating_sub(cur);
            if remaining == 0 {
                bounds.push(m);
                continue;
            }
            let min_take = usize::from(remaining > (num_cols - 1 - c));
            let mut end = (cur + min_take).min(m);
            while end < m {
                let next_h = prefix.range_height(cur, end + 1);
                if next_h > col_max_h {
                    break;
                }
                let cur_h = prefix.range_height(cur, end);
                if end > cur && next_h.abs_diff(ideal) > cur_h.abs_diff(ideal) {
                    break;
                }
                end += 1;
            }
            cur = end;
            bounds.push(cur);
        }
        bounds.push(m);
        let delta = Self::compute_split_delta(prefix, &bounds, m, num_cols);
        SplitCandidate {
            bounds,
            delta,
            penalty: Mpt(50_000),
            relaxed_wo: true,
        }
    }

    /// Builds `TextFrameBox`es and attached images/ornaments from chosen column ranges,
    /// then runs Stage 4 (`SharedColumnHeightBalancer`) and Stage 5 (`LayoutDiagnostic`).
    #[allow(clippy::too_many_arguments)]
    pub fn solve_and_build(
        items: &[ColumnLineItem],
        col_rects: &[PhysicalRect],
        distribution: ColumnDistributionMode,
        page_index: usize,
        page_num: usize,
        band_idx: usize,
        flow_id: &str,
        y_top: f32,
        avail_h_pt: f32,
    ) -> ColumnSolveOutcome {
        let num_cols = col_rects.len().max(1);
        let prefix = PrefixMetrics::new(items);
        let col_max_h = Mpt::from_pt(avail_h_pt);
        let mut diagnostics = Vec::new();

        // Determine split ranges per column.
        let chosen: SplitCandidate = if distribution == ColumnDistributionMode::Sequential
            || num_cols == 1
        {
            let mut bounds = vec![0];
            let mut cur = 0usize;
            for _ in 0..(num_cols - 1) {
                let mut end = cur;
                while end < items.len() && prefix.range_height(cur, end + 1) <= col_max_h {
                    end += 1;
                }
                // Backtrack across keep_together/keep_with_next boundaries if we don't empty the column
                let mut safe_end = end;
                while safe_end > cur + 1 && safe_end < items.len() {
                    if Self::boundary_penalty(items, safe_end, false).is_some()
                        && items[safe_end - 1].para_idx != items[safe_end].para_idx
                        && !items[safe_end - 1].keep_with_next
                    {
                        break;
                    }
                    if Self::boundary_penalty(items, safe_end, false).is_some() {
                        break;
                    }
                    safe_end -= 1;
                }
                cur = if safe_end > cur { safe_end } else { end };
                bounds.push(cur);
            }
            bounds.push(items.len());
            let delta = Self::compute_split_delta(&prefix, &bounds, items.len(), num_cols);
            SplitCandidate {
                bounds,
                delta,
                penalty: Mpt::ZERO,
                relaxed_wo: false,
            }
        } else {
            // Stage 1: Greedy equal-height target
            let s1 = Self::stage1_greedy(items, &prefix, num_cols, col_max_h);
            if let Some(ref c1) = s1 {
                if c1.delta <= COLUMN_BALANCE_TOLERANCE_MPT && c1.penalty == Mpt::ZERO {
                    c1.clone()
                } else {
                    // Stage 2: Bounded strict branch-and-bound
                    let s2 = Self::bounded_search(items, &prefix, num_cols, col_max_h, false);
                    let best_strict = match (s1, s2) {
                        (Some(a), Some(b)) => {
                            if b.delta + b.penalty < a.delta + a.penalty {
                                Some(b)
                            } else {
                                Some(a)
                            }
                        }
                        (a, b) => a.or(b),
                    };
                    if let Some(ref bs) = best_strict {
                        if bs.delta <= COLUMN_BALANCE_TOLERANCE_MPT {
                            bs.clone()
                        } else {
                            // Stage 3: Relaxed widow/orphan search
                            let s3 =
                                Self::bounded_search(items, &prefix, num_cols, col_max_h, true);
                            match (best_strict, s3) {
                                (Some(a), Some(b)) if b.delta + Mpt(1_000) < a.delta => b,
                                (Some(a), _) => a,
                                (None, Some(b)) => b,
                                (None, None) => {
                                    Self::fallback_fit_split(items, &prefix, num_cols, col_max_h)
                                }
                            }
                        }
                    } else {
                        Self::bounded_search(items, &prefix, num_cols, col_max_h, true)
                            .unwrap_or_else(|| {
                                Self::fallback_fit_split(items, &prefix, num_cols, col_max_h)
                            })
                    }
                }
            } else {
                let s2 = Self::bounded_search(items, &prefix, num_cols, col_max_h, false);
                if let Some(b2) = s2 {
                    if b2.delta <= COLUMN_BALANCE_TOLERANCE_MPT {
                        b2
                    } else {
                        Self::bounded_search(items, &prefix, num_cols, col_max_h, true)
                            .filter(|b3| b3.delta + Mpt(1_000) < b2.delta)
                            .unwrap_or(b2)
                    }
                } else {
                    Self::bounded_search(items, &prefix, num_cols, col_max_h, true).unwrap_or_else(
                        || Self::fallback_fit_split(items, &prefix, num_cols, col_max_h),
                    )
                }
            }
        };

        if chosen.relaxed_wo {
            diagnostics.push(LayoutDiagnostic {
                code: "COLUMN_WIDOW_ORPHAN_RELAXED".to_string(),
                severity: DiagnosticSeverity::Info,
                message: format!(
                    "Page {}: relaxed widow/orphan rule to improve column balance",
                    page_num
                ),
                page_index,
                bottom_delta_pt: Some(chosen.delta.to_pt()),
                waiver: None,
            });
        }

        // Materialize frames & track which item indices map into each column line for image/ornament attachment.
        let mut frames = Vec::with_capacity(num_cols);
        let mut frame_item_refs: Vec<Vec<usize>> = Vec::with_capacity(num_cols);
        for (c, &col_rect) in col_rects.iter().enumerate().take(num_cols) {
            let start = chosen.bounds[c];
            let end = chosen.bounds[c + 1];
            let mut y_cursor_mpt = Mpt::ZERO;
            let mut lines = Vec::with_capacity(end.saturating_sub(start));
            let mut item_indices = Vec::with_capacity(end.saturating_sub(start));

            for (local_i, idx) in (start..end).enumerate() {
                let item = &items[idx];
                if local_i > 0 {
                    y_cursor_mpt += item.advance_before_mpt;
                }
                let baseline_mpt = y_cursor_mpt + item.base_height_mpt;
                let mut line = item.line.clone();
                line.baseline_y = baseline_mpt.to_pt();
                lines.push(line);
                item_indices.push(idx);
                y_cursor_mpt = baseline_mpt;
                if idx + 1 < end {
                    y_cursor_mpt += item.advance_after_mpt;
                }
            }

            frames.push(TextFrameBox {
                frame_id: format!("frame_{}_b{}_col_{}", page_num, band_idx, c),
                flow_id: flow_id.to_string(),
                rect: PhysicalRect::new(
                    col_rect.x,
                    y_top,
                    col_rect.width,
                    y_cursor_mpt.to_pt().max(0.0),
                ),
                lines,
            });
            frame_item_refs.push(item_indices);
        }

        // Stage 4: Micro-leading alignment when Balanced across >= 2 populated columns.
        let populated_for_balance = frames.iter().filter(|f| f.lines.len() >= 2).count();
        if distribution == ColumnDistributionMode::Balanced
            && num_cols >= 2
            && populated_for_balance >= 2
        {
            SharedColumnHeightBalancer::balance_page_frames(
                &mut frames,
                &HeightBalanceConfig {
                    max_line_expansion_pt: 2.0,
                    max_paragraph_expansion_pt: 4.0,
                    min_lines_for_balance: 2,
                    target_mode: BalanceTargetMode::TallestColumn,
                },
            );
            for frame in &mut frames {
                if let Some(last) = frame.lines.last() {
                    frame.rect.height = frame.rect.height.max(last.baseline_y);
                }
            }
        }

        // Measure band height and residual column balance delta in Mpt.
        let mut min_col_mpt = Mpt(i64::MAX);
        let mut max_col_mpt = Mpt::ZERO;
        let mut active_cols = 0usize;
        let mut empty_present_when_enough = false;

        for frame in &frames {
            if let Some(last) = frame.lines.last() {
                active_cols += 1;
                let col_bottom = Mpt::from_pt(last.baseline_y.max(frame.rect.height));
                min_col_mpt = min_col_mpt.min(col_bottom);
                max_col_mpt = max_col_mpt.max(col_bottom);
            } else if items.len() >= num_cols {
                empty_present_when_enough = true;
                min_col_mpt = Mpt::ZERO;
            }
        }
        let residual_delta_mpt =
            if active_cols >= 2 || (active_cols >= 1 && empty_present_when_enough) {
                max_col_mpt.saturating_sub(min_col_mpt)
            } else {
                Mpt::ZERO
            };

        // Stage 5: Emit structured warning if residual delta > 2.0 pt (Balanced multi-column).
        if distribution == ColumnDistributionMode::Balanced
            && num_cols >= 2
            && items.len() >= 2
            && residual_delta_mpt > COLUMN_BALANCE_TOLERANCE_MPT
        {
            let delta_pt = residual_delta_mpt.to_pt();
            diagnostics.push(LayoutDiagnostic {
                code: "COLUMN_BALANCE_DELTA_EXCEEDED".to_string(),
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "Page {}: residual column bottom delta is {:.2}pt (threshold 2.00pt)",
                    page_num, delta_pt
                ),
                page_index,
                bottom_delta_pt: Some(delta_pt),
                waiver: None,
            });
        }

        // Build placed images and ornaments using finalized frame coordinates.
        let mut placed_images = Vec::new();
        let mut placed_ornaments = Vec::new();
        for (frame, item_indices) in frames.iter().zip(frame_item_refs.iter()) {
            for (line, &item_idx) in frame.lines.iter().zip(item_indices.iter()) {
                let item = &items[item_idx];
                let line_top = (frame.rect.y + line.baseline_y - line.height).max(frame.rect.y);
                if let Some(img) = &item.image {
                    let w = img.width_pt.min(frame.rect.width).max(1.0);
                    let h = if img.width_pt > 0.0 && w < img.width_pt {
                        img.height_pt * (w / img.width_pt)
                    } else {
                        img.height_pt.max(1.0)
                    };
                    let x = align_box_x(
                        frame.rect.x,
                        frame.rect.width,
                        w,
                        img.alignment,
                        item.line.is_rtl,
                    );
                    placed_images.push(PlacedImageBox {
                        asset_path: img.asset_path.clone(),
                        rect: PhysicalRect::new(x, line_top, w, h),
                        intrinsic_width_px: img.intrinsic_width_px,
                        intrinsic_height_px: img.intrinsic_height_px,
                        effective_dpi_x: if w > 0.0 {
                            img.intrinsic_width_px as f32 * 72.0 / w
                        } else {
                            0.0
                        },
                        effective_dpi_y: if h > 0.0 {
                            img.intrinsic_height_px as f32 * 72.0 / h
                        } else {
                            0.0
                        },
                        paragraph_id: item.line.paragraph_id,
                    });
                }
                if let Some(orn) = &item.ornament {
                    let w = orn.width_pt.min(frame.rect.width).max(1.0);
                    let h = orn.height_pt.max(1.0);
                    let x = align_box_x(
                        frame.rect.x,
                        frame.rect.width,
                        w,
                        TextAlignment::Center,
                        item.line.is_rtl,
                    );
                    placed_ornaments.push(PlacedOrnamentBox {
                        ornament_id: orn.ornament_id.clone(),
                        asset_path: orn.asset_path.clone(),
                        rect: PhysicalRect::new(x, line_top, w, h),
                        color: orn.color.clone().unwrap_or_else(Color::default),
                    });
                }
            }
        }

        // Remove empty trailing frames so UI/PDF don't emit blank overlays.
        let retained_frames: Vec<TextFrameBox> =
            frames.into_iter().filter(|f| !f.lines.is_empty()).collect();

        let band_height_pt = max_col_mpt.to_pt().max(0.0);

        ColumnSolveOutcome {
            frames: retained_frames,
            placed_images,
            placed_ornaments,
            diagnostics,
            band_height_pt,
            active_cols,
            residual_delta_mpt,
        }
    }
}

#[inline]
fn align_box_x(
    container_x: f32,
    container_w: f32,
    box_w: f32,
    alignment: TextAlignment,
    is_rtl: bool,
) -> f32 {
    match alignment {
        TextAlignment::Center => container_x + (container_w - box_w).max(0.0) * 0.5,
        TextAlignment::Left => container_x,
        TextAlignment::Right => container_x + (container_w - box_w).max(0.0),
        TextAlignment::Justified => {
            if is_rtl {
                container_x + (container_w - box_w).max(0.0)
            } else {
                container_x
            }
        }
    }
}

/// Paginator for sections with `column_count > 1` or mixed `ColumnSpanMode::SpanAllColumns`.
pub struct ContinuousColumnPaginator;

impl ContinuousColumnPaginator {
    /// Converts structured paragraphs into flat [`ColumnLineItem`]s with integer `Mpt` metrics.
    pub fn flatten_paragraphs(paragraphs: &[ColumnParagraphInput]) -> Vec<ColumnLineItem> {
        let mut items = Vec::new();
        for (p_idx, p) in paragraphs.iter().enumerate() {
            let line_count = p.lines.len();
            let before_orn = p.ornament.as_ref().is_some_and(|o| {
                o.placement == tok_core::model::OrnamentPlacement::BeforeParagraph
            });
            let after_orn = p.ornament.is_some() && !before_orn;
            let orn_line_idx = if before_orn && line_count > 0 {
                Some(0)
            } else if after_orn && line_count > 0 {
                Some(line_count - 1)
            } else {
                None
            };
            let img_line_idx = if p.image.is_some() && line_count > 0 {
                if after_orn && line_count >= 2 {
                    Some(line_count - 2)
                } else {
                    Some(line_count - 1)
                }
            } else {
                None
            };

            for (l_idx, line) in p.lines.iter().enumerate() {
                let advance_before_mpt = if l_idx == 0 {
                    Mpt::from_pt(p.space_before_pt.max(0.0))
                } else {
                    Mpt::ZERO
                };
                let advance_after_mpt = if l_idx + 1 == line_count {
                    Mpt::from_pt(p.space_after_pt.max(0.0))
                } else {
                    Mpt::ZERO
                };
                items.push(ColumnLineItem {
                    line: line.clone(),
                    para_idx: p_idx,
                    line_idx_in_para: l_idx,
                    para_line_count: line_count,
                    base_height_mpt: Mpt::from_pt(line.height.max(1.0)),
                    advance_before_mpt,
                    advance_after_mpt,
                    keep_with_next: p.keep_with_next,
                    keep_together: p.keep_together,
                    orphan_lines: p.orphan_control.max(1) as usize,
                    widow_lines: p.widow_control.max(1) as usize,
                    break_before: if l_idx == 0 {
                        p.break_before
                    } else {
                        PageBreakMode::Auto
                    },
                    force_odd_page: l_idx == 0 && p.force_odd_page,
                    column_span: p.column_span,
                    alignment: p.alignment,
                    image: if img_line_idx == Some(l_idx) {
                        p.image.clone()
                    } else {
                        None
                    },
                    ornament: if orn_line_idx == Some(l_idx) {
                        p.ornament.clone()
                    } else {
                        None
                    },
                });
            }
        }
        items
    }

    /// Computes how many items from `slice` can fit across `num_cols` columns of height `col_max_h`,
    /// stopping at any forced page break after index 0.
    fn greedy_page_capacity(
        slice: &[ColumnLineItem],
        prefix: &PrefixMetrics,
        num_cols: usize,
        col_max_h: Mpt,
    ) -> usize {
        if slice.is_empty() {
            return 0;
        }
        let mut cur = 0usize;
        for col in 0..num_cols {
            let col_start = cur;
            while cur < slice.len() {
                if cur > 0
                    && (slice[cur].force_odd_page || slice[cur].break_before != PageBreakMode::Auto)
                {
                    break;
                }
                let h = prefix.range_height(col_start, cur + 1);
                if h > col_max_h {
                    if cur == col_start && col == 0 {
                        // Force at least one line forward to guarantee termination
                        cur += 1;
                    }
                    break;
                }
                cur += 1;
            }
            // Don't leave orphan/widow or break inside keep_together between columns unless at last column/page
            if cur > col_start && cur < slice.len() && col + 1 == num_cols {
                // Keep_together paragraph rollback on page boundary if it fits on next page
                let p_idx = slice[cur].para_idx;
                if slice[cur - 1].para_idx == p_idx && slice[cur].keep_together {
                    let mut p_start = cur;
                    while p_start > col_start && slice[p_start - 1].para_idx == p_idx {
                        p_start -= 1;
                    }
                    if p_start > col_start {
                        cur = p_start;
                    }
                }
            }
            if cur >= slice.len()
                || (cur > 0
                    && (slice[cur].force_odd_page
                        || slice[cur].break_before != PageBreakMode::Auto))
            {
                break;
            }
        }
        cur.max(1).min(slice.len())
    }

    /// Paginates `paragraphs` (and optional footnote lines) across one or more [`PageLayoutBox`]es.
    pub fn paginate_section(
        paragraphs: &[ColumnParagraphInput],
        footnote_lines: &[LineBox],
        config: &ColumnFlowConfig,
        content_rect_for_side: impl Fn(SpreadSide) -> PhysicalRect,
    ) -> Vec<PageLayoutBox> {
        let items = Self::flatten_paragraphs(paragraphs);
        let mut pages: Vec<PageLayoutBox> = Vec::new();
        let mut cursor = 0usize;
        let mut fn_cursor = 0usize;
        let mut current_page_num = config.start_page_num.max(1);

        if items.is_empty() && footnote_lines.is_empty() {
            return pages;
        }

        while cursor < items.len() || fn_cursor < footnote_lines.len() {
            // Check forced odd/even parity before opening the page
            if let Some(next_item) = items.get(cursor) {
                let want_odd =
                    next_item.force_odd_page || next_item.break_before == PageBreakMode::OddPage;
                let want_even = next_item.break_before == PageBreakMode::EvenPage;
                if (want_odd && current_page_num.is_multiple_of(2))
                    || (want_even && !current_page_num.is_multiple_of(2))
                {
                    let page_idx = current_page_num - 1;
                    pages.push(PageLayoutBox::blank(
                        page_idx,
                        GematriaEngine::to_hebrew_numeral(current_page_num),
                        config.page_dimensions,
                    ));
                    current_page_num += 1;
                }
            }

            let page_idx = current_page_num - 1;
            let side = if current_page_num % 2 == 1 {
                SpreadSide::Recto
            } else {
                SpreadSide::Verso
            };
            let area = content_rect_for_side(side);

            // Reserve bottom band for footnotes if any remain
            let mut page_frames = Vec::new();
            let mut page_rules = Vec::new();
            let mut page_diags = Vec::new();
            let mut page_images = Vec::new();
            let mut page_ornaments = Vec::new();

            let fn_rem = &footnote_lines[fn_cursor..];
            let (main_budget_h, fn_frame_opt) = if !fn_rem.is_empty() {
                if let Some(target_fn_h) = DynamicFootnotePaginator::calculate_target_height(
                    fn_rem,
                    area.height,
                    &config.footnote_config,
                ) {
                    let fn_slice = DynamicFootnotePaginator::slice_for_allocated_band(
                        fn_rem,
                        target_fn_h,
                        &config.footnote_config,
                    );
                    if fn_slice.lines_consumed > 0 {
                        let used_fn_h = fn_slice.actual_height_pt.max(
                            fn_slice
                                .placed_lines
                                .last()
                                .map(|l| l.baseline_y)
                                .unwrap_or(0.0),
                        );
                        let sep = config.footnote_config.separator_gap_pt;
                        let main_h = (area.height - used_fn_h - sep).max(18.0);
                        let fn_y = area.y + area.height - used_fn_h;
                        let fn_frame = TextFrameBox {
                            frame_id: format!(
                                "frame_{}_footnote_{}",
                                current_page_num, config.footnote_flow_id
                            ),
                            flow_id: config.footnote_flow_id.clone(),
                            rect: PhysicalRect::new(area.x, fn_y, area.width, used_fn_h),
                            lines: fn_slice.placed_lines,
                        };
                        fn_cursor += fn_slice.lines_consumed;
                        (main_h, Some(fn_frame))
                    } else {
                        (area.height, None)
                    }
                } else {
                    (area.height, None)
                }
            } else {
                (area.height, None)
            };

            let page_bottom_y = area.y + main_budget_h;
            let mut y_cursor = area.y;
            let mut band_idx = 0usize;
            let col_rects_full = ColumnGeometry::compute(
                area,
                config.column_count,
                config.column_gutter_pt,
                config.is_rtl,
            );
            let col_count = col_rects_full.len();

            let mut placed_any_on_page = false;
            while cursor < items.len() {
                // Forced break strictly after previous placed item opens a new page.
                if placed_any_on_page
                    && (items[cursor].force_odd_page
                        || items[cursor].break_before != PageBreakMode::Auto)
                {
                    break;
                }

                let avail_h = (page_bottom_y - y_cursor).max(0.0);
                if avail_h < items[cursor].line.height && placed_any_on_page {
                    break;
                }

                let is_span_all =
                    items[cursor].column_span == ColumnSpanMode::SpanAllColumns || col_count == 1;

                if is_span_all {
                    // Find contiguous span-all items ending before next SingleColumn or forced break
                    let mut run_end = cursor + 1;
                    while run_end < items.len()
                        && (items[run_end].column_span == ColumnSpanMode::SpanAllColumns
                            || col_count == 1)
                        && !items[run_end].force_odd_page
                        && items[run_end].break_before == PageBreakMode::Auto
                    {
                        run_end += 1;
                    }
                    let slice = &items[cursor..run_end];
                    let prefix = PrefixMetrics::new(slice);
                    let take = Self::greedy_page_capacity(
                        slice,
                        &prefix,
                        1,
                        Mpt::from_pt(avail_h.max(items[cursor].line.height)),
                    );
                    let full_band_rect = PhysicalRect::new(area.x, y_cursor, area.width, avail_h);
                    let outcome = ColumnBalanceSolver::solve_and_build(
                        &slice[..take],
                        &[full_band_rect],
                        ColumnDistributionMode::Sequential,
                        page_idx,
                        current_page_num,
                        band_idx,
                        &config.flow_id,
                        y_cursor,
                        avail_h.max(items[cursor].line.height),
                    );
                    let span_h = outcome.band_height_pt;
                    page_frames.extend(outcome.frames);
                    page_images.extend(outcome.placed_images);
                    page_ornaments.extend(outcome.placed_ornaments);
                    page_diags.extend(outcome.diagnostics);
                    y_cursor += span_h;
                    cursor += take;
                    band_idx += 1;
                    placed_any_on_page = true;
                    // If slice overflowed this band, page is full.
                    if take < slice.len() {
                        break;
                    }
                } else {
                    // Contiguous SingleColumn run up to next SpanAllColumns or forced break.
                    let mut run_end = cursor + 1;
                    while run_end < items.len()
                        && items[run_end].column_span == ColumnSpanMode::SingleColumn
                        && !items[run_end].force_odd_page
                        && items[run_end].break_before == PageBreakMode::Auto
                    {
                        run_end += 1;
                    }
                    let slice = &items[cursor..run_end];
                    let prefix = PrefixMetrics::new(slice);
                    let eff_avail = avail_h.max(items[cursor].line.height);
                    let take = Self::greedy_page_capacity(
                        slice,
                        &prefix,
                        col_count,
                        Mpt::from_pt(eff_avail),
                    );
                    let band_rects: Vec<PhysicalRect> = col_rects_full
                        .iter()
                        .map(|r| PhysicalRect::new(r.x, y_cursor, r.width, eff_avail))
                        .collect();
                    let outcome = ColumnBalanceSolver::solve_and_build(
                        &slice[..take],
                        &band_rects,
                        config.column_distribution,
                        page_idx,
                        current_page_num,
                        band_idx,
                        &config.flow_id,
                        y_cursor,
                        eff_avail,
                    );
                    let band_h = outcome.band_height_pt;
                    if config.draw_column_rule && outcome.active_cols >= 2 && band_h > 0.0 {
                        page_rules.extend(ColumnGeometry::gutter_rules(
                            &band_rects,
                            outcome.active_cols,
                            y_cursor,
                            y_cursor + band_h,
                        ));
                    }
                    page_frames.extend(outcome.frames);
                    page_images.extend(outcome.placed_images);
                    page_ornaments.extend(outcome.placed_ornaments);
                    page_diags.extend(outcome.diagnostics);
                    y_cursor += band_h;
                    cursor += take;
                    band_idx += 1;
                    placed_any_on_page = true;
                    // Only continue on same page if a SpanAllColumns block immediately follows!
                    if cursor >= items.len()
                        || items[cursor].column_span != ColumnSpanMode::SpanAllColumns
                        || (page_bottom_y - y_cursor) < items[cursor].line.height
                    {
                        break;
                    }
                }
            }

            if let Some(fn_frame) = fn_frame_opt {
                page_frames.push(fn_frame);
            }

            let mut page = PageLayoutBox::new(
                page_idx,
                GematriaEngine::to_hebrew_numeral(current_page_num),
                config.page_dimensions,
                page_frames,
                None,
            );
            page.column_rules = page_rules;
            page.diagnostics = page_diags;
            page.images = page_images;
            page.ornaments = page_ornaments;
            pages.push(page);
            current_page_num += 1;
        }

        pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::LineBox;

    fn make_line(idx: usize, para_id: NodeId, h: f32, text: &str) -> LineBox {
        LineBox {
            line_index: idx,
            paragraph_id: Some(para_id),
            baseline_y: h,
            height: h,
            width: 150.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: Vec::new(),
            text: text.to_string(),
            is_rtl: true,
            fonts: vec!["Noto Serif Hebrew".to_string()],
            ..Default::default()
        }
    }

    fn make_para(p_idx: usize, lines_count: usize, line_h: f32) -> ColumnParagraphInput {
        let pid = NodeId::new();
        let lines = (0..lines_count)
            .map(|i| make_line(i, pid, line_h, "שורה לדוגמה בעמודה"))
            .collect();
        ColumnParagraphInput {
            section_idx: 0,
            paragraph_idx: p_idx,
            paragraph_id: Some(pid),
            flow_id: "main".to_string(),
            lines,
            space_before_pt: 0.0,
            space_after_pt: 0.0,
            keep_with_next: false,
            keep_together: false,
            orphan_control: 2,
            widow_control: 2,
            break_before: PageBreakMode::Auto,
            force_odd_page: false,
            column_span: ColumnSpanMode::SingleColumn,
            is_rtl: true,
            alignment: TextAlignment::Right,
            image: None,
            ornament: None,
        }
    }

    #[test]
    fn rtl_column_zero_is_rightmost_and_enforces_min_width() {
        let area = PhysicalRect::new(40.0, 50.0, 320.0, 500.0);
        let rtl_cols = ColumnGeometry::compute(area, 2, 20.0, true);
        assert_eq!(rtl_cols.len(), 2);
        assert!(
            rtl_cols[0].x > rtl_cols[1].x,
            "In RTL, column 0 must be to the right of column 1: {:?}",
            rtl_cols
        );
        assert!((rtl_cols[0].width - 150.0).abs() < 1e-3);

        let ltr_cols = ColumnGeometry::compute(area, 2, 20.0, false);
        assert!(ltr_cols[0].x < ltr_cols[1].x);

        // Request 6 columns on 120pt -> reduced so each column >= 36pt
        let narrow =
            ColumnGeometry::compute(PhysicalRect::new(0.0, 0.0, 120.0, 400.0), 6, 12.0, true);
        assert!(narrow.len() < 6);
        assert!(narrow[0].width >= MIN_COLUMN_WIDTH_PT - 1e-3);
    }

    #[test]
    fn sequential_vs_balanced_column_distribution() {
        let paras = vec![make_para(0, 6, 14.0)];
        let area = PhysicalRect::new(40.0, 40.0, 340.0, 400.0);

        // Sequential puts all 6 lines into column 0 because column 0 can hold 400pt
        let seq_cfg = ColumnFlowConfig {
            column_count: 2,
            column_gutter_pt: 16.0,
            column_distribution: ColumnDistributionMode::Sequential,
            draw_column_rule: true,
            ..Default::default()
        };
        let seq_pages =
            ContinuousColumnPaginator::paginate_section(&paras, &[], &seq_cfg, |_| area);
        assert_eq!(seq_pages.len(), 1);
        assert_eq!(seq_pages[0].frames.len(), 1);
        assert_eq!(seq_pages[0].frames[0].lines.len(), 6);

        // Balanced splits evenly (3 + 3) across column 0 and column 1 within 2.0pt delta
        let bal_cfg = ColumnFlowConfig {
            column_count: 2,
            column_gutter_pt: 16.0,
            column_distribution: ColumnDistributionMode::Balanced,
            draw_column_rule: true,
            ..Default::default()
        };
        let bal_pages =
            ContinuousColumnPaginator::paginate_section(&paras, &[], &bal_cfg, |_| area);
        assert_eq!(bal_pages.len(), 1);
        assert_eq!(bal_pages[0].frames.len(), 2);
        assert_eq!(bal_pages[0].frames[0].lines.len(), 3);
        assert_eq!(bal_pages[0].frames[1].lines.len(), 3);
        assert!(bal_pages[0].diagnostics.is_empty());
        assert_eq!(bal_pages[0].column_rules.len(), 1);
    }

    #[test]
    fn balance_solver_micro_leads_under_2pt_and_warns_above_2pt() {
        let area = PhysicalRect::new(40.0, 40.0, 340.0, 400.0);
        // Case 1: Unequal line heights (14, 14, 15.5) vs (14, 14, 14) -> raw delta 1.5pt <= 2pt, Stage 4 aligns without error warning
        let pid_a = NodeId::new();
        let pid_b = NodeId::new();
        let p_a = ColumnParagraphInput {
            lines: vec![
                make_line(0, pid_a, 14.0, "א"),
                make_line(1, pid_a, 14.0, "ב"),
                make_line(2, pid_a, 15.5, "ג"),
            ],
            ..make_para(0, 0, 14.0)
        };
        let p_b = ColumnParagraphInput {
            lines: vec![
                make_line(0, pid_b, 14.0, "ד"),
                make_line(1, pid_b, 14.0, "ה"),
                make_line(2, pid_b, 14.0, "ו"),
            ],
            ..make_para(1, 0, 14.0)
        };
        let cfg = ColumnFlowConfig {
            column_count: 2,
            column_gutter_pt: 16.0,
            column_distribution: ColumnDistributionMode::Balanced,
            ..Default::default()
        };
        let pages_ok =
            ContinuousColumnPaginator::paginate_section(&[p_a, p_b], &[], &cfg, |_| area);
        assert_eq!(pages_ok.len(), 1);
        assert!(
            !pages_ok[0]
                .diagnostics
                .iter()
                .any(|d| d.code == "COLUMN_BALANCE_DELTA_EXCEEDED"),
            "Delta <= 2pt must not trigger exceeded diagnostic: {:?}",
            pages_ok[0].diagnostics
        );

        // Case 2: Large keep-together block (40pt * 3 = 120pt) + short 2nd paragraph (14pt * 2 = 28pt) -> delta >> 2pt -> diagnostic
        let mut big = make_para(0, 3, 40.0);
        big.keep_together = true;
        let mut small = make_para(1, 2, 14.0);
        small.keep_together = true;
        let pages_warn =
            ContinuousColumnPaginator::paginate_section(&[big, small], &[], &cfg, |_| area);
        assert!(
            pages_warn[0]
                .diagnostics
                .iter()
                .any(|d| d.code == "COLUMN_BALANCE_DELTA_EXCEEDED"
                    && d.severity == DiagnosticSeverity::Warning),
            "Expected COLUMN_BALANCE_DELTA_EXCEEDED warning when residual delta > 2.0pt"
        );
    }

    #[test]
    fn mixed_span_all_columns_and_footnotes_compose_on_same_page() {
        let area = PhysicalRect::new(40.0, 40.0, 360.0, 420.0);
        let mut title = make_para(0, 1, 24.0);
        title.column_span = ColumnSpanMode::SpanAllColumns;
        let body = make_para(1, 4, 14.0);
        let tail = make_para(2, 1, 18.0);
        let mut tail_span = tail;
        tail_span.column_span = ColumnSpanMode::SpanAllColumns;

        let fn_pid = NodeId::new();
        let fn_lines = vec![
            make_line(0, fn_pid, 11.0, "הערה א"),
            make_line(1, fn_pid, 11.0, "הערה ב"),
        ];

        let cfg = ColumnFlowConfig {
            column_count: 2,
            column_gutter_pt: 16.0,
            column_distribution: ColumnDistributionMode::Balanced,
            draw_column_rule: true,
            ..Default::default()
        };
        let pages = ContinuousColumnPaginator::paginate_section(
            &[title, body, tail_span],
            &fn_lines,
            &cfg,
            |_| area,
        );
        assert_eq!(pages.len(), 1);
        // Should contain: full-width header + 2 balanced body columns + full-width footer + footnote frame = 5 frames!
        assert_eq!(pages[0].frames.len(), 5);
        assert!((pages[0].frames[0].rect.width - 360.0).abs() < 1e-3);
        assert!(pages[0].frames[1].rect.x > pages[0].frames[2].rect.x);
        assert!((pages[0].frames[3].rect.width - 360.0).abs() < 1e-3);
        assert!(pages[0].frames[3].rect.y > pages[0].frames[1].rect.y);
        assert!(pages[0].frames[4].frame_id.contains("_footnote_notes"));
        // Column rule should cover only the 2-column middle band (not the SpanAllColumns header!)
        assert_eq!(pages[0].column_rules.len(), 1);
        let rule = pages[0].column_rules[0];
        assert!(rule.1 >= pages[0].frames[1].rect.y - 0.1);
        assert!(rule.3 <= pages[0].frames[3].rect.y + 0.1);
    }
}

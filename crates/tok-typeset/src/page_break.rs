//! Typographic Page-Break Penalty Model & Defect Prevention.
//!
//! Evaluates page-break candidates across consecutive lines and paragraphs,
//! enforcing TeX-style penalties and badness functions to eliminate:
//! - Stranded headings (`keep_with_next`)
//! - Orphan lines at the bottom of a page (`orphan_control`)
//! - Widow lines at the top of the subsequent page (`widow_control`)
//! - Excessive underfull or overfull page slack

use tok_core::NodeId;

/// Standard typographic penalties for page breaks.
pub struct PageBreakPenalty;

impl PageBreakPenalty {
    /// Severe penalty representing a prohibited break (e.g. immediately after a heading).
    pub const INFINITE: f32 = 10_000.0;
    /// Forced page break (e.g. explicit page-break directive or section boundary).
    pub const FORCED: f32 = -10_000.0;
    /// Penalty for creating an orphan line (fewer lines than `orphan_control` at bottom of page).
    pub const ORPHAN: f32 = 8_000.0;
    /// Penalty for creating a widow line (fewer lines than `widow_control` on the next page).
    pub const WIDOW: f32 = 8_000.0;
    /// Penalty for breaking right after a paragraph marked with `keep_with_next`.
    pub const KEEP_WITH_NEXT: f32 = 10_000.0;
}

/// A candidate point between two lines where a page break could potentially occur.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakOpportunity {
    /// Absolute index in the flattened lines sequence.
    pub global_line_idx: usize,
    /// ID of the paragraph this line belongs to.
    pub paragraph_id: Option<NodeId>,
    /// 0-based index of this line within its paragraph.
    pub line_in_paragraph: usize,
    /// Total lines comprising this paragraph.
    pub total_lines_in_paragraph: usize,
    /// Whether the paragraph's style requests keeping it with the next paragraph.
    pub keep_with_next: bool,
    /// Minimum lines that must remain at the bottom of the page (typically 2).
    pub orphan_control: u8,
    /// Minimum lines that must be pushed to the top of the next page (typically 2).
    pub widow_control: u8,
    /// Height of the line box itself.
    pub line_height: f32,
    /// Collapsed inter-paragraph margin after this line (if this is the last line of the paragraph).
    pub margin_after: f32,
    /// Whether the paragraph's manual lock overrides require keeping all lines together.
    pub keep_lines_together: bool,
}

impl BreakOpportunity {
    /// Returns true if this line is the final line of its paragraph.
    #[inline]
    pub fn is_last_line_of_paragraph(&self) -> bool {
        self.line_in_paragraph + 1 == self.total_lines_in_paragraph
    }

    /// Calculates typographic penalty for breaking *immediately after* this line.
    pub fn calculate_penalty(&self) -> f32 {
        let mut penalty = 0.0;

        // 1. Heading stranded check: if paragraph requests keep_with_next and this is its last line,
        // breaking immediately after it leaves the heading stranded at the page bottom.
        if self.keep_with_next && self.is_last_line_of_paragraph() {
            penalty += PageBreakPenalty::KEEP_WITH_NEXT;
        }

        // 2. Orphan check: if paragraph is broken, how many lines remain on the current page?
        // If fewer than orphan_control lines remain, heavily penalize.
        let lines_on_current_page = self.line_in_paragraph + 1;
        if lines_on_current_page < self.total_lines_in_paragraph {
            let min_orphans = (self.orphan_control as usize).max(1);
            if lines_on_current_page < min_orphans {
                penalty += PageBreakPenalty::ORPHAN;
            }

            // 3. Widow check: how many lines will be pushed to the next page?
            // If fewer than widow_control lines are pushed, heavily penalize.
            let lines_on_next_page = self.total_lines_in_paragraph - lines_on_current_page;
            let min_widows = (self.widow_control as usize).max(1);
            if lines_on_next_page < min_widows {
                penalty += PageBreakPenalty::WIDOW;
            }
        }

        // 4. Keep lines together: if paragraph has keep_lines_together, penalize any intra-paragraph break
        if self.keep_lines_together && !self.is_last_line_of_paragraph() {
            penalty += 25_000.0;
        }

        penalty
    }
}

/// Evaluates underfull badness for a page according to TeX's cubic badness model:
/// $B = 100 \times (\Delta h / h_{\text{target}})^3$.
pub fn compute_badness(accumulated_height: f32, target_height: f32) -> f32 {
    if !target_height.is_finite() || target_height <= 0.0 {
        return 0.0;
    }
    if accumulated_height > target_height {
        // Overfull page: extreme penalty proportional to overflow
        let overflow = accumulated_height - target_height;
        return 10_000.0 + (overflow * 100.0);
    }

    let deficit = target_height - accumulated_height;
    let ratio = (deficit / target_height).clamp(0.0, 1.0);
    // Cubic scaling: a nearly full page has badness near 0; a half-empty page has badness ~ 12.5; empty is 100.
    100.0 * ratio.powi(3)
}

/// Combined cost of taking a page break at a given height and opportunity.
pub fn score_page_break(
    accumulated_height: f32,
    target_height: f32,
    opportunity: &BreakOpportunity,
) -> f32 {
    let badness = compute_badness(accumulated_height, target_height);
    let penalty = opportunity.calculate_penalty();
    badness + penalty
}

use crate::geometry::{BreakToken, LineBox};

/// Input line with paragraph context and typographic constraints for pagination.
#[derive(Debug, Clone)]
pub struct PaginationLineInput {
    pub line: LineBox,
    pub section_idx: usize,
    pub paragraph_idx: usize,
    pub paragraph_id: Option<NodeId>,
    pub line_in_paragraph: usize,
    pub total_lines_in_paragraph: usize,
    pub keep_with_next: bool,
    pub orphan_control: u8,
    pub widow_control: u8,
    pub space_before: f32,
    pub space_after: f32,
    pub char_offset: usize,
    pub force_page_break_before: bool,
    pub force_page_break_after: bool,
    pub keep_lines_together: bool,
}

impl PaginationLineInput {
    pub fn to_opportunity(&self, margin_after: f32) -> BreakOpportunity {
        BreakOpportunity {
            global_line_idx: 0,
            paragraph_id: self.paragraph_id,
            line_in_paragraph: self.line_in_paragraph,
            total_lines_in_paragraph: self.total_lines_in_paragraph,
            keep_with_next: self.keep_with_next,
            orphan_control: self.orphan_control,
            widow_control: self.widow_control,
            line_height: self.line.height,
            margin_after,
            keep_lines_together: self.keep_lines_together,
        }
    }
}

/// A sliced page containing lines and their vertical positions.
#[derive(Debug, Clone)]
pub struct PageSlice {
    pub lines: Vec<LineBox>,
    pub total_height: f32,
    pub break_token: Option<BreakToken>,
}

/// Dynamic global page breaker evaluating candidate breaks with lookahead and penalty optimization.
#[derive(Debug, Clone)]
pub struct GlobalPageBreaker {
    pub target_height: f32,
    pub lookahead_window: usize,
}

impl GlobalPageBreaker {
    pub const DEFAULT_LOOKAHEAD: usize = 8;

    pub fn new(target_height: f32) -> Self {
        Self {
            target_height,
            lookahead_window: Self::DEFAULT_LOOKAHEAD,
        }
    }

    pub fn with_lookahead(target_height: f32, lookahead_window: usize) -> Self {
        Self {
            target_height,
            lookahead_window,
        }
    }

    /// Breaks a flat list of positioned lines into pages, choosing break points
    /// that minimize typographic badness and penalties (preventing orphans, widows, and stranded headings).
    pub fn break_pages(&self, lines: &[PaginationLineInput]) -> Vec<PageSlice> {
        if lines.is_empty() {
            return Vec::new();
        }

        let mut pages = Vec::new();
        let mut start_idx = 0;

        while start_idx < lines.len() {
            let (end_idx, total_height) = self.find_best_page_break(&lines[start_idx..]);
            let page_lines_input = &lines[start_idx..start_idx + end_idx];

            // Position lines vertically within this page slice with top-of-page margin collapsing
            let mut page_lines = Vec::with_capacity(page_lines_input.len());
            let mut curr_y = 0.0f32;
            let mut prev_sec = None;
            let mut active_drop_cap_bottom = 0.0f32;
            let mut prev_was_drop_cap_for_para: Option<usize> = None;

            for (l_idx, item) in page_lines_input.iter().enumerate() {
                if prev_sec != Some(item.section_idx) {
                    prev_sec = Some(item.section_idx);
                }

                if item.line.is_drop_cap {
                    let margin = if l_idx > 0 { item.space_before } else { 0.0 };
                    curr_y = curr_y.max(active_drop_cap_bottom) + margin;
                    let mut line = item.line.clone();
                    line.line_index = l_idx;
                    let rel_baseline =
                        if line.baseline_y > 0.0 && line.baseline_y <= line.height {
                            line.baseline_y
                        } else {
                            line.height * 0.78
                        };
                    line.baseline_y = curr_y + rel_baseline;
                    active_drop_cap_bottom = active_drop_cap_bottom.max(curr_y + line.height);
                    prev_was_drop_cap_for_para = Some(item.paragraph_idx);
                    page_lines.push(line);
                    continue;
                }

                let is_first_after_dc =
                    prev_was_drop_cap_for_para == Some(item.paragraph_idx);
                prev_was_drop_cap_for_para = None;

                let margin = if item.line_in_paragraph == 0 && l_idx > 0 && !is_first_after_dc {
                    item.space_before
                } else {
                    0.0
                };

                if item.line_in_paragraph == 0 && !is_first_after_dc {
                    curr_y = curr_y.max(active_drop_cap_bottom);
                }

                curr_y += margin;
                let mut line = item.line.clone();
                line.line_index = l_idx;
                line.baseline_y = curr_y + line.height;
                curr_y += line.height;

                if item.line_in_paragraph + 1 == item.total_lines_in_paragraph {
                    curr_y = curr_y.max(active_drop_cap_bottom);
                    active_drop_cap_bottom = 0.0;
                }

                page_lines.push(line);
            }

            let break_token = if start_idx + end_idx < lines.len() {
                let next_item = &lines[start_idx + end_idx];
                Some(BreakToken {
                    section_index: next_item.section_idx,
                    paragraph_index: next_item.paragraph_idx,
                    char_offset: next_item.char_offset,
                    paragraph_id: next_item.paragraph_id,
                })
            } else {
                None
            };

            pages.push(PageSlice {
                lines: page_lines,
                total_height,
                break_token,
            });

            start_idx += end_idx;
        }

        pages
    }

    /// Finds the best break point for a single page from the given slice of remaining lines.
    /// Returns (number of lines to take on this page, actual accumulated height).
    fn find_best_page_break(&self, remaining: &[PaginationLineInput]) -> (usize, f32) {
        if remaining.is_empty() {
            return (0, 0.0);
        }

        // 1. Scan forwards accumulating lines until the page is full or overflows
        struct Candidate {
            count: usize,
            height: f32,
            score: f32,
            penalty: f32,
        }

        let mut candidates = Vec::new();
        let mut curr_h = 0.0f32;
        let mut prev_sec = None;
        let mut active_drop_cap_bottom = 0.0f32;
        let mut prev_was_drop_cap_for_para: Option<usize> = None;

        for (idx, item) in remaining.iter().enumerate() {
            if idx > 0 && item.line_in_paragraph == 0 && item.force_page_break_before {
                // Forced break immediately before this line!
                break;
            }

            if prev_sec != Some(item.section_idx) {
                prev_sec = Some(item.section_idx);
            }

            if item.line.is_drop_cap {
                let margin = if idx > 0 { item.space_before } else { 0.0 };
                let dc_top = curr_h.max(active_drop_cap_bottom) + margin;
                let dc_bottom = dc_top + item.line.height;
                if dc_bottom > self.target_height && idx > 0 {
                    break;
                }
                curr_h = dc_top;
                active_drop_cap_bottom = active_drop_cap_bottom.max(dc_bottom);
                prev_was_drop_cap_for_para = Some(item.paragraph_idx);
                // Breaking immediately after a drop cap before its adjacent body lines is forbidden.
                candidates.push(Candidate {
                    count: idx + 1,
                    height: dc_bottom,
                    score: PageBreakPenalty::INFINITE * 10.0,
                    penalty: PageBreakPenalty::INFINITE * 10.0,
                });
                continue;
            }

            let is_first_after_dc = prev_was_drop_cap_for_para == Some(item.paragraph_idx);
            prev_was_drop_cap_for_para = None;

            let margin = if item.line_in_paragraph == 0 && idx > 0 && !is_first_after_dc {
                item.space_before
            } else {
                0.0
            };

            let base_h = if item.line_in_paragraph == 0 && !is_first_after_dc {
                curr_h.max(active_drop_cap_bottom)
            } else {
                curr_h
            };

            let mut next_h = base_h + margin + item.line.height;
            if item.line_in_paragraph + 1 == item.total_lines_in_paragraph {
                next_h = next_h.max(active_drop_cap_bottom);
            }

            if next_h.max(active_drop_cap_bottom) > self.target_height && idx > 0 {
                // We reached physical capacity. Stop collecting candidates.
                break;
            }

            curr_h = next_h;
            let effective_h = curr_h.max(active_drop_cap_bottom);
            let inside_drop_cap_window = curr_h + 1e-3 < active_drop_cap_bottom
                && item.line_in_paragraph + 1 < item.total_lines_in_paragraph;
            if item.line_in_paragraph + 1 == item.total_lines_in_paragraph {
                active_drop_cap_bottom = 0.0;
            }

            let margin_after = if item.line_in_paragraph + 1 == item.total_lines_in_paragraph {
                item.space_after
            } else {
                0.0
            };

            let opp = item.to_opportunity(margin_after);
            let mut penalty = opp.calculate_penalty();
            let mut score = score_page_break(effective_h, self.target_height, &opp);
            if inside_drop_cap_window {
                penalty += PageBreakPenalty::INFINITE * 5.0;
                score += PageBreakPenalty::INFINITE * 5.0;
            }

            candidates.push(Candidate {
                count: idx + 1,
                height: effective_h,
                score,
                penalty,
            });

            if item.line_in_paragraph + 1 == item.total_lines_in_paragraph
                && item.force_page_break_after
            {
                // Forced break immediately after this paragraph!
                return (idx + 1, curr_h);
            }
        }

        if candidates.is_empty() {
            // First line alone overflows target height: take 1 line anyway as emergency
            return (1, remaining[0].line.height);
        }

        // If all remaining lines fit on this final page, take them all!
        if candidates.len() == remaining.len() {
            let last = candidates.last().unwrap();
            return (last.count, last.height);
        }

        // 2. Look back within the lookahead window from the furthest candidate that fit
        let window_size = self.lookahead_window.min(candidates.len());
        let window_start = candidates.len() - window_size;
        let window = &candidates[window_start..];

        // First attempt: choose candidate with no severe penalty (penalty < PageBreakPenalty::ORPHAN)
        let clean_best = window
            .iter()
            .filter(|c| c.penalty < PageBreakPenalty::ORPHAN - 1.0)
            .min_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(best) = clean_best {
            return (best.count, best.height);
        }

        // Second attempt: choose lowest overall score in the window
        let best_in_window = window.iter().min_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(best) = best_in_window {
            return (best.count, best.height);
        }

        // Fallback: take the furthest candidate that physically fit
        let last = candidates.last().unwrap();
        (last.count, last.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_break_has_zero_penalty() {
        let opp = BreakOpportunity {
            global_line_idx: 3,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 2, // 3rd line of 6
            total_lines_in_paragraph: 6,
            keep_with_next: false,
            orphan_control: 2,
            widow_control: 2,
            line_height: 14.5,
            margin_after: 0.0,
            keep_lines_together: false,
        };
        assert_eq!(opp.calculate_penalty(), 0.0);
    }

    #[test]
    fn test_orphan_penalty_triggered() {
        // Line 0 of 5 -> only 1 line on current page, but orphan_control is 2
        let opp = BreakOpportunity {
            global_line_idx: 10,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 0,
            total_lines_in_paragraph: 5,
            keep_with_next: false,
            orphan_control: 2,
            widow_control: 2,
            line_height: 14.5,
            margin_after: 0.0,
            keep_lines_together: false,
        };
        let penalty = opp.calculate_penalty();
        assert!(penalty >= PageBreakPenalty::ORPHAN);
    }

    #[test]
    fn test_widow_penalty_triggered() {
        // Line 3 of 5 -> 4 lines on current page, only 1 line left for next page!
        let opp = BreakOpportunity {
            global_line_idx: 14,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 3,
            total_lines_in_paragraph: 5,
            keep_with_next: false,
            orphan_control: 2,
            widow_control: 2,
            line_height: 14.5,
            margin_after: 0.0,
            keep_lines_together: false,
        };
        let penalty = opp.calculate_penalty();
        assert!(penalty >= PageBreakPenalty::WIDOW);
    }

    #[test]
    fn test_keep_with_next_heading_penalty() {
        // 1-line heading with keep_with_next = true
        let opp = BreakOpportunity {
            global_line_idx: 5,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 0,
            total_lines_in_paragraph: 1,
            keep_with_next: true,
            orphan_control: 2,
            widow_control: 2,
            line_height: 20.0,
            margin_after: 8.0,
            keep_lines_together: false,
        };
        assert!(opp.is_last_line_of_paragraph());
        let penalty = opp.calculate_penalty();
        assert!(penalty >= PageBreakPenalty::KEEP_WITH_NEXT);
    }

    #[test]
    fn test_compute_badness_cubic_and_overflow() {
        let target = 500.0;
        // Perfect fit: 0 deficit -> 0 badness
        assert!((compute_badness(500.0, target) - 0.0).abs() < 1e-4);

        // Small deficit (450/500, ratio 0.1) -> 100 * (0.1)^3 = 0.1
        let b_450 = compute_badness(450.0, target);
        assert!(b_450 < 1.0);

        // Half page (250/500, ratio 0.5) -> 100 * (0.5)^3 = 12.5
        let b_250 = compute_badness(250.0, target);
        assert!((b_250 - 12.5).abs() < 1e-4);

        // Empty page (0/500, ratio 1.0) -> 100 * (1.0)^3 = 100.0
        let b_0 = compute_badness(0.0, target);
        assert!((b_0 - 100.0).abs() < 1e-4);

        // Overfull (510/500) -> 10,000 + 10 * 100 = 11,000
        let b_over = compute_badness(510.0, target);
        assert!(b_over >= 10_000.0);
    }

    #[test]
    fn test_score_page_break_combines_badness_and_penalty() {
        let opp_clean = BreakOpportunity {
            global_line_idx: 10,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 2,
            total_lines_in_paragraph: 5,
            keep_with_next: false,
            orphan_control: 2,
            widow_control: 2,
            line_height: 14.5,
            margin_after: 0.0,
            keep_lines_together: false,
        };
        let opp_heading = BreakOpportunity {
            global_line_idx: 11,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 0,
            total_lines_in_paragraph: 1,
            keep_with_next: true,
            orphan_control: 2,
            widow_control: 2,
            line_height: 20.0,
            margin_after: 10.0,
            keep_lines_together: false,
        };

        let score_clean = score_page_break(480.0, 500.0, &opp_clean);
        let score_heading = score_page_break(480.0, 500.0, &opp_heading);

        assert!(score_clean < 5.0);
        assert!(score_heading >= 10_000.0);
        assert!(score_clean < score_heading);
    }

    fn make_test_line(idx: usize, height: f32) -> LineBox {
        LineBox {
            line_index: idx,
            paragraph_id: None,
            baseline_y: height,
            height,
            width: 200.0,
            glyphs: Vec::new(),
            text: format!("line {}", idx),
            is_rtl: true,
            fonts: Vec::new(),
            is_drop_cap: false,
            x_offset: 0.0,
        }
    }

    #[test]
    fn test_global_page_breaker_all_fit_single_page() {
        let breaker = GlobalPageBreaker::new(500.0);
        let lines = (0..5)
            .map(|i| PaginationLineInput {
                line: make_test_line(i, 20.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 5,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            })
            .collect::<Vec<_>>();

        let pages = breaker.break_pages(&lines);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].lines.len(), 5);
        assert_eq!(pages[0].total_height, 100.0);
    }

    #[test]
    fn test_global_page_breaker_heading_not_stranded() {
        // Page target height = 100.0
        // P1 has 3 lines of 20pt = 60pt.
        // Heading H1 has 1 line of 25pt, space_before = 5.0, keep_with_next = true -> total = 90pt.
        // P2 has 3 lines of 20pt. First line of P2 would overflow (90 + 20 = 110pt > 100pt).
        // Greedy breaking would place H1 alone at the bottom of Page 1!
        // GlobalPageBreaker must push H1 to Page 2.
        let breaker = GlobalPageBreaker::new(100.0);

        let mut inputs = Vec::new();
        // P1: 3 lines
        for i in 0..3 {
            inputs.push(PaginationLineInput {
                line: make_test_line(i, 20.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 3,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 0.0,
                space_after: 5.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            });
        }
        // H1: 1 line heading
        inputs.push(PaginationLineInput {
            line: make_test_line(3, 25.0),
            section_idx: 0,
            paragraph_idx: 1,
            paragraph_id: Some(NodeId::new()),
            line_in_paragraph: 0,
            total_lines_in_paragraph: 1,
            keep_with_next: true,
            orphan_control: 1,
            widow_control: 1,
            space_before: 5.0,
            space_after: 5.0,
            char_offset: 0,
            force_page_break_before: false,
            force_page_break_after: false,
            keep_lines_together: false,
        });
        // P2: 3 lines
        for i in 0..3 {
            inputs.push(PaginationLineInput {
                line: make_test_line(4 + i, 20.0),
                section_idx: 0,
                paragraph_idx: 2,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 3,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 5.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            });
        }

        let pages = breaker.break_pages(&inputs);
        assert_eq!(pages.len(), 2);
        // Page 1 should contain P1's 3 lines ONLY (heading was pushed to page 2)
        assert_eq!(pages[0].lines.len(), 3);
        // Page 2 should begin with the heading (line index 0 on page 2 is line 3 'h1')
        assert_eq!(pages[1].lines.len(), 4);
        assert_eq!(pages[1].lines[0].text, "line 3");
    }

    #[test]
    fn test_global_page_breaker_orphan_protection() {
        // Page target height = 100.0
        // P1 has 4 lines of 20pt = 80pt.
        // P2 has 3 lines of 15pt. Line 0 of P2 fits (80 + 15 = 95pt <= 100pt).
        // Greedy breaking would leave Line 0 of P2 as an orphan at the bottom of Page 1.
        // GlobalPageBreaker must avoid orphan and break before P2.
        let breaker = GlobalPageBreaker::new(100.0);
        let mut inputs = Vec::new();

        for i in 0..4 {
            inputs.push(PaginationLineInput {
                line: make_test_line(i, 20.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 4,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            });
        }
        for i in 0..3 {
            inputs.push(PaginationLineInput {
                line: make_test_line(4 + i, 15.0),
                section_idx: 0,
                paragraph_idx: 1,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 3,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            });
        }

        let pages = breaker.break_pages(&inputs);
        assert_eq!(pages.len(), 2);
        // Page 1 has 4 lines (P1)
        assert_eq!(pages[0].lines.len(), 4);
        // Page 2 has all 3 lines of P2 (no 1-line orphan on page 1)
        assert_eq!(pages[1].lines.len(), 3);
        assert_eq!(pages[1].lines[0].text, "line 4");
    }

    #[test]
    fn test_global_page_breaker_widow_protection() {
        // Page target height = 100.0
        // P1 has 3 lines of 20pt = 60pt.
        // P2 has 3 lines of 18pt = 54pt. Lines 0 and 1 of P2 fit (60 + 36 = 96pt <= 100pt).
        // But taking 2 lines leaves line 2 as a widow on page 2!
        // GlobalPageBreaker must avoid widow and keep P2 together on page 2.
        let breaker = GlobalPageBreaker::new(100.0);
        let mut inputs = Vec::new();

        for i in 0..3 {
            inputs.push(PaginationLineInput {
                line: make_test_line(i, 20.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 3,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            });
        }
        for i in 0..3 {
            inputs.push(PaginationLineInput {
                line: make_test_line(3 + i, 18.0),
                section_idx: 0,
                paragraph_idx: 1,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: i,
                total_lines_in_paragraph: 3,
                keep_with_next: false,
                orphan_control: 2,
                widow_control: 2,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            });
        }

        let pages = breaker.break_pages(&inputs);
        assert_eq!(pages.len(), 2);
        // Page 1 has P1 (3 lines)
        assert_eq!(pages[0].lines.len(), 3);
        // Page 2 has all 3 lines of P2 (no 1-line widow on page 2)
        assert_eq!(pages[1].lines.len(), 3);
    }

    #[test]
    fn test_global_page_breaker_top_of_page_margin_collapsing() {
        let breaker = GlobalPageBreaker::new(50.0);
        let inputs = vec![
            PaginationLineInput {
                line: make_test_line(0, 30.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: 0,
                total_lines_in_paragraph: 1,
                keep_with_next: false,
                orphan_control: 1,
                widow_control: 1,
                space_before: 20.0,
                space_after: 20.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            },
            PaginationLineInput {
                line: make_test_line(1, 30.0),
                section_idx: 0,
                paragraph_idx: 1,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: 0,
                total_lines_in_paragraph: 1,
                keep_with_next: false,
                orphan_control: 1,
                widow_control: 1,
                space_before: 20.0,
                space_after: 20.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            },
        ];

        let pages = breaker.break_pages(&inputs);
        assert_eq!(pages.len(), 2);
        // On Page 2, line 1 is at top of page, so top margin collapses to 0.0
        assert_eq!(pages[1].lines[0].baseline_y, 30.0);
    }

    #[test]
    fn test_global_page_breaker_forced_page_break_and_keep_together() {
        let breaker = GlobalPageBreaker::new(100.0);

        // Test forced page break before
        let inputs_before = vec![
            PaginationLineInput {
                line: make_test_line(0, 30.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: 0,
                total_lines_in_paragraph: 1,
                keep_with_next: false,
                orphan_control: 1,
                widow_control: 1,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            },
            PaginationLineInput {
                line: make_test_line(1, 30.0),
                section_idx: 0,
                paragraph_idx: 1,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: 0,
                total_lines_in_paragraph: 1,
                keep_with_next: false,
                orphan_control: 1,
                widow_control: 1,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: true,
                force_page_break_after: false,
                keep_lines_together: false,
            },
        ];

        let pages_before = breaker.break_pages(&inputs_before);
        assert_eq!(
            pages_before.len(),
            2,
            "Forced break before must push line 1 to page 2"
        );

        // Test forced page break after
        let inputs_after = vec![
            PaginationLineInput {
                line: make_test_line(0, 30.0),
                section_idx: 0,
                paragraph_idx: 0,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: 0,
                total_lines_in_paragraph: 1,
                keep_with_next: false,
                orphan_control: 1,
                widow_control: 1,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: true,
                keep_lines_together: false,
            },
            PaginationLineInput {
                line: make_test_line(1, 30.0),
                section_idx: 0,
                paragraph_idx: 1,
                paragraph_id: Some(NodeId::new()),
                line_in_paragraph: 0,
                total_lines_in_paragraph: 1,
                keep_with_next: false,
                orphan_control: 1,
                widow_control: 1,
                space_before: 0.0,
                space_after: 0.0,
                char_offset: 0,
                force_page_break_before: false,
                force_page_break_after: false,
                keep_lines_together: false,
            },
        ];

        let pages_after = breaker.break_pages(&inputs_after);
        assert_eq!(
            pages_after.len(),
            2,
            "Forced break after must terminate page immediately"
        );
    }
}

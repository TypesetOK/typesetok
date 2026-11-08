use crate::geometry::BreakToken;
use crate::page_break::{GlobalPageBreaker, PageSlice, PaginationLineInput};

/// Convergence and diagnostic metrics resulting from incremental page breaking.
#[derive(Debug, Clone, PartialEq)]
pub struct IncrementalBreakDiagnostics {
    pub total_pages: usize,
    pub first_affected_page: usize,
    pub recomputed_pages_count: usize,
    pub reused_leading_pages: usize,
    pub reused_trailing_pages: usize,
    pub converged_at_page: Option<usize>,
    pub zero_cascade: bool,
}

/// Overall performance and convergence metrics for full document incremental typesetting.
#[derive(Debug, Clone, PartialEq)]
pub struct IncrementalTypesetDiagnostics {
    pub total_pages: usize,
    pub first_affected_page: usize,
    pub recomputed_pages: usize,
    pub reused_pages: usize,
    pub recomputed_paragraphs: usize,
    pub reused_paragraphs: usize,
    pub zero_cascade: bool,
    pub converged_at_page: Option<usize>,
    pub duration: std::time::Duration,
}

/// Incremental Page Breaker with break-token convergence detection.
///
/// Instead of re-breaking an entire multi-hundred page book from page 1,
/// the incremental breaker:
/// 1. Preserves leading pages prior to the edit location.
/// 2. Repaginates only from the first affected page.
/// 3. Detects when a newly broken page ends at a `BreakToken` identical to a previous
///    break token (the mathematical convergence point).
/// 4. Immediately terminates repagination upon convergence, splicing all downstream
///    pages from the previous layout.
#[derive(Debug, Clone)]
pub struct IncrementalPageBreaker {
    pub target_height_pt: f32,
}

impl IncrementalPageBreaker {
    pub fn new(target_height_pt: f32) -> Self {
        Self { target_height_pt }
    }

    /// Incrementally breaks `new_lines` into pages, reusing unchanged pages from `previous_slices`.
    ///
    /// `first_affected_line_idx` specifies the index in `new_lines` of the earliest modified item.
    pub fn break_pages_incremental(
        &self,
        new_lines: &[PaginationLineInput],
        previous_slices: &[PageSlice],
        first_affected_line_idx: usize,
    ) -> (Vec<PageSlice>, IncrementalBreakDiagnostics) {
        if previous_slices.is_empty() || new_lines.is_empty() {
            let breaker = GlobalPageBreaker::new(self.target_height_pt);
            let pages = breaker.break_pages(new_lines);
            let total = pages.len();
            return (
                pages,
                IncrementalBreakDiagnostics {
                    total_pages: total,
                    first_affected_page: 0,
                    recomputed_pages_count: total,
                    reused_leading_pages: 0,
                    reused_trailing_pages: 0,
                    converged_at_page: None,
                    zero_cascade: false,
                },
            );
        }

        // 1. Determine how many leading pages are completely unaffected.
        // A page is unaffected if all its lines precede `first_affected_line_idx`.
        let mut leading_pages = Vec::new();
        let mut curr_line_cursor = 0;
        let mut first_affected_page = 0;

        for (p_idx, slice) in previous_slices.iter().enumerate() {
            let slice_line_count = slice.lines.len();
            // If the whole slice ends before the first affected line, it's safe to keep
            if curr_line_cursor + slice_line_count <= first_affected_line_idx {
                leading_pages.push(slice.clone());
                curr_line_cursor += slice_line_count;
            } else {
                first_affected_page = p_idx;
                break;
            }
        }

        let reused_leading_count = leading_pages.len();

        // 2. Begin repaginating from curr_line_cursor forward
        let mut result_pages = leading_pages;
        let mut start_idx = curr_line_cursor;
        let breaker = GlobalPageBreaker::new(self.target_height_pt);
        let mut converged_at_page = None;
        let mut reused_trailing_count = 0;
        let mut recomputed_count = 0;

        while start_idx < new_lines.len() {
            // Check if start_idx in new_lines matches a break token at the start of any previous page
            // that occurred after the edit point
            let current_token = BreakToken {
                section_index: new_lines[start_idx].section_idx,
                paragraph_index: new_lines[start_idx].paragraph_idx,
                char_offset: new_lines[start_idx].char_offset,
                paragraph_id: new_lines[start_idx].paragraph_id,
            };

            // Check for convergence with previous slices only after repaginating the affected page
            if recomputed_count > 0 {
                let matching_prev_page_idx = previous_slices
                    .iter()
                    .enumerate()
                    .skip(first_affected_page + 1)
                    .position(|(idx, _)| {
                        if idx == 0 {
                            false
                        } else if let Some(prev_token) = &previous_slices[idx - 1].break_token {
                            prev_token == &current_token
                        } else {
                            false
                        }
                    })
                    .map(|pos| first_affected_page + 1 + pos);

                if let Some(prev_page_idx) = matching_prev_page_idx {
                    // Convergence achieved! All remaining lines from here onwards match previous_slices[prev_page_idx..]
                    let trailing = &previous_slices[prev_page_idx..];
                    reused_trailing_count = trailing.len();
                    converged_at_page = Some(result_pages.len());

                    for slice in trailing {
                        result_pages.push(slice.clone());
                    }
                    break;
                }
            }

            // Break one page
            let single_page_slices = breaker.break_pages(&new_lines[start_idx..]);
            if single_page_slices.is_empty() {
                break;
            }
            let first_slice = single_page_slices.into_iter().next().unwrap();
            let lines_consumed = first_slice.lines.len();

            if lines_consumed == 0 {
                break;
            }

            recomputed_count += 1;
            start_idx += lines_consumed;
            result_pages.push(first_slice);
        }

        let total = result_pages.len();
        let diagnostics = IncrementalBreakDiagnostics {
            total_pages: total,
            first_affected_page,
            recomputed_pages_count: recomputed_count,
            reused_leading_pages: reused_leading_count,
            reused_trailing_pages: reused_trailing_count,
            converged_at_page,
            zero_cascade: recomputed_count <= 1 && reused_trailing_count > 0,
        };

        (result_pages, diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::LineBox;

    fn make_dummy_line(sec: usize, p: usize, l: usize, height: f32) -> PaginationLineInput {
        PaginationLineInput {
            line: LineBox {
                line_index: l,
                paragraph_id: None,
                baseline_y: height,
                height,
                width: 400.0,
                glyphs: Vec::new(),
                text: format!("Line s{} p{} l{}", sec, p, l),
                is_rtl: true,
                fonts: Vec::new(),
                is_drop_cap: false,
                x_offset: 0.0,
            },
            section_idx: sec,
            paragraph_idx: p,
            paragraph_id: None,
            line_in_paragraph: l,
            total_lines_in_paragraph: 5,
            keep_with_next: false,
            orphan_control: 1,
            widow_control: 1,
            space_before: 0.0,
            space_after: 0.0,
            char_offset: l * 10,
            force_page_break_before: false,
            force_page_break_after: false,
            keep_lines_together: false,
            force_odd_page_break_before: false,
        }
    }

    #[test]
    fn test_incremental_page_breaker_convergence() {
        // Page height of 100 fits 5 lines of height 20
        let breaker = GlobalPageBreaker::new(100.0);

        // 30 lines -> 6 pages of 5 lines each
        let lines: Vec<PaginationLineInput> = (0..30)
            .map(|i| make_dummy_line(0, i / 5, i % 5, 20.0))
            .collect();

        let initial_slices = breaker.break_pages(&lines);
        assert_eq!(initial_slices.len(), 6);

        // Modify only line 28 (on page index 5, the last page)
        let inc_breaker = IncrementalPageBreaker::new(100.0);
        let (inc_slices, diag) = inc_breaker.break_pages_incremental(&lines, &initial_slices, 28);

        assert_eq!(inc_slices.len(), 6);
        assert_eq!(diag.first_affected_page, 5);
        assert_eq!(diag.reused_leading_pages, 5); // Pages 0..4 reused!
        assert_eq!(diag.recomputed_pages_count, 1); // Only Page 5 recomputed!
    }

    #[test]
    fn test_mid_document_convergence_preserves_trailing_pages() {
        // 50 lines -> 10 pages of 5 lines each (lines of height 20, page height 100)
        let breaker = GlobalPageBreaker::new(100.0);
        let lines: Vec<PaginationLineInput> = (0..50)
            .map(|i| make_dummy_line(0, i / 5, i % 5, 20.0))
            .collect();
        let initial_slices = breaker.break_pages(&lines);
        assert_eq!(initial_slices.len(), 10);

        // Simulate local edit in paragraph 4 (line 21, which is on Page 4)
        // that preserves the line count
        let inc_breaker = IncrementalPageBreaker::new(100.0);
        let (inc_slices, diag) = inc_breaker.break_pages_incremental(&lines, &initial_slices, 21);

        assert_eq!(inc_slices.len(), 10);
        assert_eq!(diag.reused_leading_pages, 4); // Pages 0..3 reused
        assert!(diag.converged_at_page.is_some());
        assert!(diag.reused_trailing_pages > 0); // Trailing pages preserved via convergence!
    }
}

//! Shared Column Height Balancing across Parallel Flows (איזון גובה תחתון משותף).
//!
//! Provides mathematical vertical justification and bottom baseline alignment
//! across parallel columns (Gemara, Rashi, Tosafot / Torah, Targum, Ramban):
//! 1. Bottom Baseline Envelope: Computes the target bottom baseline $Y_{\text{target}}$ across columns.
//! 2. Elastic Spring Distribution: Spreads vertical expansion across inter-line and inter-paragraph
//!    gaps in shorter columns so their bottom lines align with $Y_{\text{target}}$.
//! 3. Proportional Balance: Clamps max vertical spring expansion to prevent typographic defects.
//! 4. Section-End Equalization: Equalizes column heights when text finishes mid-page.

use crate::geometry::TextFrameBox;

/// Configuration for vertical column height balancing.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightBalanceConfig {
    /// Maximum vertical expansion per line gap in points (default: 3.0 pt).
    pub max_line_expansion_pt: f32,
    /// Maximum vertical expansion per paragraph boundary in points (default: 15.0 pt).
    pub max_paragraph_expansion_pt: f32,
    /// Minimum line count in a column to participate in balancing (default: 3).
    pub min_lines_for_balance: usize,
    /// Target mode for vertical alignment.
    pub target_mode: BalanceTargetMode,
}

impl Default for HeightBalanceConfig {
    fn default() -> Self {
        Self {
            max_line_expansion_pt: 3.5,
            max_paragraph_expansion_pt: 16.0,
            min_lines_for_balance: 3,
            target_mode: BalanceTargetMode::TallestColumn,
        }
    }
}

/// Target baseline selection mode for vertical column balancing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BalanceTargetMode {
    /// Align all columns to the bottom baseline of the tallest column on the page.
    TallestColumn,
    /// Align all columns to a fixed bottom baseline coordinate (in points).
    FixedPageBottom(f32),
    /// Align all columns to the average bottom baseline of all participating columns.
    AverageHeight,
}

/// Audit report detailing vertical balancing adjustments applied to a column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnBalanceReport {
    pub flow_id: String,
    pub original_bottom_y: f32,
    pub balanced_bottom_y: f32,
    pub spring_injected_pt: f32,
    pub gaps_expanded: usize,
}

pub struct SharedColumnHeightBalancer;

impl SharedColumnHeightBalancer {
    /// Balances text lines across parallel text frames on a page using distributed elastic springs,
    /// aligning their bottom baselines along a shared vertical baseline envelope.
    pub fn balance_page_frames(
        frames: &mut [TextFrameBox],
        config: &HeightBalanceConfig,
    ) -> Vec<ColumnBalanceReport> {
        let mut reports = Vec::new();

        // 1. Identify active participating frames and their current bottom baselines.
        // Detect whether frames share a common vertical origin `rect.y` or span stepped origins
        // (such as an L-shape `_expansion` frame below an upper column).
        let min_rect_y = frames
            .iter()
            .map(|f| f.rect.y)
            .fold(f32::INFINITY, |a, b| a.min(b));
        let max_rect_y = frames
            .iter()
            .map(|f| f.rect.y)
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));
        let has_stepped_origins = (max_rect_y - min_rect_y).abs() > 1.0;

        let mut column_bottoms: Vec<(usize, f32, f32)> = Vec::new();
        for (idx, frame) in frames.iter().enumerate() {
            let non_dc_lines = frame.lines.iter().filter(|l| !l.is_drop_cap).count();
            if non_dc_lines >= config.min_lines_for_balance {
                if let Some(last_line) = frame.lines.last() {
                    let local_bottom = last_line.baseline_y;
                    let page_bottom = if has_stepped_origins {
                        frame.rect.y + local_bottom
                    } else {
                        local_bottom
                    };
                    column_bottoms.push((idx, local_bottom, page_bottom));
                }
            }
        }

        if column_bottoms.is_empty() {
            return reports;
        }

        // 2. Determine target bottom baseline Y_target (in page space when stepped, local space when uniform)
        let target_page_y = match config.target_mode {
            BalanceTargetMode::TallestColumn => column_bottoms
                .iter()
                .map(|(_, _, page_y)| *page_y)
                .fold(0.0_f32, |acc, y| acc.max(y)),
            BalanceTargetMode::FixedPageBottom(y) => y,
            BalanceTargetMode::AverageHeight => {
                let sum: f32 = column_bottoms.iter().map(|(_, _, page_y)| *page_y).sum();
                sum / column_bottoms.len() as f32
            }
        };

        // 3. For each participating frame, distribute elastic springs to bridge the height deficit
        for (frame_idx, orig_bottom_y, orig_page_y) in column_bottoms {
            let raw_deficit_pt = target_page_y - orig_page_y;
            let frame = &mut frames[frame_idx];

            // Bound expansion so lines never overflow their own frame's physical height
            // (crucial when an upper column terminates above an L-shape expansion frame).
            let max_frame_room = if frame.rect.height > 0.0 {
                (frame.rect.height - orig_bottom_y).max(0.0)
            } else {
                raw_deficit_pt.max(0.0)
            };
            let deficit_pt = raw_deficit_pt.min(max_frame_room);

            if deficit_pt <= 0.5 {
                // Already at or exceeding target (or bounded by frame ceiling)
                reports.push(ColumnBalanceReport {
                    flow_id: frame.flow_id.clone(),
                    original_bottom_y: orig_bottom_y,
                    balanced_bottom_y: orig_bottom_y,
                    spring_injected_pt: 0.0,
                    gaps_expanded: 0,
                });
                continue;
            }

            let num_lines = frame.lines.len();
            if num_lines < 2 {
                continue;
            }

            // Determine if a drop cap window occupies the top of the frame so we don't
            // separate the drop cap from its adjacent body lines.
            let drop_cap_bottom = frame
                .lines
                .iter()
                .find(|l| l.is_drop_cap)
                .map(|l| l.height)
                .unwrap_or(0.0);

            // Identify gaps: inter-line and paragraph boundaries
            let mut gap_weights = Vec::with_capacity(num_lines - 1);
            let mut total_weight = 0.0;

            for i in 0..(num_lines - 1) {
                if frame.lines[i].is_drop_cap
                    || (drop_cap_bottom > 0.0
                        && frame.lines[i].paragraph_id == frame.lines[0].paragraph_id
                        && frame.lines[i + 1].baseline_y <= drop_cap_bottom + 0.5)
                {
                    gap_weights.push((0.0, false));
                    continue;
                }
                let is_para_boundary =
                    frame.lines[i].paragraph_id != frame.lines[i + 1].paragraph_id;
                let weight = if is_para_boundary {
                    // Paragraph boundaries absorb more vertical spring
                    config.max_paragraph_expansion_pt.max(1.0)
                } else {
                    // Regular line gap
                    config.max_line_expansion_pt.max(0.5)
                };
                gap_weights.push((weight, is_para_boundary));
                total_weight += weight;
            }

            if total_weight <= 0.0 {
                reports.push(ColumnBalanceReport {
                    flow_id: frame.flow_id.clone(),
                    original_bottom_y: orig_bottom_y,
                    balanced_bottom_y: orig_bottom_y,
                    spring_injected_pt: 0.0,
                    gaps_expanded: 0,
                });
                continue;
            }

            // Clamp total expansion by max capacity
            let allowable_expansion = deficit_pt.min(total_weight);

            // Apply cumulative vertical spring to each subsequent line
            let mut accumulated_spring = 0.0;
            let gaps_count = gap_weights.iter().filter(|(w, _)| *w > 0.0).count();

            for (i, (weight, is_para_boundary)) in gap_weights.into_iter().enumerate() {
                if weight <= 0.0 {
                    frame.lines[i + 1].baseline_y += accumulated_spring;
                    continue;
                }
                let gap_expansion = if is_para_boundary {
                    (allowable_expansion * (weight / total_weight))
                        .min(config.max_paragraph_expansion_pt)
                } else {
                    (allowable_expansion * (weight / total_weight))
                        .min(config.max_line_expansion_pt)
                };

                accumulated_spring += gap_expansion;
                frame.lines[i + 1].baseline_y += accumulated_spring;
            }

            let new_bottom_y = frame
                .lines
                .last()
                .map(|l| l.baseline_y)
                .unwrap_or(orig_bottom_y);

            reports.push(ColumnBalanceReport {
                flow_id: frame.flow_id.clone(),
                original_bottom_y: orig_bottom_y,
                balanced_bottom_y: new_bottom_y,
                spring_injected_pt: accumulated_spring,
                gaps_expanded: gaps_count,
            });
        }

        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LineBox, PhysicalRect};
    use tok_core::id::NodeId;

    fn deterministic_node_id(idx: usize) -> NodeId {
        let chars = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
        ];
        let c1 = chars[(idx / 16) % 16];
        let c2 = chars[idx % 16];
        NodeId::from_string(&format!("01ARZ3NDEKTSV4RRFFQ69G5F{}{}", c1, c2))
            .unwrap_or_else(|_| NodeId::new())
    }

    fn make_test_line(line_index: usize, paragraph_id: usize, baseline_y: f32) -> LineBox {
        LineBox {
            line_index,
            paragraph_id: Some(deterministic_node_id(paragraph_id)),
            baseline_y,
            height: 14.0,
            width: 150.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: Vec::new(),
            text: format!("Line {} in paragraph {}", line_index, paragraph_id),
            is_rtl: true,
            fonts: vec!["Frank Ruhl Libre".to_string()],
        }
    }

    fn make_test_frame(flow_id: &str, line_specs: &[(usize, f32)]) -> TextFrameBox {
        let lines = line_specs
            .iter()
            .enumerate()
            .map(|(idx, &(p_id, y))| make_test_line(idx, p_id, y))
            .collect();
        TextFrameBox {
            frame_id: format!("frame_{}", flow_id),
            flow_id: flow_id.to_string(),
            rect: PhysicalRect::new(0.0, 0.0, 200.0, 600.0),
            lines,
        }
    }

    #[test]
    fn test_talmud_spread_shared_column_height_balancing() {
        // Gemara has 10 lines ending at 500.0 pt (tallest column)
        let gemara_specs: Vec<(usize, f32)> =
            (0..10).map(|i| (0, 50.0 + i as f32 * 50.0)).collect();

        // Rashi has 8 lines ending at 480.0 pt (20 pt deficit)
        let rashi_specs: Vec<(usize, f32)> =
            (0..8).map(|i| (i / 4, 60.0 + i as f32 * 60.0)).collect();

        // Tosafot has 7 lines ending at 485.0 pt (15 pt deficit)
        let tosafot_specs: Vec<(usize, f32)> =
            (0..7).map(|i| (i / 3, 65.0 + i as f32 * 70.0)).collect();

        let mut frames = vec![
            make_test_frame("gemara", &gemara_specs),
            make_test_frame("rashi", &rashi_specs),
            make_test_frame("tosafot", &tosafot_specs),
        ];

        let config = HeightBalanceConfig {
            max_line_expansion_pt: 4.0,
            max_paragraph_expansion_pt: 15.0,
            min_lines_for_balance: 3,
            target_mode: BalanceTargetMode::TallestColumn,
        };

        let reports = SharedColumnHeightBalancer::balance_page_frames(&mut frames, &config);

        assert_eq!(reports.len(), 3);

        // Gemara was the tallest column -> 0 spring injected
        let gemara_rep = reports.iter().find(|r| r.flow_id == "gemara").unwrap();
        assert_eq!(gemara_rep.spring_injected_pt, 0.0);
        assert_eq!(gemara_rep.balanced_bottom_y, 500.0);

        // Rashi and Tosafot should now reach 500.0 pt
        let rashi_rep = reports.iter().find(|r| r.flow_id == "rashi").unwrap();
        assert!(
            (rashi_rep.balanced_bottom_y - 500.0).abs() < 0.1,
            "Rashi bottom must align to 500.0"
        );
        assert!(rashi_rep.spring_injected_pt > 0.0);

        let tosafot_rep = reports.iter().find(|r| r.flow_id == "tosafot").unwrap();
        assert!(
            (tosafot_rep.balanced_bottom_y - 500.0).abs() < 0.1,
            "Tosafot bottom must align to 500.0"
        );
        assert!(tosafot_rep.spring_injected_pt > 0.0);

        // Verify strict monotonic increasing baselines in all columns
        for frame in &frames {
            for i in 0..(frame.lines.len() - 1) {
                assert!(
                    frame.lines[i + 1].baseline_y > frame.lines[i].baseline_y,
                    "Baselines must be strictly monotonic in flow {}",
                    frame.flow_id
                );
            }
        }
    }

    #[test]
    fn test_mikraot_gedolot_4_stream_column_height_balancing() {
        // 4 streams: Torah (main), Targum, Rashi, Ramban
        let torah_specs: Vec<(usize, f32)> = (0..12).map(|i| (0, 40.0 + i as f32 * 45.0)).collect(); // ends at 535.0
        let targum_specs: Vec<(usize, f32)> =
            (0..10).map(|i| (i / 5, 45.0 + i as f32 * 50.0)).collect(); // ends at 495.0
        let rashi_specs: Vec<(usize, f32)> =
            (0..11).map(|i| (i / 4, 42.0 + i as f32 * 48.0)).collect(); // ends at 522.0
        let ramban_specs: Vec<(usize, f32)> =
            (0..9).map(|i| (i / 3, 50.0 + i as f32 * 55.0)).collect(); // ends at 490.0

        let mut frames = vec![
            make_test_frame("torah", &torah_specs),
            make_test_frame("targum", &targum_specs),
            make_test_frame("rashi", &rashi_specs),
            make_test_frame("ramban", &ramban_specs),
        ];

        let config = HeightBalanceConfig {
            max_line_expansion_pt: 5.0,
            max_paragraph_expansion_pt: 20.0,
            min_lines_for_balance: 3,
            target_mode: BalanceTargetMode::TallestColumn,
        };

        let reports = SharedColumnHeightBalancer::balance_page_frames(&mut frames, &config);

        assert_eq!(reports.len(), 4);
        let tallest = 535.0;

        for rep in &reports {
            assert!(
                (rep.balanced_bottom_y - tallest).abs() < 0.2,
                "Stream {} failed to balance to target tallest baseline {}: got {}",
                rep.flow_id,
                tallest,
                rep.balanced_bottom_y
            );
        }
    }

    #[test]
    fn test_height_balancing_respects_max_spring_clamp() {
        // 3 lines ending at 100.0 pt (deficit = 100.0 pt to target 200.0 pt)
        let specs = vec![(0, 20.0), (0, 60.0), (0, 100.0)];
        let mut frames = vec![
            make_test_frame("short", &specs),
            make_test_frame("tall", &[(0, 50.0), (0, 100.0), (0, 150.0), (0, 200.0)]),
        ];

        let config = HeightBalanceConfig {
            max_line_expansion_pt: 3.0,
            max_paragraph_expansion_pt: 10.0,
            min_lines_for_balance: 3,
            target_mode: BalanceTargetMode::TallestColumn,
        };

        let reports = SharedColumnHeightBalancer::balance_page_frames(&mut frames, &config);
        let short_rep = reports.iter().find(|r| r.flow_id == "short").unwrap();

        // 2 line gaps with 3.0 max expansion = max 6.0 pt total expansion
        assert!(
            short_rep.spring_injected_pt <= 6.01,
            "Spring must be clamped to max capacity (6.0), got {}",
            short_rep.spring_injected_pt
        );
        assert_eq!(
            short_rep.balanced_bottom_y,
            100.0 + short_rep.spring_injected_pt
        );
    }

    #[test]
    fn test_height_balancing_skips_short_columns_below_threshold() {
        // 2 lines in frame (below threshold of 3)
        let specs = vec![(0, 20.0), (0, 60.0)];
        let mut frames = vec![
            make_test_frame("tiny", &specs),
            make_test_frame("main", &[(0, 50.0), (0, 100.0), (0, 150.0)]),
        ];

        let config = HeightBalanceConfig {
            min_lines_for_balance: 3,
            ..Default::default()
        };

        let reports = SharedColumnHeightBalancer::balance_page_frames(&mut frames, &config);
        assert!(!reports.iter().any(|r| r.flow_id == "tiny"));
        assert_eq!(frames[0].lines.last().unwrap().baseline_y, 60.0);
    }
}

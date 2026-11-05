//! Multi-Column Polygon Flow Synthesizer for Absorbed Commentary Frames.
//!
//! Synthesizes non-rectangular geometric frames (L-shaped, stepped, and expanded columns)
//! and lays out Hebrew commentary streams across dynamic column boundaries using the
//! polygon-aware Knuth-Plass line breaker and Hebrew justifier.

use crate::engine::TypesettingEngine;
use crate::geometry::{LineBox, PhysicalRect};
use crate::polygon_engine::PolygonFrame;
use crate::solver::AbsorptionKind;
use tok_core::model::ParagraphNode;
use tok_core::styles::ParagraphStyle;
use tok_core::FlowId;

/// Specification for dynamic commentary column absorption.
#[derive(Debug, Clone, PartialEq)]
pub struct CommentaryAbsorptionSpec {
    pub absorbing_flow_id: FlowId,
    pub terminating_flow_id: FlowId,
    pub terminating_height_pt: f32,
    pub gutter_pt: f32,
    pub kind: AbsorptionKind,
}

impl CommentaryAbsorptionSpec {
    pub fn new(
        absorbing_flow_id: FlowId,
        terminating_flow_id: FlowId,
        terminating_height_pt: f32,
        gutter_pt: f32,
        kind: AbsorptionKind,
    ) -> Self {
        Self {
            absorbing_flow_id,
            terminating_flow_id,
            terminating_height_pt,
            gutter_pt,
            kind,
        }
    }
}

/// The result of typesetting a text stream into a non-rectangular polygon frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthesizedPolygonFlow {
    pub flow_id: FlowId,
    pub frame: PolygonFrame,
    pub lines: Vec<LineBox>,
    pub consumed_paragraphs: usize,
    pub is_complete: bool,
}

/// Synthesizer for multi-column text flows wrapped within non-rectangular polygon frames.
pub struct MultiColumnPolygonFlowSynthesizer;

impl MultiColumnPolygonFlowSynthesizer {
    /// Synthesizes an L-shaped `PolygonFrame` where an absorbing commentary column expands
    /// below an adjacent terminating flow.
    pub fn synthesize_l_shaped_frame(
        absorbing_rect: PhysicalRect,
        terminating_rect: PhysicalRect,
        gutter_pt: f32,
        content_bottom_pt: f32,
        kind: AbsorptionKind,
        terminating_is_left: bool,
    ) -> PolygonFrame {
        let absorption_top = terminating_rect.y + terminating_rect.height + gutter_pt;
        let upper_height = (absorption_top - absorbing_rect.y).max(0.0);

        // Upper band: absorbing column before expansion
        let upper_band = PhysicalRect::new(
            absorbing_rect.x,
            absorbing_rect.y,
            absorbing_rect.width,
            upper_height,
        );

        // Lower band: absorbed expansion region
        let lower_y = absorption_top;
        let lower_height = (content_bottom_pt - lower_y).max(0.0);

        let lower_band = match kind {
            AbsorptionKind::ColumnExtension => {
                // Occupies the column below the terminating stream
                PhysicalRect::new(
                    terminating_rect.x,
                    lower_y,
                    terminating_rect.width,
                    lower_height,
                )
            }
            AbsorptionKind::CombinedAdjacent | AbsorptionKind::FullWidth => {
                // Spans continuously across both columns including the gutter
                let left_x = if terminating_is_left {
                    terminating_rect.x
                } else {
                    absorbing_rect.x
                };
                let right_x = if terminating_is_left {
                    absorbing_rect.x + absorbing_rect.width
                } else {
                    terminating_rect.x + terminating_rect.width
                };
                let width = (right_x - left_x).max(0.0);
                PhysicalRect::new(left_x, lower_y, width, lower_height)
            }
        };

        PolygonFrame {
            bands: vec![upper_band, lower_band],
            polygon: None,
            exclusions: Vec::new(),
        }
    }

    /// Synthesizes a multi-stepped `PolygonFrame` where an absorbing column progressively
    /// expands across multiple adjacent terminating columns ordered by their termination Y.
    pub fn synthesize_multi_step_frame(
        absorbing_rect: PhysicalRect,
        terminating_rects: &[PhysicalRect],
        gutter_pt: f32,
        content_bottom_pt: f32,
    ) -> PolygonFrame {
        if terminating_rects.is_empty() {
            return PolygonFrame::from_rect(absorbing_rect);
        }

        let mut sorted: Vec<PhysicalRect> = terminating_rects.to_vec();
        sorted.sort_by(|a, b| (a.y + a.height).total_cmp(&(b.y + b.height)));

        let mut bands = Vec::with_capacity(sorted.len() + 1);
        let mut current_top = absorbing_rect.y;
        let mut current_left = absorbing_rect.x;
        let mut current_right = absorbing_rect.x + absorbing_rect.width;

        for term in &sorted {
            let step_y = (term.y + term.height + gutter_pt).clamp(current_top, content_bottom_pt);
            let band_h = step_y - current_top;
            if band_h > 1e-3 {
                bands.push(PhysicalRect::new(
                    current_left,
                    current_top,
                    (current_right - current_left).max(0.0),
                    band_h,
                ));
                current_top = step_y;
            }
            current_left = current_left.min(term.x);
            current_right = current_right.max(term.x + term.width);
        }

        let final_h = (content_bottom_pt - current_top).max(0.0);
        if final_h > 1e-3 {
            bands.push(PhysicalRect::new(
                current_left,
                current_top,
                (current_right - current_left).max(0.0),
                final_h,
            ));
        }

        PolygonFrame::stepped(bands)
    }

    /// Typesets a series of paragraphs into a `PolygonFrame`, calculating exact Knuth-Plass
    /// line breaking across variable column widths and stepped absorption transitions.
    #[allow(clippy::too_many_arguments)]
    pub fn typeset_flow_into_polygon(
        engine: &TypesettingEngine,
        flow_id: FlowId,
        paragraphs: &[ParagraphNode],
        styles: &[ParagraphStyle],
        frame: &PolygonFrame,
        default_font_family: &str,
        font_size_pt: f32,
        line_height_pt: f32,
    ) -> SynthesizedPolygonFlow {
        let max_capacity = frame.capacity_lines(line_height_pt);
        let mut lines = Vec::new();
        let mut consumed_paras = 0;
        let mut current_line_idx = 0;
        let default_style = ParagraphStyle::default();

        for p in paragraphs {
            if current_line_idx >= max_capacity {
                break;
            }

            let style = styles
                .iter()
                .find(|s| s.id == p.style_id)
                .unwrap_or(&default_style);

            let family = if p.style_id == default_style.id {
                default_font_family
            } else {
                &style.font_family
            };

            let remaining_capacity = max_capacity - current_line_idx;
            // Generate variable profile starting at current line position within the polygon frame
            let profile =
                frame.profile_for_paragraph(line_height_pt, current_line_idx, remaining_capacity);

            let mut p_lines =
                engine.typeset_paragraph_shaped(p, family, &profile, font_size_pt, line_height_pt);

            // Re-position baseline and coordinate origins within the polygon frame
            let bounds = frame.total_bounds();
            for (offset, line) in p_lines.iter_mut().enumerate() {
                let abs_line = current_line_idx + offset;
                let y_top = bounds.y + (abs_line as f32) * line_height_pt;
                line.baseline_y = y_top + line_height_pt;

                if let Some(span) = frame.span_at_y(y_top, line_height_pt) {
                    line.x_offset = span.x;
                }
            }

            current_line_idx += p_lines.len();
            lines.extend(p_lines);
            consumed_paras += 1;

            if current_line_idx >= max_capacity {
                break;
            }
        }

        let is_complete = consumed_paras == paragraphs.len();

        SynthesizedPolygonFlow {
            flow_id,
            frame: frame.clone(),
            lines,
            consumed_paragraphs: consumed_paras,
            is_complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::TypesettingEngineConfig;
    use tok_core::FractionalIndex;

    #[test]
    fn test_synthesize_l_shaped_frame_combined_and_extension() {
        let absorbing = PhysicalRect::new(310.0, 50.0, 140.0, 700.0);
        let terminating = PhysicalRect::new(50.0, 50.0, 140.0, 250.0);
        let gutter = 10.0;
        let content_bottom = 750.0;

        // 1. CombinedAdjacent: lower band merges from left edge (50) to right edge (450)
        let frame_combined = MultiColumnPolygonFlowSynthesizer::synthesize_l_shaped_frame(
            absorbing,
            terminating,
            gutter,
            content_bottom,
            AbsorptionKind::CombinedAdjacent,
            true, // terminating on left
        );
        assert_eq!(frame_combined.bands.len(), 2);
        // Upper band: absorbing column (x=310, w=140, y=50, h=260)
        assert_eq!(frame_combined.bands[0].x, 310.0);
        assert_eq!(frame_combined.bands[0].width, 140.0);
        assert_eq!(frame_combined.bands[0].y, 50.0);
        assert_eq!(frame_combined.bands[0].height, 260.0);

        // Lower band: wide continuous band (x=50, w=400, y=310, h=440)
        assert_eq!(frame_combined.bands[1].x, 50.0);
        assert_eq!(frame_combined.bands[1].width, 400.0);
        assert_eq!(frame_combined.bands[1].y, 310.0);
        assert_eq!(frame_combined.bands[1].height, 440.0);

        // 2. ColumnExtension: lower band stays under terminating column
        let frame_ext = MultiColumnPolygonFlowSynthesizer::synthesize_l_shaped_frame(
            absorbing,
            terminating,
            gutter,
            content_bottom,
            AbsorptionKind::ColumnExtension,
            true,
        );
        assert_eq!(frame_ext.bands[1].x, 50.0);
        assert_eq!(frame_ext.bands[1].width, 140.0);
        assert_eq!(frame_ext.bands[1].y, 310.0);
        assert_eq!(frame_ext.bands[1].height, 440.0);
    }

    #[test]
    fn test_typeset_commentary_flow_into_l_shaped_absorbed_frame() {
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let flow_id = FlowId::new("tosafot");

        // Upper band: x=200, y=0, w=100, h=60 (fits 4 lines at 15pt)
        // Lower band: x=0, y=60, w=300, h=150 (fits 10 lines at 15pt)
        let frame = PolygonFrame::l_shaped(200.0, 0.0, 100.0, 60.0, 0.0, 300.0, 150.0);

        let commentary_text = "מאימתי קורין את שמע בערבין משעה שהכהנים נכנסין לאכול בתרומתן עד סוף האשמורה הראשונה דברי רבי אליעזר וחכמים אומרים עד חצות רבן גמליאל אומר עד שיעלה עמוד השחר מעשה ובאו בניו מבית המשתה אמרו לו לא קרינו את שמע אמר להם אם לא עלה עמוד השחר חייבין אתם לקרות ולא זו בלבד אמרו אלא כל מה שאמרו חכמים עד חצות מצותן עד שיעלה עמוד השחר הקטר חלבים ואברים מצותן עד שיעלה עמוד השחר וכל הנאכלים ליום אחד מצותן עד שיעלה עמוד השחר אם כן למה אמרו חכמים עד חצות כדי להרחיק את האדם מן העבירה";

        let p1 = ParagraphNode::new(FractionalIndex::initial(), "default", commentary_text);

        let result = MultiColumnPolygonFlowSynthesizer::typeset_flow_into_polygon(
            &engine,
            flow_id,
            &[p1],
            &[],
            &frame,
            "Noto Serif Hebrew",
            10.0,
            15.0,
        );

        assert!(!result.lines.is_empty());
        assert!(
            result.lines.len() >= 6,
            "Expected at least 6 lines placed, got {}",
            result.lines.len()
        );

        // First 4 lines should fall in the upper band (x_offset = 200, span width = 100)
        for i in 0..4 {
            let l = &result.lines[i];
            assert_eq!(
                l.x_offset, 200.0,
                "Line {} x_offset should be in upper band",
                i
            );
            assert!(
                l.width <= 100.5,
                "Line {} width ({}) exceeded upper band width 100",
                i,
                l.width
            );
            assert_eq!(l.baseline_y, ((i + 1) as f32) * 15.0);
        }

        // Line 4 and subsequent lines should transition into lower expanded band (x_offset = 0, span width = 300)
        let l4 = &result.lines[4];
        assert_eq!(
            l4.x_offset, 0.0,
            "Line 4 should transition into lower expanded band (x_offset=0)"
        );
        assert_eq!(l4.baseline_y, 5.0 * 15.0);

        // Lines in the lower band should be significantly wider than lines in the upper band
        assert!(
            l4.width > 120.0,
            "Expanded line 4 width ({}) should be wider than upper band (100)",
            l4.width
        );
    }
}

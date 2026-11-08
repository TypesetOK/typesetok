//! Spatial Hit-Testing Engine for WYSIWYG Page Editing (Section 9.2 & Section 10).
//!
//! Provides sub-pixel accurate coordinate-to-text mapping:
//! - Maps physical points (pt) on a page to exact TextFrameBox, LineBox, GlyphBox, and cluster offset.
//! - Accurately differentiates leading vs trailing edge of glyphs for caret positioning.
//! - RTL-aware: accounts for right-to-left Hebrew layout and mixed Bidirectional runs.
//! - Computes multi-line selection bounding rectangles for mouse-drag visual selection.

use crate::geometry::{LineBox, PageLayoutBox, PhysicalPoint, PhysicalRect, TextFrameBox};
use serde::{Deserialize, Serialize};
use tok_core::id::NodeId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitTestResult {
    pub page_index: usize,
    pub frame_id: String,
    pub flow_id: String,
    pub paragraph_id: Option<NodeId>,
    pub line_index: usize,
    pub glyph_index: Option<usize>,
    pub cluster: u32,
    pub character: Option<char>,
    pub visual_x_pt: f32,
    pub visual_y_pt: f32,
    pub caret_height: f32,
    pub is_rtl: bool,
    pub is_leading_edge: bool,
}

/// (glyph index, cluster, character, caret x relative to the frame, leading edge)
type GlyphHit = (Option<usize>, u32, Option<char>, f32, bool);

pub struct HitTester;

impl HitTester {
    /// Hit-tests a physical coordinate (x_pt, y_pt) within a PageLayoutBox.
    /// Returns the exact text target and visual caret placement.
    pub fn hit_test(page: &PageLayoutBox, x_pt: f32, y_pt: f32) -> Option<HitTestResult> {
        if !x_pt.is_finite() || !y_pt.is_finite() {
            return None;
        }

        // 1. Find target frame: prioritize frames containing the point, or closest frame
        let target_frame = Self::find_best_frame(&page.frames, x_pt, y_pt)?;

        // Relative coordinates inside the frame
        let frame_rel_y = y_pt - target_frame.rect.y;
        let frame_rel_x = x_pt - target_frame.rect.x;

        // 2. Find best matching line; an empty frame puts the caret at its start
        let Some((line_idx, line)) = Self::find_best_line(&target_frame.lines, frame_rel_y) else {
            return Some(HitTestResult {
                page_index: page.page_index,
                frame_id: target_frame.frame_id.clone(),
                flow_id: target_frame.flow_id.clone(),
                paragraph_id: None,
                line_index: 0,
                glyph_index: None,
                cluster: 0,
                character: None,
                visual_x_pt: target_frame.rect.x,
                visual_y_pt: target_frame.rect.y,
                caret_height: 12.0,
                is_rtl: true,
                is_leading_edge: true,
            });
        };

        // 3. Resolve glyph and caret within the line
        let (glyph_idx, cluster, ch, glyph_rel_x, is_leading) =
            Self::hit_test_line_glyphs(line, frame_rel_x);

        let visual_x_pt = target_frame.rect.x + glyph_rel_x;
        let visual_y_pt = target_frame.rect.y + (line.baseline_y - line.height);

        Some(HitTestResult {
            page_index: page.page_index,
            frame_id: target_frame.frame_id.clone(),
            flow_id: target_frame.flow_id.clone(),
            paragraph_id: line.paragraph_id,
            line_index: line_idx,
            glyph_index: glyph_idx,
            cluster,
            character: ch,
            visual_x_pt,
            visual_y_pt,
            caret_height: line.height,
            is_rtl: line.is_rtl,
            is_leading_edge: is_leading,
        })
    }

    /// Computes visual highlight bounding rectangles for a selection spanning
    /// between two hit-test points on the same page.
    pub fn hit_test_range(
        page: &PageLayoutBox,
        start_pt: PhysicalPoint,
        end_pt: PhysicalPoint,
    ) -> Vec<PhysicalRect> {
        let Some(hit_a) = Self::hit_test(page, start_pt.x, start_pt.y) else {
            return Vec::new();
        };
        let Some(hit_b) = Self::hit_test(page, end_pt.x, end_pt.y) else {
            return Vec::new();
        };

        let Some(target_frame) = page.frames.iter().find(|f| f.frame_id == hit_a.frame_id) else {
            return Vec::new();
        };
        // Line indices of a hit in another frame do not index this frame.
        let hit_b = if hit_b.frame_id == hit_a.frame_id {
            hit_b
        } else {
            let last = target_frame.lines.len().saturating_sub(1);
            HitTestResult {
                line_index: last,
                visual_x_pt: target_frame
                    .lines
                    .get(last)
                    .map(|l| {
                        let (left, right) = l.horizontal_extent();
                        target_frame.rect.x + if l.is_rtl { left } else { right }
                    })
                    .unwrap_or(target_frame.rect.x),
                ..hit_b
            }
        };

        let (min_hit, max_hit) = if hit_a.line_index <= hit_b.line_index {
            (&hit_a, &hit_b)
        } else {
            (&hit_b, &hit_a)
        };

        let mut rects = Vec::new();
        let frame_x = target_frame.rect.x;

        if min_hit.line_index == max_hit.line_index {
            // Single-line selection
            let Some(line) = target_frame.lines.get(min_hit.line_index) else {
                return Vec::new();
            };
            let left_x = hit_a.visual_x_pt.min(hit_b.visual_x_pt);
            let right_x = hit_a.visual_x_pt.max(hit_b.visual_x_pt);
            let width = (right_x - left_x).max(2.0);
            let y = target_frame.rect.y + line.baseline_y - line.height;
            rects.push(PhysicalRect::new(left_x, y, width, line.height));
            return rects;
        }

        // Multi-line selection
        for l_idx in min_hit.line_index..=max_hit.line_index {
            let Some(line) = target_frame.lines.get(l_idx) else {
                continue;
            };
            let y = target_frame.rect.y + line.baseline_y - line.height;
            let (extent_left, extent_right) = line.horizontal_extent();
            let (line_left, line_right) = (frame_x + extent_left, frame_x + extent_right);

            let (left, right) = if l_idx == min_hit.line_index {
                // First line: from the hit to the logical end of the line.
                if line.is_rtl {
                    (line_left, min_hit.visual_x_pt)
                } else {
                    (min_hit.visual_x_pt, line_right)
                }
            } else if l_idx == max_hit.line_index {
                // Last line: from the logical start of the line to the hit.
                if line.is_rtl {
                    (max_hit.visual_x_pt, line_right)
                } else {
                    (line_left, max_hit.visual_x_pt)
                }
            } else {
                (line_left, line_right)
            };
            rects.push(PhysicalRect::new(
                left,
                y,
                (right - left).max(2.0),
                line.height,
            ));
        }

        rects
    }

    fn find_best_frame(frames: &[TextFrameBox], x: f32, y: f32) -> Option<&TextFrameBox> {
        let pt = PhysicalPoint { x, y };
        if let Some(frame) = frames.iter().find(|f| f.rect.contains_point(pt)) {
            return Some(frame);
        }

        frames.iter().min_by(|a, b| {
            Self::dist_sq_to_rect(x, y, &a.rect).total_cmp(&Self::dist_sq_to_rect(x, y, &b.rect))
        })
    }

    /// Squared distance from the point to the nearest point of the rectangle.
    fn dist_sq_to_rect(x: f32, y: f32, rect: &PhysicalRect) -> f32 {
        let dx = (rect.x - x).max(0.0).max(x - (rect.x + rect.width));
        let dy = (rect.y - y).max(0.0).max(y - (rect.y + rect.height));
        dx * dx + dy * dy
    }

    fn find_best_line(lines: &[LineBox], frame_rel_y: f32) -> Option<(usize, &LineBox)> {
        let first = lines.first()?;
        if frame_rel_y <= first.baseline_y - first.height {
            return Some((0, first));
        }

        let last_idx = lines.len() - 1;
        if frame_rel_y >= lines[last_idx].baseline_y {
            return Some((last_idx, &lines[last_idx]));
        }

        if let Some(hit) = lines.iter().enumerate().find(|(_, line)| {
            frame_rel_y >= line.baseline_y - line.height && frame_rel_y <= line.baseline_y
        }) {
            return Some(hit);
        }

        lines.iter().enumerate().min_by(|(_, a), (_, b)| {
            (a.baseline_y - frame_rel_y)
                .abs()
                .total_cmp(&(b.baseline_y - frame_rel_y).abs())
        })
    }

    fn caret_for(line: &LineBox, idx: usize, frame_rel_x: f32) -> GlyphHit {
        let g = &line.glyphs[idx];
        let mid = g.x + g.width / 2.0;
        let caret_x = if frame_rel_x < mid {
            g.x
        } else {
            g.x + g.width
        };
        let is_leading = if line.is_rtl {
            frame_rel_x >= mid
        } else {
            frame_rel_x <= mid
        };
        (Some(idx), g.cluster, g.character, caret_x, is_leading)
    }

    fn hit_test_line_glyphs(line: &LineBox, frame_rel_x: f32) -> GlyphHit {
        let (Some(first_g), Some(last_g)) = (line.glyphs.first(), line.glyphs.last()) else {
            return (None, 0, None, 0.0, true);
        };

        if frame_rel_x <= first_g.x {
            // Left of the line: the left edge is the leading edge only for LTR.
            return (
                Some(0),
                first_g.cluster,
                first_g.character,
                first_g.x,
                !line.is_rtl,
            );
        }

        let last_idx = line.glyphs.len() - 1;
        if frame_rel_x >= last_g.x + last_g.width {
            return (
                Some(last_idx),
                last_g.cluster,
                last_g.character,
                last_g.x + last_g.width,
                line.is_rtl,
            );
        }

        // Prefer glyphs with an advance: zero-width marks sit on top of their base.
        if let Some(idx) = line
            .glyphs
            .iter()
            .position(|g| g.width > 0.0 && frame_rel_x >= g.x && frame_rel_x <= g.x + g.width)
        {
            return Self::caret_for(line, idx, frame_rel_x);
        }

        let idx = line
            .glyphs
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let ca = a.x + a.width / 2.0;
                let cb = b.x + b.width / 2.0;
                (ca - frame_rel_x)
                    .abs()
                    .total_cmp(&(cb - frame_rel_x).abs())
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        Self::caret_for(line, idx, frame_rel_x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::GlyphBox;

    fn glyph(id: u32, cluster: u32, x: f32, width: f32, ch: char) -> GlyphBox {
        GlyphBox {
            glyph_id: id,
            cluster,
            x,
            y: 0.0,
            width,
            height: 12.0,
            character: Some(ch),
            font_index: 0,
        }
    }

    fn line(index: usize, baseline: f32, glyphs: Vec<GlyphBox>, text: &str) -> LineBox {
        LineBox {
            line_index: index,
            paragraph_id: Some(NodeId::new()),
            baseline_y: baseline,
            height: 14.0,
            width: 30.0,
            glyphs,
            text: text.to_string(),
            is_rtl: true,
            fonts: Vec::new(),
            is_drop_cap: false,
            x_offset: 0.0,
        }
    }

    fn create_test_page() -> PageLayoutBox {
        let line0 = line(
            0,
            14.0,
            vec![
                glyph(1, 0, 0.0, 10.0, 'א'),
                glyph(2, 1, 10.0, 10.0, 'ב'),
                glyph(3, 2, 20.0, 10.0, 'ג'),
            ],
            "אבג",
        );
        let line1 = line(
            1,
            28.0,
            vec![glyph(4, 0, 0.0, 15.0, 'ד'), glyph(5, 1, 15.0, 15.0, 'ה')],
            "דה",
        );

        let frame = TextFrameBox {
            frame_id: "frame_1".to_string(),
            flow_id: "main".to_string(),
            rect: PhysicalRect::new(50.0, 50.0, 200.0, 400.0),
            lines: vec![line0, line1],
        };

        PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![frame],
            break_token: None,
            is_blank: false,
        }
    }

    #[test]
    fn test_hit_test_precise_glyph_detection() {
        let page = create_test_page();

        // Hit near first glyph of line 0 (x: 50.0 + 3.0 = 53.0, y: 50.0 + 7.0 = 57.0)
        let hit = HitTester::hit_test(&page, 53.0, 57.0).expect("Should hit glyph");
        assert_eq!(hit.line_index, 0);
        assert_eq!(hit.glyph_index, Some(0));
        assert_eq!(hit.character, Some('א'));
        assert_eq!(hit.visual_x_pt, 50.0); // snapped to left edge of glyph (frame_x + g.x)
        assert_eq!(hit.visual_y_pt, 50.0); // frame_y + baseline(14) - height(14) = 50.0

        // Hit second glyph on trailing edge
        let hit2 = HitTester::hit_test(&page, 50.0 + 18.0, 57.0).expect("Should hit glyph");
        assert_eq!(hit2.glyph_index, Some(1));
        assert_eq!(hit2.character, Some('ב'));
        assert_eq!(hit2.visual_x_pt, 70.0); // snapped to right edge of glyph (50 + 20)
    }

    #[test]
    fn test_hit_test_line_selection_range() {
        let page = create_test_page();
        let start = PhysicalPoint { x: 55.0, y: 55.0 };
        let end = PhysicalPoint { x: 75.0, y: 55.0 };

        let rects = HitTester::hit_test_range(&page, start, end);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].width >= 20.0);
        assert_eq!(rects[0].height, 14.0);
    }

    #[test]
    fn empty_page_and_frame_and_nan_points() {
        let mut page = create_test_page();
        assert!(HitTester::hit_test(&page, f32::NAN, 10.0).is_none());
        page.frames[0].lines.clear();
        let hit = HitTester::hit_test(&page, 60.0, 60.0).unwrap();
        assert_eq!(hit.glyph_index, None);
        page.frames.clear();
        assert!(HitTester::hit_test(&page, 60.0, 60.0).is_none());
    }

    /// Regression: a right-aligned RTL line was selected from the frame's left
    /// edge instead of from where its glyphs actually start.
    #[test]
    fn multi_line_rtl_selection_uses_line_extents() {
        let mut page = create_test_page();
        // Shift line 1 to the right, like a short last line of an RTL paragraph.
        for g in &mut page.frames[0].lines[1].glyphs {
            g.x += 100.0;
        }
        let rects = HitTester::hit_test_range(
            &page,
            PhysicalPoint { x: 65.0, y: 55.0 },
            PhysicalPoint { x: 170.0, y: 70.0 },
        );
        assert_eq!(rects.len(), 2);
        // First line (RTL): from its left extent (frame x) to the hit.
        assert_eq!(rects[0].x, 50.0);
        // Last line (RTL): from the hit to the right extent of its glyphs.
        let right = rects[1].x + rects[1].width;
        assert!((right - (50.0 + 130.0)).abs() < 1e-3, "right = {right}");
    }

    #[test]
    fn closest_frame_is_measured_to_its_edge() {
        let mut page = create_test_page();
        let mut small = page.frames[0].clone();
        small.frame_id = "small".to_string();
        small.rect = PhysicalRect::new(300.0, 440.0, 10.0, 10.0);
        page.frames.push(small);
        // Just right of the big frame's edge: the big frame is nearer by edge
        // distance even though the small frame's center is closer.
        let hit = HitTester::hit_test(&page, 255.0, 440.0).unwrap();
        assert_eq!(hit.frame_id, "frame_1");
    }
}

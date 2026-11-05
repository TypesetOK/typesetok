//! Variable-Width Polygon Engine for Non-Rectangular Text Frame Wrapping.
//!
//! Maps non-rectangular geometric frames (such as L-shaped commentary columns,
//! absorbed frames, and polygon exclusion zones) to Knuth-Plass measure profiles,
//! ensuring seamless multi-paragraph flow across expanding or narrowing boundaries.

use crate::geometry::{LineBox, PhysicalRect};
use crate::knuth_plass::MeasureProfile;
use crate::polygon::{ExclusionZone, GeometricPolygon, ScanlineSpan};
use serde::{Deserialize, Serialize};

/// A non-rectangular or multi-segment text frame capable of receiving text flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolygonFrame {
    /// Distinct contiguous rectangular bands composing the frame (ordered vertically).
    pub bands: Vec<PhysicalRect>,
    /// Optional arbitrary polygonal boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<GeometricPolygon>,
    /// Active exclusion zones inside or intersecting this frame.
    #[serde(default)]
    pub exclusions: Vec<ExclusionZone>,
}

impl PolygonFrame {
    /// Creates a polygon frame from a single rectangular boundary.
    pub fn from_rect(rect: PhysicalRect) -> Self {
        Self {
            bands: vec![rect],
            polygon: Some(GeometricPolygon::from_rect(&rect)),
            exclusions: Vec::new(),
        }
    }

    /// Creates an L-shaped composite frame consisting of a narrow upper column
    /// and a wide lower absorption band.
    pub fn l_shaped(
        upper_x: f32,
        upper_y: f32,
        upper_width: f32,
        upper_height: f32,
        lower_x: f32,
        lower_width: f32,
        lower_height: f32,
    ) -> Self {
        let upper_rect = PhysicalRect::new(upper_x, upper_y, upper_width, upper_height);
        let lower_y = upper_y + upper_height;
        let lower_rect = PhysicalRect::new(lower_x, lower_y, lower_width, lower_height);

        Self {
            bands: vec![upper_rect, lower_rect],
            polygon: None,
            exclusions: Vec::new(),
        }
    }

    /// Creates a polygon frame from an arbitrary GeometricPolygon.
    pub fn from_polygon(poly: GeometricPolygon) -> Self {
        Self {
            bands: Vec::new(),
            polygon: Some(poly),
            exclusions: Vec::new(),
        }
    }

    /// Creates a stepped polygon frame from an ordered sequence of vertical bands.
    pub fn stepped(bands: Vec<PhysicalRect>) -> Self {
        Self {
            bands,
            polygon: None,
            exclusions: Vec::new(),
        }
    }

    /// Adds an exclusion zone to the frame.
    pub fn with_exclusion(mut self, exclusion: ExclusionZone) -> Self {
        self.exclusions.push(exclusion);
        self
    }

    /// Adds multiple exclusion zones to the frame.
    pub fn with_exclusions(mut self, exclusions: Vec<ExclusionZone>) -> Self {
        self.exclusions.extend(exclusions);
        self
    }

    /// Total bounding rectangle enclosing all bands and polygon vertices.
    pub fn total_bounds(&self) -> PhysicalRect {
        if let Some(ref poly) = self.polygon {
            let mut pb = poly.bounds();
            for b in &self.bands {
                let min_x = pb.x.min(b.x);
                let min_y = pb.y.min(b.y);
                let max_x = (pb.x + pb.width).max(b.x + b.width);
                let max_y = (pb.y + pb.height).max(b.y + b.height);
                pb = PhysicalRect::new(min_x, min_y, max_x - min_x, max_y - min_y);
            }
            pb
        } else if !self.bands.is_empty() {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for b in &self.bands {
                min_x = min_x.min(b.x);
                min_y = min_y.min(b.y);
                max_x = max_x.max(b.x + b.width);
                max_y = max_y.max(b.y + b.height);
            }
            PhysicalRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
        } else {
            PhysicalRect::new(0.0, 0.0, 0.0, 0.0)
        }
    }

    /// Computes the usable horizontal span at physical vertical coordinate `y` with given `height`.
    pub fn span_at_y(&self, y: f32, height: f32) -> Option<ScanlineSpan> {
        let eps = (height * 0.02).clamp(1e-3, 0.25);
        let band_top = y + eps;
        let band_bot = (y + height - eps).max(band_top);

        let mut base_span: Option<ScanlineSpan> = None;

        // 1. If bands are explicitly defined, intersect all bands overlapping [band_top, band_bot]
        // so that a line straddling a step transition never overflows the narrower band.
        if !self.bands.is_empty() {
            let mut inter_left = f32::NEG_INFINITY;
            let mut inter_right = f32::INFINITY;
            let mut covered_top = false;
            let mut covered_bot = false;

            for b in &self.bands {
                let b_top = b.y;
                let b_bot = b.y + b.height;
                if band_bot > b_top && band_top < b_bot {
                    inter_left = inter_left.max(b.x);
                    inter_right = inter_right.min(b.x + b.width);
                    if b_top <= band_top + eps {
                        covered_top = true;
                    }
                    if b_bot >= band_bot - eps {
                        covered_bot = true;
                    }
                }
            }

            if covered_top && covered_bot && inter_right > inter_left + 1e-3 {
                base_span = Some(ScanlineSpan::new(inter_left, inter_right - inter_left));
            }
        }

        // 2. Otherwise fall back to polygon scanline slicing
        if base_span.is_none() {
            if let Some(ref poly) = self.polygon {
                let spans = poly.scanline_spans(y, height);
                base_span = spans.into_iter().max_by(|a, b| a.width.total_cmp(&b.width));
            }
        }

        let mut span = base_span?;

        // 3. Subtract intersecting exclusions
        for ex in &self.exclusions {
            if ex.intersects_y(y, height) {
                let exp = ex.expanded_rect();
                let remaining = span.subtract_range(exp.x, exp.x + exp.width);
                span = remaining
                    .into_iter()
                    .max_by(|a, b| a.width.total_cmp(&b.width))?;
            }
        }

        if span.width > 1.0 {
            Some(span)
        } else {
            None
        }
    }

    /// Computes total capacity of lines fitting inside this frame.
    pub fn capacity_lines(&self, line_height: f32) -> usize {
        if line_height <= 0.0 {
            return 0;
        }
        let bounds = self.total_bounds();
        let num_bands = (bounds.height / line_height).ceil() as usize;

        let mut count = 0;
        for i in 0..num_bands {
            let y = bounds.y + (i as f32) * line_height;
            if self.span_at_y(y, line_height).is_some() {
                count += 1;
            }
        }
        count
    }

    /// Synthesizes a variable [`MeasureProfile`] for a paragraph beginning at `start_line`
    /// within this polygon frame.
    pub fn profile_for_paragraph(
        &self,
        line_height: f32,
        start_line: usize,
        max_lines: usize,
    ) -> MeasureProfile {
        let bounds = self.total_bounds();
        let mut measures = Vec::with_capacity(max_lines);

        for i in 0..max_lines {
            let line_idx = start_line + i;
            let y = bounds.y + (line_idx as f32) * line_height;

            if let Some(span) = self.span_at_y(y, line_height) {
                measures.push((span.x, span.width));
            } else {
                // Beyond frame capacity: repeat last valid span or use the frame's actual bounding width
                if let Some(&last) = measures.last() {
                    measures.push(last);
                } else if bounds.width > 0.0 {
                    measures.push((bounds.x, bounds.width));
                }
            }
        }

        if measures.is_empty() {
            MeasureProfile::uniform(bounds.width)
        } else {
            MeasureProfile::from_measures(&measures, true)
        }
    }

    /// Positions a set of formatted paragraph lines within the frame starting at `start_line`.
    pub fn position_paragraph_lines(
        &self,
        lines: &mut [LineBox],
        start_line: usize,
        line_height: f32,
    ) {
        let bounds = self.total_bounds();
        for (i, line) in lines.iter_mut().enumerate() {
            let abs_line = start_line + i;
            let y_top = bounds.y + (abs_line as f32) * line_height;
            line.baseline_y = y_top + line_height;

            if let Some(span) = self.span_at_y(y_top, line_height) {
                line.x_offset = span.x;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_frame_rectangular_capacity_and_profile() {
        let frame = PolygonFrame::from_rect(PhysicalRect::new(50.0, 100.0, 200.0, 300.0));
        assert_eq!(frame.capacity_lines(15.0), 20);

        let profile = frame.profile_for_paragraph(15.0, 0, 5);
        assert_eq!(profile.width_for_line(0), 200.0);
        assert_eq!(profile.offset_for_line(0), 50.0);
    }

    #[test]
    fn test_polygon_frame_l_shaped_commentary_absorption() {
        // Commentary frame with upper narrow column (120pt wide, 150pt tall, x=0)
        // and lower expanded absorption band (300pt wide, 150pt tall, x=0)
        let frame = PolygonFrame::l_shaped(0.0, 0.0, 120.0, 150.0, 0.0, 300.0, 150.0);

        // 10 lines of 15pt in upper band (150pt), 10 lines in lower band (150pt)
        assert_eq!(frame.capacity_lines(15.0), 20);

        // Paragraph 1 starts at line 0: spans 10 narrow lines, then expands to 300pt
        let prof1 = frame.profile_for_paragraph(15.0, 0, 15);
        for l in 0..10 {
            assert_eq!(prof1.width_for_line(l), 120.0);
        }
        for l in 10..15 {
            assert_eq!(prof1.width_for_line(l), 300.0);
        }

        // Paragraph 2 starts at line 8 (after paragraph 1 took 8 lines):
        // line 0 and 1 of paragraph 2 are narrow (120pt), then expands to 300pt from line 2 onwards!
        let prof2 = frame.profile_for_paragraph(15.0, 8, 10);
        assert_eq!(prof2.width_for_line(0), 120.0);
        assert_eq!(prof2.width_for_line(1), 120.0);
        assert_eq!(prof2.width_for_line(2), 300.0);
        assert_eq!(prof2.width_for_line(3), 300.0);
    }

    #[test]
    fn test_polygon_frame_with_exclusions_notching() {
        let frame =
            PolygonFrame::from_rect(PhysicalRect::new(0.0, 0.0, 400.0, 500.0)).with_exclusion(
                ExclusionZone::new(PhysicalRect::new(300.0, 0.0, 100.0, 45.0), 5.0),
            );

        let prof = frame.profile_for_paragraph(15.0, 0, 6);
        // Lines 0, 1, 2 intersect the exclusion at x >= 295
        assert!((prof.width_for_line(0) - 295.0).abs() < 1e-2);
        assert!((prof.width_for_line(1) - 295.0).abs() < 1e-2);
        assert!((prof.width_for_line(2) - 295.0).abs() < 1e-2);
        // Line 4+ is full 400 (starts at y=60, past exclusion bottom at 50)
        assert_eq!(prof.width_for_line(4), 400.0);
    }
}

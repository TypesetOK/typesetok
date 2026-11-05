//! Geometric Polygon and Exclusion Zone Engine.
//!
//! Provides polygon representation, scanline slicing, and exclusion zone subtractions
//! for non-rectangular text frames and drop cap windows ("חלון").

use crate::geometry::{PhysicalPoint, PhysicalRect};
use serde::{Deserialize, Serialize};

/// A single continuous horizontal slice available for text on a line.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScanlineSpan {
    /// Leftmost x-coordinate of the span (in pt).
    pub x: f32,
    /// Usable width of the span (in pt).
    pub width: f32,
}

impl ScanlineSpan {
    pub fn new(x: f32, width: f32) -> Self {
        Self {
            x,
            width: width.max(0.0),
        }
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Subtracts a horizontal exclusion range `[ex_left, ex_right]` from this span.
    /// Returns 0, 1, or 2 remaining spans.
    pub fn subtract_range(&self, ex_left: f32, ex_right: f32) -> Vec<ScanlineSpan> {
        if ex_right <= self.x || ex_left >= self.right() {
            // No overlap
            return vec![*self];
        }

        let mut out = Vec::with_capacity(2);
        // Left remainder
        if ex_left > self.x {
            out.push(ScanlineSpan::new(self.x, ex_left - self.x));
        }
        // Right remainder
        if ex_right < self.right() {
            out.push(ScanlineSpan::new(ex_right, self.right() - ex_right));
        }
        out
    }
}

/// A geometric exclusion zone (e.g. drop cap window, embedded image, callout).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExclusionZone {
    pub rect: PhysicalRect,
    /// Margin / clearance (pt) applied around the exclusion zone.
    pub margin_pt: f32,
}

impl ExclusionZone {
    pub fn new(rect: PhysicalRect, margin_pt: f32) -> Self {
        Self {
            rect,
            margin_pt: margin_pt.max(0.0),
        }
    }

    /// Bounding rectangle including margin.
    pub fn expanded_rect(&self) -> PhysicalRect {
        PhysicalRect::new(
            self.rect.x - self.margin_pt,
            self.rect.y - self.margin_pt,
            self.rect.width + 2.0 * self.margin_pt,
            self.rect.height + 2.0 * self.margin_pt,
        )
    }

    /// Checks if this exclusion intersects the vertical band `[y, y + height]`.
    pub fn intersects_y(&self, y: f32, height: f32) -> bool {
        let exp = self.expanded_rect();
        let top = exp.y;
        let bottom = exp.y + exp.height;
        let band_top = y;
        let band_bottom = y + height;
        band_top < bottom && band_bottom > top
    }
}

/// A 2D polygon defining a text frame boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometricPolygon {
    /// Ordered boundary vertices (clockwise or counter-clockwise).
    pub vertices: Vec<PhysicalPoint>,
}

impl GeometricPolygon {
    pub fn new(vertices: Vec<PhysicalPoint>) -> Self {
        Self { vertices }
    }

    /// Creates an axis-aligned rectangular polygon.
    pub fn from_rect(rect: &PhysicalRect) -> Self {
        Self {
            vertices: vec![
                PhysicalPoint {
                    x: rect.x,
                    y: rect.y,
                },
                PhysicalPoint {
                    x: rect.x + rect.width,
                    y: rect.y,
                },
                PhysicalPoint {
                    x: rect.x + rect.width,
                    y: rect.y + rect.height,
                },
                PhysicalPoint {
                    x: rect.x,
                    y: rect.y + rect.height,
                },
            ],
        }
    }

    /// Creates an L-shaped polygon:
    /// - Narrow upper column: `[x, y]` with `narrow_width` and `narrow_height`.
    /// - Expanded lower band: `[x, y + narrow_height]` with `wide_width` and `expanded_height`.
    pub fn l_shape(
        x: f32,
        y: f32,
        narrow_width: f32,
        narrow_height: f32,
        wide_width: f32,
        expanded_height: f32,
    ) -> Self {
        Self {
            vertices: vec![
                PhysicalPoint { x, y },
                PhysicalPoint {
                    x: x + narrow_width,
                    y,
                },
                PhysicalPoint {
                    x: x + narrow_width,
                    y: y + narrow_height,
                },
                PhysicalPoint {
                    x: x + wide_width,
                    y: y + narrow_height,
                },
                PhysicalPoint {
                    x: x + wide_width,
                    y: y + narrow_height + expanded_height,
                },
                PhysicalPoint {
                    x,
                    y: y + narrow_height + expanded_height,
                },
            ],
        }
    }

    /// Computes bounding box enclosing the polygon.
    pub fn bounds(&self) -> PhysicalRect {
        if self.vertices.is_empty() {
            return PhysicalRect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for v in &self.vertices {
            if v.x < min_x {
                min_x = v.x;
            }
            if v.x > max_x {
                max_x = v.x;
            }
            if v.y < min_y {
                min_y = v.y;
            }
            if v.y > max_y {
                max_y = v.y;
            }
        }
        PhysicalRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Alias for [`Self::bounds`].
    #[inline]
    pub fn bounding_box(&self) -> PhysicalRect {
        self.bounds()
    }

    /// Creates a trapezoid polygon with top horizontal edge `[top_x, top_x + top_width]` at `y`
    /// and bottom horizontal edge `[bottom_x, bottom_x + bottom_width]` at `y + height`.
    pub fn trapezoid(
        top_x: f32,
        top_width: f32,
        bottom_x: f32,
        bottom_width: f32,
        y: f32,
        height: f32,
    ) -> Self {
        Self {
            vertices: vec![
                PhysicalPoint { x: top_x, y },
                PhysicalPoint {
                    x: top_x + top_width,
                    y,
                },
                PhysicalPoint {
                    x: bottom_x + bottom_width,
                    y: y + height,
                },
                PhysicalPoint {
                    x: bottom_x,
                    y: y + height,
                },
            ],
        }
    }

    /// Computes 1D horizontal scanline spans at a single vertical coordinate `sample_y`.
    pub fn scanline_spans_at_y(&self, sample_y: f32) -> Vec<ScanlineSpan> {
        let n = self.vertices.len();
        if n < 3 {
            return Vec::new();
        }

        let mut x_inters = Vec::new();
        for i in 0..n {
            let p1 = self.vertices[i];
            let p2 = self.vertices[(i + 1) % n];

            let y_min = p1.y.min(p2.y);
            let y_max = p1.y.max(p2.y);

            if sample_y >= y_min && sample_y < y_max {
                let dy = p2.y - p1.y;
                if dy.abs() > 1e-5 {
                    let t = (sample_y - p1.y) / dy;
                    let inter_x = p1.x + t * (p2.x - p1.x);
                    x_inters.push(inter_x);
                }
            }
        }

        x_inters.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut spans = Vec::new();
        for &[left, right] in x_inters.as_chunks::<2>().0 {
            if right > left + 1e-3 {
                spans.push(ScanlineSpan::new(left, right - left));
            }
        }
        spans
    }

    /// Intersects two sets of sorted horizontal spans, returning the spans contained in both.
    fn intersect_span_sets(a: &[ScanlineSpan], b: &[ScanlineSpan]) -> Vec<ScanlineSpan> {
        let mut out = Vec::new();
        for sa in a {
            let a_left = sa.x;
            let a_right = sa.right();
            for sb in b {
                let left = a_left.max(sb.x);
                let right = a_right.min(sb.right());
                if right > left + 1e-3 {
                    out.push(ScanlineSpan::new(left, right - left));
                }
            }
        }
        out
    }

    /// Slices the polygon across the horizontal band `[y, y + height]`.
    ///
    /// Performs conservative band intersection across the top, midpoint, bottom, and any
    /// intermediate polygon vertex Y-levels inside `(y, y + height)` so sloped edges and
    /// stepped corners never overestimate usable line width.
    pub fn scanline_spans(&self, y: f32, height: f32) -> Vec<ScanlineSpan> {
        let n = self.vertices.len();
        if n < 3 {
            return Vec::new();
        }
        if height <= 1e-3 {
            return self.scanline_spans_at_y(y);
        }

        let eps = (height * 0.02).clamp(1e-3, 0.25);
        let y_top = y + eps;
        let y_mid = y + height * 0.5;
        let y_bot = (y + height - eps).max(y_top);

        let mut sample_ys = vec![y_top, y_mid, y_bot];
        for v in &self.vertices {
            if v.y > y_top + eps && v.y < y_bot - eps {
                sample_ys.push(v.y - eps * 0.5);
                sample_ys.push(v.y + eps * 0.5);
            }
        }

        let mut current: Option<Vec<ScanlineSpan>> = None;
        for sy in sample_ys {
            let spans = self.scanline_spans_at_y(sy);
            if spans.is_empty() {
                return Vec::new();
            }
            current = Some(match current {
                None => spans,
                Some(prev) => Self::intersect_span_sets(&prev, &spans),
            });
            if current.as_ref().is_some_and(|c| c.is_empty()) {
                return Vec::new();
            }
        }

        current.unwrap_or_default()
    }

    /// Slices the polygon with a horizontal band at `y`, subtracting all overlapping exclusion zones.
    pub fn scanline_spans_with_exclusions(
        &self,
        y: f32,
        height: f32,
        exclusions: &[ExclusionZone],
    ) -> Vec<ScanlineSpan> {
        let base_spans = self.scanline_spans(y, height);
        let mut current_spans = base_spans;

        for ex in exclusions {
            if !ex.intersects_y(y, height) {
                continue;
            }
            let exp = ex.expanded_rect();
            let ex_left = exp.x;
            let ex_right = exp.x + exp.width;

            let mut next_spans = Vec::new();
            for span in current_spans {
                next_spans.extend(span.subtract_range(ex_left, ex_right));
            }
            current_spans = next_spans;
        }

        current_spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanline_span_subtraction() {
        let span = ScanlineSpan::new(10.0, 100.0); // [10, 110]

        // Outside exclusion (no effect)
        let rem1 = span.subtract_range(120.0, 150.0);
        assert_eq!(rem1.len(), 1);
        assert_eq!(rem1[0], span);

        // Right-side exclusion (drop cap in RTL)
        let rem2 = span.subtract_range(80.0, 120.0);
        assert_eq!(rem2.len(), 1);
        assert_eq!(rem2[0], ScanlineSpan::new(10.0, 70.0));

        // Left-side exclusion (drop cap in LTR)
        let rem3 = span.subtract_range(0.0, 40.0);
        assert_eq!(rem3.len(), 1);
        assert_eq!(rem3[0], ScanlineSpan::new(40.0, 70.0));

        // Middle exclusion (split into 2)
        let rem4 = span.subtract_range(40.0, 70.0);
        assert_eq!(rem4.len(), 2);
        assert_eq!(rem4[0], ScanlineSpan::new(10.0, 30.0));
        assert_eq!(rem4[1], ScanlineSpan::new(70.0, 40.0));
    }

    #[test]
    fn test_rectangular_polygon_scanline() {
        let rect = PhysicalRect::new(50.0, 100.0, 200.0, 400.0);
        let poly = GeometricPolygon::from_rect(&rect);

        assert_eq!(poly.bounds(), rect);

        // Within rect
        let spans = poly.scanline_spans(150.0, 15.0);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].x - 50.0).abs() < 1e-3);
        assert!((spans[0].width - 200.0).abs() < 1e-3);

        // Outside rect
        let outside = poly.scanline_spans(50.0, 15.0);
        assert!(outside.is_empty());
    }

    #[test]
    fn test_l_shaped_polygon_scanline_expansion() {
        // L-shape: upper narrow (100pt wide, 200pt tall), lower wide (300pt wide, 200pt tall)
        let l_poly = GeometricPolygon::l_shape(0.0, 0.0, 100.0, 200.0, 300.0, 200.0);

        // Upper section (y = 50)
        let upper_spans = l_poly.scanline_spans(50.0, 14.0);
        assert_eq!(upper_spans.len(), 1);
        assert!((upper_spans[0].width - 100.0).abs() < 1e-3);

        // Lower expanded section (y = 250)
        let lower_spans = l_poly.scanline_spans(250.0, 14.0);
        assert_eq!(lower_spans.len(), 1);
        assert!((lower_spans[0].width - 300.0).abs() < 1e-3);
    }

    #[test]
    fn test_exclusion_zone_drop_cap_notch() {
        let poly = GeometricPolygon::from_rect(&PhysicalRect::new(0.0, 0.0, 400.0, 600.0));
        // Drop cap window in top-right corner of RTL text: x: 320 to 400, y: 0 to 45 (3 lines of 15pt)
        let ex = ExclusionZone::new(PhysicalRect::new(320.0, 0.0, 80.0, 45.0), 5.0);

        // Line 1: y = 0 to 15 (intersecting drop cap exclusion)
        let line1_spans = poly.scanline_spans_with_exclusions(0.0, 15.0, std::slice::from_ref(&ex));
        assert_eq!(line1_spans.len(), 1);
        // Available width should be 320 - 5.0 = 315.0
        assert!((line1_spans[0].width - 315.0).abs() < 1e-2);

        // Line 4: y = 60 to 75 (below drop cap window + margin)
        let line4_spans =
            poly.scanline_spans_with_exclusions(60.0, 15.0, std::slice::from_ref(&ex));
        assert_eq!(line4_spans.len(), 1);
        assert!((line4_spans[0].width - 400.0).abs() < 1e-2);
    }

    #[test]
    fn test_trapezoid_and_stepped_band_intersection_never_overestimates() {
        // Trapezoid narrowing from width 200 at y=0 to width 100 at y=100 (left x=0, right x(y) = 200 - y)
        let trap = GeometricPolygon::trapezoid(0.0, 200.0, 0.0, 100.0, 0.0, 100.0);

        // Band [0, 20]: at y=0 width is 200, at y=10 (midpoint) width is 190, at y=20 width is 180.
        // Conservative band intersection must return width <= 180.5 so the bottom of the line does not overflow!
        let spans = trap.scanline_spans(0.0, 20.0);
        assert_eq!(spans.len(), 1);
        assert!(
            spans[0].width <= 180.5 && spans[0].width >= 179.5,
            "Trapezoid band [0, 20] width should be bounded by narrower bottom edge (~180pt), got {}",
            spans[0].width
        );

        // L-shape with step at y=50: band [40, 60] straddles the step (y=40..50 has width 100, y=50..60 has width 300).
        // Conservative band intersection must restrict straddling line [40, 60] to 100pt so top half does not collide!
        let l_poly = GeometricPolygon::l_shape(0.0, 0.0, 100.0, 50.0, 300.0, 50.0);
        let straddle_spans = l_poly.scanline_spans(40.0, 20.0);
        assert_eq!(straddle_spans.len(), 1);
        assert!(
            (straddle_spans[0].width - 100.0).abs() < 1e-2,
            "Line straddling L-shape step must be restricted to narrow width (100pt), got {}",
            straddle_spans[0].width
        );
    }
}

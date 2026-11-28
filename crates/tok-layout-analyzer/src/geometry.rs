//! Normalized geometric primitives for page layout analysis.
//!
//! All coordinates are normalized to `[0.0, 1.0]` relative to the net content
//! bounding box of a page (with origin `(0, 0)` at top-left).

use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in normalized `[0.0, 1.0]` page coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalizedBBox {
    /// Left edge X in `[0.0, 1.0]`.
    pub x0: f32,
    /// Top edge Y in `[0.0, 1.0]`.
    pub y0: f32,
    /// Right edge X in `[0.0, 1.0]`.
    pub x1: f32,
    /// Bottom edge Y in `[0.0, 1.0]`.
    pub y1: f32,
}

impl NormalizedBBox {
    /// Constructs a clamped, well-ordered bounding box.
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        let nx0 = x0.min(x1).clamp(0.0, 1.0);
        let ny0 = y0.min(y1).clamp(0.0, 1.0);
        let nx1 = x0.max(x1).clamp(0.0, 1.0);
        let ny1 = y0.max(y1).clamp(0.0, 1.0);
        Self {
            x0: nx0,
            y0: ny0,
            x1: nx1,
            y1: ny1,
        }
    }

    /// Width in normalized units.
    #[inline]
    pub fn width(&self) -> f32 {
        (self.x1 - self.x0).max(0.0)
    }

    /// Height in normalized units.
    #[inline]
    pub fn height(&self) -> f32 {
        (self.y1 - self.y0).max(0.0)
    }

    /// Area in normalized units squared.
    #[inline]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Horizontal center X.
    #[inline]
    pub fn center_x(&self) -> f32 {
        (self.x0 + self.x1) * 0.5
    }

    /// Vertical center Y.
    #[inline]
    pub fn center_y(&self) -> f32 {
        (self.y0 + self.y1) * 0.5
    }

    /// Horizontal overlap fraction relative to the smaller width.
    pub fn horizontal_overlap_ratio(&self, other: &Self) -> f32 {
        let ix0 = self.x0.max(other.x0);
        let ix1 = self.x1.min(other.x1);
        let inter = (ix1 - ix0).max(0.0);
        let min_w = self.width().min(other.width());
        if min_w <= f32::EPSILON {
            0.0
        } else {
            inter / min_w
        }
    }

    /// Vertical overlap fraction relative to the smaller height.
    pub fn vertical_overlap_ratio(&self, other: &Self) -> f32 {
        let iy0 = self.y0.max(other.y0);
        let iy1 = self.y1.min(other.y1);
        let inter = (iy1 - iy0).max(0.0);
        let min_h = self.height().min(other.height());
        if min_h <= f32::EPSILON {
            0.0
        } else {
            inter / min_h
        }
    }

    /// Intersection-over-Union (IoU) with another box.
    pub fn iou(&self, other: &Self) -> f32 {
        let ix0 = self.x0.max(other.x0);
        let iy0 = self.y0.max(other.y0);
        let ix1 = self.x1.min(other.x1);
        let iy1 = self.y1.min(other.y1);
        let inter = (ix1 - ix0).max(0.0) * (iy1 - iy0).max(0.0);
        let union = self.area() + other.area() - inter;
        if union <= f32::EPSILON {
            0.0
        } else {
            inter / union
        }
    }

    /// Returns the union bounding box enclosing both `self` and `other`.
    pub fn union(&self, other: &Self) -> Self {
        Self::new(
            self.x0.min(other.x0),
            self.y0.min(other.y0),
            self.x1.max(other.x1),
            self.y1.max(other.y1),
        )
    }

    /// Returns `true` if `self` is positioned below `other` and horizontally crosses into `other`'s X-span
    /// (key geometric primitive for L-shape commentary expansion below Gemara).
    pub fn wraps_under(&self, central: &Self, min_cross_ratio: f32) -> bool {
        self.y0 >= central.y1 - 0.02 && self.horizontal_overlap_ratio(central) >= min_cross_ratio
    }
}

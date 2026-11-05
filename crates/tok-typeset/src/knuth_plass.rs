//! Knuth-Plass Global Paragraph Line Breaking Algorithm.
//!
//! Replaces greedy line-breaking with dynamic programming to find optimal
//! breakpoints across the whole paragraph, eliminating rivers of white.
//!
//! Semantics follow TeX:
//! - A line may break at a glue that follows a box, or at a penalty below
//!   [`KnuthPlassBreaker::INFINITY_PENALTY`]. A penalty at or below
//!   [`KnuthPlassBreaker::FORCED_BREAK_PENALTY`] must be broken at.
//! - The breaking glue is dropped; a penalty's width (e.g. a hyphen) only
//!   counts on the line that breaks at it.
//! - Glue and non-forced penalties at the start of a line after a break are
//!   discarded.
//! - The last line behaves as if it ended with infinitely stretchable fill
//!   glue: it may be short without penalty.
//! - If no break sequence satisfies the tolerance, a second "emergency" pass
//!   accepts any stretch and, as a last resort, overfull lines (e.g. a single
//!   word wider than the measure), minimising the overflow. A paragraph with
//!   valid metrics therefore always gets a layout.

use crate::shaper::PositionedGlyph;

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutItem {
    Box {
        width: f32,
        text: String,
        glyphs: Vec<PositionedGlyph>,
    },
    Glue {
        width: f32,
        stretch: f32,
        shrink: f32,
    },
    Penalty {
        width: f32,
        penalty: f32,
        flagged: bool,
    },
}

/// A line chosen by the breaker, expressed as a range of the input items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSpan {
    /// Index of the first item on the line.
    pub start: usize,
    /// One past the last item on the line. A penalty the line breaks at is
    /// included (its width counts); a breaking glue is not.
    pub end: usize,
    /// Natural width of the line (boxes, inner glue, break penalty width).
    pub natural_width: f32,
    /// Target width assigned to this line.
    pub target_width: f32,
    /// Horizontal start offset of this line (in pt).
    pub x_offset: f32,
    /// r: < 0 is shrink, > 0 is stretch; 0 when the ratio is undefined
    /// (no stretch/shrink available) and on a short last line.
    pub adjustment_ratio: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrokenLine {
    pub line_number: usize,
    pub items: Vec<LayoutItem>,
    pub width: f32,
    pub target_width: f32,
    pub x_offset: f32,
    pub adjustment_ratio: f32,
}

/// Constraints governing the final line of a multi-line paragraph in Knuth-Plass breaking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LastLineConstraint {
    /// Minimum natural width (in points) required for the last line of a multi-line paragraph.
    pub min_width_pt: f32,
    /// Minimum natural width as a ratio `[0.0, 1.0]` of the last line's target measure.
    pub min_width_ratio: f32,
    /// Minimum number of words (boxes) required on the last line of a multi-line paragraph.
    pub min_words: usize,
    /// Whether to penalize excessive slack on the last line to balance line lengths.
    pub balance_slack: bool,
    /// Maximum natural width as a ratio `(0.0, 1.0]` of the last line's target measure.
    pub max_width_ratio: f32,
}

impl Default for LastLineConstraint {
    fn default() -> Self {
        Self::none()
    }
}

impl LastLineConstraint {
    /// Unconstrained last line (classic TeX `\parfillskip = 0pt plus 1fil`).
    pub const fn none() -> Self {
        Self {
            min_width_pt: 0.0,
            min_width_ratio: 0.0,
            min_words: 1,
            balance_slack: false,
            max_width_ratio: 1.0,
        }
    }

    /// Requires the final line of a multi-line paragraph to have at least `min_width_pt` points.
    pub const fn min_width(min_width_pt: f32) -> Self {
        Self {
            min_width_pt,
            min_width_ratio: 0.0,
            min_words: 1,
            balance_slack: false,
            max_width_ratio: 1.0,
        }
    }

    /// Requires the final line of a multi-line paragraph to fill at least `min_width_ratio` of its measure.
    pub const fn min_ratio(min_width_ratio: f32) -> Self {
        Self {
            min_width_pt: 0.0,
            min_width_ratio,
            min_words: 1,
            balance_slack: false,
            max_width_ratio: 1.0,
        }
    }

    /// Prevents single-word runts and enforces a minimum width ratio on the final line.
    pub const fn no_runt(min_words: usize, min_width_ratio: f32) -> Self {
        Self {
            min_width_pt: 0.0,
            min_width_ratio,
            min_words,
            balance_slack: false,
            max_width_ratio: 1.0,
        }
    }

    /// Enforces balanced last-line placement with minimum fill ratio, multi-word ending, and slack balancing.
    pub const fn balanced(min_width_ratio: f32) -> Self {
        Self {
            min_width_pt: 0.0,
            min_width_ratio,
            min_words: 2,
            balance_slack: true,
            max_width_ratio: 1.0,
        }
    }

    /// Returns true if any last-line constraint or balancing objective is active.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.min_width_pt > 0.0
            || self.min_width_ratio > 0.0
            || self.min_words > 1
            || self.balance_slack
            || self.max_width_ratio < 1.0
    }
}

impl From<&tok_core::styles::LastLineConstraintConfig> for LastLineConstraint {
    fn from(cfg: &tok_core::styles::LastLineConstraintConfig) -> Self {
        Self {
            min_width_pt: cfg.min_width_pt,
            min_width_ratio: cfg.min_width_ratio,
            min_words: cfg.min_words.max(1),
            balance_slack: cfg.balance_slack,
            max_width_ratio: 1.0,
        }
    }
}

/// Specification of target line widths and horizontal offsets for a paragraph, following TeX's `\parshape`.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureProfile {
    /// Target width for each line (0-indexed: index 0 is line 1, index 1 is line 2, etc.)
    pub widths: Vec<f32>,
    /// Horizontal start offset for each line (0-indexed, default 0.0).
    pub offsets: Vec<f32>,
    /// If true, lines beyond `widths.len() - 1` repeat the last width and offset.
    pub repeat_last: bool,
    /// Optional constraints for the final line of the paragraph.
    pub last_line_constraint: LastLineConstraint,
}

impl MeasureProfile {
    /// Creates a uniform measure where every line has the same width and zero offset.
    pub fn uniform(width: f32) -> Self {
        Self {
            widths: vec![width],
            offsets: vec![0.0],
            repeat_last: true,
            last_line_constraint: LastLineConstraint::default(),
        }
    }

    /// Creates a profile from an explicit list of widths.
    pub fn from_widths(widths: Vec<f32>, repeat_last: bool) -> Self {
        let offsets = vec![0.0; widths.len()];
        Self {
            widths,
            offsets,
            repeat_last,
            last_line_constraint: LastLineConstraint::default(),
        }
    }

    /// Creates a profile from a slice of widths.
    pub fn from_slice(widths: &[f32], repeat_last: bool) -> Self {
        Self::from_widths(widths.to_vec(), repeat_last)
    }

    /// Creates a profile from pairs of `(offset_x, width)`.
    pub fn from_measures(measures: &[(f32, f32)], repeat_last: bool) -> Self {
        let mut offsets = Vec::with_capacity(measures.len());
        let mut widths = Vec::with_capacity(measures.len());
        for &(x, w) in measures {
            offsets.push(x);
            widths.push(w);
        }
        Self {
            widths,
            offsets,
            repeat_last,
            last_line_constraint: LastLineConstraint::default(),
        }
    }

    /// Overrides horizontal start offsets for the profile.
    pub fn with_offsets(mut self, offsets: Vec<f32>) -> Self {
        self.offsets = offsets;
        self
    }

    /// Attaches a [`LastLineConstraint`] to this measure profile.
    pub fn with_last_line_constraint(mut self, constraint: LastLineConstraint) -> Self {
        self.last_line_constraint = constraint;
        self
    }

    /// Creates an L-shaped profile: `first_lines` of `first_width`,
    /// followed by `remaining_width` for all subsequent lines.
    pub fn l_shape(first_lines: usize, first_width: f32, remaining_width: f32) -> Self {
        let mut widths = Vec::with_capacity(first_lines + 1);
        for _ in 0..first_lines {
            widths.push(first_width);
        }
        widths.push(remaining_width);
        let offsets = vec![0.0; widths.len()];
        Self {
            widths,
            offsets,
            repeat_last: true,
            last_line_constraint: LastLineConstraint::default(),
        }
    }

    /// Creates an L-shaped profile with independent horizontal offsets.
    pub fn l_shape_with_offsets(
        first_lines: usize,
        first_offset: f32,
        first_width: f32,
        rem_offset: f32,
        rem_width: f32,
    ) -> Self {
        let mut widths = Vec::with_capacity(first_lines + 1);
        let mut offsets = Vec::with_capacity(first_lines + 1);
        for _ in 0..first_lines {
            offsets.push(first_offset);
            widths.push(first_width);
        }
        offsets.push(rem_offset);
        widths.push(rem_width);
        Self {
            widths,
            offsets,
            repeat_last: true,
            last_line_constraint: LastLineConstraint::default(),
        }
    }

    /// Returns the target width for line at `line_index` (0-indexed).
    #[inline]
    pub fn width_for_line(&self, line_index: usize) -> f32 {
        if self.widths.is_empty() {
            return 0.0;
        }
        if line_index < self.widths.len() {
            self.widths[line_index]
        } else if self.repeat_last {
            *self.widths.last().unwrap()
        } else {
            0.0
        }
    }

    /// Returns the horizontal start offset for line at `line_index` (0-indexed).
    #[inline]
    pub fn offset_for_line(&self, line_index: usize) -> f32 {
        if self.offsets.is_empty() {
            return 0.0;
        }
        if line_index < self.offsets.len() {
            self.offsets[line_index]
        } else if self.repeat_last {
            *self.offsets.last().unwrap_or(&0.0)
        } else {
            0.0
        }
    }

    /// Builds a MeasureProfile by slicing a polygon at regular line heights.
    pub fn from_polygon(
        polygon: &crate::polygon::GeometricPolygon,
        line_height: f32,
        start_y: f32,
        max_lines: usize,
    ) -> Self {
        let mut widths = Vec::with_capacity(max_lines);
        let mut offsets = Vec::with_capacity(max_lines);
        let safe_lh = line_height.max(1.0);

        for i in 0..max_lines {
            let y = start_y + (i as f32) * safe_lh;
            let spans = polygon.scanline_spans(y, safe_lh);
            if let Some(span) = spans.iter().max_by(|a, b| a.width.total_cmp(&b.width)) {
                offsets.push(span.x);
                widths.push(span.width);
            } else {
                break;
            }
        }

        // If `start_y` fell outside the polygon's vertical range, sample from the polygon's
        // actual bounding box rather than falling back to a hardcoded constant width.
        if widths.is_empty() && max_lines > 0 {
            let bbox = polygon.bounding_box();
            if bbox.width > 0.0 && bbox.height > 0.0 {
                let sample_lines = ((bbox.height / safe_lh).ceil() as usize)
                    .max(1)
                    .min(max_lines);
                for i in 0..sample_lines {
                    let y = bbox.y + (i as f32) * safe_lh;
                    let spans = polygon.scanline_spans(y, safe_lh);
                    if let Some(span) = spans.iter().max_by(|a, b| a.width.total_cmp(&b.width)) {
                        offsets.push(span.x);
                        widths.push(span.width);
                    }
                }
                if widths.is_empty() {
                    offsets.push(bbox.x);
                    widths.push(bbox.width);
                }
            }
        }

        Self {
            widths,
            offsets,
            repeat_last: true,
            last_line_constraint: LastLineConstraint::default(),
        }
    }

    /// Modifies this profile to subtract an exclusion zone (e.g. drop cap window) on intersecting lines.
    pub fn with_drop_cap_exclusion(
        &self,
        line_height: f32,
        exclusion: &crate::polygon::ExclusionZone,
        is_rtl: bool,
    ) -> Self {
        let mut new_widths = Vec::new();
        let mut new_offsets = Vec::new();

        let max_check = self.widths.len().max(10);
        for i in 0..max_check {
            let y = (i as f32) * line_height;
            let orig_w = self.width_for_line(i);
            let orig_x = self.offset_for_line(i);

            if exclusion.intersects_y(y, line_height) {
                let exp = exclusion.expanded_rect();
                let ex_w = exp.width;
                let notched_w = (orig_w - ex_w).max(20.0);
                if is_rtl {
                    // In RTL, drop cap is at the right edge; body text remains on the left
                    new_offsets.push(orig_x);
                    new_widths.push(notched_w);
                } else {
                    // In LTR, drop cap is at left edge; body text is indented
                    new_offsets.push(orig_x + ex_w);
                    new_widths.push(notched_w);
                }
            } else {
                new_offsets.push(orig_x);
                new_widths.push(orig_w);
            }
        }

        Self {
            widths: new_widths,
            offsets: new_offsets,
            repeat_last: self.repeat_last,
            last_line_constraint: self.last_line_constraint,
        }
    }

    /// Validates that the profile has at least one width and all widths are positive finite numbers.
    pub fn is_valid(&self) -> bool {
        !self.widths.is_empty() && self.widths.iter().all(|&w| w.is_finite() && w > 0.0)
    }

    /// Returns true if all lines have the exact same target width and horizontal offset.
    pub fn is_uniform(&self) -> bool {
        if self.widths.is_empty() {
            return false;
        }
        let first_w = self.widths[0];
        let first_o = self.offset_for_line(0);

        let widths_uniform = if self.widths.len() == 1 {
            self.repeat_last
        } else {
            self.widths.iter().all(|&w| (w - first_w).abs() < 1e-4)
        };

        let offsets_uniform = if self.offsets.is_empty() || self.offsets.len() == 1 {
            true
        } else {
            self.offsets.iter().all(|&o| (o - first_o).abs() < 1e-4)
        };

        widths_uniform && offsets_uniform
    }
}

// f64 accumulation reduces cancellation when subtracting long paragraph prefixes.
// Penalties contribute no width here: their width only counts when broken at.
struct PrefixMetrics {
    width: Vec<f64>,
    stretch: Vec<f64>,
    shrink: Vec<f64>,
    boxes: Vec<usize>,
}

impl PrefixMetrics {
    fn new(items: &[LayoutItem]) -> Self {
        let mut sums = Self {
            width: Vec::with_capacity(items.len() + 1),
            stretch: Vec::with_capacity(items.len() + 1),
            shrink: Vec::with_capacity(items.len() + 1),
            boxes: Vec::with_capacity(items.len() + 1),
        };
        let (mut w, mut st, mut sh, mut bx) = (0.0f64, 0.0f64, 0.0f64, 0usize);
        sums.width.push(w);
        sums.stretch.push(st);
        sums.shrink.push(sh);
        sums.boxes.push(bx);
        for item in items {
            match item {
                LayoutItem::Box { width, .. } => {
                    w += f64::from(*width);
                    bx += 1;
                }
                LayoutItem::Glue {
                    width,
                    stretch,
                    shrink,
                } => {
                    w += f64::from(*width);
                    st += f64::from(*stretch);
                    sh += f64::from(*shrink);
                }
                LayoutItem::Penalty { .. } => {}
            }
            sums.width.push(w);
            sums.stretch.push(st);
            sums.shrink.push(sh);
            sums.boxes.push(bx);
        }
        sums
    }

    /// (width, stretch, shrink) summed over `items[start..end]`.
    fn range(&self, start: usize, end: usize) -> (f64, f64, f64) {
        (
            self.width[end] - self.width[start],
            self.stretch[end] - self.stretch[start],
            self.shrink[end] - self.shrink[start],
        )
    }

    /// Number of `LayoutItem::Box` items in `items[start..end]`.
    fn box_count(&self, start: usize, end: usize) -> usize {
        self.boxes[end.min(self.boxes.len() - 1)]
            .saturating_sub(self.boxes[start.min(self.boxes.len() - 1)])
    }
}

#[derive(Clone, Debug)]
struct Node {
    /// Item index of the break (0 for the paragraph start).
    position: usize,
    /// First item of the line that follows this break.
    line_start: usize,
    /// Number of lines formed up to this break (0 for paragraph start).
    line_count: usize,
    total_demerits: f64,
    prev: Option<usize>,
    natural_width: f64,
    target_width: f64,
    ratio: f64,
    flagged: bool,
    fitness: u8,
    last_line_violated: bool,
}

pub struct KnuthPlassBreaker;

impl KnuthPlassBreaker {
    pub const INFINITY_PENALTY: f32 = 10000.0;
    pub const FORCED_BREAK_PENALTY: f32 = -10000.0;

    /// Badness ceiling (TeX's `inf_bad`).
    const INF_BAD: f64 = 10000.0;
    /// Extra demerits for two consecutive flagged (hyphen) breaks.
    const DOUBLE_HYPHEN_DEMERITS: f64 = 3000.0;
    /// Extra demerits when the penultimate line ends with a hyphen.
    const FINAL_HYPHEN_DEMERITS: f64 = 5000.0;
    /// Extra demerits for adjacent lines with incompatible fitness classes (e.g. tight next to loose).
    const ADJACENT_LOOSE_TIGHT_DEMERITS: f64 = 100.0;
    /// Demerits applied when the final line of a multi-line paragraph is narrower than the minimum width constraint.
    const LAST_LINE_SHORT_DEMERITS: f64 = 5.0e6;
    /// Demerits applied when the final line of a multi-line paragraph has fewer words than `min_words`.
    const LAST_LINE_RUNT_DEMERITS: f64 = 5.0e6;
    /// Demerits per point of unavoidable overflow in the emergency pass.
    const OVERFLOW_DEMERITS_PER_PT: f64 = 1.0e8;

    /// Breaks a stream of Box-Glue-Penalty items into optimal lines using a uniform target width.
    pub fn break_paragraph(
        items: &[LayoutItem],
        target_width: f32,
        tolerance: f32, // Typically 1.0 - 2.5
    ) -> Vec<BrokenLine> {
        let profile = MeasureProfile::uniform(target_width);
        Self::break_paragraph_with_profile(items, &profile, tolerance)
    }

    /// Like [`Self::break_paragraph`] but returns item ranges instead of
    /// cloning the items of every line.
    pub fn break_paragraph_spans(
        items: &[LayoutItem],
        target_width: f32,
        tolerance: f32,
    ) -> Vec<LineSpan> {
        let profile = MeasureProfile::uniform(target_width);
        Self::break_paragraph_spans_with_profile(items, &profile, tolerance)
    }

    /// Breaks a stream of items into lines matching a variable [`MeasureProfile`] (TeX `\parshape`).
    pub fn break_paragraph_with_profile(
        items: &[LayoutItem],
        profile: &MeasureProfile,
        tolerance: f32,
    ) -> Vec<BrokenLine> {
        Self::break_paragraph_spans_with_profile(items, profile, tolerance)
            .into_iter()
            .enumerate()
            .map(|(i, span)| BrokenLine {
                line_number: i + 1,
                items: items[span.start..span.end].to_vec(),
                width: span.natural_width,
                target_width: span.target_width,
                x_offset: span.x_offset,
                adjustment_ratio: span.adjustment_ratio,
            })
            .collect()
    }

    /// Breaks a stream of items into line spans matching a variable [`MeasureProfile`].
    pub fn break_paragraph_spans_with_profile(
        items: &[LayoutItem],
        profile: &MeasureProfile,
        tolerance: f32,
    ) -> Vec<LineSpan> {
        if items.is_empty() || !profile.is_valid() {
            return Vec::new();
        }

        let metrics_valid = items.iter().all(|item| match item {
            LayoutItem::Box { width, .. } => width.is_finite() && *width >= 0.0,
            LayoutItem::Glue {
                width,
                stretch,
                shrink,
            } => {
                width.is_finite()
                    && stretch.is_finite()
                    && shrink.is_finite()
                    && *width >= 0.0
                    && *stretch >= 0.0
                    && *shrink >= 0.0
            }
            LayoutItem::Penalty { width, penalty, .. } => {
                width.is_finite() && penalty.is_finite() && *width >= 0.0
            }
        });
        if !tolerance.is_finite() || tolerance < 0.0 || !metrics_valid {
            return Vec::new();
        }

        let ctx = BreakContext::new(items, profile);
        let primary = ctx.run(Some(f64::from(tolerance)));
        let chosen = match primary {
            Some(nodes) => {
                if profile.last_line_constraint.is_active() && ctx.best_final_violated(&nodes) {
                    let relaxed_tol = (f64::from(tolerance) * 2.5).max(5.0);
                    if let Some(relaxed_nodes) = ctx.run(Some(relaxed_tol)) {
                        if ctx.best_final_demerits(&relaxed_nodes) < ctx.best_final_demerits(&nodes)
                        {
                            Some(relaxed_nodes)
                        } else {
                            Some(nodes)
                        }
                    } else {
                        Some(nodes)
                    }
                } else {
                    Some(nodes)
                }
            }
            None => {
                if profile.last_line_constraint.is_active() {
                    let relaxed_tol = (f64::from(tolerance) * 2.5).max(5.0);
                    ctx.run(Some(relaxed_tol)).or_else(|| ctx.run(None))
                } else {
                    ctx.run(None)
                }
            }
        };

        match chosen {
            Some(nodes) => ctx.spans(&nodes),
            None => {
                debug_assert!(false, "emergency pass failed to reach the paragraph end");
                vec![LineSpan {
                    start: 0,
                    end: items.len(),
                    natural_width: ctx.metrics.range(0, items.len()).0 as f32,
                    target_width: profile.width_for_line(0),
                    x_offset: profile.offset_for_line(0),
                    adjustment_ratio: 0.0,
                }]
            }
        }
    }
}

struct BreakContext<'a> {
    items: &'a [LayoutItem],
    profile: &'a MeasureProfile,
    metrics: PrefixMetrics,
    candidates: Vec<usize>,
    monotone: bool,
}

impl<'a> BreakContext<'a> {
    fn new(items: &'a [LayoutItem], profile: &'a MeasureProfile) -> Self {
        let mut candidates = Vec::new();
        for (i, item) in items.iter().enumerate() {
            match item {
                LayoutItem::Glue { .. } => {
                    if i > 0 && matches!(items[i - 1], LayoutItem::Box { .. }) {
                        candidates.push(i);
                    }
                }
                LayoutItem::Penalty { penalty, .. }
                    if *penalty < KnuthPlassBreaker::INFINITY_PENALTY =>
                {
                    candidates.push(i);
                }
                _ => {}
            }
        }
        // The paragraph must end with a forced break; add an implicit one.
        let last_is_forced = candidates.last() == Some(&(items.len() - 1))
            && Self::forced_at(items, items.len() - 1);
        if !last_is_forced {
            candidates.push(items.len());
        }

        let monotone = items.iter().all(|item| match item {
            LayoutItem::Box { width, .. } => *width >= 0.0,
            LayoutItem::Glue { width, shrink, .. } => *shrink >= 0.0 && *width - *shrink >= 0.0,
            LayoutItem::Penalty { .. } => true,
        });

        Self {
            items,
            profile,
            metrics: PrefixMetrics::new(items),
            candidates,
            monotone,
        }
    }

    fn forced_at(items: &[LayoutItem], j: usize) -> bool {
        matches!(items.get(j), Some(LayoutItem::Penalty { penalty, .. })
            if *penalty <= KnuthPlassBreaker::FORCED_BREAK_PENALTY)
    }

    fn is_forced(&self, j: usize) -> bool {
        j == self.items.len() || Self::forced_at(self.items, j)
    }

    /// (penalty value, penalty width, flagged) of a break at `j`.
    fn break_penalty(&self, j: usize) -> (f64, f64, bool) {
        match self.items.get(j) {
            Some(LayoutItem::Penalty {
                width,
                penalty,
                flagged,
            }) => (f64::from(*penalty), f64::from(*width), *flagged),
            _ => (0.0, 0.0, false),
        }
    }

    /// One past the last item of a line that breaks at `j`.
    fn line_end(&self, j: usize) -> usize {
        match self.items.get(j) {
            Some(LayoutItem::Penalty { .. }) => j + 1,
            _ => j,
        }
    }

    /// First item of the line after a break at `j` (discardable items skipped).
    fn next_line_start(&self, j: usize) -> usize {
        let mut k = j + 1;
        while k < self.items.len() {
            match &self.items[k] {
                LayoutItem::Glue { .. } => k += 1,
                LayoutItem::Penalty { penalty, .. }
                    if *penalty > KnuthPlassBreaker::FORCED_BREAK_PENALTY =>
                {
                    k += 1
                }
                _ => break,
            }
        }
        k.min(self.items.len())
    }

    fn badness(r: f64) -> f64 {
        if r.is_finite() {
            (100.0 * r.abs().powi(3)).min(KnuthPlassBreaker::INF_BAD)
        } else {
            KnuthPlassBreaker::INF_BAD
        }
    }

    #[inline]
    fn fitness_class(r: f64) -> u8 {
        if r < -0.5 {
            0 // tight
        } else if r <= 0.5 {
            1 // normal
        } else if r <= 1.0 {
            2 // loose
        } else {
            3 // very loose
        }
    }

    /// Evaluates last-line constraint demerits and returns `(extra_demerits, violated)`.
    fn evaluate_last_line(
        &self,
        node: &Node,
        natural: f64,
        target: f64,
        delta: f64,
        j: usize,
        is_uniform: bool,
    ) -> (f64, bool) {
        // Single-line paragraphs (`node.line_count == 0`) do not have a runt last line.
        if node.line_count == 0 {
            return (0.0, false);
        }

        let mut extra = 0.0;
        let mut violated = false;

        if node.flagged {
            extra += KnuthPlassBreaker::FINAL_HYPHEN_DEMERITS;
        }

        let llc = &self.profile.last_line_constraint;
        let line_end_idx = self.line_end(j).max(node.line_start);
        let words_on_last = self.metrics.box_count(node.line_start, line_end_idx);
        let total_words = self.metrics.box_count(0, self.items.len());

        // 1. Minimum word count (runt prevention)
        if llc.min_words > 1 && words_on_last < llc.min_words && total_words >= llc.min_words {
            let missing = (llc.min_words - words_on_last) as f64;
            extra += KnuthPlassBreaker::LAST_LINE_RUNT_DEMERITS * missing * missing;
            violated = true;
        }

        // 2. Minimum line-width constraint (in pt and/or ratio of target measure)
        let req_min_width = f64::from(llc.min_width_pt)
            .max(target * f64::from(llc.min_width_ratio.clamp(0.0, 1.0)));
        if req_min_width > 0.0 && natural + 1e-4 < req_min_width {
            let shortfall = req_min_width - natural;
            let shortfall_ratio = shortfall / req_min_width.max(1.0);
            extra += KnuthPlassBreaker::LAST_LINE_SHORT_DEMERITS
                * (1.0 + shortfall_ratio * 10.0).powi(2)
                + shortfall * 5000.0;
            violated = true;
        }

        // 3. Maximum line-width ratio constraint (when a visible paragraph-end indent is required)
        if llc.max_width_ratio < 1.0 && llc.max_width_ratio > 0.0 {
            let max_w = target * f64::from(llc.max_width_ratio);
            if natural > max_w + 1e-4 {
                let excess = natural - max_w;
                extra += 1.0e5 * (1.0 + excess);
                violated = true;
            }
        }

        // 4. Slack balancing and optimal final-line placement
        if llc.balance_slack && delta > 0.0 && target > 0.0 {
            let slack_ratio = (delta / target).clamp(0.0, 1.0);
            let slack_badness = 100.0 * slack_ratio.powi(3);
            extra += (1.0 + slack_badness).powi(2) * 0.5;
            // Penalize abrupt jump between a very stretched penultimate line and an empty final line
            if node.ratio > 0.5 && slack_ratio > 0.5 {
                extra += node.ratio * slack_ratio * 500.0;
            }
        } else if !is_uniform && delta > 0.0 && target > 0.0 {
            // In variable-width profiles (L-shapes, polygons), gently prefer final-line placements
            // whose natural width matches the target measure at that line index.
            let slack_ratio = (delta / target).clamp(0.0, 1.0);
            extra += slack_ratio.powi(3) * 20.0;
        }

        (extra, violated)
    }

    fn best_final_node<'b>(&self, nodes: &'b [Node]) -> Option<&'b Node> {
        let final_break = *self.candidates.last()?;
        nodes
            .iter()
            .filter(|n| n.position == final_break)
            .min_by(|a, b| a.total_demerits.total_cmp(&b.total_demerits))
    }

    fn best_final_violated(&self, nodes: &[Node]) -> bool {
        self.best_final_node(nodes)
            .is_some_and(|n| n.last_line_violated)
    }

    fn best_final_demerits(&self, nodes: &[Node]) -> f64 {
        self.best_final_node(nodes)
            .map_or(f64::INFINITY, |n| n.total_demerits)
    }

    /// Runs one pass. `tolerance == None` is the emergency pass, which accepts
    /// every line and therefore always succeeds.
    fn run(&self, tolerance: Option<f64>) -> Option<Vec<Node>> {
        let emergency = tolerance.is_none();
        let tolerance = tolerance.unwrap_or(f64::INFINITY);
        let final_break = *self.candidates.last()?;

        let mut nodes = vec![Node {
            position: 0,
            line_start: 0,
            line_count: 0,
            total_demerits: 0.0,
            prev: None,
            natural_width: 0.0,
            target_width: f64::from(self.profile.width_for_line(0)),
            ratio: 0.0,
            flagged: false,
            fitness: 1,
            last_line_violated: false,
        }];
        let mut window_start = 0;
        let is_uniform = self.profile.is_uniform();
        let track_by_line = !is_uniform || self.profile.last_line_constraint.is_active();

        for &j in &self.candidates {
            let forced = self.is_forced(j);
            let is_last = j == final_break;
            let (penalty, penalty_width, flagged) = self.break_penalty(j);

            let mut best_single: Option<Node> = None;
            let mut best_by_line: std::collections::BTreeMap<usize, Node> =
                std::collections::BTreeMap::new();

            let mut overfull_prefix_end = window_start;
            let mut prefix_overfull = true;

            for (k, node) in nodes.iter().enumerate().skip(window_start) {
                if node.line_start > j || (node.line_start == j && !forced) {
                    prefix_overfull = false;
                    continue;
                }

                let target = f64::from(self.profile.width_for_line(node.line_count));
                let (sum_w, stretch, shrink) = self.metrics.range(node.line_start, j);
                let natural = sum_w + penalty_width;
                let delta = target - natural;

                // Ignores the break-penalty width so that it is monotone in j.
                if prefix_overfull && sum_w - shrink > target {
                    overfull_prefix_end = k + 1;
                } else {
                    prefix_overfull = false;
                }

                let r = if delta > 0.0 {
                    if is_last {
                        0.0 // fill glue at the end of the paragraph
                    } else if stretch > 0.0 {
                        delta / stretch
                    } else {
                        f64::INFINITY
                    }
                } else if delta < 0.0 {
                    if shrink > 0.0 {
                        delta / shrink
                    } else {
                        f64::NEG_INFINITY
                    }
                } else {
                    0.0
                };
                let overfull = r < -1.0;
                if !emergency && (overfull || r > tolerance) {
                    continue;
                }

                let mut demerits = (1.0 + Self::badness(r)).powi(2);
                if penalty >= 0.0 {
                    demerits += penalty * penalty;
                } else if !forced {
                    demerits -= penalty * penalty;
                }
                if flagged && node.flagged {
                    demerits += KnuthPlassBreaker::DOUBLE_HYPHEN_DEMERITS;
                }
                let fitness = Self::fitness_class(r);
                if !is_last && node.line_count > 0 && fitness.abs_diff(node.fitness) > 1 {
                    demerits += KnuthPlassBreaker::ADJACENT_LOOSE_TIGHT_DEMERITS;
                }
                let mut last_line_violated = false;
                if is_last {
                    let (extra_last, violated) =
                        self.evaluate_last_line(node, natural, target, delta, j, is_uniform);
                    demerits += extra_last;
                    last_line_violated = violated;
                }
                if overfull {
                    let overflow = (natural - shrink - target).max(0.0);
                    demerits += KnuthPlassBreaker::OVERFLOW_DEMERITS_PER_PT * (1.0 + overflow);
                }

                let total = node.total_demerits + demerits;
                let candidate_node = Node {
                    position: j,
                    line_start: if j < self.items.len() {
                        self.next_line_start(j)
                    } else {
                        j
                    },
                    line_count: node.line_count + 1,
                    total_demerits: total,
                    prev: Some(k),
                    natural_width: natural,
                    target_width: target,
                    ratio: if r.is_finite() { r } else { 0.0 },
                    flagged,
                    fitness,
                    last_line_violated,
                };

                if !track_by_line {
                    if best_single
                        .as_ref()
                        .is_none_or(|b| total < b.total_demerits)
                    {
                        best_single = Some(candidate_node);
                    }
                } else {
                    let new_lines = candidate_node.line_count;
                    match best_by_line.entry(new_lines) {
                        std::collections::btree_map::Entry::Vacant(e) => {
                            e.insert(candidate_node);
                        }
                        std::collections::btree_map::Entry::Occupied(mut e) => {
                            if total < e.get().total_demerits {
                                e.insert(candidate_node);
                            }
                        }
                    }
                }
            }

            // Advancing window_start via overfull_prefix_end prunes any contiguous prefix of nodes
            // that are already overfull for candidate j. For monotone metrics, any node with
            // sum_w - shrink > target will have sum_w' - shrink' >= sum_w - shrink > target for all
            // future candidates j' > j. Since each node's line_count (and thus its target width) is
            // invariant, no node in this overfull prefix can ever form a valid non-overfull line to
            // any future break j' >= j. This pruning is disabled in emergency passes where overfull
            // lines are allowed.
            if self.monotone && !emergency {
                window_start = overfull_prefix_end;
            }

            let nodes_to_add: Vec<Node> = if !track_by_line {
                best_single.into_iter().collect()
            } else {
                best_by_line.into_values().collect()
            };

            let added_count = nodes_to_add.len();
            if added_count > 0 {
                nodes.extend(nodes_to_add);
                if forced {
                    window_start = nodes.len() - added_count;
                }
                if is_last {
                    break;
                }
            } else if forced {
                return None;
            } else if window_start >= nodes.len() {
                if emergency {
                    window_start = nodes.len() - 1;
                } else {
                    return None;
                }
            }
        }

        let best_final = self.best_final_node(&nodes);

        if best_final.is_some() && nodes.len() > 1 {
            Some(nodes)
        } else {
            None
        }
    }

    fn spans(&self, nodes: &[Node]) -> Vec<LineSpan> {
        let mut chain = Vec::new();
        let final_break = *self.candidates.last().unwrap();
        let best_final_idx = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.position == final_break)
            .min_by(|(_, a), (_, b)| a.total_demerits.total_cmp(&b.total_demerits))
            .map(|(i, _)| i);

        let mut current = best_final_idx;
        while let Some(idx) = current {
            chain.push(idx);
            current = nodes[idx].prev;
        }
        chain.reverse();
        chain
            .windows(2)
            .map(|w| {
                let (from, to) = (&nodes[w[0]], &nodes[w[1]]);
                let line_idx = to.line_count.saturating_sub(1);
                LineSpan {
                    start: from.line_start,
                    end: self.line_end(to.position).max(from.line_start),
                    natural_width: to.natural_width as f32,
                    target_width: to.target_width as f32,
                    x_offset: self.profile.offset_for_line(line_idx),
                    adjustment_ratio: to.ratio as f32,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(width: f32) -> LayoutItem {
        LayoutItem::Box {
            width,
            text: String::new(),
            glyphs: Vec::new(),
        }
    }

    fn space() -> LayoutItem {
        LayoutItem::Glue {
            width: 10.0,
            stretch: 5.0,
            shrink: 2.0,
        }
    }

    fn forced() -> LayoutItem {
        LayoutItem::Penalty {
            width: 0.0,
            penalty: KnuthPlassBreaker::FORCED_BREAK_PENALTY,
            flagged: false,
        }
    }

    fn paragraph(widths: &[f32]) -> Vec<LayoutItem> {
        let mut items = Vec::new();
        for (i, w) in widths.iter().enumerate() {
            if i > 0 {
                items.push(space());
            }
            items.push(word(*w));
        }
        items.push(forced());
        items
    }

    fn boxes_per_line(items: &[LayoutItem], spans: &[LineSpan]) -> Vec<usize> {
        spans
            .iter()
            .map(|s| {
                items[s.start..s.end]
                    .iter()
                    .filter(|i| matches!(i, LayoutItem::Box { .. }))
                    .count()
            })
            .collect()
    }

    #[test]
    fn prefix_metrics_match_reference_scan_for_all_ranges() {
        let mut items = Vec::new();
        for i in 0..64 {
            items.push(LayoutItem::Box {
                width: 10.25 + (i % 7) as f32,
                text: String::new(),
                glyphs: vec![],
            });
            items.push(LayoutItem::Glue {
                width: 3.25,
                stretch: 1.5,
                shrink: 0.75,
            });
            items.push(LayoutItem::Penalty {
                width: 2.5,
                penalty: 50.0,
                flagged: false,
            });
        }
        let metrics = PrefixMetrics::new(&items);
        for start in 0..items.len() {
            for end in start + 1..=items.len() {
                let (mut width, mut stretch, mut shrink) = (0.0f64, 0.0f64, 0.0f64);
                for item in &items[start..end] {
                    match item {
                        LayoutItem::Box { width: w, .. } => width += f64::from(*w),
                        LayoutItem::Glue {
                            width: w,
                            stretch: s,
                            shrink: h,
                        } => {
                            width += f64::from(*w);
                            stretch += f64::from(*s);
                            shrink += f64::from(*h);
                        }
                        LayoutItem::Penalty { .. } => {}
                    }
                }
                let (w, s, h) = metrics.range(start, end);
                assert!((w - width).abs() < 1e-9);
                assert!((s - stretch).abs() < 1e-9);
                assert!((h - shrink).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn nonfinite_metrics_and_tolerance_are_rejected() {
        let item = LayoutItem::Box {
            width: f32::NAN,
            text: String::new(),
            glyphs: vec![],
        };
        assert!(KnuthPlassBreaker::break_paragraph(&[item], 100.0, 2.0).is_empty());
        let item = LayoutItem::Glue {
            width: 2.0,
            stretch: 1.0,
            shrink: 1.0,
        };
        assert!(KnuthPlassBreaker::break_paragraph(&[item], 100.0, f32::INFINITY).is_empty());
    }

    #[test]
    fn test_knuth_plass_line_break() {
        // Target width 120 pt -> should break into 2 balanced lines of 2 words each
        let items = paragraph(&[50.0, 50.0, 50.0, 50.0]);
        let lines = KnuthPlassBreaker::break_paragraph(&items, 120.0, 2.0);
        assert_eq!(lines.len(), 2);
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 120.0, 2.0);
        assert_eq!(boxes_per_line(&items, &spans), vec![2, 2]);
    }

    #[test]
    fn test_knuth_plass_empty_paragraph() {
        let items: Vec<LayoutItem> = vec![];
        let lines = KnuthPlassBreaker::break_paragraph(&items, 300.0, 2.0);
        assert!(lines.is_empty(), "Empty items should produce empty lines");
    }

    #[test]
    fn test_knuth_plass_single_word() {
        let items = vec![LayoutItem::Box {
            width: 50.0,
            text: "word".to_string(),
            glyphs: Vec::new(),
        }];
        let lines = KnuthPlassBreaker::break_paragraph(&items, 300.0, 2.0);
        assert_eq!(lines.len(), 1, "Single word should produce one line");
        let has_word = lines[0].items.iter().any(|item| {
            if let LayoutItem::Box { text, .. } = item {
                text.contains("word")
            } else {
                false
            }
        });
        assert!(has_word);
    }

    #[test]
    fn test_knuth_plass_zero_width() {
        let items = vec![word(50.0)];
        assert!(KnuthPlassBreaker::break_paragraph(&items, 0.0, 2.0).is_empty());
    }

    #[test]
    fn test_knuth_plass_nan_width() {
        let items = vec![word(50.0)];
        assert!(KnuthPlassBreaker::break_paragraph(&items, f32::NAN, 2.0).is_empty());
    }

    /// Regression: the line after a glue break used to start with that glue,
    /// counting its width and stretch.
    #[test]
    fn lines_never_start_with_glue() {
        let items = paragraph(&[40.0, 30.0, 55.0, 20.0, 45.0, 35.0, 50.0, 25.0, 60.0]);
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 120.0, 2.0);
        assert!(spans.len() > 1);
        for s in &spans {
            assert!(
                matches!(items[s.start], LayoutItem::Box { .. }),
                "line starts with {:?}",
                items[s.start]
            );
        }
        // Lines partition the boxes: every word appears exactly once.
        let total: usize = boxes_per_line(&items, &spans).iter().sum();
        assert_eq!(total, 9);
    }

    /// Regression: a short last line was infeasible (it needed more stretch than
    /// the tolerance), which sent every ordinary paragraph to a greedy fallback.
    #[test]
    fn short_last_line_is_free() {
        let items = paragraph(&[50.0, 50.0, 50.0, 50.0, 30.0]);
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 120.0, 2.0);
        assert_eq!(boxes_per_line(&items, &spans), vec![2, 2, 1]);
        let last = spans.last().unwrap();
        assert_eq!(last.adjustment_ratio, 0.0);
        for s in &spans[..spans.len() - 1] {
            assert!(s.adjustment_ratio.abs() <= 2.0);
        }
    }

    /// Regression: a forced break in the middle of a paragraph could be
    /// skipped over by a line.
    #[test]
    fn forced_break_in_the_middle_is_honoured() {
        let mut items = vec![word(20.0), space(), word(20.0)];
        items.push(forced());
        items.extend([word(20.0), space(), word(20.0), forced()]);
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 300.0, 2.0);
        assert_eq!(boxes_per_line(&items, &spans), vec![2, 2]);
    }

    /// Regression: a word wider than the measure made the whole paragraph fall
    /// back to a greedy breaker whose lines could exceed the measure anywhere.
    #[test]
    fn overlong_word_gets_its_own_line_and_the_rest_stays_optimal() {
        let items = paragraph(&[50.0, 50.0, 400.0, 50.0, 50.0]);
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 120.0, 2.0);
        assert_eq!(boxes_per_line(&items, &spans), vec![2, 1, 2]);
        assert!(spans[1].natural_width > 120.0);
        assert!(spans[0].natural_width <= 120.0 + 1e-3);
    }

    #[test]
    fn penalty_width_counts_only_when_broken_at() {
        let items = vec![
            word(50.0),
            LayoutItem::Penalty {
                width: 7.0,
                penalty: 50.0,
                flagged: true,
            },
            word(50.0),
            forced(),
        ];
        // Fits on one line: the unused hyphen must not add 7pt.
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 100.0, 2.0);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].natural_width, 100.0);
        // Too narrow: break at the hyphen, whose width now counts.
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 60.0, 2.0);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].natural_width, 57.0);
        assert_eq!(spans[0].end, 2);
    }

    #[test]
    fn paragraph_without_final_penalty_ends_implicitly() {
        let items = vec![word(30.0), space(), word(30.0)];
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 100.0, 2.0);
        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].start, spans[0].end), (0, 3));
    }

    #[test]
    fn no_extra_empty_line_at_paragraph_end() {
        let items = paragraph(&[30.0, 30.0]);
        let lines = KnuthPlassBreaker::break_paragraph(&items, 100.0, 2.0);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn huge_paragraph_is_fast_and_complete() {
        let widths: Vec<f32> = (0..20_000).map(|i| 20.0 + (i % 13) as f32 * 3.0).collect();
        let items = paragraph(&widths);
        let start = std::time::Instant::now();
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 300.0, 2.0);
        assert!(start.elapsed().as_secs() < 5);
        let total: usize = boxes_per_line(&items, &spans).iter().sum();
        assert_eq!(total, widths.len());
    }

    #[test]
    fn test_l_shaped_line_break() {
        // First 3 lines narrow (120 pt), subsequent lines wide (240 pt)
        // Each word is 50pt wide. Space is 10pt (stretch 4, shrink 2).
        // On narrow (120pt): 2 words = 50 + 10 + 50 = 110pt (fits). 3 words = 170pt (overfull).
        // On wide (240pt): 4 words = 50*4 + 10*3 = 230pt (fits).
        let words = vec![50.0; 12];
        let items = paragraph(&words);
        let profile = MeasureProfile::l_shape(3, 120.0, 240.0);
        let lines = KnuthPlassBreaker::break_paragraph_with_profile(&items, &profile, 2.0);

        assert!(
            lines.len() >= 4,
            "Expected at least 4 lines, got {}",
            lines.len()
        );
        assert_eq!(lines[0].target_width, 120.0);
        assert_eq!(lines[1].target_width, 120.0);
        assert_eq!(lines[2].target_width, 120.0);
        assert_eq!(lines[3].target_width, 240.0);
        if lines.len() > 4 {
            assert_eq!(lines[4].target_width, 240.0);
        }

        let spans = KnuthPlassBreaker::break_paragraph_spans_with_profile(&items, &profile, 2.0);
        let word_counts = boxes_per_line(&items, &spans);
        let total_boxes: usize = word_counts.iter().sum();
        assert_eq!(total_boxes, 12);

        // Rigorous proof that text is physically packed according to the variable measure:
        // Narrow lines (0, 1, 2) can hold at most 2 words each.
        assert_eq!(word_counts[0], 2, "Line 0 (narrow 120pt) must pack 2 words");
        assert_eq!(word_counts[1], 2, "Line 1 (narrow 120pt) must pack 2 words");
        assert_eq!(word_counts[2], 2, "Line 2 (narrow 120pt) must pack 2 words");
        // Expanded non-terminal line 3 must pack at least 4 words in the 240pt measure:
        assert!(
            word_counts[3] >= 4,
            "Line 3 (expanded 240pt) must pack >= 4 words, got {}",
            word_counts[3]
        );
    }

    #[test]
    fn test_non_monotone_zigzag_profile() {
        // Alternating widths: 200pt (wide), 80pt (narrow), 240pt (wide), 80pt (narrow), 200pt (wide)
        // Proves that DP navigates non-monotone width profiles without dropping valid candidate nodes.
        let profile = MeasureProfile::from_widths(vec![200.0, 80.0, 240.0, 80.0, 200.0], true);
        assert!(!profile.is_uniform());

        let words = vec![35.0; 16];
        let items = paragraph(&words);
        let spans = KnuthPlassBreaker::break_paragraph_spans_with_profile(&items, &profile, 2.0);
        assert!(!spans.is_empty());

        let word_counts = boxes_per_line(&items, &spans);
        let total_words: usize = word_counts.iter().sum();
        assert_eq!(total_words, 16);

        // Verify target widths assigned to spans match the alternating profile
        assert_eq!(spans[0].target_width, 200.0);
        assert_eq!(spans[1].target_width, 80.0);
        assert_eq!(spans[2].target_width, 240.0);
        if spans.len() > 3 {
            assert_eq!(spans[3].target_width, 80.0);
        }

        // Narrow line 1 (80pt) should pack fewer words (word=35, space=10 -> at most 2 words = 80pt)
        assert!(
            word_counts[1] <= 2,
            "Narrow line 1 should pack <= 2 words, got {}",
            word_counts[1]
        );
        // Wide line 2 (240pt) should pack significantly more words
        assert!(
            word_counts[2] >= 4,
            "Wide line 2 should pack >= 4 words, got {}",
            word_counts[2]
        );
    }

    #[test]
    fn test_mid_paragraph_width_change_cutout() {
        // Line 1: wide (200pt)
        // Line 2: narrow (90pt) - wrapping around cutout/sidebar
        // Line 3: narrow (90pt)
        // Line 4+: wide (200pt)
        let words = vec![40.0; 10];
        let items = paragraph(&words);
        let profile = MeasureProfile::from_widths(vec![200.0, 90.0, 90.0, 200.0], true);
        let lines = KnuthPlassBreaker::break_paragraph_with_profile(&items, &profile, 2.0);

        assert_eq!(lines[0].target_width, 200.0);
        assert_eq!(lines[1].target_width, 90.0);
        assert_eq!(lines[2].target_width, 90.0);
        if lines.len() >= 4 {
            assert_eq!(lines[3].target_width, 200.0);
        }

        let spans = KnuthPlassBreaker::break_paragraph_spans_with_profile(&items, &profile, 2.0);
        let total_boxes: usize = boxes_per_line(&items, &spans).iter().sum();
        assert_eq!(total_boxes, 10);
    }

    #[test]
    fn test_measure_profile_repeat_last() {
        let profile = MeasureProfile::from_widths(vec![100.0, 200.0], true);
        assert_eq!(profile.width_for_line(0), 100.0);
        assert_eq!(profile.width_for_line(1), 200.0);
        assert_eq!(profile.width_for_line(2), 200.0);
        assert_eq!(profile.width_for_line(10), 200.0);

        let no_repeat = MeasureProfile::from_widths(vec![100.0, 200.0], false);
        assert_eq!(no_repeat.width_for_line(0), 100.0);
        assert_eq!(no_repeat.width_for_line(1), 200.0);
        assert_eq!(no_repeat.width_for_line(2), 0.0);
    }

    #[test]
    fn test_measure_profile_edge_cases() {
        let items = paragraph(&[50.0, 50.0]);

        // Empty widths
        let empty_profile = MeasureProfile::from_widths(vec![], true);
        assert!(!empty_profile.is_valid());
        assert!(
            KnuthPlassBreaker::break_paragraph_with_profile(&items, &empty_profile, 2.0).is_empty()
        );

        // Non-positive or NaN widths
        let zero_profile = MeasureProfile::from_widths(vec![0.0], true);
        assert!(!zero_profile.is_valid());
        let nan_profile = MeasureProfile::from_widths(vec![f32::NAN], true);
        assert!(!nan_profile.is_valid());
        let neg_profile = MeasureProfile::from_widths(vec![-50.0], true);
        assert!(!neg_profile.is_valid());

        // Single word wider than first narrow line triggers emergency pass without loss
        let items_overlong = paragraph(&[150.0, 40.0]);
        let narrow_first = MeasureProfile::l_shape(1, 100.0, 200.0);
        let lines =
            KnuthPlassBreaker::break_paragraph_with_profile(&items_overlong, &narrow_first, 2.0);
        assert!(!lines.is_empty());
        let total_boxes: usize = lines
            .iter()
            .map(|l| {
                l.items
                    .iter()
                    .filter(|i| matches!(i, LayoutItem::Box { .. }))
                    .count()
            })
            .sum();
        assert_eq!(total_boxes, 2);
    }

    #[test]
    fn test_hebrew_words_measure_profile() {
        let words = [
            "מאימתי",
            "קורין",
            "את",
            "שמע",
            "בערבין",
            "משעה",
            "שהכהנים",
            "נכנסים",
            "לאכול",
            "בתרומתן",
        ];
        let mut items = Vec::new();
        for (i, w) in words.iter().enumerate() {
            if i > 0 {
                items.push(space());
            }
            items.push(LayoutItem::Box {
                width: 35.0,
                text: w.to_string(),
                glyphs: Vec::new(),
            });
        }
        items.push(forced());

        let profile = MeasureProfile::l_shape(2, 90.0, 180.0);
        let lines = KnuthPlassBreaker::break_paragraph_with_profile(&items, &profile, 2.0);
        assert!(lines.len() >= 3);
        assert_eq!(lines[0].target_width, 90.0);
        assert_eq!(lines[1].target_width, 90.0);
        assert_eq!(lines[2].target_width, 180.0);

        let total_words: usize = lines
            .iter()
            .map(|l| {
                l.items
                    .iter()
                    .filter(|it| matches!(it, LayoutItem::Box { .. }))
                    .count()
            })
            .sum();
        assert_eq!(total_words, words.len());
    }

    #[test]
    fn test_knuth_plass_degenerate_and_overconstrained_metrics() {
        // Negative box width is rejected
        let neg_box = LayoutItem::Box {
            width: -10.0,
            text: "test".to_string(),
            glyphs: vec![],
        };
        assert!(KnuthPlassBreaker::break_paragraph(&[neg_box], 100.0, 2.0).is_empty());

        // Negative glue shrink is rejected
        let neg_shrink = LayoutItem::Glue {
            width: 10.0,
            stretch: 5.0,
            shrink: -2.0,
        };
        assert!(
            KnuthPlassBreaker::break_paragraph(&[word(50.0), neg_shrink], 100.0, 2.0).is_empty()
        );

        // Infinite penalty is rejected
        let inf_penalty = LayoutItem::Penalty {
            width: 0.0,
            penalty: f32::INFINITY,
            flagged: false,
        };
        assert!(
            KnuthPlassBreaker::break_paragraph(&[word(50.0), inf_penalty], 100.0, 2.0).is_empty()
        );

        // Target width zero or negative returns empty without panic
        let items = paragraph(&[40.0, 40.0]);
        assert!(KnuthPlassBreaker::break_paragraph(&items, 0.0, 2.0).is_empty());
        assert!(KnuthPlassBreaker::break_paragraph(&items, -50.0, 2.0).is_empty());

        // Extreme overconstraint: 50pt words into 5pt column
        // Emergency pass must succeed and preserve every word across lines
        let extreme_narrow = 5.0;
        let lines = KnuthPlassBreaker::break_paragraph(&items, extreme_narrow, 2.0);
        assert_eq!(lines.len(), 2);
        let total_words: usize = lines
            .iter()
            .map(|l| {
                l.items
                    .iter()
                    .filter(|it| matches!(it, LayoutItem::Box { .. }))
                    .count()
            })
            .sum();
        assert_eq!(total_words, 2);
    }

    #[test]
    fn test_knuth_plass_multiple_forced_breaks() {
        // Multiple consecutive forced breaks (empty lines)
        let items = vec![word(30.0), forced(), forced(), word(40.0), forced()];
        let lines = KnuthPlassBreaker::break_paragraph(&items, 100.0, 2.0);
        assert!(!lines.is_empty());
        let total_words: usize = lines
            .iter()
            .map(|l| {
                l.items
                    .iter()
                    .filter(|it| matches!(it, LayoutItem::Box { .. }))
                    .count()
            })
            .sum();
        assert_eq!(total_words, 2);
    }

    #[test]
    fn test_knuth_plass_glue_only() {
        let items = vec![space(), space()];
        let lines = KnuthPlassBreaker::break_paragraph(&items, 100.0, 2.0);
        // Valid glue-only sequence completes without panic
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_measure_profile_with_offsets_and_measures() {
        let measures = vec![(0.0, 100.0), (30.0, 70.0), (0.0, 100.0)];
        let profile = MeasureProfile::from_measures(&measures, true);

        assert_eq!(profile.width_for_line(0), 100.0);
        assert_eq!(profile.offset_for_line(0), 0.0);

        assert_eq!(profile.width_for_line(1), 70.0);
        assert_eq!(profile.offset_for_line(1), 30.0);

        assert_eq!(profile.width_for_line(2), 100.0);
        assert_eq!(profile.offset_for_line(2), 0.0);

        // Repeated lines follow last
        assert_eq!(profile.width_for_line(5), 100.0);
        assert_eq!(profile.offset_for_line(5), 0.0);
    }

    #[test]
    fn test_measure_profile_from_polygon_slicing() {
        let l_poly = crate::polygon::GeometricPolygon::l_shape(0.0, 0.0, 120.0, 45.0, 240.0, 45.0);
        let profile = MeasureProfile::from_polygon(&l_poly, 15.0, 0.0, 6);

        // Lines 0, 1, 2 (y < 45) have width ~120
        assert!((profile.width_for_line(0) - 120.0).abs() < 1e-2);
        assert!((profile.width_for_line(1) - 120.0).abs() < 1e-2);
        assert!((profile.width_for_line(2) - 120.0).abs() < 1e-2);

        // Lines 3, 4, 5 (y >= 45) expand to ~240
        assert!((profile.width_for_line(3) - 240.0).abs() < 1e-2);
        assert!((profile.width_for_line(4) - 240.0).abs() < 1e-2);
    }

    #[test]
    fn test_knuth_plass_breaking_with_drop_cap_notch() {
        // Drop cap window in top right: 3 lines tall, 80pt wide with 5pt margin (total 90pt clearance)
        let ex = crate::polygon::ExclusionZone::new(
            crate::geometry::PhysicalRect::new(220.0, 0.0, 80.0, 42.0),
            5.0,
        );
        let base_profile = MeasureProfile::uniform(300.0);
        let notched_profile = base_profile.with_drop_cap_exclusion(14.0, &ex, true);

        // Line 0..3 have reduced width: 300 - 90 = 210
        assert!((notched_profile.width_for_line(0) - 210.0).abs() < 1e-2);
        assert!((notched_profile.width_for_line(1) - 210.0).abs() < 1e-2);
        assert!((notched_profile.width_for_line(2) - 210.0).abs() < 1e-2);
        // Line 4+ has full 300
        assert!((notched_profile.width_for_line(4) - 300.0).abs() < 1e-2);

        // Break a stream of 12 words of 35pt each + 10pt spaces into optimal lines
        let mut items = Vec::new();
        for _ in 0..12 {
            if !items.is_empty() {
                items.push(space());
            }
            items.push(word(35.0));
        }
        items.push(forced());

        let lines = KnuthPlassBreaker::break_paragraph_with_profile(&items, &notched_profile, 2.0);
        assert!(lines.len() >= 3);
        assert_eq!(lines[0].target_width, 210.0);
        assert_eq!(lines[1].target_width, 210.0);
    }

    #[test]
    fn test_last_line_minimum_width_and_runt_prevention() {
        // 7 words: [40, 40, 40, 40, 40, 40, 25], space width 10 (stretch 15, shrink 2)
        // Target measure: 140pt.
        // Without last-line constraint:
        //   Line 1: 3 words (40*3 + 20 = 140pt, r = 0)
        //   Line 2: 3 words (40*3 + 20 = 140pt, r = 0)
        //   Line 3: 1 word (25pt -> single-word runt, natural width 25pt < 60pt!)
        // With LastLineConstraint { min_width_pt: 60.0, min_words: 2, .. }:
        //   Breaker rebalances preceding lines (e.g. [3, 2, 2] or [2, 3, 2]) so the final line
        //   has 2 words and natural width 40 + 10 + 25 = 75pt >= 60pt.
        let widths = [40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 25.0];
        let mut items = Vec::new();
        for (i, &w) in widths.iter().enumerate() {
            if i > 0 {
                items.push(LayoutItem::Glue {
                    width: 10.0,
                    stretch: 15.0,
                    shrink: 2.0,
                });
            }
            items.push(word(w));
        }
        items.push(forced());

        let unconstrained_spans = KnuthPlassBreaker::break_paragraph_spans(&items, 140.0, 2.0);
        assert_eq!(
            boxes_per_line(&items, &unconstrained_spans),
            vec![3, 3, 1],
            "Unconstrained break leaves a 1-word 25pt runt on the last line"
        );
        assert!((unconstrained_spans.last().unwrap().natural_width - 25.0).abs() < 1e-3);

        let constrained_profile = MeasureProfile::uniform(140.0).with_last_line_constraint(
            LastLineConstraint {
                min_width_pt: 60.0,
                min_width_ratio: 0.4,
                min_words: 2,
                balance_slack: true,
                max_width_ratio: 1.0,
            },
        );
        let constrained_spans =
            KnuthPlassBreaker::break_paragraph_spans_with_profile(&items, &constrained_profile, 2.0);
        let word_counts = boxes_per_line(&items, &constrained_spans);
        let total_words: usize = word_counts.iter().sum();
        assert_eq!(total_words, 7);

        let last_span = constrained_spans.last().unwrap();
        let last_words = *word_counts.last().unwrap();
        assert!(
            last_words >= 2,
            "Constrained last line must have >= 2 words, got {}",
            last_words
        );
        assert!(
            last_span.natural_width >= 60.0,
            "Constrained last line natural width ({}) must be >= 60.0pt",
            last_span.natural_width
        );
    }

    #[test]
    fn test_last_line_constraint_relaxes_tolerance_when_needed() {
        // At strict tolerance = 1.0, a 2-word line (40 + 10 + 40 = 90pt on 140pt measure, delta = 50pt)
        // with stretch = 15pt has r = 50 / 15 = 3.33 > 1.0.
        // Verify that when last_line_constraint is active, the breaker automatically relaxes tolerance
        // to eliminate the 1-word runt without producing overfull lines.
        let widths = [40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 25.0];
        let mut items = Vec::new();
        for (i, &w) in widths.iter().enumerate() {
            if i > 0 {
                items.push(LayoutItem::Glue {
                    width: 10.0,
                    stretch: 15.0,
                    shrink: 2.0,
                });
            }
            items.push(word(w));
        }
        items.push(forced());

        let profile = MeasureProfile::uniform(140.0)
            .with_last_line_constraint(LastLineConstraint::balanced(0.45));
        let spans = KnuthPlassBreaker::break_paragraph_spans_with_profile(&items, &profile, 1.0);
        let counts = boxes_per_line(&items, &spans);
        assert_eq!(counts.iter().sum::<usize>(), 7);
        assert!(
            *counts.last().unwrap() >= 2,
            "Relaxed pass must eliminate single-word runt: {:?}",
            counts
        );
        assert!(spans.last().unwrap().natural_width >= 140.0 * 0.45);
    }

    #[test]
    fn test_final_hyphen_demerits_avoids_penultimate_hyphenation() {
        // Construct a paragraph on a 100pt measure where the penultimate line can either:
        // - Break cleanly at a word space (index 3 -> end = 4):
        //   Line 1 = 42 + 10 + 40 = 92pt (stretch = 16 -> r = 0.5, badness ~ 12.5, demerits ~ 183)
        //   Line 2 = 3 + 0 + 30 + 10 + 35 = 78pt (final line, r = 0, demerits = 1)
        // - Break at a flagged hyphen penalty (index 5 -> end = 6):
        //   Line 1 = 42 + 10 + 40 + 3 + 3 + 2 = 100pt (r = 0, flagged = true -> +FINAL_HYPHEN_DEMERITS = 5036)
        //   Line 2 = 30 + 10 + 35 = 75pt (final line, r = 0, demerits = 1)
        let items = vec![
            word(42.0),
            LayoutItem::Glue {
                width: 10.0,
                stretch: 16.0,
                shrink: 2.0,
            },
            word(40.0),
            LayoutItem::Glue {
                width: 3.0,
                stretch: 4.0,
                shrink: 1.0,
            },
            word(3.0),
            LayoutItem::Penalty {
                width: 2.0,
                penalty: 5.0,
                flagged: true,
            },
            word(30.0),
            LayoutItem::Glue {
                width: 10.0,
                stretch: 16.0,
                shrink: 2.0,
            },
            word(35.0),
            forced(),
        ];
        let spans = KnuthPlassBreaker::break_paragraph_spans(&items, 100.0, 2.5);
        assert_eq!(spans.len(), 2);
        // Line 1 should break at the word space (glue index 3 -> span end = 3), NOT at the flagged hyphen (end = 6)
        assert_eq!(
            spans[0].end, 3,
            "Penultimate line should avoid breaking at a hyphen when a clean word space break is feasible"
        );
    }
}

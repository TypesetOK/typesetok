//! Spatial profile and detected region models for a single analyzed page.

use crate::geometry::NormalizedBBox;
use crate::spread::SpreadSide;
use crate::typography::TypographyMetrics;
use serde::{Deserialize, Serialize};

/// Semantic class of a detected visual or vector region on a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionClass {
    /// Body text block (square or commentary).
    Text,
    /// Section or page title / spanning header.
    Title,
    /// Footnote or bottom apparatus block.
    Footnote,
    /// Running page header.
    Header,
    /// Running page footer or page number.
    Footer,
    /// Horizontal or vertical separator rule.
    Rule,
    /// Decorative frame or ornament.
    Ornament,
    /// Embedded figure, diagram, or table.
    Figure,
    /// Narrow outer-margin reference (e.g. Ein Mishpat / Masoret HaShas).
    Marginalia,
}

/// A single detected region on a page with normalized coordinates and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectedRegion {
    /// Normalized bounding box `[0.0, 1.0]`.
    pub bbox: NormalizedBBox,
    /// Semantic region class.
    pub region_class: RegionClass,
    /// Detection confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Optional median font size or estimated line height in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_pt: Option<f32>,
    /// Optional font family name (when extracted from vector PDF).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    /// Estimated median line height in normalized page units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median_line_height: Option<f32>,
    /// Whether the region uses Rashi / semi-cursive script (if classified).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_rashi_script: Option<bool>,
}

/// Intermediate representation of a page's spatial and typographic structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageSpatialProfile {
    /// 1-based physical page index in the input document.
    pub page_index: usize,
    /// Page width-to-height aspect ratio (`width / height`).
    pub aspect_ratio: f32,
    /// Inferred spread side (`Recto` / `Verso`) when known or inferred from page parity.
    pub spread_side: SpreadSide,
    /// All detected regions on this page.
    pub regions: Vec<DetectedRegion>,
    /// Effective number of vertical text columns detected via XY-Cut / vertical projection.
    pub effective_column_count: usize,
    /// Normalized width proportions of the primary vertical columns (sum ~ 1.0).
    pub column_proportions: Vec<f32>,
    /// Normalized horizontal gutter width between adjacent columns.
    pub estimated_gutter_ratio: f32,
    /// Horizontal asymmetry index in `[0.0, 1.0]` (0 = symmetric, >0.25 = asymmetric).
    pub horizontal_asymmetry: f32,
    /// Fraction of page area covered by text regions `[0.0, 1.0]`.
    pub text_coverage: f32,
    /// Whether an L-shaped commentary wrap was detected beneath a central block.
    pub has_l_shape_signature: bool,
    /// Whether a horizontal separator rule sits above a dense bottom footnote band.
    pub has_footnote_rule_and_band: bool,
    /// Whether a wide spanning header sits above multi-column body regions.
    pub has_spanning_header: bool,
    /// Whether decorative framing or large upper whitespace indicates a title/opening page.
    pub has_title_ornament_or_whitespace: bool,
    /// Aggregate typographic metrics for the page.
    pub typography: TypographyMetrics,
    /// Source pipeline path used (`"vector_pdf"` or `"raster_scan"`).
    pub source_kind: String,
}

//! Typographic metrics and Hebrew script-mode classification types.

use serde::{Deserialize, Serialize};

/// Dominant Hebrew script mode detected in a region or page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptMode {
    /// Standard square Hebrew script (Meruba — Ashurit / FrankRuehl / Vilna).
    Square,
    /// Semi-cursive Rashi script used for traditional commentaries.
    Rashi,
    /// Both Square and Rashi scripts present on the same page (classic Shas / Mikraot).
    Mixed,
    /// Script mode could not be reliably determined.
    #[default]
    Unknown,
}

/// Page-level typographic statistics extracted from vector fonts or raster connected components.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TypographyMetrics {
    /// Median line height (or font size) of the primary/central body text in points.
    pub primary_font_size_pt: Option<f32>,
    /// Median line height (or font size) of secondary/commentary/footnote text in points.
    pub secondary_font_size_pt: Option<f32>,
    /// Ratio of primary-to-secondary font/line size (`>= 1.0` when primary is larger).
    pub primary_to_secondary_ratio: Option<f32>,
    /// Maximum font/line size on the page relative to body median (for title detection).
    pub max_to_body_ratio: Option<f32>,
    /// Number of distinct font-size tiers clustered on the page.
    pub font_size_tier_count: usize,
    /// Detected script mode across the page.
    pub script_mode: ScriptMode,
    /// Whether distinct Square (center) vs Rashi (side) contrast was observed.
    pub has_square_vs_rashi_contrast: bool,
    /// Whether Hebrew cantillation (Ta'amim) or dense Nikud/Masoretic marks were detected.
    pub has_cantillation_or_masoretic_marks: bool,
}

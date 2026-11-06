use crate::numbering::{NumberingScheme, NumberingScope};
use crate::outline::HierarchyLevel;
use serde::{Deserialize, Serialize};

/// Target spread side for alternating running headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HeaderSpreadSide {
    #[default]
    Both,
    Recto, // Right-hand page in Hebrew (verso in LTR)
    Verso, // Left-hand page in Hebrew (recto in LTR)
}

/// Dynamic content extraction rule for running headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HeaderExtractionRule {
    /// Fixed static text (e.g. "ספר משנה תורה").
    Static(String),
    /// Current document title.
    DocumentTitle,
    /// Current section / chapter title.
    #[default]
    CurrentSectionTitle,
    /// Highest-level ancestor outline title matching a specific hierarchy level.
    OutlineLevelTitle(HierarchyLevel),
    /// Content of the first marker/anchor occurring on the current page (e.g. "סימן י״א").
    FirstMarkerOnPage,
    /// Content of the last marker/anchor occurring on the current page.
    LastMarkerOnPage,
    /// Range formatted from first to last marker on page (e.g. "סימן י״א – י״ג").
    FirstAndLastMarkerRange { separator: String },
}

/// Positioning of the folio (page number) within the header/footer band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HeaderFolioPosition {
    /// Outside margin (facing away from spine: right on recto, left on verso).
    #[default]
    OuterMargin,
    /// Inside spine gutter.
    InnerSpine,
    /// Centered horizontally.
    Centered,
    /// Omitted from running header band.
    Omit,
}

/// Complete running header configuration for facing pages in a book.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunningHeaderConfig {
    pub recto_rule: HeaderExtractionRule,
    pub verso_rule: HeaderExtractionRule,
    pub recto_folio: HeaderFolioPosition,
    pub verso_folio: HeaderFolioPosition,
    pub numbering_scheme: NumberingScheme,
    pub style_id: String,
    pub suppress_on_first_page_of_section: bool,
    pub height_pt: f32,
}

impl Default for RunningHeaderConfig {
    fn default() -> Self {
        Self {
            recto_rule: HeaderExtractionRule::CurrentSectionTitle,
            verso_rule: HeaderExtractionRule::DocumentTitle,
            recto_folio: HeaderFolioPosition::OuterMargin,
            verso_folio: HeaderFolioPosition::OuterMargin,
            numbering_scheme: NumberingScheme::HebrewGematria(
                crate::numbering::GematriaPunctuation::StandardPunctuation,
            ),
            style_id: "default-header".to_string(),
            suppress_on_first_page_of_section: true,
            height_pt: 18.0,
        }
    }
}

/// Apparatus and footnote configuration for large multi-stream books.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FootnoteApparatusConfig {
    #[serde(default = "default_footnote_id")]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_id: Option<String>,
    #[serde(default = "default_one_u32")]
    pub columns: u32,
    #[serde(default = "default_column_gap_pt")]
    pub column_gap_pt: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_suffix: Option<String>,
    pub numbering_scheme: NumberingScheme,
    pub scope: NumberingScope,
    pub continuous_numbering: bool,
    pub separator_rule_width_pt: f32,
    pub space_above_pt: f32,
    pub max_band_height_ratio: f32,
}

fn default_footnote_id() -> String {
    "footnote".to_string()
}
fn default_one_u32() -> u32 {
    1
}
fn default_column_gap_pt() -> f32 {
    12.0
}

impl Default for FootnoteApparatusConfig {
    fn default() -> Self {
        Self {
            id: default_footnote_id(),
            title: None,
            flow_id: Some(default_footnote_id()),
            columns: 1,
            column_gap_pt: 12.0,
            style_id: None,
            marker_prefix: None,
            marker_suffix: None,
            numbering_scheme: NumberingScheme::HebrewGematria(
                crate::numbering::GematriaPunctuation::StandardPunctuation,
            ),
            scope: NumberingScope::PerPage,
            continuous_numbering: false,
            separator_rule_width_pt: 0.5,
            space_above_pt: 8.0,
            max_band_height_ratio: 0.35, // Footnotes may occupy up to 35% of page
        }
    }
}

impl FootnoteApparatusConfig {
    pub fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            title: None,
            flow_id: Some(id_str),
            columns: 1,
            column_gap_pt: 12.0,
            style_id: None,
            marker_prefix: None,
            marker_suffix: None,
            ..Self::default()
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_flow_id(mut self, flow_id: impl Into<String>) -> Self {
        self.flow_id = Some(flow_id.into());
        self
    }

    pub fn with_columns(mut self, columns: u32, gap_pt: f32) -> Self {
        self.columns = columns.max(1);
        self.column_gap_pt = gap_pt;
        self
    }

    pub fn with_numbering(mut self, scheme: NumberingScheme) -> Self {
        self.numbering_scheme = scheme;
        self
    }

    pub fn with_marker_delimiters(
        mut self,
        prefix: impl Into<String>,
        suffix: impl Into<String>,
    ) -> Self {
        self.marker_prefix = Some(prefix.into());
        self.marker_suffix = Some(suffix.into());
        self
    }

    /// Formats footnote marker string for a 1-based index (e.g. "[א]", "(1)").
    pub fn format_marker(&self, index: usize) -> String {
        let core = crate::numbering::NumberingFormatter::format(&self.numbering_scheme, index);
        let prefix = self.marker_prefix.as_deref().unwrap_or("");
        let suffix = self.marker_suffix.as_deref().unwrap_or("");
        format!("{}{}{}", prefix, core, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_header_config_defaults() {
        let config = RunningHeaderConfig::default();
        assert!(config.suppress_on_first_page_of_section);
        assert_eq!(config.recto_folio, HeaderFolioPosition::OuterMargin);
        assert_eq!(config.verso_folio, HeaderFolioPosition::OuterMargin);
        assert_eq!(config.height_pt, 18.0);
    }

    #[test]
    fn test_apparatus_config_serialization() {
        let apparatus = FootnoteApparatusConfig {
            numbering_scheme: NumberingScheme::Arabic,
            scope: NumberingScope::PerSection,
            continuous_numbering: true,
            separator_rule_width_pt: 1.0,
            space_above_pt: 10.0,
            max_band_height_ratio: 0.40,
            ..Default::default()
        };
        let json = serde_json::to_string(&apparatus).unwrap();
        let loaded: FootnoteApparatusConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.max_band_height_ratio, 0.40);
        assert_eq!(loaded.scope, NumberingScope::PerSection);
        assert_eq!(loaded.id, "footnote");
        assert_eq!(loaded.columns, 1);
    }

    #[test]
    fn test_multi_apparatus_builder_and_marker_formatting() {
        let sources = FootnoteApparatusConfig::new("sources")
            .with_title("מקורות")
            .with_columns(2, 10.0)
            .with_numbering(NumberingScheme::HebrewGematria(
                crate::numbering::GematriaPunctuation::PlainLetters,
            ))
            .with_marker_delimiters("[", "]");

        assert_eq!(sources.id, "sources");
        assert_eq!(sources.columns, 2);
        assert_eq!(sources.column_gap_pt, 10.0);
        assert_eq!(sources.format_marker(1), "[א]");
        assert_eq!(sources.format_marker(2), "[ב]");

        let biurim = FootnoteApparatusConfig::new("biurim")
            .with_title("ביאורים")
            .with_columns(1, 12.0)
            .with_numbering(NumberingScheme::Arabic)
            .with_marker_delimiters("(", ")");

        assert_eq!(biurim.id, "biurim");
        assert_eq!(biurim.columns, 1);
        assert_eq!(biurim.format_marker(1), "(1)");
        assert_eq!(biurim.format_marker(2), "(2)");
    }
}

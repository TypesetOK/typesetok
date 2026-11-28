//! Output schema for layout classification and TypesetOK section configuration.

use crate::profile::PageSpatialProfile;
use crate::taxonomy::PageForm;
use serde::{Deserialize, Serialize};
use tok_typeset::layout_family::LayoutFamily;
use tok_typeset::multi_flow::FlowPlacementRole;

/// Serializable role assignment for a detected text stream in a synthesized section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedFlowRole {
    /// Suggested flow identifier (e.g. `"main"`, `"rashi"`, `"tosafot"`, `"notes"`).
    pub flow_id: String,
    /// Placement role (`"Primary"`, `"InnerSpine"`, `"OuterMargin"`, `"BottomBand"`, `"Column(i)"`).
    pub role_name: String,
    /// Suggested normalized width proportion (when applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_ratio: Option<f32>,
}

impl RecommendedFlowRole {
    /// Converts a `tok_typeset::multi_flow::FlowPlacementRole` into a serializable recommendation entry.
    pub fn from_placement_role(
        flow_id: impl Into<String>,
        role: FlowPlacementRole,
        width_ratio: Option<f32>,
    ) -> Self {
        let role_name = match role {
            FlowPlacementRole::Primary => "Primary".to_string(),
            FlowPlacementRole::InnerSpine => "InnerSpine".to_string(),
            FlowPlacementRole::OuterMargin => "OuterMargin".to_string(),
            FlowPlacementRole::BottomBand => "BottomBand".to_string(),
            FlowPlacementRole::Column(i) => format!("Column({i})"),
        };
        Self {
            flow_id: flow_id.into(),
            role_name,
            width_ratio,
        }
    }
}

/// Suggested `tok_core::SectionNode` configuration values derived from the analyzed layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedSectionConfig {
    /// Value for `SectionNode.layout_kind`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_kind: Option<String>,
    /// Value for `SectionNode.column_proportions`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_proportions: Option<Vec<f32>>,
    /// Value for `SectionNode.column_count` (1..=6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_count: Option<usize>,
    /// Value for `SectionNode.column_gutter_pt`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_gutter_pt: Option<f32>,
    /// Value for `SectionNode.draw_column_rule`.
    pub draw_column_rule: bool,
    /// Recommended flow placement roles.
    pub flow_roles: Vec<RecommendedFlowRole>,
}

/// Alternative candidate layout with its confidence score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutAlternative {
    pub page_form: PageForm,
    pub family: LayoutFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_type: Option<String>,
    pub confidence: f32,
}

/// Final document-level layout recommendation returned to `tok-engine-service`, `tok-desktop`, and `tok-ui`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRecommendation {
    /// Dominant classified page form across sampled content pages.
    pub page_form: PageForm,
    /// Target `tok_typeset::LayoutFamily` variant.
    pub recommended_family: LayoutFamily,
    /// Target `tok-ui` `TemplateType` (`"prose" | "notes" | "gemara" | "mikraot" | "bulletin"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_template_type: Option<String>,
    /// Human-readable Hebrew label for UI display.
    pub display_name_he: String,
    /// Overall confidence score in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Ranked alternative classifications for borderline cases.
    pub alternatives: Vec<LayoutAlternative>,
    /// Ready-to-apply `SectionNode` configuration.
    pub suggested_section_config: SuggestedSectionConfig,
    /// Number of pages analyzed (excluding filtered outliers, or total sampled).
    pub analyzed_page_count: usize,
    /// Number of pages filtered out as title/opening/blank outliers.
    pub outlier_page_count: usize,
    /// Whether Recto/Verso mirror symmetry was verified across facing pages.
    pub recto_verso_verified: bool,
    /// Whether the optional ML ONNX fallback stage was invoked on any page.
    pub used_ml_fallback: bool,
    /// Per-page spatial profiles (optional diagnostic payload).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub page_profiles: Vec<PageSpatialProfile>,
}

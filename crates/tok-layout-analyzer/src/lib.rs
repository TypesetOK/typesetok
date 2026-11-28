//! `tok-layout-analyzer` — Page Layout & Form Recognition Engine for TypesetOK.
//!
//! Implements a two-stage cascaded hybrid pipeline:
//! 1. Deterministic geometric analysis (PDF vector stream extraction, XY-Cut, projection profiles,
//!    L-shape detection, and Hebrew script heuristics).
//! 2. Optional local ONNX Runtime (`ort`) ML fallback using Apache-2.0 `PP-DocLayout-S` for
//!    noisy or historical scanned documents.
//!
//! Outputs [`LayoutRecommendation`] mapped directly to `tok_typeset::LayoutFamily`,
//! `tok_core::SectionNode`, and `tok-ui` `TemplateType`.

pub mod error;
pub mod geometry;
pub mod input;
pub mod profile;
pub mod recommendation;
pub mod spread;
pub mod taxonomy;
pub mod typography;

pub use error::{AnalyzerError, AnalyzerResult};
pub use geometry::NormalizedBBox;
pub use input::{AnalysisInput, AnalysisOptions, PageRange};
pub use profile::{DetectedRegion, PageSpatialProfile, RegionClass};
pub use recommendation::{
    LayoutAlternative, LayoutRecommendation, RecommendedFlowRole, SuggestedSectionConfig,
};
pub use spread::SpreadSide;
pub use taxonomy::PageForm;
pub use typography::{ScriptMode, TypographyMetrics};

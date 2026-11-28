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

pub mod augment;
pub mod classifier_geom;
pub mod components;
pub mod context;
pub mod error;
pub mod features;
pub mod geometry;
pub mod hybrid;
pub mod input;
pub mod lshape;
pub mod ml_labels;
pub mod ml_onnx;
pub mod noise_filter;
pub mod pdf_vector;
pub mod pipeline;
pub mod preprocess;
pub mod profile;
pub mod projection;
pub mod raster;
pub mod recommendation;
pub mod rules;
pub mod script_classifier;
pub mod spread;
pub mod spread_split;
pub mod synthetic;
pub mod synthesizer;
pub mod taxonomy;
pub mod typography;
pub mod xycut;

pub use error::{AnalyzerError, AnalyzerResult};
pub use geometry::NormalizedBBox;
pub use input::{AnalysisInput, AnalysisOptions, PageRange};
pub use pipeline::analyze_document_pages;
pub use profile::{DetectedRegion, PageSpatialProfile, RegionClass};
pub use raster::GrayImage;
pub use recommendation::{
    LayoutAlternative, LayoutRecommendation, RecommendedFlowRole, SuggestedSectionConfig,
};
pub use spread::SpreadSide;
pub use taxonomy::PageForm;
pub use typography::{ScriptMode, TypographyMetrics};

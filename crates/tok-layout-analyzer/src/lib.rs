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

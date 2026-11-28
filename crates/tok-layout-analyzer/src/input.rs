//! Input specification and tuning options for layout analysis.

use crate::raster::GrayImage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Inclusive 1-based page range to sample during document analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRange {
    /// First 1-based page to analyze.
    pub start_page: usize,
    /// Last 1-based page to analyze (inclusive).
    pub end_page: usize,
}

impl Default for PageRange {
    fn default() -> Self {
        Self {
            start_page: 1,
            end_page: 10,
        }
    }
}

/// Source input passed to `analyze_document_pages`.
#[derive(Debug, Clone)]
pub enum AnalysisInput {
    /// Path to a PDF or image file on disk.
    FilePath(PathBuf),
    /// In-memory PDF bytes.
    PdfBytes {
        bytes: Vec<u8>,
        filename_hint: Option<String>,
    },
    /// One or more pre-decoded grayscale page images.
    RasterPages(Vec<GrayImage>),
}

/// Tuning parameters for the two-stage hybrid analysis pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Page range to sample for PDF inputs (defaults to pages 1..=10).
    pub page_range: PageRange,
    /// Target rasterization DPI when rendering PDF pages (150-200 DPI recommended).
    pub target_dpi: u16,
    /// Confidence threshold above which Stage 1 (deterministic geometric) returns directly.
    pub fast_path_confidence_threshold: f32,
    /// Whether to enable automatic two-page spread splitting when `W > H`.
    pub auto_split_spreads: bool,
    /// Whether to enable Stage 2 ONNX fallback (when compiled with `ml-onnx` feature).
    pub enable_ml_fallback: bool,
    /// Optional path to `pp_doclayout_s.onnx` weights file.
    pub onnx_model_path: Option<PathBuf>,
    /// Whether to include per-page `PageSpatialProfile` entries in the returned recommendation.
    pub include_page_profiles: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            page_range: PageRange::default(),
            target_dpi: 150,
            fast_path_confidence_threshold: 0.85,
            auto_split_spreads: true,
            enable_ml_fallback: true,
            onnx_model_path: None,
            include_page_profiles: false,
        }
    }
}

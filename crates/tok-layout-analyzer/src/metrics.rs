//! Evaluation metrics and target acceptance thresholds for `tok-layout-analyzer`.
//!
//! Tracks Macro-F1, TzuratHaDaf vs ParallelColumns confusion rate, mAP50:95,
//! CPU latency, and peak RAM targets from the architecture specification.

use serde::{Deserialize, Serialize};

/// Target acceptance thresholds defined by the TypesetOK architecture specification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AcceptanceThresholds {
    /// Minimum Macro-F1 on clean vector PDFs (`>= 0.94`).
    pub min_macro_f1_vector: f32,
    /// Minimum Macro-F1 on 150-300 DPI scanned pages (`>= 0.88`).
    pub min_macro_f1_scan: f32,
    /// Maximum confusion rate between `TzuratHaDaf` and `ParallelColumns` (`< 0.04`).
    pub max_tzurat_parallel_confusion: f32,
    /// Minimum region-detection mAP50:95 for ML stage (`>= 0.82`).
    pub min_region_map_50_95: f32,
    /// Maximum per-page CPU latency for deterministic fast-path in ms (`< 35.0`).
    pub max_vector_latency_ms: f32,
    /// Maximum per-page CPU latency for raster + ONNX path in ms (`< 180.0`).
    pub max_raster_onnx_latency_ms: f32,
    /// Maximum peak additional RAM in MB (`< 450.0`).
    pub max_peak_ram_mb: f32,
}

impl Default for AcceptanceThresholds {
    fn default() -> Self {
        Self {
            min_macro_f1_vector: 0.94,
            min_macro_f1_scan: 0.88,
            max_tzurat_parallel_confusion: 0.04,
            min_region_map_50_95: 0.82,
            max_vector_latency_ms: 35.0,
            max_raster_onnx_latency_ms: 180.0,
            max_peak_ram_mb: 450.0,
        }
    }
}

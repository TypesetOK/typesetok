//! Error types for the `tok-layout-analyzer` crate.

use thiserror::Error;

/// Errors produced during page layout analysis, preprocessing, or ML inference.
#[derive(Debug, Error)]
pub enum AnalyzerError {
    /// Failed to read input file or stream.
    #[error("I/O error during layout analysis: {0}")]
    Io(#[from] std::io::Error),

    /// Input document or image bytes are empty, truncated, or malformed.
    #[error("Invalid or unsupported input format: {0}")]
    InvalidInput(String),

    /// PDF parsing or vector stream extraction failed.
    #[error("PDF extraction error: {0}")]
    PdfExtraction(String),

    /// Raster image decoding or preprocessing failed.
    #[error("Raster processing error: {0}")]
    RasterProcessing(String),

    /// Requested page range is out of bounds or produced no valid pages.
    #[error("Empty or invalid page range: {0}")]
    InvalidPageRange(String),

    /// Optional ONNX Runtime inference failed or model was not found.
    #[error("ML inference error: {0}")]
    MlInference(String),

    /// Serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convenience result alias for layout analysis operations.
pub type AnalyzerResult<T> = Result<T, AnalyzerError>;

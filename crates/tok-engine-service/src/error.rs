use thiserror::Error;

/// Unified error type returned by `tok-engine-service` operations.
#[derive(Debug, Error)]
pub enum EngineServiceError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("PDF generation error: {0}")]
    Pdf(String),

    #[error("Document parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<tok_storage::error::StorageError> for EngineServiceError {
    fn from(err: tok_storage::error::StorageError) -> Self {
        Self::Storage(err.to_string())
    }
}

impl From<tok_pdf::pdf_engine::PdfError> for EngineServiceError {
    fn from(err: tok_pdf::pdf_engine::PdfError) -> Self {
        Self::Pdf(err.to_string())
    }
}

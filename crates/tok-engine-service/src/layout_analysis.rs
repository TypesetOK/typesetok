//! Layout analysis service facade for `tok-cli` and `tok-desktop`.

use crate::error::EngineServiceError;
use std::path::Path;
use tok_core::model::SectionNode;
use tok_layout_analyzer::{
    analyze_document_pages, AnalysisInput, AnalysisOptions, LayoutRecommendation, PageRange,
};

/// Analyzes a document file (PDF or raster image) and returns a [`LayoutRecommendation`]
/// ready for `SectionNode` configuration or UI template pre-selection.
pub fn recommend_layout_from_file(
    path: impl AsRef<Path>,
    start_page: usize,
    end_page: usize,
) -> Result<LayoutRecommendation, EngineServiceError> {
    let options = AnalysisOptions {
        page_range: PageRange {
            start_page: start_page.max(1),
            end_page: end_page.max(start_page.max(1)),
        },
        ..AnalysisOptions::default()
    };
    analyze_document_pages(AnalysisInput::FilePath(path.as_ref().to_path_buf()), &options)
        .map_err(|e| EngineServiceError::Validation(e.to_string()))
}

/// Analyzes a batch of file paths and returns `(path_string, Result<LayoutRecommendation, String>)` pairs.
pub fn recommend_layouts_for_batch(
    paths: &[String],
    start_page: usize,
    end_page: usize,
) -> Vec<(String, Result<LayoutRecommendation, String>)> {
    paths
        .iter()
        .map(|p| {
            let res = recommend_layout_from_file(p, start_page, end_page)
                .map_err(|e| e.to_string());
            (p.clone(), res)
        })
        .collect()
}

/// Applies a [`LayoutRecommendation`]'s `SuggestedSectionConfig` onto a `tok_core::SectionNode`.
pub fn apply_recommendation_to_section(
    section: &mut SectionNode,
    rec: &LayoutRecommendation,
) {
    let cfg = &rec.suggested_section_config;
    if let Some(kind) = &cfg.layout_kind {
        section.layout_kind = Some(kind.clone());
    }
    section.column_proportions = cfg.column_proportions.clone();
    section.column_count = cfg.column_count;
    section.column_gutter_pt = cfg.column_gutter_pt;
    section.draw_column_rule = cfg.draw_column_rule;
}

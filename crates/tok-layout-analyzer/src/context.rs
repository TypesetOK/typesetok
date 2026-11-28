//! Document-level multi-page context aggregation:
//! 1. Outlier filtering (`TitleOrOutlierPage` and low-coverage `< 0.25` pages).
//! 2. Confidence-weighted majority voting across content pages.
//! 3. Recto / Verso mirror-symmetry parity verification for Tzurat HaDaf (+0.15 confidence boost).

use crate::hybrid::HybridPageResult;
use crate::profile::{PageSpatialProfile, RegionClass};
use crate::taxonomy::PageForm;
use std::collections::HashMap;

/// Aggregated multi-page decision before final [`LayoutRecommendation`] synthesis.
#[derive(Debug, Clone)]
pub struct MultiPageAggregation {
    pub dominant_form: PageForm,
    pub confidence: f32,
    pub ranked_alternatives: Vec<(PageForm, f32)>,
    pub analyzed_page_count: usize,
    pub outlier_page_count: usize,
    pub recto_verso_verified: bool,
    pub representative_profile: Option<PageSpatialProfile>,
}

/// Aggregates per-page hybrid results into a document-level classification.
pub fn aggregate_document_context(pages: &[HybridPageResult]) -> MultiPageAggregation {
    if pages.is_empty() {
        return MultiPageAggregation {
            dominant_form: PageForm::StandardProse,
            confidence: 0.0,
            ranked_alternatives: Vec::new(),
            analyzed_page_count: 0,
            outlier_page_count: 0,
            recto_verso_verified: false,
            representative_profile: None,
        };
    }

    let mut content_pages: Vec<&HybridPageResult> = Vec::new();
    let mut outlier_count = 0usize;

    for p in pages {
        let is_low_coverage = p.profile.text_coverage < 0.25;
        let is_title = p.classification.best_form == PageForm::TitleOrOutlierPage;
        if is_low_coverage || is_title {
            outlier_count += 1;
        } else {
            content_pages.push(p);
        }
    }

    // If all pages were outliers (e.g. single title-page document), fall back to all pages.
    if content_pages.is_empty() {
        content_pages = pages.iter().collect();
        outlier_count = 0;
    }

    // Confidence-weighted majority voting across non-outlier content pages.
    let mut vote_weights: HashMap<PageForm, f32> = HashMap::new();
    let mut total_weight = 0.0_f32;
    for p in &content_pages {
        let w = p.classification.confidence.max(0.05);
        *vote_weights
            .entry(p.classification.best_form)
            .or_insert(0.0) += w;
        total_weight += w;
    }

    let mut ranked: Vec<(PageForm, f32)> = vote_weights
        .into_iter()
        .map(|(form, w)| (form, (w / total_weight.max(1e-4)).clamp(0.0, 1.0)))
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));

    let dominant_form = ranked
        .first()
        .map(|(f, _)| *f)
        .unwrap_or(PageForm::StandardProse);
    let mut confidence = ranked.first().map(|(_, c)| *c).unwrap_or(0.5);

    // Recto/Verso mirror-symmetry parity check for Tzurat HaDaf.
    let recto_verso_verified = if dominant_form == PageForm::TzuratHaDaf {
        verify_recto_verso_mirror_symmetry(&content_pages)
    } else {
        false
    };

    if recto_verso_verified {
        confidence = (confidence + 0.15).min(0.99);
    }

    let representative_profile = content_pages
        .iter()
        .filter(|p| p.classification.best_form == dominant_form)
        .max_by(|a, b| {
            a.classification
                .confidence
                .total_cmp(&b.classification.confidence)
        })
        .or_else(|| content_pages.first())
        .map(|p| p.profile.clone());

    let alternatives = ranked.into_iter().skip(1).take(2).collect();

    MultiPageAggregation {
        dominant_form,
        confidence,
        ranked_alternatives: alternatives,
        analyzed_page_count: content_pages.len(),
        outlier_page_count: outlier_count,
        recto_verso_verified,
        representative_profile,
    }
}

/// Checks whether odd (Recto) and even (Verso) pages swap their narrow vs wide side columns
/// across the spine, confirming authentic Tzurat HaDaf facing-page parity.
pub fn verify_recto_verso_mirror_symmetry(pages: &[&HybridPageResult]) -> bool {
    let mut odd_left_narrower: Option<bool> = None;
    let mut even_left_narrower: Option<bool> = None;

    for p in pages {
        let side_cols = find_outer_side_column_widths(&p.profile);
        if let Some((left_w, right_w)) = side_cols {
            if (left_w - right_w).abs() < 0.02 {
                continue;
            }
            let left_is_narrower = left_w < right_w;
            if p.profile.page_index % 2 == 1 {
                odd_left_narrower = Some(left_is_narrower);
            } else {
                even_left_narrower = Some(left_is_narrower);
            }
        }
    }

    match (odd_left_narrower, even_left_narrower) {
        (Some(odd_val), Some(even_val)) => odd_val != even_val,
        _ => false,
    }
}

fn find_outer_side_column_widths(profile: &PageSpatialProfile) -> Option<(f32, f32)> {
    let mut text_cols: Vec<_> = profile
        .regions
        .iter()
        .filter(|r| matches!(r.region_class, RegionClass::Text) && r.bbox.height() >= 0.15)
        .collect();
    if text_cols.len() < 3 {
        return None;
    }
    text_cols.sort_by(|a, b| a.bbox.x0.total_cmp(&b.bbox.x0));
    let leftmost = text_cols.first()?.bbox.width();
    let rightmost = text_cols.last()?.bbox.width();
    Some((leftmost, rightmost))
}

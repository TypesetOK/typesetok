//! Stage 1: Deterministic geometric & typographic classifier (Fast-Path).
//!
//! Scores all six [`PageForm`] classes from a [`PageSpatialProfile`] and returns
//! ranked candidates with confidence scores in `[0.0, 1.0]`.

use crate::profile::PageSpatialProfile;
use crate::taxonomy::PageForm;

/// Ranked classification output for a single page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageClassification {
    pub best_form: PageForm,
    pub confidence: f32,
    pub ranked: Vec<(PageForm, f32)>,
}

/// Classifies a single page deterministically from its [`PageSpatialProfile`].
pub fn classify_page_geometric(profile: &PageSpatialProfile) -> PageClassification {
    let mut scores = [
        (PageForm::StandardProse, score_standard_prose(profile)),
        (PageForm::ProseWithFootnotes, score_prose_with_footnotes(profile)),
        (PageForm::TzuratHaDaf, score_tzurat_hadaf(profile)),
        (PageForm::MultiCommentaryGrid, score_multi_commentary(profile)),
        (PageForm::MultiColumnModern, score_multi_column_modern(profile)),
        (PageForm::TitleOrOutlierPage, score_title_or_outlier(profile)),
    ];

    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    let best_form = scores[0].0;
    let best_score = scores[0].1.clamp(0.0, 1.0);
    let second_score = scores[1].1.clamp(0.0, 1.0);

    // Calibrate confidence by combining raw fit score and margin over runner-up.
    let margin = (best_score - second_score).clamp(0.0, 0.5);
    let confidence = (best_score * 0.78 + margin * 0.44).clamp(0.05, 0.99);

    PageClassification {
        best_form,
        confidence,
        ranked: scores.to_vec(),
    }
}

fn score_standard_prose(p: &PageSpatialProfile) -> f32 {
    let mut s = 0.0_f32;
    if p.effective_column_count == 1 {
        s += 0.55;
    }
    if p.text_coverage >= 0.30 {
        s += 0.15;
    }
    if !p.has_footnote_rule_and_band {
        s += 0.15;
    }
    if !p.has_l_shape_signature {
        s += 0.10;
    }
    if p.horizontal_asymmetry < 0.25 {
        s += 0.05;
    }
    s
}

fn score_prose_with_footnotes(p: &PageSpatialProfile) -> f32 {
    let mut s = 0.0_f32;
    if p.has_footnote_rule_and_band {
        s += 0.58;
    }
    if p.effective_column_count <= 2 {
        s += 0.15;
    }
    if let Some(ratio) = p.typography.primary_to_secondary_ratio {
        if ratio >= 1.22 {
            s += 0.20;
        }
    }
    if p.text_coverage >= 0.30 {
        s += 0.07;
    }
    s
}

fn score_tzurat_hadaf(p: &PageSpatialProfile) -> f32 {
    let mut s = 0.0_f32;
    if p.has_l_shape_signature {
        s += 0.48;
    }
    if p.effective_column_count == 3 {
        s += 0.22;
    } else if p.effective_column_count == 2 && p.has_l_shape_signature {
        s += 0.10;
    }
    if p.typography.has_square_vs_rashi_contrast {
        s += 0.22;
    }
    if let Some(ratio) = p.typography.primary_to_secondary_ratio {
        if ratio >= 1.18 {
            s += 0.08;
        }
    }
    s
}

fn score_multi_commentary(p: &PageSpatialProfile) -> f32 {
    let mut s = 0.0_f32;
    if p.effective_column_count >= 4 {
        s += 0.52;
    }
    if p.typography.font_size_tier_count >= 3 {
        s += 0.22;
    }
    if p.typography.has_cantillation_or_masoretic_marks {
        s += 0.16;
    }
    if p.typography.has_square_vs_rashi_contrast {
        s += 0.10;
    }
    s
}

fn score_multi_column_modern(p: &PageSpatialProfile) -> f32 {
    let mut s = 0.0_f32;
    if (2..=6).contains(&p.effective_column_count) {
        s += 0.45;
    }
    if !p.has_l_shape_signature && !p.typography.has_square_vs_rashi_contrast {
        s += 0.25;
    }
    if p.has_spanning_header {
        s += 0.18;
    }
    if p.estimated_gutter_ratio >= 0.015 {
        s += 0.10;
    }
    s
}

fn score_title_or_outlier(p: &PageSpatialProfile) -> f32 {
    let mut s = 0.0_f32;
    if p.has_title_ornament_or_whitespace {
        s += 0.55;
    }
    if p.text_coverage < 0.28 {
        s += 0.30;
    }
    if let Some(max_ratio) = p.typography.max_to_body_ratio {
        if max_ratio >= 1.75 {
            s += 0.15;
        }
    }
    s
}

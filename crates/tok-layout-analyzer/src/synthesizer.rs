//! Synthesizes [`LayoutRecommendation`] mapped to `tok_typeset::LayoutFamily`,
//! `tok_core::SectionNode`, and `tok_typeset::multi_flow::FlowPlacementRole`.

use crate::context::MultiPageAggregation;
use crate::profile::PageSpatialProfile;
use crate::recommendation::{
    LayoutAlternative, LayoutRecommendation, RecommendedFlowRole, SuggestedSectionConfig,
};
use crate::taxonomy::PageForm;
use tok_core::FlowId;
use tok_typeset::layout_family::LayoutFamily;
use tok_typeset::multi_flow::FlowPlacementRole;

/// Builds a complete [`LayoutRecommendation`] from multi-page aggregation results.
pub fn synthesize_recommendation(
    agg: &MultiPageAggregation,
    used_ml_fallback: bool,
    page_profiles: Vec<PageSpatialProfile>,
) -> LayoutRecommendation {
    let profile_ref = agg.representative_profile.as_ref();
    let recommended_family = map_form_to_layout_family(agg.dominant_form, profile_ref);
    let suggested_section_config = build_section_config(agg.dominant_form, profile_ref);

    let alternatives = agg
        .ranked_alternatives
        .iter()
        .map(|&(form, conf)| LayoutAlternative {
            page_form: form,
            family: map_form_to_layout_family(form, profile_ref),
            template_type: form.to_ui_template_type().map(str::to_string),
            confidence: conf,
        })
        .collect();

    LayoutRecommendation {
        page_form: agg.dominant_form,
        recommended_family,
        recommended_template_type: agg
            .dominant_form
            .to_ui_template_type()
            .map(str::to_string),
        display_name_he: agg.dominant_form.display_name_he().to_string(),
        confidence: agg.confidence,
        alternatives,
        suggested_section_config,
        analyzed_page_count: agg.analyzed_page_count,
        outlier_page_count: agg.outlier_page_count,
        recto_verso_verified: agg.recto_verso_verified,
        used_ml_fallback,
        page_profiles,
    }
}

/// Maps a [`PageForm`] + optional [`PageSpatialProfile`] to a `tok_typeset::LayoutFamily`.
pub fn map_form_to_layout_family(
    form: PageForm,
    profile: Option<&PageSpatialProfile>,
) -> LayoutFamily {
    match form {
        PageForm::StandardProse | PageForm::TitleOrOutlierPage => LayoutFamily::SingleFlow {
            flow_id: FlowId::main(),
        },
        PageForm::ProseWithFootnotes => LayoutFamily::FootnotesBand {
            primary_flow: FlowId::main(),
            footnote_flow: FlowId::new("notes"),
            column_flows: vec![FlowId::main()],
        },
        PageForm::TzuratHaDaf => LayoutFamily::TzuratHaDaf {
            primary_flow: FlowId::new("gemara"),
            spine_inner_flow: Some(FlowId::new("rashi")),
            spine_outer_flow: Some(FlowId::new("tosafot")),
            expansion_flow: Some(FlowId::new("tosafot")),
            has_bottom_band: profile.map(|p| p.has_footnote_rule_and_band).unwrap_or(false),
        },
        PageForm::MultiCommentaryGrid => {
            let count = profile
                .map(|p| p.effective_column_count.max(4))
                .unwrap_or(4);
            let proportions = profile
                .filter(|p| p.column_proportions.len() == count)
                .map(|p| p.column_proportions.clone());
            let flow_ids = (0..count)
                .map(|i| FlowId::new(format!("col_{}", i + 1)))
                .collect();
            LayoutFamily::ParallelColumns {
                column_count: count,
                proportions,
                flow_ids,
            }
        }
        PageForm::MultiColumnModern => {
            let count = profile
                .map(|p| p.effective_column_count.clamp(2, 6))
                .unwrap_or(2);
            let proportions = profile
                .filter(|p| p.column_proportions.len() == count)
                .map(|p| p.column_proportions.clone());
            let flow_ids = (0..count)
                .map(|i| FlowId::new(format!("col_{}", i + 1)))
                .collect();
            LayoutFamily::ParallelColumns {
                column_count: count,
                proportions,
                flow_ids,
            }
        }
    }
}

/// Builds suggested `SectionNode` configuration values for the chosen [`PageForm`].
pub fn build_section_config(
    form: PageForm,
    profile: Option<&PageSpatialProfile>,
) -> SuggestedSectionConfig {
    let gutter_pt = profile
        .map(|p| (p.estimated_gutter_ratio * 595.0).clamp(8.0, 28.0))
        .unwrap_or(12.0);

    match form {
        PageForm::StandardProse | PageForm::TitleOrOutlierPage => SuggestedSectionConfig {
            layout_kind: Some("single_flow".into()),
            column_proportions: None,
            column_count: Some(1),
            column_gutter_pt: None,
            draw_column_rule: false,
            flow_roles: vec![RecommendedFlowRole::from_placement_role(
                "main",
                FlowPlacementRole::Primary,
                Some(1.0),
            )],
        },
        PageForm::ProseWithFootnotes => SuggestedSectionConfig {
            layout_kind: Some("footnotes_band".into()),
            column_proportions: None,
            column_count: Some(1),
            column_gutter_pt: Some(gutter_pt),
            draw_column_rule: false,
            flow_roles: vec![
                RecommendedFlowRole::from_placement_role(
                    "main",
                    FlowPlacementRole::Primary,
                    Some(1.0),
                ),
                RecommendedFlowRole::from_placement_role(
                    "notes",
                    FlowPlacementRole::BottomBand,
                    Some(1.0),
                ),
            ],
        },
        PageForm::TzuratHaDaf => {
            let props = profile
                .filter(|p| p.column_proportions.len() == 3)
                .map(|p| p.column_proportions.clone())
                .unwrap_or_else(|| vec![0.26, 0.44, 0.30]);
            SuggestedSectionConfig {
                layout_kind: Some("tzurat_hadaf".into()),
                column_proportions: Some(props.clone()),
                column_count: Some(3),
                column_gutter_pt: Some(gutter_pt),
                draw_column_rule: false,
                flow_roles: vec![
                    RecommendedFlowRole::from_placement_role(
                        "rashi",
                        FlowPlacementRole::InnerSpine,
                        props.first().copied(),
                    ),
                    RecommendedFlowRole::from_placement_role(
                        "gemara",
                        FlowPlacementRole::Primary,
                        props.get(1).copied(),
                    ),
                    RecommendedFlowRole::from_placement_role(
                        "tosafot",
                        FlowPlacementRole::OuterMargin,
                        props.get(2).copied(),
                    ),
                ],
            }
        }
        PageForm::MultiCommentaryGrid => {
            let count = profile
                .map(|p| p.effective_column_count.max(4))
                .unwrap_or(4);
            let props = profile
                .filter(|p| p.column_proportions.len() == count)
                .map(|p| p.column_proportions.clone())
                .unwrap_or_else(|| vec![1.0 / count as f32; count]);
            let roles = (0..count)
                .map(|i| {
                    RecommendedFlowRole::from_placement_role(
                        format!("col_{}", i + 1),
                        FlowPlacementRole::Column(i),
                        props.get(i).copied(),
                    )
                })
                .collect();
            SuggestedSectionConfig {
                layout_kind: Some("parallel_columns".into()),
                column_proportions: Some(props),
                column_count: Some(count),
                column_gutter_pt: Some(gutter_pt),
                draw_column_rule: false,
                flow_roles: roles,
            }
        }
        PageForm::MultiColumnModern => {
            let count = profile
                .map(|p| p.effective_column_count.clamp(2, 6))
                .unwrap_or(2);
            let props = profile
                .filter(|p| p.column_proportions.len() == count)
                .map(|p| p.column_proportions.clone())
                .unwrap_or_else(|| vec![1.0 / count as f32; count]);
            let roles = (0..count)
                .map(|i| {
                    RecommendedFlowRole::from_placement_role(
                        format!("col_{}", i + 1),
                        FlowPlacementRole::Column(i),
                        props.get(i).copied(),
                    )
                })
                .collect();
            SuggestedSectionConfig {
                layout_kind: Some("parallel_columns".into()),
                column_proportions: Some(props),
                column_count: Some(count),
                column_gutter_pt: Some(gutter_pt),
                draw_column_rule: count >= 2,
                flow_roles: roles,
            }
        }
    }
}

//! Cascaded two-stage hybrid fusion (Stage 1 Geometric + Stage 2 ONNX ML + Talmud topology rules).

use crate::classifier_geom::{classify_page_geometric, PageClassification};
use crate::features::extract_page_spatial_profile;
use crate::ml_onnx::OnnxLayoutDetector;
use crate::noise_filter::filter_noise_and_tag_marginalia;
use crate::profile::{DetectedRegion, PageSpatialProfile};
use crate::raster::GrayImage;
use crate::rules::detect_horizontal_rules;

/// Result of running the cascaded hybrid analyzer on a single page.
#[derive(Debug, Clone)]
pub struct HybridPageResult {
    pub profile: PageSpatialProfile,
    pub classification: PageClassification,
    pub used_ml_fallback: bool,
}

/// Runs Stage 1 geometric classification, and if confidence < `fast_path_threshold`
/// invokes Stage 2 ONNX detection (when available) and merges the resulting regions.
pub fn analyze_page_hybrid(
    page_index: usize,
    aspect_ratio: f32,
    stage1_regions: Vec<DetectedRegion>,
    raster_img: Option<&GrayImage>,
    ml_detector: Option<&mut OnnxLayoutDetector>,
    fast_path_threshold: f32,
    source_kind: &str,
) -> HybridPageResult {
    let cleaned_s1 = filter_noise_and_tag_marginalia(stage1_regions);
    let profile_s1 =
        extract_page_spatial_profile(page_index, aspect_ratio, cleaned_s1, source_kind);
    let class_s1 = classify_page_geometric(&profile_s1);

    if class_s1.confidence >= fast_path_threshold {
        return HybridPageResult {
            profile: profile_s1,
            classification: class_s1,
            used_ml_fallback: false,
        };
    }

    if let (Some(img), Some(detector)) = (raster_img, ml_detector) {
        if let Ok(mut ml_regions) = detector.detect(img) {
            if !ml_regions.is_empty() {
                // Supplement ML regions with deterministic horizontal rules for footnote accuracy.
                let mut rules = detect_horizontal_rules(img, 140);
                ml_regions.append(&mut rules);
                let merged = filter_noise_and_tag_marginalia(ml_regions);
                let profile_s2 = extract_page_spatial_profile(
                    page_index,
                    aspect_ratio,
                    merged,
                    format!("{source_kind}+ml_onnx"),
                );
                let class_s2 = classify_page_geometric(&profile_s2);
                if class_s2.confidence >= class_s1.confidence {
                    return HybridPageResult {
                        profile: profile_s2,
                        classification: class_s2,
                        used_ml_fallback: true,
                    };
                }
            }
        }
    }

    HybridPageResult {
        profile: profile_s1,
        classification: class_s1,
        used_ml_fallback: false,
    }
}

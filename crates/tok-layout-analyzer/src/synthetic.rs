//! Synthetic ground-truth annotation exporter (zero-cost self-bootstrapping).
//!
//! Converts `tok_typeset::geometry::PageLayoutBox` outputs directly into normalized
//! DocLayNet/COCO-style bounding-box annotations for training and evaluating
//! `PP-DocLayout-S` / RT-DETR models on Hebrew layouts (including Tzurat HaDaf).

use crate::geometry::NormalizedBBox;
use crate::profile::RegionClass;
use crate::taxonomy::PageForm;
use serde::{Deserialize, Serialize};
use tok_typeset::geometry::PageLayoutBox;

/// A single ground-truth region annotation extracted from `tok-typeset`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntheticAnnotation {
    pub bbox: NormalizedBBox,
    pub region_class: RegionClass,
    pub flow_id: String,
}

/// Complete ground-truth record for one rendered `PageLayoutBox`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntheticPageGroundTruth {
    pub page_index: usize,
    pub width_pt: f32,
    pub height_pt: f32,
    pub page_form: PageForm,
    pub annotations: Vec<SyntheticAnnotation>,
}

/// Converts a `tok-typeset` [`PageLayoutBox`] into zero-cost ground-truth annotations.
pub fn extract_ground_truth_from_layout(
    page: &PageLayoutBox,
    page_form: PageForm,
) -> SyntheticPageGroundTruth {
    let pw = page.dimensions.width.max(1.0);
    let ph = page.dimensions.height.max(1.0);
    let mut annotations = Vec::new();

    for frame in &page.frames {
        let bbox = NormalizedBBox::new(
            frame.rect.x / pw,
            frame.rect.y / ph,
            (frame.rect.x + frame.rect.width) / pw,
            (frame.rect.y + frame.rect.height) / ph,
        );
        let region_class = classify_flow_id_to_region(&frame.flow_id);
        annotations.push(SyntheticAnnotation {
            bbox,
            region_class,
            flow_id: frame.flow_id.clone(),
        });
    }

    for orn in &page.ornaments {
        annotations.push(SyntheticAnnotation {
            bbox: NormalizedBBox::new(
                orn.rect.x / pw,
                orn.rect.y / ph,
                (orn.rect.x + orn.rect.width) / pw,
                (orn.rect.y + orn.rect.height) / ph,
            ),
            region_class: RegionClass::Ornament,
            flow_id: orn.ornament_id.clone(),
        });
    }

    for img in &page.images {
        annotations.push(SyntheticAnnotation {
            bbox: NormalizedBBox::new(
                img.rect.x / pw,
                img.rect.y / ph,
                (img.rect.x + img.rect.width) / pw,
                (img.rect.y + img.rect.height) / ph,
            ),
            region_class: RegionClass::Figure,
            flow_id: img.asset_path.clone(),
        });
    }

    SyntheticPageGroundTruth {
        page_index: page.page_index,
        width_pt: pw,
        height_pt: ph,
        page_form,
        annotations,
    }
}

fn classify_flow_id_to_region(flow_id: &str) -> RegionClass {
    let lower = flow_id.to_lowercase();
    if lower.contains("note") || lower.contains("footnote") || lower.contains("bottom") {
        RegionClass::Footnote
    } else if lower.contains("header") {
        RegionClass::Header
    } else if lower.contains("footer") {
        RegionClass::Footer
    } else if lower.contains("title") {
        RegionClass::Title
    } else {
        RegionClass::Text
    }
}

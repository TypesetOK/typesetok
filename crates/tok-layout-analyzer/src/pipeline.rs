//! Top-level orchestration pipeline for `tok-layout-analyzer`.

use crate::components::analyze_region_components;
use crate::context::aggregate_document_context;
use crate::error::{AnalyzerError, AnalyzerResult};
use crate::hybrid::{analyze_page_hybrid, HybridPageResult};
use crate::input::{AnalysisInput, AnalysisOptions};
use crate::ml_onnx::OnnxLayoutDetector;
use crate::pdf_vector::{extract_vector_pages, has_vector_text_layer, vector_page_to_regions};
use crate::preprocess::{
    compute_content_bounds, deskew_image, normalize_to_content_box, remove_gutter_shadow,
    sauvola_binarize,
};
use crate::raster::GrayImage;
use crate::recommendation::LayoutRecommendation;
use crate::rules::detect_horizontal_rules;
use crate::script_classifier::classify_region_script;
use crate::spread_split::{detect_and_split_spread, SpreadSplitResult};
use crate::synthesizer::synthesize_recommendation;
use crate::typography::ScriptMode;
use crate::xycut::{segment_page_xycut, XyCutConfig};
use std::fs;

/// Analyzes one or more document pages (PDF or raster scans) and returns a [`LayoutRecommendation`].
pub fn analyze_document_pages(
    input: AnalysisInput,
    options: &AnalysisOptions,
) -> AnalyzerResult<LayoutRecommendation> {
    let mut ml_detector = if options.enable_ml_fallback {
        options
            .onnx_model_path
            .as_deref()
            .and_then(|p| OnnxLayoutDetector::load(p).ok())
    } else {
        None
    };

    let page_results: Vec<HybridPageResult> = match input {
        AnalysisInput::FilePath(path) => {
            let bytes = fs::read(&path)?;
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "pdf" || bytes.starts_with(b"%PDF-") {
                analyze_pdf_bytes(&bytes, options, ml_detector.as_mut())?
            } else {
                let img = GrayImage::from_pgm_bytes(&bytes)?;
                analyze_raster_pages(&[img], options, ml_detector.as_mut())?
            }
        }
        AnalysisInput::PdfBytes { bytes, .. } => {
            analyze_pdf_bytes(&bytes, options, ml_detector.as_mut())?
        }
        AnalysisInput::RasterPages(pages) => {
            analyze_raster_pages(&pages, options, ml_detector.as_mut())?
        }
    };

    if page_results.is_empty() {
        return Err(AnalyzerError::InvalidPageRange(
            "No analyzable pages found in input".into(),
        ));
    }

    let used_ml_fallback = page_results.iter().any(|r| r.used_ml_fallback);
    let agg = aggregate_document_context(&page_results);
    let profiles = if options.include_page_profiles {
        page_results.into_iter().map(|r| r.profile).collect()
    } else {
        Vec::new()
    };

    Ok(synthesize_recommendation(&agg, used_ml_fallback, profiles))
}

fn analyze_pdf_bytes(
    bytes: &[u8],
    options: &AnalysisOptions,
    ml_detector: Option<&mut OnnxLayoutDetector>,
) -> AnalyzerResult<Vec<HybridPageResult>> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(AnalyzerError::PdfExtraction(
            "Input bytes do not represent a valid PDF document".into(),
        ));
    }

    if has_vector_text_layer(bytes) {
        let vector_pages = extract_vector_pages(
            bytes,
            options.page_range.start_page,
            options.page_range.end_page,
        )?;
        let mut results = Vec::with_capacity(vector_pages.len());
        for vp in vector_pages {
            let aspect = (vp.width_pt / vp.height_pt.max(1.0)).max(0.1);
            let regions = vector_page_to_regions(&vp);
            let res = analyze_page_hybrid(
                vp.page_index,
                aspect,
                regions,
                None,
                None,
                options.fast_path_confidence_threshold,
                "vector_pdf",
            );
            results.push(res);
        }
        if !results.is_empty() {
            return Ok(results);
        }
    }

    // Fallback when PDF has no uncompressed vector text stream.
    let _ = ml_detector;
    Err(AnalyzerError::PdfExtraction(
        "PDF contains no extractable vector text layer; provide rasterized pages or enable PDF rasterizer".into(),
    ))
}

fn analyze_raster_pages(
    pages: &[GrayImage],
    options: &AnalysisOptions,
    mut ml_detector: Option<&mut OnnxLayoutDetector>,
) -> AnalyzerResult<Vec<HybridPageResult>> {
    if pages.is_empty() {
        return Err(AnalyzerError::InvalidInput("Empty raster page list".into()));
    }

    let mut single_pages: Vec<GrayImage> = Vec::new();
    for img in pages {
        if options.auto_split_spreads {
            match detect_and_split_spread(img)? {
                SpreadSplitResult::Single(p) => single_pages.push(p),
                SpreadSplitResult::Split {
                    right_page,
                    left_page,
                    ..
                } => {
                    single_pages.push(right_page);
                    single_pages.push(left_page);
                }
            }
        } else {
            single_pages.push(img.clone());
        }
    }

    let mut results = Vec::with_capacity(single_pages.len());
    for (idx, raw_img) in single_pages.iter().enumerate() {
        let page_index = idx + 1;
        let de_gutter = remove_gutter_shadow(raw_img)?;
        let (deskewed, _angle) = deskew_image(&de_gutter)?;
        let bin = sauvola_binarize(&deskewed, 15, 0.34)?;

        let bounds = compute_content_bounds(&bin, 140);
        let mut regions = segment_page_xycut(&bin, XyCutConfig::default());
        let mut rules = detect_horizontal_rules(&bin, 140);
        regions.append(&mut rules);

        for r in &mut regions {
            let stats = analyze_region_components(&bin, r.bbox, 140);
            if stats.count > 0 {
                r.median_line_height = Some(stats.median_height_norm);
                let script = classify_region_script(r, Some(&stats));
                if script == ScriptMode::Rashi {
                    r.is_rashi_script = Some(true);
                } else if script == ScriptMode::Square {
                    r.is_rashi_script = Some(false);
                }
            }
            r.bbox = normalize_to_content_box(r.bbox, bounds);
        }

        let res = analyze_page_hybrid(
            page_index,
            bin.aspect_ratio(),
            regions,
            Some(&bin),
            ml_detector.as_deref_mut(),
            options.fast_path_confidence_threshold,
            "raster_scan",
        );
        results.push(res);
    }

    Ok(results)
}

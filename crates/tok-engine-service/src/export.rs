use crate::error::EngineServiceError;
use std::collections::BTreeMap;
use tok_core::model::DocumentRoot;
use tok_pdf::color::IccProfileId;
use tok_pdf::html_projection::HtmlProjectionCompiler;
use tok_pdf::pdf_engine::{PdfExportOptions, PdfPrePressEngine, PdfXStandard};
use tok_pdf::preflight::{PreflightEngine, PreflightReport};
use tok_storage::package::{TokManifest, TokPackage};
use tok_typeset::font::FontManager;
use tok_typeset::geometry::PageLayoutBox;

/// Configuration options for PDF/X prepress export.
#[derive(Debug, Clone)]
pub struct PdfRenderRequest {
    pub standard: PdfXStandard,
    pub color_profile: IccProfileId,
    pub bleed_pt: f32,
    pub draw_crop_marks: bool,
    pub draw_registration_marks: bool,
}

impl Default for PdfRenderRequest {
    fn default() -> Self {
        Self {
            standard: PdfXStandard::PdfX1a2001,
            color_profile: IccProfileId::Fogra39,
            bleed_pt: 8.504,
            draw_crop_marks: true,
            draw_registration_marks: false,
        }
    }
}

/// Configuration options for prepress preflight evaluation.
#[derive(Debug, Clone)]
pub struct PreflightRequest {
    pub standard: PdfXStandard,
    pub bleed_pt: f32,
}

impl Default for PreflightRequest {
    fn default() -> Self {
        Self {
            standard: PdfXStandard::PdfX1a2001,
            bleed_pt: 8.504,
        }
    }
}

fn collect_package_media(
    assets_pkg: Option<&TokPackage>,
) -> (BTreeMap<String, Vec<u8>>, BTreeMap<String, String>) {
    let mut image_assets = BTreeMap::new();
    let mut ornament_assets = BTreeMap::new();
    if let Some(pkg) = assets_pkg {
        for (rel, bytes) in pkg.image_assets() {
            image_assets.insert(rel.to_string(), bytes.to_vec());
        }
        for (rel, bytes) in pkg.ornament_assets() {
            if let Ok(svg) = std::str::from_utf8(bytes) {
                ornament_assets.insert(rel.to_string(), svg.to_string());
            }
        }
    }
    (image_assets, ornament_assets)
}

/// Exports typeset [`PageLayoutBox`] pages into ISO PDF/X bytes.
pub fn render_pdf_bytes(
    doc: &DocumentRoot,
    manifest: &TokManifest,
    pages: &[PageLayoutBox],
    font_manager: &FontManager,
    assets_pkg: Option<&TokPackage>,
    req: &PdfRenderRequest,
) -> Result<Vec<u8>, EngineServiceError> {
    let (image_assets, ornament_assets) = collect_package_media(assets_pkg);
    let options = PdfExportOptions {
        standard: req.standard,
        color_profile: req.color_profile,
        title: doc.metadata.title.clone(),
        author: doc.metadata.author.clone(),
        bleed_pt: req.bleed_pt,
        slug_pt: if req.draw_crop_marks || req.draw_registration_marks {
            28.346
        } else {
            req.bleed_pt
        },
        draw_crop_marks: req.draw_crop_marks,
        draw_registration_marks: req.draw_registration_marks,
        draw_page_numbers: true,
        custom_font_data: None,
        creation_date: Some(manifest.created_at.clone()),
        mod_date: Some(manifest.modified_at.clone()),
        compress_streams: true,
        image_assets,
        ornament_assets,
    };

    Ok(PdfPrePressEngine::export_pdf_with_fonts(
        pages,
        &options,
        font_manager,
    )?)
}

/// Compiles typeset pages into pre-paginated HTML projection output.
pub fn render_html_string(pages: &[PageLayoutBox]) -> String {
    let (page_w_mm, page_h_mm) = pages
        .first()
        .map(|p| {
            (
                p.dimensions.width * 25.4 / 72.0,
                p.dimensions.height * 25.4 / 72.0,
            )
        })
        .unwrap_or((210.0, 297.0));
    HtmlProjectionCompiler::compile_to_html(pages, page_w_mm, page_h_mm)
}

/// Evaluates prepress preflight diagnostics against a document and its typeset pages.
pub fn evaluate_preflight(
    doc: &DocumentRoot,
    pages: &[PageLayoutBox],
    font_manager: &FontManager,
    assets_pkg: Option<&TokPackage>,
    req: &PreflightRequest,
) -> PreflightReport {
    let (image_assets, _) = collect_package_media(assets_pkg);
    let opts = PdfExportOptions {
        standard: req.standard,
        bleed_pt: req.bleed_pt,
        image_assets,
        ..PdfExportOptions::default()
    };
    PreflightEngine::evaluate(doc, pages, font_manager, &opts)
}

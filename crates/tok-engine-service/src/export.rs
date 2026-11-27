use crate::error::EngineServiceError;
use std::collections::BTreeMap;
use tok_core::model::DocumentRoot;
use tok_pdf::color::IccProfileId;
use tok_pdf::icc::RenderingIntent;
use tok_pdf::html_projection::HtmlProjectionCompiler;
use tok_pdf::pdf_engine::{PdfExportOptions, PdfPrePressEngine, PdfXStandard};
use tok_pdf::preflight::{PreflightEngine, PreflightReport};
use tok_storage::package::{TokManifest, TokPackage};
use tok_typeset::font::FontManager;
use tok_typeset::geometry::PageLayoutBox;

/// Maps a UI/CLI format id (`pdfx1a`, `pdfx4`, `pdf`) to a PDF standard.
/// Unknown ids return `None` so the caller keeps its default.
fn parse_standard(id: &str) -> Option<PdfXStandard> {
    match id.trim().to_ascii_lowercase().as_str() {
        "pdfx1a" | "pdfx1a2001" => Some(PdfXStandard::PdfX1a2001),
        "pdfx4" => Some(PdfXStandard::PdfX4),
        "pdf" => Some(PdfXStandard::Plain),
        _ => None,
    }
}

/// Configuration options for PDF/X prepress export.
#[derive(Debug, Clone)]
pub struct PdfRenderRequest {
    pub standard: PdfXStandard,
    pub color_profile: IccProfileId,
    pub bleed_pt: f32,
    pub draw_crop_marks: bool,
    pub draw_registration_marks: bool,
    /// Pages to export, 1-based and inclusive. `None` exports every page.
    pub page_range: Option<(usize, usize)>,
    /// Optional 2-up booklet / signature imposition with creep compensation.
    pub imposition: Option<tok_pdf::imposition::ImpositionConfig>,
    /// ICC rendering intent for color conversion.
    pub rendering_intent: RenderingIntent,
    /// Optional external ICC profile bytes overriding `color_profile`.
    pub external_icc_profile: Option<Vec<u8>>,
}

impl Default for PdfRenderRequest {
    fn default() -> Self {
        Self {
            standard: PdfXStandard::PdfX1a2001,
            color_profile: IccProfileId::Fogra39,
            bleed_pt: 8.504,
            draw_crop_marks: true,
            draw_registration_marks: false,
            page_range: None,
            imposition: None,
            rendering_intent: RenderingIntent::default(),
            external_icc_profile: None,
        }
    }
}

impl PdfRenderRequest {
    /// Limits the export to pages `from..=to` (1-based, inclusive).
    /// Missing or invalid bounds leave the export at every page.
    pub fn with_page_range(mut self, from: Option<usize>, to: Option<usize>) -> Self {
        if let (Some(from), Some(to)) = (from, to) {
            if from >= 1 && to >= from {
                self.page_range = Some((from, to));
            }
        }
        self
    }

    /// Parses UI/host export dialog options into a typed [`PdfRenderRequest`].
    pub fn from_ui_options(
        format: Option<&str>,
        color_profile: Option<&str>,
        bleed: Option<bool>,
        crop_marks: Option<bool>,
        registration_marks: Option<bool>,
    ) -> Self {
        let mut req = Self::default();
        if let Some(standard) = format.and_then(parse_standard) {
            req.standard = standard;
        }
        if let Some(profile) = color_profile {
            if !profile.trim().is_empty() {
                req.color_profile = IccProfileId::from_id_str(profile);
            }
        }
        if bleed == Some(false) {
            req.bleed_pt = 0.0;
        }
        if crop_marks == Some(false) {
            req.draw_crop_marks = false;
        }
        if registration_marks == Some(true) {
            req.draw_registration_marks = true;
        }
        req
    }
}

/// Configuration options for prepress preflight evaluation.
#[derive(Debug, Clone)]
pub struct PreflightRequest {
    pub standard: PdfXStandard,
    pub color_profile: IccProfileId,
    pub bleed_pt: f32,
}

impl Default for PreflightRequest {
    fn default() -> Self {
        Self {
            standard: PdfXStandard::PdfX1a2001,
            color_profile: IccProfileId::Fogra39,
            bleed_pt: 8.504,
        }
    }
}

impl PreflightRequest {
    /// Parses UI/host preflight options into a typed [`PreflightRequest`].
    pub fn from_ui_options(standard: Option<&str>, bleed: Option<bool>) -> Self {
        let mut req = Self::default();
        if let Some(parsed) = standard.and_then(parse_standard) {
            req.standard = parsed;
        }
        if bleed == Some(false) {
            req.bleed_pt = 0.0;
        }
        req
    }

    /// Sets the target ICC export profile for preflight color checks.
    pub fn with_color_profile(mut self, color_profile: IccProfileId) -> Self {
        self.color_profile = color_profile;
        self
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
    let selected: Vec<PageLayoutBox>;
    let pages = match req.page_range {
        Some((from, to)) => {
            selected = pages
                .iter()
                .skip(from - 1)
                .take(to - from + 1)
                .cloned()
                .collect();
            if selected.is_empty() {
                pages
            } else {
                selected.as_slice()
            }
        }
        None => pages,
    };
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
        imposition: req.imposition.clone(),
        rendering_intent: req.rendering_intent,
        external_icc_profile: req.external_icc_profile.clone(),
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
        color_profile: req.color_profile,
        bleed_pt: req.bleed_pt,
        image_assets,
        ..PdfExportOptions::default()
    };
    PreflightEngine::evaluate(doc, pages, font_manager, &opts)
}

#[cfg(test)]
mod page_range_tests {
    use super::*;

    #[test]
    fn page_range_needs_valid_one_based_bounds() {
        let req = PdfRenderRequest::default().with_page_range(Some(3), Some(5));
        assert_eq!(req.page_range, Some((3, 5)));
        for (from, to) in [
            (Some(0), Some(2)),
            (Some(4), Some(2)),
            (Some(1), None),
            (None, None),
        ] {
            let req = PdfRenderRequest::default().with_page_range(from, to);
            assert_eq!(req.page_range, None, "{from:?}..{to:?}");
        }
    }

    fn std_of(id: &str) -> PdfXStandard {
        let req = PdfRenderRequest::from_ui_options(Some(id), None, None, None, None);
        req.standard
    }

    #[test]
    fn format_ids_map_to_their_own_standard() {
        assert_eq!(std_of("pdfx1a"), PdfXStandard::PdfX1a2001);
        assert_eq!(std_of("pdfx4"), PdfXStandard::PdfX4);
        assert_eq!(std_of("pdf"), PdfXStandard::Plain);
        assert_eq!(std_of("???"), PdfXStandard::PdfX1a2001);
        let pf = PreflightRequest::from_ui_options(Some("pdf"), None);
        assert_eq!(pf.standard, PdfXStandard::Plain);
    }
}

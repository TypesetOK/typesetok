//! Real Pre-Press Preflight Evaluator (Single Source of Truth in Rust).
//!
//! Replaces placeholder UI status badges by evaluating actual font embedding/fsType/
//! Hebrew glyph coverage, ICC/GCR color limits (`TAC <= 300%`), Trim/Bleed geometry,
//! placed raster image effective DPI, 5-stage column balance diagnostics, and frame
//! overflow across the typeset `PageLayoutBox` list.

use crate::pdf_engine::{PdfExportOptions, PdfXStandard};
use serde::{Deserialize, Serialize};
use tok_core::model::DocumentRoot;
use tok_typeset::diagnostics::DiagnosticSeverity;
use tok_typeset::font::{FontManager, MissingFontDiagnostic};
use tok_typeset::geometry::PageLayoutBox;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PreflightStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightCheckItem {
    pub id: String,
    pub category: String,
    pub title: String,
    pub status: PreflightStatus,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreflightReport {
    pub overall_status: PreflightStatus,
    pub profile_name: String,
    pub pdfx_standard: String,
    pub total_pages: usize,
    pub checks: Vec<PreflightCheckItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_fonts: Vec<MissingFontDiagnostic>,
    pub low_dpi_images_count: usize,
    pub balance_warnings_count: usize,
}

pub struct PreflightEngine;

impl PreflightEngine {
    pub fn evaluate(
        doc: &DocumentRoot,
        pages: &[PageLayoutBox],
        font_mgr: &FontManager,
        options: &PdfExportOptions,
    ) -> PreflightReport {
        let mut checks = Vec::new();

        // 1. Font embedding, fsType license permissions, & Hebrew/Niqqud/Taamim glyph coverage
        let missing_fonts = font_mgr.find_missing_or_incomplete_fonts(doc);
        let mut font_details = Vec::new();
        let mut font_status = PreflightStatus::Pass;
        for diag in &missing_fonts {
            if diag.missing_family || diag.missing_hebrew_coverage {
                font_status = PreflightStatus::Fail;
            } else if font_status != PreflightStatus::Fail {
                font_status = PreflightStatus::Warning;
            }
            let mut flags = Vec::new();
            if diag.missing_family {
                flags.push("font not installed/embedded");
            }
            if diag.missing_hebrew_coverage {
                flags.push("missing Hebrew letters");
            }
            if diag.missing_niqqud_coverage {
                flags.push("missing Niqqud");
            }
            if diag.missing_taamim_coverage {
                flags.push("missing Taamim");
            }
            font_details.push(format!(
                "{} ({} paragraphs): {}",
                diag.family_name,
                diag.affected_paragraphs,
                flags.join(", ")
            ));
        }
        // Check registered external fonts for non-embeddable OS/2 fsType
        for (face_name, _) in font_mgr.external_fonts() {
            if let Some(rep) = font_mgr.inspect_registered_family(&face_name) {
                if !rep.is_embeddable {
                    font_status = PreflightStatus::Fail;
                    font_details.push(format!(
                        "{}: restricted OS/2 fsType=0x{:04X}",
                        face_name, rep.fs_type
                    ));
                }
            }
        }
        let font_summary = match font_status {
            PreflightStatus::Pass => {
                "All referenced fonts embedded & licensed (TrueType glyf)".to_string()
            }
            PreflightStatus::Warning => format!(
                "{} font glyph-coverage warning(s) detected",
                missing_fonts.len()
            ),
            PreflightStatus::Fail => format!(
                "{} missing or unembeddable font(s) detected",
                missing_fonts.len()
            ),
        };
        checks.push(PreflightCheckItem {
            id: "fonts".to_string(),
            category: "Fonts".to_string(),
            title: "Font Embedding & Hebrew Coverage".to_string(),
            status: font_status,
            summary: font_summary,
            details: font_details,
        });

        // 2. Color Compliance (ICC Profile, GCR/TAC <= 300%, Spot /Separation)
        let profile = options.color_profile;
        let profile_name = profile.output_condition_identifier().to_string();
        let pdfx_standard = match options.standard {
            PdfXStandard::PdfX1a2001 => "PDF/X-1a:2001".to_string(),
            PdfXStandard::PdfX4 => "PDF/X-4".to_string(),
            PdfXStandard::Plain => "PDF".to_string(),
        };
        let mut color_status = PreflightStatus::Pass;
        let mut color_details = Vec::new();
        let mut spot_inks = std::collections::BTreeSet::new();
        let mut tac_clamped = 0usize;

        let max_tac = profile.max_tac_percent();
        let mut inspect_color = |c: &tok_core::styles::Color| match c {
            tok_core::styles::Color::DeviceCmyk { c, m, y, k } => {
                if (c + m + y + k) * 100.0 > max_tac + 0.5 {
                    tac_clamped += 1;
                }
            }
            tok_core::styles::Color::Spot {
                name,
                fallback_cmyk,
                ..
            } => {
                if name.trim().is_empty() {
                    color_status = PreflightStatus::Fail;
                    color_details.push("Spot color with empty ink name".to_string());
                } else {
                    spot_inks.insert(name.clone());
                }
                let sum =
                    (fallback_cmyk.0 + fallback_cmyk.1 + fallback_cmyk.2 + fallback_cmyk.3) * 100.0;
                if sum > max_tac + 0.5 {
                    tac_clamped += 1;
                }
            }
            tok_core::styles::Color::Rgb { .. } => {}
        };

        for sty in &doc.paragraph_styles {
            inspect_color(&sty.color);
        }
        for page in pages {
            for frame in &page.frames {
                for line in &frame.lines {
                    for c in &line.colors {
                        inspect_color(c);
                    }
                }
            }
            for orn in &page.ornaments {
                inspect_color(&orn.color);
            }
        }
        if tac_clamped > 0 && color_status == PreflightStatus::Pass {
            color_status = PreflightStatus::Warning;
            color_details.push(format!(
                "{} color specification(s) exceeded 300% TAC and are auto-clamped via GCR",
                tac_clamped
            ));
        }
        if !spot_inks.is_empty() {
            color_details.push(format!(
                "Spot /Separation plates: {}",
                spot_inks.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        let color_summary = format!("{} (GCR TAC <= 300%)", profile_name);
        checks.push(PreflightCheckItem {
            id: "color".to_string(),
            category: "Color".to_string(),
            title: "ICC Profile & Ink Limit (TAC <= 300%)".to_string(),
            status: color_status,
            summary: color_summary,
            details: color_details,
        });

        // 3. Trim & Bleed Geometry (>= 3mm = 8.5pt recommended)
        let mut geom_status = PreflightStatus::Pass;
        let mut geom_details = Vec::new();
        if pages.is_empty()
            || pages
                .iter()
                .any(|p| p.dimensions.width <= 0.0 || p.dimensions.height <= 0.0)
        {
            geom_status = PreflightStatus::Fail;
            geom_details.push("Invalid or empty page TrimBox dimensions".to_string());
        } else if options.bleed_pt < 8.5 {
            geom_status = PreflightStatus::Warning;
            geom_details.push(format!(
                "Bleed is {:.2}pt (< 8.5pt / 3mm commercial print standard)",
                options.bleed_pt
            ));
        }
        let geom_summary = if geom_status == PreflightStatus::Pass {
            format!("TrimBox valid; Bleed {:.1}pt (>= 3mm)", options.bleed_pt)
        } else {
            geom_details.first().cloned().unwrap_or_default()
        };
        checks.push(PreflightCheckItem {
            id: "geometry".to_string(),
            category: "Geometry".to_string(),
            title: "TrimBox & Bleed Margins".to_string(),
            status: geom_status,
            summary: geom_summary,
            details: geom_details,
        });

        // 4. Image Effective Resolution (>= 300 DPI Pass, 150..299 Warning, < 150 Fail)
        let mut img_status = PreflightStatus::Pass;
        let mut img_details = Vec::new();
        let mut total_images = 0usize;
        let mut low_dpi_images_count = 0usize;
        for (p_idx, page) in pages.iter().enumerate() {
            for img in &page.images {
                total_images += 1;
                let min_dpi = img.effective_dpi_x.min(img.effective_dpi_y);
                if min_dpi > 0.0 && min_dpi < 150.0 {
                    img_status = PreflightStatus::Fail;
                    low_dpi_images_count += 1;
                    img_details.push(format!(
                        "Page {}: {} effective {:.0} DPI (< 150 DPI minimum)",
                        p_idx + 1,
                        img.asset_path,
                        min_dpi
                    ));
                } else if min_dpi > 0.0 && min_dpi < 300.0 {
                    if img_status == PreflightStatus::Pass {
                        img_status = PreflightStatus::Warning;
                    }
                    low_dpi_images_count += 1;
                    img_details.push(format!(
                        "Page {}: {} effective {:.0} DPI (150-299 DPI print warning)",
                        p_idx + 1,
                        img.asset_path,
                        min_dpi
                    ));
                }
            }
        }
        let img_summary = if total_images == 0 {
            "No raster images placed (vector-only text)".to_string()
        } else if low_dpi_images_count == 0 {
            format!("All {} image(s) >= 300 effective DPI", total_images)
        } else {
            format!(
                "{} of {} image(s) below 300 DPI",
                low_dpi_images_count, total_images
            )
        };
        checks.push(PreflightCheckItem {
            id: "images".to_string(),
            category: "Images".to_string(),
            title: "Raster Image Effective DPI".to_string(),
            status: img_status,
            summary: img_summary,
            details: img_details,
        });

        // 5. Multi-Column Balance & Layout Diagnostics
        let mut col_status = PreflightStatus::Pass;
        let mut col_details = Vec::new();
        let mut balance_warnings_count = 0usize;
        for page in pages {
            for diag in &page.diagnostics {
                if diag.waiver.is_some() {
                    continue;
                }
                if diag.code == "COLUMN_BALANCE_DELTA_EXCEEDED"
                    || diag.severity == DiagnosticSeverity::Warning
                {
                    balance_warnings_count += 1;
                    if col_status == PreflightStatus::Pass {
                        col_status = PreflightStatus::Warning;
                    }
                    col_details.push(diag.message.clone());
                } else if diag.severity == DiagnosticSeverity::Error {
                    col_status = PreflightStatus::Fail;
                    balance_warnings_count += 1;
                    col_details.push(diag.message.clone());
                }
            }
        }
        let col_summary = if balance_warnings_count == 0 {
            "Column bottoms aligned within <= 2.0pt tolerance".to_string()
        } else {
            format!(
                "{} column balance warning(s) (> 2.0pt delta)",
                balance_warnings_count
            )
        };
        checks.push(PreflightCheckItem {
            id: "columns".to_string(),
            category: "Layout".to_string(),
            title: "Column Height Balance".to_string(),
            status: col_status,
            summary: col_summary,
            details: col_details,
        });

        // 6. Page & Frame Bounds Overset Check
        let mut page_status = PreflightStatus::Pass;
        let mut page_details = Vec::new();
        for (p_idx, page) in pages.iter().enumerate() {
            for frame in &page.frames {
                if frame.rect.y + frame.rect.height > page.dimensions.height + 1.0 {
                    page_status = PreflightStatus::Warning;
                    page_details.push(format!(
                        "Page {} frame '{}' exceeds page bottom by {:.1}pt",
                        p_idx + 1,
                        frame.frame_id,
                        frame.rect.y + frame.rect.height - page.dimensions.height
                    ));
                }
            }
        }
        let page_summary = if page_status == PreflightStatus::Pass {
            format!("{} page(s) free of frame overset", pages.len())
        } else {
            format!("{} overset warning(s) detected", page_details.len())
        };
        checks.push(PreflightCheckItem {
            id: "pagination".to_string(),
            category: "Pagination".to_string(),
            title: "Overset & Frame Boundaries".to_string(),
            status: page_status,
            summary: page_summary,
            details: page_details,
        });

        let overall_status = checks
            .iter()
            .map(|c| c.status)
            .max()
            .unwrap_or(PreflightStatus::Pass);

        PreflightReport {
            overall_status,
            profile_name,
            pdfx_standard,
            total_pages: pages.len(),
            checks,
            missing_fonts,
            low_dpi_images_count,
            balance_warnings_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tok_core::id::FractionalIndex;
    use tok_core::model::ParagraphNode;
    use tok_core::styles::ParagraphStyle;
    use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
    use tok_typeset::geometry::{PhysicalRect, PlacedImageBox};

    #[test]
    fn clean_document_passes_preflight() {
        let mut doc = DocumentRoot::new("Clean Book");
        doc.sections[0].flows[0].paragraphs = vec![ParagraphNode::new(
            FractionalIndex::initial(),
            "default-body",
            "בראשית ברא אלהים את השמים ואת הארץ",
        )];
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let pages = engine.typeset_document(&doc);
        let rep = PreflightEngine::evaluate(
            &doc,
            &pages,
            &engine.font_manager,
            &PdfExportOptions::default(),
        );
        assert_eq!(rep.overall_status, PreflightStatus::Pass);
        assert!(rep.missing_fonts.is_empty());
        assert_eq!(rep.low_dpi_images_count, 0);
        assert_eq!(rep.balance_warnings_count, 0);
    }

    #[test]
    fn missing_font_and_low_dpi_image_fail_preflight() {
        let mut doc = DocumentRoot::new("Failing Book");
        doc.paragraph_styles.push(ParagraphStyle {
            id: "missing-sty".to_string(),
            name: "Missing".to_string(),
            font_family: "MissingFontXYZ".to_string(),
            ..Default::default()
        });
        doc.sections[0].flows[0].paragraphs = vec![ParagraphNode::new(
            FractionalIndex::initial(),
            "missing-sty",
            "שלום",
        )];
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let mut pages = engine.typeset_document(&doc);
        pages[0].images.push(PlacedImageBox {
            asset_path: "images/lowres.png".to_string(),
            rect: PhysicalRect::new(50.0, 50.0, 200.0, 200.0),
            intrinsic_width_px: 200,
            intrinsic_height_px: 200,
            effective_dpi_x: 72.0,
            effective_dpi_y: 72.0,
            paragraph_id: None,
        });
        let rep = PreflightEngine::evaluate(
            &doc,
            &pages,
            &engine.font_manager,
            &PdfExportOptions::default(),
        );
        assert_eq!(rep.overall_status, PreflightStatus::Fail);
        assert_eq!(rep.missing_fonts.len(), 1);
        assert_eq!(rep.low_dpi_images_count, 1);
    }

    #[test]
    fn column_balance_diagnostic_surfaces_warning_in_preflight() {
        let doc = DocumentRoot::new("Col Warning");
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let mut pages = engine.typeset_document(&doc);
        pages[0]
            .diagnostics
            .push(tok_typeset::diagnostics::LayoutDiagnostic {
                code: "COLUMN_BALANCE_DELTA_EXCEEDED".to_string(),
                severity: DiagnosticSeverity::Warning,
                message: "Page 1: delta 8.0pt".to_string(),
                page_index: 0,
                bottom_delta_pt: Some(8.0),
                waiver: None,
            });
        let rep = PreflightEngine::evaluate(
            &doc,
            &pages,
            &engine.font_manager,
            &PdfExportOptions::default(),
        );
        assert_eq!(rep.overall_status, PreflightStatus::Warning);
        assert_eq!(rep.balance_warnings_count, 1);
    }
}

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
pub struct PreflightIssue {
    pub code: String,
    pub check_id: String,
    pub severity: PreflightStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub message: String,
    pub message_he: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion_he: Option<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<PreflightIssue>,
    #[serde(default)]
    pub error_count: usize,
    #[serde(default)]
    pub warning_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_pages: Vec<usize>,
}

impl PreflightReport {
    /// Returns all issues targeting a specific 1-based page number.
    pub fn issues_for_page(&self, page_number: usize) -> Vec<&PreflightIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.page_number == Some(page_number))
            .collect()
    }

    /// Returns document-wide issues that are not bound to a single page.
    pub fn document_wide_issues(&self) -> Vec<&PreflightIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.page_number.is_none())
            .collect()
    }
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
        let mut issues: Vec<PreflightIssue> = Vec::new();

        // 1. Font embedding, fsType license permissions, & Hebrew/Niqqud/Taamim glyph coverage
        let missing_fonts = font_mgr.find_missing_or_incomplete_fonts(doc);
        let mut font_details = Vec::new();
        let mut font_status = PreflightStatus::Pass;
        let mut unembeddable_count = 0usize;
        for diag in &missing_fonts {
            if diag.missing_family || diag.missing_hebrew_coverage {
                font_status = PreflightStatus::Fail;
            } else if font_status != PreflightStatus::Fail {
                font_status = PreflightStatus::Warning;
            }
            let mut flags = Vec::new();
            let mut flags_he = Vec::new();
            if diag.missing_family {
                flags.push("font not installed/embedded");
                flags_he.push("הגופן אינו מותקן או אינו מוטמע");
            }
            if diag.missing_hebrew_coverage {
                flags.push("missing Hebrew letters");
                flags_he.push("חסרות אותיות עבריות");
            }
            if diag.missing_niqqud_coverage {
                flags.push("missing Niqqud");
                flags_he.push("חסר ניקוד");
            }
            if diag.missing_taamim_coverage {
                flags.push("missing Taamim");
                flags_he.push("חסרים טעמי מקרא");
            }
            font_details.push(format!(
                "{} ({} paragraphs): {}",
                diag.family_name,
                diag.affected_paragraphs,
                flags.join(", ")
            ));
            let sev = if diag.missing_family || diag.missing_hebrew_coverage {
                PreflightStatus::Fail
            } else {
                PreflightStatus::Warning
            };
            issues.push(PreflightIssue {
                code: if diag.missing_family {
                    "FONT_NOT_EMBEDDED".to_string()
                } else {
                    "FONT_INCOMPLETE_GLYPHS".to_string()
                },
                check_id: "fonts".to_string(),
                severity: sev,
                page_number: None,
                location: Some(format!(
                    "{} ({} paragraphs)",
                    diag.family_name, diag.affected_paragraphs
                )),
                message: format!(
                    "Font '{}' ({} paragraphs): {}",
                    diag.family_name,
                    diag.affected_paragraphs,
                    flags.join(", ")
                ),
                message_he: format!(
                    "הגופן '{}' ({} פסקאות): {}",
                    diag.family_name,
                    diag.affected_paragraphs,
                    flags_he.join(", ")
                ),
                suggestion: Some(
                    "Install/embed the font or switch to a print-ready Hebrew font such as Frank Ruhl Libre."
                        .to_string(),
                ),
                suggestion_he: Some(
                    "התקינו והטמיעו את הגופן, או החליפו לגופן דפוס תומך עברית וניקוד כגון Frank Ruhl Libre."
                        .to_string(),
                ),
            });
        }
        // Check registered external fonts for non-embeddable OS/2 fsType
        let mut restricted_families = std::collections::BTreeSet::new();
        for (face_name, _) in font_mgr.external_fonts() {
            if let Some(rep) = font_mgr.inspect_registered_family(&face_name) {
                if !rep.is_embeddable {
                    font_status = PreflightStatus::Fail;
                    unembeddable_count += 1;
                    restricted_families.insert(face_name.clone());
                    font_details.push(format!(
                        "{}: restricted OS/2 fsType=0x{:04X}",
                        face_name, rep.fs_type
                    ));
                    issues.push(PreflightIssue {
                        code: "FONT_EMBEDDING_RESTRICTED".to_string(),
                        check_id: "fonts".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: None,
                        location: Some(face_name.clone()),
                        message: format!(
                            "Font '{}' has restricted embedding license (OS/2 fsType=0x{:04X})",
                            face_name, rep.fs_type
                        ),
                        message_he: format!(
                            "רישיון הגופן '{}' אוסר הטמעה ב-PDF (OS/2 fsType=0x{:04X})",
                            face_name, rep.fs_type
                        ),
                        suggestion: Some(
                            "Replace with a font whose OS/2 fsType permits PDF embedding."
                                .to_string(),
                        ),
                        suggestion_he: Some(
                            "החליפו בגופן שרישיון ה-OS/2 שלו מתיר הטמעה בקובץ PDF.".to_string(),
                        ),
                    });
                }
            }
        }
        // Per-page font embedding & missing-glyph (.notdef) check across typeset pages
        for (p_idx, page) in pages.iter().enumerate() {
            let page_num = p_idx + 1;
            let mut page_fonts = std::collections::BTreeSet::new();
            let mut notdef_chars = std::collections::BTreeSet::new();
            let mut has_unknown_font_glyphs = false;

            if let Some(pns) = &page.page_number_style {
                if !pns.font_family.trim().is_empty() {
                    page_fonts.insert(pns.font_family.clone());
                }
            }
            for frame in &page.frames {
                for line in &frame.lines {
                    for f in &line.fonts {
                        if !f.trim().is_empty() {
                            page_fonts.insert(f.clone());
                        }
                    }
                    for run in &line.visual_runs {
                        if !run.font_family.trim().is_empty() {
                            page_fonts.insert(run.font_family.clone());
                        }
                    }
                    for g in &line.glyphs {
                        if g.font_index == tok_typeset::geometry::UNKNOWN_FONT {
                            has_unknown_font_glyphs = true;
                        } else if g.glyph_id == 0 {
                            if let Some(ch) = g.character {
                                if !ch.is_whitespace() && !ch.is_control() {
                                    notdef_chars.insert(ch);
                                }
                            }
                        }
                    }
                }
            }

            for fam in page_fonts {
                if !font_mgr.is_family_available(&fam) {
                    font_status = PreflightStatus::Fail;
                    unembeddable_count += 1;
                    font_details.push(format!(
                        "Page {}: font '{}' is not registered for PDF embedding",
                        page_num, fam
                    ));
                    issues.push(PreflightIssue {
                        code: "FONT_PAGE_NOT_EMBEDDED".to_string(),
                        check_id: "fonts".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: Some(fam.clone()),
                        message: format!(
                            "Page {}: font '{}' is not available for PDF embedding",
                            page_num, fam
                        ),
                        message_he: format!(
                            "עמוד {}: הגופן '{}' אינו זמין להטמעה בקובץ ה-PDF",
                            page_num, fam
                        ),
                        suggestion: Some("Embed or install the font before exporting.".to_string()),
                        suggestion_he: Some("התקינו או הטמיעו את הגופן לפני הייצוא.".to_string()),
                    });
                } else if restricted_families.contains(&fam) {
                    font_status = PreflightStatus::Fail;
                    issues.push(PreflightIssue {
                        code: "FONT_PAGE_EMBEDDING_RESTRICTED".to_string(),
                        check_id: "fonts".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: Some(fam.clone()),
                        message: format!(
                            "Page {}: font '{}' cannot be embedded due to license restrictions",
                            page_num, fam
                        ),
                        message_he: format!(
                            "עמוד {}: לא ניתן להטמיע את הגופן '{}' עקב מגבלת רישיון",
                            page_num, fam
                        ),
                        suggestion: Some("Switch to an embeddable TrueType font.".to_string()),
                        suggestion_he: Some("החליפו לגופן TrueType שמותר להטמעה.".to_string()),
                    });
                }
            }

            if has_unknown_font_glyphs {
                if font_status == PreflightStatus::Pass {
                    font_status = PreflightStatus::Warning;
                }
                font_details.push(format!(
                    "Page {}: contains glyphs shaped without a resolved font",
                    page_num
                ));
                issues.push(PreflightIssue {
                    code: "FONT_UNRESOLVED_GLYPHS".to_string(),
                    check_id: "fonts".to_string(),
                    severity: PreflightStatus::Warning,
                    page_number: Some(page_num),
                    location: None,
                    message: format!(
                        "Page {}: some glyphs were shaped using heuristic fallback without an embedded font",
                        page_num
                    ),
                    message_he: format!(
                        "עמוד {}: קיימים תווים שעברו עימוד ללא גופן מוטמע מזוהה",
                        page_num
                    ),
                    suggestion: Some("Verify all paragraph styles reference installed fonts.".to_string()),
                    suggestion_he: Some("ודאו שכל סגנונות הפסקה מפנים לגופנים מותקנים.".to_string()),
                });
            }

            if !notdef_chars.is_empty() {
                if font_status == PreflightStatus::Pass {
                    font_status = PreflightStatus::Warning;
                }
                let sample: String = notdef_chars.iter().take(6).collect();
                font_details.push(format!(
                    "Page {}: {} missing glyph(s) (.notdef) e.g. '{}'",
                    page_num,
                    notdef_chars.len(),
                    sample
                ));
                issues.push(PreflightIssue {
                    code: "FONT_MISSING_GLYPHS_NOTDEF".to_string(),
                    check_id: "fonts".to_string(),
                    severity: PreflightStatus::Warning,
                    page_number: Some(page_num),
                    location: Some(format!("glyphs: {}", sample)),
                    message: format!(
                        "Page {}: {} character(s) mapped to .notdef (missing glyph), e.g. '{}'",
                        page_num,
                        notdef_chars.len(),
                        sample
                    ),
                    message_he: format!(
                        "עמוד {}: {} תווים חסרים בגופן (.notdef), לדוגמה '{}'",
                        page_num,
                        notdef_chars.len(),
                        sample
                    ),
                    suggestion: Some("Use a font that contains glyphs for all characters on this page.".to_string()),
                    suggestion_he: Some("השתמשו בגופן שמכיל גליפים לכל התווים בעמוד זה.".to_string()),
                });
            }
        }
        let font_summary = match font_status {
            PreflightStatus::Pass => {
                "All referenced fonts embedded & licensed (TrueType glyf)".to_string()
            }
            PreflightStatus::Warning => format!(
                "{} font glyph-coverage warning(s) detected",
                missing_fonts.len().max(1)
            ),
            PreflightStatus::Fail => format!(
                "{} missing or unembeddable font issue(s) detected",
                missing_fonts.len() + unembeddable_count
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

        // 3. TrimBox, BleedBox, & Slug Geometry (>= 3mm = 8.5pt recommended)
        let mut geom_status = PreflightStatus::Pass;
        let mut geom_details = Vec::new();
        if pages.is_empty() {
            geom_status = PreflightStatus::Fail;
            let msg = "Invalid or empty page TrimBox dimensions".to_string();
            geom_details.push(msg.clone());
            issues.push(PreflightIssue {
                code: "TRIMBOX_EMPTY_DOCUMENT".to_string(),
                check_id: "geometry".to_string(),
                severity: PreflightStatus::Fail,
                page_number: None,
                location: None,
                message: msg,
                message_he: "המסמך ריק או שאין עמודים להגדרת TrimBox".to_string(),
                suggestion: Some("Add at least one page with valid trim dimensions.".to_string()),
                suggestion_he: Some("הוסיפו לפחות עמוד אחד עם מידות חיתוך תקינות.".to_string()),
            });
        } else {
            let ref_w = pages[0].dimensions.width;
            let ref_h = pages[0].dimensions.height;
            let mut mixed_pages_reported = 0usize;

            for (p_idx, page) in pages.iter().enumerate() {
                let page_num = p_idx + 1;
                let w = page.dimensions.width;
                let h = page.dimensions.height;
                if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
                    geom_status = PreflightStatus::Fail;
                    let msg = format!(
                        "Page {}: invalid TrimBox dimensions ({:.2} x {:.2}pt)",
                        page_num, w, h
                    );
                    geom_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "TRIMBOX_INVALID_DIMENSIONS".to_string(),
                        check_id: "geometry".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: Some(format!("{:.2} x {:.2}pt", w, h)),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: מידות TrimBox אינן תקינות ({:.2} x {:.2} נק')",
                            page_num, w, h
                        ),
                        suggestion: Some("Set positive finite page width and height.".to_string()),
                        suggestion_he: Some("הגדירו רוחב וגובה חיוביים ותקינים לעמוד.".to_string()),
                    });
                    continue;
                }

                if ((w - ref_w).abs() > 0.5 || (h - ref_h).abs() > 0.5) && mixed_pages_reported < 10
                {
                    mixed_pages_reported += 1;
                    if geom_status == PreflightStatus::Pass {
                        geom_status = PreflightStatus::Warning;
                    }
                    let msg = format!(
                        "Page {}: TrimBox ({:.1} x {:.1}pt) differs from page 1 ({:.1} x {:.1}pt)",
                        page_num, w, h, ref_w, ref_h
                    );
                    geom_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "TRIMBOX_MIXED_PAGE_SIZES".to_string(),
                        check_id: "geometry".to_string(),
                        severity: PreflightStatus::Warning,
                        page_number: Some(page_num),
                        location: Some(format!("{:.1} x {:.1}pt", w, h)),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: גודל TrimBox ({:.1} x {:.1} נק') שונה מעמוד 1 ({:.1} x {:.1} נק')",
                            page_num, w, h, ref_w, ref_h
                        ),
                        suggestion: Some("Ensure consistent TrimBox dimensions across the signature.".to_string()),
                        suggestion_he: Some("ודאו שמידות החיתוך אחידות בכל עמודי הספר.".to_string()),
                    });
                }

                let boxes = crate::boxes::PrePressPageBoxes::new(
                    w,
                    h,
                    options.bleed_pt,
                    options.slug_pt,
                );
                if boxes.bleed_box[0] > boxes.trim_box[0]
                    || boxes.bleed_box[2] < boxes.trim_box[2]
                    || boxes.media_box[0] > boxes.bleed_box[0]
                    || boxes.media_box[2] < boxes.bleed_box[2]
                {
                    geom_status = PreflightStatus::Fail;
                    let msg = format!(
                        "Page {}: BleedBox/MediaBox does not enclose TrimBox",
                        page_num
                    );
                    geom_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "BLEEDBOX_ENCLOSURE_ERROR".to_string(),
                        check_id: "geometry".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: None,
                        message: msg,
                        message_he: format!(
                            "עמוד {}: תיבת ה-BleedBox/MediaBox אינה מכילה את ה-TrimBox",
                            page_num
                        ),
                        suggestion: Some("Verify bleed and slug margins are non-negative.".to_string()),
                        suggestion_he: Some("ודאו ששולי החיתוך וה-Slug אינם שליליים.".to_string()),
                    });
                }

                // Check for edge-bleed hazard when bleed < 3mm
                if options.bleed_pt < 8.5 {
                    let touches_edge = |r: &tok_typeset::geometry::PhysicalRect| -> bool {
                        r.x <= 1.0
                            || r.y <= 1.0
                            || (r.x + r.width) >= (w - 1.0)
                            || (r.y + r.height) >= (h - 1.0)
                    };
                    for img in &page.images {
                        if touches_edge(&img.rect) {
                            let sev = if options.bleed_pt <= 0.0
                                && options.standard != PdfXStandard::Plain
                            {
                                geom_status = PreflightStatus::Fail;
                                PreflightStatus::Fail
                            } else {
                                if geom_status == PreflightStatus::Pass {
                                    geom_status = PreflightStatus::Warning;
                                }
                                PreflightStatus::Warning
                            };
                            let msg = format!(
                                "Page {}: image '{}' reaches trim edge with only {:.2}pt bleed",
                                page_num, img.asset_path, options.bleed_pt
                            );
                            geom_details.push(msg.clone());
                            issues.push(PreflightIssue {
                                code: "BLEED_EDGE_HAZARD".to_string(),
                                check_id: "geometry".to_string(),
                                severity: sev,
                                page_number: Some(page_num),
                                location: Some(img.asset_path.clone()),
                                message: msg,
                                message_he: format!(
                                    "עמוד {}: התמונה '{}' נוגעת בקצה החיתוך עם Bleed של {:.2} נק' בלבד",
                                    page_num, img.asset_path, options.bleed_pt
                                ),
                                suggestion: Some("Enable >= 3mm (8.5pt) bleed and extend edge artwork into the bleed area.".to_string()),
                                suggestion_he: Some("הגדירו שולי חיתוך של 3 מ\"מ (8.5 נק') לפחות והאריכו את התמונה לאזור ה-Bleed.".to_string()),
                            });
                        }
                    }
                    for orn in &page.ornaments {
                        if touches_edge(&orn.rect) {
                            if geom_status == PreflightStatus::Pass {
                                geom_status = PreflightStatus::Warning;
                            }
                            let msg = format!(
                                "Page {}: ornament '{}' reaches trim edge with {:.2}pt bleed",
                                page_num, orn.ornament_id, options.bleed_pt
                            );
                            geom_details.push(msg.clone());
                            issues.push(PreflightIssue {
                                code: "BLEED_ORNAMENT_EDGE_HAZARD".to_string(),
                                check_id: "geometry".to_string(),
                                severity: PreflightStatus::Warning,
                                page_number: Some(page_num),
                                location: Some(orn.ornament_id.clone()),
                                message: msg,
                                message_he: format!(
                                    "עמוד {}: העיטור '{}' נוגע בקצה החיתוך עם Bleed של {:.2} נק'",
                                    page_num, orn.ornament_id, options.bleed_pt
                                ),
                                suggestion: Some("Enable >= 3mm (8.5pt) bleed for edge ornaments.".to_string()),
                                suggestion_he: Some("הגדירו שולי חיתוך של 3 מ\"מ (8.5 נק') לפחות.".to_string()),
                            });
                        }
                    }
                }
            }

            if !options.bleed_pt.is_finite() || options.bleed_pt < 0.0 {
                geom_status = PreflightStatus::Fail;
                let msg = format!("Invalid Bleed value: {:.2}pt", options.bleed_pt);
                geom_details.push(msg.clone());
                issues.push(PreflightIssue {
                    code: "BLEED_INVALID".to_string(),
                    check_id: "geometry".to_string(),
                    severity: PreflightStatus::Fail,
                    page_number: None,
                    location: None,
                    message: msg,
                    message_he: format!("ערך שולי החיתוך (Bleed) אינו תקין: {:.2} נק'", options.bleed_pt),
                    suggestion: Some("Set bleed to >= 8.5pt (3mm).".to_string()),
                    suggestion_he: Some("הגדירו שולי חיתוך ל-8.5 נק' (3 מ\"מ) ומעלה.".to_string()),
                });
            } else if options.bleed_pt < 8.5 {
                if geom_status == PreflightStatus::Pass {
                    geom_status = PreflightStatus::Warning;
                }
                let msg = format!(
                    "Bleed is {:.2}pt (< 8.5pt / 3mm commercial print standard)",
                    options.bleed_pt
                );
                geom_details.push(msg.clone());
                issues.push(PreflightIssue {
                    code: "BLEED_BELOW_STANDARD".to_string(),
                    check_id: "geometry".to_string(),
                    severity: PreflightStatus::Warning,
                    page_number: None,
                    location: None,
                    message: msg,
                    message_he: format!(
                        "שולי החיתוך (Bleed) הם {:.2} נק' (מתחת לתקן המסחרי של 3 מ\"מ / 8.5 נק')",
                        options.bleed_pt
                    ),
                    suggestion: Some("Increase bleed to at least 3mm (8.5pt) in export settings.".to_string()),
                    suggestion_he: Some("הגדילו את שולי החיתוך ל-3 מ\"מ (8.5 נק') לפחות בהגדרות הייצוא.".to_string()),
                });
            }

            if (options.draw_crop_marks || options.draw_registration_marks)
                && options.slug_pt < options.bleed_pt + 14.0
            {
                if geom_status == PreflightStatus::Pass {
                    geom_status = PreflightStatus::Warning;
                }
                let msg = format!(
                    "Slug margin ({:.1}pt) is tight for printer marks (recommended >= {:.1}pt)",
                    options.slug_pt,
                    options.bleed_pt + 14.0
                );
                geom_details.push(msg.clone());
                issues.push(PreflightIssue {
                    code: "SLUG_TIGHT_FOR_MARKS".to_string(),
                    check_id: "geometry".to_string(),
                    severity: PreflightStatus::Warning,
                    page_number: None,
                    location: None,
                    message: msg,
                    message_he: format!(
                        "שולי ה-Slug ({:.1} נק') צפופים מדי לסימני החיתוך והרישום",
                        options.slug_pt
                    ),
                    suggestion: Some("Use default 10mm (28.35pt) slug margin when printer marks are enabled.".to_string()),
                    suggestion_he: Some("השתמשו בשולי Slug של 10 מ\"מ (28.35 נק') כאשר סימני דפוס פעילים.".to_string()),
                });
            }
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

        let error_count = issues
            .iter()
            .filter(|i| i.severity == PreflightStatus::Fail)
            .count();
        let warning_count = issues
            .iter()
            .filter(|i| i.severity == PreflightStatus::Warning)
            .count();
        let affected_pages: Vec<usize> = issues
            .iter()
            .filter_map(|i| i.page_number)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        PreflightReport {
            overall_status,
            profile_name,
            pdfx_standard,
            total_pages: pages.len(),
            checks,
            missing_fonts,
            low_dpi_images_count,
            balance_warnings_count,
            issues,
            error_count,
            warning_count,
            affected_pages,
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

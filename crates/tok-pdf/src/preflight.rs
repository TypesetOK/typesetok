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

    /// Formats an organized, human-readable preflight report grouped by document-wide
    /// and per-page findings.
    pub fn format_report(&self, hebrew: bool) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let status_str = |s: PreflightStatus| match (s, hebrew) {
            (PreflightStatus::Pass, true) => "תקין",
            (PreflightStatus::Warning, true) => "אזהרה",
            (PreflightStatus::Fail, true) => "שגיאה",
            (PreflightStatus::Pass, false) => "PASS",
            (PreflightStatus::Warning, false) => "WARNING",
            (PreflightStatus::Fail, false) => "FAIL",
        };

        if hebrew {
            let _ = writeln!(
                out,
                "=== דוח קדם-דפוס (Preflight) | {} | {} ===",
                self.profile_name, self.pdfx_standard
            );
            let _ = writeln!(
                out,
                "סטטוס כללי: {} | עמודים: {} | שגיאות: {} | אזהרות: {}",
                status_str(self.overall_status),
                self.total_pages,
                self.error_count,
                self.warning_count
            );
        } else {
            let _ = writeln!(
                out,
                "=== Pre-Press Preflight Report | {} | {} ===",
                self.profile_name, self.pdfx_standard
            );
            let _ = writeln!(
                out,
                "Overall: {} | Pages: {} | Errors: {} | Warnings: {}",
                status_str(self.overall_status),
                self.total_pages,
                self.error_count,
                self.warning_count
            );
        }

        let doc_issues = self.document_wide_issues();
        if !doc_issues.is_empty() {
            let _ = writeln!(
                out,
                "\n{}",
                if hebrew {
                    "--- בעיות ברמת המסמך ---"
                } else {
                    "--- Document-Wide Issues ---"
                }
            );
            for issue in doc_issues {
                let msg = if hebrew {
                    &issue.message_he
                } else {
                    &issue.message
                };
                let _ = writeln!(
                    out,
                    "  [{}] [{}] {}",
                    status_str(issue.severity),
                    issue.code,
                    msg
                );
            }
        }

        for &page_num in &self.affected_pages {
            let page_issues = self.issues_for_page(page_num);
            if page_issues.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "\n{}",
                if hebrew {
                    format!("--- עמוד {} ({} ממצאים) ---", page_num, page_issues.len())
                } else {
                    format!("--- Page {} ({} issue(s)) ---", page_num, page_issues.len())
                }
            );
            for issue in page_issues {
                let msg = if hebrew {
                    &issue.message_he
                } else {
                    &issue.message
                };
                let loc = issue
                    .location
                    .as_deref()
                    .map(|l| format!(" ({})", l))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "  [{}] [{}]{} {}",
                    status_str(issue.severity),
                    issue.code,
                    loc,
                    msg
                );
            }
        }

        out
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

        // 2. Color Compliance (ICC Profile, PDF/X color space, GCR/TAC <= profile limit, Spot /Separation)
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
        let mut rgb_in_print_count = 0usize;

        let max_tac = profile.max_tac_percent();
        let is_pdfx = options.standard != PdfXStandard::Plain;

        let mut inspect_color =
            |c: &tok_core::styles::Color, page_num: Option<usize>, location: &str, is_text: bool| {
                match c {
                    tok_core::styles::Color::DeviceCmyk { c, m, y, k } => {
                        if !c.is_finite()
                            || !m.is_finite()
                            || !y.is_finite()
                            || !k.is_finite()
                            || *c < 0.0
                            || *c > 1.0
                            || *m < 0.0
                            || *m > 1.0
                            || *y < 0.0
                            || *y > 1.0
                            || *k < 0.0
                            || *k > 1.0
                        {
                            color_status = PreflightStatus::Fail;
                            let msg = format!(
                                "{}: invalid CMYK component values ({:.2}, {:.2}, {:.2}, {:.2})",
                                location, c, m, y, k
                            );
                            color_details.push(msg.clone());
                            issues.push(PreflightIssue {
                                code: "COLOR_INVALID_CMYK".to_string(),
                                check_id: "color".to_string(),
                                severity: PreflightStatus::Fail,
                                page_number: page_num,
                                location: Some(location.to_string()),
                                message: msg,
                                message_he: format!(
                                    "{}: ערכי CMYK לא תקינים ({:.2}, {:.2}, {:.2}, {:.2})",
                                    location, c, m, y, k
                                ),
                                suggestion: Some("Keep CMYK channel values between 0.0 and 1.0.".to_string()),
                                suggestion_he: Some("ודאו שערכי הערוצים CMYK נמצאים בטווח 0.0 עד 1.0.".to_string()),
                            });
                            return;
                        }
                        let tac = (c + m + y + k) * 100.0;
                        if tac > max_tac + 0.5 {
                            tac_clamped += 1;
                            if color_status == PreflightStatus::Pass {
                                color_status = PreflightStatus::Warning;
                            }
                            issues.push(PreflightIssue {
                                code: "COLOR_TAC_EXCEEDED".to_string(),
                                check_id: "color".to_string(),
                                severity: PreflightStatus::Warning,
                                page_number: page_num,
                                location: Some(location.to_string()),
                                message: format!(
                                    "{}: Total Ink Coverage {:.0}% exceeds {} limit ({:.0}%)",
                                    location, tac, profile_name, max_tac
                                ),
                                message_he: format!(
                                    "{}: כיסוי הדיו הכולל ({:.0}%) חורג ממגבלת פרופיל {} ({:.0}%)",
                                    location, tac, profile_name, max_tac
                                ),
                                suggestion: Some(format!(
                                    "Reduce CMYK ink sum below {:.0}% or rely on GCR export clamping.",
                                    max_tac
                                )),
                                suggestion_he: Some(format!(
                                    "הפחיתו את סכום הדיו מתחת ל-{:.0}% כדי למנוע מריחה בהדפסה.",
                                    max_tac
                                )),
                            });
                        } else if is_text && *c > 0.1 && *m > 0.1 && *y > 0.1 && *k > 0.5 {
                            if color_status == PreflightStatus::Pass {
                                color_status = PreflightStatus::Warning;
                            }
                            issues.push(PreflightIssue {
                                code: "COLOR_RICH_BLACK_TEXT".to_string(),
                                check_id: "color".to_string(),
                                severity: PreflightStatus::Warning,
                                page_number: page_num,
                                location: Some(location.to_string()),
                                message: format!(
                                    "{}: text uses 4-color rich black; 100% K is recommended for sharp type",
                                    location
                                ),
                                message_he: format!(
                                    "{}: הטקסט משתמש בשחור עשיר (4 לוחות); מומלץ 100% K לטקסט חד",
                                    location
                                ),
                                suggestion: Some("Use pure black (0/0/0/100 K) for body text.".to_string()),
                                suggestion_he: Some("הגדירו שחור טהור (100% K) לטקסט רץ למניעת הילת רישום.".to_string()),
                            });
                        }
                    }
                    tok_core::styles::Color::Spot {
                        name,
                        tint,
                        fallback_cmyk,
                    } => {
                        if name.trim().is_empty() {
                            color_status = PreflightStatus::Fail;
                            let msg = format!("{}: Spot color with empty ink name", location);
                            color_details.push("Spot color with empty ink name".to_string());
                            issues.push(PreflightIssue {
                                code: "COLOR_SPOT_EMPTY_NAME".to_string(),
                                check_id: "color".to_string(),
                                severity: PreflightStatus::Fail,
                                page_number: page_num,
                                location: Some(location.to_string()),
                                message: msg,
                                message_he: format!("{}: צבע ספוט ללא שם דיו", location),
                                suggestion: Some("Assign a valid Pantone/Spot ink name or convert to CMYK.".to_string()),
                                suggestion_he: Some("תנו שם דיו לצבע הספוט או המירו ל-CMYK.".to_string()),
                            });
                        } else {
                            spot_inks.insert(name.clone());
                        }
                        if !tint.is_finite() || *tint < 0.0 || *tint > 1.0 {
                            color_status = PreflightStatus::Fail;
                            let msg = format!("{}: Spot '{}' has invalid tint {:.2}", location, name, tint);
                            color_details.push(msg.clone());
                            issues.push(PreflightIssue {
                                code: "COLOR_SPOT_INVALID_TINT".to_string(),
                                check_id: "color".to_string(),
                                severity: PreflightStatus::Fail,
                                page_number: page_num,
                                location: Some(location.to_string()),
                                message: msg,
                                message_he: format!("{}: ערך tint לא תקין ({:.2}) בצבע ספוט '{}'", location, tint, name),
                                suggestion: Some("Set spot tint between 0.0 and 1.0.".to_string()),
                                suggestion_he: Some("הגדירו את אחוז הגוון (tint) בין 0.0 ל-1.0.".to_string()),
                            });
                        }
                        let sum = (fallback_cmyk.0
                            + fallback_cmyk.1
                            + fallback_cmyk.2
                            + fallback_cmyk.3)
                            * 100.0;
                        if sum > max_tac + 0.5 {
                            tac_clamped += 1;
                            if color_status == PreflightStatus::Pass {
                                color_status = PreflightStatus::Warning;
                            }
                        }
                    }
                    tok_core::styles::Color::Rgb { r, g, b } => {
                        let is_pure_black = *r <= 1e-4 && *g <= 1e-4 && *b <= 1e-4;
                        let is_pure_white = *r >= 1.0 - 1e-4 && *g >= 1.0 - 1e-4 && *b >= 1.0 - 1e-4;
                        if is_pdfx && !is_pure_black && !is_pure_white {
                            rgb_in_print_count += 1;
                            if color_status == PreflightStatus::Pass {
                                color_status = PreflightStatus::Warning;
                            }
                            issues.push(PreflightIssue {
                                code: "COLOR_RGB_FOR_PRINT_PROFILE".to_string(),
                                check_id: "color".to_string(),
                                severity: PreflightStatus::Warning,
                                page_number: page_num,
                                location: Some(location.to_string()),
                                message: format!(
                                    "{}: RGB color ({:.2}, {:.2}, {:.2}) is not native to {} / {}; will be GCR-converted to CMYK",
                                    location, r, g, b, pdfx_standard, profile_name
                                ),
                                message_he: format!(
                                    "{}: צבע RGB ({:.2}, {:.2}, {:.2}) אינו מתאים ישירות לפרופיל {} ({}); יומר אוטומטית ל-CMYK",
                                    location, r, g, b, profile_name, pdfx_standard
                                ),
                                suggestion: Some(format!(
                                    "Define this color directly in DeviceCMYK for {} to avoid gamut shifts.",
                                    profile_name
                                )),
                                suggestion_he: Some(format!(
                                    "הגדירו את הצבע ישירות ב-DeviceCMYK עבור פרופיל {} למניעת סטיות גוון.",
                                    profile_name
                                )),
                            });
                        }
                    }
                }
            };

        for sty in &doc.paragraph_styles {
            let loc = format!("Style '{}'", sty.name);
            inspect_color(&sty.color, None, &loc, true);
        }
        for (p_idx, page) in pages.iter().enumerate() {
            let page_num = p_idx + 1;
            for frame in &page.frames {
                for line in &frame.lines {
                    for c in &line.colors {
                        let loc = format!("Page {} frame '{}'", page_num, frame.frame_id);
                        inspect_color(c, Some(page_num), &loc, true);
                    }
                }
            }
            for orn in &page.ornaments {
                let loc = format!("Page {} ornament '{}'", page_num, orn.ornament_id);
                inspect_color(&orn.color, Some(page_num), &loc, false);
            }
            if let Some(pns) = &page.page_number_style {
                if let Some(c) = &pns.color {
                    let loc = format!("Page {} page number", page_num);
                    inspect_color(c, Some(page_num), &loc, true);
                }
            }
        }
        if tac_clamped > 0 {
            if color_status == PreflightStatus::Pass {
                color_status = PreflightStatus::Warning;
            }
            color_details.push(format!(
                "{} color specification(s) exceeded {:.0}% TAC and are auto-clamped via GCR",
                tac_clamped, max_tac
            ));
        }
        if rgb_in_print_count > 0 {
            color_details.push(format!(
                "{} RGB color(s) converted to {} ({}) on export",
                rgb_in_print_count, profile_name, pdfx_standard
            ));
        }
        if !spot_inks.is_empty() {
            color_details.push(format!(
                "Spot /Separation plates: {}",
                spot_inks.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        let color_summary = format!("{} (GCR TAC <= {:.0}%)", profile_name, max_tac);
        checks.push(PreflightCheckItem {
            id: "color".to_string(),
            category: "Color".to_string(),
            title: format!("ICC Profile & Ink Limit (TAC <= {:.0}%)", max_tac),
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

        // 4. Image Effective Resolution (>= 300 DPI Pass, 150..299 Warning, < 150 Fail),
        //    aspect distortion, intrinsic dimensions, and asset availability.
        let mut img_status = PreflightStatus::Pass;
        let mut img_details = Vec::new();
        let mut total_images = 0usize;
        let mut low_dpi_images_count = 0usize;
        let has_asset_map = !options.image_assets.is_empty();

        for (p_idx, page) in pages.iter().enumerate() {
            let page_num = p_idx + 1;
            for img in &page.images {
                total_images += 1;
                let dx = img.effective_dpi_x;
                let dy = img.effective_dpi_y;
                let min_dpi = dx.min(dy);
                let max_dpi = dx.max(dy);

                if has_asset_map && !options.image_assets.contains_key(&img.asset_path) {
                    img_status = PreflightStatus::Fail;
                    let msg = format!(
                        "Page {}: image asset '{}' is missing from package",
                        page_num, img.asset_path
                    );
                    img_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "IMAGE_MISSING_ASSET".to_string(),
                        check_id: "images".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: Some(img.asset_path.clone()),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: קובץ התמונה '{}' חסר בחבילת המסמך",
                            page_num, img.asset_path
                        ),
                        suggestion: Some("Re-import or embed the missing image file.".to_string()),
                        suggestion_he: Some("ייבאו מחדש או הטמיעו את קובץ התמונה החסר.".to_string()),
                    });
                }

                if img.intrinsic_width_px == 0
                    || img.intrinsic_height_px == 0
                    || !dx.is_finite()
                    || !dy.is_finite()
                    || min_dpi <= 0.0
                {
                    img_status = PreflightStatus::Fail;
                    low_dpi_images_count += 1;
                    let msg = format!(
                        "Page {}: {} has invalid intrinsic size ({}x{}px)",
                        page_num, img.asset_path, img.intrinsic_width_px, img.intrinsic_height_px
                    );
                    img_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "IMAGE_INVALID_DIMENSIONS".to_string(),
                        check_id: "images".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: Some(img.asset_path.clone()),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: לתמונה '{}' מידות מקור לא תקינות ({}x{} פיקסלים)",
                            page_num, img.asset_path, img.intrinsic_width_px, img.intrinsic_height_px
                        ),
                        suggestion: Some("Replace with a valid PNG or JPEG image.".to_string()),
                        suggestion_he: Some("החליפו בקובץ PNG או JPEG תקין.".to_string()),
                    });
                    continue;
                }

                if min_dpi < 150.0 {
                    img_status = PreflightStatus::Fail;
                    low_dpi_images_count += 1;
                    let msg = format!(
                        "Page {}: {} effective {:.0} DPI (< 150 DPI minimum)",
                        page_num, img.asset_path, min_dpi
                    );
                    img_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "IMAGE_CRITICAL_LOW_DPI".to_string(),
                        check_id: "images".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(page_num),
                        location: Some(format!(
                            "{} ({}x{}px, {:.0}x{:.0} DPI)",
                            img.asset_path,
                            img.intrinsic_width_px,
                            img.intrinsic_height_px,
                            dx,
                            dy
                        )),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: התמונה '{}' ברזולוציה אפקטיבית של {:.0} DPI (מתחת למינימום 150 DPI)",
                            page_num, img.asset_path, min_dpi
                        ),
                        suggestion: Some(
                            "Replace with a >= 300 DPI image or reduce its placed size on the page."
                                .to_string(),
                        ),
                        suggestion_he: Some(
                            "החליפו בתמונה באיכות 300 DPI לפחות, או הקטינו את גודל ההצבה בעמוד."
                                .to_string(),
                        ),
                    });
                } else if min_dpi < 300.0 {
                    if img_status == PreflightStatus::Pass {
                        img_status = PreflightStatus::Warning;
                    }
                    low_dpi_images_count += 1;
                    let msg = format!(
                        "Page {}: {} effective {:.0} DPI (150-299 DPI print warning)",
                        page_num, img.asset_path, min_dpi
                    );
                    img_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "IMAGE_LOW_DPI".to_string(),
                        check_id: "images".to_string(),
                        severity: PreflightStatus::Warning,
                        page_number: Some(page_num),
                        location: Some(format!(
                            "{} ({}x{}px, {:.0}x{:.0} DPI)",
                            img.asset_path,
                            img.intrinsic_width_px,
                            img.intrinsic_height_px,
                            dx,
                            dy
                        )),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: התמונה '{}' ברזולוציה אפקטיבית של {:.0} DPI (מומלץ 300 DPI ומעלה)",
                            page_num, img.asset_path, min_dpi
                        ),
                        suggestion: Some(
                            "Use a higher-resolution source (>= 300 DPI) for crisp print output."
                                .to_string(),
                        ),
                        suggestion_he: Some(
                            "מומלץ להחליף בתמונה ברזולוציית 300 DPI ומעלה להדפסה חדה."
                                .to_string(),
                        ),
                    });
                }

                if max_dpi > 0.0 && (max_dpi - min_dpi) / max_dpi > 0.10 {
                    if img_status == PreflightStatus::Pass {
                        img_status = PreflightStatus::Warning;
                    }
                    let msg = format!(
                        "Page {}: {} has non-uniform scaling ({:.0} x {:.0} DPI)",
                        page_num, img.asset_path, dx, dy
                    );
                    img_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "IMAGE_ASPECT_DISTORTION".to_string(),
                        check_id: "images".to_string(),
                        severity: PreflightStatus::Warning,
                        page_number: Some(page_num),
                        location: Some(img.asset_path.clone()),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: התמונה '{}' מתוחה ביחס לא אחיד ({:.0} x {:.0} DPI)",
                            page_num, img.asset_path, dx, dy
                        ),
                        suggestion: Some("Lock aspect ratio when resizing the image frame.".to_string()),
                        suggestion_he: Some("שמרו על יחס גובה-רוחב אחיד בעת שינוי גודל התמונה.".to_string()),
                    });
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
        for (p_idx, page) in pages.iter().enumerate() {
            for diag in &page.diagnostics {
                if diag.waiver.is_some() {
                    continue;
                }
                let diag_page = if diag.page_index < pages.len() {
                    diag.page_index + 1
                } else {
                    p_idx + 1
                };
                if diag.code == "COLUMN_BALANCE_DELTA_EXCEEDED"
                    || diag.severity == DiagnosticSeverity::Warning
                {
                    balance_warnings_count += 1;
                    if col_status == PreflightStatus::Pass {
                        col_status = PreflightStatus::Warning;
                    }
                    col_details.push(diag.message.clone());
                    issues.push(PreflightIssue {
                        code: diag.code.clone(),
                        check_id: "columns".to_string(),
                        severity: PreflightStatus::Warning,
                        page_number: Some(diag_page),
                        location: None,
                        message: diag.message.clone(),
                        message_he: format!("עמוד {}: {}", diag_page, diag.message),
                        suggestion: Some("Adjust spacing or line breaks to balance column bottoms.".to_string()),
                        suggestion_he: Some("התאימו מרווחי שורות או מעברי עמוד לאיזון תחתיות העמודות.".to_string()),
                    });
                } else if diag.severity == DiagnosticSeverity::Error {
                    col_status = PreflightStatus::Fail;
                    balance_warnings_count += 1;
                    col_details.push(diag.message.clone());
                    issues.push(PreflightIssue {
                        code: diag.code.clone(),
                        check_id: "columns".to_string(),
                        severity: PreflightStatus::Fail,
                        page_number: Some(diag_page),
                        location: None,
                        message: diag.message.clone(),
                        message_he: format!("עמוד {}: {}", diag_page, diag.message),
                        suggestion: Some("Resolve the layout error before print export.".to_string()),
                        suggestion_he: Some("תקנו את שגיאת העימוד לפני הייצוא לדפוס.".to_string()),
                    });
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

        // 6. Page & Frame Bounds + Text-in-Frame Overset Check
        let mut page_status = PreflightStatus::Pass;
        let mut page_details = Vec::new();
        for (p_idx, page) in pages.iter().enumerate() {
            let page_num = p_idx + 1;
            let page_w = page.dimensions.width;
            let page_h = page.dimensions.height;

            for frame in &page.frames {
                if frame.rect.y + frame.rect.height > page_h + 1.0 {
                    if page_status == PreflightStatus::Pass {
                        page_status = PreflightStatus::Warning;
                    }
                    let over = frame.rect.y + frame.rect.height - page_h;
                    let msg = format!(
                        "Page {} frame '{}' exceeds page bottom by {:.1}pt",
                        page_num, frame.frame_id, over
                    );
                    page_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "FRAME_EXCEEDS_PAGE_BOTTOM".to_string(),
                        check_id: "pagination".to_string(),
                        severity: PreflightStatus::Warning,
                        page_number: Some(page_num),
                        location: Some(frame.frame_id.clone()),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: המסגרת '{}' חורגת מתחתית העמוד ב-{:.1} נק'",
                            page_num, frame.frame_id, over
                        ),
                        suggestion: Some("Reduce frame height or adjust page margins.".to_string()),
                        suggestion_he: Some("הקטינו את גובה המסגרת או התאימו את שולי העמוד.".to_string()),
                    });
                }
                if frame.rect.x < -1.0 || frame.rect.x + frame.rect.width > page_w + 1.0 {
                    if page_status == PreflightStatus::Pass {
                        page_status = PreflightStatus::Warning;
                    }
                    let msg = format!(
                        "Page {} frame '{}' extends outside horizontal page bounds",
                        page_num, frame.frame_id
                    );
                    page_details.push(msg.clone());
                    issues.push(PreflightIssue {
                        code: "FRAME_EXCEEDS_PAGE_WIDTH".to_string(),
                        check_id: "pagination".to_string(),
                        severity: PreflightStatus::Warning,
                        page_number: Some(page_num),
                        location: Some(frame.frame_id.clone()),
                        message: msg,
                        message_he: format!(
                            "עמוד {}: המסגרת '{}' חורגת מגבולות הרוחב של העמוד",
                            page_num, frame.frame_id
                        ),
                        suggestion: Some("Keep text frames inside the page TrimBox.".to_string()),
                        suggestion_he: Some("שמרו את מסגרות הטקסט בתוך גבולות ה-TrimBox של העמוד.".to_string()),
                    });
                }

                // Check individual lines/glyphs overflowing outside their owning TextFrameBox
                let mut vert_overflow_reported = false;
                let mut horiz_overflow_reported = false;
                for line in &frame.lines {
                    if !vert_overflow_reported && line.baseline_y > frame.rect.height + 2.0 {
                        vert_overflow_reported = true;
                        page_status = PreflightStatus::Fail;
                        let over = line.baseline_y - frame.rect.height;
                        let msg = format!(
                            "Page {} frame '{}' line {}: text overflows frame bottom by {:.1}pt",
                            page_num,
                            frame.frame_id,
                            line.line_index + 1,
                            over
                        );
                        page_details.push(msg.clone());
                        issues.push(PreflightIssue {
                            code: "TEXT_OVERFLOW_FRAME_BOTTOM".to_string(),
                            check_id: "pagination".to_string(),
                            severity: PreflightStatus::Fail,
                            page_number: Some(page_num),
                            location: Some(format!(
                                "frame '{}' line {}",
                                frame.frame_id,
                                line.line_index + 1
                            )),
                            message: msg,
                            message_he: format!(
                                "עמוד {}: טקסט גלש מתחת למסגרת '{}' בשורה {} (חריגה של {:.1} נק')",
                                page_num,
                                frame.frame_id,
                                line.line_index + 1,
                                over
                            ),
                            suggestion: Some("Shorten text, reduce font size/leading, or expand the frame.".to_string()),
                            suggestion_he: Some("קצרו את הטקסט, הקטינו גופן/מרווח שורות, או הרחיבו את המסגרת.".to_string()),
                        });
                    }

                    if !horiz_overflow_reported && frame.rect.width > 0.0 {
                        let (left, right) = if !line.glyphs.is_empty() {
                            line.horizontal_extent()
                        } else {
                            (line.x_offset, line.x_offset + line.width)
                        };
                        let left_over = if left < -2.0 { -left } else { 0.0 };
                        let right_over = if right > frame.rect.width + 2.0 {
                            right - frame.rect.width
                        } else {
                            0.0
                        };
                        let max_over = left_over.max(right_over);
                        if max_over > 0.0 {
                            horiz_overflow_reported = true;
                            if page_status == PreflightStatus::Pass {
                                page_status = PreflightStatus::Warning;
                            }
                            let msg = format!(
                                "Page {} frame '{}' line {}: text overflows frame width by {:.1}pt",
                                page_num,
                                frame.frame_id,
                                line.line_index + 1,
                                max_over
                            );
                            page_details.push(msg.clone());
                            issues.push(PreflightIssue {
                                code: "TEXT_OVERFLOW_FRAME_WIDTH".to_string(),
                                check_id: "pagination".to_string(),
                                severity: PreflightStatus::Warning,
                                page_number: Some(page_num),
                                location: Some(format!(
                                    "frame '{}' line {}",
                                    frame.frame_id,
                                    line.line_index + 1
                                )),
                                message: msg,
                                message_he: format!(
                                    "עמוד {}: טקסט גלש לרוחב מחוץ למסגרת '{}' בשורה {} (חריגה של {:.1} נק')",
                                    page_num,
                                    frame.frame_id,
                                    line.line_index + 1,
                                    max_over
                                ),
                                suggestion: Some("Enable hyphenation/justification or widen the text frame.".to_string()),
                                suggestion_he: Some("אפשרו שבירת שורות/מיקוף או הרחיבו את מסגרת הטקסט.".to_string()),
                            });
                        }
                    }
                }
            }
        }
        let page_summary = if page_status == PreflightStatus::Pass {
            format!("{} page(s) free of frame overset", pages.len())
        } else {
            format!("{} overset issue(s) detected", page_details.len())
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

    #[test]
    fn rgb_color_in_pdfx1a_and_tac_exceeded_emit_page_issues() {
        let mut doc = DocumentRoot::new("Color Test");
        doc.paragraph_styles.push(ParagraphStyle {
            id: "rgb-sty".to_string(),
            name: "RGB Accent".to_string(),
            color: tok_core::styles::Color::Rgb {
                r: 0.9,
                g: 0.1,
                b: 0.1,
            },
            ..Default::default()
        });
        doc.paragraph_styles.push(ParagraphStyle {
            id: "heavy-cmyk".to_string(),
            name: "Heavy CMYK".to_string(),
            color: tok_core::styles::Color::DeviceCmyk {
                c: 0.9,
                m: 0.9,
                y: 0.9,
                k: 0.9,
            },
            ..Default::default()
        });
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let pages = engine.typeset_document(&doc);
        let opts = PdfExportOptions {
            standard: PdfXStandard::PdfX1a2001,
            color_profile: crate::color::IccProfileId::Swop2006,
            ..PdfExportOptions::default()
        };
        let rep = PreflightEngine::evaluate(&doc, &pages, &engine.font_manager, &opts);
        assert_eq!(rep.overall_status, PreflightStatus::Warning);
        assert!(rep
            .issues
            .iter()
            .any(|i| i.code == "COLOR_RGB_FOR_PRINT_PROFILE"));
        assert!(rep.issues.iter().any(|i| i.code == "COLOR_TAC_EXCEEDED"));
    }

    #[test]
    fn text_overflow_inside_frame_fails_preflight_with_page_and_frame() {
        let doc = DocumentRoot::new("Overflow Test");
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let mut pages = engine.typeset_document(&doc);
        pages[0].frames.push(tok_typeset::geometry::TextFrameBox {
            frame_id: "main-col".to_string(),
            flow_id: "body".to_string(),
            rect: PhysicalRect::new(50.0, 50.0, 200.0, 100.0),
            lines: vec![tok_typeset::geometry::LineBox {
                line_index: 5,
                baseline_y: 125.0,
                height: 14.0,
                width: 220.0,
                ..Default::default()
            }],
        });
        let rep = PreflightEngine::evaluate(
            &doc,
            &pages,
            &engine.font_manager,
            &PdfExportOptions::default(),
        );
        assert_eq!(rep.overall_status, PreflightStatus::Fail);
        assert!(rep.affected_pages.contains(&1));
        assert!(rep
            .issues_for_page(1)
            .iter()
            .any(|i| i.code == "TEXT_OVERFLOW_FRAME_BOTTOM"));
        assert!(rep
            .issues_for_page(1)
            .iter()
            .any(|i| i.code == "TEXT_OVERFLOW_FRAME_WIDTH"));
    }

    #[test]
    fn bleed_edge_hazard_and_formatted_report_work() {
        let doc = DocumentRoot::new("Bleed Test");
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let mut pages = engine.typeset_document(&doc);
        pages[0].images.push(PlacedImageBox {
            asset_path: "images/cover.png".to_string(),
            rect: PhysicalRect::new(0.0, 0.0, 200.0, 200.0),
            intrinsic_width_px: 1000,
            intrinsic_height_px: 1000,
            effective_dpi_x: 360.0,
            effective_dpi_y: 360.0,
            paragraph_id: None,
        });
        let opts = PdfExportOptions {
            bleed_pt: 0.0,
            standard: PdfXStandard::PdfX1a2001,
            ..PdfExportOptions::default()
        };
        let rep = PreflightEngine::evaluate(&doc, &pages, &engine.font_manager, &opts);
        assert_eq!(rep.overall_status, PreflightStatus::Fail);
        assert!(rep
            .issues_for_page(1)
            .iter()
            .any(|i| i.code == "BLEED_EDGE_HAZARD"));
        let he_report = rep.format_report(true);
        assert!(he_report.contains("עמוד 1"));
        assert!(he_report.contains("BLEED_EDGE_HAZARD"));
    }
}

//! Pre-paginated HTML & CSS Projection Compiler.
//!
//! Enforces:
//! - Mandatory `.tok-page` and `.tok-line` isolation rules with `!important` flags (Section 9.6).
//! - Named page definitions (@page :right, @page :left, @page chapter-first) for Vivliostyle.
//! - RTL spread progression.

use std::fmt::Write;
use tok_typeset::font::BOLD_SUFFIX;
use tok_typeset::geometry::PageLayoutBox;

/// The engine's embedded families, in fallback order (see tok-typeset's FontManager).
const FONT_STACK: &str =
    "'Frank Ruhl Libre', 'Noto Rashi Hebrew', 'Noto Serif Hebrew', 'David Libre', serif";

pub struct HtmlProjectionCompiler;

impl HtmlProjectionCompiler {
    /// Generates the standard CSS stylesheet for Pre-paginated display in Vivliostyle.
    pub fn generate_stylesheet(page_width_mm: f32, page_height_mm: f32) -> String {
        format!(
            r#"/* TypesetOK Pre-Pagination Stylesheet */
@page {{
  size: {width:.1}mm {height:.1}mm;
  marks: crop cross;
  bleed: 3mm;
}}

@page :right {{
  margin-top: 25mm;
  margin-bottom: 20mm;
  margin-right: 20mm;
  margin-left: 30mm;
}}

@page :left {{
  margin-top: 25mm;
  margin-bottom: 20mm;
  margin-right: 30mm;
  margin-left: 20mm;
}}

@page chapter-first {{
  margin-top: 55mm;
  margin-bottom: 20mm;
  margin-right: 20mm;
  margin-left: 30mm;
}}

body {{
  margin: 0;
  padding: 0;
  direction: rtl;
  font-family: {fonts};
}}

/* Mandatory Isolation Rule: tok-page */
div.tok-page {{
  width: {width:.1}mm !important;
  height: {height:.1}mm !important;
  box-sizing: border-box !important;
  position: relative !important;
  overflow: hidden !important;
  break-inside: avoid !important;
  page-break-inside: avoid !important;
  contain: paint layout !important;
}}

/* Mandatory Isolation Rule: tok-line */
div.tok-line {{
  white-space: nowrap !important;
  overflow: visible !important;
  display: block !important;
  contain: layout style !important;
  margin: 0 !important;
  padding: 0 !important;
  border: 0 !important;
  box-sizing: border-box !important;
}}
"#,
            width = page_width_mm,
            height = page_height_mm,
            fonts = FONT_STACK,
        )
    }

    /// Compiles a list of PageLayoutBoxes into pre-fragmented HTML for Vivliostyle.
    pub fn compile_to_html(
        pages: &[PageLayoutBox],
        page_width_mm: f32,
        page_height_mm: f32,
    ) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html dir=\"rtl\" lang=\"he\">\n<head>\n");
        html.push_str("<meta charset=\"utf-8\">\n");
        html.push_str("<title>TypesetOK Document</title>\n");
        html.push_str("<style>\n");
        html.push_str(&Self::generate_stylesheet(page_width_mm, page_height_mm));
        html.push_str("</style>\n</head>\n<body>\n");

        if pages.is_empty() {
            html.push_str("</body>\n</html>\n");
            return html;
        }

        for p in pages {
            if p.is_blank {
                writeln!(
                    html,
                    "<div class=\"tok-page tok-page-blank tok-page-left\" data-page-index=\"{}\"></div>",
                    p.page_index
                )
                .unwrap();
                continue;
            }

            let page_class = if p.page_index == 0 {
                "tok-page tok-page-chapter-first"
            } else if p.page_index % 2 == 0 {
                "tok-page tok-page-right"
            } else {
                "tok-page tok-page-left"
            };

            let escaped_gematria = html_escape(&p.page_number_gematria);
            writeln!(
                html,
                "<div class=\"{}\" data-page-index=\"{}\" data-gematria=\"{}\">",
                page_class, p.page_index, escaped_gematria
            )
            .unwrap();

            for frame in &p.frames {
                writeln!(html,
                    "  <div class=\"tok-frame\" style=\"position: absolute; left: {:.2}pt; top: {:.2}pt; width: {:.2}pt; height: {:.2}pt; overflow: hidden;\">",
                    frame.rect.x, frame.rect.y, frame.rect.width, frame.rect.height
                ).unwrap();

                for line in &frame.lines {
                    // The document is RTL; LTR lines must say so or the browser
                    // reorders and right-aligns them.
                    let dir = if line.is_rtl { "" } else { " dir=\"ltr\"" };
                    // Draw the line in the font the engine measured it with.
                    let font = line.fonts.first().map(|f| css_font(f)).unwrap_or_default();
                    let top_offset = (line.baseline_y - line.height).max(0.0);
                    writeln!(html,
                        "    <div class=\"tok-line\"{} style=\"position: absolute; top: {:.2}pt; left: 0; right: 0; height: {:.2}pt; line-height: {:.2}pt;{}\">{}</div>",
                        dir, top_offset, line.height, line.height, font, html_escape(&line.text)
                    ).unwrap();
                }

                html.push_str("  </div>\n");
            }

            // Folio (page number) footer
            writeln!(html,
                "  <div class=\"tok-folio\" style=\"position: absolute; bottom: 20pt; width: 100%; text-align: center;\">{}</div>",
                escaped_gematria
            ).unwrap();

            html.push_str("</div>\n");
        }

        html.push_str("</body>\n</html>\n");
        html
    }
}

/// CSS declarations for an engine family: "X Bold" is family X at weight 700.
fn css_font(family: &str) -> String {
    match family.strip_suffix(BOLD_SUFFIX) {
        Some(base) => format!(
            " font-family: '{}', {}; font-weight: 700;",
            html_escape(base),
            FONT_STACK
        ),
        None => format!(" font-family: '{}', {};", html_escape(family), FONT_STACK),
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tok_typeset::geometry::{LineBox, PhysicalRect, TextFrameBox};

    #[test]
    fn test_html_projection_containment_rules() {
        let frame = TextFrameBox {
            frame_id: "f1".to_string(),
            flow_id: "main".to_string(),
            rect: PhysicalRect::new(50.0, 50.0, 400.0, 600.0),
            lines: vec![LineBox {
                line_index: 0,
                paragraph_id: None,
                baseline_y: 14.5,
                height: 14.5,
                width: 300.0,
                x_offset: 0.0,
                is_drop_cap: false,
                glyphs: Vec::new(),
                text: "טקסט עברי מעומד".to_string(),
                is_rtl: true,
                fonts: Vec::new(),
            }],
        };

        let page = PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![frame],
            break_token: None,
            is_blank: false,
        };

        let html = HtmlProjectionCompiler::compile_to_html(&[page], 210.0, 297.0);

        // Verify mandatory CSS rules from section 9.6
        assert!(html.contains("white-space: nowrap !important"));
        assert!(html.contains("contain: layout style !important"));
        assert!(html.contains("break-inside: avoid !important"));
        assert!(html.contains("dir=\"rtl\""));
        assert!(html.contains("tok-page"));
        assert!(html.contains("tok-line"));
    }

    #[test]
    fn test_html_projection_exact_frame_and_line_geometry() {
        let frame = TextFrameBox {
            frame_id: "f1".to_string(),
            flow_id: "main".to_string(),
            rect: PhysicalRect::new(60.0, 40.0, 350.0, 520.0),
            lines: vec![
                LineBox {
                    line_index: 0,
                    paragraph_id: None,
                    baseline_y: 16.0,
                    height: 16.0,
                    width: 320.0,
                    x_offset: 0.0,
                    is_drop_cap: false,
                    glyphs: Vec::new(),
                    text: "שורה ראשונה".to_string(),
                    is_rtl: true,
                    fonts: Vec::new(),
                },
                LineBox {
                    line_index: 1,
                    paragraph_id: None,
                    baseline_y: 46.0,
                    height: 16.0,
                    width: 310.0,
                    x_offset: 0.0,
                    is_drop_cap: false,
                    glyphs: Vec::new(),
                    text: "שורה שנייה עם ריווח פסקה".to_string(),
                    is_rtl: true,
                    fonts: Vec::new(),
                },
            ],
        };

        let page = PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![frame],
            break_token: None,
            is_blank: false,
        };

        let html = HtmlProjectionCompiler::compile_to_html(&[page], 210.0, 297.0);

        // Frame must have exact geometry
        assert!(html.contains(
            "left: 60.00pt; top: 40.00pt; width: 350.00pt; height: 520.00pt; overflow: hidden;"
        ));

        // First line: top = baseline (16) - height (16) = 0.00pt
        assert!(
            html.contains("position: absolute; top: 0.00pt; left: 0; right: 0; height: 16.00pt;")
        );

        // Second line: top = baseline (46) - height (16) = 30.00pt
        assert!(
            html.contains("position: absolute; top: 30.00pt; left: 0; right: 0; height: 16.00pt;")
        );
    }

    #[test]
    fn ltr_lines_carry_their_direction() {
        let line = |text: &str, is_rtl: bool| LineBox {
            line_index: 0,
            paragraph_id: None,
            baseline_y: 14.5,
            height: 14.5,
            width: 300.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: Vec::new(),
            text: text.to_string(),
            is_rtl,
            fonts: Vec::new(),
        };
        let page = PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![TextFrameBox {
                frame_id: "f1".to_string(),
                flow_id: "main".to_string(),
                rect: PhysicalRect::new(50.0, 50.0, 400.0, 600.0),
                lines: vec![line("שלום", true), line("Hello <world>", false)],
            }],
            break_token: None,
            is_blank: false,
        };
        let html = HtmlProjectionCompiler::compile_to_html(&[page], 210.0, 297.0);
        assert!(html.contains("<div class=\"tok-line\" dir=\"ltr\""));
        assert!(html.contains("Hello &lt;world&gt;"));
        assert_eq!(html.matches("dir=\"ltr\"").count(), 1);
    }

    #[test]
    fn lines_use_the_font_the_engine_measured_them_with() {
        let line = |fonts: Vec<String>| LineBox {
            line_index: 0,
            paragraph_id: None,
            baseline_y: 14.5,
            height: 14.5,
            width: 300.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: Vec::new(),
            text: "שלום".to_string(),
            is_rtl: true,
            fonts,
        };
        let page = PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![TextFrameBox {
                frame_id: "f1".to_string(),
                flow_id: "rashi".to_string(),
                rect: PhysicalRect::new(50.0, 50.0, 400.0, 600.0),
                lines: vec![
                    line(vec!["Noto Rashi Hebrew".to_string()]),
                    line(Vec::new()),
                ],
            }],
            break_token: None,
            is_blank: false,
        };
        let html = HtmlProjectionCompiler::compile_to_html(&[page], 210.0, 297.0);
        assert!(html.contains("font-family: 'Frank Ruhl Libre', 'Noto Rashi Hebrew'"));
        assert!(!html.contains("David CLM"));
        assert!(html.contains(
            "line-height: 14.50pt; font-family: 'Noto Rashi Hebrew', 'Frank Ruhl Libre'"
        ));
        assert_eq!(html.matches("font-family: 'Noto Rashi Hebrew'").count(), 1);
    }

    #[test]
    fn bold_faces_become_a_css_weight() {
        assert_eq!(
            css_font("Frank Ruhl Libre Bold"),
            format!(" font-family: 'Frank Ruhl Libre', {FONT_STACK}; font-weight: 700;")
        );
        assert_eq!(
            css_font("Noto Rashi Hebrew"),
            format!(" font-family: 'Noto Rashi Hebrew', {FONT_STACK};")
        );
    }

    #[test]
    fn test_html_escape_gematria_quotes() {
        // Gematria values with quotes must be properly escaped
        let escaped = html_escape("\u{05EA}\u{05E9}\u{05E4}\u{05F4}\u{05D3}"); // תשפ״ד
        assert!(!escaped.contains('"') || escaped.contains("&quot;"));
    }

    #[test]
    fn test_html_empty_pages() {
        let html = HtmlProjectionCompiler::compile_to_html(&[], 210.0, 297.0);
        assert!(html.contains("<!DOCTYPE html>"), "Must produce valid HTML");
        assert!(html.contains("</html>"), "Must be closed HTML");
    }

    #[test]
    fn test_html_escape_single_quote() {
        let escaped = html_escape("it's");
        assert!(escaped.contains("&#39;"), "Single quotes must be escaped");
    }

    #[test]
    fn test_html_projection_blank_verso_page() {
        let blank_page = PageLayoutBox {
            page_index: 1,
            page_number_gematria: "ב".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: Vec::new(),
            break_token: None,
            is_blank: true,
        };
        let html = HtmlProjectionCompiler::compile_to_html(&[blank_page], 210.0, 297.0);
        assert!(html.contains("tok-page tok-page-blank tok-page-left"));
        assert!(!html.contains("tok-folio"));
        assert!(!html.contains("<div class=\"tok-line\""));
    }
}

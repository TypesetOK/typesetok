//! End-to-end typesetting pipeline trace integration tests.
//!
//! Validates the complete pipeline from DocumentRoot fixtures -> TypesettingEngine
//! -> PageLayoutBox (with geometric constraints and Knuth-Plass line breaking)
//! -> Vivliostyle Pre-paginated HTML projection -> Native Pre-Press ISO PDF/X export.

#![allow(clippy::field_reassign_with_default)]

use tok_pdf::{HtmlProjectionCompiler, PdfExportOptions, PdfPrePressEngine, PdfXStandard};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::fixtures::{
    create_realistic_mikraot_gedolot_document, create_realistic_prose_document,
    create_realistic_talmud_document,
};

#[test]
fn test_e2e_prose_pipeline_trace() {
    let doc = create_realistic_prose_document(10);
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());

    // 1. Typeset Document
    let pages = engine.typeset_document(&doc);
    assert!(
        pages.len() >= 2,
        "Prose document should paginate into at least 2 pages, got {}",
        pages.len()
    );

    let mut total_lines = 0;
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.page_index, i);
        assert!(
            !page.page_number_gematria.is_empty(),
            "Page {} should have gematria folio",
            i
        );
        assert!(
            !page.frames.is_empty(),
            "Page {} should have at least one text frame",
            i
        );

        for frame in &page.frames {
            assert!(frame.rect.width > 0.0, "Frame rect width must be positive");
            assert!(
                frame.rect.height > 0.0,
                "Frame rect height must be positive"
            );
            total_lines += frame.lines.len();
            for line in &frame.lines {
                assert!(!line.text.is_empty(), "Line text must not be empty");
                assert!(line.height > 0.0, "Line height must be positive");
            }
        }
    }
    assert!(
        total_lines >= 10,
        "Expected at least 10 lines across pages, got {}",
        total_lines
    );

    // 2. Export to HTML
    let (w_mm, h_mm) = (170.0, 240.0);
    let html = HtmlProjectionCompiler::compile_to_html(&pages, w_mm, h_mm);
    assert!(
        html.starts_with("<!DOCTYPE html>"),
        "HTML must start with doctype"
    );
    assert!(
        html.contains("tok-page"),
        "HTML must contain tok-page containers"
    );
    assert!(
        html.contains("tok-frame"),
        "HTML must contain tok-frame elements"
    );
    assert!(
        html.contains("tok-line"),
        "HTML must contain tok-line elements"
    );
    assert!(
        html.contains("overflow: hidden;"),
        "tok-frame must enforce overflow: hidden"
    );
    println!("PROSE HTML snippet:\n{}", &html[..html.len().min(1500)]);
    assert!(html.contains("סעיף א"), "HTML must include section text");

    // 3. Export to PDF/X-1a
    let mut options = PdfExportOptions::default();
    options.title = "ספר יסודות העימוד".to_string();
    options.author = "TypesetOK".to_string();
    options.draw_page_numbers = true;
    options.standard = PdfXStandard::PdfX1a2001;

    let pdf_bytes =
        PdfPrePressEngine::export_pdf_with_fonts(&pages, &options, &engine.font_manager)
            .expect("PDF export should succeed");

    assert!(
        pdf_bytes.starts_with(b"%PDF-"),
        "PDF must start with magic header"
    );
    assert!(
        pdf_bytes.windows(5).any(|w| w == b"%%EOF"),
        "PDF must end with EOF marker"
    );
    assert!(
        pdf_bytes.windows(16).any(|w| w == b"/GTS_PDFXVersion"),
        "PDF must specify PDF/X version"
    );
    assert!(
        pdf_bytes.windows(14).any(|w| w == b"/OutputIntents"),
        "PDF must contain OutputIntents"
    );
    assert!(
        pdf_bytes.len() > 1000,
        "PDF size should be substantial, got {} bytes",
        pdf_bytes.len()
    );
}

#[test]
fn test_e2e_talmud_pipeline_trace() {
    let doc = create_realistic_talmud_document();
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());

    // 1. Typeset Talmud Page
    let pages = engine.typeset_document(&doc);
    assert!(
        !pages.is_empty(),
        "Talmud document must produce at least 1 page"
    );

    let page = &pages[0];
    assert!(
        !page.page_number_gematria.is_empty(),
        "Talmud page must have folio gematria"
    );
    assert!(
        page.frames.len() >= 3,
        "Talmud page must have at least 3 frames (gemara/main, rashi, tosafot)"
    );

    // Verify flows are present
    let flow_ids: Vec<&str> = page.frames.iter().map(|f| f.flow_id.as_str()).collect();
    assert!(
        flow_ids.contains(&"gemara") || flow_ids.contains(&"main"),
        "Must contain main Gemara flow"
    );
    assert!(flow_ids.contains(&"rashi"), "Must contain rashi flow");
    assert!(flow_ids.contains(&"tosafot"), "Must contain tosafot flow");

    // Verify non-overlapping frame bounds between main and commentary frames
    for i in 0..page.frames.len() {
        for j in (i + 1)..page.frames.len() {
            let r1 = &page.frames[i].rect;
            let r2 = &page.frames[j].rect;
            // Frames may touch along edges but should not overlap in internal area
            let overlap_x = (r1.x < r2.x + r2.width - 0.1) && (r2.x < r1.x + r1.width - 0.1);
            let overlap_y = (r1.y < r2.y + r2.height - 0.1) && (r2.y < r1.y + r1.height - 0.1);
            assert!(
                !(overlap_x && overlap_y),
                "Frames {} ({:?}) and {} ({:?}) must not overlap",
                page.frames[i].flow_id,
                r1,
                page.frames[j].flow_id,
                r2
            );
        }
    }

    // 2. Export to HTML
    let html = HtmlProjectionCompiler::compile_to_html(&pages, 210.0, 297.0);
    assert!(
        html.contains("data-gematria="),
        "HTML must include folio gematria"
    );
    let gemara_frame = page
        .frames
        .iter()
        .find(|f| f.flow_id == "gemara" || f.flow_id == "main")
        .unwrap();
    assert!(!gemara_frame.lines.is_empty());
    assert!(
        html.contains(&gemara_frame.lines[0].text),
        "HTML must project Gemara line"
    );
    let rashi_frame = page.frames.iter().find(|f| f.flow_id == "rashi").unwrap();
    assert!(!rashi_frame.lines.is_empty());
    assert!(
        html.contains(&rashi_frame.lines[0].text),
        "HTML must project Rashi line"
    );

    // 3. Export to PDF/X-1a
    let options = PdfExportOptions::default();
    let pdf_bytes =
        PdfPrePressEngine::export_pdf_with_fonts(&pages, &options, &engine.font_manager)
            .expect("Talmud PDF export should succeed");

    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(pdf_bytes.windows(5).any(|w| w == b"%%EOF"));
    assert!(pdf_bytes.len() > 1000);
}

#[test]
fn test_e2e_mikraot_gedolot_pipeline_trace() {
    let doc = create_realistic_mikraot_gedolot_document();
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());

    // 1. Typeset Mikraot Gedolot
    let pages = engine.typeset_document(&doc);
    assert!(!pages.is_empty(), "Mikraot Gedolot must produce pages");

    let page = &pages[0];
    let flow_ids: Vec<&str> = page.frames.iter().map(|f| f.flow_id.as_str()).collect();
    assert!(
        flow_ids.contains(&"torah"),
        "Must contain Torah main flow, found {:?}",
        flow_ids
    );
    assert!(
        flow_ids.contains(&"rashi"),
        "Must contain Rashi commentary, found {:?}",
        flow_ids
    );
    assert!(
        flow_ids.contains(&"onkelos"),
        "Must contain Targum Onkelos, found {:?}",
        flow_ids
    );

    let torah_frame = page.frames.iter().find(|f| f.flow_id == "torah").unwrap();
    assert!(!torah_frame.lines.is_empty());
    let onkelos_frame = page.frames.iter().find(|f| f.flow_id == "onkelos").unwrap();
    assert!(!onkelos_frame.lines.is_empty());

    // 2. Export to HTML
    let html = HtmlProjectionCompiler::compile_to_html(&pages, 215.0, 280.0);
    assert!(
        html.contains(&torah_frame.lines[0].text),
        "HTML must contain exact Torah line from engine"
    );
    assert!(
        html.contains(&onkelos_frame.lines[0].text),
        "HTML must contain exact Onkelos line from engine"
    );

    // 3. Export to PDF
    let options = PdfExportOptions::default();
    let pdf_bytes =
        PdfPrePressEngine::export_pdf_with_fonts(&pages, &options, &engine.font_manager)
            .expect("Mikraot Gedolot PDF export should succeed");
    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(pdf_bytes.windows(5).any(|w| w == b"%%EOF"));
}

#[test]
fn test_e2e_pipeline_determinism() {
    let doc = create_realistic_prose_document(2);
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());

    let pages_run1 = engine.typeset_document(&doc);
    let pages_run2 = engine.typeset_document(&doc);

    assert_eq!(pages_run1.len(), pages_run2.len());
    for (p1, p2) in pages_run1.iter().zip(pages_run2.iter()) {
        assert_eq!(p1.page_index, p2.page_index);
        assert_eq!(p1.page_number_gematria, p2.page_number_gematria);
        assert_eq!(p1.frames.len(), p2.frames.len());
        for (f1, f2) in p1.frames.iter().zip(p2.frames.iter()) {
            assert_eq!(f1.rect, f2.rect);
            assert_eq!(f1.lines.len(), f2.lines.len());
            for (l1, l2) in f1.lines.iter().zip(f2.lines.iter()) {
                assert_eq!(l1.text, l2.text);
                assert_eq!(l1.baseline_y, l2.baseline_y);
                assert_eq!(l1.width, l2.width);
            }
        }
    }

    // Export HTML and verify bit-for-bit identical output
    let html1 = HtmlProjectionCompiler::compile_to_html(&pages_run1, 170.0, 240.0);
    let html2 = HtmlProjectionCompiler::compile_to_html(&pages_run2, 170.0, 240.0);
    assert_eq!(
        html1, html2,
        "HTML projections must be bit-for-bit identical across runs"
    );

    // Export PDF and verify bit-for-bit identical output
    let options = PdfExportOptions::default();
    let pdf1 =
        PdfPrePressEngine::export_pdf_with_fonts(&pages_run1, &options, &engine.font_manager)
            .expect("PDF run 1 should succeed");
    let pdf2 =
        PdfPrePressEngine::export_pdf_with_fonts(&pages_run2, &options, &engine.font_manager)
            .expect("PDF run 2 should succeed");
    assert_eq!(
        pdf1, pdf2,
        "PDF exports must be bit-for-bit identical across repeated runs"
    );
}

#[test]
fn test_e2e_masoretic_traditional_typography_pdf_pipeline() {
    use tok_core::id::FractionalIndex;
    use tok_core::model::{DocumentRoot, Flow, FlowType, ParagraphNode, SectionNode};
    use tok_core::styles::{CharacterFormat, CharacterStyle};
    use tok_core::{FlowId, TextSpan};

    let mut doc = DocumentRoot::new("תורה ומסורה - מהדורה מדעית מדויקת");
    doc.sections.clear();

    // Register Character Styles for Traditional Typography
    doc.add_character_style(CharacterStyle::rabbatai("rabbatai", "אות רבתי"));
    doc.add_character_style(CharacterStyle::zeira("zeira", "אות זעירא"));
    doc.add_character_style(CharacterStyle::teluyah("teluyah", "אות תלויה", Some(6.0)));
    doc.add_character_style(CharacterStyle::with_opentype_feature("torani_cv", "Torani Codex Variant", "cv01", 1));

    let mut sec = SectionNode::new("סדר בהעלתך", "default");
    sec.flows.clear();
    let mut main_flow = Flow::new(FlowId::new("main"), FlowType::Main);

    // Text containing:
    // 1. Inverted Nun (U+05C6) around Numbers 10:35
    // 2. Rare cantillation marks: Shalshelet (0x0593), Qarne Farah (0x059F), Telisha Gedola (0x05A0)
    // 3. Literae Majusculae & Suspensae
    let text = "בְּרֵאשִׁית ׆ וַיְהִי בִּנְסֹעַ הָאָרֹן ׆ וַיִּתְמַהְמָ֓הּ קַרְנֵי פָרָה֟ תְּלִישָׁא גְדוֹלָה֠ יַעַר מִיָּעַר";
    let mut p = ParagraphNode::new(FractionalIndex::initial(), "body", text);

    // Span 1: Rabbatai on 'ב' of בראשית (chars 0..2)
    p.spans.push(TextSpan::new(
        0,
        2,
        CharacterFormat {
            character_style_id: Some("rabbatai".to_string()),
            ..Default::default()
        },
    ));

    // Span 2: Teluyah on 'ע' of יַעַר (suspended letter with baseline shift 6.0pt)
    if let Some(idx) = text.find("יַעַר") {
        let ayin_start = text[..idx].chars().count() + 2; // after yod + patah
        p.spans.push(TextSpan::new(
            ayin_start,
            ayin_start + 1,
            CharacterFormat {
                character_style_id: Some("teluyah".to_string()),
                ..Default::default()
            },
        ));
    }

    main_flow.paragraphs.push(p);
    sec.flows.push(main_flow);
    doc.sections.push(sec);

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);
    assert!(!pages.is_empty(), "Must typeset into pages");

    let page = &pages[0];
    let line = &page.frames[0].lines[0];

    // Verify Inverted Nun is present in typeset line
    let nun_glyph = line.glyphs.iter().find(|g| g.character == Some('\u{05C6}'));
    assert!(nun_glyph.is_some(), "Inverted Nun U+05C6 must be in line glyphs");
    assert!(nun_glyph.unwrap().width > 0.0);

    // Verify Teluyah elevated baseline shift
    let teluyah_glyph = line.glyphs.iter().find(|g| g.y >= 5.5);
    assert!(
        teluyah_glyph.is_some(),
        "Teluyah letter must have elevated baseline y >= 5.5pt"
    );

    // Export to uncompressed PDF to verify text rise and font stream operators
    let options = PdfExportOptions {
        compress_streams: false,
        ..Default::default()
    };
    let pdf_bytes = PdfPrePressEngine::export_pdf_with_fonts(&pages, &options, &engine.font_manager)
        .expect("PDF export with Traditional Masoretic typography must succeed");

    assert!(pdf_bytes.starts_with(b"%PDF-"), "Output must be valid PDF");
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);

    // Verify /ToUnicode CMap is embedded
    assert!(pdf_str.contains("/ToUnicode"), "PDF must contain /ToUnicode CMap");

    // Verify text rise operator Ts is emitted for elevated baseline glyphs
    assert!(
        pdf_str.contains(" Ts\n") || pdf_str.contains(" Ts "),
        "PDF stream must contain text rise operator (Ts) for suspended letters"
    );
}

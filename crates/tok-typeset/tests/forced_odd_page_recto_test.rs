//! Comprehensive Integration Test for Forced Odd Page (Recto) Opening
//! and Automatic Blank Verso Page Injection (נושא 7.1 / סעיף 50).
//!
//! Verifies:
//! 1. Sections marked with `start_on_odd_page` or `PageBreakMode::OddPage` always start on an odd page (Recto).
//! 2. When preceding content ends on an odd page, an intentional Blank Verso Page (`is_blank: true`) is injected.
//! 3. When preceding content ends on an even page, the next section opens directly on the next odd page without injecting a blank page.
//! 4. Heading paragraphs with `force_odd_page_break_before` or styles with `force_odd_page` force odd opening.
//! 5. Document-wide setting `chaptersStartOnOddPage: true` enforces this across all sections.
//! 6. Page numbers (gematria) remain strictly synchronized across all pages (e.g. א, [ב חלק], ג...).
//! 7. Blank pages suppress text frames, running headers, and page number folios.

use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, ParagraphNode, SectionNode};
use tok_core::overrides::{LayoutLockOverrides, PageBreakMode};
use tok_core::styles::ParagraphStyle;
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};

fn make_para(text: &str) -> ParagraphNode {
    ParagraphNode::new(FractionalIndex::initial(), "default-body", text)
}

fn make_para_with_style(style_id: &str, text: &str) -> ParagraphNode {
    ParagraphNode::new(FractionalIndex::initial(), style_id, text)
}

#[test]
fn test_forced_odd_page_section_injects_blank_verso_page() {
    let mut doc = DocumentRoot::new("שו\"ת אור עולם");

    // Section 1 (סימן א) - Short, fits entirely on page 1 (Recto)
    let mut sec1 = SectionNode::new("סימן א", "default");
    sec1.main_flow_mut().unwrap().paragraphs = vec![
        make_para("סימן א - דיני השכמת הבוקר בהלכה ובאגדה."),
        make_para("יתגבר כארי לעמוד בבוקר לעבודת בוראו שיהא הוא מעורר את השחר."),
    ];

    // Section 2 (סימן ב) - Configured to start on an odd page (Recto)
    let mut sec2 = SectionNode::new("סימן ב", "default").with_start_on_odd_page(true);
    sec2.main_flow_mut().unwrap().paragraphs = vec![
        make_para("סימן ב - דיני לבישת בגדים ועטיפת טלית."),
        make_para("ילבש בגדיו בדרך צניעות ולא ילבש שני בגדים בבת אחת."),
    ];

    doc.sections = vec![sec1, sec2];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    // Section 1 took Page 1.
    // Section 2 requires an odd page.
    // Page 2 would be even (Verso), so Page 2 must be an injected blank verso page!
    // Section 2 must open on Page 3 (Recto).
    assert_eq!(
        pages.len(),
        3,
        "Expected exactly 3 pages: Page 1 (סימן א), Page 2 (עמוד חלק), Page 3 (סימן ב)"
    );

    // Verify Page 1
    assert_eq!(pages[0].page_index, 0);
    assert_eq!(pages[0].page_number_gematria, "א׳");
    assert!(!pages[0].is_blank);
    assert!(!pages[0].frames.is_empty());
    assert!(pages[0].frames[0].lines[0].text.contains("סימן א"));

    // Verify Page 2 (Blank Verso Page)
    assert_eq!(pages[1].page_index, 1);
    assert_eq!(pages[1].page_number_gematria, "ב׳");
    assert!(
        pages[1].is_blank,
        "Page 2 must be marked as an injected blank page (Blank Verso Page)"
    );
    assert!(
        pages[1].frames.is_empty(),
        "Blank page must have no content frames"
    );

    // Verify Page 3 (Recto opening of Section 2)
    assert_eq!(pages[2].page_index, 2);
    assert_eq!(pages[2].page_number_gematria, "ג׳");
    assert!(!pages[2].is_blank);
    assert!(!pages[2].frames.is_empty());
    assert!(pages[2].frames[0].lines[0].text.contains("סימן ב"));
}

#[test]
fn test_even_ending_section_does_not_inject_spurious_blank_page() {
    let mut doc = DocumentRoot::new("ביאורי הלכה");

    // Section 1 - Many paragraphs that fill Page 1 and overflow into Page 2 (ending on an even page)
    let mut sec1 = SectionNode::new("סימן א", "default");
    for i in 0..14 {
        sec1.main_flow_mut().unwrap().paragraphs.push(make_para(
            &format!("פסקה מספר {} בביאור סימן ראשון עם דינים וטעמים בארוכה וברוחב עיוני מעמיק להלכה ולמעשה. ", i).repeat(5)
        ));
    }

    // Section 2 - Configured to start on an odd page
    let mut sec2 = SectionNode::new("סימן ב", "default").with_start_on_odd_page(true);
    sec2.main_flow_mut().unwrap().paragraphs = vec![
        make_para("סימן ב - דיני הנהגת אדם בבוקר."),
    ];

    doc.sections = vec![sec1, sec2];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    // Verify Section 1 takes 2 pages (ending on page 2)
    assert_eq!(pages.len(), 3);
    assert!(!pages[0].is_blank);
    assert!(!pages[1].is_blank);

    // Since Section 1 filled pages 1 and 2, Section 1 ends on Page 2 (even / Verso).
    // The next consecutive page is Page 3, which is ALREADY odd (Recto)!
    // No blank page should be injected!
    assert!(
        !pages[2].is_blank,
        "Page 3 must NOT be blank because page 2 was already even"
    );
    assert!(pages[2].frames[0].lines[0].text.contains("סימן ב"));
}

#[test]
fn test_paragraph_style_force_odd_page_opens_on_recto() {
    let mut doc = DocumentRoot::new("ספר הלכות קצובות");

    // Add a heading style that forces odd page
    let heading_style = ParagraphStyle {
        id: "siman-heading".to_string(),
        name: "כותרת סימן".to_string(),
        font_size_pt: 16.0,
        line_height_pt: 20.0,
        force_odd_page: true,
        break_before: PageBreakMode::OddPage,
        ..Default::default()
    };
    doc.paragraph_styles.push(heading_style);

    let mut sec = SectionNode::new("חלק א", "default");
    let p1 = make_para_with_style("siman-heading", "סימן א - כותרת ראשית הפותחת את הספר.");
    let p2 = make_para("תוכן סימן א גוף ההלכה.");
    let p3 = make_para_with_style("siman-heading", "סימן ב - כותרת סימן ב הפותחת בעמוד ימני אי-זוגי.");
    let p4 = make_para("תוכן סימן ב גוף ההלכה.");

    sec.main_flow_mut().unwrap().paragraphs = vec![p1, p2, p3, p4];
    doc.sections = vec![sec];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    // P1 and P2 fit on Page 1 (odd).
    // P3 has style `force_odd_page: true`.
    // Page 2 must be an injected blank verso page!
    // P3 must start on Page 3 (Recto).
    assert_eq!(pages.len(), 3);
    assert!(!pages[0].is_blank);
    assert!(pages[0].frames[0].lines[0].text.contains("סימן א"));

    assert!(pages[1].is_blank);
    assert_eq!(pages[1].page_number_gematria, "ב׳");

    assert!(!pages[2].is_blank);
    assert_eq!(pages[2].page_number_gematria, "ג׳");
    assert!(pages[2].frames[0].lines[0].text.contains("סימן ב"));
}

#[test]
fn test_document_settings_chapters_start_on_odd_page() {
    let mut doc = DocumentRoot::new("שולחן ערוך");

    // Configure document settings to force all chapters/sections onto odd pages
    let mut settings = serde_json::Map::new();
    settings.insert("chaptersStartOnOddPage".to_string(), serde_json::Value::Bool(true));
    doc.metadata.document_settings = Some(serde_json::Value::Object(settings));

    let mut sec1 = SectionNode::new("סימן א", "default");
    sec1.main_flow_mut().unwrap().paragraphs = vec![make_para("הלכות השכמת הבוקר סימן א.")];

    let mut sec2 = SectionNode::new("סימן ב", "default");
    sec2.main_flow_mut().unwrap().paragraphs = vec![make_para("הלכות לבישת בגדים סימן ב.")];

    doc.sections = vec![sec1, sec2];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    assert_eq!(pages.len(), 3);
    assert!(!pages[0].is_blank);
    assert!(pages[1].is_blank, "Document setting must force blank verso insertion before Section 2");
    assert!(!pages[2].is_blank);
    assert!(pages[2].frames[0].lines[0].text.contains("סימן ב"));
}

#[test]
fn test_layout_lock_override_odd_page_break_before() {
    let mut doc = DocumentRoot::new("פסקי תשובות");
    let mut sec = SectionNode::new("ראשי", "default");

    let p1 = make_para("סימן א תוכן קצר.");
    let mut p2 = make_para("סימן ב תוכן עם נעילת עמוד אי-זוגי ידנית.");
    p2.overrides = Some(LayoutLockOverrides::odd_page_break_before());

    sec.main_flow_mut().unwrap().paragraphs = vec![p1, p2];
    doc.sections = vec![sec];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    assert_eq!(pages.len(), 3);
    assert!(!pages[0].is_blank);
    assert!(pages[1].is_blank);
    assert!(!pages[2].is_blank);
    assert_eq!(pages[2].page_number_gematria, "ג׳");
}

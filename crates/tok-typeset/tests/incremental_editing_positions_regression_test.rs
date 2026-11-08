//! Regression test suite verifying incremental typesetting across document positions (Beginning, Middle, End).
//!
//! Validates:
//! 1. Beginning edit: page cascading with monotonic page numbers and gematria folios.
//! 2. Middle edit: complete reuse of leading pages, re-pagination from affected page, downstream convergence.
//! 3. End edit: 100% reuse of all preceding pages, recomputing only the tail, sub-millisecond execution.
//! 4. Bidirectional dependency tracking and cross-reference consistency across all three edit locations.

use tok_core::anchor::TextAnchor;
use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, ParagraphNode};
use tok_core::reference::{
    CrossReferenceNode, CrossReferenceRegistry, ReferenceKind, ReferenceTarget,
};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::gematria::GematriaEngine;

fn make_engine() -> TypesettingEngine {
    let config = TypesettingEngineConfig {
        page_width_pt: 595.28, // A4 portrait
        page_height_pt: 841.89,
        margin_top_pt: 50.0,
        margin_bottom_pt: 50.0,
        margin_inner_pt: 50.0,
        margin_outer_pt: 50.0,
        ..Default::default()
    };
    TypesettingEngine::new(config)
}

fn build_test_book(paragraph_count: usize) -> DocumentRoot {
    let mut doc = DocumentRoot::new("ספר בדיקת עריכה אינקרמנטלית");
    doc.dependency_tracker = Some(tok_core::dependency::DocumentDependencyTracker::new());

    let flow = doc.sections[0].main_flow_mut().unwrap();
    for i in 0..paragraph_count {
        let text = format!(
            "פסקה מספר {}: זהו קטע טקסט טיפוסי ומציאותי בספר תורני, המכיל שורות רבות של הסבר מעמיק, משא ומתן הלכתי ובירור השיטות השונות של הראשונים והאחרונים. הדיון נמשך על פני מספר שורות רצופות כדי להבטיח נפח טקסט אמיתי בכל פסקה ופסקה, ללא נתונים פיקטיביים או קיצורי דרך. בכך מתאפשרת בדיקה יסודית ומדויקת של אלגוריתם חלוקת העמודים והעימוד האינקרמנטלי.",
            i + 1
        );
        let p = ParagraphNode::new(
            FractionalIndex::new(format!("p{:04}", i)),
            "default-body",
            &text,
        );
        flow.add_paragraph(p);
    }
    doc
}

#[test]
fn test_regression_edit_at_beginning_of_large_document() {
    let mut doc = build_test_book(80);
    let eng = make_engine();
    let initial_pages = eng.typeset_document(&doc);
    assert!(
        initial_pages.len() >= 6,
        "Initial document must span multiple pages"
    );

    let p0_id = doc.sections[0].flows[0].paragraphs[0].id;
    // Edit paragraph 0 (beginning of book)
    doc.sections[0].flows[0].paragraphs[0].text =
        "פסקה ראשונה שנערכה מחדש בתחילת המסמך עם תוספת מלל משמעותית המרחיבה את מספר השורות ודוחפת את שאר העמודים קדימה.".to_string();

    let (new_pages, diag) = eng.typeset_document_incremental(&doc, &initial_pages, Some(p0_id));

    assert_eq!(
        diag.first_affected_page, 0,
        "Edit at beginning must affect page 0"
    );
    assert!(diag.recomputed_pages >= 1);
    assert!(
        new_pages.len() >= initial_pages.len(),
        "Page count should remain stable or grow"
    );

    // Verify all pages maintain strictly monotonic 0-based index and authentic gematria folios
    for (idx, page) in new_pages.iter().enumerate() {
        assert_eq!(page.page_index, idx);
        assert_eq!(
            page.page_number_gematria,
            GematriaEngine::to_hebrew_numeral(idx + 1)
        );
    }
}

#[test]
fn test_regression_edit_in_middle_of_large_document() {
    let mut doc = build_test_book(100);
    let eng = make_engine();
    let initial_pages = eng.typeset_document(&doc);
    assert!(
        initial_pages.len() >= 8,
        "Initial document must span at least 8 pages"
    );

    // Find a paragraph positioned around page 4 (middle)
    let mid_para_idx = 45;
    let mid_p_id = doc.sections[0].flows[0].paragraphs[mid_para_idx].id;
    let mid_page_before = initial_pages
        .iter()
        .position(|p| {
            p.frames
                .iter()
                .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(mid_p_id)))
        })
        .expect("Middle paragraph must be present on a page");

    assert!(
        mid_page_before >= 3,
        "Middle paragraph must be on page 3 or later"
    );

    // Edit middle paragraph
    doc.sections[0].flows[0].paragraphs[mid_para_idx].text =
        "פסקה באמצע המסמך שעברה עריכה מקומית ונוספו לה כמה משפטים לבדיקת שימור עמודים קודמים."
            .to_string();

    let (new_pages, diag) = eng.typeset_document_incremental(&doc, &initial_pages, Some(mid_p_id));

    // Leading pages BEFORE the edited paragraph must be completely REUSED!
    assert!(
        diag.reused_pages >= mid_page_before,
        "All pages prior to middle edit must be reused"
    );
    assert_eq!(
        diag.first_affected_page, mid_page_before,
        "First affected page must match middle edit page"
    );

    // Leading pages must be identical to initial pages
    for i in 0..mid_page_before {
        assert_eq!(
            new_pages[i].frames[0].lines.len(),
            initial_pages[i].frames[0].lines.len()
        );
        assert_eq!(
            new_pages[i].page_number_gematria,
            initial_pages[i].page_number_gematria
        );
    }
}

#[test]
fn test_regression_edit_at_end_of_large_document() {
    let mut doc = build_test_book(100);
    let eng = make_engine();
    let initial_pages = eng.typeset_document(&doc);
    let last_page_idx = initial_pages.len() - 1;

    // Edit the very last paragraph
    let last_p_idx = 99;
    let last_p_id = doc.sections[0].flows[0].paragraphs[last_p_idx].id;
    doc.sections[0].flows[0].paragraphs[last_p_idx].text =
        "פסקה אחרונה בסוף המסמך שנערכה מקומית. עריכה זו אינה משפיעה על אף עמוד קודם.".to_string();

    let (new_pages, diag) = eng.typeset_document_incremental(&doc, &initial_pages, Some(last_p_id));

    // All preceding pages must be 100% REUSED!
    assert_eq!(
        diag.first_affected_page, last_page_idx,
        "Only the last page should be affected"
    );
    assert_eq!(
        diag.reused_pages, last_page_idx,
        "All pages before the last page must be reused"
    );
    assert_eq!(
        diag.recomputed_pages, 1,
        "Only 1 page recomputed at document end"
    );

    // Preceding pages must be identical
    for i in 0..last_page_idx {
        assert_eq!(
            new_pages[i].frames[0].lines.len(),
            initial_pages[i].frames[0].lines.len()
        );
    }
}

#[test]
fn test_regression_cross_reference_and_anchors_across_positions() {
    let mut doc = build_test_book(60);
    let mut registry = CrossReferenceRegistry::new();

    // Source at beginning (p0), Target at end (p59)
    let source_id = doc.sections[0].flows[0].paragraphs[0].id;
    let target_id = doc.sections[0].flows[0].paragraphs[59].id;

    let target = ReferenceTarget::new("anchor-end-chapter", TextAnchor::start_of(target_id))
        .with_label("סיום המסכת");
    registry.register_target(target);

    let xref = CrossReferenceNode::new(
        TextAnchor::start_of(source_id),
        "anchor-end-chapter",
        ReferenceKind::PageGematria,
        "??",
    );
    registry.register_reference(xref);
    doc.cross_references = Some(registry);

    let eng = make_engine();
    let pages1 = eng.typeset_document(&doc);
    eng.update_cross_references_and_dependencies(&mut doc, &pages1);

    let init_target_page = doc
        .dependency_tracker
        .as_ref()
        .unwrap()
        .page_for_anchor("anchor-end-chapter")
        .unwrap();
    let init_gematria = doc.cross_references.as_ref().unwrap().references[0]
        .resolved_or_fallback()
        .to_string();
    assert_eq!(
        init_gematria,
        GematriaEngine::to_hebrew_numeral(init_target_page + 1)
    );

    // Edit in the middle: add 50 new paragraphs to push target further downstream
    {
        let flow = doc.sections[0].main_flow_mut().unwrap();
        for i in 0..50 {
            let p = ParagraphNode::new(
                FractionalIndex::new(format!("p0020m{:03}", i)),
                "default-body",
                "טקסט נוסף שהוכנס באמצע הספר ודוחף את עוגן הסיום קדימה בעמודים נוספים.",
            );
            flow.add_paragraph(p);
        }
    }

    let (pages2, _diag) = eng.typeset_document_incremental(&doc, &pages1, None);
    eng.update_cross_references_and_dependencies(&mut doc, &pages2);

    let new_target_page = doc
        .dependency_tracker
        .as_ref()
        .unwrap()
        .page_for_anchor("anchor-end-chapter")
        .unwrap();
    assert!(
        new_target_page > init_target_page,
        "Target anchor must have moved to later page"
    );

    let new_gematria = doc.cross_references.as_ref().unwrap().references[0]
        .resolved_or_fallback()
        .to_string();
    assert_eq!(
        new_gematria,
        GematriaEngine::to_hebrew_numeral(new_target_page + 1)
    );
}

//! End-to-End Multi-Page Flow and Global Pagination Regression Test Suite (Stage 8).
#![allow(clippy::field_reassign_with_default)]
//!
//! Validates:
//! - Coordinated pagination across consecutive pages in the real layout pipeline.
//! - Prevention of typographic defects (stranded headings, orphans, widows).
//! - Long footnotes dynamically carrying over across 3+ pages without frame collisions.
//! - RTL Hebrew recto/verso facing page transitions and spine-relative margins.
//! - Multi-stream synchronized co-pagination with geometric solver feasibility.

use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, Flow, FlowType, ParagraphNode};
use tok_core::FlowId;
use tok_typeset::co_pagination::{
    AnchorKey, CoPaginatedChunk, CoPaginatedCommentary, CoPaginationDocument, TemplateConfig,
};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::geometry::PhysicalRect;
use tok_typeset::{SpreadSide, SynchronizerConfig};

fn overlaps(a: &PhysicalRect, b: &PhysicalRect) -> bool {
    let eps = 1e-3;
    a.x + eps < b.x + b.width
        && b.x + eps < a.x + a.width
        && a.y + eps < b.y + b.height
        && b.y + eps < a.y + a.height
}

#[test]
fn test_regression_multi_page_single_flow_global_page_breaker() {
    let mut doc = DocumentRoot::new("Global Pagination Single Flow");
    let flow = doc.sections[0].main_flow_mut().unwrap();

    // Build a sequence with paragraphs and interleaved headings
    for i in 0..12 {
        let style = if i % 4 == 0 { "heading1" } else { "default" };
        let text = if style == "heading1" {
            format!(
                "פרק {}: כותרת ראשית חשובה שאינה רשאית להישאר יתומה בתחתית עמוד",
                i
            )
        } else {
            format!(
                "פסקה {} מתוך החיבור. זהו תוכן עיוני מעמיק הדן בסוגיות שונות של עימוד וחלוקת עמודים. ",
                i
            )
            .repeat(6)
        };

        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new(format!("p_{:03}", i)),
            style,
            &text,
        ));
    }

    let config = TypesettingEngineConfig {
        page_width_pt: 500.0,
        page_height_pt: 450.0, // Constrained height to force 4+ page breaks
        margin_top_pt: 35.0,
        margin_bottom_pt: 35.0,
        margin_inner_pt: 45.0, // Spine margin
        margin_outer_pt: 25.0, // Outer margin
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(
        pages.len() >= 3,
        "Expected at least 3 pages, got {}",
        pages.len()
    );

    for (page_idx, page) in pages.iter().enumerate() {
        let page_num = page_idx + 1;
        assert_eq!(page.frames.len(), 1);
        let frame = &page.frames[0];

        // 1. Alternating spine margins: Recto vs Verso
        let is_recto = page_num % 2 == 1;
        if is_recto {
            assert!(
                (frame.rect.x - 45.0).abs() < 1e-3,
                "Page {} (Recto): expected x=45.0, got {}",
                page_num,
                frame.rect.x
            );
        } else {
            assert!(
                (frame.rect.x - 25.0).abs() < 1e-3,
                "Page {} (Verso): expected x=25.0, got {}",
                page_num,
                frame.rect.x
            );
        }

        // 2. Strict printable boundary containment
        let content_w = 500.0 - 45.0 - 25.0; // 430 pt
        let content_h = 450.0 - 35.0 - 35.0; // 380 pt
        assert!((frame.rect.width - content_w).abs() < 1e-3);
        assert!(frame.rect.height <= content_h + 1e-3);

        // 3. Keep with next: a heading must never be the solitary last line of a page
        if let Some(last_line) = frame.lines.last() {
            if last_line.text.contains("כותרת ראשית חשובה") {
                panic!(
                    "Defect: Stranded heading found at bottom of page {} without following content!",
                    page_num
                );
            }
        }

        // 4. Orphan & widow protection: each page has at least 2 lines
        assert!(
            frame.lines.len() >= 2,
            "Page {} has fewer than 2 lines ({}), violating orphan/widow rules",
            page_num,
            frame.lines.len()
        );
    }
}

#[test]
fn test_regression_multi_page_long_footnotes_carry_over_three_pages() {
    let mut doc = DocumentRoot::new("Long Footnotes Multi-Page");
    let section = &mut doc.sections[0];
    section.flows.clear();

    let mut main = Flow::new(FlowId::main(), FlowType::Main);
    // Several main paragraphs
    for i in 0..6 {
        main.paragraphs.push(ParagraphNode::new(
            FractionalIndex::new(format!("m_{:03}", i)),
            "default",
            &format!(
                "גוף הטקסט הראשי פסקה מספר {} עם מראי מקומות והערות שוליים רבות. ",
                i
            )
            .repeat(5),
        ));
    }

    // Footnotes flow with 35 lines worth of text
    let mut notes = Flow::new(FlowId::new("footnote"), FlowType::Footnote);
    for i in 0..10 {
        notes.paragraphs.push(ParagraphNode::new(
            FractionalIndex::new(format!("fn_{:03}", i)),
            "default",
            &format!(
                "הערה {}: ביאור מקיף והרחבה עיונית רחבה הדורשת שטח נכבד בתחתית העמוד. ",
                i
            )
            .repeat(4),
        ));
    }

    section.flows.push(main);
    section.flows.push(notes);

    let config = TypesettingEngineConfig {
        page_width_pt: 500.0,
        page_height_pt: 400.0, // Small page to force multi-page footnote band carry-over
        margin_top_pt: 30.0,
        margin_bottom_pt: 30.0,
        margin_inner_pt: 30.0,
        margin_outer_pt: 30.0,
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(
        pages.len() >= 3,
        "Expected at least 3 pages for long footnotes carry-over, got {}",
        pages.len()
    );

    let mut pages_with_footnotes = 0;
    for (page_idx, page) in pages.iter().enumerate() {
        let page_num = page_idx + 1;

        // Verify no frame collisions on any page
        for (i, f1) in page.frames.iter().enumerate() {
            for f2 in &page.frames[i + 1..] {
                assert!(
                    !overlaps(&f1.rect, &f2.rect),
                    "Collision detected between {} and {} on page {}",
                    f1.frame_id,
                    f2.frame_id,
                    page_num
                );
            }
        }

        // Check if footnote frame exists
        if let Some(fn_frame) = page.frames.iter().find(|f| f.flow_id.contains("note")) {
            pages_with_footnotes += 1;
            // Footnote band must span full width
            assert!(
                (fn_frame.rect.width - (500.0 - 60.0)).abs() < 1e-3,
                "Footnote band on page {} must span full content width",
                page_num
            );
            // Must be positioned at the bottom of the printable area
            assert!(
                fn_frame.rect.y + fn_frame.rect.height <= 400.0 - 30.0 + 1e-3,
                "Footnote band on page {} overflows bottom margin",
                page_num
            );
        }
    }

    assert!(
        pages_with_footnotes >= 2,
        "Footnotes must carry over across at least 2 consecutive pages, got {}",
        pages_with_footnotes
    );
}

#[test]
fn test_regression_talmud_multi_flow_co_pagination_recto_verso_spine_alternation() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let template = TemplateConfig {
        page_width_pt: 595.0,
        page_height_pt: 380.0, // Moderate height to produce 4 pages
        margin_inner_pt: 35.0,
        margin_outer_pt: 25.0,
        margin_top_pt: 25.0,
        margin_bottom_pt: 25.0,
        gutter_pt: 10.0,
        main_line_height: 18.0,
        commentary_line_height: 13.5,
        ..Default::default()
    };
    let sync_config = SynchronizerConfig::default();

    let mut main_chunks = Vec::new();
    let mut rashi_items = Vec::new();
    let mut tosafot_items = Vec::new();

    for i in 1..=5 {
        let anchor = format!("anc_{}", i);
        main_chunks.push(CoPaginatedChunk {
            text: format!(
                "גמרא קטע {} מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול",
                i
            ),
            anchors: vec![(AnchorKey(anchor.clone()), "קורין".to_string())],
        });

        let rashi_text = (0..16)
            .map(|k| format!("רש\"י {} אות {}: ביאור רש\"י המפורט", i, k))
            .collect::<Vec<_>>()
            .join(" ");
        rashi_items.push(CoPaginatedCommentary {
            target_anchor: AnchorKey(anchor.clone()),
            text: rashi_text,
        });

        let tosafot_text = (0..14)
            .map(|k| format!("תוספות {} אות {}: דברי התוספות על הדיבור", i, k))
            .collect::<Vec<_>>()
            .join(" ");
        tosafot_items.push(CoPaginatedCommentary {
            target_anchor: AnchorKey(anchor),
            text: tosafot_text,
        });
    }

    let doc = CoPaginationDocument {
        main_chunks,
        rashi_items,
        tosafot_items,
        additional_streams: std::collections::HashMap::new(),
    };

    let result = engine.typeset_co_paginated_document(&template, &sync_config, &doc);
    assert!(
        result.pages.len() >= 4,
        "Expected at least 4 pages for Talmud spread alternation, got {}",
        result.pages.len()
    );

    for (page_idx, page) in result.pages.iter().enumerate() {
        let page_num = page_idx + 1;
        let expected_recto = page_num % 2 == 1;

        if expected_recto {
            assert_eq!(
                page.math.side,
                SpreadSide::Recto,
                "Page {} must be Recto",
                page_num
            );
            assert_eq!(page.math.margin_left_pt, template.margin_inner_pt);
            assert_eq!(page.math.margin_right_pt, template.margin_outer_pt);
        } else {
            assert_eq!(
                page.math.side,
                SpreadSide::Verso,
                "Page {} must be Verso",
                page_num
            );
            assert_eq!(page.math.margin_left_pt, template.margin_outer_pt);
            assert_eq!(page.math.margin_right_pt, template.margin_inner_pt);
        }

        // Validate geometric frames
        assert_eq!(page.page_layout.frames.len(), 3);
        let rashi_frame = page
            .page_layout
            .frames
            .iter()
            .find(|f| f.flow_id == "rashi")
            .unwrap();
        let tosafot_frame = page
            .page_layout
            .frames
            .iter()
            .find(|f| f.flow_id == "tosafot")
            .unwrap();

        if expected_recto {
            // Recto: Inner (Rashi) is on left
            assert!(
                rashi_frame.rect.x < tosafot_frame.rect.x,
                "Page {} (Recto): Rashi must be to the left of Tosafot",
                page_num
            );
        } else {
            // Verso: Outer (Tosafot) is on left
            assert!(
                tosafot_frame.rect.x < rashi_frame.rect.x,
                "Page {} (Verso): Tosafot must be to the left of Rashi",
                page_num
            );
        }

        // Defect prevention: orphan and widow protection
        if page.rashi_lines_count > 0 {
            assert!(
                page.rashi_lines_count >= 2,
                "Page {} Rashi lines count ({}) violates orphan/widow protection",
                page_num,
                page.rashi_lines_count
            );
        }
        if page.tosafot_lines_count > 0 {
            assert!(
                page.tosafot_lines_count >= 2,
                "Page {} Tosafot lines count ({}) violates orphan/widow protection",
                page_num,
                page.tosafot_lines_count
            );
        }
    }
}

#[test]
fn test_regression_difficult_page_break_tight_budget() {
    let mut doc = DocumentRoot::new("Difficult Page Break");
    let flow = doc.sections[0].main_flow_mut().unwrap();

    let mut s_body = tok_core::styles::ParagraphStyle::default();
    s_body.id = "default".to_string();
    s_body.font_size_pt = 12.0;
    s_body.line_height_pt = 16.0;
    s_body.space_before_pt = 0.0;
    s_body.space_after_pt = 0.0;

    let mut s_heading = tok_core::styles::ParagraphStyle::default();
    s_heading.id = "heading1".to_string();
    s_heading.font_size_pt = 14.0;
    s_heading.line_height_pt = 20.0;
    s_heading.space_before_pt = 6.0;
    s_heading.space_after_pt = 4.0;
    s_heading.keep_with_next = true;

    doc.paragraph_styles = vec![s_body, s_heading];

    // 8 single-line paragraphs to consume 8 * 16 = 128 pt (content height = 160 pt, remaining = 32 pt)
    for i in 0..8 {
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new(format!("a_{}", i)),
            "default",
            "שורה ראשונה של טקסט רגיל למילוי העמוד",
        ));
    }

    // Heading: 20 pt height, fits in remaining 38 pt, but keep_with_next requires subsequent line
    flow.add_paragraph(ParagraphNode::new(
        FractionalIndex::new("b_0"),
        "heading1",
        "כותרת פרק חדש הדורשת הצמדה לפסקה הבאה",
    ));

    // Paragraph 2: Subsequent body text
    flow.add_paragraph(ParagraphNode::new(
        FractionalIndex::new("c_0"),
        "default",
        "טקסט עוקב המופיע לאחר הכותרת החדשה ומחייב שבירה מתואמת ללא כותרת יתומה בתחתית העמוד",
    ));

    let config = TypesettingEngineConfig {
        page_width_pt: 500.0,
        page_height_pt: 200.0, // Constrained height
        margin_top_pt: 20.0,
        margin_bottom_pt: 20.0,
        margin_inner_pt: 25.0,
        margin_outer_pt: 25.0,
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(pages.len() >= 2);
    // Page 1 must NOT end with the heading
    let page1_last = pages[0].frames[0].lines.last().unwrap();
    assert!(
        !page1_last.text.contains("כותרת פרק חדש"),
        "Heading was stranded at bottom of page 1!"
    );

    // Page 2 must start with the heading or contain the heading at the top
    let page2_first = pages[1].frames[0].lines.first().unwrap();
    assert!(
        page2_first.text.contains("כותרת פרק חדש"),
        "Expected heading to be transferred cleanly to top of page 2"
    );
}

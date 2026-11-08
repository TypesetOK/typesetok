//! Deterministic Integration and Regression Tests for the Geometric Constraint Pipeline.
//!
//! Verifies that TypesettingEngine end-to-end frame allocation is driven by
//! the geometric simplex constraint solver across various page dimensions,
//! facing spreads, L-shaped commentary cutouts, and bottom apparatus bands.

use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, Flow, FlowType, ParagraphNode};
use tok_core::FlowId;
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::geometry::PhysicalRect;

fn overlaps(a: &PhysicalRect, b: &PhysicalRect) -> bool {
    let eps = 1e-3;
    a.x + eps < b.x + b.width
        && b.x + eps < a.x + a.width
        && a.y + eps < b.y + b.height
        && b.y + eps < a.y + a.height
}

fn contains(container: &PhysicalRect, inner: &PhysicalRect) -> bool {
    let eps = 1e-3;
    inner.x + eps >= container.x
        && inner.x + inner.width <= container.x + container.width + eps
        && inner.y + eps >= container.y
        && inner.y + inner.height <= container.y + container.height + eps
}

#[test]
fn test_end_to_end_single_flow_spine_alternation() {
    let mut doc = DocumentRoot::new("Single Flow Book");
    let flow = doc.sections[0].main_flow_mut().unwrap();

    // Add several paragraphs to create multiple pages
    for i in 0..15 {
        flow.add_paragraph(ParagraphNode::new(
            FractionalIndex::new(format!("a{:03}", i)),
            "default",
            &"זהו טקסט עברי ארוך לצורך בדיקת עימוד עמודים מרובים והיפוך שולי שדרה בין עמוד ימין לעמוד שמאל."
                .repeat(5),
        ));
    }

    let config = TypesettingEngineConfig {
        page_width_pt: 500.0,
        page_height_pt: 700.0,
        margin_top_pt: 40.0,
        margin_bottom_pt: 40.0,
        margin_inner_pt: 50.0, // Spine margin
        margin_outer_pt: 20.0, // Outer margin
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(pages.len() >= 2, "Expected multiple pages");

    for (page_idx, page) in pages.iter().enumerate() {
        assert_eq!(page.frames.len(), 1);
        let frame_rect = &page.frames[0].rect;
        let is_odd = (page_idx + 1) % 2 == 1;

        if is_odd {
            // Recto (odd page): Spine on LEFT -> margin_left = margin_inner (50.0)
            assert!(
                (frame_rect.x - 50.0).abs() < 1e-3,
                "Page {} (Recto) frame left must be 50.0, got {}",
                page_idx + 1,
                frame_rect.x
            );
        } else {
            // Verso (even page): Spine on RIGHT -> margin_left = margin_outer (20.0)
            assert!(
                (frame_rect.x - 20.0).abs() < 1e-3,
                "Page {} (Verso) frame left must be 20.0, got {}",
                page_idx + 1,
                frame_rect.x
            );
        }

        // Frame width must be 500 - 50 - 20 = 430 pt
        assert!((frame_rect.width - 430.0).abs() < 1e-3);
    }
}

#[test]
fn test_end_to_end_talmud_spread_solver_driven_placement() {
    let mut doc = DocumentRoot::new("Talmud Page");
    let section = &mut doc.sections[0];
    section.flows.clear();

    // 1. Gemara (Main)
    let mut gemara = Flow::new(FlowId::new("gemara"), FlowType::Main);
    gemara.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("g01"),
        "default",
        &"תנו רבנן כיצד מעמדין את הדף על פי משוואות גיאומטריות מדויקות.".repeat(8),
    ));

    // 2. Rashi (Commentary A)
    let mut rashi = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
    rashi.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("r01"),
        "default",
        &"פירוש רש\"י על הצד הפנימי של השדרה לאורך כל העמוד כולו.".repeat(10),
    ));

    // 3. Tosafot (Commentary B)
    let mut tosafot = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
    tosafot.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("t01"),
        "default",
        &"תוספות דיבור המתחיל על הצד החיצוני של השוליים החיצוניים.".repeat(10),
    ));

    section.flows.push(gemara);
    section.flows.push(rashi);
    section.flows.push(tosafot);

    let config = TypesettingEngineConfig {
        page_width_pt: 595.0,
        page_height_pt: 842.0,
        margin_top_pt: 36.0,
        margin_bottom_pt: 36.0,
        margin_inner_pt: 40.0,
        margin_outer_pt: 30.0,
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(!pages.is_empty());

    for page in &pages {
        let page_bounds = PhysicalRect::new(0.0, 0.0, 595.0, 842.0);

        // Verify all frames are strictly contained within page bounds
        for frame in &page.frames {
            assert!(
                contains(&page_bounds, &frame.rect),
                "Frame {} out of page bounds: {:?}",
                frame.frame_id,
                frame.rect
            );
        }

        // Verify strict non-overlap between all frames on page
        for (i, f1) in page.frames.iter().enumerate() {
            for f2 in &page.frames[i + 1..] {
                assert!(
                    !overlaps(&f1.rect, &f2.rect),
                    "Collision detected between frame {} ({:?}) and frame {} ({:?})",
                    f1.frame_id,
                    f1.rect,
                    f2.frame_id,
                    f2.rect
                );
            }
        }
    }
}

#[test]
fn test_end_to_end_talmud_l_shape_commentary_expansion_in_engine() {
    let mut doc = DocumentRoot::new("Talmud L-Shape");
    let section = &mut doc.sections[0];
    section.flows.clear();

    // Short Gemara text (finishes early on page 1)
    let mut gemara = Flow::new(FlowId::new("gemara"), FlowType::Main);
    gemara.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("g01"),
        "default",
        "קטע גמרא קצר המסתיים באמצע העמוד הראשון בלבד.",
    ));

    // Abundant Rashi commentary that expands into the freed space below Gemara
    let mut rashi = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
    rashi.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("r01"),
        "default",
        &"פירוש רש\"י המתרחב למטה מתחת לגמרא בצורת דף קלאסית עם טקסט ארוך ומלא.".repeat(25),
    ));

    // Tosafot commentary
    let mut tosafot = Flow::new(FlowId::new("tosafot"), FlowType::CommentB);
    tosafot.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("t01"),
        "default",
        &"תוספות בעמודה החיצונית הממלאת את כל הגובה הרגיל.".repeat(15),
    ));

    section.flows.push(gemara);
    section.flows.push(rashi);
    section.flows.push(tosafot);

    let config = TypesettingEngineConfig {
        page_width_pt: 595.0,
        page_height_pt: 842.0,
        margin_top_pt: 36.0,
        margin_bottom_pt: 36.0,
        margin_inner_pt: 40.0,
        margin_outer_pt: 30.0,
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(!pages.is_empty());
    let page1 = &pages[0];

    // Page 1 should contain an expansion frame
    let has_expansion = page1.frames.iter().any(|f| f.flow_id.contains("expansion"));

    assert!(
        has_expansion,
        "Page 1 must contain dynamic commentary expansion frame"
    );

    // Strict non-overlap verification
    for (i, f1) in page1.frames.iter().enumerate() {
        for f2 in &page1.frames[i + 1..] {
            assert!(
                !overlaps(&f1.rect, &f2.rect),
                "Collision between {} and {}",
                f1.frame_id,
                f2.frame_id
            );
        }
    }
}

#[test]
fn test_end_to_end_footnote_band_in_engine() {
    let mut doc = DocumentRoot::new("Footnotes Band");
    let section = &mut doc.sections[0];
    section.flows.clear();

    let mut main = Flow::new(FlowId::main(), FlowType::Main);
    main.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("m01"),
        "default",
        &"טקסט ראשי בפסקה ראשית. ".repeat(15),
    ));

    let mut notes = Flow::new(FlowId::new("footnote"), FlowType::Footnote);
    notes.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("f01"),
        "default",
        &"הערת שוליים בתחתית הדף בפס רחב. ".repeat(8),
    ));

    section.flows.push(main);
    section.flows.push(notes);

    let config = TypesettingEngineConfig {
        page_width_pt: 595.0,
        page_height_pt: 842.0,
        margin_top_pt: 36.0,
        margin_bottom_pt: 36.0,
        margin_inner_pt: 36.0,
        margin_outer_pt: 36.0,
        ..Default::default()
    };

    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(&doc);

    assert!(!pages.is_empty());
    let page1 = &pages[0];

    // Verify presence of footnote frame
    let fn_frame = page1.frames.iter().find(|f| f.flow_id.contains("note"));
    assert!(fn_frame.is_some(), "Footnote frame must exist on page 1");

    let fn_rect = &fn_frame.unwrap().rect;
    // Footnote must be positioned at bottom
    assert!(fn_rect.y > 600.0);
    // Footnote must span full content width (595 - 72 = 523 pt)
    assert!((fn_rect.width - 523.0).abs() < 1e-3);

    // No overlaps
    for (i, f1) in page1.frames.iter().enumerate() {
        for f2 in &page1.frames[i + 1..] {
            assert!(!overlaps(&f1.rect, &f2.rect));
        }
    }
}

use tok_core::anchor::{AnchorBias, TextAnchor};
use tok_core::id::{FractionalIndex, NodeId};
use tok_core::model::{DocumentRoot, Flow, FlowId, FlowType, ParagraphNode, SectionNode};
use tok_core::multi_flow_anchor::{AnchorDefinition, CommentaryLink, MultiFlowAnchorMap};
use tok_core::styles::{
    DropCapConfig, LastLineConstraintConfig, OpticalMarginConfig, ParagraphStyle, TextAlignment,
};
use tok_typeset::co_pagination::{
    AnchorKey, BalanceTargetMode, CoPaginatedChunk, CoPaginatedCommentary, CoPaginationDocument,
    CoPaginationEngine, HeightBalanceConfig, SharedColumnHeightBalancer, SynchronizerConfig,
    TemplateConfig,
};
use tok_typeset::drop_cap::HebrewDropCapShaper;
use tok_typeset::knuth_plass::{KnuthPlassBreaker, LastLineConstraint, LayoutItem, MeasureProfile};
use tok_typeset::optical_margin::OpticalMarginAligner;
use tok_typeset::polygon::GeometricPolygon;
use tok_typeset::polygon_engine::PolygonFrame;
use tok_typeset::polygon_flow::MultiColumnPolygonFlowSynthesizer;
use tok_typeset::shaper::PositionedGlyph;
use tok_typeset::{
    FontManager, LineBox, PhysicalRect, TextFrameBox, TypesettingEngine, TypesettingEngineConfig,
};

fn make_para(order: u32, style_id: &str, text: &str) -> ParagraphNode {
    ParagraphNode::new(
        FractionalIndex::new(format!("a{:04}", order)),
        style_id,
        text,
    )
}

fn link_primary_to_commentary(
    map: &mut MultiFlowAnchorMap,
    anchor_id: &str,
    primary_flow: &str,
    primary_node: NodeId,
    comm_flow: &str,
    comm_node: NodeId,
) {
    map.register_anchor(AnchorDefinition::new(
        anchor_id,
        FlowId::new(primary_flow),
        primary_node,
        TextAnchor::new(primary_node, FractionalIndex::initial(), AnchorBias::After),
    ));
    map.link_commentary(CommentaryLink::new(
        FlowId::new(comm_flow),
        comm_node,
        anchor_id,
    ));
}

// ============================================================================
// 1. MULTI-FLOW SYNCHRONIZATION, ANCHOR PROPAGATION & CO-PAGINATION
// ============================================================================

#[test]
fn test_multi_flow_cross_section_merging_and_anchor_invariants() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let mut doc = DocumentRoot::new("Multi-Flow Cross Section Test");

    // Section 1: Gemara p1 + Rashi r1 (anchored to p1)
    let mut sec1 = SectionNode::new("Sec1", "default").with_flows(Vec::new());
    let mut g_flow1 =
        Flow::new(FlowId::new("gemara"), FlowType::Main).with_placement_role("primary");
    let p1 = make_para(
        1,
        "gemara",
        "תנו רבנן מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול בתרומתן עד סוף האשמורה הראשונה דברי רבי אליעזר.",
    );
    let p1_id = p1.id;
    g_flow1.paragraphs.push(p1);

    let mut r_flow1 =
        Flow::new(FlowId::new("rashi"), FlowType::CommentA).with_placement_role("inner_margin");
    let r1 = make_para(
        1,
        "rashi",
        "מאימתי קורין את שמע. משעה שהכהנים טהורים נכנסים לאכול תרומתן והיינו מצאת הכוכבים.",
    );
    let r1_id = r1.id;
    r_flow1.paragraphs.push(r1);
    link_primary_to_commentary(&mut sec1.anchors, "a1", "gemara", p1_id, "rashi", r1_id);
    sec1.flows.push(g_flow1);
    sec1.flows.push(r_flow1);

    // Section 2: Gemara p2 + Rashi r2 (anchored to p2)
    let mut sec2 = SectionNode::new("Sec2", "default").with_flows(Vec::new());
    let mut g_flow2 =
        Flow::new(FlowId::new("gemara"), FlowType::Main).with_placement_role("primary");
    let p2 = make_para(
        2,
        "gemara",
        "וחכמים אומרים עד חצות רבן גמליאל אומר עד שיעלה עמוד השחר מעשה ובאו בניו מבית המשתה אמרו לו לא קרינו את שמע.",
    );
    let p2_id = p2.id;
    g_flow2.paragraphs.push(p2);

    let mut r_flow2 =
        Flow::new(FlowId::new("rashi"), FlowType::CommentA).with_placement_role("inner_margin");
    let r2 = make_para(
        2,
        "rashi",
        "וחכמים אומרים עד חצות. כדי להרחיק את האדם מן העבירה עשו סייג לתורה.",
    );
    let r2_id = r2.id;
    r_flow2.paragraphs.push(r2);
    link_primary_to_commentary(&mut sec2.anchors, "a2", "gemara", p2_id, "rashi", r2_id);
    sec2.flows.push(g_flow2);
    sec2.flows.push(r_flow2);

    doc.sections = vec![sec1, sec2];

    let pages = engine.typeset_document(&doc);
    assert!(!pages.is_empty());

    let mut placed_pids = std::collections::BTreeSet::new();
    for page in &pages {
        for frame in &page.frames {
            for line in &frame.lines {
                if let Some(pid) = line.paragraph_id {
                    placed_pids.insert(pid);
                }
            }
        }
    }

    assert!(placed_pids.contains(&p1_id));
    assert!(placed_pids.contains(&p2_id));
    assert!(placed_pids.contains(&r1_id));
    assert!(placed_pids.contains(&r2_id));
}

#[test]
fn test_multi_flow_intra_page_vertical_alignment_and_overflow_holdback() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let mut doc = DocumentRoot::new("Intra-page Alignment Test");
    let mut sec = SectionNode::new("Main", "default").with_flows(Vec::new());

    let mut gemara =
        Flow::new(FlowId::new("gemara"), FlowType::Main).with_placement_role("primary");
    let p1 = make_para(
        1,
        "gemara",
        &"אמר מר הלכה כרבי יהודה בערב הפסח ".repeat(14),
    );
    let p1_id = p1.id;
    let p2 = make_para(
        2,
        "gemara",
        "תניא נמי הכי האוכל מצה בערב הפסח כאילו בועל ארוסתו בבית חמיו.",
    );
    let p2_id = p2.id;
    gemara.paragraphs.push(p1);
    gemara.paragraphs.push(p2);

    let mut rashi =
        Flow::new(FlowId::new("rashi"), FlowType::CommentA).with_placement_role("inner_margin");
    let c1 = make_para(1, "rashi", "אמר מר. פירוש קצר לראש העמוד.");
    let c1_id = c1.id;
    let c2 = make_para(2, "rashi", "תניא נמי הכי. סייעתא לדברי האמורא.");
    let c2_id = c2.id;
    rashi.paragraphs.push(c1);
    rashi.paragraphs.push(c2);

    let mut anchors = MultiFlowAnchorMap::default();
    link_primary_to_commentary(&mut anchors, "a1", "gemara", p1_id, "rashi", c1_id);
    link_primary_to_commentary(&mut anchors, "a2", "gemara", p2_id, "rashi", c2_id);
    doc.multi_flow_anchors = Some(anchors);

    sec.flows.push(gemara);
    sec.flows.push(rashi);
    doc.sections = vec![sec];

    let pages = engine.typeset_document(&doc);
    assert!(!pages.is_empty());

    let page0 = &pages[0];
    let g_frame = page0
        .frames
        .iter()
        .find(|f| f.flow_id == "gemara")
        .expect("gemara frame");
    let r_frame = page0
        .frames
        .iter()
        .find(|f| f.flow_id == "rashi")
        .expect("rashi frame");

    let p2_first_line = g_frame
        .lines
        .iter()
        .find(|l| l.paragraph_id == Some(p2_id))
        .expect("p2 on page 1");
    let c1_last_line = r_frame
        .lines
        .iter()
        .rev()
        .find(|l| l.paragraph_id == Some(c1_id))
        .expect("c1 on page 1");
    let c2_first_line = r_frame
        .lines
        .iter()
        .find(|l| l.paragraph_id == Some(c2_id))
        .expect("c2 on page 1");

    // Intra-page spring alignment must advance c2 down toward p2's top Y
    let tight_c2_baseline = c1_last_line.baseline_y + c2_first_line.height;
    assert!(
        c2_first_line.baseline_y > tight_c2_baseline + 15.0,
        "c2 baseline ({}) should spring down toward p2 baseline ({}) rather than sitting tightly at {}",
        c2_first_line.baseline_y,
        p2_first_line.baseline_y,
        tight_c2_baseline
    );
}

#[test]
fn test_co_pagination_engine_accumulates_multi_paragraph_y_coordinates() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let doc = CoPaginationDocument::new()
        .with_main(vec![
            CoPaginatedChunk {
                text: "משנה ראשונה בעמוד זה ".repeat(8),
                anchors: vec![(AnchorKey::new("k1"), "משנה".to_string())],
            },
            CoPaginatedChunk {
                text: "משנה שניה בעמוד זה ".repeat(8),
                anchors: vec![(AnchorKey::new("k2"), "משנה".to_string())],
            },
        ])
        .with_rashi(vec![
            CoPaginatedCommentary {
                target_anchor: AnchorKey::new("k1"),
                text: "רש״י ראשון על המשנה הראשונה ".repeat(5),
            },
            CoPaginatedCommentary {
                target_anchor: AnchorKey::new("k2"),
                text: "רש״י שני על המשנה השניה ".repeat(5),
            },
        ])
        .with_tosafot(vec![
            CoPaginatedCommentary {
                target_anchor: AnchorKey::new("k1"),
                text: "תוספות ראשון על המשנה הראשונה ".repeat(5),
            },
            CoPaginatedCommentary {
                target_anchor: AnchorKey::new("k2"),
                text: "תוספות שני על המשנה השניה ".repeat(5),
            },
        ]);

    let result = CoPaginationEngine::co_paginate(
        &TemplateConfig::default(),
        &SynchronizerConfig::default(),
        &doc,
        &engine,
    );
    assert!(!result.pages.is_empty());

    let p0 = &result.pages[0].page_layout;
    for frame in &p0.frames {
        for pair in frame.lines.windows(2) {
            assert!(
                pair[1].baseline_y > pair[0].baseline_y,
                "Frame '{}' baselines must strictly increase across paragraphs: {} vs {}",
                frame.flow_id,
                pair[0].baseline_y,
                pair[1].baseline_y
            );
        }
    }
}

// ============================================================================
// 2. L-SHAPE & VARIABLE COLUMNS
// ============================================================================

#[test]
fn test_polygon_scanline_conservative_intersection_on_trapezoid_and_steps() {
    // Top edge [0..300] at y=0, bottom edge [0..200] at y=100
    let trap = GeometricPolygon::trapezoid(0.0, 300.0, 0.0, 200.0, 0.0, 100.0);

    let spans = trap.scanline_spans(0.0, 20.0);
    assert_eq!(spans.len(), 1);
    let usable_width = spans[0].width;
    assert!(
        (usable_width - 280.0).abs() < 1.0,
        "Expected conservative trapezoid width ~280pt, got {}",
        usable_width
    );

    let stepped = PolygonFrame::stepped(vec![
        PhysicalRect::new(100.0, 0.0, 120.0, 40.0),
        PhysicalRect::new(50.0, 40.0, 220.0, 40.0),
        PhysicalRect::new(0.0, 80.0, 320.0, 60.0),
    ]);

    let span_straddle = stepped.span_at_y(30.0, 16.0).expect("straddle span");
    assert!(
        (span_straddle.width - 120.0).abs() < 1.0,
        "Line straddling step boundary must use narrowest intersecting span (120pt), got {}",
        span_straddle.width
    );

    let span_mid = stepped.span_at_y(50.0, 16.0).expect("mid span");
    assert!((span_mid.width - 220.0).abs() < 1.0);

    let span_low = stepped.span_at_y(90.0, 16.0).expect("low span");
    assert!((span_low.width - 320.0).abs() < 1.0);

    let synth = MultiColumnPolygonFlowSynthesizer::synthesize_multi_step_frame(
        PhysicalRect::new(220.0, 0.0, 100.0, 140.0),
        &[
            PhysicalRect::new(110.0, 0.0, 100.0, 40.0),
            PhysicalRect::new(0.0, 0.0, 100.0, 80.0),
        ],
        0.0,
        140.0,
    );
    assert!((synth.span_at_y(10.0, 15.0).unwrap().width - 100.0).abs() < 1.0);
    assert!((synth.span_at_y(50.0, 15.0).unwrap().width - 210.0).abs() < 1.0);
    assert!((synth.span_at_y(95.0, 15.0).unwrap().width - 320.0).abs() < 1.0);
}

#[test]
fn test_narrow_polygon_has_no_hardcoded_200_or_100_width_fallback() {
    let narrow_poly = GeometricPolygon::from_rect(&PhysicalRect::new(0.0, 0.0, 72.0, 120.0));
    let profile = MeasureProfile::from_polygon(&narrow_poly, 0.0, 14.0, 6);
    for i in 0..6 {
        assert!(
            (profile.width_for_line(i) - 72.0).abs() < 0.5,
            "Line {} width must be 72pt without 100/200pt fallback, got {}",
            i,
            profile.width_for_line(i)
        );
    }

    let narrow_frame = PolygonFrame::l_shaped(0.0, 0.0, 65.0, 60.0, 0.0, 95.0, 60.0);
    let frame_profile = narrow_frame.profile_for_paragraph(15.0, 0, 6);
    assert!((frame_profile.width_for_line(0) - 65.0).abs() < 0.5);
    assert!((frame_profile.width_for_line(5) - 95.0).abs() < 0.5);
}

#[test]
fn test_l_shape_expansion_frame_rebreaks_paragraphs_to_wider_measure() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let mut doc = DocumentRoot::new("L-Shape Expansion Rebreak Test");
    let mut sec = SectionNode::new("Main", "default").with_flows(Vec::new());

    let mut gemara =
        Flow::new(FlowId::new("gemara"), FlowType::Main).with_placement_role("primary");
    let gp = make_para(1, "gemara", "משנה קצרה מאוד בגמרא שמסתיימת בראש העמוד.");
    gemara.paragraphs.push(gp);

    let mut rashi =
        Flow::new(FlowId::new("rashi"), FlowType::CommentA).with_placement_role("inner_margin");
    let rp = make_para(
        1,
        "rashi",
        &"פירוש רש״י ארוך ומפורט שמתחיל בעמודה הצרה ומתרחב לתוך צורת האות למ״ד מתחת לגמרא ברוחב מלא "
            .repeat(35),
    );
    rashi.paragraphs.push(rp);

    let mut tosafot =
        Flow::new(FlowId::new("tosafot"), FlowType::CommentB).with_placement_role("outer_margin");
    tosafot.paragraphs.push(make_para(
        1,
        "tosafot",
        "תוספות קצר בעמודה החיצונית.",
    ));

    sec.flows.push(gemara);
    sec.flows.push(rashi);
    sec.flows.push(tosafot);
    doc.sections = vec![sec];

    let pages = engine.typeset_document(&doc);
    let page0 = &pages[0];

    let rashi_upper = page0
        .frames
        .iter()
        .find(|f| f.flow_id == "rashi")
        .expect("upper rashi frame");
    let rashi_exp = page0
        .frames
        .iter()
        .find(|f| f.flow_id == "rashi_expansion")
        .expect("rashi_expansion frame");

    assert!(rashi_exp.rect.width > rashi_upper.rect.width + 40.0);
    assert!(!rashi_exp.lines.is_empty());

    let max_upper_glyphs = rashi_upper
        .lines
        .iter()
        .map(|l| l.glyphs.len())
        .max()
        .unwrap_or(0);
    let max_exp_glyphs = rashi_exp
        .lines
        .iter()
        .map(|l| l.glyphs.len())
        .max()
        .unwrap_or(0);

    assert!(
        max_exp_glyphs > max_upper_glyphs + 6,
        "Re-broken expansion lines should pack more glyphs ({}) than narrow upper column lines ({})",
        max_exp_glyphs,
        max_upper_glyphs
    );
}

// ============================================================================
// 3. HEBREW DROP CAPS (פתיח)
// ============================================================================

#[test]
fn test_hebrew_drop_cap_ink_bounds_cantillation_and_pagination() {
    let fm = FontManager::default();
    let cfg = DropCapConfig {
        enabled: true,
        lines: 3,
        characters_count: 1,
        whole_word: true,
        font_family: None,
        font_weight: 700,
        margin_pt: 8.0,
        font_size_pt: None,
    };

    let text = "בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ וְהָאָ֗רֶץ הָיְתָ֥ה תֹ֙הוּ֙ וָבֹ֔הוּ וְחֹ֖שֶׁךְ עַל־פְּנֵ֣י תְהֹ֑ום וְר֣וּחַ אֱלֹהִ֔ים מְרַחֶ֖פֶת עַל־פְּנֵ֥י הַמָּֽיִם.";
    let layout = HebrewDropCapShaper::shape_drop_cap(text, &cfg, &fm, 300.0, 18.0, true)
        .expect("drop cap layout");

    // 1. Cantillation mark triggers fallback to Noto Serif Hebrew
    assert_eq!(layout.font_family, "Noto Serif Hebrew");

    // 2. True outline ink_bounds is strictly contained inside window_rect
    assert!(layout.ink_bounds.width > 0.0 && layout.ink_bounds.height > 0.0);
    assert!(layout.ink_bounds.x >= layout.window_rect.x - 0.5);
    assert!(layout.ink_bounds.y >= layout.window_rect.y - 0.5);
    assert!(
        layout.ink_bounds.x + layout.ink_bounds.width
            <= layout.window_rect.x + layout.window_rect.width + 0.5
    );
    assert!(
        layout.ink_bounds.y + layout.ink_bounds.height
            <= layout.window_rect.y + layout.window_rect.height + 0.5
    );

    // 3. End-to-end document pagination with drop cap
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let mut doc = DocumentRoot::new("Drop Cap Pagination Test");
    let mut style = ParagraphStyle::default();
    style.id = "drop_cap_body".to_string();
    style.name = "Drop Cap Body".to_string();
    style.font_size_pt = 14.0;
    style.line_height_pt = 18.0;
    style.drop_cap = Some(cfg);
    doc.paragraph_styles.push(style);

    let p = make_para(1, "drop_cap_body", &format!("{} {}", text, text));
    doc.sections[0].flows[0].paragraphs.push(p);

    let pages = engine.typeset_document(&doc);
    let frame = &pages[0].frames[0];
    assert!(frame.lines.len() >= 4);

    let dc_line = &frame.lines[0];
    assert!(dc_line.is_drop_cap);
    let body_line_1 = &frame.lines[1];
    let body_line_2 = &frame.lines[2];
    let body_line_3 = &frame.lines[3];

    assert!(
        (body_line_1.baseline_y - 18.0).abs() < 0.5,
        "First body line must sit beside the top of the drop cap window (baseline 18.0), got {}",
        body_line_1.baseline_y
    );
    assert!((body_line_2.baseline_y - 36.0).abs() < 0.5);
    assert!((body_line_3.baseline_y - 54.0).abs() < 0.5);

    let min_body_cluster = body_line_1
        .glyphs
        .iter()
        .map(|g| g.cluster as usize)
        .min()
        .unwrap_or(0);
    assert!(
        min_body_cluster >= layout.remaining_byte_offset,
        "Body line cluster {} must be offset by remaining_byte_offset ({})",
        min_body_cluster,
        layout.remaining_byte_offset
    );
}

// ============================================================================
// 4. GLOBAL VERTICAL ALIGNMENT
// ============================================================================

#[test]
fn test_global_vertical_alignment_across_stepped_and_parallel_columns() {
    let p1 = NodeId::new();
    let p2 = NodeId::new();
    let make_lines = |count: usize, line_h: f32, split: usize| -> Vec<LineBox> {
        (0..count)
            .map(|i| LineBox {
                line_index: i,
                paragraph_id: Some(if i < split { p1 } else { p2 }),
                baseline_y: (i + 1) as f32 * line_h,
                height: line_h,
                width: 150.0,
                glyphs: Vec::new(),
                text: String::new(),
                is_rtl: true,
                fonts: Vec::new(),
                is_drop_cap: false,
                x_offset: 0.0,
            })
            .collect()
    };

    let upper_col = TextFrameBox {
        frame_id: "upper".to_string(),
        flow_id: "rashi".to_string(),
        rect: PhysicalRect::new(0.0, 50.0, 150.0, 180.0),
        lines: make_lines(10, 17.5, 5), // bottom at 50 + 175 = 225 <= 230
    };
    let exp_col = TextFrameBox {
        frame_id: "exp".to_string(),
        flow_id: "rashi_expansion".to_string(),
        rect: PhysicalRect::new(0.0, 242.0, 320.0, 258.0),
        lines: make_lines(14, 17.5, 7), // bottom at 242 + 245 = 487
    };
    let full_col = TextFrameBox {
        frame_id: "full".to_string(),
        flow_id: "tosafot".to_string(),
        rect: PhysicalRect::new(330.0, 50.0, 150.0, 450.0),
        lines: make_lines(25, 18.0, 12), // bottom at 50 + 450 = 500
    };

    let mut frames = vec![upper_col, exp_col, full_col];
    let cfg = HeightBalanceConfig {
        max_line_expansion_pt: 2.5,
        max_paragraph_expansion_pt: 12.0,
        min_lines_for_balance: 3,
        target_mode: BalanceTargetMode::TallestColumn,
    };

    SharedColumnHeightBalancer::balance_page_frames(&mut frames, &cfg);

    let upper_page_bottom = frames[0].rect.y + frames[0].lines.last().unwrap().baseline_y;
    assert!(
        upper_page_bottom <= 230.01,
        "Upper column bottom ({}) must not overflow its frame ceiling (230.0)",
        upper_page_bottom
    );

    let exp_page_bottom = frames[1].rect.y + frames[1].lines.last().unwrap().baseline_y;
    let full_page_bottom = frames[2].rect.y + frames[2].lines.last().unwrap().baseline_y;
    assert!(
        (exp_page_bottom - full_page_bottom).abs() < 0.05,
        "Stepped expansion bottom ({}) and full column bottom ({}) must align in page space",
        exp_page_bottom,
        full_page_bottom
    );
}

// ============================================================================
// 5. KNUTH-PLASS LAST-LINE CONSTRAINED BALANCING
// ============================================================================

#[test]
fn test_knuth_plass_last_line_constraints_and_optimal_variable_placement() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let mut doc = DocumentRoot::new("Knuth-Plass Last Line Constraint Test");

    let mut style = ParagraphStyle::default();
    style.id = "kp_constrained".to_string();
    style.name = "KP Constrained".to_string();
    style.font_size_pt = 13.0;
    style.line_height_pt = 17.0;
    style.last_line_constraint = Some(LastLineConstraintConfig {
        min_width_pt: 40.0,
        min_width_ratio: 0.30,
        min_words: 2,
        balance_slack: true,
    });
    doc.paragraph_styles.push(style);

    let p = make_para(
        1,
        "kp_constrained",
        "זהו טקסט בדיקה עבור אלגוריתם קנות׳־פלאס המוודא כי השורה האחרונה בפסקה אינה קצרה מדי ואינה מכילה מילה בודדת יתומה בסוף הפסקה.",
    );
    doc.sections[0].flows[0].paragraphs.push(p);

    let pages = engine.typeset_document(&doc);
    let lines = &pages[0].frames[0].lines;
    assert!(lines.len() >= 2);

    let last_line = lines.last().unwrap();
    let space_count = last_line
        .glyphs
        .iter()
        .filter(|g| g.character == Some(' '))
        .count();
    assert!(
        space_count >= 1,
        "Last line must contain at least 2 words (at least 1 space), got {} spaces",
        space_count
    );
    assert!(
        last_line.width >= 40.0,
        "Last line width ({}) must satisfy min_width_pt (40.0)",
        last_line.width
    );

    // Variable measure profile with optimal final-line placement
    let profile = MeasureProfile::from_widths(vec![200.0, 200.0, 140.0], true)
        .with_last_line_constraint(LastLineConstraint {
            min_width_pt: 50.0,
            min_width_ratio: 0.35,
            min_words: 2,
            balance_slack: true,
            max_width_ratio: 0.96,
        });

    let mut items = Vec::new();
    for (idx, &w) in [65.0_f32, 65.0, 65.0, 65.0, 65.0, 60.0].iter().enumerate() {
        if idx > 0 {
            items.push(LayoutItem::Glue {
                width: 6.0,
                stretch: 3.0,
                shrink: 2.0,
            });
        }
        items.push(LayoutItem::Box {
            width: w,
            text: "מילה".to_string(),
            glyphs: Vec::new(),
        });
    }
    items.push(LayoutItem::Penalty {
        width: 0.0,
        penalty: KnuthPlassBreaker::FORCED_BREAK_PENALTY,
        flagged: false,
    });

    let broken = KnuthPlassBreaker::break_paragraph_with_profile(&items, &profile, 2.0);
    assert!(!broken.is_empty());
    let final_line = broken.last().unwrap();
    let final_target = profile.width_for_line(final_line.line_number);
    assert!(
        final_line.width >= final_target * 0.35,
        "Final line width ({}) must be >= 35% of its line target width ({})",
        final_line.width,
        final_target
    );
}

// ============================================================================
// 6. OPTICAL MARGIN ALIGNMENT (RTL HEBREW + LTR + NIQQUD TRANSPARENCY)
// ============================================================================

#[test]
fn test_optical_margin_alignment_hebrew_rtl_niqqud_and_ltr_pipeline() {
    let cfg = OpticalMarginConfig {
        enabled: true,
        factor: 1.0,
        protrude_punctuation: true,
        protrude_glyphs: true,
    };

    // 1. Combining-mark transparency: vocalized maqaf at left edge and vocalized resh at right edge
    let glyphs_vocalized = vec![
        PositionedGlyph {
            glyph_id: 1,
            cluster: 10,
            x_advance: 6.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('\u{05BE}'), // maqaf
        },
        PositionedGlyph {
            glyph_id: 2,
            cluster: 10,
            x_advance: 0.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('\u{05B0}'), // sheva
        },
        PositionedGlyph {
            glyph_id: 3,
            cluster: 5,
            x_advance: 180.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('מ'),
        },
        PositionedGlyph {
            glyph_id: 4,
            cluster: 0,
            x_advance: 14.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('ר'),
        },
        PositionedGlyph {
            glyph_id: 5,
            cluster: 0,
            x_advance: 0.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('\u{05B8}'), // kamatz
        },
    ];

    let prot = OpticalMarginAligner::compute_line_protrusion(&glyphs_vocalized, 14.0, &cfg);
    assert!(
        prot.left_pt > 2.0,
        "Left maqaf should protrude > 2pt despite trailing combining mark, got {}",
        prot.left_pt
    );
    assert!(
        prot.right_pt > 0.5,
        "Right resh ('ר') should protrude > 0.5pt despite trailing kamatz, got {}",
        prot.right_pt
    );

    let (eff_target, eff_x_offset, right_hang) = OpticalMarginAligner::adjust_line_geometry(
        200.0,
        0.0,
        true,
        false,
        TextAlignment::Justified,
        prot,
    );
    assert!(eff_target > 202.5);
    assert!(eff_x_offset < -2.0);
    assert!(right_hang > 0.5);

    // 2. End-to-end pipeline test with OpticalMarginConfig enabled vs disabled
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let text = "״אמר רבי עקיבא ואהבת לרעך כמוך זה כלל גדול בתורה ובנביאים ובכתובים תמיד״ "
        .repeat(4);

    let mut doc_off = DocumentRoot::new("OM Off");
    let mut style_off = ParagraphStyle::default();
    style_off.id = "om_off".to_string();
    style_off.name = "OM Off".to_string();
    style_off.alignment = TextAlignment::Justified;
    doc_off.paragraph_styles.push(style_off);
    doc_off.sections[0].flows[0]
        .paragraphs
        .push(make_para(1, "om_off", &text));

    let mut doc_on = DocumentRoot::new("OM On");
    let mut style_on = ParagraphStyle::default();
    style_on.id = "om_on".to_string();
    style_on.name = "OM On".to_string();
    style_on.alignment = TextAlignment::Justified;
    style_on.optical_margin = Some(cfg);
    doc_on.paragraph_styles.push(style_on);
    doc_on.sections[0].flows[0]
        .paragraphs
        .push(make_para(1, "om_on", &text));

    let pages_off = engine.typeset_document(&doc_off);
    let pages_on = engine.typeset_document(&doc_on);

    let line0_off = &pages_off[0].frames[0].lines[0];
    let line0_on = &pages_on[0].frames[0].lines[0];

    let max_x_off = line0_off
        .glyphs
        .iter()
        .map(|g| g.x + g.width)
        .fold(0.0_f32, f32::max);
    let max_x_on = line0_on
        .glyphs
        .iter()
        .map(|g| g.x + g.width)
        .fold(0.0_f32, f32::max);

    assert!(
        max_x_on > max_x_off + 1.0,
        "Opening gershayim at right margin in RTL justified paragraph must protrude into right margin ({} vs {})",
        max_x_on,
        max_x_off
    );
}

#[test]
fn test_multi_flow_cross_page_commentary_overflow_holdback() {
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let mut doc = DocumentRoot::new("Commentary Overflow Holdback Test");
    let mut sec = SectionNode::new("Main", "default").with_flows(Vec::new());

    let mut gemara =
        Flow::new(FlowId::new("gemara"), FlowType::Main).with_placement_role("primary");
    // p1 occupies ~45% of Gemara column height
    let p1 = make_para(
        1,
        "gemara",
        &"אמר רב יהודה אמר רב הלכה כדברי חכמים במסכת זו ".repeat(12),
    );
    let p1_id = p1.id;
    // p2 would physically fit in Gemara on Page 1, but its Rashi commentary c2 cannot fit on Page 1
    // because c1 already saturates the Rashi column on Page 1.
    let p2 = make_para(
        2,
        "gemara",
        &"תנן התם כל הפסולים ששחטו שחיטתן כשרה בדיעבד ".repeat(18),
    );
    let p2_id = p2.id;
    gemara.paragraphs.push(p1);
    gemara.paragraphs.push(p2);

    let mut rashi =
        Flow::new(FlowId::new("rashi"), FlowType::CommentA).with_placement_role("inner_margin");
    // Very long commentary c1 anchored to p1 that fills the entire Rashi column on Page 1
    let c1 = make_para(
        1,
        "rashi",
        &"פירוש רש״י ארוך מאוד על הפסקה הראשונה שממלא את כל עמודת רש״י בעמוד הראשון עד תומה "
            .repeat(48),
    );
    let c1_id = c1.id;
    let c2 = make_para(
        2,
        "rashi",
        &"פירוש רש״י על הפסקה השניה שצריך לעבור יחד איתה לעמוד הבא ".repeat(8),
    );
    let c2_id = c2.id;
    rashi.paragraphs.push(c1);
    rashi.paragraphs.push(c2);

    let mut tosafot =
        Flow::new(FlowId::new("tosafot"), FlowType::CommentB).with_placement_role("outer_margin");
    tosafot.paragraphs.push(make_para(
        1,
        "tosafot",
        &"תוספות בעמודה החיצונית לאורך העמוד ".repeat(25),
    ));

    let mut anchors = MultiFlowAnchorMap::default();
    link_primary_to_commentary(&mut anchors, "a1", "gemara", p1_id, "rashi", c1_id);
    link_primary_to_commentary(&mut anchors, "a2", "gemara", p2_id, "rashi", c2_id);
    doc.multi_flow_anchors = Some(anchors);

    sec.flows.push(gemara);
    sec.flows.push(rashi);
    sec.flows.push(tosafot);
    doc.sections = vec![sec];

    let pages = engine.typeset_document(&doc);
    assert!(pages.len() >= 2, "Expected at least 2 pages, got {}", pages.len());

    // Page 1 must contain p1, and p2 must be held back to Page 2 where c2 can appear alongside it
    let page1_g = pages[0]
        .frames
        .iter()
        .find(|f| f.flow_id == "gemara")
        .expect("page 1 gemara");
    assert!(page1_g.lines.iter().any(|l| l.paragraph_id == Some(p1_id)));
    assert!(
        page1_g.lines.iter().all(|l| l.paragraph_id != Some(p2_id)),
        "p2 should be held back from Page 1 because Rashi column is saturated by c1"
    );

    let p2_page_idx = pages
        .iter()
        .position(|p| {
            p.frames
                .iter()
                .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(p2_id)))
        })
        .expect("p2 placed on subsequent page");
    assert!(p2_page_idx >= 1);
}

#[test]
fn test_property_based_invariants_across_all_six_capabilities() {
    let fm = FontManager::default();
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());

    // Property 1: DropCap ink_bounds is strictly inside window_rect across varied line spans (2..=5) and margins
    let sample_hebrew = "בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ וְהָאָ֗רֶץ הָיְתָ֥ה תֹ֙הוּ֙ וָבֹ֔הוּ.";
    for lines_spanned in 2..=5 {
        for margin_pt in [4.0_f32, 8.0, 12.0] {
            let cfg = DropCapConfig {
                enabled: true,
                lines: lines_spanned,
                characters_count: 1,
                whole_word: true,
                font_family: None,
                font_weight: 700,
                margin_pt,
                font_size_pt: None,
            };
            let dc = HebrewDropCapShaper::shape_drop_cap(sample_hebrew, &cfg, &fm, 320.0, 17.0, true)
                .expect("drop cap");
            assert!(dc.ink_bounds.x >= dc.window_rect.x - 1e-3);
            assert!(dc.ink_bounds.y >= dc.window_rect.y - 1e-3);
            assert!(
                dc.ink_bounds.x + dc.ink_bounds.width
                    <= dc.window_rect.x + dc.window_rect.width + 1e-3
            );
            assert!(
                dc.ink_bounds.y + dc.ink_bounds.height
                    <= dc.window_rect.y + dc.window_rect.height + 1e-3
            );
        }
    }

    // Property 2: Optical protrusion scales monotonically with `factor`
    let glyphs = vec![
        PositionedGlyph {
            glyph_id: 1,
            cluster: 0,
            x_advance: 6.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('\u{05BE}'),
        },
        PositionedGlyph {
            glyph_id: 2,
            cluster: 2,
            x_advance: 10.0,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            character: Some('\u{05F4}'),
        },
    ];
    let mut prev_left = 0.0_f32;
    let mut prev_right = 0.0_f32;
    for factor in [0.25_f32, 0.5, 0.75, 1.0, 1.25] {
        let cfg = OpticalMarginConfig {
            enabled: true,
            factor,
            protrude_punctuation: true,
            protrude_glyphs: true,
        };
        let p = OpticalMarginAligner::compute_line_protrusion(&glyphs, 14.0, &cfg);
        assert!(p.left_pt > prev_left);
        assert!(p.right_pt > prev_right);
        prev_left = p.left_pt;
        prev_right = p.right_pt;
    }

    // Property 3: Layout determinism across repeated invocations
    let mut doc = DocumentRoot::new("Determinism Property Test");
    let mut style = ParagraphStyle::default();
    style.id = "det".to_string();
    style.optical_margin = Some(OpticalMarginConfig::default());
    style.last_line_constraint = Some(LastLineConstraintConfig {
        min_width_pt: 35.0,
        min_width_ratio: 0.25,
        min_words: 2,
        balance_slack: true,
    });
    doc.paragraph_styles.push(style);
    doc.sections[0].flows[0].paragraphs.push(make_para(
        1,
        "det",
        &"בדיקת דטרמיניזם מלאה של מנוע העימוד עבור פסקאות מורכבות בעברית ".repeat(6),
    ));

    let run1 = engine.typeset_document(&doc);
    let run2 = engine.typeset_document(&doc);
    assert_eq!(run1, run2, "Typesetting output must be 100% deterministic");
}

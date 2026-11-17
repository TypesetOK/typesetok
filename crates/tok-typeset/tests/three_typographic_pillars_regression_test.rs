//! End-to-end integration regression suite for the Three Typographic Pillars:
//!
//! 1. Dynamic Multi-Flow Anchoring ("צורת הדף" חיה / Dynamic Multi-Flow Anchoring):
//!    - Bidirectional logical anchoring across parallel streams (Torah, Targum, Rashi, Ramban / Gemara, Rashi, Tosafot).
//!    - Automatic commentary cascading when primary text is pushed across page boundaries.
//!    - Shared column height balancing across parallel flows at page bottoms.
//!
//! 2. Absorption Frames & Variable-Width Polygon Engine (מסגרות בליעה וטורים ברוחב משתנה):
//!    - Mid-page commentary completion triggers adjacent commentary absorption into L-shaped polygons.
//!    - Knuth-Plass line breaking inside variable width line profiles and non-rectangular geometries.
//!
//! 3. Hebrew Drop Cap Opening Word Window ("חלון" במילת פתיחה):
//!    - Exact HarfBuzz/Rustybuzz bounding box measurement accounting for Niqqud and Ta'amim.
//!    - Multi-line exclusion zone window indenting lines 2, 3+ with configured standoff.

use std::collections::HashMap;
use tok_core::anchor::TextAnchor;
use tok_core::id::{FractionalIndex, NodeId};
use tok_core::model::{FlowId, ParagraphNode};
use tok_core::multi_flow_anchor::{AnchorDefinition, AnchorId, CommentaryLink, MultiFlowAnchorMap};
use tok_core::styles::DropCapConfig;
use tok_typeset::co_pagination::{
    AnchorCascadeEngine, BalanceTargetMode, CascadeNodeInput, CoPaginatedChunk,
    CoPaginatedCommentary, CoPaginationDocument, CoPaginationEngine, HeightBalanceConfig,
    SharedColumnHeightBalancer, SpreadMathematics, TemplateConfig, TemplateKind,
};
use tok_typeset::drop_cap::HebrewDropCapShaper;
use tok_typeset::geometry::{LineBox, PhysicalPoint, PhysicalRect, TextFrameBox};
use tok_typeset::hebrew_justify::HebrewJustifier;
use tok_typeset::knuth_plass::MeasureProfile;
use tok_typeset::polygon::GeometricPolygon;
use tok_typeset::polygon_engine::PolygonFrame;
use tok_typeset::{TypesettingEngine, TypesettingEngineConfig};

fn create_engine() -> TypesettingEngine {
    TypesettingEngine::new(TypesettingEngineConfig::default())
}

// ============================================================================
// PILLAR 1: Dynamic Multi-Flow Anchoring, Commentary Cascading & Shared Height Balancing
// ============================================================================

#[test]
fn test_pillar1_dynamic_multi_flow_anchoring_cascades_commentaries_and_balances_height() {
    let primary_flow = FlowId::new("torah");
    let targum_flow = FlowId::new("targum");
    let rashi_flow = FlowId::new("rashi");
    let ramban_flow = FlowId::new("ramban");

    let p1_id = NodeId::new();
    let p2_id = NodeId::new();

    let c1_targum_id = NodeId::new();
    let c2_rashi_id = NodeId::new();
    let c3_ramban_id = NodeId::new();
    let c4_rashi_p2_id = NodeId::new();
    let c5_ramban_p2_id = NodeId::new();

    let a1 = AnchorId::new("dh_bereshit");
    let a2 = AnchorId::new("dh_yehi_or");

    let mut anchor_map = MultiFlowAnchorMap::new();
    anchor_map.register_anchor(AnchorDefinition::new(
        a1.clone(),
        primary_flow.clone(),
        p1_id,
        TextAnchor::start_of(p1_id),
    ));
    anchor_map.register_anchor(AnchorDefinition::new(
        a2.clone(),
        primary_flow.clone(),
        p2_id,
        TextAnchor::start_of(p2_id),
    ));

    // P1 anchors: Targum C1, Rashi C2, Ramban C3
    anchor_map.link_commentary(CommentaryLink::new(
        targum_flow.clone(),
        c1_targum_id,
        a1.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(
        rashi_flow.clone(),
        c2_rashi_id,
        a1.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(ramban_flow.clone(), c3_ramban_id, a1));

    // P2 anchors: Rashi C4, Ramban C5
    anchor_map.link_commentary(CommentaryLink::new(
        rashi_flow.clone(),
        c4_rashi_p2_id,
        a2.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(
        ramban_flow.clone(),
        c5_ramban_p2_id,
        a2,
    ));

    let max_column_height = 400.0;

    // Initially, P1 (180pt) and P2 (180pt) fit on Page 1 (360pt <= 400pt)
    let mut nodes = HashMap::new();
    nodes.insert(
        primary_flow.clone(),
        vec![
            CascadeNodeInput::new(p1_id, primary_flow.clone(), 180.0, "בראשית ברא אלהים"),
            CascadeNodeInput::new(p2_id, primary_flow.clone(), 180.0, "ויאמר אלהים יהי אור"),
        ],
    );
    nodes.insert(
        targum_flow.clone(),
        vec![CascadeNodeInput::new(
            c1_targum_id,
            targum_flow.clone(),
            80.0,
            "תרגום בראשית",
        )],
    );
    nodes.insert(
        rashi_flow.clone(),
        vec![
            CascadeNodeInput::new(c2_rashi_id, rashi_flow.clone(), 100.0, "רש\"י בראשית"),
            CascadeNodeInput::new(c4_rashi_p2_id, rashi_flow.clone(), 100.0, "רש\"י יהי אור"),
        ],
    );
    nodes.insert(
        ramban_flow.clone(),
        vec![
            CascadeNodeInput::new(c3_ramban_id, ramban_flow.clone(), 120.0, "רמב\"ן בראשית"),
            CascadeNodeInput::new(
                c5_ramban_p2_id,
                ramban_flow.clone(),
                110.0,
                "רמב\"ן יהי אור",
            ),
        ],
    );

    let baseline = AnchorCascadeEngine::paginate_with_cascade(
        &primary_flow,
        &nodes,
        max_column_height,
        &anchor_map,
    );

    // All fit on Page 1
    assert_eq!(baseline.node_to_page.get(&p1_id), Some(&1));
    assert_eq!(baseline.node_to_page.get(&p2_id), Some(&1));
    assert_eq!(baseline.node_to_page.get(&c4_rashi_p2_id), Some(&1));
    assert_eq!(baseline.node_to_page.get(&c5_ramban_p2_id), Some(&1));
    assert_eq!(baseline.total_pages, 1);

    // Dynamic Edit: P1 expands (font enlargement or inserted commentary) to 320pt
    // Now 320 + 180 = 500 > 400 -> P2 MUST push to Page 2!
    let mut updated_nodes = nodes.clone();
    updated_nodes.insert(
        primary_flow.clone(),
        vec![
            CascadeNodeInput::new(p1_id, primary_flow.clone(), 320.0, "בראשית ברא אלהים מורחב"),
            CascadeNodeInput::new(p2_id, primary_flow.clone(), 180.0, "ויאמר אלהים יהי אור"),
        ],
    );

    let cascaded = AnchorCascadeEngine::paginate_with_cascade(
        &primary_flow,
        &updated_nodes,
        max_column_height,
        &anchor_map,
    );

    // P1 stays on Page 1, but P2 moves to Page 2
    assert_eq!(cascaded.node_to_page.get(&p1_id), Some(&1));
    assert_eq!(
        cascaded.node_to_page.get(&p2_id),
        Some(&2),
        "P2 must be pushed to Page 2"
    );

    // Commentaries on P1 (Targum C1, Rashi C2, Ramban C3) remain on Page 1
    assert_eq!(cascaded.node_to_page.get(&c1_targum_id), Some(&1));
    assert_eq!(cascaded.node_to_page.get(&c2_rashi_id), Some(&1));
    assert_eq!(cascaded.node_to_page.get(&c3_ramban_id), Some(&1));

    // CRITICAL: Commentaries on P2 (Rashi C4, Ramban C5) MUST automatically cascade to Page 2!
    assert_eq!(
        cascaded.node_to_page.get(&c4_rashi_p2_id),
        Some(&2),
        "Rashi C4 must cascade to Page 2"
    );
    assert_eq!(
        cascaded.node_to_page.get(&c5_ramban_p2_id),
        Some(&2),
        "Ramban C5 must cascade to Page 2"
    );

    assert_eq!(cascaded.total_pages, 2);
    assert!(AnchorCascadeEngine::verify_invariants(&cascaded, &anchor_map).is_ok());

    // Now verify Shared Column Height Balancing across the 4 columns on Page 1
    let make_line = |idx: usize, y: f32| LineBox {
        line_index: idx,
        paragraph_id: Some(NodeId::new()),
        baseline_y: y,
        height: 14.0,
        width: 140.0,
        x_offset: 0.0,
        is_drop_cap: false,
        glyphs: Vec::new(),
        text: format!("Line {}", idx),
        is_rtl: true,
        fonts: vec!["Frank Ruhl Libre".to_string()],
        ..Default::default()
    };

    let mut page1_frames = vec![
        TextFrameBox {
            frame_id: "f_torah".to_string(),
            flow_id: "torah".to_string(),
            rect: PhysicalRect::new(10.0, 10.0, 150.0, 400.0),
            lines: (0..10)
                .map(|i| make_line(i, 40.0 + i as f32 * 36.0))
                .collect(), // ends at 364.0
        },
        TextFrameBox {
            frame_id: "f_targum".to_string(),
            flow_id: "targum".to_string(),
            rect: PhysicalRect::new(170.0, 10.0, 150.0, 400.0),
            lines: (0..8)
                .map(|i| make_line(i, 45.0 + i as f32 * 40.0))
                .collect(), // ends at 325.0 (39pt deficit)
        },
        TextFrameBox {
            frame_id: "f_rashi".to_string(),
            flow_id: "rashi".to_string(),
            rect: PhysicalRect::new(330.0, 10.0, 150.0, 400.0),
            lines: (0..9)
                .map(|i| make_line(i, 42.0 + i as f32 * 38.0))
                .collect(), // ends at 346.0 (18pt deficit)
        },
        TextFrameBox {
            frame_id: "f_ramban".to_string(),
            flow_id: "ramban".to_string(),
            rect: PhysicalRect::new(490.0, 10.0, 150.0, 400.0),
            lines: (0..9)
                .map(|i| make_line(i, 40.0 + i as f32 * 39.0))
                .collect(), // ends at 352.0 (12pt deficit)
        },
    ];

    let bal_config = HeightBalanceConfig {
        max_line_expansion_pt: 6.0,
        max_paragraph_expansion_pt: 20.0,
        min_lines_for_balance: 3,
        target_mode: BalanceTargetMode::TallestColumn,
    };

    let reports = SharedColumnHeightBalancer::balance_page_frames(&mut page1_frames, &bal_config);
    assert_eq!(reports.len(), 4);

    let target_envelope = 364.0;
    for frame in &page1_frames {
        let bottom = frame.lines.last().unwrap().baseline_y;
        assert!(
            (bottom - target_envelope).abs() < 0.25,
            "Frame {} bottom ({}) failed to align to target envelope ({})",
            frame.flow_id,
            bottom,
            target_envelope
        );
    }
}

// ============================================================================
// PILLAR 2: Dynamic Commentary Absorption (L-Shaped Frames) & Variable Width Polygon Knuth-Plass
// ============================================================================

#[test]
fn test_pillar2_commentary_absorption_and_variable_width_polygon_flow() {
    let engine = create_engine();

    // Talmud spread where Gemara and Rashi finish mid-page, Tosafot absorbs the vacant width
    let config = TemplateConfig {
        page_width_pt: 600.0,
        page_height_pt: 800.0,
        ..Default::default()
    };
    let math = SpreadMathematics::for_page(&config, 1);

    let rashi_height = 200.0;
    let rashi_narrow_lines = (rashi_height / config.commentary_line_height).ceil() as usize;

    // Build L-shaped profile for commentary expansion
    let profile = math.create_measure_profile(&config, "rashi", Some(rashi_height));
    assert!(
        !profile.is_uniform(),
        "Rashi profile under early-terminating Gemara must be L-shaped"
    );

    // Verify Knuth-Plass line breaking inside L-shaped polygon
    let long_commentary = (0..20)
        .map(|i| {
            format!(
                "דיבור המתחיל אות {} ביאור רחב ומעמיק בדברי הגמרא והמשנה עם הרחבה מלאה",
                i
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let p_node = ParagraphNode::new(FractionalIndex::new("r_abs_1"), "rashi", &long_commentary);

    let shaped_lines = engine.typeset_paragraph_shaped(
        &p_node,
        &config.commentary_font_family,
        &profile,
        config.commentary_font_size,
        config.commentary_line_height,
    );

    assert!(shaped_lines.len() > rashi_narrow_lines);

    // Verify lines before step have narrow measure, lines after step expand to wider measure
    let narrow_w = math.inner_column_width_pt;
    for (idx, line) in shaped_lines.iter().enumerate() {
        if idx < rashi_narrow_lines - 1 {
            assert!(
                line.width <= narrow_w + 1.0,
                "Narrow line {} width ({}) exceeded narrow measure ({})",
                idx,
                line.width,
                narrow_w
            );
        } else if idx > rashi_narrow_lines && idx < shaped_lines.len() - 1 {
            assert!(
                line.width > narrow_w + 10.0,
                "Expanded line {} width ({}) should exceed narrow measure ({})",
                idx,
                line.width,
                narrow_w
            );
        }
    }

    // Verify non-rectangular polygon slicing with PolygonFrame
    let poly = GeometricPolygon::new(vec![
        PhysicalPoint { x: 0.0, y: 0.0 },
        PhysicalPoint { x: 150.0, y: 0.0 },
        PhysicalPoint { x: 150.0, y: 100.0 },
        PhysicalPoint { x: 350.0, y: 100.0 },
        PhysicalPoint { x: 350.0, y: 300.0 },
        PhysicalPoint { x: 0.0, y: 300.0 },
    ]);

    let frame = PolygonFrame::from_polygon(poly);
    let poly_profile = frame.profile_for_paragraph(15.0, 0, 20);

    assert_eq!(poly_profile.widths.len(), 20);
    // Upper lines (y < 100) have width ~150
    assert!((poly_profile.widths[0] - 150.0).abs() < 1.0);
    // Lower lines (y >= 100) have width ~350
    assert!((poly_profile.widths[10] - 350.0).abs() < 1.0);

    // Test Hebrew justification start X calculation for variable-width lines
    // In RTL, a line of 140pt inside a 150pt target span aligns flush-right:
    // start X = x_offset + (target_width - line_width) = 0.0 + (150.0 - 140.0) = 10.0
    let rtl_x_narrow = HebrewJustifier::calculate_line_start_x(140.0, 150.0, 0.0, true);
    assert_eq!(
        rtl_x_narrow, 10.0,
        "RTL flush right alignment at offset 0 + remaining slack"
    );

    // In LTR, lines align flush-left at x_offset:
    let ltr_x = HebrewJustifier::calculate_line_start_x(140.0, 150.0, 0.0, false);
    assert_eq!(ltr_x, 0.0, "LTR flush left alignment at offset 0");
}

// ============================================================================
// PILLAR 3: Hebrew Opening Word Drop Cap Window with Complex Niqqud & Ta'amim
// ============================================================================

#[test]
fn test_pillar3_hebrew_drop_cap_window_with_complex_niqqud_and_taamim() {
    let engine = create_engine();

    // Opening word with full Niqqud, Dagesh, and Ta'amim: "בְּרֵאשִׁ֖ית"
    let opening_word = "בְּרֵאשִׁ֖ית";
    let body_text = "בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ: וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ וְחֹשֶׁךְ עַל פְּנֵי תְהוֹם וְרוּחַ אֱלֹהִים מְרַחֶפֶת עַל פְּנֵי הַמָּיִם: וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר וַיְהִי אוֹר: וַיַּרְא אֱלֹהִים אֶת הָאוֹר כִּי טוֹב וַיַּבְדֵּל אֱלֹהִים בֵּין הָאוֹר וּבֵין הַחֹשֶׁךְ: וַיִּקְרָא אֱלֹהִים לָאוֹר יוֹם וְלַחֹשֶׁךְ קָרָא לָיְלָה:";
    let full_paragraph_text = format!("{} {}", opening_word, body_text);

    let font_size = 13.0;
    let line_height = 18.0;
    let col_width = 300.0;

    // Shape drop cap opening word with 3 lines depth
    let drop_cap_cfg = DropCapConfig {
        enabled: true,
        lines: 3,
        whole_word: true,
        margin_pt: 8.0,
        font_family: Some("Frank Ruhl Libre".to_string()),
        ..Default::default()
    };

    let drop_layout = HebrewDropCapShaper::shape_drop_cap(
        &full_paragraph_text,
        &drop_cap_cfg,
        &engine.font_manager,
        col_width,
        line_height,
        true,
    )
    .expect("Failed to shape Hebrew drop cap opening word");

    assert!(
        drop_layout.window_rect.width > 20.0,
        "Drop cap width must be positive"
    );
    assert!(
        drop_layout.window_rect.height >= 3.0 * line_height - 0.1,
        "Drop cap height must cover 3 lines"
    );
    assert!(
        !drop_layout.glyphs.is_empty(),
        "Drop cap must contain shaped glyphs"
    );

    let notch_width = drop_layout.window_rect.width;

    // Verify paragraph typesetting with drop cap notch window
    let p_node = ParagraphNode::new(
        FractionalIndex::new("p_drop_cap"),
        "main",
        &full_paragraph_text,
    )
    .with_drop_cap(drop_cap_cfg);

    let profile = MeasureProfile::uniform(col_width);

    let lines = engine.typeset_paragraph_shaped(
        &p_node,
        "Frank Ruhl Libre",
        &profile,
        font_size,
        line_height,
    );

    assert!(
        lines.len() >= 5,
        "Expected at least 5 lines in shaped paragraph"
    );

    // Line 0 is the drop cap line itself
    assert!(lines[0].is_drop_cap, "Line 0 must be flagged as drop cap");

    // Lines 1, 2, 3 (the subsequent 3 lines within the 3-line drop cap window)
    // MUST notch within the remaining measure next to the drop cap window
    for line in &lines[1..=3] {
        assert!(!line.is_drop_cap);
        assert!(
            line.width <= col_width - notch_width + 1.0,
            "Line width ({}) exceeded window measure ({})",
            line.width,
            col_width - notch_width
        );
        for g in &line.glyphs {
            assert!(
                g.x <= drop_layout.window_rect.x + 1.0,
                "Glyph at {} overlaps drop cap window at {}",
                g.x,
                drop_layout.window_rect.x
            );
        }
    }

    // Line 4 (after drop cap window) returns to full column width
    assert!(!lines[4].is_drop_cap);
    assert!(
        lines[4].width > col_width - notch_width + 5.0,
        "Line 4 after window must expand to full column measure (got {}, expected > {})",
        lines[4].width,
        col_width - notch_width + 5.0
    );
}

// ============================================================================
// GRAND UNIFIED REGRESSION TEST: All 3 Typographic Pillars Simultaneously Active
// ============================================================================

#[test]
fn test_grand_unified_all_three_typographic_pillars_integrated() {
    let engine = create_engine();

    // 1. Setup multi-flow template with Talmud layout
    let tmpl_config = TemplateConfig {
        page_width_pt: 600.0,
        page_height_pt: 700.0,
        margin_inner_pt: 30.0,
        margin_outer_pt: 30.0,
        margin_top_pt: 30.0,
        margin_bottom_pt: 30.0,
        gutter_pt: 10.0,
        kind: TemplateKind::TalmudClassic,
        ..Default::default()
    };
    let sync_config = tok_typeset::co_pagination::SynchronizerConfig::default();

    // 2. Main Gemara text starting with a Hebrew Drop Cap opening word
    let opening = "מַאימָתַי";
    let gemara_chunk1 = format!("{} קורין את שמע בערבין משעה שהכהנים נכנסים לאכול בתרומתן עד סוף האשמורה הראשונה דברי רבי אליעזר וחכמים אומרים עד חצות רבן גמליאל אומר עד שיעלה עמוד השחר", opening);
    let gemara_chunk2 = "תנא היכא קאי דקתני מאימתי ותו מאי שנא דתני בערבית ברישא לתני דשחרית ברישא תנא אקרא קאי דכתיב בשכבך ובקומך וכך דרך המקרא להקדים ערב לבוקר".to_string();

    let mut doc = CoPaginationDocument::new().with_main(vec![
        CoPaginatedChunk {
            text: gemara_chunk1,
            anchors: vec![
                (tok_typeset::AnchorKey::new("dh_m1"), opening.to_string()),
                (tok_typeset::AnchorKey::new("dh_m2"), "עד חצות".to_string()),
            ],
        },
        CoPaginatedChunk {
            text: gemara_chunk2,
            anchors: vec![(
                tok_typeset::AnchorKey::new("dh_m3"),
                "תנא היכא קאי".to_string(),
            )],
        },
    ]);

    // 3. Add commentary items: Rashi ends early on page 1, triggering Tosafot absorption
    doc.add_commentary(
        "rashi",
        CoPaginatedCommentary {
            target_anchor: tok_typeset::AnchorKey::new("dh_m1"),
            text: "מאימתי - פירוש קצר על המשנה המבאר את זמן קריאת שמע".to_string(),
        },
    );
    doc.add_commentary(
        "rashi",
        CoPaginatedCommentary {
            target_anchor: tok_typeset::AnchorKey::new("dh_m2"),
            text: "עד חצות - סייג לתורה כדי להרחיק את האדם מן העבירה".to_string(),
        },
    );

    // Tosafot has substantial text that will absorb beneath Rashi / Gemara
    let tosafot_long = (0..12)
        .map(|i| {
            format!(
                "תוספות אות {} תימא דבכל מקום מקדים תפלת שחרית לתפלת ערבית ומשני קרא דבשכבך ובקומך",
                i
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    doc.add_commentary(
        "tosafot",
        CoPaginatedCommentary {
            target_anchor: tok_typeset::AnchorKey::new("dh_m1"),
            text: tosafot_long,
        },
    );
    doc.add_commentary(
        "tosafot",
        CoPaginatedCommentary {
            target_anchor: tok_typeset::AnchorKey::new("dh_m3"),
            text: "תנא היכא קאי - אור\"י דקאי אקרא דבשכבך ובקומך".to_string(),
        },
    );

    // 4. Run coordinated multi-page co-pagination
    let result = CoPaginationEngine::co_paginate(&tmpl_config, &sync_config, &doc, &engine);

    assert!(result.total_pages >= 1, "Must generate at least 1 page");
    assert!(result.total_gemara_lines > 0, "Gemara lines must exist");
    assert!(result.total_rashi_lines > 0, "Rashi lines must exist");
    assert!(result.total_tosafot_lines > 0, "Tosafot lines must exist");

    let page1 = &result.pages[0];
    assert_eq!(page1.page_number, 1);
    assert_eq!(page1.page_layout.frames.len(), 3); // rashi, main, tosafot

    // 5. Apply shared column height balancing across page 1 frames
    let mut frames = page1.page_layout.frames.clone();
    let bal_config = HeightBalanceConfig {
        max_line_expansion_pt: 4.0,
        max_paragraph_expansion_pt: 16.0,
        min_lines_for_balance: 2,
        target_mode: BalanceTargetMode::TallestColumn,
    };

    let balance_reports = SharedColumnHeightBalancer::balance_page_frames(&mut frames, &bal_config);
    assert!(
        !balance_reports.is_empty(),
        "Balance reports must be generated"
    );

    // All active frames should converge to the tallest column baseline
    let tallest_baseline = balance_reports
        .iter()
        .map(|r| r.balanced_bottom_y)
        .fold(0.0_f32, |acc, y| acc.max(y));

    for report in &balance_reports {
        assert!(
            (report.balanced_bottom_y - tallest_baseline).abs() < 1.0,
            "Flow {} balanced bottom ({}) did not align with tallest baseline ({})",
            report.flow_id,
            report.balanced_bottom_y,
            tallest_baseline
        );
    }
}

//! Integration test suite for Dynamic Multi-Flow Anchoring ("Living Tzurat HaDaf").
//!
//! Verifies:
//! 1. Bi-Directional Anchoring: strict logical dependency between primary text segments and commentaries.
//! 2. Synchronized Pagination: overflowing primary text automatically pushes linked commentaries across page boundaries.
//! 3. Shared Height Balancing: dynamic bottom baseline alignment across all parallel flows on each page.
//! 4. Edge Cases: oversized commentary carry-over with orphan/widow control, cascading breaks, and DH auto-synthesis.

use std::collections::HashSet;
use tok_core::anchor::TextAnchor;
use tok_core::id::{FractionalIndex, NodeId};
use tok_core::model::{DocumentRoot, Flow, FlowId, FlowType, ParagraphNode};
use tok_core::multi_flow_anchor::{AnchorDefinition, AnchorId, CommentaryLink, MultiFlowAnchorMap};
use tok_typeset::{TypesettingEngine, TypesettingEngineConfig};

fn create_engine() -> TypesettingEngine {
    let config = TypesettingEngineConfig {
        page_width_pt: 595.0, // A4
        page_height_pt: 842.0,
        margin_top_pt: 40.0,
        margin_bottom_pt: 40.0,
        margin_inner_pt: 35.0,
        margin_outer_pt: 35.0,
        ..Default::default()
    };
    TypesettingEngine::new(config)
}

#[test]
fn test_talmud_live_anchor_cascading_on_primary_text_growth() {
    let engine = create_engine();

    let primary_flow_id = FlowId::new("gemara");
    let rashi_flow_id = FlowId::new("rashi");
    let tosafot_flow_id = FlowId::new("tosafot");

    let p1_id = NodeId::new();
    let p2_id = NodeId::new();

    let r1_id = NodeId::new();
    let r2_id = NodeId::new();

    let t1_id = NodeId::new();
    let t2_id = NodeId::new();

    let a1 = AnchorId::new("dh_meematai");
    let a2 = NodeId::new();
    let a2_id = AnchorId::new(format!("dh_tanna_{}", a2));

    let mut anchor_map = MultiFlowAnchorMap::new();
    anchor_map.register_anchor(
        AnchorDefinition::new(
            a1.clone(),
            primary_flow_id.clone(),
            p1_id,
            TextAnchor::start_of(p1_id),
        )
        .with_label("ד\"ה מאימתי"),
    );
    anchor_map.register_anchor(
        AnchorDefinition::new(
            a2_id.clone(),
            primary_flow_id.clone(),
            p2_id,
            TextAnchor::start_of(p2_id),
        )
        .with_label("ד\"ה תנא היכא קאי"),
    );

    // Link P1: R1 and T1
    anchor_map.link_commentary(CommentaryLink::new(
        rashi_flow_id.clone(),
        r1_id,
        a1.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(tosafot_flow_id.clone(), t1_id, a1));

    // Link P2: R2 and T2
    anchor_map.link_commentary(CommentaryLink::new(
        rashi_flow_id.clone(),
        r2_id,
        a2_id.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(tosafot_flow_id.clone(), t2_id, a2_id));

    // Initial state: P1 and P2 fit together on Page 1
    let mut doc = DocumentRoot::new("תלמוד בבלי ברכות ב ע\"א");
    doc.multi_flow_anchors = Some(anchor_map.clone());

    let sec = &mut doc.sections[0];
    sec.flows.clear();

    let mut gemara_flow = Flow::new(primary_flow_id.clone(), FlowType::Main);
    gemara_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "gemara",
        "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן, עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה, דִּבְרֵי רַבִּי אֱלִיעֶזֶר. וַחֲכָמִים אוֹמְרִים: עַד חֲצוֹת. רַבָּן גַּמְלִיאֵל אוֹמֵר: עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר.",
    ));
    gemara_flow.paragraphs[0].id = p1_id;

    gemara_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "gemara",
        "תַּנָּא הֵיכָא קָאֵי דְּקָתָנֵי מֵאֵימָתַי? וְתוּ, מַאי שְׁנָא דְּתָנֵי בְּעַרְבִית בְּרֵישָׁא, לִתְנֵי דְּשַׁחֲרִית בְּרֵישָׁא? תַּנָּא אַקְּרָא קָאֵי, דִּכְתִיב: בְּשָׁכְבְּךָ וּבְקוּמֶךָ. וְהָכִי קָתָנֵי: זְמַן קְרִיאַת שְׁמַע דִּשְׁכִיבָה אֵימַת? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.",
    ));
    gemara_flow.paragraphs[1].id = p2_id;

    let mut rashi_flow = Flow::new(rashi_flow_id.clone(), FlowType::CommentA);
    rashi_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "rashi",
        "<b>מֵאֵימָתַי קוֹרִין</b> - תַּנָּא אַקְּרָא קָאֵי דִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ, וְהָכִי קָתָנֵי: זְמַן קְרִיאַת שְׁמַע שֶׁל שְׁכִיבָה אֵימַת.",
    ));
    rashi_flow.paragraphs[0].id = r1_id;

    rashi_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "rashi",
        "<b>דְּקָתָנֵי מֵאֵימָתַי</b> - מִכְּלָל דִּפְשִׁיטָא לֵיהּ דְּחוֹבָה הִיא, וְאַלְמָא קָאֵי אַקְּרָא דְּאוֹרַיְיתָא.",
    ));
    rashi_flow.paragraphs[1].id = r2_id;

    let mut tosafot_flow = Flow::new(tosafot_flow_id.clone(), FlowType::CommentB);
    tosafot_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "tosafot",
        "<b>מֵאֵימָתַי קוֹרִין</b> - פֵּרֵשׁ רַשִׁ\"י דְּתַנָּא אַקְּרָא קָאֵי, וְתֵימַהּ לְרִ\"י, דְּהָא תְּנַן בִּבְרָכוֹת פֶּרֶק שֵׁנִי: הַקּוֹרֵא לְמַפְרֵעַ לֹא יָצָא.",
    ));
    tosafot_flow.paragraphs[0].id = t1_id;

    tosafot_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "tosafot",
        "<b>וְעוֹד יֵשׁ לוֹמַר</b> - דְּמִדִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ שָׁמְעִינַן תַּרְתֵּי: חֲדָא דְּחוֹבָה לִקְרוֹת, וַחֲדָא דִּזְמַן שְׁכִיבָה קָא קָבַע רַחֲמָנָא.",
    ));
    tosafot_flow.paragraphs[1].id = t2_id;

    sec.flows.push(gemara_flow);
    sec.flows.push(rashi_flow);
    sec.flows.push(tosafot_flow);

    let initial_pages = engine.typeset_document(&doc);
    assert!(!initial_pages.is_empty());

    // In initial state, both P1 and P2 fit on Page 1
    let p0 = &initial_pages[0];
    let p0_para_ids: HashSet<NodeId> = p0
        .frames
        .iter()
        .flat_map(|f| f.lines.iter().filter_map(|l| l.paragraph_id))
        .collect();

    assert!(p0_para_ids.contains(&p1_id), "P1 must be on Page 1");
    assert!(p0_para_ids.contains(&p2_id), "P2 must be on Page 1");
    assert!(p0_para_ids.contains(&r1_id), "R1 must be on Page 1");
    assert!(p0_para_ids.contains(&r2_id), "R2 must be on Page 1");
    assert!(p0_para_ids.contains(&t1_id), "T1 must be on Page 1");
    assert!(p0_para_ids.contains(&t2_id), "T2 must be on Page 1");

    // Dynamic Edit: P1 expands massively (simulate adding extensive Gemara discussion)
    // This pushes P2 to Page 2!
    let mut expanded_doc = doc.clone();
    let long_p1_text = (0..35)
        .map(|i| format!("קטע הרחבה בסוגיית הגמרא חלק {} מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול בתרומתן.", i))
        .collect::<Vec<_>>()
        .join(" ");

    expanded_doc.sections[0].flows[0].paragraphs[0].text = long_p1_text;

    let cascaded_pages = engine.typeset_document(&expanded_doc);
    assert!(
        cascaded_pages.len() >= 2,
        "Document must expand to at least 2 pages"
    );

    // Find the page index where P2 is located
    let p2_page_idx = cascaded_pages
        .iter()
        .position(|p| {
            p.frames
                .iter()
                .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(p2_id)))
        })
        .expect("P2 must be placed on some page");

    assert!(
        p2_page_idx >= 1,
        "P2 must be pushed beyond Page 0 (actual page: {})",
        p2_page_idx
    );

    // On all pages before p2_page_idx, neither P2 nor R2 nor T2 may appear
    for (page_idx, page) in cascaded_pages.iter().enumerate().take(p2_page_idx) {
        let page_paras: HashSet<NodeId> = page
            .frames
            .iter()
            .flat_map(|f| f.lines.iter().filter_map(|l| l.paragraph_id))
            .collect();

        assert!(
            !page_paras.contains(&p2_id),
            "P2 must not appear on Page {}",
            page_idx
        );
        assert!(
            !page_paras.contains(&r2_id),
            "R2 must not appear on Page {} before P2",
            page_idx
        );
        assert!(
            !page_paras.contains(&t2_id),
            "T2 must not appear on Page {} before P2",
            page_idx
        );
    }

    // On p2_page_idx, P2, R2, and T2 must appear together
    let p2_page_paras: HashSet<NodeId> = cascaded_pages[p2_page_idx]
        .frames
        .iter()
        .flat_map(|f| f.lines.iter().filter_map(|l| l.paragraph_id))
        .collect();

    assert!(
        p2_page_paras.contains(&p2_id),
        "P2 must appear on target page {}",
        p2_page_idx
    );
    assert!(
        p2_page_paras.contains(&r2_id),
        "R2 must cascade to target page {} with P2",
        p2_page_idx
    );
    assert!(
        p2_page_paras.contains(&t2_id),
        "T2 must cascade to target page {} with P2",
        p2_page_idx
    );

    // Initial commentaries R1 and T1 must be on Page 0
    let p0_paras: HashSet<NodeId> = cascaded_pages[0]
        .frames
        .iter()
        .flat_map(|f| f.lines.iter().filter_map(|l| l.paragraph_id))
        .collect();
    assert!(
        p0_paras.contains(&r1_id),
        "R1 must remain on Page 0 with P1"
    );
    assert!(
        p0_paras.contains(&t1_id),
        "T1 must remain on Page 0 with P1"
    );
}

#[test]
fn test_mikraot_gedolot_4_flow_synchronized_pagination_and_height_balancing() {
    let engine = create_engine();

    let torah_flow_id = FlowId::new("torah");
    let targum_flow_id = FlowId::new("targum");
    let rashi_flow_id = FlowId::new("rashi");
    let ramban_flow_id = FlowId::new("ramban");

    let v1_id = NodeId::new();
    let v2_id = NodeId::new();

    let c1_targum_id = NodeId::new();
    let c2_rashi_id = NodeId::new();
    let c3_ramban_id = NodeId::new();

    let c4_targum_id = NodeId::new();
    let c5_rashi_id = NodeId::new();
    let c6_ramban_id = NodeId::new();

    let a1 = AnchorId::new("dh_bereshit");
    let a2 = AnchorId::new("dh_yehi_or");

    let mut anchor_map = MultiFlowAnchorMap::new();
    anchor_map.register_anchor(AnchorDefinition::new(
        a1.clone(),
        torah_flow_id.clone(),
        v1_id,
        TextAnchor::start_of(v1_id),
    ));
    anchor_map.register_anchor(AnchorDefinition::new(
        a2.clone(),
        torah_flow_id.clone(),
        v2_id,
        TextAnchor::start_of(v2_id),
    ));

    // Verse 1 anchors
    anchor_map.link_commentary(CommentaryLink::new(
        targum_flow_id.clone(),
        c1_targum_id,
        a1.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(
        rashi_flow_id.clone(),
        c2_rashi_id,
        a1.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(
        ramban_flow_id.clone(),
        c3_ramban_id,
        a1,
    ));

    // Verse 2 anchors
    anchor_map.link_commentary(CommentaryLink::new(
        targum_flow_id.clone(),
        c4_targum_id,
        a2.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(
        rashi_flow_id.clone(),
        c5_rashi_id,
        a2.clone(),
    ));
    anchor_map.link_commentary(CommentaryLink::new(
        ramban_flow_id.clone(),
        c6_ramban_id,
        a2,
    ));

    let mut doc = DocumentRoot::new("מקראות גדולות בראשית");
    doc.multi_flow_anchors = Some(anchor_map);

    let sec = &mut doc.sections[0];
    sec.flows.clear();

    // 1. Torah flow with extensive verse 1 text
    let mut torah_flow = Flow::new(torah_flow_id.clone(), FlowType::Main);
    let long_v1 = (0..80)
        .map(|i| format!("בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ פסוק א חלק {}", i))
        .collect::<Vec<_>>()
        .join(" ");
    torah_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "torah",
        &long_v1,
    ));
    torah_flow.paragraphs[0].id = v1_id;

    torah_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "torah",
        "וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר וַיְהִי־אוֹר׃ וַיַּרְא אֱלֹהִים אֶת־הָאוֹר כִּי־טוֹב.",
    ));
    torah_flow.paragraphs[1].id = v2_id;

    // 2. Targum
    let mut targum_flow = Flow::new(targum_flow_id.clone(), FlowType::CommentA);
    targum_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "targum",
        "בְּקַדְמִין בְּרָא יְיָ יָת שְׁמַיָּא וְיָת אַרְעָא.",
    ));
    targum_flow.paragraphs[0].id = c1_targum_id;
    targum_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "targum",
        "וַאֲמַר יְיָ יְהֵי נְהוֹרָא וַהֲוָה נְהוֹרָא.",
    ));
    targum_flow.paragraphs[1].id = c4_targum_id;

    // 3. Rashi
    let mut rashi_flow = Flow::new(rashi_flow_id.clone(), FlowType::CommentB);
    rashi_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "rashi",
        "<b>בְּרֵאשִׁית בָּרָא</b> — אָמַר רַבִּי יִצְחָק: לֹא הָיָה צָרִיךְ לְהַתְחִיל אֶת הַתּוֹרָה אֶלָּא מֵ״הַחֹדֶשׁ הַזֶּה לָכֶם״.",
    ));
    rashi_flow.paragraphs[0].id = c2_rashi_id;
    rashi_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "rashi",
        "<b>יְהִי אוֹר</b> — רָאָה שֶׁאֵין כְּדַאי לְהִשְׁתַּמֵּשׁ בּוֹ רְשָׁעִים וְהִבְדִּילוֹ לַצַּדִּיקִים.",
    ));
    rashi_flow.paragraphs[1].id = c5_rashi_id;

    // 4. Ramban
    let mut ramban_flow = Flow::new(ramban_flow_id.clone(), FlowType::CommentB);
    ramban_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "ramban",
        "<b>בְּרֵאשִׁית</b> — בְּרִיאָה יֵשׁ מֵאַיִן הַגָּמוּר, כִּי לֹא הָיָה הָעוֹלָם קַדְמוֹן כְּלָל.",
    ));
    ramban_flow.paragraphs[0].id = c3_ramban_id;
    ramban_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "ramban",
        "<b>יְהִי אוֹר</b> — אוֹר שֶׁנִּבְרָא בְּיוֹם רִאשׁוֹן נִגְנַז לֶעָתִיד לָבוֹא.",
    ));
    ramban_flow.paragraphs[1].id = c6_ramban_id;

    sec.flows.push(torah_flow);
    sec.flows.push(targum_flow);
    sec.flows.push(rashi_flow);
    sec.flows.push(ramban_flow);

    let pages = engine.typeset_document(&doc);
    assert!(pages.len() >= 2, "Document must span at least 2 pages");

    let v2_page_idx = pages
        .iter()
        .position(|p| {
            p.frames
                .iter()
                .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(v2_id)))
        })
        .expect("V2 must be placed on some page");

    assert!(v2_page_idx >= 1, "V2 must be pushed beyond Page 0");

    let p0_nodes: HashSet<NodeId> = pages[0]
        .frames
        .iter()
        .flat_map(|f| f.lines.iter().filter_map(|l| l.paragraph_id))
        .collect();
    let v2_nodes: HashSet<NodeId> = pages[v2_page_idx]
        .frames
        .iter()
        .flat_map(|f| f.lines.iter().filter_map(|l| l.paragraph_id))
        .collect();

    // Verse 1 and its commentaries on Page 0
    assert!(p0_nodes.contains(&v1_id));
    assert!(p0_nodes.contains(&c1_targum_id));
    assert!(p0_nodes.contains(&c2_rashi_id));
    assert!(p0_nodes.contains(&c3_ramban_id));

    // Verse 2 and ALL its commentaries automatically cascaded to v2_page_idx
    assert!(v2_nodes.contains(&v2_id));
    assert!(!p0_nodes.contains(&v2_id));

    assert!(
        v2_nodes.contains(&c4_targum_id),
        "Targum for V2 must cascade to Page {}",
        v2_page_idx
    );
    assert!(
        v2_nodes.contains(&c5_rashi_id),
        "Rashi for V2 must cascade to Page {}",
        v2_page_idx
    );
    assert!(
        v2_nodes.contains(&c6_ramban_id),
        "Ramban for V2 must cascade to Page {}",
        v2_page_idx
    );

    assert!(!p0_nodes.contains(&c4_targum_id));
    assert!(!p0_nodes.contains(&c5_rashi_id));
    assert!(!p0_nodes.contains(&c6_ramban_id));
}

#[test]
fn test_oversized_commentary_clean_break_and_carryover_with_orphan_widow_control() {
    let engine = create_engine();

    let primary_flow_id = FlowId::new("gemara");
    let tosafot_flow_id = FlowId::new("tosafot");

    let p1_id = NodeId::new();
    let t_huge_id = NodeId::new();

    let a1 = AnchorId::new("dh_huge_tosafot");
    let mut anchor_map = MultiFlowAnchorMap::new();
    anchor_map.register_anchor(AnchorDefinition::new(
        a1.clone(),
        primary_flow_id.clone(),
        p1_id,
        TextAnchor::start_of(p1_id),
    ));
    anchor_map.link_commentary(CommentaryLink::new(tosafot_flow_id.clone(), t_huge_id, a1));

    let mut doc = DocumentRoot::new("תוספות ענק");
    doc.multi_flow_anchors = Some(anchor_map);

    let sec = &mut doc.sections[0];
    sec.flows.clear();

    let mut gemara_flow = Flow::new(primary_flow_id, FlowType::Main);
    gemara_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "gemara",
        "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.",
    ));
    gemara_flow.paragraphs[0].id = p1_id;

    // Huge Tosafot paragraph with ~70 lines (exceeds single page column capacity)
    let huge_tosafot_text = (0..70)
        .map(|i| format!("שורה מספר {} בתוספות הארוך הדן בעומק שיטת רבנו תם וקושיות הר\"י מקורביל על סוגיית קריאת שמע של ערבית.", i))
        .collect::<Vec<_>>()
        .join(" ");

    let mut tosafot_flow = Flow::new(tosafot_flow_id, FlowType::CommentB);
    tosafot_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "tosafot",
        &huge_tosafot_text,
    ));
    tosafot_flow.paragraphs[0].id = t_huge_id;

    sec.flows.push(gemara_flow);
    sec.flows.push(tosafot_flow);

    let pages = engine.typeset_document(&doc);
    assert!(
        pages.len() >= 2,
        "Oversized Tosafot must carry over to at least Page 2"
    );

    // Verify Tosafot appears on both Page 1 and Page 2
    let p1_tosafot_lines = pages[0]
        .frames
        .iter()
        .find(|f| f.flow_id == "tosafot")
        .map(|f| f.lines.len())
        .unwrap_or(0);

    let p2_tosafot_lines = pages[1]
        .frames
        .iter()
        .find(|f| f.flow_id == "tosafot")
        .map(|f| f.lines.len())
        .unwrap_or(0);

    assert!(
        p1_tosafot_lines >= 2,
        "Page 1 must have at least 2 lines (orphan protection)"
    );
    assert!(
        p2_tosafot_lines >= 2,
        "Page 2 must have at least 2 lines (widow protection)"
    );
    assert!(
        p1_tosafot_lines + p2_tosafot_lines >= 40,
        "Total lines placed across pages must accommodate the oversized paragraph"
    );
}

#[test]
fn test_auto_synthesized_anchors_from_dibur_hamatchil() {
    let engine = create_engine();

    // Document without explicit multi_flow_anchors
    let mut doc = DocumentRoot::new("תלמוד עם זיהוי ד\"ה אוטומטי");
    assert!(doc.multi_flow_anchors.is_none());

    let sec = &mut doc.sections[0];
    sec.flows.clear();

    let mut gemara_flow = Flow::new(FlowId::new("gemara"), FlowType::Main);
    // P1: Long Gemara that fills page 1
    let p1_text = (0..35)
        .map(|i| {
            format!(
                "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית משעה שהכהנים נכנסים לאכול בתרומתן קטע {}",
                i
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    gemara_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "gemara",
        &p1_text,
    ));
    // P2: Second Gemara paragraph that pushes to page 2
    gemara_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "gemara",
        "תַּנָּא הֵיכָא קָאֵי דְּקָתָנֵי מֵאֵימָתַי? וְתוּ מַאי שְׁנָא דְּתָנֵי בְּעַרְבִית בְּרֵישָׁא?",
    ));

    let mut rashi_flow = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
    // R1: matches P1 DH
    rashi_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::initial(),
        "rashi",
        "<b>מֵאֵימָתַי קוֹרִין</b> - תַּנָּא אַקְּרָא קָאֵי דִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ.",
    ));
    // R2: matches P2 DH
    rashi_flow.paragraphs.push(ParagraphNode::new(
        FractionalIndex::new("a1"),
        "rashi",
        "<b>דְּקָתָנֵי מֵאֵימָתַי</b> - מִכְּלָל דִּפְשִׁיטָא לֵיהּ דְּחוֹבָה הִיא.",
    ));

    let p2_id = gemara_flow.paragraphs[1].id;
    let r2_id = rashi_flow.paragraphs[1].id;

    sec.flows.push(gemara_flow);
    sec.flows.push(rashi_flow);

    let pages = engine.typeset_document(&doc);
    assert!(pages.len() >= 2, "Must split into at least 2 pages");

    let p2_page_idx = pages
        .iter()
        .position(|p| {
            p.frames
                .iter()
                .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(p2_id)))
        })
        .expect("P2 must be placed on some page");

    assert!(
        p2_page_idx > 0,
        "P2 must be pushed beyond Page 0 due to 35-line P1"
    );

    for (idx, page) in pages.iter().enumerate().take(p2_page_idx) {
        let page_has_r2 = page
            .frames
            .iter()
            .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(r2_id)));
        assert!(
            !page_has_r2,
            "Auto-synthesized DH anchoring must prevent R2 from appearing on Page {} before P2 (on Page {})",
            idx, p2_page_idx
        );
    }

    let p2_page_has_r2 = pages[p2_page_idx]
        .frames
        .iter()
        .any(|f| f.lines.iter().any(|l| l.paragraph_id == Some(r2_id)));
    assert!(
        p2_page_has_r2,
        "R2 must appear on Page {} alongside its auto-anchored primary paragraph P2",
        p2_page_idx
    );
}

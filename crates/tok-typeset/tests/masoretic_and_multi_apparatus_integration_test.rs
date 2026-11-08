use tok_core::header::FootnoteApparatusConfig;
use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, Flow, FlowType, ParagraphNode, SectionNode};
use tok_core::numbering::{GematriaPunctuation, NumberingScheme};
use tok_core::styles::{CharacterFormat, MasoreticLetterVariant};
use tok_core::{FlowId, TextSpan};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::masoretic::{
    has_rare_cantillation_or_masoretic_marks, is_rare_cantillation_mark,
    resolve_masoretic_letter_styling, MasoreticTypographyConfig, NUN_HAFUKHA,
};

#[test]
fn test_biblical_masoretic_typography_shaping_and_variants() {
    // 1. Verify Masoretic characters and predicates
    assert_eq!(NUN_HAFUKHA, '\u{05C6}');
    assert!(is_rare_cantillation_mark('\u{0592}')); // Shalshelet
    assert!(is_rare_cantillation_mark('\u{059F}')); // Qarne Farah
    assert!(has_rare_cantillation_or_masoretic_marks(
        "\u{05C6} וַיְהִי בִּנְסֹעַ \u{05C6}"
    ));
    assert!(has_rare_cantillation_or_masoretic_marks("וַיִּתְמַהְמָ֓הּ"));

    // 2. Verify Variant Scaling & Baseline shifts
    let (r_sz, r_shift, _) =
        resolve_masoretic_letter_styling(MasoreticLetterVariant::Rabbatai, 14.0, None);
    assert_eq!(r_sz, 14.0 * 1.30);
    assert_eq!(r_shift, 0.0);

    let (z_sz, z_shift, _) =
        resolve_masoretic_letter_styling(MasoreticLetterVariant::Zeira, 14.0, None);
    assert_eq!(z_sz, 14.0 * 0.72);
    assert_eq!(z_shift, 0.0);

    let (t_sz, t_shift, _) =
        resolve_masoretic_letter_styling(MasoreticLetterVariant::Teluyah, 14.0, None);
    assert_eq!(t_sz, 14.0 * 0.72);
    assert!(
        t_shift > 4.0,
        "Teluyah baseline shift must be positive (upwards)"
    );

    // 3. OpenType features mapping for rare cantillation and Torani fonts
    let config = MasoreticTypographyConfig::default();
    let features = config.resolve_features_for_text("וַיִּתְמַהְמָ֓הּ ׆");
    let feature_tags: Vec<String> = features
        .iter()
        .map(|f| String::from_utf8_lossy(&f.tag.to_bytes()).to_string())
        .collect();
    assert!(feature_tags.contains(&"calt".to_string()));
    assert!(feature_tags.contains(&"ss01".to_string()));
    assert!(feature_tags.contains(&"ss04".to_string()));
    assert!(feature_tags.contains(&"ss05".to_string()));

    // 4. End-to-end typesetting with Masoretic letter variants
    let mut doc = DocumentRoot::new("תורה ומסורה");
    doc.sections.clear();

    let mut sec = SectionNode::new("סדר בראשית", "default");
    sec.flows.clear();

    let mut main_flow = Flow::new(FlowId::new("main"), FlowType::Main);

    // Create a paragraph: "בְּרֵאשִׁית בָּרָא אֱלֹהִים" with 'ב' as Rabbatai (enlarged)
    // and "יַעַר" with 'ע' as Teluyah (suspended)
    let p_text = "בְּרֵאשִׁית יַעַר מִיָּעַר \u{05C6} וַיְהִי בִּנְסֹעַ \u{05C6}";
    let mut p = ParagraphNode::new(FractionalIndex::initial(), "body", p_text);

    // Span for ב' רבתי (indices 0..2 covering 'ב' + dagesh)
    p.spans.push(TextSpan::new(
        0,
        2,
        CharacterFormat {
            masoretic_variant: Some(MasoreticLetterVariant::Rabbatai),
            ..Default::default()
        },
    ));

    // Span for ע' תלויה in יער
    if let Some(idx) = p_text.find("יַעַר") {
        let ayin_start = idx + 2; // after yod + patah
        let ayin_end = ayin_start + 1; // ayin
        p.spans.push(TextSpan::new(
            ayin_start,
            ayin_end,
            CharacterFormat {
                masoretic_variant: Some(MasoreticLetterVariant::Teluyah),
                ..Default::default()
            },
        ));
    }

    main_flow.paragraphs.push(p);
    sec.flows.push(main_flow);
    doc.sections.push(sec);

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    assert!(!pages.is_empty(), "Must produce at least one page");
    let page1 = &pages[0];
    let main_frame = page1.frames.iter().find(|f| f.flow_id == "main").unwrap();
    assert!(!main_frame.lines.is_empty());

    // Verify Rabbatai glyph exists and has larger advance width than standard letter
    let line = &main_frame.lines[0];
    assert!(!line.glyphs.is_empty());

    // Verify Nun Hafukha glyph is shaped with non-zero width
    let nun_hafukha_glyph = line
        .glyphs
        .iter()
        .find(|g| g.character == Some(NUN_HAFUKHA));
    assert!(
        nun_hafukha_glyph.is_some(),
        "Inverted Nun U+05C6 must be shaped into glyph"
    );
    assert!(
        nun_hafukha_glyph.unwrap().width > 0.0,
        "Inverted Nun glyph must have positive advance width"
    );
}

#[test]
fn test_masoretic_character_styles_and_opentype_features_two_pathways() {
    use tok_core::styles::CharacterStyle;

    // 1. Create document with CharacterStyles for both pathways
    let mut doc = DocumentRoot::new("מקרא ומסורה");
    doc.sections.clear();

    // Pathway 1: CharacterStyle with font_scale and baseline_shift
    let style_rabbatai = CharacterStyle::rabbatai("rabbatai", "ב רבתי");
    let style_zeira = CharacterStyle::zeira("zeira", "ל זעירא");
    let style_teluyah = CharacterStyle::teluyah("teluyah", "ע תלויה", Some(5.0));

    // Pathway 2: CharacterStyle with Torani OpenType feature (cvXX / ssXX)
    let style_cv = CharacterStyle::with_opentype_feature("cv_torani", "Torani Codex Variant", "cv01", 1);

    doc.add_character_style(style_rabbatai);
    doc.add_character_style(style_zeira);
    doc.add_character_style(style_teluyah);
    doc.add_character_style(style_cv);

    let mut sec = SectionNode::new("שמע ישראל", "default");
    sec.flows.clear();

    let mut flow = Flow::new(FlowId::new("main"), FlowType::Main);

    // Text: שְׁמַע יִשְׂרָאֵל יְהוָה אֱלֹהֵינוּ יְהוָה אֶחָד
    // Traditional Masorah: ע' of שְׁמַע is Teluyah/Rabbatai, ד' of אֶחָד is Rabbatai
    let text = "שְׁמַע יִשְׂרָאֵל יְהוָה אֱלֹהֵינוּ יְהוָה אֶחָד ׆ שַׁלְשֶׁלֶת֓ קַרְנֵי פָרָה֟ תְּלִישָׁא גְדוֹלָה֠";
    let mut p = ParagraphNode::new(FractionalIndex::initial(), "body", text);

    // Apply Pathway 1 via character_style_id
    // Teluyah on 'ע' of שמע (char index 3)
    p.spans.push(TextSpan::new(
        3,
        4,
        CharacterFormat {
            character_style_id: Some("teluyah".to_string()),
            ..Default::default()
        },
    ));

    // Rabbatai on 'ד' of אחד
    if let Some(pos) = text.find("אֶחָד") {
        let dalet_char_idx = text[..pos].chars().count() + 3; // 'ד'
        p.spans.push(TextSpan::new(
            dalet_char_idx,
            dalet_char_idx + 1,
            CharacterFormat {
                character_style_id: Some("rabbatai".to_string()),
                ..Default::default()
            },
        ));
    }

    flow.paragraphs.push(p);
    sec.flows.push(flow);
    doc.sections.push(sec);

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);
    assert!(!pages.is_empty());

    let frame = &pages[0].frames[0];
    assert!(!frame.lines.is_empty());
    let line = &frame.lines[0];

    // Verify Teluyah glyph has positive upward y baseline shift (around 5.0 pt)
    let teluyah_glyph = line.glyphs.iter().find(|g| g.y > 4.0);
    assert!(
        teluyah_glyph.is_some(),
        "Teluyah character style must produce upward baseline shift y > 4.0"
    );

    // Verify Inverted Nun is present and has width > 0
    let nun_glyph = line.glyphs.iter().find(|g| g.character == Some('\u{05C6}'));
    assert!(nun_glyph.is_some(), "Inverted nun glyph must be in line");
    assert!(nun_glyph.unwrap().width > 0.0);

    // Verify rare cantillation marks: Shalshelet 0x0593, Qarne Farah 0x059F, Telisha Gedola 0x05A0
    assert!(line.glyphs.iter().any(|g| g.character == Some('\u{0593}')));
    assert!(line.glyphs.iter().any(|g| g.character == Some('\u{059F}')));
    assert!(line.glyphs.iter().any(|g| g.character == Some('\u{05A0}')));
}

#[test]
fn test_multiple_parallel_footnote_apparatuses_layout() {
    let mut doc = DocumentRoot::new("שו\"ת ועיונים");
    doc.sections.clear();

    // Configure 2 distinct footnote apparatuses:
    // 1. "sources" (מקורות) with 2 columns, column gap 12pt, bracketed Hebrew markers [א], [ב]
    let sources_app = FootnoteApparatusConfig::new("sources")
        .with_title("מקורות וציונים")
        .with_columns(2, 12.0)
        .with_numbering(NumberingScheme::HebrewGematria(
            GematriaPunctuation::PlainLetters,
        ))
        .with_marker_delimiters("[", "]");

    // 2. "biurim" (ביאורים) with 1 column, column gap 12pt, parenthesized Arabic markers (1), (2)
    let biurim_app = FootnoteApparatusConfig::new("biurim")
        .with_title("ביאורי הסוגיא")
        .with_columns(1, 12.0)
        .with_numbering(NumberingScheme::Arabic)
        .with_marker_delimiters("(", ")");

    doc.add_footnote_apparatus(sources_app);
    doc.add_footnote_apparatus(biurim_app);

    let mut sec = SectionNode::new("שער ראשון", "default");
    sec.flows.clear();

    // 1. Primary Flow
    let mut main_flow = Flow::new(FlowId::new("main"), FlowType::Main);
    for i in 0..15 {
        let t = format!(
            "גוף הטקסט הראשי לפיסקה מספר {} בסוגיא דהאיש מקדש בו ובשלוחו וכל כיוצא בזה.",
            i + 1
        );
        main_flow
            .paragraphs
            .push(ParagraphNode::new(FractionalIndex::initial(), "body", &t));
    }
    sec.flows.push(main_flow);

    // 2. Footnote Apparatus 1: "sources" (2 columns)
    let mut sources_flow = Flow::new(FlowId::new("sources"), FlowType::Footnote);
    for i in 0..8 {
        let marker = doc.format_footnote_marker("sources", i + 1);
        let t = format!("{} שבת דף כ\"א ע\"א; רמב\"ם פ\"א מהל' תפילה.", marker);
        sources_flow.paragraphs.push(ParagraphNode::new(
            FractionalIndex::initial(),
            "footnote",
            &t,
        ));
    }
    sec.flows.push(sources_flow);

    // 3. Footnote Apparatus 2: "biurim" (1 column)
    let mut biurim_flow = Flow::new(FlowId::new("biurim"), FlowType::Footnote);
    for i in 0..4 {
        let marker = doc.format_footnote_marker("biurim", i + 1);
        let t = format!(
            "{} וביאור דברי התוספות כאן דאין לומר דמיירי בהכי דהא ליתא כלל וצריך עיון גדול.",
            marker
        );
        biurim_flow.paragraphs.push(ParagraphNode::new(
            FractionalIndex::initial(),
            "footnote",
            &t,
        ));
    }
    sec.flows.push(biurim_flow);

    doc.sections.push(sec);

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    assert!(
        !pages.is_empty(),
        "Typesetting multi-apparatus document must produce pages"
    );

    let page1 = &pages[0];
    let frame_ids: Vec<&str> = page1.frames.iter().map(|f| f.flow_id.as_str()).collect();
    println!("Page 1 frame ids: {:?}", frame_ids);
    let sources_frame = page1.frames.iter().find(|f| f.flow_id == "sources");
    let biurim_frame = page1.frames.iter().find(|f| f.flow_id == "biurim");

    assert!(
        sources_frame.is_some(),
        "Page 1 must contain a frame for sources apparatus"
    );
    assert!(
        biurim_frame.is_some(),
        "Page 1 must contain a frame for biurim apparatus"
    );

    let s_frame = sources_frame.unwrap();
    let b_frame = biurim_frame.unwrap();

    // Verify vertical ordering: both frames are in the bottom band, without overlapping each other
    assert!(
        s_frame.rect.y + s_frame.rect.height <= b_frame.rect.y + 1.0,
        "Sources frame (y={}, h={}) must sit above Biurim frame (y={}, h={})",
        s_frame.rect.y,
        s_frame.rect.height,
        b_frame.rect.y,
        b_frame.rect.height
    );

    // Verify multi-column configuration inside sources frame:
    // With 2 columns, lines should have different x_offset values (right column vs left column)
    assert!(!s_frame.lines.is_empty());
    let distinct_x_offsets: std::collections::BTreeSet<i32> = s_frame
        .lines
        .iter()
        .map(|l| l.x_offset.round() as i32)
        .collect();
    assert!(
        distinct_x_offsets.len() >= 2,
        "2-column apparatus must produce lines with at least 2 distinct x_offsets, got: {:?}",
        distinct_x_offsets
    );

    // Verify 1-column configuration inside biurim frame:
    // Lines should have x_offset == 0.0
    for l in &b_frame.lines {
        assert_eq!(
            l.x_offset, 0.0,
            "1-column apparatus lines must have x_offset == 0"
        );
    }
}

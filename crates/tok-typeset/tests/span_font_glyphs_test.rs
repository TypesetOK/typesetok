//! A run is drawn with the font it is tagged with (`LineBox::fonts[font_index]`), so its
//! glyph ids must come from that font. Otherwise the PDF shows garbage (issue #40).

use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, Flow, FlowType, ParagraphNode, SectionNode};
use tok_core::styles::CharacterFormat;
use tok_core::{FlowId, TextSpan};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};

const TEXT: &str = "וכתב רבינו שכך היא ההלכה";

fn typeset_with_span(format: CharacterFormat) -> (TypesettingEngine, Vec<(String, char, u32)>) {
    let mut doc = DocumentRoot::new("span fonts");
    doc.sections.clear();
    let mut sec = SectionNode::new("s", "default");
    sec.flows.clear();
    let mut flow = Flow::new(FlowId::new("main"), FlowType::Main);
    let mut p = ParagraphNode::new(FractionalIndex::initial(), "body", TEXT);
    p.spans.push(TextSpan::new(0, TEXT.chars().count(), format));
    flow.paragraphs.push(p);
    sec.flows.push(flow);
    doc.sections.push(sec);

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);
    let mut glyphs = Vec::new();
    for frame in &pages[0].frames {
        for line in &frame.lines {
            for g in &line.glyphs {
                let Some(ch) = g.character.filter(|c| !c.is_whitespace()) else {
                    continue;
                };
                glyphs.push((line.fonts[usize::from(g.font_index)].clone(), ch, g.glyph_id));
            }
        }
    }
    assert!(!glyphs.is_empty());
    (engine, glyphs)
}

fn assert_ids_belong_to_tagged_font(engine: &TypesettingEngine, glyphs: &[(String, char, u32)]) {
    for (family, ch, gid) in glyphs {
        let font = engine.font_manager.get_font(family).unwrap();
        let face = font.shaping_face().unwrap();
        let expected = face.glyph_index(*ch).map(|g| u32::from(g.0));
        assert_eq!(expected, Some(*gid), "{ch} tagged {family} has a glyph id of another font");
    }
}

#[test]
fn span_in_another_family_is_shaped_with_that_family() {
    let (engine, glyphs) = typeset_with_span(CharacterFormat {
        font_family: Some("David Libre".to_string()),
        ..Default::default()
    });
    assert!(glyphs.iter().all(|(f, _, _)| f == "David Libre"));
    assert_ids_belong_to_tagged_font(&engine, &glyphs);
}

#[test]
fn bold_span_is_shaped_with_the_bold_face() {
    let (engine, glyphs) = typeset_with_span(CharacterFormat {
        font_weight: Some(700),
        ..Default::default()
    });
    assert!(glyphs.iter().all(|(f, _, _)| f == "Frank Ruhl Libre Bold"));
    assert_ids_belong_to_tagged_font(&engine, &glyphs);
}

#[test]
fn bold_span_in_a_family_without_bold_keeps_the_regular_face() {
    let (engine, glyphs) = typeset_with_span(CharacterFormat {
        font_family: Some("David Libre".to_string()),
        font_weight: Some(700),
        ..Default::default()
    });
    assert!(glyphs.iter().all(|(f, _, _)| f == "David Libre"));
    assert_ids_belong_to_tagged_font(&engine, &glyphs);
}

use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, ParagraphNode, SectionNode};
use tok_core::notes::{InlineNote, Sidenote, SidenoteConfig, SidenoteMarginSide};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};

#[test]
fn test_inline_notes_weave_into_knuth_plass_flow() {
    let mut doc = DocumentRoot::new("הערות משולבות");
    let mut sec = SectionNode::new("פרק א", "default");

    let mut p = ParagraphNode::new(
        FractionalIndex::initial(),
        "default-body",
        "אמר רב יהודה אמר שמואל הלכה כרבי שמעון בן גמליאל בכל מקום.",
    );
    p = p.with_inline_note(InlineNote::new(19, "בבלי שבת דף מ עמוד ב"));

    sec.main_flow_mut().unwrap().paragraphs.push(p);
    doc.sections = vec![sec];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    assert_eq!(pages.len(), 1);
    let combined_text: String = pages[0].frames[0]
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        combined_text.contains("(בבלי שבת דף מ עמוד ב)"),
        "Expected inline note text woven into paragraph lines, got: {}",
        combined_text
    );
}

#[test]
fn test_sidenotes_marginalia_and_collision_avoidance() {
    let mut doc = DocumentRoot::new("סיידנוטס ומסורת השס");
    let mut sec = SectionNode::new("דף ב", "default").with_sidenote_config(SidenoteConfig {
        enabled: true,
        default_side: SidenoteMarginSide::Outer,
        width_pt: 68.0,
        gap_from_body_pt: 10.0,
        vertical_gap_pt: 6.0,
        ..Default::default()
    });

    let p = ParagraphNode::new(
        FractionalIndex::initial(),
        "default-body",
        "מאימתי קורין את שמע בערבין משעה שהכהנים נכנסים לאכול בתרומתן.",
    )
    .with_sidenote(Sidenote::new(0, "מסורת הש\"ס: ברכות דף ב ע\"א").with_marker("א"))
    .with_sidenote(Sidenote::new(20, "עין משפט: רמב\"ם פ\"א מהל' ק\"ש").with_marker("ב"));

    sec.main_flow_mut().unwrap().paragraphs.push(p);
    doc.sections = vec![sec];

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let pages = engine.typeset_document(&doc);

    assert_eq!(pages.len(), 1);
    let sidenote_frames: Vec<_> = pages[0]
        .frames
        .iter()
        .filter(|f| f.flow_id == "sidenotes")
        .collect();
    assert_eq!(
        sidenote_frames.len(),
        2,
        "Expected 2 sidenote frames on page 1"
    );

    let f0 = sidenote_frames[0];
    let f1 = sidenote_frames[1];
    // Ensure no vertical overlap (collision avoidance pushed f1 below f0)
    assert!(
        f1.rect.y >= f0.rect.y + f0.rect.height + 5.9,
        "Sidenotes overlapped: f0={:?}, f1={:?}",
        f0.rect,
        f1.rect
    );
}

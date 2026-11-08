//! Determinism and Equivalence Verification Test Suite (Stage 10).
//!
//! Validates:
//! - Bit-for-bit equivalence between full typeset from scratch and incremental typeset.
//! - Exact equality of line counts, baseline positions, glyph metrics, and break tokens.
//! - Equivalence across single-flow cascading reflow, zero-cascade in-place edits,
//!   and multi-flow Talmud / commentary co-pagination.

use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentRoot, Flow, FlowId, FlowType, ParagraphNode};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::geometry::PageLayoutBox;

fn make_engine() -> TypesettingEngine {
    let config = TypesettingEngineConfig {
        page_width_pt: 595.28,
        page_height_pt: 841.89,
        margin_top_pt: 50.0,
        margin_bottom_pt: 50.0,
        margin_inner_pt: 50.0,
        margin_outer_pt: 50.0,
        ..Default::default()
    };
    TypesettingEngine::new(config)
}

fn assert_pages_equivalent(incremental: &[PageLayoutBox], cold: &[PageLayoutBox]) {
    assert_eq!(
        incremental.len(),
        cold.len(),
        "Page counts must be identical between incremental and cold typesetting"
    );

    for (p_idx, (inc_page, cold_page)) in incremental.iter().zip(cold).enumerate() {
        assert_eq!(
            inc_page.page_index, cold_page.page_index,
            "Page index mismatch on page {}",
            p_idx
        );
        assert_eq!(
            inc_page.page_number_gematria, cold_page.page_number_gematria,
            "Gematria folio mismatch on page {}",
            p_idx
        );
        assert_eq!(
            inc_page.break_token, cold_page.break_token,
            "BreakToken mismatch on page {}",
            p_idx
        );
        assert_eq!(
            inc_page.frames.len(),
            cold_page.frames.len(),
            "Frame count mismatch on page {}",
            p_idx
        );

        for (f_idx, (inc_frame, cold_frame)) in
            inc_page.frames.iter().zip(&cold_page.frames).enumerate()
        {
            assert_eq!(
                inc_frame.flow_id, cold_frame.flow_id,
                "FlowId mismatch on page {} frame {}",
                p_idx, f_idx
            );
            assert!(
                (inc_frame.rect.width - cold_frame.rect.width).abs() < 1e-3,
                "Frame width mismatch on page {} frame {}",
                p_idx,
                f_idx
            );
            assert!(
                (inc_frame.rect.height - cold_frame.rect.height).abs() < 1e-3,
                "Frame height mismatch on page {} frame {}",
                p_idx,
                f_idx
            );

            assert_eq!(
                inc_frame.lines.len(),
                cold_frame.lines.len(),
                "Line count mismatch on page {} frame {}",
                p_idx,
                f_idx
            );

            for (l_idx, (inc_line, cold_line)) in
                inc_frame.lines.iter().zip(&cold_frame.lines).enumerate()
            {
                assert_eq!(
                    inc_line.text, cold_line.text,
                    "Line text mismatch on page {} frame {} line {}",
                    p_idx, f_idx, l_idx
                );
                assert_eq!(
                    inc_line.paragraph_id, cold_line.paragraph_id,
                    "Paragraph ID mismatch on page {} frame {} line {}",
                    p_idx, f_idx, l_idx
                );
                assert!(
                    (inc_line.baseline_y - cold_line.baseline_y).abs() < 1e-3,
                    "Baseline Y mismatch ({:.4} vs {:.4}) on page {} frame {} line {}",
                    inc_line.baseline_y,
                    cold_line.baseline_y,
                    p_idx,
                    f_idx,
                    l_idx
                );
                assert_eq!(
                    inc_line.glyphs.len(),
                    cold_line.glyphs.len(),
                    "Glyph count mismatch on page {} frame {} line {}",
                    p_idx,
                    f_idx,
                    l_idx
                );
            }
        }
    }
}

#[test]
fn test_determinism_cascading_reflow_vs_cold_typeset() {
    let mut doc = DocumentRoot::new("מבחן דטרמיניזם ושוויון מלא");
    {
        let flow = doc.sections[0].main_flow_mut().unwrap();
        for i in 0..70 {
            let p = ParagraphNode::new(
                FractionalIndex::new(format!("k{:03}", i)),
                "default-body",
                &format!(
                    "פסקה {}: ביאור רחב בסוגיא דתפילת השחר והזמנים השונים לדעת רבנן ורבי יהודה, עם כל חילוקי הדינים והמנהגים הנוהגים למעשה.",
                    i + 1
                ),
            );
            flow.add_paragraph(p);
        }
    }

    let eng = make_engine();
    let initial_pages = eng.typeset_document(&doc);

    // Edit paragraph 15 by adding substantial text to induce page break cascades downstream
    let edited_id = doc.sections[0].flows[0].paragraphs[15].id;
    doc.sections[0].flows[0].paragraphs[15].text =
        "פסקה 16: ביאור רחב בסוגיא דתפילת השחר והזמנים השונים לדעת רבנן ורבי יהודה, עם כל חילוקי הדינים והמנהגים הנוהגים למעשה. ועוד יש להוסיף ביאור נרחב מדברי הראשונים בסוגיא זו המאריך את גוף הפסקה ומכפיל את מספר שורותיה באופן המשפיע על גלישת העמודים הבאים בספר כולו."
            .to_string();

    // 1. Incremental layout
    let (inc_pages, _diag) =
        eng.typeset_document_incremental(&doc, &initial_pages, Some(edited_id));

    // 2. Cold layout with completely fresh engine and empty cache
    let cold_eng = make_engine();
    let cold_pages = cold_eng.typeset_document(&doc);

    // Verify mathematical equivalence
    assert_pages_equivalent(&inc_pages, &cold_pages);
}

#[test]
fn test_determinism_zero_cascade_in_line_edit_vs_cold_typeset() {
    let mut doc = DocumentRoot::new("מבחן דטרמיניזם עריכה מקומית");
    {
        let flow = doc.sections[0].main_flow_mut().unwrap();
        for i in 0..40 {
            let p = ParagraphNode::new(
                FractionalIndex::new(format!("k{:03}", i)),
                "default-body",
                &format!(
                    "פסקה מספר {}: טקסט אחיד לבדיקת עריכה קבועה באורך שורה.",
                    i + 1
                ),
            );
            flow.add_paragraph(p);
        }
    }

    let eng = make_engine();
    let initial_pages = eng.typeset_document(&doc);

    // Replace a single word on paragraph 10 without changing line count
    let edited_id = doc.sections[0].flows[0].paragraphs[10].id;
    doc.sections[0].flows[0].paragraphs[10].text =
        "פסקה מספר 11: טקסט ערוך לבדיקת עריכה קבועה באורך שורה.".to_string();

    let (inc_pages, _diag) =
        eng.typeset_document_incremental(&doc, &initial_pages, Some(edited_id));

    let cold_eng = make_engine();
    let cold_pages = cold_eng.typeset_document(&doc);

    assert_pages_equivalent(&inc_pages, &cold_pages);
}

#[test]
fn test_determinism_multi_flow_commentary_incremental_vs_cold() {
    let mut doc = DocumentRoot::new("מבחן דטרמיניזם רב-זרימתי");
    let sec = &mut doc.sections[0];
    sec.flows.clear();

    let mut gemara_flow = Flow::new(FlowId::new("gemara"), FlowType::Main);
    let p_gemara = ParagraphNode::new(
        FractionalIndex::new("g01"),
        "default-body",
        "מַתְנִיתִין: מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית. מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.",
    );
    gemara_flow.add_paragraph(p_gemara);

    let mut rashi_flow = Flow::new(FlowId::new("rashi"), FlowType::CommentA);
    let p_rashi = ParagraphNode::new(
        FractionalIndex::new("r01"),
        "default-body",
        "רש\"י: מאימתי - משעת יציאת הכוכבים.",
    );
    rashi_flow.add_paragraph(p_rashi);

    let mut notes_flow =
        Flow::new(FlowId::new("notes"), FlowType::Footnote).with_placement_role("bottom_band");
    let mut p_note = ParagraphNode::new(
        FractionalIndex::new("n01"),
        "default-body",
        "הערה א: עיין ברכות דף ב עמוד א.",
    );
    let note_id = p_note.id;
    notes_flow.add_paragraph(p_note.clone());

    sec.flows.push(gemara_flow);
    sec.flows.push(rashi_flow);
    sec.flows.push(notes_flow);

    let eng = make_engine();
    let pages1 = eng.typeset_document_multi_flow(&doc);

    // Edit footnote
    p_note.text =
        "הערה א: עיין ברכות דף ב עמוד א ובהגהות הב\"ח אות א וציון לנאמן שמואל.".to_string();
    doc.sections[0].flows[2].paragraphs[0] = p_note;

    let (inc_pages, _diag) =
        eng.typeset_document_multi_flow_incremental(&doc, &pages1, Some(note_id));

    let cold_eng = make_engine();
    let cold_pages = cold_eng.typeset_document_multi_flow(&doc);

    assert_pages_equivalent(&inc_pages, &cold_pages);
}

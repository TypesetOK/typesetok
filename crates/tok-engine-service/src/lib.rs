//! `tok-engine-service` — Shared Stateful Application Service for TypesetOK.
//!
//! Centralizes document loading/parsing, persistent [`WorkspaceSession`] state,
//! font inspection/loading, `.tok` packaging, preflight evaluation, and PDF/HTML
//! rendering so both `tok-cli` and future desktop hosts (`tok-desktop`) share a
//! single source of truth with an in-memory [`TypesettingEngine`] paragraph cache.

pub mod document;
pub mod error;
pub mod export;
pub mod io;
pub mod session;

pub use document::{
    build_font_manager, collect_referenced_font_families, create_sample_hebrew_document,
    inspect_font_bytes, inspect_font_file, inspect_package, load_document_input,
    load_package_or_doc, manifest_for_root, parse_document_json, run_builtin_plugins, save_package,
    sync_heading_numbers, PackageInspectionSummary,
};
pub use error::EngineServiceError;
pub use export::{
    evaluate_preflight, render_html_string, render_pdf_bytes, PdfRenderRequest, PreflightRequest,
};
pub use io::{format_hex, sanitize_asset_filename, write_atomic};
pub use session::{TypesetOutput, WorkspaceSession};

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tok_core::id::FractionalIndex;
    use tok_core::model::{DocumentModel, DocumentRoot, ParagraphNode};
    use tok_core::outline::{
        DocumentHierarchyNode, DocumentOutline, HierarchyLevel, OutlineNumberingConfig,
    };
    use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};

    #[test]
    fn workspace_session_reuses_paragraph_cache_and_font_manager_across_edits() {
        let mut session = WorkspaceSession::new();
        let model = create_sample_hebrew_document(12);
        let mut root = model.root().clone();

        let first = session.typeset_document(&root, false);
        assert!(!first.pages.is_empty());
        let after_first = session.cache_metrics();
        assert_eq!(after_first.misses, 12);
        assert_eq!(after_first.entry_count, 12);

        // Edit a single paragraph and re-typeset on the same persistent session.
        let flow = root.sections[0].main_flow_mut().unwrap();
        flow.paragraphs[3]
            .text
            .push_str(" תוספת בדיקה לעימוד מהיר.");

        let second = session.typeset_document(&root, true);
        assert!(second.preflight.is_some());
        let after_second = session.cache_metrics();
        assert!(
            after_second.hits >= 11,
            "expected >= 11 paragraph cache hits across edit, got {}",
            after_second.hits
        );

        // Output must be bit-for-bit equivalent to a cold TypesettingEngine.
        let fresh_engine =
            TypesettingEngine::new(TypesettingEngineConfig::from_document_root(&root));
        let fresh_pages = fresh_engine.typeset_document(&root);
        assert_eq!(second.pages, fresh_pages);
    }

    #[test]
    fn workspace_session_clears_cache_on_document_switch_or_geometry_change() {
        let mut session = WorkspaceSession::new();
        let doc1 = create_sample_hebrew_document(4);
        let mut root1 = doc1.root().clone();
        session.typeset_document(&root1, false);
        assert_eq!(session.cache_metrics().entry_count, 4);

        // Geometry change clears old width-bound cache entries cleanly.
        root1.metadata.document_settings = Some(serde_json::json!({
            "pageWidthMm": 170.0,
            "pageHeightMm": 240.0
        }));
        session.typeset_document(&root1, false);
        assert_eq!(session.cache_metrics().entry_count, 4);

        // Different DocumentRoot::id clears prior document's cache entries.
        let doc2 = create_sample_hebrew_document(2);
        session.typeset_document(doc2.root(), false);
        assert_eq!(session.cache_metrics().entry_count, 2);
    }

    #[test]
    fn parse_document_json_supports_root_and_multiflow() {
        let doc = create_sample_hebrew_document(2);
        let json = doc.root().to_json().unwrap();
        let (parsed, manifest) = parse_document_json(&json).unwrap();
        assert_eq!(parsed.root().id, doc.root().id);
        assert_eq!(manifest.title, doc.root().metadata.title);

        let raw_multi = serde_json::json!({
            "title": "מסמך מרובה זרימות",
            "templateType": "prose",
            "flows": {
                "main": [{ "id": "p1", "text": "בראשית ברא אלהים" }]
            }
        });
        let (converted, multi_manifest) =
            parse_document_json(&serde_json::to_string(&raw_multi).unwrap()).unwrap();
        assert_eq!(multi_manifest.title, "מסמך מרובה זרימות");
        assert_eq!(
            converted.root().sections[0].main_flow().unwrap().paragraphs[0].text,
            "בראשית ברא אלהים"
        );
    }

    #[test]
    fn inspect_font_validates_ttf_and_rejects_woff() {
        let dir = env::temp_dir().join(format!("tok_engine_service_font_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let ttf = dir.join("noto.ttf");
        fs::write(&ttf, tok_typeset::font::EMBEDDED_NOTO_SERIF_HEBREW).unwrap();
        let valid_report = inspect_font_file(&ttf).unwrap();
        assert_eq!(valid_report["valid"], serde_json::Value::Bool(true));
        assert_eq!(
            valid_report["report"]["family_name"],
            serde_json::Value::String("Noto Serif Hebrew".to_string())
        );

        let woff = dir.join("bad.woff");
        fs::write(&woff, b"wOFF\x00\x01\x00\x00\x00\x00\x00\x20bad").unwrap();
        let invalid_report = inspect_font_file(&woff).unwrap();
        assert_eq!(invalid_report["valid"], serde_json::Value::Bool(false));
        assert_eq!(
            invalid_report["error"],
            serde_json::Value::String("RejectWoff".to_string())
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_open_package_roundtrip_and_pdf_export() {
        let dir = env::temp_dir().join(format!("tok_engine_service_pkg_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pkg_path = dir.join("book.tok");

        let model = create_sample_hebrew_document(3);
        let manifest = manifest_for_root(model.root());
        save_package(&model, manifest, &pkg_path, None, None).unwrap();

        let summary = inspect_package(&pkg_path).unwrap();
        assert_eq!(summary.total_paragraphs, 3);
        assert_eq!(summary.title, "תלמוד בבלי - מסכת ברכות");

        let mut session = WorkspaceSession::new();
        session.open_package(&pkg_path, None).unwrap();
        let pdf_bytes = session
            .render_active_pdf(&PdfRenderRequest::default())
            .unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF-"));
        assert!(pdf_bytes.ends_with(b"%%EOF\n") || pdf_bytes.ends_with(b"%%EOF"));

        let html = session.render_active_html().unwrap();
        assert!(html.contains("<html"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_builtin_plugins_and_heading_sync_update_document() {
        let mut root = DocumentRoot::new("ספר הלכה");
        let flow = root.sections[0].main_flow_mut().unwrap();

        let p1 = ParagraphNode::new(
            FractionalIndex::initial(),
            "heading-siman",
            "אמר ר' משה בן מימון (רמב\"ם)",
        );
        let p1_id = p1.id;
        flow.paragraphs.push(p1);

        let mut outline = DocumentOutline::new();
        let siman = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות ברכות")
            .with_target_paragraph(p1_id);
        outline.add_root_node(siman);
        root.outline = Some(outline);

        let txs = run_builtin_plugins(&mut root);
        assert!(!txs.is_empty());
        assert_eq!(
            root.sections[0].main_flow().unwrap().paragraphs[0].text,
            "אמר ר׳ משה בן מימון (רמב״ם)"
        );

        let affected = sync_heading_numbers(&mut root, false);
        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].1, "סימן א׳");

        let _model = DocumentModel::new(root);
        let _cfg = OutlineNumberingConfig::torah_default(
            tok_core::numbering::GematriaPunctuation::StandardPunctuation,
        );
    }

    #[test]
    fn workspace_session_is_send_across_threads_and_preserves_cache() {
        use std::sync::{Arc, Mutex};

        let session = Arc::new(Mutex::new(WorkspaceSession::new()));
        let doc = create_sample_hebrew_document(8);
        let doc_json = doc.root().to_json().unwrap();

        {
            let mut guard = session.lock().unwrap();
            let first = guard
                .typeset_json_or_path(Some(&doc_json), None, None, None, false)
                .unwrap();
            assert!(!first.pages.is_empty());
            assert_eq!(guard.cache_metrics().misses, 8);
        }

        let worker_session = Arc::clone(&session);
        let handle = std::thread::spawn(move || {
            let mut guard = worker_session.lock().unwrap();
            let second = guard
                .typeset_json_or_path(Some(&doc_json), None, None, None, true)
                .unwrap();
            assert!(second.preflight.is_some());
            assert!(
                guard.cache_metrics().hits >= 8,
                "expected cross-thread cache hits >= 8, got {}",
                guard.cache_metrics().hits
            );
        });
        handle.join().unwrap();
    }

    #[test]
    fn session_save_render_and_preflight_facade_roundtrip() {
        let dir = env::temp_dir().join(format!("tok_engine_service_facade_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pkg_path = dir.join("facade.tok");
        let pdf_path = dir.join("facade.pdf");
        let html_path = dir.join("facade.html");

        let model = create_sample_hebrew_document(4);
        let doc_json = model.root().to_json().unwrap();

        let mut session = WorkspaceSession::new();
        session
            .save_document_json(&doc_json, &pkg_path, None, None)
            .unwrap();
        assert!(pkg_path.exists());

        let pdf_req = PdfRenderRequest::from_ui_options(
            Some("pdfx4"),
            Some("fogra51"),
            Some(false),
            Some(false),
            Some(true),
        );
        assert_eq!(pdf_req.standard, tok_pdf::pdf_engine::PdfXStandard::PdfX4);
        assert_eq!(pdf_req.bleed_pt, 0.0);
        assert!(!pdf_req.draw_crop_marks);
        assert!(pdf_req.draw_registration_marks);

        let pdf_len = session
            .render_pdf_from_json_or_path(None, Some(&pkg_path), &pdf_path, None, None, &pdf_req)
            .unwrap();
        assert!(pdf_len > 100);
        assert!(fs::read(&pdf_path).unwrap().starts_with(b"%PDF-"));

        let html_len = session
            .render_html_from_json_or_path(None, Some(&pkg_path), &html_path, None, None)
            .unwrap();
        assert!(html_len > 50);
        assert!(fs::read_to_string(&html_path).unwrap().contains("<html"));

        let pf_req = PreflightRequest::from_ui_options(Some("pdfx4"), Some(false));
        assert_eq!(pf_req.standard, tok_pdf::pdf_engine::PdfXStandard::PdfX4);
        assert_eq!(pf_req.bleed_pt, 0.0);
        let report = session
            .run_preflight_from_json_or_path(Some(&doc_json), None, None, Some(&pkg_path), &pf_req)
            .unwrap();
        assert!(!report.checks.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "Gate 1 300-page incremental typesetting benchmark (run with --release -- --ignored --nocapture)"]
    fn bench_300_page_incremental_typeset_latency() {
        use std::time::Instant;

        // ~8,100 Hebrew Mishnah/Gemara paragraphs -> ~300 pages
        let model = create_sample_hebrew_document(8100);
        let mut root = model.root().clone();
        let mut session = WorkspaceSession::new();

        let t_cold = Instant::now();
        let cold = session.typeset_document(&root, false);
        let cold_ms = t_cold.elapsed().as_secs_f64() * 1000.0;
        let page_count = cold.pages.len();
        assert!(page_count >= 250, "expected >= 250 pages, got {page_count}");

        // Edit 1 paragraph in the middle (same-length character tweak, typical live typing)
        let mid_idx = root.sections[0].main_flow().unwrap().paragraphs.len() / 2;
        let mid_id = {
            let flow = root.sections[0].main_flow_mut().unwrap();
            let p = &mut flow.paragraphs[mid_idx];
            p.text.push_str(" וְכֵן הֲלָכָה.");
            p.id
        };

        let t_inc = Instant::now();
        let (inc_pages, diag) = session.typeset_incremental_owned(root, Some(mid_id));
        let inc_ms = t_inc.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(inc_pages.len(), page_count);

        println!(
            "[GATE1-BENCH] pages={} cold_ms={:.2} incremental_ms={:.2} engine_ms={:.2} reused_pages={} recomputed_pages={} cache_hits={}",
            page_count,
            cold_ms,
            inc_ms,
            diag.duration.as_secs_f64() * 1000.0,
            diag.reused_pages,
            diag.recomputed_pages,
            session.cache_metrics().hits
        );
    }
}

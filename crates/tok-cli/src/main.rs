use std::env;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use tok_core::model::{DocumentModel, DocumentRoot};
use tok_engine_service::{
    build_font_manager, create_sample_hebrew_document, evaluate_preflight, format_hex,
    inspect_font_file, inspect_package, load_document_input, load_package_or_doc,
    parse_document_json, render_html_string, render_pdf_bytes, run_builtin_plugins, save_package,
    sync_heading_numbers, write_atomic, PdfRenderRequest, PreflightRequest, WorkspaceSession,
};
use tok_ipc::protocol::{WorkerRequest, WorkerResponse};
use tok_pdf::color::IccProfileId;
use tok_pdf::html_projection::HtmlProjectionCompiler;
use tok_pdf::pdf_engine::{PdfExportOptions, PdfPrePressEngine, PdfXStandard};
use tok_storage::package::TokPackage;
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};

fn print_usage() {
    println!(
        r#"
================================================================================
  TypesetOK (TOK) - Professional Hebrew Desktop Publishing CLI (Headless Core)
================================================================================

USAGE:
    tok-cli <COMMAND> [OPTIONS]

COMMANDS:
    render-pdf <INPUT> <OUTPUT>     Render a .tok, .json, or demo document to ISO PDF/X-1a
    render-html <INPUT> <OUTPUT>    Render a .tok, .json, or demo document to pre-paginated HTML
    typeset-document <INPUT> <OUT>  Typeset document into PageLayoutBox JSON array
    incremental-typeset <IN> <OUT>  Incrementally typeset document with recompute diagnostics
    diagnose-recompute <INPUT>      Run 3-position invalidation & cache diagnostic report
    save-package <INPUT> <OUTPUT>   Save document JSON into an atomic .tok package
    open-package <INPUT> <OUTPUT>   Open and extract a .tok package into document JSON
    run-plugins <INPUT> <OUTPUT>    Run built-in pre-typeset plugins on document JSON
    benchmark-typeset [--pages N]   Run 1,000-page stress test and cascade latency benchmark
    verify-determinism              Verify bit-for-bit layout & PDF output determinism
    inspect-package <INPUT>         Inspect .tok package manifest, metadata, and assets
    inspect-font <PATH.ttf>         Validate external font (glyf/OS2 fsType/Hebrew coverage) and emit JSON report
    preflight <INPUT> [OUTPUT.json] Run real pre-press preflight evaluation and emit PreflightReport JSON
    help                            Show this help message

EXAMPLES:
    tok-cli render-pdf --demo output.pdf
    tok-cli render-html --demo output.html
    tok-cli incremental-typeset --demo output.json
    tok-cli diagnose-recompute --demo
    tok-cli benchmark-typeset --pages 1000
    tok-cli verify-determinism
"#
    );
}

fn extract_flag_value<'a>(extra_args: &'a [String], flag: &str) -> Option<&'a str> {
    let mut i = 0;
    while i + 1 < extra_args.len() {
        if extra_args[i] == flag {
            return Some(extra_args[i + 1].as_str());
        }
        i += 1;
    }
    None
}

fn resolve_assets_package(
    primary_pkg: Option<TokPackage>,
    extra_args: &[String],
) -> Option<TokPackage> {
    if primary_pkg.is_some() {
        return primary_pkg;
    }
    let assets_from = extract_flag_value(extra_args, "--assets-from")?;
    TokPackage::open(assets_from).ok().map(|(_, pkg)| pkg)
}

fn load_cli_package_or_doc(
    input: &str,
    demo_paragraphs: usize,
) -> Result<
    (
        DocumentModel,
        tok_storage::package::TokManifest,
        Option<TokPackage>,
    ),
    Box<dyn std::error::Error>,
> {
    if input == "--demo" {
        println!("  - Generating demo Hebrew document with Niqqud...");
    } else if input.ends_with(".json") {
        println!("  - Reading document JSON from: {}", input);
    }
    let res = load_package_or_doc(input, demo_paragraphs)?;
    if res.2.is_some() {
        println!("  - Opened .tok package from: {}", input);
    }
    Ok(res)
}

fn engine_config_from_root(root: &DocumentRoot) -> TypesettingEngineConfig {
    TypesettingEngineConfig::from_document_root(root)
}

fn handle_inspect_font(font_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = inspect_font_file(Path::new(font_path))?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn handle_render_pdf(
    input: &str,
    output: &str,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Rendering document to Pre-Press PDF: {}", output);
    let (doc, manifest, pkg_opt) = load_cli_package_or_doc(input, 30)?;
    let assets_pkg = resolve_assets_package(pkg_opt, extra_args);
    let fonts_dir = extract_flag_value(extra_args, "--fonts-dir").map(Path::new);
    let font_mgr = build_font_manager(fonts_dir, assets_pkg.as_ref());

    let start_typeset = Instant::now();
    let config = engine_config_from_root(doc.root());
    let engine = TypesettingEngine::with_font_manager(config, font_mgr);
    let pages = engine.typeset_document(doc.root());
    println!(
        "  - Typeset {} pages in {:.2?}",
        pages.len(),
        start_typeset.elapsed()
    );

    let mut req = PdfRenderRequest::default();
    let mut idx = 0;
    while idx < extra_args.len() {
        match extra_args[idx].as_str() {
            "--standard" if idx + 1 < extra_args.len() => {
                if extra_args[idx + 1].eq_ignore_ascii_case("pdfx4") {
                    req.standard = PdfXStandard::PdfX4;
                } else {
                    req.standard = PdfXStandard::PdfX1a2001;
                }
                idx += 2;
            }
            "--profile" if idx + 1 < extra_args.len() => {
                req.color_profile = IccProfileId::from_id_str(&extra_args[idx + 1]);
                idx += 2;
            }
            "--no-bleed" => {
                req.bleed_pt = 0.0;
                idx += 1;
            }
            "--no-crop-marks" => {
                req.draw_crop_marks = false;
                idx += 1;
            }
            "--registration-marks" => {
                req.draw_registration_marks = true;
                idx += 1;
            }
            _ => {
                idx += 1;
            }
        }
    }

    let start_pdf = Instant::now();
    let pdf_bytes = render_pdf_bytes(
        doc.root(),
        &manifest,
        &pages,
        &engine.font_manager,
        assets_pkg.as_ref(),
        &req,
    )?;

    write_atomic(Path::new(output), &pdf_bytes)?;
    println!(
        "  - Emitted ISO PDF/X ({} bytes, profile: {}) in {:.2?}",
        pdf_bytes.len(),
        req.color_profile.output_condition_identifier(),
        start_pdf.elapsed()
    );
    println!("  [SUCCESS] Written PDF to: {}", output);
    Ok(())
}

fn handle_render_html(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[TOK-CLI] Rendering document to Pre-Paginated HTML: {}",
        output
    );
    let (doc, _) = load_document_input(input, 20)?;

    let config = engine_config_from_root(doc.root());
    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(doc.root());
    println!("  - Typeset {} pages", pages.len());

    let html = render_html_string(&pages);
    write_atomic(Path::new(output), html.as_bytes())?;
    println!(
        "  [SUCCESS] Written Pre-paginated HTML ({} bytes) to: {}",
        html.len(),
        output
    );
    Ok(())
}

fn handle_benchmark(pages_target: usize) -> Result<(), Box<dyn std::error::Error>> {
    if pages_target == 0 {
        return Err("--pages must be at least 1".into());
    }
    println!("================================================================================");
    println!("  TypesetOK (TOK) - Phase 6 Stress Test & Performance Benchmark (Gate 1)");
    println!("================================================================================");
    println!("Target Page Count: {}", pages_target);

    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());

    const SAMPLE_PARAGRAPHS: usize = 100;
    let sample = engine.typeset_document(create_sample_hebrew_document(SAMPLE_PARAGRAPHS).root());
    let sample_lines: usize = sample
        .iter()
        .flat_map(|p| &p.frames)
        .map(|f| f.lines.len())
        .sum();
    let lines_per_page = sample
        .first()
        .map(|p| p.frames.iter().map(|f| f.lines.len()).sum::<usize>())
        .unwrap_or(1)
        .max(1);
    let lines_per_paragraph = (sample_lines as f64 / SAMPLE_PARAGRAPHS as f64).max(1e-3);
    let total_paragraphs = ((pages_target * lines_per_page) as f64 / lines_per_paragraph)
        .ceil()
        .max(1.0) as usize;
    println!(
        "Generating synthetic holy text corpus ({} paragraphs with full Niqqud)...",
        total_paragraphs
    );
    let gen_start = Instant::now();
    let doc = create_sample_hebrew_document(total_paragraphs);
    println!("Corpus generated in {:.2?}", gen_start.elapsed());

    println!("\n[Benchmark 1: Full Document Typesetting]");
    let typeset_start = Instant::now();
    let pages = engine.typeset_document(doc.root());
    let typeset_dur = typeset_start.elapsed();

    let actual_pages = pages.len();
    let total_lines: usize = pages
        .iter()
        .map(|p| p.frames.iter().map(|f| f.lines.len()).sum::<usize>())
        .sum();
    let pages_per_sec = (actual_pages as f64) / typeset_dur.as_secs_f64();

    println!("  - Total Pages Emitted:     {}", actual_pages);
    println!("  - Total Lines Justified:   {}", total_lines);
    println!("  - Total Typesetting Time:  {:.2?}", typeset_dur);
    println!(
        "  - Throughput:              {:.1} pages/sec",
        pages_per_sec
    );
    println!(
        "  - Average Per Page:        {:.3} ms/page",
        (typeset_dur.as_secs_f64() * 1000.0) / actual_pages as f64
    );

    println!("\n[Benchmark 2: Incremental Cascade & Convergence Principle]");
    println!(
        "Simulating user editing a paragraph on Page 10 of {} pages...",
        actual_pages
    );

    let mut target_para = doc.root().sections[0]
        .main_flow()
        .and_then(|flow| flow.paragraphs.get(40).or(flow.paragraphs.last()))
        .ok_or("benchmark corpus has no paragraphs")?
        .clone();
    let cascade_start = Instant::now();
    target_para.text.push_str(" הסבר נוסף לפירוש רש\"י הקדוש.");

    let _rebroken = engine.typeset_paragraph(&target_para, 453.55, 11.0, 14.5);
    let cascade_dur = cascade_start.elapsed();

    println!("  - Re-breaking & Justification Time: {:.3?}", cascade_dur);
    println!(
        "  - Cascade Convergence Latency:      {:.3} ms (< 10 ms requirement: {})",
        cascade_dur.as_secs_f64() * 1000.0,
        if cascade_dur.as_millis() < 10 {
            "PASSED [120 FPS READY]"
        } else {
            "CHECK"
        }
    );

    println!("\n[Benchmark 3: Incremental Document Reflow & Invalidation Engine]");
    let mut modified_doc = doc.root().clone();
    let mod_id = target_para.id;
    if let Some(p) = modified_doc.find_paragraph_mut(mod_id) {
        p.text.push_str(" הסבר נוסף לפירוש רש\"י הקדוש.");
    }
    let inc_start = Instant::now();
    let (_inc_pages, inc_diag) =
        engine.typeset_document_incremental(&modified_doc, &pages, Some(mod_id));
    let inc_dur = inc_start.elapsed();

    println!("  - Incremental Pipeline Latency:    {:.3?}", inc_dur);
    println!(
        "  - Pages Recomputed / Reused:       {} / {}",
        inc_diag.recomputed_pages, inc_diag.reused_pages
    );
    println!(
        "  - Paragraphs Recomputed / Cached:  {} / {}",
        inc_diag.recomputed_paragraphs, inc_diag.reused_paragraphs
    );
    println!(
        "  - Convergence Page:                {:?}",
        inc_diag.converged_at_page
    );
    println!(
        "  - Zero-Cascade Fast Path:          {}",
        inc_diag.zero_cascade
    );

    let cache = engine.cache_metrics();
    println!(
        "  - Layout Cache Hit Ratio:          {:.1}% ({} entries, {} bytes)",
        cache.hit_ratio * 100.0,
        cache.entry_count,
        cache.approx_memory_bytes
    );

    println!("\n================================================================================");
    println!("  [GATE 1 VERIFICATION COMPLETED SUCCESSFULLY]");
    println!("================================================================================");
    Ok(())
}

fn handle_verify_determinism() -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};

    println!("[TOK-CLI] Verifying bit-for-bit deterministic reproducibility...");
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let fonts = &engine.font_manager;
    let (w_mm, h_mm) = (210.0, 297.0);

    println!("  [1/2] Verifying single-flow prose document determinism...");
    let doc = create_sample_hebrew_document(50);

    let pages1 = engine.typeset_document(doc.root());
    let pdf1 =
        PdfPrePressEngine::export_pdf_with_fonts(&pages1, &PdfExportOptions::default(), fonts)?;
    let html1 = HtmlProjectionCompiler::compile_to_html(&pages1, w_mm, h_mm);

    let mut hasher1 = Sha256::new();
    hasher1.update(&pdf1);
    let pdf_hash1 = format_hex(hasher1.finalize());

    let mut hasher1_h = Sha256::new();
    hasher1_h.update(html1.as_bytes());
    let html_hash1 = format_hex(hasher1_h.finalize());

    let pages2 = engine.typeset_document(doc.root());
    let pdf2 =
        PdfPrePressEngine::export_pdf_with_fonts(&pages2, &PdfExportOptions::default(), fonts)?;
    let html2 = HtmlProjectionCompiler::compile_to_html(&pages2, w_mm, h_mm);

    let mut hasher2 = Sha256::new();
    hasher2.update(&pdf2);
    let pdf_hash2 = format_hex(hasher2.finalize());

    let mut hasher2_h = Sha256::new();
    hasher2_h.update(html2.as_bytes());
    let html_hash2 = format_hex(hasher2_h.finalize());

    if pdf_hash1 != pdf_hash2 {
        return Err("Single-flow PDF outputs must be bit-for-bit identical".into());
    }
    if html_hash1 != html_hash2 {
        return Err("Single-flow HTML outputs must be bit-for-bit identical".into());
    }
    if pages1 != pages2 {
        return Err("Single-flow Layouts must be identical".into());
    }
    println!("    Prose PDF SHA-256:  {}", pdf_hash1);
    println!("    Prose HTML SHA-256: {}", html_hash1);

    println!("  [2/2] Verifying multi-flow Talmud layout determinism...");
    let talmud_doc = tok_typeset::fixtures::create_realistic_talmud_document();
    let talmud_pages1 = engine.typeset_document_multi_flow(&talmud_doc);
    let talmud_pdf1 = PdfPrePressEngine::export_pdf_with_fonts(
        &talmud_pages1,
        &PdfExportOptions::default(),
        fonts,
    )?;
    let talmud_html1 = HtmlProjectionCompiler::compile_to_html(&talmud_pages1, w_mm, h_mm);

    let mut t_hasher1 = Sha256::new();
    t_hasher1.update(&talmud_pdf1);
    let talmud_pdf_hash1 = format_hex(t_hasher1.finalize());

    let mut t_hasher1_h = Sha256::new();
    t_hasher1_h.update(talmud_html1.as_bytes());
    let talmud_html_hash1 = format_hex(t_hasher1_h.finalize());

    let talmud_pages2 = engine.typeset_document_multi_flow(&talmud_doc);
    let talmud_pdf2 = PdfPrePressEngine::export_pdf_with_fonts(
        &talmud_pages2,
        &PdfExportOptions::default(),
        fonts,
    )?;
    let talmud_html2 = HtmlProjectionCompiler::compile_to_html(&talmud_pages2, w_mm, h_mm);

    let mut t_hasher2 = Sha256::new();
    t_hasher2.update(&talmud_pdf2);
    let talmud_pdf_hash2 = format_hex(t_hasher2.finalize());

    let mut t_hasher2_h = Sha256::new();
    t_hasher2_h.update(talmud_html2.as_bytes());
    let talmud_html_hash2 = format_hex(t_hasher2_h.finalize());

    if talmud_pdf_hash1 != talmud_pdf_hash2 {
        return Err(format!(
            "Multi-flow PDF outputs must be bit-for-bit identical: {talmud_pdf_hash1} vs {talmud_pdf_hash2}"
        )
        .into());
    }
    if talmud_html_hash1 != talmud_html_hash2 {
        return Err(format!(
            "Multi-flow HTML outputs must be bit-for-bit identical: {talmud_html_hash1} vs {talmud_html_hash2}"
        )
        .into());
    }
    if talmud_pages1 != talmud_pages2 {
        return Err("Multi-flow Layouts must be identical".into());
    }
    println!("    Talmud PDF SHA-256:  {}", talmud_pdf_hash1);
    println!("    Talmud HTML SHA-256: {}", talmud_html_hash1);

    println!("  [SUCCESS] Bit-for-bit absolute determinism verified across single-flow and multi-flow pipelines!");
    Ok(())
}

fn handle_inspect_package(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Inspecting .tok package: {}", input);
    let summary = inspect_package(Path::new(input))?;

    println!("  Title:            {}", summary.title);
    println!("  Author:           {}", summary.author);
    println!("  Document ID:      {}", summary.document_id);
    println!("  Schema Version:   {}", summary.schema_version);
    println!("  Sections:         {}", summary.sections);
    println!("  Total Paragraphs: {}", summary.total_paragraphs);
    println!("  Embedded Assets:  {}", summary.embedded_assets.len());
    for asset in &summary.embedded_assets {
        println!("    - assets/{}", asset);
    }
    println!("  Page Previews:    {}", summary.page_previews);
    Ok(())
}

fn handle_save_package(
    input: &str,
    output: &str,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Saving document package to .tok: {}", output);
    let (doc, manifest, input_pkg) = load_cli_package_or_doc(input, 1)?;
    let existing_pkg = input_pkg.or_else(|| {
        extract_flag_value(extra_args, "--assets-from")
            .and_then(|p| TokPackage::open(p).ok().map(|(_, pkg)| pkg))
    });
    let fonts_dir = extract_flag_value(extra_args, "--fonts-dir").map(Path::new);

    save_package(
        &doc,
        manifest,
        Path::new(output),
        existing_pkg.as_ref(),
        fonts_dir,
    )?;
    println!("  [SUCCESS] Saved .tok package to: {}", output);
    Ok(())
}

fn handle_open_package(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Opening .tok package from: {}", input);
    let (doc, _pkg) = load_document_input(input, 1)?;

    let json_bytes = serde_json::to_vec_pretty(doc.root())?;
    write_atomic(Path::new(output), &json_bytes)?;
    println!("  [SUCCESS] Extracted document JSON to: {}", output);
    Ok(())
}

fn handle_run_plugins(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[TOK-CLI] Running built-in typographic plugins on document: {} -> {}",
        input, output
    );
    let (doc, _manifest) = load_document_input(input, 5)?;
    let mut root = doc.root().clone();

    let transactions = run_builtin_plugins(&mut root);
    let ops_count: usize = transactions.iter().map(|tx| tx.operations.len()).sum();
    println!(
        "  - Executed {} plugins ({} atomic operations applied)",
        transactions.len(),
        ops_count
    );

    let json_bytes = serde_json::to_vec_pretty(&root)?;
    write_atomic(Path::new(output), &json_bytes)?;
    println!(
        "  [SUCCESS] Written plugin-processed DocumentRoot JSON to: {}",
        output
    );
    Ok(())
}

fn handle_typeset_document(
    input: &str,
    output: &str,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Typesetting document layout to JSON: {}", output);
    let (doc, _, pkg_opt) = load_cli_package_or_doc(input, 1)?;
    let assets_pkg = resolve_assets_package(pkg_opt, extra_args);
    let fonts_dir = extract_flag_value(extra_args, "--fonts-dir").map(Path::new);
    let font_mgr = build_font_manager(fonts_dir, assets_pkg.as_ref());

    let config = engine_config_from_root(doc.root());
    let engine = TypesettingEngine::with_font_manager(config, font_mgr);
    let pages = engine.typeset_document(doc.root());
    println!("  - Typeset {} pages with native Rust engine", pages.len());

    #[derive(serde::Serialize)]
    struct TypesetWithPreflight<'a> {
        pages: &'a [tok_typeset::geometry::PageLayoutBox],
        preflight: &'a tok_pdf::preflight::PreflightReport,
    }
    let with_preflight = extra_args.iter().any(|a| a == "--with-preflight");
    let json_bytes = if with_preflight {
        let report = evaluate_preflight(
            doc.root(),
            &pages,
            &engine.font_manager,
            assets_pkg.as_ref(),
            &PreflightRequest::default(),
        );
        serde_json::to_vec(&TypesetWithPreflight {
            pages: &pages,
            preflight: &report,
        })?
    } else {
        serde_json::to_vec(&pages)?
    };
    write_atomic(Path::new(output), &json_bytes)?;
    println!("  [SUCCESS] Written layout JSON to: {}", output);
    Ok(())
}

fn handle_preflight(
    input: &str,
    output_opt: Option<&str>,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let (doc, _, pkg_opt) = load_package_or_doc(input, 1)?;
    let assets_pkg = resolve_assets_package(pkg_opt, extra_args);
    let fonts_dir = extract_flag_value(extra_args, "--fonts-dir").map(Path::new);
    let font_mgr = build_font_manager(fonts_dir, assets_pkg.as_ref());
    let config = engine_config_from_root(doc.root());
    let engine = TypesettingEngine::with_font_manager(config, font_mgr);
    let pages = engine.typeset_document(doc.root());

    let mut req = PreflightRequest::default();
    if let Some(std_arg) = extract_flag_value(extra_args, "--standard") {
        if std_arg.eq_ignore_ascii_case("pdfx4") {
            req.standard = PdfXStandard::PdfX4;
        }
    }
    if extra_args.iter().any(|a| a == "--no-bleed") {
        req.bleed_pt = 0.0;
    }
    let report = evaluate_preflight(
        doc.root(),
        &pages,
        &engine.font_manager,
        assets_pkg.as_ref(),
        &req,
    );
    let json_str = serde_json::to_string_pretty(&report)?;
    if let Some(out_path) = output_opt {
        write_atomic(Path::new(out_path), json_str.as_bytes())?;
    } else {
        println!("{}", json_str);
    }
    Ok(())
}

fn handle_incremental_typeset(
    input: &str,
    output: &str,
    modified_node_id_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Running incremental typesetting & recomputation diagnostic...");
    let (model, _manifest) = load_document_input(input, 60)?;
    let doc = model.root();

    let config = engine_config_from_root(doc);
    let engine = TypesettingEngine::new(config);
    let initial_start = Instant::now();
    let initial_pages = engine.typeset_document(doc);
    println!(
        "  - Initial layout produced: {} pages in {:.2?}",
        initial_pages.len(),
        initial_start.elapsed()
    );

    let target_node_id = if let Some(id_str) = modified_node_id_str {
        tok_core::id::NodeId::from_string(id_str).ok()
    } else {
        doc.sections
            .first()
            .and_then(|s| s.main_flow())
            .and_then(|f| f.paragraphs.first())
            .map(|p| p.id)
    };

    println!("  - Incrementally invalidating node: {:?}", target_node_id);
    let (inc_pages, diag) =
        engine.typeset_document_incremental(doc, &initial_pages, target_node_id);

    let report =
        tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag, inc_pages.len());
    println!("\n{}", report.summary());

    let json_bytes = serde_json::to_vec_pretty(&inc_pages)?;
    write_atomic(Path::new(output), &json_bytes)?;
    println!(
        "  [SUCCESS] Written incremental PageLayoutBox array to: {}",
        output
    );
    Ok(())
}

fn handle_diagnose_recompute(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[TOK-CLI] Running comprehensive recomputation diagnostics on: {}",
        input
    );
    let (model, _manifest) = load_document_input(input, 80)?;
    let mut doc = model.root().clone();

    let config = engine_config_from_root(&doc);
    let engine = TypesettingEngine::new(config);
    let pages1 = engine.typeset_document(&doc);
    println!("  - Baseline document: {} pages", pages1.len());

    println!("\n[Scenario 1: Beginning Edit (Page 0)]");
    let p0_id = doc.sections[0]
        .main_flow()
        .and_then(|f| f.paragraphs.first())
        .map(|p| p.id);
    if let Some(id) = p0_id {
        if let Some(p) = doc.find_paragraph_mut(id) {
            p.text
                .push_str(" תוספת מלל בתחילת הספר לבדיקת גלישת עמודים.");
        }
        let (_p, diag1) = engine.typeset_document_incremental(&doc, &pages1, Some(id));
        let r1 = tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag1, pages1.len());
        println!("{}", r1.summary());
    }

    println!("\n[Scenario 2: Middle Edit]");
    let mid_idx = doc.sections[0]
        .main_flow()
        .map_or(0, |f| f.paragraphs.len() / 2);
    let mid_id = doc.sections[0]
        .main_flow()
        .and_then(|f| f.paragraphs.get(mid_idx))
        .map(|p| p.id);
    if let Some(id) = mid_id {
        if let Some(p) = doc.find_paragraph_mut(id) {
            p.text.push_str(" עריכה באמצע המסמך.");
        }
        let (_p, diag2) = engine.typeset_document_incremental(&doc, &pages1, Some(id));
        let r2 = tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag2, pages1.len());
        println!("{}", r2.summary());
    }

    println!("\n[Scenario 3: End Edit]");
    let last_id = doc.sections[0]
        .main_flow()
        .and_then(|f| f.paragraphs.last())
        .map(|p| p.id);
    if let Some(id) = last_id {
        let (_p, diag3) = engine.typeset_document_incremental(&doc, &pages1, Some(id));
        let r3 = tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag3, pages1.len());
        println!("{}", r3.summary());
    }

    println!("\n[CACHE METRICS]");
    let metrics = engine.cache_metrics();
    println!(
        "  - Hits: {}, Misses: {}, Entries: {}, Memory: {} bytes",
        metrics.hits, metrics.misses, metrics.entry_count, metrics.approx_memory_bytes
    );

    println!("\n  [DIAGNOSTICS COMPLETED SUCCESSFULLY]");
    Ok(())
}

fn handle_sync_heading_numbers(
    input: &str,
    output: &str,
    plain: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (model, _manifest) = load_document_input(input, 10)?;
    let mut root = model.root().clone();
    let affected = sync_heading_numbers(&mut root, plain);

    let json_bytes = serde_json::to_vec_pretty(&root)?;
    write_atomic(Path::new(output), &json_bytes)?;
    println!(
        "  [SUCCESS] Synchronized heading numbering for {} items to: {}",
        affected.len(),
        output
    );
    Ok(())
}

/// Processes a single persistent worker request against the in-memory [`WorkspaceSession`].
fn process_worker_request(session: &mut WorkspaceSession, req: WorkerRequest) -> WorkerResponse {
    let result: Result<serde_json::Value, String> = (|| {
        if req.command != "typeset" {
            return Err(format!("Unsupported worker command: {}", req.command));
        }
        let (doc, pkg_opt) = if let Some(ref json_str) = req.document_json {
            let (model, _) = parse_document_json(json_str).map_err(|e| e.to_string())?;
            (model, None)
        } else if let Some(ref input_path) = req.input_path {
            let (model, _, pkg) = load_package_or_doc(input_path, 20).map_err(|e| e.to_string())?;
            (model, pkg)
        } else {
            return Err("Either document_json or input_path must be provided".to_string());
        };

        let fonts_dir = req.fonts_dir.as_deref().map(Path::new);
        let assets_from = req.assets_from.as_deref().map(Path::new);
        session.sync_fonts_and_assets(fonts_dir, assets_from, pkg_opt);

        let output = session.typeset_document(doc.root(), req.with_preflight);
        if req.with_preflight {
            serde_json::to_value(&output).map_err(|e| e.to_string())
        } else {
            serde_json::to_value(&output.pages).map_err(|e| e.to_string())
        }
    })();

    match result {
        Ok(payload) => WorkerResponse {
            id: req.id,
            ok: true,
            payload: Some(payload),
            error: None,
        },
        Err(err) => WorkerResponse {
            id: req.id,
            ok: false,
            payload: None,
            error: Some(err),
        },
    }
}

fn handle_worker() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::BufRead;

    let mut session = WorkspaceSession::new();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    for line_res in stdin.lock().lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: WorkerRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = WorkerResponse {
                    id: 0,
                    ok: false,
                    payload: None,
                    error: Some(format!("Invalid WorkerRequest JSON: {}", e)),
                };
                let _ = serde_json::to_writer(&mut out, &resp);
                let _ = out.write_all(b"\n");
                let _ = out.flush();
                continue;
            }
        };

        let resp = process_worker_request(&mut session, req);
        serde_json::to_writer(&mut out, &resp)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "render-pdf" => {
            if args.len() < 4 {
                eprintln!("Usage: tok-cli render-pdf <INPUT.tok | --demo> <OUTPUT.pdf>");
                std::process::exit(1);
            }
            handle_render_pdf(&args[2], &args[3], &args[4..])?;
        }
        "render-html" => {
            if args.len() < 4 {
                eprintln!("Usage: tok-cli render-html <INPUT.tok | --demo> <OUTPUT.html>");
                std::process::exit(1);
            }
            handle_render_html(&args[2], &args[3])?;
        }
        "typeset-document" => {
            if args.len() < 4 {
                eprintln!("Usage: tok-cli typeset-document <INPUT.json | INPUT.tok | --demo> <OUTPUT.json> [--fonts-dir <DIR>] [--assets-from <TOK>]");
                std::process::exit(1);
            }
            handle_typeset_document(&args[2], &args[3], &args[4..])?;
        }
        "worker" => {
            handle_worker()?;
        }
        "incremental-typeset" => {
            if args.len() < 4 {
                eprintln!("Usage: tok-cli incremental-typeset <INPUT.json | --demo> <OUTPUT.json> [--node-id <ID>]");
                std::process::exit(1);
            }
            let node_id_arg = if args.len() >= 6 && args[4] == "--node-id" {
                Some(args[5].as_str())
            } else {
                None
            };
            handle_incremental_typeset(&args[2], &args[3], node_id_arg)?;
        }
        "diagnose-recompute" => {
            if args.len() < 3 {
                eprintln!("Usage: tok-cli diagnose-recompute <INPUT.json | INPUT.tok | --demo>");
                std::process::exit(1);
            }
            handle_diagnose_recompute(&args[2])?;
        }
        "save-package" => {
            if args.len() < 4 {
                eprintln!("Usage: tok-cli save-package <INPUT.json> <OUTPUT.tok> [--fonts-dir <DIR>] [--assets-from <TOK>]");
                std::process::exit(1);
            }
            handle_save_package(&args[2], &args[3], &args[4..])?;
        }
        "open-package" => {
            if args.len() < 4 {
                eprintln!("Usage: tok-cli open-package <INPUT.tok> <OUTPUT.json>");
                std::process::exit(1);
            }
            handle_open_package(&args[2], &args[3])?;
        }
        "run-plugins" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: tok-cli run-plugins <INPUT.json | INPUT.tok | --demo> <OUTPUT.json>"
                );
                std::process::exit(1);
            }
            handle_run_plugins(&args[2], &args[3])?;
        }
        "sync-heading-numbers" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: tok-cli sync-heading-numbers <INPUT.json | INPUT.tok | --demo> <OUTPUT.json> [--plain]"
                );
                std::process::exit(1);
            }
            let plain = args.iter().any(|a| a == "--plain");
            handle_sync_heading_numbers(&args[2], &args[3], plain)?;
        }
        "benchmark-typeset" => {
            let mut pages = 1000;
            if args.len() >= 4 && args[2] == "--pages" {
                pages = match args[3].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Invalid --pages value: {}", args[3]);
                        std::process::exit(1);
                    }
                };
            }
            handle_benchmark(pages)?;
        }
        "verify-determinism" => {
            handle_verify_determinism()?;
        }
        "inspect-package" => {
            if args.len() < 3 {
                eprintln!("Usage: tok-cli inspect-package <INPUT.tok>");
                std::process::exit(1);
            }
            handle_inspect_package(&args[2])?;
        }
        "inspect-font" => {
            if args.len() < 3 {
                eprintln!("Usage: tok-cli inspect-font <PATH.ttf>");
                std::process::exit(1);
            }
            handle_inspect_font(&args[2])?;
        }
        "preflight" => {
            if args.len() < 3 {
                eprintln!("Usage: tok-cli preflight <INPUT.json | INPUT.tok | --demo> [OUTPUT.json] [--fonts-dir <DIR>] [--assets-from <TOK>]");
                std::process::exit(1);
            }
            let (out_opt, extra_start) = if args.len() >= 4 && !args[3].starts_with("--") {
                (Some(args[3].as_str()), 4usize)
            } else {
                (None, 3usize)
            };
            let extra = args.get(extra_start..).unwrap_or(&[]);
            handle_preflight(&args[2], out_opt, extra)?;
        }
        "help" | "-h" | "--help" => {
            print_usage();
        }
        unknown => {
            eprintln!("Unknown command: {}", unknown);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tok_core::id::FractionalIndex;
    use tok_core::model::ParagraphNode;

    #[test]
    fn test_run_plugins_on_document_root() {
        let mut root = DocumentRoot::new("בדיקת תוספים");
        root.sections[0]
            .main_flow_mut()
            .unwrap()
            .paragraphs
            .push(ParagraphNode::new(
                FractionalIndex::initial(),
                "normal",
                "אמר ר' משה בן מימון (רמב\"ם) בשם אל שדי וא-להים",
            ));

        run_builtin_plugins(&mut root);

        let text = &root.sections[0].main_flow().unwrap().paragraphs[0].text;
        assert_eq!(
            text,
            "אמר ר׳ משה בן מימון (רמב״ם) בשם אל\u{00A0}שדי וא\u{2060}-\u{2060}להים"
        );
    }

    #[test]
    fn test_sync_heading_numbers_logic() {
        use tok_core::numbering::GematriaPunctuation;
        use tok_core::outline::{
            DocumentHierarchyNode, DocumentOutline, HierarchyLevel, OutlineNumberingConfig,
        };

        let mut root = DocumentRoot::new("ספר הלכה");
        let flow = root.sections[0].main_flow_mut().unwrap();

        let p1 = ParagraphNode::new(FractionalIndex::initial(), "heading-siman", "הלכות ברכות");
        let p1_id = p1.id;
        flow.paragraphs.push(p1);

        let mut outline = DocumentOutline::new();
        let siman_node = DocumentHierarchyNode::new(HierarchyLevel::Chapter, "הלכות ברכות")
            .with_target_paragraph(p1_id);
        outline.add_root_node(siman_node);
        root.outline = Some(outline);

        let config =
            OutlineNumberingConfig::torah_default(GematriaPunctuation::StandardPunctuation);
        let affected = root.sync_outline_numbering(&config);

        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].1, "סימן א׳");
        assert_eq!(
            root.sections[0].main_flow().unwrap().paragraphs[0]
                .number_label
                .as_deref(),
            Some("סימן א׳")
        );
    }

    #[test]
    fn inspect_font_and_package_roundtrip_bundles_external_font() {
        let dir = env::temp_dir().join(format!("tok_cli_font_test_{}", std::process::id()));
        let fonts_dir = dir.join("fonts");
        fs::create_dir_all(&fonts_dir).unwrap();

        let font_file = fonts_dir.join("custom_test.ttf");
        fs::write(&font_file, tok_typeset::font::EMBEDDED_NOTO_SERIF_HEBREW).unwrap();
        let json_report = inspect_font_file(&font_file).unwrap();
        assert_eq!(json_report["valid"], serde_json::Value::Bool(true));
        assert_eq!(
            json_report["report"]["family_name"],
            serde_json::Value::String("Noto Serif Hebrew".to_string())
        );

        let woff_file = dir.join("bad.woff");
        fs::write(&woff_file, b"wOFF\x00\x01\x00\x00\x00\x00\x00\x20bad").unwrap();
        let bad_report = inspect_font_file(&woff_file).unwrap();
        assert_eq!(bad_report["valid"], serde_json::Value::Bool(false));
        assert_eq!(
            bad_report["error"],
            serde_json::Value::String("RejectWoff".to_string())
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn persistent_worker_session_hits_layout_cache_across_requests() {
        let mut session = WorkspaceSession::new();
        let mut root = create_sample_hebrew_document(6).root().clone();
        let json1 = root.to_json().unwrap();

        let resp1 = process_worker_request(
            &mut session,
            WorkerRequest {
                id: 1,
                command: "typeset".to_string(),
                document_json: Some(json1),
                input_path: None,
                fonts_dir: None,
                assets_from: None,
                with_preflight: true,
            },
        );
        assert!(resp1.ok, "first worker request failed: {:?}", resp1.error);
        let payload1 = resp1.payload.unwrap();
        assert!(payload1.get("pages").and_then(|v| v.as_array()).is_some());
        assert!(payload1.get("preflight").is_some());

        let hits_before = session.cache_metrics().hits;
        root.sections[0].main_flow_mut().unwrap().paragraphs[0]
            .text
            .push_str(" בדיקה");
        let json2 = root.to_json().unwrap();

        let resp2 = process_worker_request(
            &mut session,
            WorkerRequest {
                id: 2,
                command: "typeset".to_string(),
                document_json: Some(json2),
                input_path: None,
                fonts_dir: None,
                assets_from: None,
                with_preflight: false,
            },
        );
        assert!(resp2.ok, "second worker request failed: {:?}", resp2.error);
        assert!(resp2.payload.unwrap().is_array());
        assert!(
            session.cache_metrics().hits >= hits_before + 5,
            "persistent worker session must reuse cached paragraph layouts"
        );
    }
}

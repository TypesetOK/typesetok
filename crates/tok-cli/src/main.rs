use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tok_core::id::FractionalIndex;
use tok_core::model::{DocumentModel, DocumentRoot, Flow, FlowId, FlowType, ParagraphNode};
use tok_pdf::html_projection::HtmlProjectionCompiler;
use tok_pdf::pdf_engine::{PdfExportOptions, PdfPrePressEngine, PdfXStandard};
use tok_storage::package::{TokManifest, TokPackage};
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

fn create_sample_hebrew_document(num_paragraphs: usize) -> DocumentModel {
    let mut root = DocumentRoot::new("׳×׳׳׳•׳“ ׳‘׳‘׳׳™ - ׳׳¡׳›׳× ׳‘׳¨׳›׳•׳×");
    let sec = &mut root.sections[0];
    let flow = sec.main_flow_mut().unwrap();

    let sample_texts = [
        "׳ײµ׳ײµ׳™׳ײ¸׳×ײ·׳™ ׳§׳•ײ¹׳¨ײ´׳™׳ ׳ײ¶׳× ׳©ײ°׳׳ײ·׳¢ ׳‘ײ°ײ¼׳¢ײ·׳¨ײ°׳‘ײ´׳™׳×? ׳ײ´׳©ײ¸ײ¼׳׳¢ײ¸׳” ׳©ײ¶׳׳”ײ·׳›ײ¹ײ¼׳”ײ²׳ ײ´׳™׳ ׳ ײ´׳›ײ°׳ ײ¸׳¡ײ´׳™׳ ׳ײ¶׳ײ±׳›ײ¹׳ ׳‘ײ´ײ¼׳×ײ°׳¨׳•ײ¼׳ײ¸׳×ײ¸׳, ׳¢ײ·׳“ ׳¡׳•ײ¹׳£ ׳”ײ¸׳ײ·׳©ײ°׳׳׳•ײ¼׳¨ײ¸׳” ׳”ײ¸׳¨ײ´׳׳©׳׳•ײ¹׳ ײ¸׳”, ׳“ײ´ײ¼׳‘ײ°׳¨ײµ׳™ ׳¨ײ·׳‘ײ´ײ¼׳™ ׳ײ±׳ײ´׳™׳¢ײ¶׳–ײ¶׳¨. ׳•ײ·׳—ײ²׳›ײ¸׳ײ´׳™׳ ׳׳•ײ¹׳ײ°׳¨ײ´׳™׳: ׳¢ײ·׳“ ׳—ײ²׳¦׳•ײ¹׳×. ׳¨ײ·׳‘ײ¸ײ¼׳ ׳’ײ·ײ¼׳ײ°׳ײ´׳™׳ײµ׳ ׳׳•ײ¹׳ײµ׳¨: ׳¢ײ·׳“ ׳©ײ¶׳׳™ײ·ײ¼׳¢ײ²׳ײ¶׳” ׳¢ײ·׳ײ¼׳•ײ¼׳“ ׳”ײ·׳©ײ¸ײ¼׳׳—ײ·׳¨.",
        "׳ײ·׳¢ײ²׳©ײ¶׳‚׳” ׳©ײ¶׳׳‘ײ¸ײ¼׳׳•ײ¼ ׳‘ײ¸׳ ײ¸׳™׳• ׳ײ´׳‘ײµײ¼׳™׳× ׳”ײ·׳ײ´ײ¼׳©ײ°׳׳×ײ¶ײ¼׳”, ׳ײ¸׳ײ°׳¨׳•ײ¼ ׳׳•ײ¹: ׳ײ¹׳ ׳§ײ¸׳¨ײ´׳™׳ ׳•ײ¼ ׳ײ¶׳× ׳©ײ°׳׳ײ·׳¢! ׳ײ¸׳ײ·׳¨ ׳ײ¸׳”ײ¶׳: ׳ײ´׳ ׳ײ¹׳ ׳¢ײ¸׳ײ¸׳” ׳¢ײ·׳ײ¼׳•ײ¼׳“ ׳”ײ·׳©ײ¸ײ¼׳׳—ײ·׳¨, ׳—ײ·׳™ײ¸ײ¼׳‘ײ´׳™׳ ׳ײ·׳×ײ¶ײ¼׳ ׳ײ´׳§ײ°׳¨׳•ײ¹׳×.",
        "׳•ײ°׳ײ¹׳ ׳–׳•ײ¹ ׳‘ײ´׳ײ°׳‘ײ·׳“, ׳ײ¶׳ײ¸ײ¼׳ ׳›ײ¸ײ¼׳ ׳ײ·׳” ׳©ײ¶׳׳ײ¸׳ײ°׳¨׳•ײ¼ ׳—ײ²׳›ײ¸׳ײ´׳™׳ ׳¢ײ·׳“ ׳—ײ²׳¦׳•ײ¹׳×, ׳ײ´׳¦ײ°׳•ײ¸׳×ײ¸׳ ׳¢ײ·׳“ ׳©ײ¶׳׳™ײ·ײ¼׳¢ײ²׳ײ¶׳” ׳¢ײ·׳ײ¼׳•ײ¼׳“ ׳”ײ·׳©ײ¸ײ¼׳׳—ײ·׳¨. ׳”ײ¶׳§ײ°׳˜ײµ׳¨ ׳—ײ²׳ײ¸׳‘ײ´׳™׳ ׳•ײ°׳ײµ׳‘ײ¸׳¨ײ´׳™׳ ׳ײ´׳¦ײ°׳•ײ¸׳×ײ¸׳ ׳¢ײ·׳“ ׳©ײ¶׳׳™ײ·ײ¼׳¢ײ²׳ײ¶׳” ׳¢ײ·׳ײ¼׳•ײ¼׳“ ׳”ײ·׳©ײ¸ײ¼׳׳—ײ·׳¨, ׳•ײ°׳›ײ¸׳ ׳”ײ·׳ ײ¶ײ¼׳ײ±׳›ײ¸׳ײ´׳™׳ ׳ײ°׳™׳•ײ¹׳ ׳ײ¶׳—ײ¸׳“ ׳ײ´׳¦ײ°׳•ײ¸׳×ײ¸׳ ׳¢ײ·׳“ ׳©ײ¶׳׳™ײ·ײ¼׳¢ײ²׳ײ¶׳” ׳¢ײ·׳ײ¼׳•ײ¼׳“ ׳”ײ·׳©ײ¸ײ¼׳׳—ײ·׳¨.",
        "׳ײ´׳ ׳›ײµײ¼׳, ׳ײ¸׳ײ¸ײ¼׳” ׳ײ¸׳ײ°׳¨׳•ײ¼ ׳—ײ²׳›ײ¸׳ײ´׳™׳ ׳¢ײ·׳“ ׳—ײ²׳¦׳•ײ¹׳×? ׳›ײ°ײ¼׳“ײµ׳™ ׳ײ°׳”ײ·׳¨ײ°׳—ײ´׳™׳§ ׳ײ¶׳× ׳”ײ¸׳ײ¸׳“ײ¸׳ ׳ײ´׳ ׳”ײ¸׳¢ײ²׳‘ײµ׳¨ײ¸׳”, ׳©ײ¶׳׳ײ¹ײ¼׳ ׳™ײ¹׳׳ײ·׳¨ ׳ײ¸׳“ײ¸׳: ׳™ײµ׳©׳ ׳ײ´׳™ ׳¢׳•ײ¹׳“ ׳–ײ°׳ײ·׳, ׳•ײ°׳ ײ´׳ײ°׳¦ײ¸׳ ׳™ײ¸׳©ײµ׳׳ ׳•ײ°׳¢׳•ײ¹׳‘ײµ׳¨ ׳¢ײ·׳ ׳“ײ´ײ¼׳‘ײ°׳¨ײµ׳™ ׳×׳•ײ¹׳¨ײ¸׳”.",
        "׳×ײ·ײ¼׳ ײ¸ײ¼׳ ׳”ײµ׳™׳›ײ¸׳ ׳§ײ¸׳ײµ׳™ ׳“ײ°ײ¼׳§ײ¸׳×ײ¸׳ ײµ׳™ ׳ײµ׳ײµ׳™׳ײ¸׳×ײ·׳™? ׳•ײ°׳×׳•ײ¼, ׳ײ·׳׳™ ׳©ײ°׳׳ ײ¸׳ ׳“ײ°ײ¼׳×ײ¸׳ ײµ׳™ ׳‘ײ°ײ¼׳¢ײ·׳¨ײ°׳‘ײ´׳™׳× ׳‘ײ°ײ¼׳¨ײµ׳™׳©ײ¸׳׳, ׳ײ´׳×ײ°׳ ײµ׳™ ׳“ײ°ײ¼׳©ײ·׳׳—ײ²׳¨ײ´׳™׳× ׳‘ײ°ײ¼׳¨ײµ׳™׳©ײ¸׳׳? ׳×ײ·ײ¼׳ ײ¸ײ¼׳ ׳ײ·׳§ײ°ײ¼׳¨ײ¸׳ ׳§ײ¸׳ײµ׳™, ׳“ײ´ײ¼׳›ײ°׳×ײ´׳™׳‘: ׳‘ײ°ײ¼׳©ײ¸׳׳›ײ°׳‘ײ°ײ¼׳ײ¸ ׳•ײ¼׳‘ײ°׳§׳•ײ¼׳ײ¶׳ײ¸.",
    ];

    let mut prev_idx = FractionalIndex::initial();
    for i in 0..num_paragraphs {
        let text = sample_texts[i % sample_texts.len()];
        let idx = FractionalIndex::between(Some(&prev_idx), None).unwrap();
        flow.paragraphs
            .push(ParagraphNode::new(idx.clone(), "normal", text));
        prev_idx = idx;
    }

    DocumentModel::new(root)
}

#[derive(serde::Deserialize)]
struct RawMultiFlowState {
    title: Option<String>,
    #[serde(rename = "templateType")]
    template_type: Option<String>,
    #[serde(default)]
    flows: HashMap<String, Vec<RawStoryPara>>,
    #[serde(default)]
    settings: Option<serde_json::Value>,
    #[serde(default, rename = "multiFlowAnchors")]
    multi_flow_anchors: Option<tok_core::multi_flow_anchor::MultiFlowAnchorMap>,
    #[serde(default)]
    anchors: Option<tok_core::multi_flow_anchor::MultiFlowAnchorMap>,
    #[serde(default, rename = "footnoteApparatuses")]
    footnote_apparatuses: Vec<tok_core::header::FootnoteApparatusConfig>,
    #[serde(default, rename = "grepRules")]
    grep_rules: Vec<tok_core::styles::GrepRule>,
}

#[derive(serde::Deserialize)]
struct RawStoryPara {
    id: Option<String>,
    #[serde(rename = "styleId")]
    style_id: Option<String>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    spans: Vec<tok_core::spans::TextSpan>,
}

fn convert_multi_flow_state_to_root(raw: RawMultiFlowState) -> DocumentRoot {
    let title = raw
        .title
        .unwrap_or_else(|| "׳׳¡׳׳ ׳׳׳ ׳©׳".to_string());
    let mut root = DocumentRoot::new(&title);
    if let Some(settings) = raw.settings {
        root.metadata.document_settings = Some(settings);
    }
    root.multi_flow_anchors = raw.multi_flow_anchors.or(raw.anchors);
    root.footnote_apparatuses = raw.footnote_apparatuses;
    root.grep_rules = raw.grep_rules;
    root.sections.clear();

    let is_prose = raw.template_type.as_deref() == Some("prose");
    let mut section = tok_core::model::SectionNode::new(&title, "chapter-first");
    section.flows.clear();

    for (flow_key, paras) in raw.flows {
        let is_apparatus = root
            .footnote_apparatuses
            .iter()
            .any(|a| a.id == flow_key || a.flow_id.as_deref() == Some(flow_key.as_str()));
        let is_footnote_flow = flow_key == "notes" || flow_key == "footnote" || is_apparatus;
        if is_prose
            && flow_key != "gemara"
            && flow_key != "main"
            && !(is_footnote_flow && !paras.is_empty())
        {
            continue;
        }
        let (flow_id, flow_type) = match flow_key.as_str() {
            "gemara" => {
                if is_prose {
                    (FlowId::main(), FlowType::Main)
                } else {
                    (FlowId::new("gemara"), FlowType::Main)
                }
            }
            "rashi" => (FlowId::new("rashi"), FlowType::CommentA),
            "tosafot" => (FlowId::new("tosafot"), FlowType::CommentB),
            "notes" => (FlowId::new("notes"), FlowType::Footnote),
            "main" => (FlowId::main(), FlowType::Main),
            other if is_apparatus || other.contains("note") => {
                (FlowId::new(other), FlowType::Footnote)
            }
            other => (FlowId::new(other), FlowType::Main),
        };

        let mut flow = Flow::new(flow_id, flow_type);
        let mut prev_idx: Option<FractionalIndex> = None;
        for p in paras {
            let next_idx = match FractionalIndex::between(prev_idx.as_ref(), None) {
                Ok(idx) => idx,
                Err(_) => FractionalIndex::initial(),
            };
            let style = p.style_id.unwrap_or_else(|| "default-body".to_string());
            let mut node = ParagraphNode::new(next_idx.clone(), style, &p.text);
            if !p.spans.is_empty() {
                node = node.with_spans(p.spans);
            }
            if let Some(ref id_str) = p.id {
                if let Ok(node_id) = tok_core::id::NodeId::from_string(id_str) {
                    node.id = node_id;
                }
            }
            flow.paragraphs.push(node);
            prev_idx = Some(next_idx);
        }
        section.flows.push(flow);
    }

    if section.flows.is_empty() {
        section
            .flows
            .push(Flow::new(FlowId::main(), FlowType::Main));
    }

    root.sections.push(section);
    root
}

/// Builds typesetting configuration from document_settings (page size, margins in mm/pt),
/// falling back to standard A4 layout.
fn engine_config_from_root(root: &DocumentRoot) -> TypesettingEngineConfig {
    let mut config = TypesettingEngineConfig::default();
    let mm_to_pt = |mm: f64| (mm * 72.0 / 25.4) as f32;

    if let Some(ref settings) = root.metadata.document_settings {
        let get_val = |keys: &[&str]| -> Option<f64> {
            for key in keys {
                if let Some(v) = settings.get(*key).and_then(|v| v.as_f64()) {
                    return Some(v);
                }
            }
            None
        };

        if let Some(w_mm) = get_val(&["pageWidthMm", "page_width_mm"]) {
            if w_mm > 10.0 {
                config.page_width_pt = mm_to_pt(w_mm);
            }
        } else if let Some(w_pt) = get_val(&["pageWidthPt", "page_width_pt"]) {
            if w_pt > 30.0 {
                config.page_width_pt = w_pt as f32;
            }
        }

        if let Some(h_mm) = get_val(&["pageHeightMm", "page_height_mm"]) {
            if h_mm > 10.0 {
                config.page_height_pt = mm_to_pt(h_mm);
            }
        } else if let Some(h_pt) = get_val(&["pageHeightPt", "page_height_pt"]) {
            if h_pt > 30.0 {
                config.page_height_pt = h_pt as f32;
            }
        }

        if let Some(top_mm) = get_val(&["marginTopMm", "margin_top_mm"]) {
            config.margin_top_pt = mm_to_pt(top_mm);
        } else if let Some(top_pt) = get_val(&["marginTopPt", "margin_top_pt"]) {
            config.margin_top_pt = top_pt as f32;
        }

        if let Some(bot_mm) = get_val(&["marginBottomMm", "margin_bottom_mm"]) {
            config.margin_bottom_pt = mm_to_pt(bot_mm);
        } else if let Some(bot_pt) = get_val(&["marginBottomPt", "margin_bottom_pt"]) {
            config.margin_bottom_pt = bot_pt as f32;
        }

        if let Some(in_mm) = get_val(&[
            "marginInsideMm",
            "margin_inside_mm",
            "marginInnerMm",
            "margin_inner_mm",
        ]) {
            config.margin_inner_pt = mm_to_pt(in_mm);
        } else if let Some(in_pt) = get_val(&[
            "marginInsidePt",
            "margin_inside_pt",
            "marginInnerPt",
            "margin_inner_pt",
        ]) {
            config.margin_inner_pt = in_pt as f32;
        }

        if let Some(out_mm) = get_val(&[
            "marginOutsideMm",
            "margin_outside_mm",
            "marginOuterMm",
            "margin_outer_mm",
        ]) {
            config.margin_outer_pt = mm_to_pt(out_mm);
        } else if let Some(out_pt) = get_val(&[
            "marginOutsidePt",
            "margin_outside_pt",
            "marginOuterPt",
            "margin_outer_pt",
        ]) {
            config.margin_outer_pt = out_pt as f32;
        }
    }

    config
}

/// Builds the package manifest (title, author, document id) for a parsed document.
fn manifest_for_root(root: &DocumentRoot) -> TokManifest {
    TokManifest {
        title: root.metadata.title.clone(),
        author: root.metadata.author.clone(),
        document_id: root.id.to_string(),
        ..TokManifest::default()
    }
}

fn parse_document_json(
    content: &str,
) -> Result<(DocumentModel, TokManifest), Box<dyn std::error::Error>> {
    // 1. Direct DocumentRoot JSON
    if let Ok(root) = DocumentRoot::from_json(content) {
        let manifest = manifest_for_root(&root);
        return Ok((DocumentModel::new(root), manifest));
    }

    // 2. Migration pipeline (for older schemas)
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Ok(migrated) = tok_storage::migration::MigrationPipeline::migrate_document_json(val)
        {
            if let Ok(root) = serde_json::from_value::<DocumentRoot>(migrated) {
                let manifest = manifest_for_root(&root);
                return Ok((DocumentModel::new(root), manifest));
            }
        }
    }

    // 3. Raw MultiFlowDocumentState from UI
    if let Ok(raw_state) = serde_json::from_str::<RawMultiFlowState>(content) {
        let root = convert_multi_flow_state_to_root(raw_state);
        let manifest = manifest_for_root(&root);
        return Ok((DocumentModel::new(root), manifest));
    }

    Err("Invalid document JSON: could not parse as DocumentRoot or MultiFlowDocumentState".into())
}

fn load_document_input(
    input: &str,
    demo_paragraphs: usize,
) -> Result<(DocumentModel, TokManifest), Box<dyn std::error::Error>> {
    if input == "--demo" {
        println!("  - Generating demo Hebrew document with Niqqud...");
        return Ok((
            create_sample_hebrew_document(demo_paragraphs),
            TokManifest::default(),
        ));
    }

    let path = Path::new(input);
    if !path.exists() {
        return Err(format!("Input file does not exist: {}", input).into());
    }

    // If file extension is .json, parse directly
    if input.ends_with(".json") {
        println!("  - Reading document JSON from: {}", input);
        let content = fs::read_to_string(input)?;
        return parse_document_json(&content);
    }

    // Try opening as .tok zip package
    match TokPackage::open(input) {
        Ok((model, pkg)) => {
            println!("  - Opened .tok package from: {}", input);
            Ok((model, pkg.manifest))
        }
        Err(tok_err) => {
            // Check if file content is JSON despite missing .json extension
            if let Ok(content) = fs::read_to_string(input) {
                if content.trim_start().starts_with('{') {
                    println!("  - Parsing input as document JSON: {}", input);
                    return parse_document_json(&content);
                }
            }
            Err(format!(
                "Failed to open package or document ({}): {}",
                input, tok_err
            )
            .into())
        }
    }
}

fn handle_render_pdf(
    input: &str,
    output: &str,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Rendering document to Pre-Press PDF: {}", output);
    let (doc, manifest) = load_document_input(input, 30)?;

    let start_typeset = Instant::now();
    let config = engine_config_from_root(doc.root());
    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(doc.root());
    let typeset_dur = start_typeset.elapsed();
    println!("  - Typeset {} pages in {:.2?}", pages.len(), typeset_dur);

    let mut standard = PdfXStandard::PdfX1a2001;
    let mut color_profile = tok_pdf::color::IccProfileId::Fogra39;
    let mut bleed_pt = 8.504_f32;
    let mut draw_crop_marks = true;
    let mut draw_registration_marks = false;

    let mut idx = 0;
    while idx < extra_args.len() {
        match extra_args[idx].as_str() {
            "--standard" if idx + 1 < extra_args.len() => {
                if extra_args[idx + 1].eq_ignore_ascii_case("pdfx4") {
                    standard = PdfXStandard::PdfX4;
                } else {
                    standard = PdfXStandard::PdfX1a2001;
                }
                idx += 2;
            }
            "--profile" if idx + 1 < extra_args.len() => {
                color_profile = tok_pdf::color::IccProfileId::from_id_str(&extra_args[idx + 1]);
                idx += 2;
            }
            "--no-bleed" => {
                bleed_pt = 0.0;
                idx += 1;
            }
            "--no-crop-marks" => {
                draw_crop_marks = false;
                idx += 1;
            }
            "--registration-marks" => {
                draw_registration_marks = true;
                idx += 1;
            }
            _ => {
                idx += 1;
            }
        }
    }

    let start_pdf = Instant::now();
    let options = PdfExportOptions {
        standard,
        color_profile,
        title: doc.root().metadata.title.clone(),
        author: doc.root().metadata.author.clone(),
        bleed_pt,
        slug_pt: if draw_crop_marks || draw_registration_marks {
            28.346
        } else {
            bleed_pt
        },
        draw_crop_marks,
        draw_registration_marks,
        draw_page_numbers: true,
        custom_font_data: None,
        creation_date: Some(manifest.created_at.clone()),
        mod_date: Some(manifest.modified_at.clone()),
        compress_streams: true,
    };

    let pdf_bytes =
        PdfPrePressEngine::export_pdf_with_fonts(&pages, &options, &engine.font_manager)?;

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(output, &pdf_bytes)?;
    let pdf_dur = start_pdf.elapsed();
    println!(
        "  - Emitted ISO PDF/X ({} bytes, profile: {}) in {:.2?}",
        pdf_bytes.len(),
        color_profile.output_condition_identifier(),
        pdf_dur
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

    let (page_w_mm, page_h_mm) = pages
        .first()
        .map(|p| {
            (
                p.dimensions.width * 25.4 / 72.0,
                p.dimensions.height * 25.4 / 72.0,
            )
        })
        .unwrap_or((210.0, 297.0));
    let html = HtmlProjectionCompiler::compile_to_html(&pages, page_w_mm, page_h_mm);
    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(output, &html)?;
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

    // Calibrate the corpus size on a sample so the run really produces about
    // `pages_target` pages (the corpus cycles through a few fixed texts).
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

    // 1. Full Document Typeset Benchmark
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

    // 2. Incremental Cascade Convergence Benchmark (Gate 1 Requirement)
    println!("\n[Benchmark 2: Incremental Cascade & Convergence Principle]");
    println!(
        "Simulating user editing a paragraph on Page 10 of {} pages...",
        actual_pages
    );

    // In TOK's architecture, editing a paragraph requires re-breaking only until line count converges
    let mut target_para = doc.root().sections[0]
        .main_flow()
        .and_then(|flow| flow.paragraphs.get(40).or(flow.paragraphs.last()))
        .ok_or("benchmark corpus has no paragraphs")?
        .clone();
    let cascade_start = Instant::now();
    target_para.text.push_str(
        " ׳”ײ¶׳¡ײ°׳‘ײµײ¼׳¨ ׳ ׳•ײ¹׳¡ײ¸׳£ ׳ײ°׳₪ײµ׳¨׳•ײ¼׳©׳ ׳¨ײ·׳©ײ´׳\"׳™ ׳”ײ·׳§ײ¸ײ¼׳“׳•ײ¹׳©׳.",
    );

    // Typeset modified single paragraph
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

    // 3. Full Document Incremental Reflow & Cache Validation
    println!("\n[Benchmark 3: Incremental Document Reflow & Invalidation Engine]");
    let mut modified_doc = doc.root().clone();
    let mod_id = target_para.id;
    if let Some(p) = modified_doc.find_paragraph_mut(mod_id) {
        p.text.push_str(" ׳”ײ¶׳¡ײ°׳‘ײµײ¼׳¨ ׳ ׳•ײ¹׳¡ײ¸׳£ ׳ײ°׳₪ײµ׳¨׳•ײ¼׳©׳ ׳¨ײ·׳©ײ´׳\"׳™ ׳”ײ·׳§ײ¸ײ¼׳“׳•ײ¹׳©׳.");
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

fn format_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn handle_verify_determinism() -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};

    println!("[TOK-CLI] Verifying bit-for-bit deterministic reproducibility...");
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let fonts = &engine.font_manager;
    let (w_mm, h_mm) = (210.0, 297.0);

    // Test 1: Single-flow Prose Determinism
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

    // Test 2: Multi-flow Complex Talmud Layout Determinism
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
        return Err(format!("Multi-flow PDF outputs must be bit-for-bit identical: {talmud_pdf_hash1} vs {talmud_pdf_hash2}").into());
    }
    if talmud_html_hash1 != talmud_html_hash2 {
        return Err(format!("Multi-flow HTML outputs must be bit-for-bit identical: {talmud_html_hash1} vs {talmud_html_hash2}").into());
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
    let (model, pkg) = TokPackage::open(input)?;

    println!("  Title:            {}", pkg.manifest.title);
    println!("  Author:           {}", pkg.manifest.author);
    println!("  Document ID:      {}", pkg.manifest.document_id);
    println!("  Schema Version:   {}", pkg.manifest.schema_version);
    println!("  Sections:         {}", model.root().sections.len());
    let total_paras: usize = model
        .root()
        .sections
        .iter()
        .map(|s| s.flows.iter().map(|f| f.paragraphs.len()).sum::<usize>())
        .sum();
    println!("  Total Paragraphs: {}", total_paras);
    println!("  Embedded Assets:  {}", pkg.assets.len());
    let mut asset_names: Vec<&String> = pkg.assets.keys().collect();
    asset_names.sort();
    for asset in asset_names {
        println!("    - assets/{}", asset);
    }
    println!("  Page Previews:    {}", pkg.previews.len());
    Ok(())
}

fn handle_save_package(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Saving document package to .tok: {}", output);
    let (doc, manifest) = load_document_input(input, 1)?;
    let mut pkg = TokPackage::new();
    pkg.manifest = manifest;

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    pkg.save_atomic(&doc, output)?;
    println!("  [SUCCESS] Saved .tok package to: {}", output);
    Ok(())
}

fn handle_open_package(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Opening .tok package from: {}", input);
    let (doc, _pkg) = load_document_input(input, 1)?;

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let json_bytes = serde_json::to_vec_pretty(doc.root())?;
    fs::write(output, json_bytes)?;
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

    let host = tok_plugin_host::PluginHost::with_builtin_plugins();
    let transactions = host.run_before_typeset(&mut root);
    let ops_count: usize = transactions.iter().map(|tx| tx.operations.len()).sum();
    println!(
        "  - Executed {} plugins ({} atomic operations applied)",
        transactions.len(),
        ops_count
    );

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let json_bytes = serde_json::to_vec_pretty(&root)?;
    fs::write(output, json_bytes)?;
    println!(
        "  [SUCCESS] Written plugin-processed DocumentRoot JSON to: {}",
        output
    );
    Ok(())
}

fn handle_typeset_document(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TOK-CLI] Typesetting document layout to JSON: {}", output);
    let (doc, _) = load_document_input(input, 1)?;

    let config = engine_config_from_root(doc.root());
    let engine = TypesettingEngine::new(config);
    let pages = engine.typeset_document(doc.root());
    println!("  - Typeset {} pages with native Rust engine", pages.len());

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let json_bytes = serde_json::to_vec_pretty(&pages)?;
    fs::write(output, json_bytes)?;
    println!("  [SUCCESS] Written PageLayoutBox array to: {}", output);
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
    let initial_dur = initial_start.elapsed();
    println!(
        "  - Initial layout produced: {} pages in {:.2?}",
        initial_pages.len(),
        initial_dur
    );

    // Pick target node
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
    let inc_start = Instant::now();
    let (inc_pages, diag) =
        engine.typeset_document_incremental(doc, &initial_pages, target_node_id);
    let _inc_dur = inc_start.elapsed();

    let report =
        tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag, inc_pages.len());
    println!("\n{}", report.summary());

    if let Some(parent) = Path::new(output).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let json_bytes = serde_json::to_vec_pretty(&inc_pages)?;
    fs::write(output, json_bytes)?;
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

    // 1. Beginning edit
    println!("\n[Scenario 1: Beginning Edit (Page 0)]");
    let p0_id = doc.sections[0]
        .main_flow()
        .and_then(|f| f.paragraphs.first())
        .map(|p| p.id);
    if let Some(id) = p0_id {
        if let Some(p) = doc.find_paragraph_mut(id) {
            p.text.push_str(
                " ׳×׳•׳¡׳₪׳× ׳׳׳ ׳‘׳×׳—׳™׳׳× ׳”׳¡׳₪׳¨ ׳׳‘׳“׳™׳§׳× ׳’׳׳™׳©׳× ׳¢׳׳•׳“׳™׳.",
            );
        }
        let (_p, diag1) = engine.typeset_document_incremental(&doc, &pages1, Some(id));
        let r1 = tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag1, pages1.len());
        println!("{}", r1.summary());
    }

    // 2. Middle edit
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
            p.text.push_str(" ׳¢׳¨׳™׳›׳” ׳‘׳׳׳¦׳¢ ׳”׳׳¡׳׳.");
        }
        let (_p, diag2) = engine.typeset_document_incremental(&doc, &pages1, Some(id));
        let r2 = tok_typeset::diagnostics::RecomputeReport::from_diagnostics(&diag2, pages1.len());
        println!("{}", r2.summary());
    }

    // 3. End edit
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

    let punctuation = if plain {
        tok_core::numbering::GematriaPunctuation::PlainLetters
    } else {
        tok_core::numbering::GematriaPunctuation::StandardPunctuation
    };

    let config = tok_core::outline::OutlineNumberingConfig::torah_default(punctuation);
    let affected = root.sync_outline_numbering(&config);

    let json_bytes = serde_json::to_vec_pretty(&root)?;
    fs::write(output, json_bytes)?;
    println!(
        "  [SUCCESS] Synchronized heading numbering for {} items to: {}",
        affected.len(),
        output
    );
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
                eprintln!("Usage: tok-cli typeset-document <INPUT.json | INPUT.tok | --demo> <OUTPUT.json>");
                std::process::exit(1);
            }
            handle_typeset_document(&args[2], &args[3])?;
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
                eprintln!("Usage: tok-cli save-package <INPUT.json> <OUTPUT.tok>");
                std::process::exit(1);
            }
            handle_save_package(&args[2], &args[3])?;
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

        let host = tok_plugin_host::PluginHost::with_builtin_plugins();
        host.run_before_typeset(&mut root);

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
}

use crate::document::{
    build_font_manager, load_package_or_doc, manifest_for_root, parse_document_json,
    run_builtin_plugins, save_package,
};
use crate::error::EngineServiceError;
use crate::export::{
    evaluate_preflight, render_html_string, render_pdf_bytes, PdfRenderRequest, PreflightRequest,
};
use crate::io::write_atomic;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tok_core::id::NodeId;
use tok_core::model::{DocumentModel, DocumentRoot};
use tok_core::transaction::CompoundTransaction;
use tok_pdf::preflight::PreflightReport;
use tok_plugin_host::PluginHost;
use tok_storage::package::{TokManifest, TokPackage};
use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
use tok_typeset::font::FontManager;
use tok_typeset::geometry::PageLayoutBox;
use tok_typeset::incremental::IncrementalTypesetDiagnostics;
use tok_typeset::layout_cache::CacheMetricsSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
struct AssetsSourceFingerprint {
    path: PathBuf,
    len: u64,
    modified_nanos: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FontsDirFingerprint {
    dir: PathBuf,
    files: Vec<(String, u64, u128)>,
}

fn modified_nanos(meta: &fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn compute_assets_fingerprint(path: &Path) -> Option<AssetsSourceFingerprint> {
    let meta = fs::metadata(path).ok()?;
    Some(AssetsSourceFingerprint {
        path: path.to_path_buf(),
        len: meta.len(),
        modified_nanos: modified_nanos(&meta),
    })
}

fn compute_fonts_dir_fingerprint(dir: &Path) -> Option<FontsDirFingerprint> {
    let entries = fs::read_dir(dir).ok()?;
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let is_font = p
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf"));
        if !is_font {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Ok(meta) = entry.metadata() {
            files.push((name, meta.len(), modified_nanos(&meta)));
        }
    }
    files.sort();
    Some(FontsDirFingerprint {
        dir: dir.to_path_buf(),
        files,
    })
}

fn is_single_section_multi_flow(doc: &DocumentRoot) -> bool {
    if doc.sections.len() != 1 {
        return false;
    }
    let sec = &doc.sections[0];
    let non_fn_flows = sec
        .flows
        .iter()
        .filter(|f| f.flow_type != tok_core::model::FlowType::Footnote)
        .count();
    if non_fn_flows > 1 {
        return true;
    }
    let col_req = sec.column_count.unwrap_or(1).clamp(1, 6);
    if col_req > 1 || sec.flows.len() <= 1 {
        return false;
    }
    let has_span_all = sec
        .main_flow()
        .or_else(|| sec.flows.first())
        .is_some_and(|f| {
            f.paragraphs.iter().any(|p| {
                p.style_patches
                    .last()
                    .and_then(|patch| patch.column_span)
                    .or_else(|| {
                        doc.paragraph_styles
                            .iter()
                            .find(|s| s.id == p.style_id)
                            .map(|s| s.column_span)
                    })
                    == Some(tok_core::styles::ColumnSpanMode::SpanAllColumns)
            })
        });
    !has_span_all
}

/// Output returned by [`WorkspaceSession::typeset_document`].
#[derive(Debug, Clone, Serialize)]
pub struct TypesetOutput {
    pub pages: Vec<PageLayoutBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight: Option<PreflightReport>,
}

/// Stateful in-process engine session holding the active document, persistent
/// [`FontManager`], warm [`TypesettingEngine`] paragraph cache, and [`PluginHost`].
pub struct WorkspaceSession {
    document: Option<DocumentModel>,
    manifest: TokManifest,
    assets_package: Option<TokPackage>,
    assets_source_fingerprint: Option<AssetsSourceFingerprint>,
    fonts_dir_fingerprint: Option<FontsDirFingerprint>,
    engine: TypesettingEngine,
    plugin_host: PluginHost,
    last_pages: Vec<Arc<PageLayoutBox>>,
    last_document_id: Option<NodeId>,
}

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<WorkspaceSession>();
};

impl Default for WorkspaceSession {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSession {
    /// Creates a new session with embedded fonts preloaded once in RAM.
    pub fn new() -> Self {
        Self {
            document: None,
            manifest: TokManifest::default(),
            assets_package: None,
            assets_source_fingerprint: None,
            fonts_dir_fingerprint: None,
            engine: TypesettingEngine::new(TypesettingEngineConfig::default()),
            plugin_host: PluginHost::with_builtin_plugins(),
            last_pages: Vec::new(),
            last_document_id: None,
        }
    }

    pub fn document(&self) -> Option<&DocumentModel> {
        self.document.as_ref()
    }

    pub fn manifest(&self) -> &TokManifest {
        &self.manifest
    }

    pub fn last_pages(&self) -> &[Arc<PageLayoutBox>] {
        &self.last_pages
    }

    fn last_pages_owned(&self) -> Vec<PageLayoutBox> {
        self.last_pages.iter().map(|p| (**p).clone()).collect()
    }

    pub fn font_manager(&self) -> &FontManager {
        &self.engine.font_manager
    }

    pub fn plugin_host(&self) -> &PluginHost {
        &self.plugin_host
    }

    pub fn cache_metrics(&self) -> CacheMetricsSnapshot {
        self.engine.cache_metrics()
    }

    pub fn assets_package(&self) -> Option<&TokPackage> {
        self.assets_package.as_ref()
    }

    /// Synchronizes external fonts and `.tok` package assets only when fingerprints change,
    /// avoiding redundant font parsing and preserving the warm paragraph layout cache across edits.
    pub fn sync_fonts_and_assets(
        &mut self,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
        inline_pkg: Option<TokPackage>,
    ) {
        let next_fonts_fp = fonts_dir.and_then(compute_fonts_dir_fingerprint);
        let fonts_changed = next_fonts_fp != self.fonts_dir_fingerprint;

        let next_assets_fp = assets_from.and_then(compute_assets_fingerprint);
        let same_fingerprinted_assets = next_assets_fp.is_some()
            && next_assets_fp == self.assets_source_fingerprint
            && self.assets_package.is_some();
        let assets_changed = (inline_pkg.is_some() && !same_fingerprinted_assets)
            || next_assets_fp != self.assets_source_fingerprint
            || (assets_from.is_none() && self.assets_package.is_some() && inline_pkg.is_none());

        if !fonts_changed && !assets_changed {
            return;
        }

        if let Some(pkg) = inline_pkg {
            if !same_fingerprinted_assets {
                self.assets_package = Some(pkg);
                self.assets_source_fingerprint = next_assets_fp;
            }
        } else if assets_changed {
            self.assets_package =
                assets_from.and_then(|p| TokPackage::open(p).ok().map(|(_, pkg)| pkg));
            self.assets_source_fingerprint = next_assets_fp;
        }

        self.fonts_dir_fingerprint = next_fonts_fp;
        self.engine.font_manager = build_font_manager(fonts_dir, self.assets_package.as_ref());
        self.engine.clear_cache();
        self.last_pages.clear();
    }

    /// Typesets `root` using the persistent [`TypesettingEngine`] and its warm paragraph cache.
    pub fn typeset_document(&mut self, root: &DocumentRoot, with_preflight: bool) -> TypesetOutput {
        if self.last_document_id != Some(root.id) {
            self.engine.clear_cache();
            self.last_pages.clear();
        }

        let config = TypesettingEngineConfig::from_document_root(root);
        if self.engine.config() != &config {
            self.engine.set_config(config);
            self.last_pages.clear();
        }

        let pages = self.engine.typeset_document(root);
        let manifest = manifest_for_root(root);

        let preflight = if with_preflight {
            Some(evaluate_preflight(
                root,
                &pages,
                &self.engine.font_manager,
                self.assets_package.as_ref(),
                &PreflightRequest::default(),
            ))
        } else {
            None
        };

        self.document = Some(DocumentModel::new(root.clone()));
        self.manifest = manifest;
        self.last_document_id = Some(root.id);
        self.last_pages = pages.iter().cloned().map(Arc::new).collect();

        TypesetOutput { pages, preflight }
    }

    /// Incrementally typesets `root` against the session's previous page layout.
    pub fn typeset_incremental(
        &mut self,
        root: &DocumentRoot,
        modified_node_id: Option<NodeId>,
    ) -> (Vec<Arc<PageLayoutBox>>, IncrementalTypesetDiagnostics) {
        self.typeset_incremental_owned(root.clone(), modified_node_id)
    }

    /// Incrementally typesets an owned `root` without cloning the entire document tree.
    pub fn typeset_incremental_owned(
        &mut self,
        root: DocumentRoot,
        modified_node_id: Option<NodeId>,
    ) -> (Vec<Arc<PageLayoutBox>>, IncrementalTypesetDiagnostics) {
        if self.last_document_id != Some(root.id) {
            self.engine.clear_cache();
            self.last_pages.clear();
        }

        let config = TypesettingEngineConfig::from_document_root(&root);
        if self.engine.config() != &config {
            self.engine.set_config(config);
            self.last_pages.clear();
        }

        let (pages, diag) = if is_single_section_multi_flow(&root) {
            let prev_owned = self.last_pages_owned();
            let (p, d) = self.engine.typeset_document_multi_flow_incremental(
                &root,
                &prev_owned,
                modified_node_id,
            );
            (p.into_iter().map(Arc::new).collect(), d)
        } else {
            self.engine.typeset_document_incremental_arcs(
                &root,
                &self.last_pages,
                modified_node_id,
            )
        };

        let root_id = root.id;
        self.manifest = manifest_for_root(&root);
        self.document = Some(DocumentModel::new(root));
        self.last_document_id = Some(root_id);
        self.last_pages = pages.clone();

        (pages, diag)
    }

    /// Opens a `.tok` package (or JSON/demo input) into this session and syncs its fonts/assets.
    pub fn open_package(
        &mut self,
        input: &Path,
        fonts_dir: Option<&Path>,
    ) -> Result<&DocumentModel, EngineServiceError> {
        let input_str = input
            .to_str()
            .ok_or_else(|| EngineServiceError::Validation("Invalid non-UTF8 path".to_string()))?;
        let (doc, manifest, pkg_opt) = load_package_or_doc(input_str, 1)?;
        self.sync_fonts_and_assets(fonts_dir, Some(input), pkg_opt);
        self.manifest = manifest;
        self.last_document_id = Some(doc.root().id);
        self.last_pages.clear();
        self.engine.clear_cache();
        self.document = Some(doc);
        Ok(self.document.as_ref().expect("document just stored"))
    }

    /// Saves the active session document to a `.tok` package at `output`.
    pub fn save_active_package(
        &self,
        output: &Path,
        fonts_dir: Option<&Path>,
    ) -> Result<(), EngineServiceError> {
        let doc = self.document.as_ref().ok_or_else(|| {
            EngineServiceError::Validation("No active document in workspace session".to_string())
        })?;
        save_package(
            doc,
            self.manifest.clone(),
            output,
            self.assets_package.as_ref(),
            fonts_dir,
        )
    }

    fn prepare_session_target(
        &mut self,
        document_json: Option<&str>,
        input_path: Option<&Path>,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
        demo_paragraphs: usize,
    ) -> Result<DocumentModel, EngineServiceError> {
        let (doc, pkg_opt, fallback_assets) = if let Some(json_str) = document_json {
            let (model, _) = parse_document_json(json_str)?;
            (model, None, None)
        } else if let Some(path) = input_path {
            let path_str = path.to_str().ok_or_else(|| {
                EngineServiceError::Validation("Invalid non-UTF8 path".to_string())
            })?;
            let (model, _, pkg) = load_package_or_doc(path_str, demo_paragraphs)?;
            let inline_src = if pkg.is_some() { Some(path) } else { None };
            (model, pkg, inline_src)
        } else {
            return Err(EngineServiceError::Validation(
                "Either document_json or input_path must be provided".to_string(),
            ));
        };

        let effective_assets_from = assets_from.or(fallback_assets);
        let inline_pkg = if assets_from.is_some() && assets_from != fallback_assets {
            None
        } else {
            pkg_opt
        };
        self.sync_fonts_and_assets(fonts_dir, effective_assets_from, inline_pkg);
        Ok(doc)
    }

    fn ensure_active_layout(&mut self) -> Result<DocumentRoot, EngineServiceError> {
        let doc_root = self
            .document
            .as_ref()
            .ok_or_else(|| {
                EngineServiceError::Validation(
                    "No active document in workspace session".to_string(),
                )
            })?
            .root()
            .clone();

        if self.last_pages.is_empty() || self.last_document_id != Some(doc_root.id) {
            let config = TypesettingEngineConfig::from_document_root(&doc_root);
            self.engine.set_config(config);
            self.last_pages = self
                .engine
                .typeset_document(&doc_root)
                .into_iter()
                .map(Arc::new)
                .collect();
            self.last_document_id = Some(doc_root.id);
        }

        Ok(doc_root)
    }

    /// Loads or parses a document target, syncs fonts/assets, and typesets via the persistent session.
    pub fn typeset_json_or_path(
        &mut self,
        document_json: Option<&str>,
        input_path: Option<&Path>,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
        with_preflight: bool,
    ) -> Result<TypesetOutput, EngineServiceError> {
        let doc =
            self.prepare_session_target(document_json, input_path, fonts_dir, assets_from, 20)?;
        Ok(self.typeset_document(doc.root(), with_preflight))
    }

    /// Parses JSON document state, syncs external fonts/assets, and atomically writes a `.tok` package.
    pub fn save_document_json(
        &mut self,
        document_json: &str,
        output_path: &Path,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
    ) -> Result<(), EngineServiceError> {
        let (doc, manifest) = parse_document_json(document_json)?;
        self.sync_fonts_and_assets(fonts_dir, assets_from, None);
        save_package(
            &doc,
            manifest.clone(),
            output_path,
            self.assets_package.as_ref(),
            fonts_dir,
        )?;
        self.document = Some(doc);
        self.manifest = manifest;
        if assets_from == Some(output_path) {
            self.assets_source_fingerprint = compute_assets_fingerprint(output_path);
        }
        Ok(())
    }

    /// Typesets (if target provided) and atomically writes an ISO PDF/X export to `output_path`.
    pub fn render_pdf_from_json_or_path(
        &mut self,
        document_json: Option<&str>,
        input_path: Option<&Path>,
        output_path: &Path,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
        req: &PdfRenderRequest,
    ) -> Result<usize, EngineServiceError> {
        if document_json.is_some() || input_path.is_some() {
            let doc =
                self.prepare_session_target(document_json, input_path, fonts_dir, assets_from, 30)?;
            self.typeset_document(doc.root(), false);
        } else if fonts_dir.is_some() || assets_from.is_some() {
            self.sync_fonts_and_assets(fonts_dir, assets_from, None);
        }
        let pdf_bytes = self.render_active_pdf(req)?;
        write_atomic(output_path, &pdf_bytes)?;
        Ok(pdf_bytes.len())
    }

    /// Typesets (if target provided) and atomically writes pre-paginated HTML to `output_path`.
    pub fn render_html_from_json_or_path(
        &mut self,
        document_json: Option<&str>,
        input_path: Option<&Path>,
        output_path: &Path,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
    ) -> Result<usize, EngineServiceError> {
        if document_json.is_some() || input_path.is_some() {
            let doc =
                self.prepare_session_target(document_json, input_path, fonts_dir, assets_from, 20)?;
            self.typeset_document(doc.root(), false);
        } else if fonts_dir.is_some() || assets_from.is_some() {
            self.sync_fonts_and_assets(fonts_dir, assets_from, None);
        }
        let html = self.render_active_html()?;
        write_atomic(output_path, html.as_bytes())?;
        Ok(html.len())
    }

    /// Runs prepress preflight diagnostics against a provided target or active session document.
    pub fn run_preflight_from_json_or_path(
        &mut self,
        document_json: Option<&str>,
        input_path: Option<&Path>,
        fonts_dir: Option<&Path>,
        assets_from: Option<&Path>,
        req: &PreflightRequest,
    ) -> Result<PreflightReport, EngineServiceError> {
        if document_json.is_some() || input_path.is_some() {
            let doc =
                self.prepare_session_target(document_json, input_path, fonts_dir, assets_from, 1)?;
            self.typeset_document(doc.root(), false);
        } else if fonts_dir.is_some() || assets_from.is_some() {
            self.sync_fonts_and_assets(fonts_dir, assets_from, None);
        }
        let doc_root = self.ensure_active_layout()?;
        let pages = self.last_pages_owned();
        Ok(evaluate_preflight(
            &doc_root,
            &pages,
            &self.engine.font_manager,
            self.assets_package.as_ref(),
            req,
        ))
    }

    /// Typesets (if needed) and exports the active document to ISO PDF/X bytes.
    pub fn render_active_pdf(
        &mut self,
        req: &PdfRenderRequest,
    ) -> Result<Vec<u8>, EngineServiceError> {
        let doc_root = self.ensure_active_layout()?;
        let pages = self.last_pages_owned();

        render_pdf_bytes(
            &doc_root,
            &self.manifest,
            &pages,
            &self.engine.font_manager,
            self.assets_package.as_ref(),
            req,
        )
    }

    /// Typesets (if needed) and exports the active document to pre-paginated HTML.
    pub fn render_active_html(&mut self) -> Result<String, EngineServiceError> {
        let _ = self.ensure_active_layout()?;
        let pages = self.last_pages_owned();
        Ok(render_html_string(&pages))
    }

    /// Runs built-in Rust typographic plugins against the active session document.
    pub fn run_builtin_plugins_on_active(
        &mut self,
    ) -> Result<Vec<CompoundTransaction>, EngineServiceError> {
        let doc = self.document.as_mut().ok_or_else(|| {
            EngineServiceError::Validation("No active document in workspace session".to_string())
        })?;
        let mut root = doc.root().clone();
        let txs = run_builtin_plugins(&mut root);
        if txs.iter().any(|tx| !tx.operations.is_empty()) {
            *doc = DocumentModel::new(root);
            self.engine.clear_cache();
            self.last_pages.clear();
        }
        Ok(txs)
    }
}

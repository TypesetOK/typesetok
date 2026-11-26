use crate::fonts;
use crate::plugins::{LoadedPlugin, PluginSystemStatus};
use crate::state::{trace_milestone, AppState};
use crate::updater::{
    self, ApplyUpdateResult, DownloadUpdateResult, OfflineUpdateCandidate, UpdateCheckResult,
};
use base64::Engine;
use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tok_engine_service::{write_atomic, PdfRenderRequest, PreflightRequest};
use zip::ZipArchive;

const MAX_DECOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentResponse {
    pub success: bool,
    pub file_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaximizeResponse {
    pub success: bool,
    pub is_maximized: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsMaximizedResponse {
    pub is_maximized: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoResponse {
    pub version: &'static str,
    pub name: &'static str,
    pub repo_url: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypesetResponse {
    pub success: bool,
    pub pages: Value,
    pub preflight: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DialogOptions {
    pub default_path: Option<String>,
    pub filters: Option<Vec<DialogFilter>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentPayload {
    pub document: Value,
    pub file_path: String,
    pub assets_from: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypesetPayload {
    pub document: Option<Value>,
    pub input_path: Option<String>,
    pub assets_from: Option<String>,
    pub with_preflight: Option<bool>,
    /// Export format id (`pdfx1a`, `pdfx4`, `pdf`) the print check is evaluated against.
    pub preflight_standard: Option<String>,
    /// Whether the print check assumes bleed (the export dialog's bleed switch).
    pub preflight_bleed: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderPdfOptions {
    pub format: Option<String>,
    pub color_profile: Option<String>,
    pub bleed: Option<bool>,
    pub crop_marks: Option<bool>,
    pub registration_marks: Option<bool>,
    /// First and last page to export (1-based, inclusive); both missing exports every page.
    pub page_from: Option<usize>,
    pub page_to: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPayload {
    pub input_path: Option<String>,
    pub document: Option<Value>,
    pub output_path: Option<String>,
    pub options: Option<RenderPdfOptions>,
    pub assets_from: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightPayload {
    pub document: Option<Value>,
    pub input_path: Option<String>,
    pub assets_from: Option<String>,
    pub standard: Option<String>,
    pub bleed: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupTimings {
    pub nav_start: Option<f64>,
    pub html_loaded: Option<f64>,
    pub script_start: Option<f64>,
    pub dcl_start: Option<f64>,
    pub ui_built: Option<f64>,
    pub dom_content_loaded: Option<f64>,
    pub fcp: Option<f64>,
}

fn json_value_to_string(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        other => serde_json::to_string(other).map_err(|e| e.to_string()),
    }
}

fn optional_fonts_dir(dir: &Path) -> Option<PathBuf> {
    if dir.is_dir() {
        Some(dir.to_path_buf())
    } else {
        None
    }
}

fn authorize_read_path(state: &AppState, raw: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(raw);
    let paths = state.paths.lock().map_err(|_| "PathPolicy lock poisoned")?;
    paths.assert_authorized(&p)
}

#[tauri::command]
pub fn tok_read_settings(state: State<'_, AppState>) -> Result<BTreeMap<String, String>, String> {
    let guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    Ok(guard.snapshot())
}

#[tauri::command]
pub fn tok_set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<SuccessResponse, String> {
    let mut guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    guard.set(key, value)?;
    Ok(SuccessResponse { success: true })
}

#[tauri::command]
pub fn tok_set_settings(
    state: State<'_, AppState>,
    entries: BTreeMap<String, String>,
) -> Result<SuccessResponse, String> {
    let mut guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    guard.set_many(entries)?;
    Ok(SuccessResponse { success: true })
}

#[tauri::command]
pub fn tok_remove_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<SuccessResponse, String> {
    let mut guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    guard.remove(&key)?;
    Ok(SuccessResponse { success: true })
}

#[tauri::command]
pub fn tok_get_app_info() -> AppInfoResponse {
    AppInfoResponse {
        version: env!("CARGO_PKG_VERSION"),
        name: "TypesetOK",
        repo_url: "https://github.com/tovish/TypesetOK",
    }
}

#[tauri::command]
pub fn tok_get_projects_dir(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state
        .paths
        .lock()
        .map_err(|_| "PathPolicy lock poisoned".to_string())?;
    Ok(guard.projects_dir().to_string_lossy().into_owned())
}

#[tauri::command]
pub fn tok_get_startup_file(state: State<'_, AppState>) -> Option<String> {
    state.startup_file.clone()
}

#[tauri::command]
pub fn tok_open_external(app: AppHandle, url: String) -> Result<(), String> {
    let trimmed = url.trim();
    let allowed = trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("mailto:");
    if !allowed {
        return Err("Blocked unsupported external URL scheme".to_string());
    }
    app.opener()
        .open_url(trimmed, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tok_close_window(window: WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tok_minimize_window(window: WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tok_maximize_window(window: WebviewWindow) -> Result<MaximizeResponse, String> {
    let currently_max = window.is_maximized().map_err(|e| e.to_string())?;
    if currently_max {
        window.unmaximize().map_err(|e| e.to_string())?;
    } else {
        window.maximize().map_err(|e| e.to_string())?;
    }
    let is_maximized = window.is_maximized().unwrap_or(!currently_max);
    Ok(MaximizeResponse {
        success: true,
        is_maximized,
    })
}

#[tauri::command]
pub fn tok_is_maximized(window: WebviewWindow) -> Result<IsMaximizedResponse, String> {
    let is_maximized = window.is_maximized().map_err(|e| e.to_string())?;
    Ok(IsMaximizedResponse { is_maximized })
}

#[tauri::command]
pub fn tok_quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn tok_show_open_dialog(
    app: AppHandle,
    state: State<'_, AppState>,
    options: Option<DialogOptions>,
) -> Result<Option<String>, String> {
    let projects_dir = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| "PathPolicy lock poisoned".to_string())?;
        guard.projects_dir().to_path_buf()
    };

    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut builder = app.dialog().file().set_directory(&projects_dir);
        if let Some(opts) = options {
            if let Some(filters) = opts.filters {
                for f in filters {
                    let ext_refs: Vec<&str> = f.extensions.iter().map(|s| s.as_str()).collect();
                    builder = builder.add_filter(&f.name, &ext_refs);
                }
            }
        }
        builder.blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Some(fp) = picked {
        let path = fp.into_path().map_err(|e| e.to_string())?;
        let mut guard = state
            .paths
            .lock()
            .map_err(|_| "PathPolicy lock poisoned".to_string())?;
        guard.authorize_path(&path);
        Ok(Some(path.to_string_lossy().into_owned()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn tok_show_import_dialog(
    app: AppHandle,
    state: State<'_, AppState>,
    options: Option<DialogOptions>,
) -> Result<Option<String>, String> {
    let has_filters = options
        .as_ref()
        .and_then(|o| o.filters.as_ref())
        .is_some_and(|f| !f.is_empty());
    let options = if has_filters {
        options
    } else {
        let filter = |name: &str, exts: &[&str]| DialogFilter {
            name: name.to_string(),
            extensions: exts.iter().map(|e| e.to_string()).collect(),
        };
        Some(DialogOptions {
            filters: Some(vec![
                filter(
                    "כל המסמכים הנתמכים (*.docx, *.md, *.txt)",
                    &["docx", "md", "txt"],
                ),
                filter("מסמכי Microsoft Word (*.docx)", &["docx"]),
                filter("מסמכי Markdown (*.md)", &["md"]),
                filter("קובצי טקסט פשוט (*.txt)", &["txt"]),
            ]),
            ..options.unwrap_or_default()
        })
    };
    tok_show_open_dialog(app, state, options).await
}

#[tauri::command]
pub async fn tok_show_save_dialog(
    app: AppHandle,
    state: State<'_, AppState>,
    options: Option<DialogOptions>,
) -> Result<Option<String>, String> {
    let projects_dir = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| "PathPolicy lock poisoned".to_string())?;
        guard.projects_dir().to_path_buf()
    };

    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut builder = app.dialog().file().set_directory(&projects_dir);
        if let Some(opts) = options {
            if let Some(def) = opts.default_path {
                let def_path = PathBuf::from(&def);
                if def_path.components().count() <= 1 {
                    builder = builder.set_file_name(&def);
                } else {
                    if let Some(parent) = def_path.parent() {
                        if parent.is_dir() {
                            builder = builder.set_directory(parent);
                        }
                    }
                    if let Some(name) = def_path.file_name().and_then(|n| n.to_str()) {
                        builder = builder.set_file_name(name);
                    }
                }
            }
            if let Some(filters) = opts.filters {
                for f in filters {
                    let ext_refs: Vec<&str> = f.extensions.iter().map(|s| s.as_str()).collect();
                    builder = builder.add_filter(&f.name, &ext_refs);
                }
            }
        }
        builder.blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Some(fp) = picked {
        let path = fp.into_path().map_err(|e| e.to_string())?;
        let mut guard = state
            .paths
            .lock()
            .map_err(|_| "PathPolicy lock poisoned".to_string())?;
        guard.authorize_path(&path);
        Ok(Some(path.to_string_lossy().into_owned()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn tok_open_document(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<Value, String> {
    let resolved = authorize_read_path(&state, &file_path)?;
    if !resolved.exists() {
        return Err(format!("File does not exist: {}", resolved.display()));
    }
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        let model = guard
            .open_package(&resolved, fonts_dir.as_deref())
            .map_err(|e| e.to_string())?;
        serde_json::to_value(model.root()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_save_document(
    state: State<'_, AppState>,
    payload: SaveDocumentPayload,
) -> Result<SaveDocumentResponse, String> {
    let target_path = authorize_read_path(&state, &payload.file_path)?;
    let assets_from = match payload.assets_from.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let doc_json = json_value_to_string(&payload.document)?;
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        guard
            .save_document_json(
                &doc_json,
                &target_path,
                fonts_dir.as_deref(),
                assets_from.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        Ok(SaveDocumentResponse {
            success: true,
            file_path: target_path.to_string_lossy().into_owned(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_typeset_document(
    state: State<'_, AppState>,
    payload: TypesetPayload,
) -> Result<TypesetResponse, String> {
    let input_path = match payload.input_path.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let assets_from = match payload.assets_from.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let doc_json = match payload.document.as_ref() {
        Some(v) => Some(json_value_to_string(v)?),
        None => None,
    };
    if doc_json.is_none() && input_path.is_none() {
        return Err("Either document or inputPath must be provided for typesetting".to_string());
    }
    let with_preflight = payload.with_preflight.unwrap_or(true);
    let pf_req = PreflightRequest::from_ui_options(
        payload.preflight_standard.as_deref(),
        payload.preflight_bleed,
    );
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        guard.set_preflight_request(pf_req);
        let out = guard
            .typeset_json_or_path(
                doc_json.as_deref(),
                input_path.as_deref(),
                fonts_dir.as_deref(),
                assets_from.as_deref(),
                with_preflight,
            )
            .map_err(|e| e.to_string())?;
        let pages = serde_json::to_value(out.pages).map_err(|e| e.to_string())?;
        let preflight = out
            .preflight
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;
        Ok(TypesetResponse {
            success: true,
            pages,
            preflight,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_render_pdf(
    state: State<'_, AppState>,
    payload: RenderPayload,
) -> Result<String, String> {
    let output_raw = payload
        .output_path
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "outputPath must be provided".to_string())?;
    let output_path = authorize_read_path(&state, output_raw)?;
    let input_path = match payload.input_path.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let assets_from = match payload.assets_from.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let doc_json = match payload.document.as_ref() {
        Some(v) => Some(json_value_to_string(v)?),
        None => None,
    };
    let opts = payload.options.unwrap_or_default();
    let req = PdfRenderRequest::from_ui_options(
        opts.format.as_deref(),
        opts.color_profile.as_deref(),
        opts.bleed,
        opts.crop_marks,
        opts.registration_marks,
    )
    .with_page_range(opts.page_from, opts.page_to);
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        guard
            .render_pdf_from_json_or_path(
                doc_json.as_deref(),
                input_path.as_deref(),
                &output_path,
                fonts_dir.as_deref(),
                assets_from.as_deref(),
                &req,
            )
            .map_err(|e| e.to_string())?;
        Ok(output_path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Renders the document to PDF in memory and returns it as base64, for the pre-export preview.
/// Nothing is written to disk, so no output path is needed.
#[tauri::command]
pub async fn tok_render_pdf_preview(
    state: State<'_, AppState>,
    payload: RenderPayload,
) -> Result<String, String> {
    let input_path = match payload.input_path.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let assets_from = match payload.assets_from.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let doc_json = match payload.document.as_ref() {
        Some(v) => Some(json_value_to_string(v)?),
        None => None,
    };
    let opts = payload.options.unwrap_or_default();
    let req = PdfRenderRequest::from_ui_options(
        opts.format.as_deref(),
        opts.color_profile.as_deref(),
        opts.bleed,
        opts.crop_marks,
        opts.registration_marks,
    )
    .with_page_range(opts.page_from, opts.page_to);
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        let bytes = guard
            .render_pdf_bytes_from_json_or_path(
                doc_json.as_deref(),
                input_path.as_deref(),
                fonts_dir.as_deref(),
                assets_from.as_deref(),
                &req,
            )
            .map_err(|e| e.to_string())?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_render_html(
    state: State<'_, AppState>,
    payload: RenderPayload,
) -> Result<String, String> {
    let output_raw = payload
        .output_path
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "outputPath must be provided".to_string())?;
    let output_path = authorize_read_path(&state, output_raw)?;
    let input_path = match payload.input_path.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let assets_from = match payload.assets_from.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let doc_json = match payload.document.as_ref() {
        Some(v) => Some(json_value_to_string(v)?),
        None => None,
    };
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        guard
            .render_html_from_json_or_path(
                doc_json.as_deref(),
                input_path.as_deref(),
                &output_path,
                fonts_dir.as_deref(),
                assets_from.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        Ok(output_path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_run_preflight(
    state: State<'_, AppState>,
    payload: PreflightPayload,
) -> Result<PreflightResponse, String> {
    let input_path = match payload.input_path.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let assets_from = match payload.assets_from.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => Some(authorize_read_path(&state, raw)?),
        None => None,
    };
    let doc_json = match payload.document.as_ref() {
        Some(v) => Some(json_value_to_string(v)?),
        None => None,
    };
    let req = PreflightRequest::from_ui_options(payload.standard.as_deref(), payload.bleed);
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
        match guard.run_preflight_from_json_or_path(
            doc_json.as_deref(),
            input_path.as_deref(),
            fonts_dir.as_deref(),
            assets_from.as_deref(),
            &req,
        ) {
            Ok(report) => {
                let val = serde_json::to_value(report).map_err(|e| e.to_string())?;
                Ok(PreflightResponse {
                    success: true,
                    report: Some(val),
                    error: None,
                })
            }
            Err(err) => Ok(PreflightResponse {
                success: false,
                report: None,
                error: Some(err.to_string()),
            }),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_save_draft_recovery(
    state: State<'_, AppState>,
    snapshot: Value,
) -> Result<SuccessResponse, String> {
    let path = state.draft_recovery_path.clone();
    let json = serde_json::to_string(&snapshot).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        write_atomic(&path, json.as_bytes()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(SuccessResponse { success: true })
}

#[tauri::command]
pub async fn tok_load_draft_recovery(state: State<'_, AppState>) -> Result<Option<Value>, String> {
    let path = state.draft_recovery_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let val = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        Ok(Some(val))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_clear_draft_recovery(
    state: State<'_, AppState>,
) -> Result<SuccessResponse, String> {
    let path = state.draft_recovery_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(SuccessResponse { success: true })
}

#[tauri::command]
pub fn tok_set_dirty(state: State<'_, AppState>, dirty: bool) -> SuccessResponse {
    state.is_dirty.store(dirty, Ordering::SeqCst);
    SuccessResponse { success: true }
}

#[tauri::command]
pub async fn tok_rename_document_file(
    state: State<'_, AppState>,
    payload: Value,
) -> Result<Value, String> {
    let old_raw = payload
        .get("oldPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing oldPath".to_string())?;
    let new_raw = payload
        .get("newPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing newPath".to_string())?;
    let resolved_old = authorize_read_path(&state, old_raw)?;
    let resolved_new = authorize_read_path(&state, new_raw)?;

    tauri::async_runtime::spawn_blocking(move || {
        if !resolved_old.exists() {
            return Ok(json!({ "success": false, "error": "Source file does not exist" }));
        }
        if resolved_new.exists() && resolved_old != resolved_new {
            return Ok(json!({ "success": false, "error": "TARGET_EXISTS" }));
        }
        fs::rename(&resolved_old, &resolved_new).map_err(|e| e.to_string())?;
        Ok(json!({
            "success": true,
            "filePath": resolved_new.to_string_lossy().into_owned()
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn read_zip_entry_limited(archive: &mut ZipArchive<fs::File>, name: &str) -> Option<String> {
    let entry = archive.by_name(name).ok()?;
    let mut limited = entry.take(MAX_DECOMPRESSED_BYTES);
    let mut text = String::new();
    limited.read_to_string(&mut text).ok()?;
    Some(text)
}

#[tauri::command]
pub async fn tok_read_import_file(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<Value, String> {
    let resolved = authorize_read_path(&state, &file_path)?;
    tauri::async_runtime::spawn_blocking(move || {
        if !resolved.is_file() {
            return Err(format!(
                "Import file does not exist: {}",
                resolved.display()
            ));
        }
        let meta = fs::metadata(&resolved).map_err(|e| e.to_string())?;
        let bytes = fs::read(&resolved).map_err(|e| e.to_string())?;
        let ext = resolved
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let is_binary = ext == "docx";
        if !is_binary && bytes.starts_with(b"PK\x03\x04") {
            return Err(
                "הקובץ הוא ארכיון zip ולא מסמך טקסט. אם זו חבילת פרויקט (.tok), השתמש בפתיחת פרויקט."
                    .to_string(),
            );
        }

        let mut docx_xml: Option<String> = None;
        let mut footnotes_xml: Option<String> = None;
        let mut endnotes_xml: Option<String> = None;
        let mut numbering_xml: Option<String> = None;
        let mut styles_xml: Option<String> = None;

        if is_binary {
            if let Ok(file) = fs::File::open(&resolved) {
                if let Ok(mut archive) = ZipArchive::new(file) {
                    docx_xml = read_zip_entry_limited(&mut archive, "word/document.xml");
                    footnotes_xml = read_zip_entry_limited(&mut archive, "word/footnotes.xml");
                    endnotes_xml = read_zip_entry_limited(&mut archive, "word/endnotes.xml");
                    numbering_xml = read_zip_entry_limited(&mut archive, "word/numbering.xml");
                    styles_xml = read_zip_entry_limited(&mut archive, "word/styles.xml");
                }
            }
        }

        let data_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let text = docx_xml.clone().or_else(|| {
            if is_binary {
                None
            } else {
                Some(String::from_utf8_lossy(&bytes).into_owned())
            }
        });

        Ok(json!({
            "fileName": resolved.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            "filePath": resolved.to_string_lossy().into_owned(),
            "size": meta.len(),
            "isBinary": is_binary,
            "docxXml": docx_xml,
            "footnotesXml": footnotes_xml,
            "endnotesXml": endnotes_xml,
            "numberingXml": numbering_xml,
            "stylesXml": styles_xml,
            "dataBase64": data_base64,
            "text": text,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_decompress_raw(data_base64: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let compressed = base64::engine::general_purpose::STANDARD
            .decode(data_base64.trim())
            .map_err(|e| e.to_string())?;
        let decoder = DeflateDecoder::new(&compressed[..]);
        let mut out = Vec::new();
        decoder
            .take(MAX_DECOMPRESSED_BYTES)
            .read_to_end(&mut out)
            .map_err(|e| e.to_string())?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&out))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_inspect_font(
    state: State<'_, AppState>,
    font_path: String,
) -> Result<Value, String> {
    let resolved = authorize_read_path(&state, &font_path)?;
    tauri::async_runtime::spawn_blocking(move || fonts::inspect_font(&resolved))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_install_font(
    state: State<'_, AppState>,
    font_path: String,
) -> Result<Value, String> {
    let resolved = authorize_read_path(&state, &font_path)?;
    let fonts_dir = state.fonts_dir.clone();
    tauri::async_runtime::spawn_blocking(move || fonts::install_font(&resolved, &fonts_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tok_list_user_fonts(
    state: State<'_, AppState>,
) -> Result<Vec<fonts::UserFontEntry>, String> {
    let fonts_dir = state.fonts_dir.clone();
    tauri::async_runtime::spawn_blocking(move || fonts::list_user_fonts(&fonts_dir))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tok_list_system_fonts() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(fonts::list_system_fonts)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tok_get_recent_logs(state: State<'_, AppState>) -> Vec<String> {
    state.logger.get_recent_logs(100)
}

#[tauri::command]
pub fn tok_open_logs_folder(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let dir = state.logger.logs_dir();
    let _ = fs::create_dir_all(dir);
    app.opener()
        .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn tok_clean_old_logs(state: State<'_, AppState>, days: Option<u64>) -> usize {
    state.logger.clean_old_logs(days)
}

#[tauri::command]
pub fn tok_set_log_retention(state: State<'_, AppState>, days: u64) {
    state.logger.set_retention_days(days);
}

#[tauri::command]
pub fn tok_get_plugins(state: State<'_, AppState>) -> Result<Vec<LoadedPlugin>, String> {
    let guard = state
        .plugins
        .lock()
        .map_err(|_| "PluginSystem lock poisoned".to_string())?;
    Ok(guard.discover_plugins())
}

#[tauri::command]
pub fn tok_reload_plugins(state: State<'_, AppState>) -> Result<Vec<LoadedPlugin>, String> {
    let guard = state
        .plugins
        .lock()
        .map_err(|_| "PluginSystem lock poisoned".to_string())?;
    Ok(guard.discover_plugins())
}

#[tauri::command]
pub fn tok_toggle_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<bool, String> {
    let mut guard = state
        .plugins
        .lock()
        .map_err(|_| "PluginSystem lock poisoned".to_string())?;
    guard.toggle_plugin(&plugin_id, enabled)
}

#[tauri::command]
pub fn tok_open_plugins_folder(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let dir = {
        let guard = state
            .plugins
            .lock()
            .map_err(|_| "PluginSystem lock poisoned".to_string())?;
        guard.user_plugins_dir().to_path_buf()
    };
    let _ = fs::create_dir_all(&dir);
    app.opener()
        .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn tok_get_plugin_system_status(
    state: State<'_, AppState>,
) -> Result<PluginSystemStatus, String> {
    let guard = state
        .plugins
        .lock()
        .map_err(|_| "PluginSystem lock poisoned".to_string())?;
    Ok(guard.get_status())
}

#[tauri::command]
pub fn tok_set_plugin_safe_mode(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let mut guard = state
        .plugins
        .lock()
        .map_err(|_| "PluginSystem lock poisoned".to_string())?;
    guard.set_safe_mode(enabled)
}

#[tauri::command]
pub async fn tok_check_for_updates() -> UpdateCheckResult {
    updater::check_for_updates(env!("CARGO_PKG_VERSION")).await
}

#[tauri::command]
pub fn tok_open_release_url(app: AppHandle, url: Option<String>) -> Result<(), String> {
    let target = url
        .filter(|u| updater::is_safe_external_url(u))
        .unwrap_or_else(updater::releases_page);
    app.opener()
        .open_url(target, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tok_download_update(
    app: AppHandle,
    state: State<'_, AppState>,
    download_url: String,
) -> Result<DownloadUpdateResult, String> {
    let updates_dir = state.user_data_dir.join("updates");
    let res = updater::download_update(&app, &download_url, &updates_dir).await;
    if res.success {
        if let Some(ref fp) = res.file_path {
            if let Ok(mut paths) = state.paths.lock() {
                paths.authorize_update_path(&PathBuf::from(fp));
            }
        }
    }
    Ok(res)
}

#[tauri::command]
pub fn tok_apply_update(
    state: State<'_, AppState>,
    file_path: String,
    silent: Option<bool>,
) -> Result<ApplyUpdateResult, String> {
    let resolved = {
        let paths = state
            .paths
            .lock()
            .map_err(|_| "PathPolicy lock poisoned".to_string())?;
        paths.assert_authorized_update(&PathBuf::from(&file_path))?
    };
    Ok(updater::apply_installer(&resolved, silent.unwrap_or(false)))
}

#[tauri::command]
pub fn tok_inspect_offline_update(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<Option<OfflineUpdateCandidate>, String> {
    let resolved = {
        let paths = state
            .paths
            .lock()
            .map_err(|_| "PathPolicy lock poisoned".to_string())?;
        paths.assert_authorized_update(&PathBuf::from(&file_path))?
    };
    Ok(updater::inspect_offline_file(
        &resolved,
        env!("CARGO_PKG_VERSION"),
    ))
}

#[tauri::command]
pub async fn tok_scan_usb_updates(
    state: State<'_, AppState>,
) -> Result<Vec<OfflineUpdateCandidate>, String> {
    let candidates = tauri::async_runtime::spawn_blocking(|| {
        updater::scan_usb_drives(env!("CARGO_PKG_VERSION"))
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Ok(mut paths) = state.paths.lock() {
        for cand in &candidates {
            paths.authorize_update_path(&PathBuf::from(&cand.file_path));
        }
    }
    Ok(candidates)
}

#[tauri::command]
pub fn tok_configure_heading_numbering(config: Value) -> Value {
    json!({ "success": true, "config": config })
}

#[tauri::command]
pub fn tok_reorder_heading(payload: Value) -> Value {
    json!({ "success": true, "payload": payload })
}

#[tauri::command]
pub fn tok_insert_heading(payload: Value) -> Value {
    json!({ "success": true, "payload": payload })
}

#[tauri::command]
pub fn tok_delete_heading(payload: Value) -> Value {
    json!({ "success": true, "payload": payload })
}

#[tauri::command]
pub fn tok_recalculate_heading_numbers(payload: Option<Value>) -> Value {
    json!({ "success": true, "payload": payload })
}

#[tauri::command]
pub fn tok_send_command(cmd: Value) -> Value {
    json!({ "success": true, "cmd": cmd })
}

#[tauri::command]
pub fn tok_report_startup_trace(
    app: AppHandle,
    state: State<'_, AppState>,
    timings: StartupTimings,
) {
    let enabled = state.trace_enabled();
    if !enabled {
        return;
    }
    let items: [(&str, Option<f64>); 7] = [
        ("renderer-navStart", timings.nav_start),
        ("renderer-htmlLoaded", timings.html_loaded),
        ("renderer-scriptStart", timings.script_start),
        ("renderer-dclStart", timings.dcl_start),
        ("renderer-uiBuilt", timings.ui_built),
        ("renderer-domContentLoaded", timings.dom_content_loaded),
        ("renderer-fcp", timings.fcp),
    ];
    for (label, val) in items {
        if let Some(ms) = val.filter(|v| v.is_finite() && *v > 0.0) {
            trace_milestone(true, label, Some(ms.round() as u64));
        }
    }
    trace_milestone(true, "trace-done", None);
    if state.trace_exit() {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.close();
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
            app.exit(0);
        });
    }
}

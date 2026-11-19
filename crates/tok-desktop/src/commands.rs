use crate::state::{trace_milestone, AppState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tok_engine_service::{write_atomic, PdfRenderRequest, PreflightRequest};

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
pub struct PluginSystemStatusResponse {
    pub is_safe_mode: bool,
    pub is_safer_active: bool,
    pub user_plugins_dir: String,
    pub builtin_plugins_dir: String,
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
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderPdfOptions {
    pub format: Option<String>,
    pub color_profile: Option<String>,
    pub bleed: Option<bool>,
    pub crop_marks: Option<bool>,
    pub registration_marks: Option<bool>,
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
    let session = Arc::clone(&state.session);
    let fonts_dir = optional_fonts_dir(&state.fonts_dir);

    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "WorkspaceSession lock poisoned".to_string())?;
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
    );
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
pub fn tok_get_plugins() -> Vec<Value> {
    Vec::new()
}

#[tauri::command]
pub fn tok_get_plugin_system_status(state: State<'_, AppState>) -> PluginSystemStatusResponse {
    PluginSystemStatusResponse {
        is_safe_mode: true,
        is_safer_active: false,
        user_plugins_dir: state
            .user_data_dir
            .join("plugins")
            .to_string_lossy()
            .into_owned(),
        builtin_plugins_dir: String::new(),
    }
}

#[tauri::command]
pub fn tok_list_user_fonts() -> Vec<Value> {
    Vec::new()
}

#[tauri::command]
pub fn tok_list_system_fonts() -> Vec<String> {
    Vec::new()
}

#[tauri::command]
pub fn tok_get_recent_logs() -> Vec<String> {
    Vec::new()
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

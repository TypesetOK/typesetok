#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod commands;
mod fonts;
mod logger;
mod plugins;
mod state;
mod updater;

use logger::TokLogger;
use plugins::PluginSystem;
use state::{
    read_install_registry, resolve_projects_dir, resolve_user_data_dir, trace_memory,
    trace_milestone, AppState, BootstrapPayload, PathPolicy, SettingsStore,
    SUPPORTED_CAPABILITIES,
};
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::webview::PageLoadEvent;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tok_engine_service::WorkspaceSession;

fn find_startup_file_arg() -> Option<String> {
    env::args().skip(1).find(|arg| {
        if arg.starts_with('-') {
            return false;
        }
        let p = PathBuf::from(arg);
        p.extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tok"))
            || p.is_file()
    })
}

fn build_init_script(bootstrap: &BootstrapPayload, trace_enabled: bool) -> String {
    let bootstrap_json = serde_json::to_string(bootstrap).unwrap_or_else(|_| "{}".to_string());
    let mut script = format!("window.__TOK_BOOTSTRAP__ = {bootstrap_json};\n");
    if trace_enabled {
        script.push_str(
            r#"
(function () {
  var reported = false;
  function reportTimings() {
    if (reported) return;
    reported = true;
    requestAnimationFrame(function () {
      setTimeout(function () {
        try {
          var o = performance.timeOrigin;
          var at = function (n) {
            var e = performance.getEntriesByName(n)[0];
            return e ? o + e.startTime : null;
          };
          var nav = performance.getEntriesByType('navigation')[0] || {};
          var timings = {
            navStart: o,
            htmlLoaded: o + (nav.responseEnd || 0),
            scriptStart: at('tok-script-start'),
            dclStart: o + (nav.domContentLoadedEventStart || 0),
            uiBuilt: at('tok-ui-built'),
            domContentLoaded: o + (nav.domContentLoadedEventEnd || 0),
            fcp: at('first-contentful-paint')
          };
          if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
            window.__TAURI_INTERNALS__.invoke('tok_report_startup_trace', { timings: timings });
          }
        } catch (_) {}
      }, 0);
    });
  }
  if (document.readyState === 'complete') {
    reportTimings();
  } else {
    window.addEventListener('load', reportTimings, { once: true });
  }
})();
"#,
        );
    }
    script
}

fn main() {
    let trace_mode = env::var("TOK_STARTUP_TRACE")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let trace_enabled = trace_mode.is_some();

    let shown_once = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shown_once_page = Arc::clone(&shown_once);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(file_arg) = args.iter().skip(1).find(|arg| {
                if arg.starts_with('-') {
                    return false;
                }
                let p = PathBuf::from(arg);
                p.extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tok"))
                    || p.is_file()
            }) {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut paths) = state.paths.lock() {
                        paths.authorize_path(&PathBuf::from(file_arg));
                    }
                }
                let _ = app.emit_to("main", "tok:open-file", file_arg.clone());
            }
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("tok-asset", assets::handle_tok_asset_protocol)
        .setup(move |app| {
            trace_milestone(trace_enabled, "app-ready", None);

            let app_data_fallback = app.path().app_data_dir().ok();
            let docs_fallback = app.path().document_dir().ok();

            let user_data_dir = resolve_user_data_dir(app_data_fallback);
            let _ = std::fs::create_dir_all(&user_data_dir);

            let reg_info = read_install_registry();
            let projects_dir = resolve_projects_dir(reg_info.projects_dir.as_deref(), docs_fallback);

            let settings_path = user_data_dir.join("settings.json");
            let settings_store = SettingsStore::load(settings_path);
            let settings_snapshot = settings_store.snapshot();

            let startup_file = find_startup_file_arg();
            let mut path_policy = PathPolicy::new(projects_dir);
            if let Some(ref sf) = startup_file {
                path_policy.authorize_path(&PathBuf::from(sf));
            }

            let bootstrap = BootstrapPayload {
                platform: "tauri",
                install_language: reg_info.language.clone(),
                startup_file: startup_file.clone(),
                settings: settings_snapshot,
                capabilities: SUPPORTED_CAPABILITIES.to_vec(),
            };
            let init_script = build_init_script(&bootstrap, trace_enabled);

            let logger = Arc::new(TokLogger::new(&user_data_dir));
            logger.info("TypesetOK Tauri shell starting");
            let plugin_system = PluginSystem::load(&user_data_dir);

            let state = AppState {
                session: Arc::new(Mutex::new(WorkspaceSession::new())),
                settings: Mutex::new(settings_store),
                paths: Mutex::new(path_policy),
                plugins: Mutex::new(plugin_system),
                logger,
                is_dirty: AtomicBool::new(false),
                startup_file,
                user_data_dir: user_data_dir.clone(),
                fonts_dir: user_data_dir.join("fonts"),
                draft_recovery_path: user_data_dir.join("draft_recovery.json"),
                trace_mode: trace_mode.clone(),
            };
            app.manage(state);
            trace_milestone(trace_enabled, "session-ready", None);

            let webview_data_dir = user_data_dir.join("EBWebView");
            let _ = std::fs::create_dir_all(&webview_data_dir);

            let win_builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("TypesetOK")
                    .inner_size(1440.0, 900.0)
                    .min_inner_size(1024.0, 700.0)
                    .decorations(false)
                    .shadow(true)
                    .visible(false)
                    .data_directory(webview_data_dir)
                    .initialization_script(&init_script);

            trace_milestone(trace_enabled, "before-win-build", None);
            win_builder.build()?;
            trace_milestone(trace_enabled, "main-window-created", None);

            Ok(())
        })
        .on_page_load(move |webview, payload| {
            if webview.label() == "main" && payload.event() == PageLoadEvent::Finished {
                if !shown_once_page.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    let app = webview.app_handle();
                    let trace_enabled = app
                        .try_state::<AppState>()
                        .is_some_and(|s| s.trace_enabled());
                    trace_milestone(trace_enabled, "main-ready-to-show", None);
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.maximize();
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                    trace_milestone(trace_enabled, "main-shown", None);
                    trace_memory(trace_enabled, "rust-rss");
                }
            }
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let is_dirty = app
                    .try_state::<AppState>()
                    .is_some_and(|s| s.is_dirty.load(Ordering::SeqCst));
                if !is_dirty {
                    return;
                }
                api.prevent_close();
                let app_handle = app.clone();
                let win_clone = window.clone();
                std::thread::spawn(move || {
                    let save_yes = app_handle
                        .dialog()
                        .message("במסמך קיימים שינויים שלא נשמרו.\nהאם לצאת ללא שמירה? (בחר 'כן' ליציאה ללא שמירה, 'לא' לחזרה למסמך)")
                        .title("TypesetOK")
                        .kind(MessageDialogKind::Warning)
                        .buttons(MessageDialogButtons::YesNo)
                        .blocking_show();
                    if save_yes {
                        if let Some(state) = app_handle.try_state::<AppState>() {
                            state.is_dirty.store(false, Ordering::SeqCst);
                        }
                        let _ = win_clone.close();
                    }
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::tok_read_settings,
            commands::tok_set_setting,
            commands::tok_set_settings,
            commands::tok_remove_setting,
            commands::tok_get_app_info,
            commands::tok_get_projects_dir,
            commands::tok_get_startup_file,
            commands::tok_open_external,
            commands::tok_close_window,
            commands::tok_minimize_window,
            commands::tok_maximize_window,
            commands::tok_is_maximized,
            commands::tok_quit_app,
            commands::tok_set_dirty,
            commands::tok_show_open_dialog,
            commands::tok_show_import_dialog,
            commands::tok_show_save_dialog,
            commands::tok_open_document,
            commands::tok_save_document,
            commands::tok_rename_document_file,
            commands::tok_read_import_file,
            commands::tok_decompress_raw,
            commands::tok_typeset_document,
            commands::tok_render_pdf,
            commands::tok_render_html,
            commands::tok_run_preflight,
            commands::tok_save_draft_recovery,
            commands::tok_load_draft_recovery,
            commands::tok_clear_draft_recovery,
            commands::tok_get_plugins,
            commands::tok_reload_plugins,
            commands::tok_toggle_plugin,
            commands::tok_open_plugins_folder,
            commands::tok_get_plugin_system_status,
            commands::tok_set_plugin_safe_mode,
            commands::tok_inspect_font,
            commands::tok_install_font,
            commands::tok_list_user_fonts,
            commands::tok_list_system_fonts,
            commands::tok_get_recent_logs,
            commands::tok_open_logs_folder,
            commands::tok_clean_old_logs,
            commands::tok_set_log_retention,
            commands::tok_check_for_updates,
            commands::tok_open_release_url,
            commands::tok_download_update,
            commands::tok_apply_update,
            commands::tok_inspect_offline_update,
            commands::tok_scan_usb_updates,
            commands::tok_configure_heading_numbering,
            commands::tok_reorder_heading,
            commands::tok_insert_heading,
            commands::tok_delete_heading,
            commands::tok_recalculate_heading_numbers,
            commands::tok_send_command,
            commands::tok_report_startup_trace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TypesetOK Tauri application");
}

/**
 * Tauri v2 host adapter for TypesetOK.
 *
 * Bridges `TokHostBridge` calls to Rust `#[tauri::command]` invocations and
 * Tauri backend event listeners without touching `window.__TAURI_INTERNALS__`
 * at module evaluation time, keeping Node-based unit tests runnable without a
 * DOM or WebView.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  readWindowBootstrap,
  type TokHostBootstrap,
  type TokHostBridge,
  type TokHostKind,
  type TokHostMethods,
  type TokUpdateDownloadProgress,
} from './hostBridge.js';

export interface TauriRuntimeApi {
  invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  listen: <T = unknown>(
    event: string,
    handler: (event: { payload: T }) => void
  ) => Promise<UnlistenFn>;
}

const DEFAULT_TAURI_RUNTIME: TauriRuntimeApi = {
  invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) =>
    tauriInvoke<T>(cmd, args),
  listen: <T = unknown>(
    event: string,
    handler: (event: { payload: T }) => void
  ) => tauriListen<T>(event, handler),
};

const ALL_TAURI_HOST_METHODS: ReadonlySet<keyof TokHostMethods> = new Set<keyof TokHostMethods>([
  'sendCommand',
  'onEvent',
  'renderPdf',
  'renderPdfPreview',
  'renderHtml',
  'getRecentLogs',
  'openLogsFolder',
  'cleanOldLogs',
  'setLogRetention',
  'checkForUpdates',
  'openReleaseUrl',
  'downloadUpdate',
  'onUpdateDownloadProgress',
  'applyUpdate',
  'inspectOfflineUpdate',
  'scanUsbUpdates',
  'getPlugins',
  'togglePlugin',
  'openPluginsFolder',
  'reloadPlugins',
  'getPluginSystemStatus',
  'setPluginSafeMode',
  'saveDocument',
  'openDocument',
  'renameDocumentFile',
  'getProjectsDir',
  'saveDraftRecovery',
  'loadDraftRecovery',
  'clearDraftRecovery',
  'typesetDocument',
  'runPreflight',
  'inspectFont',
  'installFont',
  'listUserFonts',
  'listSystemFonts',
  'configureHeadingNumbering',
  'reorderHeading',
  'insertHeading',
  'deleteHeading',
  'recalculateHeadingNumbers',
  'openExternal',
  'getAppInfo',
  'showSaveDialog',
  'showOpenDialog',
  'showImportDialog',
  'getStartupFile',
  'readImportFile',
  'decompressRaw',
  'closeWindow',
  'minimizeWindow',
  'maximizeWindow',
  'isMaximized',
  'quitApp',
  'onOpenFile',
  'readSettings',
  'setSetting',
  'setSettings',
  'removeSetting',
]);

type RenderInputPayload =
  | string
  | {
      inputPath?: string;
      document?: unknown;
      outputPath?: string;
      options?: unknown;
      assetsFrom?: string;
    };

function normalizeRenderPayload(
  input: RenderInputPayload,
  outputPath?: string
): Record<string, unknown> {
  if (typeof input === 'string') {
    return {
      inputPath: input,
      ...(outputPath !== undefined ? { outputPath } : {}),
    };
  }
  const base = input && typeof input === 'object' ? { ...input } : {};
  if (base.outputPath === undefined && outputPath !== undefined) {
    base.outputPath = outputPath;
  }
  return base;
}

export class TauriHostAdapter implements TokHostBridge {
  public readonly kind: TokHostKind = 'tauri';
  public readonly isDesktop = true;
  private readonly runtime: TauriRuntimeApi;
  private readonly explicitMethods?: ReadonlySet<keyof TokHostMethods>;

  constructor(
    runtime: TauriRuntimeApi = DEFAULT_TAURI_RUNTIME,
    supportedMethods?: ReadonlySet<keyof TokHostMethods>
  ) {
    this.runtime = runtime;
    this.explicitMethods = supportedMethods;
  }

  public getBootstrap(): TokHostBootstrap {
    return { platform: 'tauri', ...readWindowBootstrap() };
  }

  public has(method: keyof TokHostMethods): boolean {
    if (this.explicitMethods) {
      return this.explicitMethods.has(method);
    }
    const caps = readWindowBootstrap().capabilities;
    if (caps) {
      return caps.includes(method);
    }
    return ALL_TAURI_HOST_METHODS.has(method);
  }

  private invokeCmd<T = any>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    return this.runtime.invoke<T>(cmd, args);
  }

  private subscribe<T>(eventName: string, callback: (payload: T) => void): () => void {
    let disposed = false;
    let unlistenFn: UnlistenFn | null = null;

    void this.runtime
      .listen<T>(eventName, (event) => {
        if (!disposed) {
          callback(event.payload);
        }
      })
      .then((unlisten) => {
        if (disposed) {
          try {
            unlisten();
          } catch (err) {
            console.warn(`[TAURI] Deferred unlisten failed for ${eventName}:`, err);
          }
          return;
        }
        unlistenFn = unlisten;
      })
      .catch((err) => {
        console.warn(`[TAURI] Failed to subscribe to ${eventName}:`, err);
      });

    return () => {
      disposed = true;
      if (unlistenFn) {
        try {
          unlistenFn();
        } catch (err) {
          console.warn(`[TAURI] Unlisten failed for ${eventName}:`, err);
        }
        unlistenFn = null;
      }
    };
  }

  public sendCommand(cmd: unknown): Promise<unknown> {
    return this.invokeCmd('tok_send_command', { cmd });
  }

  public onEvent(callback: (event: unknown) => void): () => void {
    return this.subscribe('tok:event', callback);
  }

  public renderPdf(input: RenderInputPayload, outputPath?: string): Promise<string> {
    return this.invokeCmd('tok_render_pdf', {
      payload: normalizeRenderPayload(input, outputPath),
    });
  }

  public renderPdfPreview(input: RenderInputPayload): Promise<string> {
    return this.invokeCmd('tok_render_pdf_preview', {
      payload: normalizeRenderPayload(input),
    });
  }

  public renderHtml(input: RenderInputPayload, outputPath?: string): Promise<string> {
    return this.invokeCmd('tok_render_html', {
      payload: normalizeRenderPayload(input, outputPath),
    });
  }

  // Logger
  public getRecentLogs(): Promise<string[]> {
    return this.invokeCmd('tok_get_recent_logs');
  }
  public openLogsFolder(): Promise<boolean> {
    return this.invokeCmd('tok_open_logs_folder');
  }
  public cleanOldLogs(days?: number): Promise<number> {
    return this.invokeCmd('tok_clean_old_logs', { days });
  }
  public setLogRetention(days: number): Promise<void> {
    return this.invokeCmd('tok_set_log_retention', { days });
  }

  // Updater
  public checkForUpdates(): Promise<any> {
    return this.invokeCmd('tok_check_for_updates');
  }
  public openReleaseUrl(url?: string): Promise<void> {
    return this.invokeCmd('tok_open_release_url', { url });
  }
  public downloadUpdate(
    downloadUrl: string
  ): Promise<{ success: boolean; filePath?: string; fileSize?: number; sha256?: string; error?: string }> {
    return this.invokeCmd('tok_download_update', { downloadUrl });
  }
  public onUpdateDownloadProgress(callback: (data: TokUpdateDownloadProgress) => void): () => void {
    return this.subscribe('tok:update-download-progress', callback);
  }
  public applyUpdate(filePath: string, silent?: boolean): Promise<{ success: boolean; error?: string }> {
    return this.invokeCmd('tok_apply_update', { filePath, silent });
  }
  public inspectOfflineUpdate(filePath: string): Promise<any> {
    return this.invokeCmd('tok_inspect_offline_update', { filePath });
  }
  public scanUsbUpdates(): Promise<any[]> {
    return this.invokeCmd('tok_scan_usb_updates');
  }

  // Plugins
  public getPlugins(): Promise<unknown[]> {
    return this.invokeCmd('tok_get_plugins');
  }
  public togglePlugin(pluginId: string, enabled: boolean): Promise<boolean> {
    return this.invokeCmd('tok_toggle_plugin', { pluginId, enabled });
  }
  public openPluginsFolder(): Promise<void> {
    return this.invokeCmd('tok_open_plugins_folder');
  }
  public reloadPlugins(): Promise<unknown[]> {
    return this.invokeCmd('tok_reload_plugins');
  }
  public getPluginSystemStatus(): Promise<{
    isSafeMode: boolean;
    isSaferActive: boolean;
    userPluginsDir: string;
    builtinPluginsDir: string;
  }> {
    return this.invokeCmd('tok_get_plugin_system_status');
  }
  public setPluginSafeMode(enabled: boolean): Promise<boolean> {
    return this.invokeCmd('tok_set_plugin_safe_mode', { enabled });
  }

  // Document Storage & Typesetting
  public saveDocument(payload: {
    document: unknown;
    filePath: string;
    assetsFrom?: string;
  }): Promise<{ success: boolean; filePath: string }> {
    return this.invokeCmd('tok_save_document', { payload });
  }
  public openDocument(filePath: string): Promise<any> {
    return this.invokeCmd('tok_open_document', { filePath });
  }
  public renameDocumentFile(payload: {
    oldPath: string;
    newPath: string;
  }): Promise<{ success: boolean; filePath?: string; error?: string }> {
    return this.invokeCmd('tok_rename_document_file', { payload });
  }
  public getProjectsDir(): Promise<string> {
    return this.invokeCmd('tok_get_projects_dir');
  }
  public saveDraftRecovery(snapshot: unknown): Promise<{ success: boolean }> {
    return this.invokeCmd('tok_save_draft_recovery', { snapshot });
  }
  public loadDraftRecovery(): Promise<unknown | null> {
    return this.invokeCmd('tok_load_draft_recovery');
  }
  public clearDraftRecovery(): Promise<{ success: boolean }> {
    return this.invokeCmd('tok_clear_draft_recovery');
  }
  public typesetDocument(
    payload:
      | {
          document?: unknown;
          inputPath?: string;
          assetsFrom?: string;
          withPreflight?: boolean;
          /** Export format id ('pdfx1a' | 'pdfx4' | 'pdf') the print check is evaluated against. */
          preflightStandard?: string;
          preflightBleed?: boolean;
        }
      | unknown
  ): Promise<{ success: boolean; layoutJson?: string; pages?: any[]; preflight?: any }> {
    return this.invokeCmd('tok_typeset_document', { payload });
  }
  public runPreflight(payload: {
    document?: unknown;
    inputPath?: string;
    assetsFrom?: string;
    standard?: string;
    bleed?: boolean;
  }): Promise<{ success: boolean; report?: unknown; error?: string }> {
    return this.invokeCmd('tok_run_preflight', { payload });
  }

  // Font Inspection, Installation & System Discovery
  public inspectFont(fontPath: string): Promise<{ valid: boolean; error?: string; message?: string; report?: any }> {
    return this.invokeCmd('tok_inspect_font', { fontPath });
  }
  public installFont(
    fontPath: string
  ): Promise<{ success: boolean; path?: string; error?: string; message?: string; report?: any; base64Data?: string; assetUrl?: string }> {
    return this.invokeCmd('tok_install_font', { fontPath });
  }
  public listUserFonts(): Promise<{ fileName: string; path: string; base64Data?: string; assetUrl?: string; report?: any }[]> {
    return this.invokeCmd('tok_list_user_fonts');
  }
  public listSystemFonts(): Promise<string[]> {
    return this.invokeCmd('tok_list_system_fonts');
  }

  // Heading & Outline Numbering
  public configureHeadingNumbering(config: unknown): Promise<unknown> {
    return this.invokeCmd('tok_configure_heading_numbering', { config });
  }
  public reorderHeading(payload: {
    nodeId: string;
    newParentId?: string;
    targetIndex: number;
    config?: unknown;
  }): Promise<unknown> {
    return this.invokeCmd('tok_reorder_heading', { payload });
  }
  public insertHeading(payload: {
    parentId?: string;
    targetIndex: number;
    node: unknown;
    config?: unknown;
  }): Promise<unknown> {
    return this.invokeCmd('tok_insert_heading', { payload });
  }
  public deleteHeading(payload: { nodeId: string; config?: unknown }): Promise<unknown> {
    return this.invokeCmd('tok_delete_heading', { payload });
  }
  public recalculateHeadingNumbers(payload?: { outline?: unknown; config?: unknown }): Promise<unknown> {
    return this.invokeCmd('tok_recalculate_heading_numbers', { payload });
  }

  // System & Utilities
  public openExternal(url: string): Promise<void> {
    return this.invokeCmd('tok_open_external', { url });
  }
  public getAppInfo(): Promise<{ version: string; name: string; repoUrl?: string }> {
    return this.invokeCmd('tok_get_app_info');
  }
  public showSaveDialog(options?: {
    defaultPath?: string;
    filters?: { name: string; extensions: string[] }[];
  }): Promise<string | null> {
    return this.invokeCmd('tok_show_save_dialog', { options });
  }
  public showOpenDialog(options?: {
    filters?: { name: string; extensions: string[] }[];
  }): Promise<string | null> {
    return this.invokeCmd('tok_show_open_dialog', { options });
  }
  public showImportDialog(options?: {
    filters?: { name: string; extensions: string[] }[];
  }): Promise<string | null> {
    return this.invokeCmd('tok_show_import_dialog', { options });
  }
  public getStartupFile(): Promise<string | null> {
    if (!this.has('getStartupFile')) {
      return Promise.resolve(this.getBootstrap().startupFile ?? null);
    }
    return this.invokeCmd('tok_get_startup_file');
  }
  public readImportFile(filePath: string): Promise<{
    fileName: string;
    filePath: string;
    size: number;
    isBinary: boolean;
    docxXml?: string;
    footnotesXml?: string;
    endnotesXml?: string;
    numberingXml?: string;
    stylesXml?: string;
    dataBase64: string;
    text?: string;
  }> {
    return this.invokeCmd('tok_read_import_file', { filePath });
  }
  public decompressRaw(dataBase64: string): Promise<string> {
    return this.invokeCmd('tok_decompress_raw', { dataBase64 });
  }
  public closeWindow(): Promise<void> {
    return this.invokeCmd('tok_close_window');
  }
  public minimizeWindow(): Promise<void> {
    return this.invokeCmd('tok_minimize_window');
  }
  public maximizeWindow(): Promise<{ success: boolean; isMaximized: boolean }> {
    return this.invokeCmd('tok_maximize_window');
  }
  public isMaximized(): Promise<{ isMaximized: boolean }> {
    return this.invokeCmd('tok_is_maximized');
  }
  public quitApp(): Promise<void> {
    return this.invokeCmd('tok_quit_app');
  }
  public async setDirty(dirty: boolean): Promise<void> {
    await this.invokeCmd('tok_set_dirty', { dirty });
  }
  public onOpenFile(callback: (filePath: string) => void): () => void {
    return this.subscribe('tok:open-file', callback);
  }

  // Host-backed Persistent Settings
  public readSettings(): Promise<Record<string, string>> {
    return this.invokeCmd('tok_read_settings');
  }
  public setSetting(key: string, value: string): Promise<{ success: boolean }> {
    return this.invokeCmd('tok_set_setting', { key, value });
  }
  public setSettings(entries: Record<string, string>): Promise<{ success: boolean }> {
    return this.invokeCmd('tok_set_settings', { entries });
  }
  public removeSetting(key: string): Promise<{ success: boolean }> {
    return this.invokeCmd('tok_remove_setting', { key });
  }
}

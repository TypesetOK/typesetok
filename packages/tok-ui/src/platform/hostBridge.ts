/**
 * Platform-agnostic host bridge abstraction for TypesetOK.
 *
 * Isolates all desktop IPC calls behind `TokHostBridge` so UI modules never
 * touch `window.tokIpc` directly. Supports Electron (`ElectronHostAdapter`),
 * standalone browser preview (`WebPreviewAdapter`), and future Tauri adapters
 * injected via `setHostBridge()`.
 */

export type TokHostKind = 'electron' | 'tauri' | 'web';

export interface TokHostBootstrap {
  installLanguage?: 'he' | 'en' | null;
  startupFile?: string | null;
  settings?: Record<string, string>;
  platform?: TokHostKind;
}

export interface TokUpdateDownloadProgress {
  percent: number;
  transferredBytes: number;
  totalBytes: number;
  speedBytesPerSec: number;
  etaSeconds: number;
}

export interface TokHostMethods {
  sendCommand: (cmd: unknown) => Promise<unknown>;
  onEvent: (callback: (event: unknown) => void) => () => void;
  renderPdf: (
    input: string | { inputPath?: string; document?: unknown; outputPath?: string; options?: unknown; assetsFrom?: string },
    outputPath?: string
  ) => Promise<string>;
  renderHtml: (
    input: string | { inputPath?: string; document?: unknown; outputPath?: string; options?: unknown; assetsFrom?: string },
    outputPath?: string
  ) => Promise<string>;

  // Logger
  getRecentLogs: () => Promise<string[]>;
  openLogsFolder: () => Promise<boolean>;
  cleanOldLogs: (days?: number) => Promise<number>;
  setLogRetention: (days: number) => Promise<void>;

  // Updater
  checkForUpdates: () => Promise<any>;
  openReleaseUrl: (url?: string) => Promise<void>;
  downloadUpdate: (
    downloadUrl: string
  ) => Promise<{ success: boolean; filePath?: string; fileSize?: number; sha256?: string; error?: string }>;
  onUpdateDownloadProgress: (callback: (data: TokUpdateDownloadProgress) => void) => () => void;
  applyUpdate: (filePath: string, silent?: boolean) => Promise<{ success: boolean; error?: string }>;
  inspectOfflineUpdate: (filePath: string) => Promise<any>;
  scanUsbUpdates: () => Promise<any[]>;

  // Plugins
  getPlugins: () => Promise<unknown[]>;
  togglePlugin: (pluginId: string, enabled: boolean) => Promise<boolean>;
  openPluginsFolder: () => Promise<void>;
  reloadPlugins: () => Promise<unknown[]>;
  getPluginSystemStatus: () => Promise<{
    isSafeMode: boolean;
    isSaferActive: boolean;
    userPluginsDir: string;
    builtinPluginsDir: string;
  }>;
  setPluginSafeMode: (enabled: boolean) => Promise<boolean>;

  // Document Storage & Typesetting
  saveDocument: (payload: {
    document: unknown;
    filePath: string;
    assetsFrom?: string;
  }) => Promise<{ success: boolean; filePath: string }>;
  openDocument: (filePath: string) => Promise<any>;
  renameDocumentFile: (payload: {
    oldPath: string;
    newPath: string;
  }) => Promise<{ success: boolean; filePath?: string; error?: string }>;
  getProjectsDir: () => Promise<string>;
  saveDraftRecovery: (snapshot: unknown) => Promise<{ success: boolean }>;
  loadDraftRecovery: () => Promise<unknown | null>;
  clearDraftRecovery: () => Promise<{ success: boolean }>;
  typesetDocument: (
    payload: { document?: unknown; inputPath?: string; assetsFrom?: string; withPreflight?: boolean } | unknown
  ) => Promise<{ success: boolean; layoutJson?: string; pages?: any[]; preflight?: any }>;
  runPreflight: (payload: {
    document?: unknown;
    inputPath?: string;
    assetsFrom?: string;
    standard?: string;
    bleed?: boolean;
  }) => Promise<{ success: boolean; report?: unknown; error?: string }>;

  // Font Inspection, Installation & System Discovery
  inspectFont: (fontPath: string) => Promise<{ valid: boolean; error?: string; message?: string; report?: any }>;
  installFont: (
    fontPath: string
  ) => Promise<{ success: boolean; path?: string; error?: string; message?: string; report?: any; base64Data?: string }>;
  listUserFonts: () => Promise<{ fileName: string; path: string; base64Data: string; report?: any }[]>;
  listSystemFonts?: () => Promise<string[]>;

  // Heading & Outline Numbering
  configureHeadingNumbering: (config: unknown) => Promise<unknown>;
  reorderHeading: (payload: {
    nodeId: string;
    newParentId?: string;
    targetIndex: number;
    config?: unknown;
  }) => Promise<unknown>;
  insertHeading: (payload: {
    parentId?: string;
    targetIndex: number;
    node: unknown;
    config?: unknown;
  }) => Promise<unknown>;
  deleteHeading: (payload: { nodeId: string; config?: unknown }) => Promise<unknown>;
  recalculateHeadingNumbers: (payload?: { outline?: unknown; config?: unknown }) => Promise<unknown>;

  // System & Utilities
  openExternal: (url: string) => Promise<void>;
  getAppInfo: () => Promise<{ version: string; name: string; repoUrl?: string }>;
  showSaveDialog: (options?: {
    defaultPath?: string;
    filters?: { name: string; extensions: string[] }[];
  }) => Promise<string | null>;
  showOpenDialog: (options?: {
    filters?: { name: string; extensions: string[] }[];
  }) => Promise<string | null>;
  showImportDialog: (options?: {
    filters?: { name: string; extensions: string[] }[];
  }) => Promise<string | null>;
  getStartupFile: () => Promise<string | null>;
  readImportFile: (filePath: string) => Promise<{
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
  }>;
  decompressRaw: (dataBase64: string) => Promise<string>;
  closeWindow: () => Promise<void>;
  minimizeWindow: () => Promise<void>;
  maximizeWindow: () => Promise<{ success: boolean; isMaximized: boolean }>;
  isMaximized: () => Promise<{ isMaximized: boolean }>;
  quitApp: () => Promise<void>;
  onOpenFile: (callback: (filePath: string) => void) => () => void;

  // Host-backed Persistent Settings
  readSettings: () => Promise<Record<string, string>>;
  setSetting: (key: string, value: string) => Promise<{ success: boolean }>;
  setSettings: (entries: Record<string, string>) => Promise<{ success: boolean }>;
  removeSetting: (key: string) => Promise<{ success: boolean }>;
}

export interface TokHostBridge extends TokHostMethods {
  readonly kind: TokHostKind;
  readonly isDesktop: boolean;
  getBootstrap(): TokHostBootstrap;
  has(method: keyof TokHostMethods): boolean;
}

const NOOP_UNSUB = (): void => {};

export function readWindowBootstrap(): TokHostBootstrap {
  const g = typeof globalThis !== 'undefined' ? (globalThis as any) : null;
  const win = typeof window !== 'undefined' ? (window as any) : null;
  const raw = win?.__TOK_BOOTSTRAP__ ?? g?.__TOK_BOOTSTRAP__;
  if (!raw || typeof raw !== 'object') {
    return {};
  }
  const out: TokHostBootstrap = {};
  if (raw.installLanguage === 'he' || raw.installLanguage === 'en' || raw.installLanguage === null) {
    out.installLanguage = raw.installLanguage;
  }
  if (typeof raw.startupFile === 'string' || raw.startupFile === null) {
    out.startupFile = raw.startupFile;
  }
  if (raw.settings && typeof raw.settings === 'object' && !Array.isArray(raw.settings)) {
    const clean: Record<string, string> = {};
    for (const [k, v] of Object.entries(raw.settings)) {
      if (typeof k === 'string' && typeof v === 'string') {
        clean[k] = v;
      }
    }
    out.settings = clean;
  }
  if (raw.platform === 'electron' || raw.platform === 'tauri' || raw.platform === 'web') {
    out.platform = raw.platform;
  }
  return out;
}

export class ElectronHostAdapter implements TokHostBridge {
  public readonly kind: TokHostKind = 'electron';
  public readonly isDesktop = true;
  private readonly raw: Record<string, any>;

  constructor(rawIpc: Record<string, any>) {
    this.raw = rawIpc;
  }

  public getBootstrap(): TokHostBootstrap {
    return { platform: 'electron', ...readWindowBootstrap() };
  }

  public has(method: keyof TokHostMethods): boolean {
    return typeof this.raw[method as string] === 'function';
  }

  private call<T = any>(method: keyof TokHostMethods, ...args: any[]): Promise<T> {
    const fn = this.raw[method as string];
    if (typeof fn !== 'function') {
      return Promise.reject(new Error(`Electron host method unavailable: ${String(method)}`));
    }
    return Promise.resolve(fn(...args));
  }

  public sendCommand(cmd: unknown): Promise<unknown> {
    return this.call('sendCommand', cmd);
  }
  public onEvent(callback: (event: unknown) => void): () => void {
    return typeof this.raw.onEvent === 'function' ? this.raw.onEvent(callback) : NOOP_UNSUB;
  }
  public renderPdf(input: any, outputPath?: string): Promise<string> {
    return this.call('renderPdf', input, outputPath);
  }
  public renderHtml(input: any, outputPath?: string): Promise<string> {
    return this.call('renderHtml', input, outputPath);
  }
  public getRecentLogs(): Promise<string[]> {
    return this.call('getRecentLogs');
  }
  public openLogsFolder(): Promise<boolean> {
    return this.call('openLogsFolder');
  }
  public cleanOldLogs(days?: number): Promise<number> {
    return this.call('cleanOldLogs', days);
  }
  public setLogRetention(days: number): Promise<void> {
    return this.call('setLogRetention', days);
  }
  public checkForUpdates(): Promise<any> {
    return this.call('checkForUpdates');
  }
  public openReleaseUrl(url?: string): Promise<void> {
    return this.call('openReleaseUrl', url);
  }
  public downloadUpdate(downloadUrl: string) {
    return this.call('downloadUpdate', downloadUrl);
  }
  public onUpdateDownloadProgress(callback: (data: TokUpdateDownloadProgress) => void): () => void {
    return typeof this.raw.onUpdateDownloadProgress === 'function'
      ? this.raw.onUpdateDownloadProgress(callback)
      : NOOP_UNSUB;
  }
  public applyUpdate(filePath: string, silent?: boolean) {
    return this.call('applyUpdate', filePath, silent);
  }
  public inspectOfflineUpdate(filePath: string): Promise<any> {
    return this.call('inspectOfflineUpdate', filePath);
  }
  public scanUsbUpdates(): Promise<any[]> {
    return this.call('scanUsbUpdates');
  }
  public getPlugins(): Promise<unknown[]> {
    return this.call('getPlugins');
  }
  public togglePlugin(pluginId: string, enabled: boolean): Promise<boolean> {
    return this.call('togglePlugin', pluginId, enabled);
  }
  public openPluginsFolder(): Promise<void> {
    return this.call('openPluginsFolder');
  }
  public reloadPlugins(): Promise<unknown[]> {
    return this.call('reloadPlugins');
  }
  public getPluginSystemStatus() {
    return this.call('getPluginSystemStatus');
  }
  public setPluginSafeMode(enabled: boolean): Promise<boolean> {
    return this.call('setPluginSafeMode', enabled);
  }
  public saveDocument(payload: { document: unknown; filePath: string; assetsFrom?: string }) {
    return this.call('saveDocument', payload);
  }
  public openDocument(filePath: string): Promise<any> {
    return this.call('openDocument', filePath);
  }
  public renameDocumentFile(payload: { oldPath: string; newPath: string }) {
    return this.call('renameDocumentFile', payload);
  }
  public getProjectsDir(): Promise<string> {
    return this.call('getProjectsDir');
  }
  public saveDraftRecovery(snapshot: unknown) {
    return this.call('saveDraftRecovery', snapshot);
  }
  public loadDraftRecovery(): Promise<unknown | null> {
    return this.call('loadDraftRecovery');
  }
  public clearDraftRecovery() {
    return this.call('clearDraftRecovery');
  }
  public typesetDocument(payload: any) {
    return this.call('typesetDocument', payload);
  }
  public runPreflight(payload: any) {
    return this.call('runPreflight', payload);
  }
  public inspectFont(fontPath: string) {
    return this.call('inspectFont', fontPath);
  }
  public installFont(fontPath: string) {
    return this.call('installFont', fontPath);
  }
  public listUserFonts() {
    return this.call('listUserFonts');
  }
  public listSystemFonts(): Promise<string[]> {
    return this.call('listSystemFonts');
  }
  public configureHeadingNumbering(config: unknown): Promise<unknown> {
    return this.call('configureHeadingNumbering', config);
  }
  public reorderHeading(payload: { nodeId: string; newParentId?: string; targetIndex: number; config?: unknown }): Promise<unknown> {
    return this.call('reorderHeading', payload);
  }
  public insertHeading(payload: { parentId?: string; targetIndex: number; node: unknown; config?: unknown }): Promise<unknown> {
    return this.call('insertHeading', payload);
  }
  public deleteHeading(payload: { nodeId: string; config?: unknown }): Promise<unknown> {
    return this.call('deleteHeading', payload);
  }
  public recalculateHeadingNumbers(payload?: { outline?: unknown; config?: unknown }): Promise<unknown> {
    return this.call('recalculateHeadingNumbers', payload);
  }
  public openExternal(url: string): Promise<void> {
    if (typeof this.raw.openExternal === 'function') {
      return this.call('openExternal', url);
    }
    if (typeof window !== 'undefined' && typeof window.open === 'function') {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
    return Promise.resolve();
  }
  public getAppInfo(): Promise<{ version: string; name: string; repoUrl?: string }> {
    return this.call('getAppInfo');
  }
  public showSaveDialog(options?: { defaultPath?: string; filters?: { name: string; extensions: string[] }[] }): Promise<string | null> {
    return this.call('showSaveDialog', options);
  }
  public showOpenDialog(options?: { filters?: { name: string; extensions: string[] }[] }): Promise<string | null> {
    return this.call('showOpenDialog', options);
  }
  public showImportDialog(options?: { filters?: { name: string; extensions: string[] }[] }): Promise<string | null> {
    return this.call('showImportDialog', options);
  }
  public getStartupFile(): Promise<string | null> {
    return typeof this.raw.getStartupFile === 'function'
      ? this.call('getStartupFile')
      : Promise.resolve(this.getBootstrap().startupFile ?? null);
  }
  public readImportFile(filePath: string) {
    return this.call('readImportFile', filePath);
  }
  public decompressRaw(dataBase64: string): Promise<string> {
    return this.call('decompressRaw', dataBase64);
  }
  public closeWindow(): Promise<void> {
    return this.call('closeWindow');
  }
  public minimizeWindow(): Promise<void> {
    return this.call('minimizeWindow');
  }
  public maximizeWindow(): Promise<{ success: boolean; isMaximized: boolean }> {
    return this.call('maximizeWindow');
  }
  public isMaximized(): Promise<{ isMaximized: boolean }> {
    return this.call('isMaximized');
  }
  public quitApp(): Promise<void> {
    return this.call('quitApp');
  }
  public onOpenFile(callback: (filePath: string) => void): () => void {
    return typeof this.raw.onOpenFile === 'function' ? this.raw.onOpenFile(callback) : NOOP_UNSUB;
  }
  public readSettings(): Promise<Record<string, string>> {
    return this.call('readSettings');
  }
  public setSetting(key: string, value: string): Promise<{ success: boolean }> {
    return this.call('setSetting', key, value);
  }
  public setSettings(entries: Record<string, string>): Promise<{ success: boolean }> {
    return this.call('setSettings', entries);
  }
  public removeSetting(key: string): Promise<{ success: boolean }> {
    return this.call('removeSetting', key);
  }
}

export class WebPreviewAdapter implements TokHostBridge {
  public readonly kind: TokHostKind = 'web';
  public readonly isDesktop = false;

  public getBootstrap(): TokHostBootstrap {
    return { platform: 'web', ...readWindowBootstrap() };
  }

  public has(_method: keyof TokHostMethods): boolean {
    return false;
  }

  private unavailable<T = never>(method: string): Promise<T> {
    return Promise.reject(new Error(`Desktop host unavailable (${method})`));
  }

  public sendCommand(): Promise<unknown> {
    return this.unavailable('sendCommand');
  }
  public onEvent(): () => void {
    return NOOP_UNSUB;
  }
  public renderPdf(): Promise<string> {
    return this.unavailable('renderPdf');
  }
  public renderHtml(): Promise<string> {
    return this.unavailable('renderHtml');
  }
  public getRecentLogs(): Promise<string[]> {
    return Promise.resolve([]);
  }
  public openLogsFolder(): Promise<boolean> {
    return Promise.resolve(false);
  }
  public cleanOldLogs(): Promise<number> {
    return Promise.resolve(0);
  }
  public setLogRetention(): Promise<void> {
    return Promise.resolve();
  }
  public checkForUpdates(): Promise<any> {
    return this.unavailable('checkForUpdates');
  }
  public openReleaseUrl(url?: string): Promise<void> {
    if (url) return this.openExternal(url);
    return Promise.resolve();
  }
  public downloadUpdate() {
    return this.unavailable<any>('downloadUpdate');
  }
  public onUpdateDownloadProgress(): () => void {
    return NOOP_UNSUB;
  }
  public applyUpdate() {
    return this.unavailable<any>('applyUpdate');
  }
  public inspectOfflineUpdate(): Promise<any> {
    return this.unavailable('inspectOfflineUpdate');
  }
  public scanUsbUpdates(): Promise<any[]> {
    return Promise.resolve([]);
  }
  public getPlugins(): Promise<unknown[]> {
    return Promise.resolve([]);
  }
  public togglePlugin(): Promise<boolean> {
    return Promise.resolve(false);
  }
  public openPluginsFolder(): Promise<void> {
    return Promise.resolve();
  }
  public reloadPlugins(): Promise<unknown[]> {
    return Promise.resolve([]);
  }
  public getPluginSystemStatus() {
    return Promise.resolve({
      isSafeMode: false,
      isSaferActive: false,
      userPluginsDir: '',
      builtinPluginsDir: '',
    });
  }
  public setPluginSafeMode(): Promise<boolean> {
    return Promise.resolve(false);
  }
  public saveDocument() {
    return this.unavailable<any>('saveDocument');
  }
  public openDocument(): Promise<any> {
    return this.unavailable('openDocument');
  }
  public renameDocumentFile() {
    return this.unavailable<any>('renameDocumentFile');
  }
  public getProjectsDir(): Promise<string> {
    return Promise.resolve('');
  }
  public saveDraftRecovery() {
    return Promise.resolve({ success: false });
  }
  public loadDraftRecovery(): Promise<unknown | null> {
    return Promise.resolve(null);
  }
  public clearDraftRecovery() {
    return Promise.resolve({ success: false });
  }
  public typesetDocument() {
    return this.unavailable<any>('typesetDocument');
  }
  public runPreflight() {
    return this.unavailable<any>('runPreflight');
  }
  public inspectFont() {
    return this.unavailable<any>('inspectFont');
  }
  public installFont() {
    return this.unavailable<any>('installFont');
  }
  public listUserFonts() {
    return Promise.resolve([]);
  }
  public listSystemFonts(): Promise<string[]> {
    return Promise.resolve([]);
  }
  public configureHeadingNumbering(): Promise<unknown> {
    return this.unavailable('configureHeadingNumbering');
  }
  public reorderHeading(): Promise<unknown> {
    return this.unavailable('reorderHeading');
  }
  public insertHeading(): Promise<unknown> {
    return this.unavailable('insertHeading');
  }
  public deleteHeading(): Promise<unknown> {
    return this.unavailable('deleteHeading');
  }
  public recalculateHeadingNumbers(): Promise<unknown> {
    return this.unavailable('recalculateHeadingNumbers');
  }
  public openExternal(url: string): Promise<void> {
    if (typeof window !== 'undefined' && typeof window.open === 'function') {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
    return Promise.resolve();
  }
  public getAppInfo(): Promise<{ version: string; name: string; repoUrl?: string }> {
    return Promise.resolve({ version: '', name: 'TypesetOK (TOK)' });
  }
  public showSaveDialog(): Promise<string | null> {
    return Promise.resolve(null);
  }
  public showOpenDialog(): Promise<string | null> {
    return Promise.resolve(null);
  }
  public showImportDialog(): Promise<string | null> {
    return Promise.resolve(null);
  }
  public getStartupFile(): Promise<string | null> {
    return Promise.resolve(this.getBootstrap().startupFile ?? null);
  }
  public readImportFile() {
    return this.unavailable<any>('readImportFile');
  }
  public decompressRaw(): Promise<string> {
    return this.unavailable('decompressRaw');
  }
  public closeWindow(): Promise<void> {
    return Promise.resolve();
  }
  public minimizeWindow(): Promise<void> {
    return Promise.resolve();
  }
  public maximizeWindow(): Promise<{ success: boolean; isMaximized: boolean }> {
    return Promise.resolve({ success: false, isMaximized: false });
  }
  public isMaximized(): Promise<{ isMaximized: boolean }> {
    return Promise.resolve({ isMaximized: false });
  }
  public quitApp(): Promise<void> {
    return Promise.resolve();
  }
  public onOpenFile(): () => void {
    return NOOP_UNSUB;
  }
  public readSettings(): Promise<Record<string, string>> {
    return Promise.resolve({});
  }
  public setSetting(): Promise<{ success: boolean }> {
    return Promise.resolve({ success: false });
  }
  public setSettings(): Promise<{ success: boolean }> {
    return Promise.resolve({ success: false });
  }
  public removeSetting(): Promise<{ success: boolean }> {
    return Promise.resolve({ success: false });
  }
}

let explicitBridge: TokHostBridge | null = null;
let cachedElectronRaw: unknown = null;
let cachedElectronAdapter: ElectronHostAdapter | null = null;
const webPreviewAdapter = new WebPreviewAdapter();

/** Injects a custom host bridge (e.g. TauriHostAdapter or test stub). Pass `null` to restore auto-detection. */
export function setHostBridge(bridge: TokHostBridge | null): void {
  explicitBridge = bridge;
}

/** Resolves the active host bridge for the current runtime environment. */
export function getHostBridge(): TokHostBridge {
  if (explicitBridge) {
    return explicitBridge;
  }

  const win = typeof window !== 'undefined' ? (window as any) : undefined;
  const g = typeof globalThis !== 'undefined' ? (globalThis as any) : undefined;
  const raw = win?.tokIpc ?? g?.tokIpc;

  if (raw && typeof raw === 'object') {
    if (raw !== cachedElectronRaw || !cachedElectronAdapter) {
      cachedElectronRaw = raw;
      cachedElectronAdapter = new ElectronHostAdapter(raw);
    }
    return cachedElectronAdapter;
  }

  cachedElectronRaw = null;
  cachedElectronAdapter = null;
  return webPreviewAdapter;
}

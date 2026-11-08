import { contextBridge, ipcRenderer } from 'electron';

// First run after install: adopt the language picked in the installer. A language the user already
// chose inside the app (saved under tok_lang) always wins. Runs before the page, so the UI starts correctly.
try {
  if (!localStorage.getItem('tok_lang')) {
    const installLang = ipcRenderer.sendSync('tok:get-install-language');
    if (installLang === 'he' || installLang === 'en') localStorage.setItem('tok_lang', installLang);
  }
} catch {
  // storage unavailable: the UI falls back to its default language
}

export interface TokIpcBridge {
  sendCommand: (cmd: unknown) => Promise<unknown>;
  onEvent: (callback: (event: unknown) => void) => () => void;
  renderPdf: (input: string | { inputPath?: string; document?: unknown; outputPath?: string }, outputPath?: string) => Promise<string>;
  renderHtml: (input: string | { inputPath?: string; document?: unknown; outputPath?: string }, outputPath?: string) => Promise<string>;

  // Logger
  getRecentLogs: () => Promise<string[]>;
  openLogsFolder: () => Promise<boolean>;
  cleanOldLogs: (days?: number) => Promise<number>;
  setLogRetention: (days: number) => Promise<void>;

  // Updater
  checkForUpdates: () => Promise<unknown>;
  openReleaseUrl: (url?: string) => Promise<void>;
  downloadUpdate: (downloadUrl: string) => Promise<{ success: boolean; filePath?: string; fileSize?: number; sha256?: string; error?: string }>;
  onUpdateDownloadProgress: (callback: (data: { percent: number; transferredBytes: number; totalBytes: number; speedBytesPerSec: number; etaSeconds: number }) => void) => () => void;
  applyUpdate: (filePath: string, silent?: boolean) => Promise<{ success: boolean; error?: string }>;
  inspectOfflineUpdate: (filePath: string) => Promise<any>;
  scanUsbUpdates: () => Promise<any[]>;

  // Plugins
  getPlugins: () => Promise<unknown[]>;
  togglePlugin: (pluginId: string, enabled: boolean) => Promise<boolean>;
  openPluginsFolder: () => Promise<void>;
  reloadPlugins: () => Promise<unknown[]>;
  getPluginSystemStatus: () => Promise<{ isSafeMode: boolean; isSaferActive: boolean; userPluginsDir: string; builtinPluginsDir: string }>;
  setPluginSafeMode: (enabled: boolean) => Promise<boolean>;

  // Document Storage & Typesetting
  saveDocument: (payload: { document: unknown; filePath: string }) => Promise<{ success: boolean; filePath: string }>;
  openDocument: (filePath: string) => Promise<unknown>;
  renameDocumentFile: (payload: { oldPath: string; newPath: string }) => Promise<{ success: boolean; filePath?: string; error?: string }>;
  getProjectsDir: () => Promise<string>;
  typesetDocument: (payload: { document?: unknown; inputPath?: string } | unknown) => Promise<{ success: boolean; pages: any[] }>;

  // Heading & Outline Numbering
  configureHeadingNumbering: (config: unknown) => Promise<unknown>;
  reorderHeading: (payload: { nodeId: string; newParentId?: string; targetIndex: number; config?: unknown }) => Promise<unknown>;
  insertHeading: (payload: { parentId?: string; targetIndex: number; node: unknown; config?: unknown }) => Promise<unknown>;
  deleteHeading: (payload: { nodeId: string; config?: unknown }) => Promise<unknown>;
  recalculateHeadingNumbers: (payload?: { outline?: unknown; config?: unknown }) => Promise<unknown>;

  // System & Utilities
  openExternal: (url: string) => Promise<void>;
  getAppInfo: () => Promise<{ version: string; name: string }>;
  showSaveDialog: (options?: { defaultPath?: string; filters?: { name: string; extensions: string[] }[] }) => Promise<string | null>;
  showOpenDialog: (options?: { filters?: { name: string; extensions: string[] }[] }) => Promise<string | null>;
  showImportDialog: (options?: { filters?: { name: string; extensions: string[] }[] }) => Promise<string | null>;
  getStartupFile: () => Promise<string | null>;
  getStartupFileSync: () => string | null;
  readImportFile: (filePath: string) => Promise<{
    fileName: string;
    filePath: string;
    size: number;
    isBinary: boolean;
    docxXml?: string;
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
}

const tokIpc: TokIpcBridge = {
  sendCommand: async (cmd: unknown) => {
    return await ipcRenderer.invoke('tok:send-command', cmd);
  },
  onEvent: (callback: (event: unknown) => void) => {
    const eventHandler = (_: Electron.IpcRendererEvent, payload: unknown) => callback(payload);
    const menuHandler = (_: Electron.IpcRendererEvent, action: string, data?: unknown) => {
      callback({ action, data });
    };

    ipcRenderer.on('tok:event', eventHandler);
    ipcRenderer.on('menu:action', menuHandler);

    return () => {
      ipcRenderer.removeListener('tok:event', eventHandler);
      ipcRenderer.removeListener('menu:action', menuHandler);
    };
  },
  renderPdf: async (input: string | { inputPath?: string; document?: unknown; outputPath?: string }, outputPath?: string) => {
    const payload = typeof input === 'string'
      ? { inputPath: input, outputPath }
      : { ...input, outputPath: outputPath || input?.outputPath };
    return await ipcRenderer.invoke('tok:render-pdf', payload);
  },
  renderHtml: async (input: string | { inputPath?: string; document?: unknown; outputPath?: string }, outputPath?: string) => {
    const payload = typeof input === 'string'
      ? { inputPath: input, outputPath }
      : { ...input, outputPath: outputPath || input?.outputPath };
    return await ipcRenderer.invoke('tok:render-html', payload);
  },

  // Logger APIs
  getRecentLogs: async () => {
    return await ipcRenderer.invoke('tok:get-recent-logs');
  },
  openLogsFolder: async () => {
    return await ipcRenderer.invoke('tok:open-logs-folder');
  },
  cleanOldLogs: async (days?: number) => {
    return await ipcRenderer.invoke('tok:clean-old-logs', days);
  },
  setLogRetention: async (days: number) => {
    return await ipcRenderer.invoke('tok:set-log-retention', days);
  },

  // Updater APIs
  checkForUpdates: async () => {
    return await ipcRenderer.invoke('tok:check-for-updates');
  },
  openReleaseUrl: async (url?: string) => {
    return await ipcRenderer.invoke('tok:open-release-url', url);
  },
  downloadUpdate: async (url: string) => {
    return await ipcRenderer.invoke('tok:download-update', url);
  },
  applyUpdate: async (filePath: string, silent?: boolean) => {
    return await ipcRenderer.invoke('tok:apply-update', filePath, silent);
  },
  inspectOfflineUpdate: async (filePath: string) => {
    return await ipcRenderer.invoke('tok:inspect-offline-update', filePath);
  },
  scanUsbUpdates: async () => {
    return await ipcRenderer.invoke('tok:scan-usb-updates');
  },
  onUpdateDownloadProgress: (callback: (data: any) => void) => {
    const handler = (_: Electron.IpcRendererEvent, data: any) => callback(data);
    ipcRenderer.on('tok:update-download-progress', handler);
    return () => ipcRenderer.removeListener('tok:update-download-progress', handler);
  },

  // Plugin APIs
  getPlugins: async () => {
    return await ipcRenderer.invoke('tok:get-plugins');
  },
  togglePlugin: async (pluginId: string, enabled: boolean) => {
    return await ipcRenderer.invoke('tok:toggle-plugin', { pluginId, enabled });
  },
  openPluginsFolder: async () => {
    return await ipcRenderer.invoke('tok:open-plugins-folder');
  },
  reloadPlugins: async () => {
    return await ipcRenderer.invoke('tok:reload-plugins');
  },
  getPluginSystemStatus: async () => {
    return await ipcRenderer.invoke('tok:get-plugin-system-status');
  },
  setPluginSafeMode: async (enabled: boolean) => {
    return await ipcRenderer.invoke('tok:set-plugin-safe-mode', enabled);
  },

  // Document Storage & Typesetting
  saveDocument: async (payload: { document: unknown; filePath: string }) => {
    return await ipcRenderer.invoke('tok:save-document', payload);
  },
  openDocument: async (filePath: string) => {
    return await ipcRenderer.invoke('tok:open-document', filePath);
  },
  renameDocumentFile: async (payload: { oldPath: string; newPath: string }) => {
    return await ipcRenderer.invoke('tok:rename-document-file', payload);
  },
  getProjectsDir: async () => {
    return await ipcRenderer.invoke('tok:get-projects-dir');
  },
  typesetDocument: async (payload: { document?: unknown; inputPath?: string } | unknown) => {
    const normPayload = (payload && typeof payload === 'object' && ('document' in payload || 'inputPath' in payload))
      ? payload
      : { document: payload };
    return await ipcRenderer.invoke('tok:typeset-document', normPayload);
  },

  // System
  openExternal: async (url: string) => {
    return await ipcRenderer.invoke('tok:open-external', url);
  },
  getAppInfo: async () => {
    return await ipcRenderer.invoke('tok:get-app-info');
  },
  showSaveDialog: async (options?: { defaultPath?: string; filters?: { name: string; extensions: string[] }[] }) => {
    return await ipcRenderer.invoke('tok:show-save-dialog', options);
  },
  showOpenDialog: async (options?: { filters?: { name: string; extensions: string[] }[] }) => {
    return await ipcRenderer.invoke('tok:show-open-dialog', options);
  },
  showImportDialog: async (options?: { filters?: { name: string; extensions: string[] }[] }) => {
    return await ipcRenderer.invoke('tok:show-import-dialog', options);
  },
  getStartupFile: async () => {
    return await ipcRenderer.invoke('tok:get-startup-file');
  },
  getStartupFileSync: () => {
    try {
      return ipcRenderer.sendSync('tok:get-startup-file-sync') || null;
    } catch {
      return null;
    }
  },
  // Heading & Outline Numbering
  configureHeadingNumbering: async (config: unknown) => {
    return await ipcRenderer.invoke('tok:configure-heading-numbering', config);
  },
  reorderHeading: async (payload: { nodeId: string; newParentId?: string; targetIndex: number; config?: unknown }) => {
    return await ipcRenderer.invoke('tok:reorder-heading', payload);
  },
  insertHeading: async (payload: { parentId?: string; targetIndex: number; node: unknown; config?: unknown }) => {
    return await ipcRenderer.invoke('tok:insert-heading', payload);
  },
  deleteHeading: async (payload: { nodeId: string; config?: unknown }) => {
    return await ipcRenderer.invoke('tok:delete-heading', payload);
  },
  recalculateHeadingNumbers: async (payload?: { outline?: unknown; config?: unknown }) => {
    return await ipcRenderer.invoke('tok:recalculate-heading-numbers', payload);
  },

  readImportFile: async (filePath: string) => {
    return await ipcRenderer.invoke('tok:read-import-file', filePath);
  },
  decompressRaw: async (dataBase64: string) => {
    return await ipcRenderer.invoke('tok:decompress-raw', dataBase64);
  },
  closeWindow: async () => {
    return await ipcRenderer.invoke('tok:close-window');
  },
  minimizeWindow: async () => {
    return await ipcRenderer.invoke('tok:minimize-window');
  },
  maximizeWindow: async () => {
    return await ipcRenderer.invoke('tok:maximize-window');
  },
  isMaximized: async () => {
    return await ipcRenderer.invoke('tok:is-maximized');
  },
  quitApp: async () => {
    return await ipcRenderer.invoke('tok:quit-app');
  },
  onOpenFile: (callback: (filePath: string) => void) => {
    const handler = (_: Electron.IpcRendererEvent, filePath: string) => callback(filePath);
    ipcRenderer.on('tok:open-file', handler);
    return () => ipcRenderer.removeListener('tok:open-file', handler);
  }
};

contextBridge.exposeInMainWorld('tokIpc', tokIpc);

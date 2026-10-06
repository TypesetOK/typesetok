import { contextBridge, ipcRenderer } from 'electron';

export interface TokIpcBridge {
  sendCommand: (cmd: unknown) => Promise<unknown>;
  onEvent: (callback: (event: unknown) => void) => () => void;
  renderPdf: (inputPath: string, outputPath: string) => Promise<string>;
  renderHtml: (inputPath: string, outputPath: string) => Promise<string>;

  // Logger
  getRecentLogs: () => Promise<string[]>;
  openLogsFolder: () => Promise<boolean>;
  cleanOldLogs: (days?: number) => Promise<number>;
  setLogRetention: (days: number) => Promise<void>;

  // Updater
  checkForUpdates: () => Promise<unknown>;
  openReleaseUrl: (url?: string) => Promise<void>;

  // Plugins
  getPlugins: () => Promise<unknown[]>;
  togglePlugin: (pluginId: string, enabled: boolean) => Promise<boolean>;
  openPluginsFolder: () => Promise<void>;
  reloadPlugins: () => Promise<unknown[]>;
  getPluginSystemStatus: () => Promise<{ isSafeMode: boolean; isSaferActive: boolean; userPluginsDir: string; builtinPluginsDir: string }>;
  setPluginSafeMode: (enabled: boolean) => Promise<boolean>;

  // System & Utilities
  openExternal: (url: string) => Promise<void>;
  getAppInfo: () => Promise<{ version: string; name: string }>;
  showSaveDialog: (options?: { defaultPath?: string; filters?: { name: string; extensions: string[] }[] }) => Promise<string | null>;
  showOpenDialog: (options?: { filters?: { name: string; extensions: string[] }[] }) => Promise<string | null>;
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
  renderPdf: async (inputPath: string, outputPath: string) => {
    return await ipcRenderer.invoke('tok:render-pdf', { inputPath, outputPath });
  },
  renderHtml: async (inputPath: string, outputPath: string) => {
    return await ipcRenderer.invoke('tok:render-html', { inputPath, outputPath });
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
  }
};

contextBridge.exposeInMainWorld('tokIpc', tokIpc);

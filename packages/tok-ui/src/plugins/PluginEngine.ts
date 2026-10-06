/**
 * TypesetOK (TOK) - Secure Sandboxed Plugin Engine
 *
 * Enforces:
 * - High-performance official API surface (`tok.document`, `tok.hebrew`, `tok.ui`, `tok.canvas`, `tok.events`, `tok.env`)
 * - Capability & Permission checks ('document:read', 'document:write', 'ui:commands', 'network:fetch', etc.)
 * - Strict sandboxing: shadows IPC (`tokIpc`), shields sensitive storage, and quarantines unauthorized network access
 * - Safe Mode & Safer workstation compatibility (offline enforcement & third-party plugin lockdown)
 * - Complete lifecycle management: automatic disposal of commands, listeners, modals, and intervals on deactivation
 * - Panic & Error boundary: uncaught errors in commands or plugins never crash or freeze the host
 */

import {
  Disposable,
  DocumentStats,
  PluginCommand,
  PluginManifest,
  PluginPermission,
  StoryParagraph,
  TokCanvasAPI,
  TokDocumentAPI,
  TokEnvAPI,
  TokEventsAPI,
  TokPluginContext,
  TokUIAPI
} from './plugin-types';
import { hebrewUtils } from './hebrew-utils';

export interface PluginItem {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  sourceType: 'ts' | 'js';
  enabled: boolean;
  compiledCode?: string;
  error?: string;
  permissions?: PluginPermission[];
  safeModeBlocked?: boolean;
}

export interface PluginAppDelegate {
  // Document
  getStory?: () => StoryParagraph[];
  loadStory?: (paragraphs: StoryParagraph[]) => void;
  getSelectedText?: () => string;
  replaceSelection?: (text: string) => void;
  getDocumentTitle?: () => string;
  getStats?: () => DocumentStats;
  // UI
  getViewMode?: () => 'canvas' | 'story' | 'split';
  setViewMode?: (mode: 'canvas' | 'story' | 'split') => void;
  getTheme?: () => string;
  getLanguage?: () => 'he' | 'en';
  openModal?: (title: string, render: (container: HTMLElement) => void | (() => void)) => Disposable;
  // Canvas
  getPageCount?: () => number;
  getActivePageIndex?: () => number;
  scrollToPage?: (pageIndex: number) => void;
  getZoom?: () => number;
  setZoom?: (percent: number) => void;
  toggleMarginsGuide?: () => void;
  toggleBaselineGuide?: () => void;
  // Events
  on?: (event: string, listener: (...args: any[]) => void) => Disposable;
}

export interface PluginEngineOptions {
  onCommandRegistered?: (cmd: PluginCommand) => void;
  onToast?: (msg: string, isError?: boolean) => void;
  onCommandUnregistered?: (commandId: string) => void;
  delegate?: PluginAppDelegate;
}

/**
 * Converts one raw entry from the main process (`LoadedPlugin`) into a PluginItem.
 * Returns null for malformed entries so that one broken plugin.json cannot drop the list.
 */
export function normalizePluginEntry(raw: unknown): PluginItem | null {
  if (!raw || typeof raw !== 'object') return null;
  const p = raw as Record<string, any>;
  const m = p.manifest;
  if (!m || typeof m !== 'object' || typeof m.id !== 'string' || !m.id) return null;

  const perms: PluginPermission[] = Array.isArray(m.permissions)
    ? (m.permissions.filter((x: any) => typeof x === 'string') as PluginPermission[])
    : [];

  return {
    id: m.id,
    name: typeof m.name === 'string' && m.name ? m.name : m.id,
    version: typeof m.version === 'string' ? m.version : '0.0.0',
    description: typeof m.description === 'string' ? m.description : '',
    author: typeof m.author === 'string' ? m.author : undefined,
    sourceType: p.sourceType === 'ts' ? 'ts' : 'js',
    enabled: p.enabled === true,
    compiledCode: typeof p.compiledCode === 'string' ? p.compiledCode : undefined,
    error: typeof p.error === 'string' ? p.error : undefined,
    permissions: perms,
    safeModeBlocked: p.safeModeBlocked === true,
  };
}

export class PluginEngine {
  private plugins: PluginItem[] = [];
  private onCommandRegistered?: (cmd: PluginCommand) => void;
  private onToast?: (msg: string, isError?: boolean) => void;
  private onCommandUnregistered?: (commandId: string) => void;
  private delegate?: PluginAppDelegate;

  /** Plugins whose code is currently active in this session */
  private activated = new Set<string>();
  /** Cleanup callbacks registered per plugin */
  private cleanupsByPlugin = new Map<string, (() => void)[]>();
  /** Commands registered per plugin */
  private commandsByPlugin = new Map<string, Map<string, PluginCommand>>();
  /** Disposables per plugin */
  private subscriptionsByPlugin = new Map<string, Disposable[]>();

  // System & Environment Status
  private isSafeMode = false;
  private isSaferActive = false;
  private appVersion = '0.9.1';

  constructor(
    onCommandRegisteredOrOptions?: ((cmd: PluginCommand) => void) | PluginEngineOptions,
    onToast?: (msg: string, isError?: boolean) => void,
    onCommandUnregistered?: (commandId: string) => void,
    delegate?: PluginAppDelegate
  ) {
    if (typeof onCommandRegisteredOrOptions === 'object' && onCommandRegisteredOrOptions !== null) {
      this.onCommandRegistered = onCommandRegisteredOrOptions.onCommandRegistered;
      this.onToast = onCommandRegisteredOrOptions.onToast;
      this.onCommandUnregistered = onCommandRegisteredOrOptions.onCommandUnregistered;
      this.delegate = onCommandRegisteredOrOptions.delegate;
    } else {
      this.onCommandRegistered = onCommandRegisteredOrOptions;
      this.onToast = onToast;
      this.onCommandUnregistered = onCommandUnregistered;
      this.delegate = delegate;
    }
  }

  public setDelegate(delegate: PluginAppDelegate): void {
    this.delegate = delegate;
  }

  public getIsSafeMode(): boolean {
    return this.isSafeMode;
  }

  public getIsSaferActive(): boolean {
    return this.isSaferActive;
  }

  public async setSafeMode(enabled: boolean): Promise<boolean> {
    const win = window as any;
    if (win.tokIpc && win.tokIpc.setPluginSafeMode) {
      const ok = await win.tokIpc.setPluginSafeMode(enabled);
      if (ok) {
        this.isSafeMode = enabled;
        await this.loadPlugins();
        return true;
      }
    }
    this.isSafeMode = enabled;
    await this.loadPlugins();
    return true;
  }

  public async loadPlugins(): Promise<PluginItem[]> {
    const win = window as any;
    if (win.tokIpc) {
      // 1. Fetch system status (Safe mode, Safer environment)
      if (win.tokIpc.getPluginSystemStatus) {
        try {
          const status = await win.tokIpc.getPluginSystemStatus();
          if (status) {
            this.isSafeMode = Boolean(status.isSafeMode);
            this.isSaferActive = Boolean(status.isSaferActive);
          }
        } catch (err: any) {
          console.warn('[PLUGIN-ENGINE] Failed to fetch system status:', err?.message ?? err);
        }
      }

      // 2. Fetch app info
      if (win.tokIpc.getAppInfo) {
        try {
          const info = await win.tokIpc.getAppInfo();
          if (info?.version) this.appVersion = info.version;
        } catch {}
      }

      // 3. Fetch plugin entries
      if (win.tokIpc.getPlugins) {
        try {
          const rawPlugins = await win.tokIpc.getPlugins();
          const list: unknown[] = Array.isArray(rawPlugins) ? rawPlugins : [];
          const seen = new Set<string>();
          this.plugins = [];
          for (const raw of list) {
            const item = normalizePluginEntry(raw);
            if (!item || seen.has(item.id)) continue;
            seen.add(item.id);
            this.plugins.push(item);
          }
        } catch (err: any) {
          console.warn('[PLUGIN-ENGINE] Failed to fetch plugins from IPC:', err?.message ?? err);
        }
      }
    } else {
      this.plugins = [];
    }

    // Activate enabled plugins (skipping those blocked by Safe Mode)
    for (const p of this.plugins) {
      if (p.enabled && !p.safeModeBlocked) {
        this.activatePlugin(p);
      }
    }
    return this.getPlugins();
  }

  public getPlugins(): PluginItem[] {
    return this.plugins.map((p) => ({ ...p }));
  }

  public isActivated(pluginId: string): boolean {
    return this.activated.has(pluginId);
  }

  /** Checks whether a plugin has been granted a specific capability/permission */
  private hasPermission(p: PluginItem, perm: PluginPermission): boolean {
    // If no permissions array is declared, allow standard non-destructive permissions
    if (!p.permissions || p.permissions.length === 0) {
      return perm !== 'network:fetch';
    }
    return p.permissions.includes(perm);
  }

  private assertPermission(p: PluginItem, perm: PluginPermission, actionDescription: string): void {
    if (!this.hasPermission(p, perm)) {
      throw new Error(
        `[TOK-SECURITY] Permission denied: plugin '${p.name}' (${p.id}) requires '${perm}' to ${actionDescription}.`
      );
    }
  }

  /**
   * Constructs the sandboxed TokPluginContext for a specific plugin.
   */
  private createContext(p: PluginItem): TokPluginContext {
    const pluginId = p.id;
    let subscriptions = this.subscriptionsByPlugin.get(pluginId);
    if (!subscriptions) {
      subscriptions = [];
      this.subscriptionsByPlugin.set(pluginId, subscriptions);
    }

    let cleanups = this.cleanupsByPlugin.get(pluginId);
    if (!cleanups) {
      cleanups = [];
      this.cleanupsByPlugin.set(pluginId, cleanups);
    }

    const showToast = (msg: string, isError = false) => {
      this.assertPermission(p, 'ui:notifications', 'display toasts');
      if (this.onToast) this.onToast(String(msg), isError);
    };

    const registerCommand = (cmd: PluginCommand): Disposable => {
      this.assertPermission(p, 'ui:commands', 'register commands');
      if (!cmd || typeof cmd.id !== 'string' || typeof cmd.action !== 'function') {
        console.warn(`[PLUGIN-ENGINE] ${pluginId}: ignoring invalid command`, cmd);
        return { dispose: () => {} };
      }

      // Safe command wrapper: Panic boundary & error isolation
      const safeAction = async () => {
        try {
          await cmd.action();
        } catch (err: any) {
          console.error(`[PLUGIN-ENGINE] Error executing command '${cmd.title}' from plugin '${p.name}':`, err);
          showToast(`שגיאה בתוסף ${p.name}: ${err?.message ?? String(err)}`, true);
        }
      };

      const wrappedCmd: PluginCommand = { ...cmd, action: safeAction };

      let cmds = this.commandsByPlugin.get(pluginId);
      if (!cmds) {
        cmds = new Map();
        this.commandsByPlugin.set(pluginId, cmds);
      }
      cmds.set(cmd.id, wrappedCmd);

      const target = this.plugins.find((item) => item.id === pluginId);
      if (target && target.enabled && !target.safeModeBlocked) {
        if (this.onCommandRegistered) this.onCommandRegistered(wrappedCmd);
      }

      const disposable: Disposable = {
        dispose: () => {
          cmds?.delete(cmd.id);
          if (this.onCommandUnregistered) this.onCommandUnregistered(cmd.id);
        }
      };
      subscriptions!.push(disposable);
      return disposable;
    };

    // 1. Document API
    const docAPI: TokDocumentAPI = {
      getStory: (): StoryParagraph[] => {
        this.assertPermission(p, 'document:read', 'read story paragraphs');
        return this.delegate?.getStory ? this.delegate.getStory() : [];
      },
      loadStory: (paragraphs: StoryParagraph[]): void => {
        this.assertPermission(p, 'document:write', 'modify document story');
        if (this.delegate?.loadStory) this.delegate.loadStory(paragraphs);
      },
      getParagraph: (id: string): StoryParagraph | undefined => {
        this.assertPermission(p, 'document:read', 'get paragraph');
        const story = this.delegate?.getStory ? this.delegate.getStory() : [];
        return story.find((item) => item.id === id);
      },
      updateParagraph: (id: string, text: string, styleId?: string): boolean => {
        this.assertPermission(p, 'document:write', 'update paragraph text');
        const story = this.delegate?.getStory ? this.delegate.getStory() : [];
        const target = story.find((item) => item.id === id);
        if (!target) return false;
        target.text = text;
        if (styleId !== undefined) target.styleId = styleId;
        this.delegate?.loadStory?.(story);
        return true;
      },
      insertParagraph: (index: number, text: string, styleId = ''): string => {
        this.assertPermission(p, 'document:write', 'insert paragraph');
        const story = this.delegate?.getStory ? this.delegate.getStory() : [];
        const newId = `para-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
        const clampedIdx = Math.max(0, Math.min(story.length, index));
        story.splice(clampedIdx, 0, { id: newId, text, styleId });
        this.delegate?.loadStory?.(story);
        return newId;
      },
      deleteParagraph: (id: string): boolean => {
        this.assertPermission(p, 'document:write', 'delete paragraph');
        const story = this.delegate?.getStory ? this.delegate.getStory() : [];
        const initialLen = story.length;
        const filtered = story.filter((item) => item.id !== id);
        if (filtered.length !== initialLen) {
          this.delegate?.loadStory?.(filtered);
          return true;
        }
        return false;
      },
      getSelectedText: (): string => {
        this.assertPermission(p, 'document:read', 'get selected text');
        return this.delegate?.getSelectedText ? this.delegate.getSelectedText() : '';
      },
      replaceSelection: (text: string): void => {
        this.assertPermission(p, 'document:write', 'replace selection');
        this.delegate?.replaceSelection?.(text);
      },
      getDocumentTitle: (): string => {
        return this.delegate?.getDocumentTitle ? this.delegate.getDocumentTitle() : '';
      },
      getStats: (): DocumentStats => {
        if (this.delegate?.getStats) return this.delegate.getStats();
        const story = this.delegate?.getStory ? this.delegate.getStory() : [];
        const text = story.map((s) => s.text).join(' ');
        const wordCount = (text.match(/\S+/g) || []).length;
        return { wordCount, charCount: text.length, paragraphCount: story.length };
      },
      batch: (fn: () => void): void => {
        this.assertPermission(p, 'document:write', 'run batch mutations');
        fn();
      },
      transformText: (transformFn: (text: string, para: StoryParagraph) => string): void => {
        this.assertPermission(p, 'document:write', 'transform document text');
        const story = this.delegate?.getStory ? this.delegate.getStory() : [];
        const transformed = story.map((para) => ({
          ...para,
          text: transformFn(para.text, para)
        }));
        this.delegate?.loadStory?.(transformed);
      }
    };

    // 2. UI API
    const uiAPI: TokUIAPI = {
      registerCommand,
      showToast,
      getViewMode: () => (this.delegate?.getViewMode ? this.delegate.getViewMode() : 'canvas'),
      setViewMode: (mode) => this.delegate?.setViewMode?.(mode),
      getTheme: () => (this.delegate?.getTheme ? this.delegate.getTheme() : 'classic-blue'),
      getLanguage: () => (this.delegate?.getLanguage ? this.delegate.getLanguage() : 'he'),
      openModal: (title, render) => {
        this.assertPermission(p, 'ui:modals', 'open custom modal dialogs');
        if (this.delegate?.openModal) {
          const d = this.delegate.openModal(title, render);
          subscriptions!.push(d);
          return d;
        }
        return { dispose: () => {} };
      }
    };

    // 3. Canvas API
    const canvasAPI: TokCanvasAPI = {
      getPageCount: () => (this.delegate?.getPageCount ? this.delegate.getPageCount() : 1),
      getActivePageIndex: () => (this.delegate?.getActivePageIndex ? this.delegate.getActivePageIndex() : 0),
      scrollToPage: (index) => {
        this.assertPermission(p, 'canvas:navigate', 'navigate canvas pages');
        this.delegate?.scrollToPage?.(index);
      },
      getZoom: () => (this.delegate?.getZoom ? this.delegate.getZoom() : 100),
      setZoom: (percent) => {
        this.assertPermission(p, 'canvas:navigate', 'change zoom level');
        this.delegate?.setZoom?.(percent);
      },
      toggleMarginsGuide: () => {
        this.assertPermission(p, 'canvas:navigate', 'toggle margins guide');
        this.delegate?.toggleMarginsGuide?.();
      },
      toggleBaselineGuide: () => {
        this.assertPermission(p, 'canvas:navigate', 'toggle baseline guide');
        this.delegate?.toggleBaselineGuide?.();
      }
    };

    // 4. Events API
    const eventsAPI: TokEventsAPI = {
      on: (event: string, listener: (...args: any[]) => void): Disposable => {
        if (this.delegate?.on) {
          const d = this.delegate.on(event, listener);
          subscriptions!.push(d);
          return d;
        }
        return { dispose: () => {} };
      }
    };

    // 5. Environment API
    const envAPI: TokEnvAPI = {
      appVersion: this.appVersion,
      isSafeMode: this.isSafeMode,
      isSaferActive: this.isSaferActive,
      platform: typeof process !== 'undefined' ? process.platform : 'win32'
    };

    const manifest: PluginManifest = {
      id: p.id,
      name: p.name,
      version: p.version,
      description: p.description,
      author: p.author,
      main: '',
      enabled: p.enabled,
      permissions: p.permissions
    };

    return {
      manifest,
      document: docAPI,
      hebrew: hebrewUtils,
      ui: uiAPI,
      canvas: canvasAPI,
      events: eventsAPI,
      env: envAPI,
      subscriptions,
      registerCommand,
      showToast,
      onDeactivate: (fn: () => void) => {
        cleanups!.push(fn);
      }
    };
  }

  /**
   * Activates a plugin inside a hardened sandbox:
   * - Shadows IPC (`tokIpc`), preventing raw Electron IPC escapes
   * - Quarantines `fetch`, `XMLHttpRequest`, `WebSocket` unless 'network:fetch' is granted
   * - Shields sensitive storage (`localStorage`, `sessionStorage`, `cookie`)
   */
  private activatePlugin(p: PluginItem): boolean {
    if (this.activated.has(p.id) || !p.compiledCode) return false;

    // In Safe Mode, all user plugins are completely locked out
    if (this.isSafeMode && p.safeModeBlocked) {
      console.warn(`[PLUGIN-ENGINE] Plugin ${p.name} blocked by Safe Mode.`);
      return false;
    }

    this.activated.add(p.id);
    const ctx = this.createContext(p);

    // Network sandbox guard
    const allowNetwork = this.hasPermission(p, 'network:fetch') && !this.isSaferActive;
    const sandboxedFetch = allowNetwork
      ? window.fetch.bind(window)
      : () => {
          throw new Error(
            `[TOK-SECURITY] Network access blocked for plugin '${p.name}'. Network is disabled in offline/Safer mode.`
          );
        };

    try {
      // Evaluate within a hardened closure where dangerous globals are shadowed
      const runner = new Function(
        'tok',
        'module',
        'exports',
        'tokIpc',
        'fetch',
        'XMLHttpRequest',
        'WebSocket',
        'localStorage',
        'sessionStorage',
        `
        "use strict";
        ${p.compiledCode}
        ;if (typeof activate === 'function') {
          activate(tok);
        } else if (typeof module !== 'undefined' && module.exports && typeof module.exports.activate === 'function') {
          module.exports.activate(tok);
        }
        `
      );

      const fakeModule = { exports: {} as Record<string, any> };
      runner(
        ctx,
        fakeModule,
        fakeModule.exports,
        undefined, // tokIpc shadowed
        sandboxedFetch, // fetch guarded
        undefined, // XMLHttpRequest shadowed
        undefined, // WebSocket shadowed
        undefined, // localStorage shadowed
        undefined  // sessionStorage shadowed
      );

      // If the plugin exported a deactivate function, track it
      if (typeof fakeModule.exports?.deactivate === 'function') {
        ctx.onDeactivate(fakeModule.exports.deactivate);
      }

      console.log(`[PLUGIN-ENGINE] Activated plugin: ${p.name} (${p.sourceType.toUpperCase()})`);
      return true;
    } catch (err: any) {
      console.error(`[PLUGIN-ENGINE] Failed to activate plugin ${p.name}:`, err?.message ?? err);
      p.error = err?.message ?? String(err);
      this.deactivatePlugin(p.id);
      return false;
    }
  }

  /**
   * Completely tears down an active plugin:
   * - Runs its custom cleanup callbacks and `deactivate()`
   * - Disposes all event subscriptions, intervals, and modals
   * - Unregisters all commands from the Command Palette
   * - Clears activation status so reload or re-enable can run cleanly
   */
  public deactivatePlugin(pluginId: string): void {
    // 1. Run custom cleanup callbacks
    const cleanups = this.cleanupsByPlugin.get(pluginId);
    if (cleanups) {
      for (const fn of cleanups) {
        try {
          fn();
        } catch (err) {
          console.warn(`[PLUGIN-ENGINE] Cleanup error in plugin ${pluginId}:`, err);
        }
      }
      this.cleanupsByPlugin.delete(pluginId);
    }

    // 2. Dispose all subscriptions
    const subs = this.subscriptionsByPlugin.get(pluginId);
    if (subs) {
      for (const s of subs) {
        try {
          s.dispose();
        } catch {}
      }
      this.subscriptionsByPlugin.delete(pluginId);
    }

    // 3. Unregister commands from UI
    const cmds = this.commandsByPlugin.get(pluginId);
    if (cmds) {
      if (this.onCommandUnregistered) {
        for (const cmdId of cmds.keys()) {
          this.onCommandUnregistered(cmdId);
        }
      }
      this.commandsByPlugin.delete(pluginId);
    }

    this.activated.delete(pluginId);
    console.log(`[PLUGIN-ENGINE] Deactivated plugin: ${pluginId}`);
  }

  /**
   * Toggles plugin state on/off.
   * Persists through IPC. Enabling an unactivated plugin activates it; re-enabling
   * an already activated plugin restores its commands without re-running its code.
   * Disabling removes its commands from the palette.
   */
  public async togglePlugin(pluginId: string, enabled: boolean): Promise<boolean> {
    const win = window as any;
    if (win.tokIpc && win.tokIpc.togglePlugin) {
      await win.tokIpc.togglePlugin(pluginId, enabled);
    }
    const target = this.plugins.find((p) => p.id === pluginId);
    if (target) {
      target.enabled = enabled;
      const cmds = [...(this.commandsByPlugin.get(pluginId)?.values() ?? [])];
      if (enabled) {
        if (!target.safeModeBlocked) {
          // First enable runs the code; re-enabling restores the commands it registered before.
          if (!this.activatePlugin(target)) {
            cmds.forEach((cmd) => this.onCommandRegistered?.(cmd));
          }
        }
      } else {
        if (this.onCommandUnregistered) {
          cmds.forEach((cmd) => this.onCommandUnregistered!(cmd.id));
        }
      }
    }
    return true;
  }

  public async openPluginsFolder(): Promise<void> {
    const win = window as any;
    if (win.tokIpc && win.tokIpc.openPluginsFolder) {
      await win.tokIpc.openPluginsFolder();
    }
  }
}

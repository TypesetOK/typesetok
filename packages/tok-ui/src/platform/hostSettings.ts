/**
 * Host-backed persistent settings store for TypesetOK.
 *
 * Hydrates synchronously from `window.__TOK_BOOTSTRAP__.settings` on desktop
 * runtimes, persists changes atomically to disk via `TokHostBridge` (`settings.json`),
 * and performs a one-time lossless migration of legacy `localStorage` preferences
 * so migrating origins (file:// -> tauri.localhost) never drops user state.
 */

import { getHostBridge } from './hostBridge.js';

export const PERSISTED_SETTING_KEYS = [
  'tok_lang',
  'tok_theme_settings',
  'tok_canvas_zoom',
  'tok_canvas_guides',
  'tok_recent_projects',
  'tok_autosave_enabled',
  'tok_log_retention',
  'tok_updates_state_v1',
  'tok_import_templates',
  'tok_custom_templates',
] as const;

interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

function getSafeLocalStorage(): StorageLike | null {
  try {
    if (typeof globalThis !== 'undefined' && (globalThis as any).localStorage) {
      return (globalThis as any).localStorage as StorageLike;
    }
  } catch {
    // Storage access may be blocked in restricted browser contexts
  }
  return null;
}

function canUseBrowserMemoryFallback(): boolean {
  return typeof window !== 'undefined' && typeof document !== 'undefined';
}

export class HostSettingsStore {
  private hostCache = new Map<string, string>();
  private memoryFallback = new Map<string, string>();
  private initializedFromBootstrap = false;

  constructor() {
    this.hydrateFromBootstrap();
  }

  private isHostPersisted(): boolean {
    const host = getHostBridge();
    return host.isDesktop && (host.has('setSetting') || this.initializedFromBootstrap);
  }

  /** Populates the in-memory cache from the synchronous host bootstrap payload. */
  public hydrateFromBootstrap(): void {
    const settings = getHostBridge().getBootstrap().settings;
    if (!settings || typeof settings !== 'object') {
      return;
    }
    for (const [k, v] of Object.entries(settings)) {
      if (typeof k === 'string' && typeof v === 'string') {
        this.hostCache.set(k, v);
      }
    }
    this.initializedFromBootstrap = true;
  }

  /** Resets internal state (primarily for isolated unit tests) and re-reads bootstrap. */
  public resetForTesting(): void {
    this.hostCache.clear();
    this.memoryFallback.clear();
    this.initializedFromBootstrap = false;
    this.hydrateFromBootstrap();
  }

  public get(key: string): string | null {
    const host = getHostBridge();
    if (this.isHostPersisted()) {
      if (this.hostCache.has(key)) {
        return this.hostCache.get(key) ?? null;
      }
      const storage = getSafeLocalStorage();
      if (storage) {
        try {
          const legacyVal = storage.getItem(key);
          if (legacyVal !== null && legacyVal !== undefined) {
            const str = String(legacyVal);
            this.hostCache.set(key, str);
            if (host.has('setSetting')) {
              void host.setSetting(key, str).catch((err) => {
                console.warn('[SETTINGS] Host migration write failed:', err);
              });
            }
            return str;
          }
        } catch (err) {
          console.warn('[SETTINGS] Failed to read legacy localStorage key:', err);
        }
      }
      return null;
    }

    const storage = getSafeLocalStorage();
    if (storage) {
      try {
        return storage.getItem(key) ?? null;
      } catch (err) {
        console.warn('[SETTINGS] Failed to read localStorage:', err);
      }
    }
    return canUseBrowserMemoryFallback() ? (this.memoryFallback.get(key) ?? null) : null;
  }

  public set(key: string, value: string): void {
    const str = String(value);
    const storage = getSafeLocalStorage();
    if (storage) {
      try {
        storage.setItem(key, str);
      } catch (err) {
        console.warn('[SETTINGS] Failed to mirror setting to localStorage:', err);
      }
    } else if (canUseBrowserMemoryFallback()) {
      this.memoryFallback.set(key, str);
    }

    const host = getHostBridge();
    if (this.isHostPersisted()) {
      this.hostCache.set(key, str);
      if (host.has('setSetting')) {
        void host.setSetting(key, str).catch((err) => {
          console.warn('[SETTINGS] Host persist failed:', err);
        });
      }
    }
  }

  public remove(key: string): void {
    const storage = getSafeLocalStorage();
    if (storage) {
      try {
        storage.removeItem(key);
      } catch (err) {
        console.warn('[SETTINGS] Failed to remove localStorage key:', err);
      }
    } else if (canUseBrowserMemoryFallback()) {
      this.memoryFallback.delete(key);
    }

    const host = getHostBridge();
    if (this.isHostPersisted()) {
      this.hostCache.delete(key);
      if (host.has('removeSetting')) {
        void host.removeSetting(key).catch((err) => {
          console.warn('[SETTINGS] Host remove failed:', err);
        });
      }
    }
  }

  /**
   * Ensures disk-backed settings are loaded and migrates any legacy `localStorage`
   * keys that are not yet present in `settings.json`. Returns migrated key names.
   */
  public async syncAndMigrate(): Promise<string[]> {
    const host = getHostBridge();
    if (!host.isDesktop) {
      return [];
    }

    if (host.has('readSettings') && !this.initializedFromBootstrap) {
      try {
        const diskSettings = await host.readSettings();
        if (diskSettings && typeof diskSettings === 'object') {
          for (const [k, v] of Object.entries(diskSettings)) {
            if (typeof k === 'string' && typeof v === 'string') {
              this.hostCache.set(k, v);
            }
          }
          this.initializedFromBootstrap = true;
        }
      } catch (err) {
        console.warn('[SETTINGS] Failed to read host settings:', err);
      }
    }

    const storage = getSafeLocalStorage();
    if (!storage) {
      return [];
    }

    const toMigrate: Record<string, string> = {};
    for (const key of PERSISTED_SETTING_KEYS) {
      if (this.hostCache.has(key)) continue;
      try {
        const rawVal = storage.getItem(key);
        if (rawVal !== null && rawVal !== undefined) {
          const str = String(rawVal);
          this.hostCache.set(key, str);
          toMigrate[key] = str;
        }
      } catch (err) {
        console.warn('[SETTINGS] Failed reading legacy localStorage key during migration:', key, err);
      }
    }

    const migratedKeys = Object.keys(toMigrate);
    if (migratedKeys.length === 0) {
      return [];
    }

    try {
      if (host.has('setSettings')) {
        await host.setSettings(toMigrate);
      } else if (host.has('setSetting')) {
        for (const k of migratedKeys) {
          await host.setSetting(k, toMigrate[k]);
        }
      }
    } catch (err) {
      console.warn('[SETTINGS] Failed writing migrated settings to host:', err);
    }

    return migratedKeys;
  }
}

export const hostSettings = new HostSettingsStore();

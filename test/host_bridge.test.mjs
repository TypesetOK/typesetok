import { test, describe, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  getHostBridge,
  setHostBridge,
  ElectronHostAdapter,
  WebPreviewAdapter,
} from '../packages/tok-ui/dist/platform/hostBridge.js';
import {
  hostSettings,
  HostSettingsStore,
  PERSISTED_SETTING_KEYS,
} from '../packages/tok-ui/dist/platform/hostSettings.js';
import { loadTs } from './e2e/cli_helper.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

function createMockStorage(initial = {}) {
  const map = new Map(Object.entries(initial));
  return {
    getItem: (k) => (map.has(k) ? map.get(k) : null),
    setItem: (k, v) => map.set(k, String(v)),
    removeItem: (k) => map.delete(k),
    dump: () => Object.fromEntries(map.entries()),
  };
}

describe('Phase 0 Host Bridge & Persistent Settings', () => {
  const savedGlobals = {};

  beforeEach(() => {
    savedGlobals.tokIpc = globalThis.tokIpc;
    savedGlobals.window = globalThis.window;
    savedGlobals.localStorage = globalThis.localStorage;
    savedGlobals.__TOK_BOOTSTRAP__ = globalThis.__TOK_BOOTSTRAP__;
    delete globalThis.tokIpc;
    delete globalThis.window;
    delete globalThis.localStorage;
    delete globalThis.__TOK_BOOTSTRAP__;
    setHostBridge(null);
    hostSettings.resetForTesting();
  });

  afterEach(() => {
    setHostBridge(null);
    for (const key of ['tokIpc', 'window', 'localStorage', '__TOK_BOOTSTRAP__']) {
      if (savedGlobals[key] === undefined) delete globalThis[key];
      else globalThis[key] = savedGlobals[key];
    }
    hostSettings.resetForTesting();
  });

  test('1. WebPreviewAdapter activates when no desktop host is present', async () => {
    const bridge = getHostBridge();
    assert.ok(bridge instanceof WebPreviewAdapter);
    assert.equal(bridge.kind, 'web');
    assert.equal(bridge.isDesktop, false);
    assert.equal(bridge.has('renderPdf'), false);
    assert.equal(bridge.has('getAppInfo'), false);

    assert.deepEqual(await bridge.getAppInfo(), { version: '', name: 'TypesetOK (TOK)' });
    assert.equal(await bridge.getStartupFile(), null);
    assert.deepEqual(await bridge.readSettings(), {});
    assert.deepEqual(await bridge.setSetting('tok_lang', 'en'), { success: false });
    await assert.rejects(() => bridge.renderPdf({}), /Desktop host unavailable/);
  });

  test('2. ElectronHostAdapter dynamically wraps window.tokIpc and reports method availability', async () => {
    let pingArg = null;
    globalThis.window = {
      tokIpc: {
        sendCommand: async (cmd) => {
          pingArg = cmd;
          return { status: 'ok' };
        },
        getAppInfo: async () => ({ version: '0.36.6', name: 'TypesetOK (TOK)' }),
      },
      __TOK_BOOTSTRAP__: {
        installLanguage: 'en',
        startupFile: 'C:\\Books\\test.tok',
        settings: { tok_lang: 'he' },
      },
    };

    const bridge = getHostBridge();
    assert.ok(bridge instanceof ElectronHostAdapter);
    assert.equal(bridge.kind, 'electron');
    assert.equal(bridge.isDesktop, true);
    assert.equal(bridge.has('sendCommand'), true);
    assert.equal(bridge.has('getAppInfo'), true);
    assert.equal(bridge.has('renderPdf'), false);

    assert.deepEqual(await bridge.sendCommand({ action: 'ping' }), { status: 'ok' });
    assert.deepEqual(pingArg, { action: 'ping' });
    assert.deepEqual(bridge.getBootstrap(), {
      platform: 'electron',
      installLanguage: 'en',
      startupFile: 'C:\\Books\\test.tok',
      settings: { tok_lang: 'he' },
    });
    await assert.rejects(() => bridge.renderPdf({}), /Electron host method unavailable/);
  });

  test('3. setHostBridge injects custom (e.g. Tauri) bridge and restores auto-detection on null', async () => {
    const customBridge = {
      ...new WebPreviewAdapter(),
      kind: 'tauri',
      isDesktop: true,
      getBootstrap: () => ({ platform: 'tauri', startupFile: 'D:\\Torah\\book.tok', settings: { tok_lang: 'he' } }),
      has: (m) => m === 'getAppInfo',
      getAppInfo: async () => ({ version: '0.37.0', name: 'TypesetOK (TOK)' }),
    };

    setHostBridge(customBridge);
    assert.equal(getHostBridge().kind, 'tauri');
    assert.equal(getHostBridge().isDesktop, true);
    assert.equal(getHostBridge().has('getAppInfo'), true);
    assert.deepEqual(await getHostBridge().getAppInfo(), { version: '0.37.0', name: 'TypesetOK (TOK)' });

    setHostBridge(null);
    assert.equal(getHostBridge().kind, 'web');
  });

  test('4. HostSettingsStore hydrates from bootstrap, persists via host, and migrates localStorage losslessly', async () => {
    const diskStore = { tok_lang: 'en', tok_canvas_zoom: '125' };
    const setCalls = [];
    const batchCalls = [];
    const removeCalls = [];

    globalThis.localStorage = createMockStorage({
      tok_lang: 'he', // must NOT overwrite existing disk setting 'en'
      tok_theme_settings: '{"paletteId":"dark"}', // must be migrated
      tok_autosave_enabled: 'false', // must be migrated
    });
    globalThis.window = {
      localStorage: globalThis.localStorage,
      __TOK_BOOTSTRAP__: {
        platform: 'electron',
        settings: { ...diskStore },
      },
      tokIpc: {
        readSettings: async () => ({ ...diskStore }),
        setSetting: async (k, v) => {
          setCalls.push([k, v]);
          diskStore[k] = v;
          return { success: true };
        },
        setSettings: async (entries) => {
          batchCalls.push(entries);
          Object.assign(diskStore, entries);
          return { success: true };
        },
        removeSetting: async (k) => {
          removeCalls.push(k);
          delete diskStore[k];
          return { success: true };
        },
      },
    };

    const store = new HostSettingsStore();
    assert.equal(store.get('tok_lang'), 'en');
    assert.equal(store.get('tok_canvas_zoom'), '125');

    const migrated = await store.syncAndMigrate();
    assert.deepEqual(migrated.sort(), ['tok_autosave_enabled', 'tok_theme_settings']);
    assert.equal(batchCalls.length, 1);
    assert.equal(diskStore.tok_lang, 'en');
    assert.equal(diskStore.tok_theme_settings, '{"paletteId":"dark"}');
    assert.equal(diskStore.tok_autosave_enabled, 'false');

    store.set('tok_log_retention', '30');
    await Promise.resolve();
    assert.deepEqual(setCalls, [['tok_log_retention', '30']]);
    assert.equal(store.get('tok_log_retention'), '30');

    store.remove('tok_log_retention');
    await Promise.resolve();
    assert.deepEqual(removeCalls, ['tok_log_retention']);
    assert.equal(store.get('tok_log_retention'), null);

    // Web mode falls back cleanly to localStorage without throwing
    delete globalThis.window;
    const webStore = new HostSettingsStore();
    webStore.set('tok_lang', 'he');
    assert.equal(webStore.get('tok_lang'), 'he');
    assert.ok(PERSISTED_SETTING_KEYS.includes('tok_recent_projects'));
  });

  test('5. PluginEngine sandbox blocks __TOK_BOOTSTRAP__, __TAURI_INTERNALS__, and __TAURI__ in addition to tokIpc', async () => {
    const { PluginEngine } = await loadTs('packages/tok-ui/src/plugins/PluginEngine.ts');

    let probe = null;
    globalThis.__reportHostBridgeSandbox = (res) => {
      probe = res;
    };

    const code = `
      function activate() {
        globalThis.__reportHostBridgeSandbox({
          tokIpcDirect: typeof tokIpc,
          tokIpcWin: typeof window.tokIpc,
          bootstrapDirect: typeof __TOK_BOOTSTRAP__,
          bootstrapWin: typeof window.__TOK_BOOTSTRAP__,
          hasBootstrap: ('__TOK_BOOTSTRAP__' in window),
          tauriInternalsDirect: typeof __TAURI_INTERNALS__,
          tauriInternalsWin: typeof window.__TAURI_INTERNALS__,
          hasTauriInternals: ('__TAURI_INTERNALS__' in window),
          tauriDirect: typeof __TAURI__,
          tauriWin: typeof window.__TAURI__,
          hasTauri: ('__TAURI__' in window),
        });
      }
    `;

    globalThis.tokIpc = {
      getPluginSystemStatus: async () => ({ isSafeMode: false, isSaferActive: false, userPluginsDir: '' }),
      getPlugins: async () => [
        {
          manifest: { id: 'bridge-probe', name: 'BridgeProbe', version: '1.0.0', description: '' },
          sourceType: 'js',
          enabled: true,
          compiledCode: code,
        },
      ],
    };
    globalThis.__TOK_BOOTSTRAP__ = { platform: 'tauri', settings: { secret: '1' } };
    globalThis.__TAURI_INTERNALS__ = { invoke: () => 'owned' };
    globalThis.__TAURI__ = { core: {} };

    try {
      const engine = new PluginEngine();
      await engine.loadPlugins();

      assert.ok(probe, 'Sandbox probe plugin must have run');
      assert.equal(probe.tokIpcDirect, 'undefined');
      assert.equal(probe.tokIpcWin, 'undefined');
      assert.equal(probe.bootstrapDirect, 'undefined');
      assert.equal(probe.bootstrapWin, 'undefined');
      assert.equal(probe.hasBootstrap, false);
      assert.equal(probe.tauriInternalsDirect, 'undefined');
      assert.equal(probe.tauriInternalsWin, 'undefined');
      assert.equal(probe.hasTauriInternals, false);
      assert.equal(probe.tauriDirect, 'undefined');
      assert.equal(probe.tauriWin, 'undefined');
      assert.equal(probe.hasTauri, false);

      // Host globals are intact after plugin execution finishes
      assert.equal(globalThis.__TOK_BOOTSTRAP__.platform, 'tauri');
      assert.equal(typeof globalThis.__TAURI_INTERNALS__.invoke, 'function');
      assert.equal(typeof globalThis.__TAURI__.core, 'object');
    } finally {
      delete globalThis.__reportHostBridgeSandbox;
    }
  });

  test('6. UI sources contain zero direct tokIpc or getStartupFileSync usages outside platform/hostBridge.ts and PluginEngine sandbox', () => {
    const uiSrcDir = path.join(rootDir, 'packages/tok-ui/src');
    const directTokIpcOffenders = [];
    const syncStartupOffenders = [];
    const allowedTokIpcFiles = new Set([
      path.normalize(path.join(uiSrcDir, 'platform/hostBridge.ts')),
      path.normalize(path.join(uiSrcDir, 'plugins/PluginEngine.ts')),
    ]);

    const walk = (dir) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.normalize(path.join(dir, entry.name));
        if (entry.isDirectory()) {
          walk(full);
          continue;
        }
        if (!entry.name.endsWith('.ts')) continue;
        const content = fs.readFileSync(full, 'utf8');
        if (!allowedTokIpcFiles.has(full) && /\btokIpc\b/.test(content)) {
          directTokIpcOffenders.push(path.relative(rootDir, full));
        }
        if (/\bgetStartupFileSync\b/.test(content)) {
          syncStartupOffenders.push(path.relative(rootDir, full));
        }
      }
    };

    walk(uiSrcDir);
    assert.deepEqual(directTokIpcOffenders, []);
    assert.deepEqual(syncStartupOffenders, []);
  });

  test('7. Electron main and preload expose host settings and bootstrap IPC contracts', () => {
    const mainSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-electron/src/main.ts'), 'utf8');
    const preloadSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-electron/src/preload.ts'), 'utf8');

    for (const ch of ['tok:read-settings', 'tok:set-setting', 'tok:set-settings', 'tok:remove-setting', 'tok:get-bootstrap-sync']) {
      assert.ok(mainSrc.includes(ch), `main.ts must handle ${ch}`);
      assert.ok(preloadSrc.includes(ch), `preload.ts must invoke ${ch}`);
    }
    assert.ok(preloadSrc.includes("exposeInMainWorld('__TOK_BOOTSTRAP__'"));
  });
});

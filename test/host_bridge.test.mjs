import { test, describe, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  getHostBridge,
  setHostBridge,
  ElectronHostAdapter,
  TauriHostAdapter,
  WebPreviewAdapter,
} from '../packages/tok-ui/dist/platform/hostBridge.js';
import {
  hostSettings,
  HostSettingsStore,
  PERSISTED_SETTING_KEYS,
} from '../packages/tok-ui/dist/platform/hostSettings.js';
import { buildTauriFrontendDist } from '../scripts/bundle-ui.mjs';
import { parseMeasureArgs, parseTraceLine } from '../scripts/measure-startup.mjs';
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
    savedGlobals.__TAURI_INTERNALS__ = globalThis.__TAURI_INTERNALS__;
    savedGlobals.__TAURI__ = globalThis.__TAURI__;
    delete globalThis.tokIpc;
    delete globalThis.window;
    delete globalThis.localStorage;
    delete globalThis.__TOK_BOOTSTRAP__;
    delete globalThis.__TAURI_INTERNALS__;
    delete globalThis.__TAURI__;
    setHostBridge(null);
    hostSettings.resetForTesting();
  });

  afterEach(() => {
    setHostBridge(null);
    for (const key of ['tokIpc', 'window', 'localStorage', '__TOK_BOOTSTRAP__', '__TAURI_INTERNALS__', '__TAURI__']) {
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

  test('7. Tauri desktop commands and adapter expose host settings and bootstrap contracts', () => {
    const commandsRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/commands.rs'), 'utf8');
    const mainRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/main.rs'), 'utf8');
    const adapterTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/platform/tauriHostAdapter.ts'), 'utf8');

    for (const cmd of ['tok_read_settings', 'tok_set_setting', 'tok_set_settings', 'tok_remove_setting']) {
      assert.ok(commandsRs.includes(cmd), `commands.rs must handle ${cmd}`);
      assert.ok(adapterTs.includes(cmd), `tauriHostAdapter.ts must invoke ${cmd}`);
    }
    assert.ok(mainRs.includes('__TOK_BOOTSTRAP__'));
  });

  test('8. TauriHostAdapter auto-detection preserves Electron priority and resets cleanly', () => {
    globalThis.window = {
      __TAURI_INTERNALS__: { invoke: async () => ({}) },
      __TOK_BOOTSTRAP__: {
        platform: 'tauri',
        installLanguage: 'he',
        startupFile: 'D:\\Sefarim\\gemara.tok',
        settings: { tok_lang: 'he' },
      },
    };

    const tauriBridge = getHostBridge();
    assert.ok(tauriBridge instanceof TauriHostAdapter);
    assert.equal(tauriBridge.kind, 'tauri');
    assert.equal(tauriBridge.isDesktop, true);
    assert.equal(tauriBridge.getBootstrap().startupFile, 'D:\\Sefarim\\gemara.tok');

    // Electron tokIpc takes priority if both exist
    globalThis.window.tokIpc = { sendCommand: async () => ({ ok: true }) };
    const electronBridge = getHostBridge();
    assert.ok(electronBridge instanceof ElectronHostAdapter);
    assert.equal(electronBridge.kind, 'electron');

    // Removing both falls back to WebPreviewAdapter
    delete globalThis.window.tokIpc;
    delete globalThis.window.__TAURI_INTERNALS__;
    assert.ok(getHostBridge() instanceof WebPreviewAdapter);
  });

  test('9. TauriHostAdapter command mapping, capability gating, and async event unsubscribe safety', async () => {
    const invoked = [];
    const listeners = new Map();
    let unlistenCount = 0;
    let deferredResolve = null;

    const mockRuntime = {
      invoke: async (cmd, args) => {
        invoked.push({ cmd, args });
        if (cmd === 'tok_render_pdf') return 'ok-pdf';
        if (cmd === 'tok_render_html') return 'ok-html';
        if (cmd === 'tok_typeset_document') return { success: true, pages: [], preflight: null };
        if (cmd === 'tok_read_settings') return { tok_lang: 'he' };
        if (cmd === 'tok_set_setting') return { success: true };
        if (cmd === 'tok_set_settings') return { success: true };
        if (cmd === 'tok_remove_setting') return { success: true };
        if (cmd === 'tok_list_system_fonts') return ['Frank Ruhl Libre'];
        return null;
      },
      listen: (eventName, handler) => {
        if (eventName === 'tok:update-download-progress') {
          return new Promise((resolve) => {
            deferredResolve = () => {
              listeners.set(eventName, handler);
              resolve(() => {
                unlistenCount++;
                listeners.delete(eventName);
              });
            };
          });
        }
        listeners.set(eventName, handler);
        return Promise.resolve(() => {
          unlistenCount++;
          listeners.delete(eventName);
        });
      },
    };

    globalThis.window = {
      __TOK_BOOTSTRAP__: {
        platform: 'tauri',
        startupFile: 'C:\\Books\\boot.tok',
        capabilities: [
          'renderPdf',
          'renderHtml',
          'typesetDocument',
          'readSettings',
          'setSetting',
          'setSettings',
          'removeSetting',
          'listSystemFonts',
          'onEvent',
          'onOpenFile',
          'onUpdateDownloadProgress',
        ],
      },
    };

    const adapter = new TauriHostAdapter(mockRuntime);
    assert.equal(adapter.has('typesetDocument'), true);
    assert.equal(adapter.has('scanUsbUpdates'), false);
    assert.equal(adapter.has('getStartupFile'), false);
    // When getStartupFile is outside declared capabilities, falls back to bootstrap without invoking IPC
    assert.equal(await adapter.getStartupFile(), 'C:\\Books\\boot.tok');

    assert.equal(await adapter.renderPdf('in.tok', 'out.pdf'), 'ok-pdf');
    assert.equal(await adapter.renderPdf({ document: { id: 1 } }, 'out2.pdf'), 'ok-pdf');
    assert.equal(await adapter.renderHtml({ inputPath: 'doc.tok', outputPath: 'doc.html' }), 'ok-html');
    await adapter.typesetDocument({ inputPath: 'book.tok', withPreflight: true });
    await adapter.setSetting('tok_lang', 'en');
    await adapter.setSettings({ tok_canvas_zoom: '110' });
    await adapter.removeSetting('tok_canvas_zoom');
    assert.deepEqual(await adapter.listSystemFonts(), ['Frank Ruhl Libre']);

    assert.deepEqual(invoked, [
      { cmd: 'tok_render_pdf', args: { payload: { inputPath: 'in.tok', outputPath: 'out.pdf' } } },
      { cmd: 'tok_render_pdf', args: { payload: { document: { id: 1 }, outputPath: 'out2.pdf' } } },
      { cmd: 'tok_render_html', args: { payload: { inputPath: 'doc.tok', outputPath: 'doc.html' } } },
      { cmd: 'tok_typeset_document', args: { payload: { inputPath: 'book.tok', withPreflight: true } } },
      { cmd: 'tok_set_setting', args: { key: 'tok_lang', value: 'en' } },
      { cmd: 'tok_set_settings', args: { entries: { tok_canvas_zoom: '110' } } },
      { cmd: 'tok_remove_setting', args: { key: 'tok_canvas_zoom' } },
      { cmd: 'tok_list_system_fonts', args: undefined },
    ]);

    // Default capabilities without bootstrap restriction cover all host methods
    delete globalThis.window.__TOK_BOOTSTRAP__.capabilities;
    assert.equal(adapter.has('scanUsbUpdates'), true);
    assert.equal(adapter.has('getStartupFile'), true);

    const events = [];
    const openFiles = [];
    const unsubEvent = adapter.onEvent((e) => events.push(e));
    const unsubOpen = adapter.onOpenFile((p) => openFiles.push(p));
    await Promise.resolve();
    listeners.get('tok:event')?.({ payload: { action: 'menu-save' } });
    listeners.get('tok:open-file')?.({ payload: 'C:\\Books\\opened.tok' });
    assert.deepEqual(events, [{ action: 'menu-save' }]);
    assert.deepEqual(openFiles, ['C:\\Books\\opened.tok']);
    unsubEvent();
    unsubOpen();
    assert.equal(unlistenCount, 2);

    // Unsubscribing before async listen resolves still triggers unlisten on resolution without delivering late events
    const progressCalls = [];
    const unsubProgressEarly = adapter.onUpdateDownloadProgress((d) => progressCalls.push(d));
    unsubProgressEarly();
    assert.equal(typeof deferredResolve, 'function');
    deferredResolve();
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(unlistenCount, 3);
    assert.deepEqual(progressCalls, []);
  });

  test('10. TopSystemBar applies data-tauri-drag-region only to non-interactive drag elements', () => {
    const topBarSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf-8');
    assert.ok(topBarSrc.includes("this.element.setAttribute('data-tauri-drag-region', '')"));
    assert.ok(topBarSrc.includes("el('div', 'tok-top-center', { 'data-tauri-drag-region': '' })"));
    assert.ok(topBarSrc.includes("'data-tauri-drag-region': '' }, ' - ')"));
    assert.ok(topBarSrc.includes("el('span', 'tok-doc-title-page', { 'data-tauri-drag-region': '' }"));
    // Interactive buttons must not hijack click targets as drag regions
    for (const marker of ['menuBtn', 'minBtn', 'maxBtn', 'closeBtn', 'brand']) {
      const snippetRe = new RegExp(`const\\s+${marker}\\s*=[^;]+data-tauri-drag-region`);
      assert.equal(snippetRe.test(topBarSrc), false, `${marker} must not set data-tauri-drag-region`);
    }
  });

  test('11. Tauri frontend dist rewrites assets/CSP cleanly while keeping Electron src/index.html intact', () => {
    const srcHtmlBefore = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/index.html'), 'utf-8');
    const result = buildTauriFrontendDist(rootDir, path.join(rootDir, 'packages/tok-ui'));
    const srcHtmlAfter = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/index.html'), 'utf-8');
    assert.equal(srcHtmlAfter, srcHtmlBefore, 'src/index.html must never be mutated');
    assert.ok(srcHtmlAfter.includes('../../../assets/icon.png'));

    const tauriHtml = fs.readFileSync(path.join(result.tauriDistDir, 'index.html'), 'utf-8');
    assert.equal(tauriHtml.includes('../../../assets/'), false, 'dist-tauri/index.html must rewrite all ../../../assets/');
    assert.ok(tauriHtml.includes('./assets/icon.png'));
    assert.ok(tauriHtml.includes('http://ipc.localhost'));
    assert.ok(tauriHtml.includes('http://tok-asset.localhost'));
    assert.ok(fs.existsSync(path.join(result.tauriDistDir, 'renderer.js')));
    assert.ok(result.copiedAssets.length >= 16);
    for (const rel of result.copiedAssets) {
      assert.ok(fs.existsSync(path.join(result.tauriDistDir, 'assets', rel)), `Missing copied asset: ${rel}`);
    }
  });

  test('12. Gate 1 startup measurement script parses --tauri and trace/memory lines', () => {
    const parsedTauri = parseMeasureArgs(['--runs', '5', '--tauri', 'target/release/tok-desktop.exe'], rootDir);
    assert.equal(parsedTauri.runs, 5);
    assert.equal(parsedTauri.targets.length, 1);
    assert.equal(parsedTauri.targets[0].kind, 'tauri');
    assert.ok(parsedTauri.targets[0].targetPath.endsWith(path.join('target', 'release', 'tok-desktop.exe')));

    const parsedPositional = parseMeasureArgs(['3', 'target/release/tok-desktop.exe'], rootDir);
    assert.equal(parsedPositional.runs, 3);
    assert.equal(parsedPositional.targets.length, 1);
    assert.equal(parsedPositional.targets[0].kind, 'tauri');

    assert.deepEqual(parseTraceLine('[TOK-TRACE] main-shown 1700000000512', 1700000000000), {
      kind: 'trace',
      label: 'main-shown',
      value: 512,
    });
    assert.deepEqual(parseTraceLine('[TOK-MEM] working-set-bytes 73400320', 1700000000000), {
      kind: 'mem',
      label: 'working-set-bytes',
      value: 73400320,
    });
    assert.equal(parseTraceLine('unrelated stdout log line', 0), null);
  });
});

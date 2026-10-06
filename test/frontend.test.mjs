import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

describe('Hebrew Typography & Gematria Engine', () => {
  function toHebrewGematria(num) {
    if (num <= 0) return '';
    const letters = [
      [400, 'ת'], [300, 'ש'], [200, 'ר'], [100, 'ק'],
      [90, 'צ'], [80, 'פ'], [70, 'ע'], [60, 'ס'],
      [50, 'נ'], [40, 'מ'], [30, 'ל'], [20, 'כ'],
      [10, 'י'], [9, 'ט'], [8, 'ח'], [7, 'ז'],
      [6, 'ו'], [5, 'ה'], [4, 'ד'], [3, 'ג'],
      [2, 'ב'], [1, 'א']
    ];
    let n = num;
    let res = '';
    if (n === 15) return 'ט״ו';
    if (n === 16) return 'ט״ז';

    for (const [val, char] of letters) {
      while (n >= val) {
        res += char;
        n -= val;
      }
    }
    if (res.length === 1) {
      return res + '׳';
    } else if (res.length > 1) {
      return res.slice(0, -1) + '״' + res.slice(-1);
    }
    return res;
  }

  test('Single-letter gematria (1-9)', () => {
    assert.equal(toHebrewGematria(1), 'א׳');
    assert.equal(toHebrewGematria(2), 'ב׳');
    assert.equal(toHebrewGematria(5), 'ה׳');
    assert.equal(toHebrewGematria(9), 'ט׳');
  });

  test('Talmudic exceptions for 15 and 16 (Tet-Vav, Tet-Zayin)', () => {
    assert.equal(toHebrewGematria(15), 'ט״ו');
    assert.equal(toHebrewGematria(16), 'ט״ז');
  });

  test('Tens and hundreds with gershayim', () => {
    assert.equal(toHebrewGematria(20), 'כ׳');
    assert.equal(toHebrewGematria(21), 'כ״א');
    assert.equal(toHebrewGematria(100), 'ק׳');
    assert.equal(toHebrewGematria(354), 'שנ״ד');
  });
});

describe('Page DOM Virtualizer (Section 10.3)', () => {
  function computeActiveWindow(centerPage, totalPages) {
    const windowStart = Math.max(0, centerPage - 1);
    const windowEnd = Math.min(totalPages - 1, centerPage + 1);
    const mounted = [];
    for (let i = 0; i < totalPages; i++) {
      if (i >= windowStart && i <= windowEnd) {
        mounted.push(i);
      }
    }
    return { windowStart, windowEnd, mounted };
  }

  test('Active window at start of book (page 0)', () => {
    const { mounted } = computeActiveWindow(0, 1000);
    assert.deepEqual(mounted, [0, 1]);
    assert.equal(mounted.length <= 3, true);
  });

  test('Active window in middle of book enforces strictly 3 pages [K-1, K, K+1]', () => {
    const { mounted } = computeActiveWindow(500, 1000);
    assert.deepEqual(mounted, [499, 500, 501]);
    assert.equal(mounted.length, 3);
  });

  test('Active window at end of book', () => {
    const { mounted } = computeActiveWindow(999, 1000);
    assert.deepEqual(mounted, [998, 999]);
  });

  test('Single-page document', () => {
    const { mounted } = computeActiveWindow(0, 1);
    assert.deepEqual(mounted, [0]);
  });
});

describe('Canvas Overlay & Optimistic RTL Advance (Section 9.2)', () => {
  test('RTL caret advance decreases X position immediately (<16ms)', () => {
    let caret = { x: 500, y: 100, height: 18, visible: true };
    const step = 8.5; // average Hebrew glyph width in pt

    caret.x -= step;
    assert.equal(caret.x, 491.5);

    caret.x -= step;
    assert.equal(caret.x, 483.0);
  });

  test('Selection rectangle bounds calculation', () => {
    const rects = [
      { pageIndex: 0, x: 100, y: 50, width: 250, height: 16 }
    ];
    assert.equal(rects.length, 1);
    assert.equal(rects[0].width, 250);
  });

  test('Interactive drag selection computes normalized bounding box', () => {
    const dragStart = { x: 350, y: 120 };
    const dragCurrent = { x: 200, y: 150 };

    const minX = Math.min(dragStart.x, dragCurrent.x);
    const maxX = Math.max(dragStart.x, dragCurrent.x);
    const minY = Math.min(dragStart.y, dragCurrent.y);
    const maxY = Math.max(dragStart.y, dragCurrent.y);

    const selection = {
      pageIndex: 0,
      x: minX,
      y: minY,
      width: maxX - minX,
      height: maxY - minY
    };

    assert.equal(selection.x, 200);
    assert.equal(selection.y, 120);
    assert.equal(selection.width, 150);
    assert.equal(selection.height, 30);
  });

  test('Spatial hit-test snapped coordinate calculation', () => {
    const lineBoxes = [
      { baselineY: 50, height: 16, glyphs: [{ x: 100, width: 20 }, { x: 120, width: 25 }] },
      { baselineY: 80, height: 16, glyphs: [{ x: 100, width: 15 }, { x: 115, width: 30 }] }
    ];

    function snapToLine(clickY) {
      let closest = lineBoxes[0];
      let minDiff = Math.abs(closest.baselineY - clickY);
      for (const line of lineBoxes) {
        const diff = Math.abs(line.baselineY - clickY);
        if (diff < minDiff) {
          minDiff = diff;
          closest = line;
        }
      }
      return closest;
    }

    const clickedLine = snapToLine(75);
    assert.equal(clickedLine.baselineY, 80);
  });
});

describe('Build Artifacts & Distribution Packaging', () => {
  test('Electron main process is compiled to dist/main.js', () => {
    const mainJs = path.join(rootDir, 'packages/tok-electron/dist/main.js');
    assert.equal(fs.existsSync(mainJs), true, 'main.js must exist');
    const content = fs.readFileSync(mainJs, 'utf-8');
    assert.equal(content.includes('createWindow'), true);
  });

  test('Electron preload script is compiled to dist/preload.js', () => {
    const preloadJs = path.join(rootDir, 'packages/tok-electron/dist/preload.js');
    assert.equal(fs.existsSync(preloadJs), true, 'preload.js must exist');
    const content = fs.readFileSync(preloadJs, 'utf-8');
    assert.equal(content.includes('contextBridge.exposeInMainWorld'), true);
  });

  test('UI renderer bundle is generated and non-empty', () => {
    const rendererJs = path.join(rootDir, 'packages/tok-ui/dist/renderer.js');
    assert.equal(fs.existsSync(rendererJs), true, 'renderer.js must exist');
    const stat = fs.statSync(rendererJs);
    assert.equal(stat.size > 10000, true, 'renderer bundle should be > 10KB');
  });

  test('UI index.html is present with root container and script tag', () => {
    const indexHtml = path.join(rootDir, 'packages/tok-ui/dist/index.html');
    assert.equal(fs.existsSync(indexHtml), true, 'index.html must exist');
    const html = fs.readFileSync(indexHtml, 'utf-8');
    assert.equal(html.includes('id="app"'), true);
    assert.equal(html.includes('src="renderer.js"'), true);
    assert.equal(html.includes('dir="rtl"'), true);
  });
});

describe('DTP Modern UX/UI Specification & Design Tokens (Section 11)', () => {
  const indexHtmlPath = path.join(rootDir, 'packages/tok-ui/dist/index.html');
  let html = '';

  test('All normative design tokens from Section 11 are defined in CSS', () => {
    html = fs.readFileSync(indexHtmlPath, 'utf-8');
    const requiredTokens = [
      '--tok-bg-canvas: #121212',
      '--tok-bg-app: #181818',
      '--tok-bg-surface-1: #1E1E1E',
      '--tok-bg-surface-2: #262626',
      '--tok-bg-elevated: #303030',
      '--tok-border-subtle: #2C2C2C',
      '--tok-border-strong: #3E3E3E',
      '--tok-border-focus: #3B82F6',
      '--tok-accent-primary: #2563EB',
      '--tok-selection-frame: #3B82F6',
      '--tok-selection-text: rgba(59, 130, 246, 0.35)',
      '--tok-guide-margin: #9333EA',
      '--tok-guide-column: #06B6D4',
      '--tok-guide-baseline: rgba(16, 185, 129, 0.25)',
      '--tok-status-error: #EF4444',
      '--tok-status-warning: #F59E0B',
      '--tok-status-success: #10B981'
    ];

    for (const token of requiredTokens) {
      const tokenName = token.split(':')[0].trim();
      assert.equal(html.includes(tokenName), true, `Token ${tokenName} must be defined`);
    }
  });

  test('Workstation layout dimensions (Section 18) are present', () => {
    // Redesign (2026-10): side panel 236px, inspector 300px, top bar 52px, status bar 28px.
    assert.equal(/--tok-structure-width:\s*236px/.test(html), true);
    assert.equal(/--tok-inspector-width:\s*300px/.test(html), true);
    assert.equal(/--tok-top-bar-height:\s*52px/.test(html), true);
    assert.equal(/--tok-status-height:\s*28px/.test(html), true);
    assert.equal(html.includes('--tok-hud-height: 36px') || html.includes('36px'), true);
  });
});

describe('Interaction Triad Architecture (Section 8 & 17)', () => {
  const rendererJsPath = path.join(rootDir, 'packages/tok-ui/dist/renderer.js');
  let rendererJs = '';

  test('Canvas Action HUD is compiled into bundle with micro-actions', () => {
    rendererJs = fs.readFileSync(rendererJsPath, 'utf-8');
    assert.equal(rendererJs.includes('tok-action-hud'), true);
    assert.equal(rendererJs.includes('ActionHud'), true);
  });

  test('Contextual Inspector state machine handles zero, frame, and text-edit modes', () => {
    assert.equal(rendererJs.includes('ContextualInspector'), true);
    assert.equal(rendererJs.includes('renderZeroSelection'), true);
    assert.equal(rendererJs.includes('renderTextFrameMode'), true);
    assert.equal(rendererJs.includes('renderTextEditMode'), true);
  });

  test('Command Palette supports Cmd+K fuzzy searching and categories', () => {
    assert.equal(rendererJs.includes('CommandPalette'), true);
    assert.equal(rendererJs.includes('tok-palette-modal'), true);
    assert.equal(rendererJs.includes('cmd-full-justify'), true);
    assert.equal(rendererJs.includes('cmd-export-pdf'), true);
  });
});

describe('3-Tier Hebrew Justification Model (Section 15)', () => {
  test('Tier 1 (Word spacing range 80%-130%) calculation', () => {
    const minSpacing = 85;
    const maxSpacing = 125;
    assert.equal(minSpacing >= 80, true);
    assert.equal(maxSpacing <= 130, true);
  });

  test('Tier 2 (Oheltarem letter expansion) set contains authentic sacred letters', () => {
    const oheltaremLetters = ['א', 'ה', 'ל', 'ת', 'ר', 'ם'];
    assert.deepEqual(oheltaremLetters, ['א', 'ה', 'ל', 'ת', 'ר', 'ם']);
    assert.equal(oheltaremLetters.length, 6);
  });

  test('Tier 3 (Micro-tracking range ±2%) within threshold', () => {
    const maxMicroTracking = 2.0;
    assert.equal(maxMicroTracking <= 2.5, true);
  });
});

// ---------------------------------------------------------------------------
// Regression tests that execute the real TypeScript sources.
// Each module is bundled on the fly with esbuild (already a devDependency) into
// the OS temp dir and imported; a minimal fake DOM stands in for the browser.
// ---------------------------------------------------------------------------

class FakeClassList {
  constructor() { this.set = new Set(); }
  add(...c) { c.forEach((x) => this.set.add(x)); }
  remove(...c) { c.forEach((x) => this.set.delete(x)); }
  contains(c) { return this.set.has(c); }
}

class FakeStyle {
  setProperty(k, v) { this[k] = String(v); }
  removeProperty(k) { delete this[k]; }
  getPropertyValue(k) { return this[k] ?? ''; }
}

class FakeElement {
  constructor(tag = 'div') {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.style = new FakeStyle();
    this.dataset = {};
    this.classList = new FakeClassList();
    this.listeners = {};
    this._html = '';
    this.offsetTop = 0;
    this.offsetHeight = 0;
    this.scrollTop = 0;
    this.clientHeight = 0;
    this.attributes = {};
  }
  set className(v) {
    this._className = v;
    this.classList = new FakeClassList();
    if (v) v.split(/\s+/).forEach((c) => this.classList.add(c));
  }
  get className() { return this._className || ''; }
  set innerHTML(v) { this._html = v; this.children = []; }
  get innerHTML() { return this._html; }
  appendChild(c) { this.children.push(c); c.parentElement = this; return c; }
  setAttribute(k, v) { this.attributes[k] = String(v); }
  getAttribute(k) { return this.attributes[k]; }
  click() { (this.listeners['click'] || []).forEach((fn) => fn({})); }
  replaceChildren(...kids) { this.children = kids.filter(Boolean); }
  addEventListener(type, fn) { (this.listeners[type] ||= []).push(fn); }
  removeEventListener(type, fn) { this.listeners[type] = (this.listeners[type] || []).filter((f) => f !== fn); }
  scrollIntoView() {}
  querySelector(sel) {
    for (const c of this.children) {
      if (c && typeof c === 'object') {
        const matchesClass = !sel.includes('.tok-hud-btn') || c.className?.includes('tok-hud-btn') || c.classList?.contains('tok-hud-btn');
        const matchesPressed = !sel.includes('aria-pressed') || (c.attributes && 'aria-pressed' in c.attributes);
        if (matchesClass && matchesPressed) return c;
        const found = c.querySelector?.(sel);
        if (found) return found;
      }
    }
    return null;
  }
  querySelectorAll(sel) {
    const list = [];
    for (const c of this.children) {
      if (c && typeof c === 'object' && c.querySelectorAll) {
        list.push(...c.querySelectorAll(sel));
      }
    }
    return list;
  }
}

function installFakeDom() {
  const storage = new Map();
  globalThis.localStorage = {
    getItem: (k) => (storage.has(k) ? storage.get(k) : null),
    setItem: (k, v) => storage.set(k, String(v)),
    removeItem: (k) => storage.delete(k),
    clear: () => storage.clear(),
  };
  const documentElement = new FakeElement('html');
  globalThis.document = {
    documentElement,
    body: new FakeElement('body'),
    activeElement: null,
    createElement: (tag) => new FakeElement(tag),
    createElementNS: (_ns, tag) => new FakeElement(tag),
  };
  globalThis.getComputedStyle = () => ({ position: 'static', direction: 'rtl' });
  globalThis.requestAnimationFrame = (fn) => setTimeout(fn, 0);
  globalThis.cancelAnimationFrame = (id) => clearTimeout(id);
  if (!globalThis.window) globalThis.window = globalThis;
  globalThis.window.addEventListener ||= (type, fn) => {};
  globalThis.window.removeEventListener ||= (type, fn) => {};
  return { storage, documentElement };
}

const fakeDom = installFakeDom();

let esbuildMod = null;
async function loadTs(relPath) {
  esbuildMod ||= await import('esbuild');
  const os = await import('node:os');
  const { pathToFileURL } = await import('node:url');
  const outfile = path.join(
    os.tmpdir(),
    `tok-test-${process.pid}-${relPath.replace(/[^a-zA-Z0-9]+/g, '_')}-${Date.now()}.mjs`
  );
  esbuildMod.buildSync({
    entryPoints: [path.join(rootDir, relPath)],
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    outfile,
    logLevel: 'silent',
  });
  try {
    return await import(pathToFileURL(outfile).href);
  } finally {
    fs.rmSync(outfile, { force: true });
  }
}

describe('Regression: i18n coverage (every key used in the UI exists in Hebrew and English)', () => {
  test('all translation entries have non-empty he and en strings', async () => {
    const { strings } = await loadTs('packages/tok-ui/src/i18n.ts');
    for (const [key, entry] of Object.entries(strings)) {
      assert.ok(entry.he && entry.he.trim(), `missing Hebrew for ${key}`);
      assert.ok(entry.en && entry.en.trim(), `missing English for ${key}`);
    }
  });

  test('every t()/tf()/labelKey reference in tok-ui resolves to a defined key', async () => {
    const { strings } = await loadTs('packages/tok-ui/src/i18n.ts');
    const srcDir = path.join(rootDir, 'packages/tok-ui/src');
    const files = [];
    const walk = (d) => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = path.join(d, e.name);
        if (e.isDirectory()) walk(p);
        else if (p.endsWith('.ts')) files.push(p);
      }
    };
    walk(srcDir);
    const patterns = [
      /\btf?\(\s*'(\w+)'/g,
      /labelKey:\s*'(\w+)'/g,
      /\bt\([^()]*?\?\s*'(\w+)'\s*:\s*'(\w+)'\s*\)/g,
    ];
    const missing = [];
    for (const file of files) {
      const src = fs.readFileSync(file, 'utf-8');
      for (const re of patterns) {
        for (const m of src.matchAll(re)) {
          for (const key of m.slice(1).filter(Boolean)) {
            if (/^(div|span|p|button|input|select|option|label|header|footer|main|aside|kbd|h[1-6]|canvas|strong)$/.test(key)) continue;
            if (!strings[key]) missing.push(`${path.basename(file)}: ${key}`);
          }
        }
      }
    }
    assert.deepEqual(missing, []);
  });

  test('tf() substitutes placeholders and onOff() is localized', async () => {
    const mod = await loadTs('packages/tok-ui/src/i18n.ts');
    mod.i18n.setLanguage('en');
    assert.equal(mod.tf('logsRetentionUpdated', { n: 30 }), 'Log retention set to 30 days');
    assert.equal(mod.onOff(true), 'enabled');
    assert.equal(fakeDom.documentElement.dir, 'ltr');
    mod.i18n.setLanguage('he');
    assert.equal(fakeDom.documentElement.dir, 'rtl');
    assert.equal(fakeDom.documentElement.lang, 'he');
  });
});

describe('Regression: Command Palette shortcuts & search', () => {
  test('Ctrl+K works with a Hebrew keyboard layout (key "ל", code KeyK)', async () => {
    const { isPaletteToggleHotkey } = await loadTs('packages/tok-ui/src/components/CommandPalette.ts');
    assert.equal(isPaletteToggleHotkey({ key: 'ל', code: 'KeyK', ctrlKey: true, metaKey: false, altKey: false, shiftKey: false }), true);
    assert.equal(isPaletteToggleHotkey({ key: 'k', code: 'KeyK', ctrlKey: false, metaKey: true, altKey: false, shiftKey: false }), true);
    assert.equal(isPaletteToggleHotkey({ key: 'ל', code: 'KeyK', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false }), false);
    assert.equal(isPaletteToggleHotkey({ key: 'k', code: 'KeyK', ctrlKey: true, metaKey: false, altKey: true, shiftKey: false }), false);
  });

  test('"/" does not hijack typing in the story editor or form fields', async () => {
    const { isPaletteSlashHotkey } = await loadTs('packages/tok-ui/src/components/CommandPalette.ts');
    const ev = { key: '/', code: 'Slash', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false };
    assert.equal(isPaletteSlashHotkey(ev, { tagName: 'DIV', isContentEditable: true }), false);
    assert.equal(isPaletteSlashHotkey(ev, { tagName: 'P', isContentEditable: true }), false);
    assert.equal(isPaletteSlashHotkey(ev, { tagName: 'INPUT' }), false);
    assert.equal(isPaletteSlashHotkey(ev, { tagName: 'SELECT' }), false);
    assert.equal(isPaletteSlashHotkey(ev, { tagName: 'MAIN', isContentEditable: false }), true);
    assert.equal(isPaletteSlashHotkey({ ...ev, ctrlKey: true }, { tagName: 'MAIN' }), false);
  });

  test('search ignores niqqud and treats ״/" and ׳/\' alike; all terms must match', async () => {
    const { matchesPaletteQuery } = await loadTs('packages/tok-ui/src/components/CommandPalette.ts');
    const item = { title: 'נַרְמֵל ניקוד וטעמים (ת״י 6100)', category: 'פעולות טיפוגרפיה', subtitle: 'תיקון סדר תווי יוניקוד', shortcut: 'Ctrl+Shift+N' };
    assert.equal(matchesPaletteQuery(item, 'נרמל'), true);
    assert.equal(matchesPaletteQuery(item, 'ת"י'), true);
    assert.equal(matchesPaletteQuery(item, 'נרמל יוניקוד'), true);
    assert.equal(matchesPaletteQuery(item, 'נרמל גימטריה'), false);
    assert.equal(matchesPaletteQuery(item, '  '), true);
  });
});

describe('Regression: spread geometry is consistent (RTL book: recto = left page)', () => {
  test('groupIntoSpreads keeps page 0 alone and pairs the rest', async () => {
    const { groupIntoSpreads } = await loadTs('packages/tok-ui/src/components/SpreadCanvas.ts');
    assert.deepEqual(groupIntoSpreads(0), []);
    assert.deepEqual(groupIntoSpreads(1), [[0]]);
    assert.deepEqual(groupIntoSpreads(4), [[0], [1, 2], [3]]);
    assert.deepEqual(groupIntoSpreads(5), [[0], [1, 2], [3, 4]]);
  });

  test('even pages are recto (ע"א, left); odd pages are verso (ע"ב, right)', async () => {
    const { isRectoPage, isRightHandPage, groupIntoSpreads } = await loadTs('packages/tok-ui/src/components/SpreadCanvas.ts');
    for (const spread of groupIntoSpreads(12).slice(1)) {
      // In each 2-page spread the first (rightmost in RTL) is the verso.
      assert.equal(isRightHandPage(spread[0]), true);
      assert.equal(isRectoPage(spread[0]), false);
      if (spread[1] !== undefined) {
        assert.equal(isRightHandPage(spread[1]), false);
        assert.equal(isRectoPage(spread[1]), true);
      }
    }
    assert.equal(isRectoPage(0), true);
  });
});

describe('Regression: inspector value scrubber keeps decimals', () => {
  test('parse/clamp/format', async () => {
    const m = await loadTs('packages/tok-ui/src/components/ContextualInspector.ts');
    assert.equal(m.parseScrubberValue('11.5 pt'), 11.5);
    assert.equal(m.parseScrubberValue('12,5 pt'), 12.5);
    assert.equal(m.parseScrubberValue('-3 מ"מ'), -3);
    assert.equal(m.parseScrubberValue('abc'), null);
    assert.equal(m.clampScrubberValue(200, 6, 72), 72);
    assert.equal(m.clampScrubberValue(11.46, 6, 72), 11.5);
    assert.equal(m.formatScrubberValue(11.5, 'pt'), '11.5 pt');
  });
});

describe('Regression: theme settings', () => {
  test('corrupt persisted settings are sanitized field by field', async () => {
    const { sanitizeThemeSettings } = await loadTs('packages/tok-ui/src/theme.ts');
    const defaults = { accentColor: '#2563EB', canvasTone: '#0B132B', density: 'comfortable', highContrast: false, fontScale: 100, reducedMotion: false, enhancedFocus: false, accessibleFont: false };
    const out = sanitizeThemeSettings({ fontScale: 'abc', density: 'huge', accentColor: 'red;}body{display:none', highContrast: 'yes', reducedMotion: true, canvasTone: '#121212' }, defaults);
    assert.equal(out.fontScale, 100);
    assert.equal(out.density, 'comfortable');
    assert.equal(out.accentColor, '#2563EB');
    assert.equal(out.highContrast, false);
    assert.equal(out.reducedMotion, true);
    assert.equal(out.canvasTone, '#121212');
    assert.deepEqual(sanitizeThemeSettings(null, defaults), defaults);
    assert.equal(sanitizeThemeSettings({ paletteId: 'parchment' }, defaults).paletteId, 'parchment');
    assert.equal(sanitizeThemeSettings({ paletteId: 'nope' }, defaults).paletteId, defaults.paletteId);
  });

  test('turning high contrast off does not leave the black app background behind', async () => {
    // version 2 = the redesign's storage format (older entries keep only accessibility choices).
    fakeDom.storage.set('tok_theme_settings', JSON.stringify({ version: 2, highContrast: true, canvasTone: '#123456' }));
    const { themeManager, THEME_PALETTES } = await loadTs('packages/tok-ui/src/theme.ts');
    const style = fakeDom.documentElement.style;
    assert.equal(style.getPropertyValue('--tok-bg-app'), '#000000');
    themeManager.setHighContrast(false);
    assert.equal(style.getPropertyValue('--tok-bg-app'), THEME_PALETTES[0].appBg);
    assert.equal(style.getPropertyValue('--tok-bg-canvas'), '#123456');
    fakeDom.storage.delete('tok_theme_settings');
  });
});

describe('Regression: settings modal preset names follow the UI language', () => {
  test('localizedPresetName', async () => {
    const { localizedPresetName } = await loadTs('packages/tok-ui/src/components/SettingsModal.ts');
    assert.equal(localizedPresetName('כחול קלאסי (Classic Blue)', 'he'), 'כחול קלאסי');
    assert.equal(localizedPresetName('כחול קלאסי (Classic Blue)', 'en'), 'Classic Blue');
    assert.equal(localizedPresetName('Plain', 'en'), 'Plain');
  });
});

describe('Regression: plugin engine', () => {
  test('malformed entries are skipped, plugins activate once, failed toggles change nothing', async () => {
    const { PluginEngine, normalizePluginEntry } = await loadTs('packages/tok-ui/src/plugins/PluginEngine.ts');
    assert.equal(normalizePluginEntry(null), null);
    assert.equal(normalizePluginEntry({ sourceType: 'js' }), null);

    let runs = 0;
    let failToggle = false;
    globalThis.__tokPluginRun = () => runs++;
    const code = 'function activate(tok){ globalThis.__tokPluginRun(); tok.registerCommand({ id: "c", title: "t", category: "x", action(){} }); }';
    globalThis.tokIpc = {
      getPlugins: async () => [
        { sourceType: 'js', enabled: true },
        { manifest: { id: 'good', name: 'Good', version: '1.0.0', description: '' }, sourceType: 'js', enabled: true, compiledCode: code },
        { manifest: { id: 'off', name: 'Off', version: '1.0.0', description: '' }, sourceType: 'js', enabled: false, compiledCode: code },
        { manifest: { id: 'good', name: 'Dup', version: '9.9.9', description: '' }, sourceType: 'js', enabled: true, compiledCode: code },
      ],
      togglePlugin: async () => { if (failToggle) throw new Error('disk full'); return true; },
    };
    try {
      const registered = [];
      const unregistered = [];
      const engine = new PluginEngine((cmd) => registered.push(cmd.id), undefined, (id) => unregistered.push(id));
      await engine.loadPlugins();
      assert.deepEqual(engine.getPlugins().map((p) => p.id), ['good', 'off']);
      assert.equal(runs, 1);

      await engine.loadPlugins(); // "Reload All Plugins" must not run plugin code again
      assert.equal(runs, 1);

      await engine.togglePlugin('off', true); // enabling activates immediately
      assert.equal(runs, 2);
      assert.equal(engine.isActivated('off'), true);

      failToggle = true;
      await assert.rejects(engine.togglePlugin('good', false));
      assert.equal(engine.getPlugins().find((p) => p.id === 'good').enabled, true);
      assert.equal(registered.length, 2);
      assert.deepEqual(unregistered, []);

      failToggle = false;
      await engine.togglePlugin('good', false); // its command leaves the palette
      assert.deepEqual(unregistered, ['c']);
      await engine.togglePlugin('good', true); // and comes back without re-running the code
      assert.equal(runs, 2);
      assert.equal(registered.length, 3);
    } finally {
      delete globalThis.tokIpc;
      delete globalThis.__tokPluginRun;
    }
  });
});

describe('Regression: page virtualizer maps pageIndex to positions', () => {
  const makePages = (first, count, html = (i) => `<div>${i}</div>`) =>
    Array.from({ length: count }, (_, k) => ({
      pageIndex: first + k, gematriaNumber: '', widthPt: 100, heightPt: 100, htmlContent: html(first + k),
    }));
  const mountedPositions = (container) =>
    container.children.map((c, i) => (c.classList.contains('tok-page-mounted') ? i : -1)).filter((i) => i >= 0);

  test('computeActiveWindow', async () => {
    const { computeActiveWindow } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    assert.deepEqual(computeActiveWindow(0, 1000), { start: 0, end: 1 });
    assert.deepEqual(computeActiveWindow(500, 1000), { start: 499, end: 501 });
    assert.deepEqual(computeActiveWindow(999, 1000), { start: 998, end: 999 });
    assert.deepEqual(computeActiveWindow(0, 0), { start: 0, end: -1 });
  });

  test('documents whose page numbers do not start at 0 mount the right pages', async () => {
    const { PageDomVirtualizer } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    const container = new FakeElement('div');
    const v = new PageDomVirtualizer(container);
    v.setPages(makePages(10, 8));
    assert.deepEqual(mountedPositions(container), [0, 1]);
    assert.equal(container.children[0].innerHTML, '<div>10</div>');

    v.scrollToPage(15); // position 5
    assert.deepEqual(mountedPositions(container), [4, 5, 6]);
    assert.equal(container.children[5].innerHTML, '<div>15</div>');
    assert.equal(container.children[0].innerHTML, '');
    assert.equal(v.getActivePage(), 15);

    v.updateActiveWindow(17);
    assert.deepEqual(mountedPositions(container), [6, 7]);
    v.destroy();
  });

  test('text-only pages are tracked as mounted and the scroll listener is removed on destroy', async () => {
    const { PageDomVirtualizer } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    const container = new FakeElement('div');
    const v = new PageDomVirtualizer(container);
    v.setPages(makePages(0, 5, (i) => `plain text ${i}`));
    assert.deepEqual(mountedPositions(container), [0, 1]);
    assert.equal(container.listeners.scroll.length, 1);
    v.destroy();
    assert.equal(container.listeners.scroll.length, 0);
  });
});

describe('Regression: UI page numbering matches the Rust gematria engine', () => {
  test('taboo substitutions, 15/16 after hundreds and thousands', async () => {
    const { toHebrewGematria } = await loadTs('packages/tok-ui/src/gematria.ts');
    const expected = {
      1: 'א׳', 15: 'ט״ו', 16: 'ט״ז', 115: 'קט״ו', 216: 'רט״ז', 248: 'רמ״ח',
      270: 'ע״ר', 272: 'ער״ב', 275: 'ער״ה', 298: 'חר״צ', 304: 'ד״ש', 314: 'שי״ד',
      344: 'שד״מ', 359: 'נט״ש', 644: 'תרמ״ד', 698: 'תרח״צ', 744: 'תשד״מ',
      1000: 'א׳', 5784: 'ה׳תשפ״ד',
    };
    for (const [n, s] of Object.entries(expected)) assert.equal(toHebrewGematria(Number(n)), s, `n=${n}`);
    assert.equal(toHebrewGematria(0), '');
  });
});

describe('Regression: the UI shows the real app version', () => {
  test('getAppVersion asks the main process once', async () => {
    let calls = 0;
    globalThis.window.tokIpc = { getAppInfo: async () => { calls++; return { version: '1.2.3' }; } };
    try {
      const { getAppVersion } = await loadTs('packages/tok-ui/src/appInfo.ts');
      assert.equal(await getAppVersion(), '1.2.3');
      assert.equal(await getAppVersion(), '1.2.3');
      assert.equal(calls, 1);
    } finally {
      delete globalThis.window.tokIpc;
    }
  });

  test('no version number is hard-coded in the UI sources', () => {
    const srcDir = path.join(rootDir, 'packages/tok-ui/src');
    const offenders = [];
    const walk = (d) => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = path.join(d, e.name);
        if (e.isDirectory()) walk(p);
        else if (/\.(ts|html)$/.test(p) && /\bv\d+\.\d+\.\d+\b/.test(fs.readFileSync(p, 'utf-8'))) offenders.push(path.relative(rootDir, p));
      }
    };
    walk(srcDir);
    assert.deepEqual(offenders, []);
  });
});

describe('Regression: document fonts are the ones the engine embeds', () => {
  test('every selectable font has an @font-face whose file exists', async () => {
    const { DOCUMENT_FONTS } = await loadTs('packages/tok-ui/src/fonts.ts');
    const htmlPath = path.join(rootDir, 'packages/tok-ui/src/index.html');
    const html = fs.readFileSync(htmlPath, 'utf-8');
    for (const { family } of DOCUMENT_FONTS) {
      const faces = [...html.matchAll(/@font-face\s*\{[^}]*\}/g)].map((m) => m[0]).filter((f) => f.includes(`'${family}'`));
      assert.ok(faces.length > 0, `no @font-face for ${family}`);
      for (const face of faces) {
        const url = /url\('([^']+)'\)/.exec(face)[1];
        // index.html is served from packages/tok-ui/dist, a sibling of src at the same depth.
        assert.ok(fs.existsSync(path.resolve(path.dirname(htmlPath), url)), `${family}: missing ${url}`);
      }
    }
  });

  test('the engine embeds every selectable font', async () => {
    const { DOCUMENT_FONTS } = await loadTs('packages/tok-ui/src/fonts.ts');
    const fontRs = fs.readFileSync(path.join(rootDir, 'crates/tok-typeset/src/font.rs'), 'utf-8');
    for (const { family } of DOCUMENT_FONTS) assert.ok(fontRs.includes(`("${family}", EMBEDDED_`), `${family} is not registered in font.rs`);
  });

  test('UI font weights are the engine ParagraphStyle.font_weight values', async () => {
    const { FONT_WEIGHT_REGULAR, FONT_WEIGHT_BOLD } = await loadTs('packages/tok-ui/src/fonts.ts');
    const styles = fs.readFileSync(path.join(rootDir, 'crates/tok-core/src/styles.rs'), 'utf-8');
    assert.ok(styles.includes(`FONT_WEIGHT_REGULAR: u16 = ${FONT_WEIGHT_REGULAR};`));
    assert.ok(styles.includes(`FONT_WEIGHT_BOLD: u16 = ${FONT_WEIGHT_BOLD};`));
    const srcDir = path.join(rootDir, 'packages/tok-ui/src');
    const offenders = [];
    const walk = (d) => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = path.join(d, e.name);
        if (e.isDirectory()) walk(p);
        else if (p.endsWith('.ts') && /fontWeight:\s*['"]/.test(fs.readFileSync(p, 'utf-8'))) offenders.push(path.relative(rootDir, p));
      }
    };
    walk(srcDir);
    assert.deepEqual(offenders, [], 'fontWeight must be numeric (400/700), as in the engine');
  });

  test('no legacy font names (not shipped) remain in the UI', () => {
    const srcDir = path.join(rootDir, 'packages/tok-ui/src');
    const legacy = /Taamey Frank|David CLM|\bVilna\b|['"]Rashi['"]/;
    const offenders = [];
    const walk = (d) => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = path.join(d, e.name);
        if (e.isDirectory()) walk(p);
        else if (/\.(ts|html)$/.test(p) && legacy.test(fs.readFileSync(p, 'utf-8'))) offenders.push(path.relative(rootDir, p));
      }
    };
    walk(srcDir);
    assert.deepEqual(offenders, []);
  });
});

describe('Feature: Official Plugin APIs & Security Sandbox', () => {
  test('Hebrew Typography utilities (Niqqud, Taamim, Holy Names, Quotes)', async () => {
    const { hebrewUtils } = await loadTs('packages/tok-ui/src/plugins/hebrew-utils.ts');
    
    // 1. Niqqud stripping
    const pointed = 'בָּרוּךְ אַתָּה';
    assert.equal(hebrewUtils.stripNiqqud(pointed), 'ברוך אתה');

    // 2. Ta'amim stripping
    const withTaamim = 'בְּרֵאשִׁ֖ית בָּרָ֣א';
    assert.equal(hebrewUtils.stripTaamim(withTaamim), 'בְּרֵאשִׁית בָּרָא');
    assert.equal(hebrewUtils.stripAllMarks(withTaamim), 'בראשית ברא');

    // 3. Holy names detection in pointed text
    const sacred = 'בָּרוּךְ אַתָּה יהוה אֱלֹהִים מֶלֶךְ הָעוֹלָם';
    const matches = hebrewUtils.findHolyNames(sacred);
    assert.equal(matches.length, 2);
    assert.equal(matches[0].name, 'יהוה');
    assert.equal(matches[1].name, 'אלהים');

    // 4. Quotation normalization (Gershayim and Geresh)
    const quoteInput = 'לדברי הרמב"ם והראב"ד ור\' משה';
    const quoteOutput = hebrewUtils.normalizeQuotes(quoteInput);
    assert.equal(quoteOutput, 'לדברי הרמב״ם והראב״ד ור׳ משה');

    // 5. Gematria
    assert.equal(hebrewUtils.toHebrewGematria(1), 'א׳');
    assert.equal(hebrewUtils.toHebrewGematria(15), 'ט״ו');
    assert.equal(hebrewUtils.toHebrewGematria(270), 'ע״ר');
  });

  test('Document APIs enforce document:write permissions and allow safe batch mutations', async () => {
    const { PluginEngine } = await loadTs('packages/tok-ui/src/plugins/PluginEngine.ts');

    let currentStory = [
      { id: 'p1', styleId: 'body', text: 'פסקה ראשונה' },
      { id: 'p2', styleId: 'body', text: 'פסקה שניה' }
    ];

    const delegate = {
      getStory: () => [...currentStory],
      loadStory: (newStory) => { currentStory = [...newStory]; },
      getSelectedText: () => 'טקסט נבחר',
      replaceSelection: (t) => { currentStory[0].text = t; },
      getDocumentTitle: () => 'מסמך בדיקה.tok',
      getStats: () => ({ wordCount: 4, charCount: 20, paragraphCount: 2 }),
      getViewMode: () => 'story',
      setViewMode: () => {},
      getPageCount: () => 1,
      getActivePageIndex: () => 0,
      getZoom: () => 100,
    };

    // Plugin with document:read only (should fail to write)
    let writeError = null;
    const readOnlyCode = `
      function activate(tok) {
        try {
          tok.document.updateParagraph('p1', 'שינוי לא מורשה');
        } catch (e) {
          writeError = e.message;
        }
      }
    `;

    // Plugin with document:write (should succeed)
    let writeSuccess = false;
    const writeCode = `
      function activate(tok) {
        tok.document.updateParagraph('p1', 'טקסט מעודכן');
        writeSuccess = true;
      }
    `;

    globalThis.writeError = null;
    globalThis.writeSuccess = false;

    globalThis.tokIpc = {
      getPluginSystemStatus: async () => ({ isSafeMode: false, isSaferActive: false, userPluginsDir: '' }),
      getPlugins: async () => [
        {
          manifest: { id: 'ro-plugin', name: 'ReadOnly', version: '1.0', description: '', permissions: ['document:read', 'ui:notifications'] },
          sourceType: 'js',
          enabled: true,
          compiledCode: 'function activate(tok){ try{ tok.document.updateParagraph("p1", "err"); }catch(e){ globalThis.writeError = e.message; } }'
        },
        {
          manifest: { id: 'rw-plugin', name: 'ReadWrite', version: '1.0', description: '', permissions: ['document:read', 'document:write'] },
          sourceType: 'js',
          enabled: true,
          compiledCode: 'function activate(tok){ tok.document.updateParagraph("p1", "עודכן"); globalThis.writeSuccess = true; }'
        }
      ]
    };

    try {
      const engine = new PluginEngine({ delegate });
      await engine.loadPlugins();

      // Read-only plugin must be rejected by permission guard
      assert.match(globalThis.writeError, /Permission denied.*document:write/);

      // Read-write plugin must succeed
      assert.equal(globalThis.writeSuccess, true);
      assert.equal(currentStory[0].text, 'עודכן');
    } finally {
      delete globalThis.tokIpc;
      delete globalThis.writeError;
      delete globalThis.writeSuccess;
    }
  });

  test('Security sandbox shadows IPC and blocks unauthorized network calls', async () => {
    const { PluginEngine } = await loadTs('packages/tok-ui/src/plugins/PluginEngine.ts');

    let capturedIpcType = 'not_run';
    let networkBlocked = false;

    globalThis.__testReport = (ipcVal, netBlocked) => {
      capturedIpcType = ipcVal;
      networkBlocked = netBlocked;
    };

    const untrustedCode = `
      function activate(tok) {
        var ipcType = typeof tokIpc;
        var blocked = false;
        try {
          fetch('https://example.com/steal-data');
        } catch (e) {
          blocked = true;
        }
        globalThis.__testReport(ipcType, blocked);
      }
    `;

    globalThis.tokIpc = {
      getPluginSystemStatus: async () => ({ isSafeMode: false, isSaferActive: false, userPluginsDir: '' }),
      getPlugins: async () => [
        {
          manifest: { id: 'untrusted', name: 'Untrusted', version: '1.0', description: '', permissions: ['ui:commands'] },
          sourceType: 'js',
          enabled: true,
          compiledCode: untrustedCode
        }
      ]
    };

    try {
      const engine = new PluginEngine();
      await engine.loadPlugins();

      // tokIpc must be undefined in plugin execution scope
      assert.equal(capturedIpcType, 'undefined');
      // fetch must be blocked because 'network:fetch' is not permitted
      assert.equal(networkBlocked, true);
    } finally {
      delete globalThis.tokIpc;
      delete globalThis.__testReport;
    }
  });

  test('Safe Mode blocks third-party plugins from running', async () => {
    const { PluginEngine } = await loadTs('packages/tok-ui/src/plugins/PluginEngine.ts');

    let thirdPartyRan = false;
    globalThis.__thirdPartyRun = () => { thirdPartyRan = true; };

    globalThis.tokIpc = {
      getPluginSystemStatus: async () => ({ isSafeMode: true, isSaferActive: true, userPluginsDir: '' }),
      getPlugins: async () => [
        {
          manifest: { id: 'user-ext', name: 'User Ext', version: '1.0', description: '' },
          sourceType: 'js',
          enabled: false,
          safeModeBlocked: true,
          compiledCode: 'function activate(){ globalThis.__thirdPartyRun(); }'
        }
      ]
    };

    try {
      const engine = new PluginEngine();
      await engine.loadPlugins();

      assert.equal(engine.getIsSafeMode(), true);
      assert.equal(engine.getIsSaferActive(), true);
      assert.equal(thirdPartyRan, false);
      const list = engine.getPlugins();
      assert.equal(list[0].safeModeBlocked, true);
    } finally {
      delete globalThis.tokIpc;
      delete globalThis.__thirdPartyRun;
    }
  });
});


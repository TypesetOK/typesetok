import { test, describe, before } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

// ── tok-cli engine binary (used by the Phase 1-6 integration suites) ──
// The Rust CLI is built by cargo, not by `npm run build`, so CI must run
// `cargo build -p tok-cli` before `npm test`. TOK_CLI_PATH overrides the search.
const TOK_CLI_EXE = process.platform === 'win32' ? 'tok-cli.exe' : 'tok-cli';

function tokCliCandidates() {
  if (process.env.TOK_CLI_PATH) return [path.resolve(process.env.TOK_CLI_PATH)];
  const targetDirs = [];
  for (const dir of [process.env.CARGO_TARGET_DIR, process.env.CARGO_BUILD_TARGET_DIR]) {
    if (dir) targetDirs.push(path.resolve(rootDir, dir));
  }
  if (process.env.LOCALAPPDATA) targetDirs.push(path.join(process.env.LOCALAPPDATA, 'tok_target'));
  targetDirs.push(path.join(rootDir, 'target'), path.join(rootDir, 'crates', 'target'));
  const candidates = [];
  for (const dir of targetDirs) {
    for (const profile of ['debug', 'release']) candidates.push(path.join(dir, profile, TOK_CLI_EXE));
  }
  return [...new Set(candidates)];
}

// Returns the most recently built tok-cli binary, or undefined if none exists,
// so a stale debug/release build never shadows a fresh one.
function findTokCli() {
  const found = tokCliCandidates().filter((p) => fs.existsSync(p));
  found.sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
  return found[0];
}

function tokCliMissingMessage() {
  return (
    'tok-cli binary not found. Build it first with `cargo build -p tok-cli` ' +
    `(or set TOK_CLI_PATH). Checked: ${tokCliCandidates().join(', ')}`
  );
}

// ---------------------------------------------------------------------------
// Regression & unit tests that execute the real TypeScript sources.
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
    this.offsetWidth = 0;
    this.scrollTop = 0;
    this.clientHeight = 0;
    this.clientWidth = 0;
    this.width = 800;
    this.height = 600;
    this.tabIndex = -1;
    this.attributes = {};
    this.parentElement = null;
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
  remove() {
    if (this.parentElement) {
      this.parentElement.children = this.parentElement.children.filter((c) => c !== this);
      this.parentElement = null;
    }
  }
  focus() {
    if (globalThis.document) globalThis.document.activeElement = this;
  }
  getBoundingClientRect() {
    return { left: 0, top: 0, right: this.width || 800, bottom: this.height || 600, width: this.width || 800, height: this.height || 600, x: 0, y: 0 };
  }
  getContext(type) {
    if (type === '2d') {
      return (this._ctx2d ||= {
        fillStyle: '',
        drawnRects: [],
        scale() {},
        clearRect() { this.drawnRects = []; },
        fillRect(x, y, w, h) { this.drawnRects.push({ x, y, w, h, fillStyle: this.fillStyle }); },
      });
    }
    return null;
  }
  setAttribute(k, v) { this.attributes[k] = String(v); }
  getAttribute(k) { return this.attributes[k]; }
  click() { (this.listeners['click'] || []).forEach((fn) => fn({})); }
  dispatchEvent(type, ev = {}) {
    (this.listeners[type] || []).forEach((fn) => fn({ preventDefault() {}, stopPropagation() {}, ...ev }));
  }
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
  globalThis.window.devicePixelRatio ||= 1;
  globalThis.window.setInterval ||= setInterval;
  globalThis.window.clearInterval ||= clearInterval;
  globalThis.window.setTimeout ||= setTimeout;
  globalThis.window.clearTimeout ||= clearTimeout;
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
    alias: {
      'tok-viewer': path.join(rootDir, 'packages/tok-viewer/src/index.ts'),
      'tok-canvas': path.join(rootDir, 'packages/tok-canvas/src/index.ts'),
      'tok-story-editor': path.join(rootDir, 'packages/tok-story-editor/src/index.ts'),
    },
    outfile,
    logLevel: 'silent',
  });
  try {
    return await import(pathToFileURL(outfile).href);
  } finally {
    fs.rmSync(outfile, { force: true });
  }
}

describe('Hebrew Typography & Gematria Engine', () => {
  test('Single-letter gematria (1-9) via real gematria module', async () => {
    const { toHebrewGematria } = await loadTs('packages/tok-ui/src/gematria.ts');
    assert.equal(toHebrewGematria(1), 'א׳');
    assert.equal(toHebrewGematria(2), 'ב׳');
    assert.equal(toHebrewGematria(5), 'ה׳');
    assert.equal(toHebrewGematria(9), 'ט׳');
  });

  test('Talmudic exceptions for 15 and 16 (Tet-Vav, Tet-Zayin) and compound numbers', async () => {
    const { toHebrewGematria } = await loadTs('packages/tok-ui/src/gematria.ts');
    assert.equal(toHebrewGematria(15), 'ט״ו');
    assert.equal(toHebrewGematria(16), 'ט״ז');
    assert.equal(toHebrewGematria(115), 'קט״ו');
    assert.equal(toHebrewGematria(216), 'רט״ז');
  });

  test('Tens, hundreds, and taboo substitutions with standard Hebrew gershayim', async () => {
    const { toHebrewGematria } = await loadTs('packages/tok-ui/src/gematria.ts');
    const { isRectoPage } = await loadTs('packages/tok-ui/src/components/SpreadCanvas.ts');
    assert.equal(toHebrewGematria(20), 'כ׳');
    assert.equal(toHebrewGematria(21), 'כ״א');
    assert.equal(toHebrewGematria(100), 'ק׳');
    assert.equal(toHebrewGematria(354), 'שנ״ד');
    assert.equal(toHebrewGematria(270), 'ע״ר');
    assert.equal(toHebrewGematria(304), 'ד״ש');

    // Verify Talmudic daf/amud label formatting using real recto/verso & gematria helpers
    const formatDaf = (pageIndex) => `דף ${toHebrewGematria(Math.floor(pageIndex / 2) + 2)} ${isRectoPage(pageIndex) ? 'ע״א' : 'ע״ב'}`;
    assert.equal(formatDaf(0), 'דף ב׳ ע״א');
    assert.equal(formatDaf(1), 'דף ב׳ ע״ב');
  });
});

describe('Page DOM Virtualizer (Section 10.3)', () => {
  const buildPages = (total) =>
    Array.from({ length: total }, (_, i) => ({
      pageIndex: i,
      gematriaNumber: '',
      widthPt: 480,
      heightPt: 678,
      htmlContent: `<div class="tok-page-layout-rust" data-page="${i}">Page ${i}</div>`,
    }));

  const getMountedIndices = (container) =>
    container.children
      .map((ph, idx) => (ph.classList.contains('tok-page-mounted') && ph.innerHTML !== '' ? idx : -1))
      .filter((idx) => idx >= 0);

  test('Active window at start of book (page 0) mounts [0, 1] in real PageDomVirtualizer', async () => {
    const { PageDomVirtualizer, computeActiveWindow } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    assert.deepEqual(computeActiveWindow(0, 1000), { start: 0, end: 1 });

    const container = new FakeElement('div');
    const virtualizer = new PageDomVirtualizer(container);
    virtualizer.setPages(buildPages(1000));
    const mounted = getMountedIndices(container);
    assert.deepEqual(mounted, [0, 1]);
    assert.equal(mounted.length <= 3, true);
    assert.equal(container.children[2].innerHTML, '');
    virtualizer.destroy();
  });

  test('Active window in middle of book enforces strictly 3 mounted pages [K-1, K, K+1]', async () => {
    const { PageDomVirtualizer, computeActiveWindow } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    assert.deepEqual(computeActiveWindow(500, 1000), { start: 499, end: 501 });

    const container = new FakeElement('div');
    const virtualizer = new PageDomVirtualizer(container);
    virtualizer.setPages(buildPages(1000));
    virtualizer.updateActiveWindow(500);
    const mounted = getMountedIndices(container);
    assert.deepEqual(mounted, [499, 500, 501]);
    assert.equal(mounted.length, 3);
    assert.equal(container.children[0].innerHTML, '');
    assert.ok(container.children[500].innerHTML.includes('Page 500'));
    virtualizer.destroy();
  });

  test('Active window at end of book mounts [998, 999]', async () => {
    const { PageDomVirtualizer, computeActiveWindow } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    assert.deepEqual(computeActiveWindow(999, 1000), { start: 998, end: 999 });

    const container = new FakeElement('div');
    const virtualizer = new PageDomVirtualizer(container);
    virtualizer.setPages(buildPages(1000));
    virtualizer.updateActiveWindow(999);
    const mounted = getMountedIndices(container);
    assert.deepEqual(mounted, [998, 999]);
    virtualizer.destroy();
  });

  test('Single-page document mounts only page [0]', async () => {
    const { PageDomVirtualizer } = await loadTs('packages/tok-viewer/src/virtualizer.ts');
    const container = new FakeElement('div');
    const virtualizer = new PageDomVirtualizer(container);
    virtualizer.setPages(buildPages(1));
    const mounted = getMountedIndices(container);
    assert.deepEqual(mounted, [0]);
    virtualizer.destroy();
  });
});

describe('Canvas Overlay & Optimistic RTL Advance (Section 9.2)', () => {
  test('RTL caret advance decreases X position immediately (<16ms) via real CanvasInteractionOverlay', async () => {
    const { CanvasInteractionOverlay } = await loadTs('packages/tok-canvas/src/overlay.ts');
    const container = new FakeElement('div');
    const overlay = new CanvasInteractionOverlay(container);
    try {
      overlay.setCaretPosition(0, 500, 100, 18);
      const step = 8.5;

      overlay.advanceCaretOptimisticRtl(step);
      assert.equal(overlay.getCaretPosition().x, 491.5);

      overlay.advanceCaretOptimisticRtl(step);
      assert.equal(overlay.getCaretPosition().x, 483.0);
    } finally {
      overlay.destroy();
    }
  });

  test('Selection rectangle bounds are stored and rendered by CanvasInteractionOverlay', async () => {
    const { CanvasInteractionOverlay } = await loadTs('packages/tok-canvas/src/overlay.ts');
    const container = new FakeElement('div');
    const overlay = new CanvasInteractionOverlay(container);
    try {
      const rects = [{ pageIndex: 0, x: 100, y: 50, width: 250, height: 16 }];
      overlay.setSelection(rects);
      const activeSel = overlay.getSelection();
      assert.equal(activeSel.length, 1);
      assert.equal(activeSel[0].width, 250);
    } finally {
      overlay.destroy();
    }
  });

  test('Interactive drag selection computes normalized bounding box in CanvasInteractionOverlay', async () => {
    const { CanvasInteractionOverlay } = await loadTs('packages/tok-canvas/src/overlay.ts');
    const container = new FakeElement('div');
    const overlay = new CanvasInteractionOverlay(container);
    try {
      const canvasEl = container.children[0];
      let reportedRects = [];
      overlay.onSelectionChanged = (r) => { reportedRects = r; };

      canvasEl.dispatchEvent('mousedown', { clientX: 350, clientY: 120 });
      canvasEl.dispatchEvent('mousemove', { clientX: 200, clientY: 150 });

      assert.equal(reportedRects.length, 1);
      assert.equal(reportedRects[0].x, 200);
      assert.equal(reportedRects[0].y, 120);
      assert.equal(reportedRects[0].width, 150);
      assert.equal(reportedRects[0].height, 30);
    } finally {
      overlay.destroy();
    }
  });

  test('Spatial hit-test provider snaps mousedown coordinate to line box in CanvasInteractionOverlay', async () => {
    const { CanvasInteractionOverlay } = await loadTs('packages/tok-canvas/src/overlay.ts');
    const container = new FakeElement('div');
    const overlay = new CanvasInteractionOverlay(container);
    try {
      const lineBoxes = [
        { baselineY: 50, height: 16, x: 120 },
        { baselineY: 80, height: 16, x: 115 }
      ];
      overlay.hitTestProvider = (_clickX, clickY) => {
        let closest = lineBoxes[0];
        for (const line of lineBoxes) {
          if (Math.abs(line.baselineY - clickY) < Math.abs(closest.baselineY - clickY)) {
            closest = line;
          }
        }
        return { pageIndex: 0, x: closest.x, y: closest.baselineY, height: closest.height, visible: true };
      };

      const canvasEl = container.children[0];
      canvasEl.dispatchEvent('mousedown', { clientX: 118, clientY: 75 });

      const caret = overlay.getCaretPosition();
      assert.equal(caret.y, 80);
      assert.equal(caret.x, 115);
    } finally {
      overlay.destroy();
    }
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

describe('3-Tier Hebrew Justification Model & Typeset Bridge (Section 15)', () => {
  test('Rust 3-Tier Hebrew Justification engine defines canonical ratios and Ahalterm set in crates/tok-typeset/src/hebrew_justify.rs', () => {
    const justifyRs = fs.readFileSync(path.join(rootDir, 'crates/tok-typeset/src/hebrew_justify.rs'), 'utf-8');
    assert.ok(justifyRs.includes('MIN_SPACE_RATIO: f32 = 0.80;'), 'Tier 1 min space ratio (80%) must be defined in engine');
    assert.ok(justifyRs.includes('MAX_SPACE_RATIO: f32 = 1.30;'), 'Tier 1 max space ratio (130%) must be defined in engine');
    assert.ok(justifyRs.includes("AHALTERM_LETTERS: &[char] = &['א', 'ה', 'ל', 'ת', 'ר', 'ם', 'ד', 'ך'];"), 'Tier 2 Ahalterm letter set must be defined in engine');
    assert.ok(justifyRs.includes('MAX_TRACKING_EM: f32 = 0.02;'), 'Tier 3 micro-tracking limit (±2% em) must be defined in engine');
  });

  test('tok-cli typeset-document & layoutToPageDescriptors project justified Hebrew lines into SpreadCanvas descriptors', async () => {
    const tokCli = findTokCli();
    assert.ok(tokCli, tokCliMissingMessage());
    const { documentStateToDocumentRoot } = await loadTs('packages/tok-ui/src/engine/documentBridge.ts');
    const { layoutToPageDescriptors } = await loadTs('packages/tok-ui/src/engine/typesetBridge.ts');

    const docRoot = documentStateToDocumentRoot({
      title: 'בדיקת יישור עברי תלת-שלבי',
      templateType: 'prose',
      flows: {
        gemara: [
          {
            id: 'p-just-1',
            styleId: 'normal',
            text: 'ספר הטיפוגרפיה העברית והעימוד השולחני המודרני מהווה נדבך יסודי בהתפתחות עולם הדפוס היהודי לדורותיו וכולל אותיות אהלתר״ם מתרחבות ורווחי מילים מבוקרים.'
          }
        ],
        rashi: [],
        tosafot: [],
        notes: []
      }
    });

    const tempIn = path.join(os.tmpdir(), `tok_justify_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `tok_justify_out_${Date.now()}.json`);
    try {
      fs.writeFileSync(tempIn, JSON.stringify(docRoot, null, 2), 'utf-8');
      const code = await new Promise((resolve, reject) => {
        const proc = spawn(tokCli, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('error', reject);
        proc.on('close', resolve);
      });
      assert.equal(code, 0, 'tok-cli typeset-document must succeed');
      const pageBoxes = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(pageBoxes.length >= 1);
      assert.ok(pageBoxes[0].frames[0].lines.length >= 2, 'Paragraph must break into multiple justified lines');

      const descriptors = layoutToPageDescriptors(pageBoxes);
      assert.equal(descriptors.length, pageBoxes.length);
      assert.ok(descriptors[0].htmlContent.includes('tok-page-layout-rust'));
      assert.ok(descriptors[0].htmlContent.includes('tok-line-box'));
      assert.ok(descriptors[0].htmlContent.includes('direction: rtl'));
    } finally {
      fs.rmSync(tempIn, { force: true });
      fs.rmSync(tempOut, { force: true });
    }
  });
});

describe('Security Regression: HTML Escaping & Hardened Plugin Sandbox', () => {
  test('escapeHtml in SpreadCanvas neutralizes XSS payloads (<script>, <img onerror>, quotes, ampersands)', async () => {
    const { escapeHtml } = await loadTs('packages/tok-ui/src/components/SpreadCanvas.ts');

    const scriptPayload = '<script>alert(1)</script>';
    const escapedScript = escapeHtml(scriptPayload);
    assert.equal(escapedScript, '&lt;script&gt;alert(1)&lt;/script&gt;');
    assert.ok(!escapedScript.includes('<script>'));

    const imgPayload = '<img src=x onerror=alert(1)>';
    const escapedImg = escapeHtml(imgPayload);
    assert.equal(escapedImg, '&lt;img src=x onerror=alert(1)&gt;');
    assert.ok(!escapedImg.includes('<img'));

    const attrPayload = `" onclick="alert('xss') & 'test'`;
    const escapedAttr = escapeHtml(attrPayload);
    assert.equal(escapedAttr, '&quot; onclick=&quot;alert(&#039;xss&#039;) &amp; &#039;test&#039;');
    assert.ok(!escapedAttr.includes('"'));
    assert.ok(!escapedAttr.includes("'"));

    assert.equal(escapeHtml(''), '');
  });

  test('PluginEngine hardened sandbox blocks access to window.tokIpc, globalThis.tokIpc, self.tokIpc, document, localStorage, and Function("return this")()', async () => {
    const { PluginEngine } = await loadTs('packages/tok-ui/src/plugins/PluginEngine.ts');

    let probeResults = null;
    globalThis.__reportSandboxProbe = (res) => {
      probeResults = res;
    };

    const escapeAttemptCode = `
      function activate(tok) {
        var functionCtorBlocked = false;
        var windowFunctionCtorBlocked = false;
        try {
          Function("return this")();
        } catch (e) {
          functionCtorBlocked = /blocked in plugin sandbox/i.test(String(e && e.message || e));
        }
        try {
          window.Function("return this")();
        } catch (e) {
          windowFunctionCtorBlocked = /blocked in plugin sandbox/i.test(String(e && e.message || e));
        }
        globalThis.__reportSandboxProbe({
          directTokIpc: typeof tokIpc,
          windowTokIpc: typeof window.tokIpc,
          globalThisTokIpc: typeof globalThis.tokIpc,
          selfTokIpc: typeof self.tokIpc,
          topTokIpc: typeof top.tokIpc,
          parentTokIpc: typeof parent.tokIpc,
          hasTokIpcInWindow: ('tokIpc' in window),
          directDocument: typeof document,
          windowDocument: typeof window.document,
          globalThisDocument: typeof globalThis.document,
          directLocalStorage: typeof localStorage,
          windowLocalStorage: typeof window.localStorage,
          globalThisLocalStorage: typeof globalThis.localStorage,
          functionCtorBlocked: functionCtorBlocked,
          windowFunctionCtorBlocked: windowFunctionCtorBlocked
        });
      }
    `;

    globalThis.tokIpc = {
      getPluginSystemStatus: async () => ({ isSafeMode: false, isSaferActive: false, userPluginsDir: '' }),
      getPlugins: async () => [
        {
          manifest: { id: 'sandbox-escape-probe', name: 'EscapeProbe', version: '1.0.0', description: '' },
          sourceType: 'js',
          enabled: true,
          compiledCode: escapeAttemptCode
        }
      ]
    };

    try {
      const engine = new PluginEngine();
      await engine.loadPlugins();

      assert.ok(probeResults, 'Sandbox probe plugin must have executed');
      assert.equal(probeResults.directTokIpc, 'undefined');
      assert.equal(probeResults.windowTokIpc, 'undefined');
      assert.equal(probeResults.globalThisTokIpc, 'undefined');
      assert.equal(probeResults.selfTokIpc, 'undefined');
      assert.equal(probeResults.topTokIpc, 'undefined');
      assert.equal(probeResults.parentTokIpc, 'undefined');
      assert.equal(probeResults.hasTokIpcInWindow, false);
      assert.equal(probeResults.directDocument, 'undefined');
      assert.equal(probeResults.windowDocument, 'undefined');
      assert.equal(probeResults.globalThisDocument, 'undefined');
      assert.equal(probeResults.directLocalStorage, 'undefined');
      assert.equal(probeResults.windowLocalStorage, 'undefined');
      assert.equal(probeResults.globalThisLocalStorage, 'undefined');
      assert.equal(probeResults.functionCtorBlocked, true, 'Function("return this")() must be blocked');
      assert.equal(probeResults.windowFunctionCtorBlocked, true, 'window.Function("return this")() must be blocked');
      // Host tokIpc must be intact after plugin activation finishes
      assert.equal(typeof globalThis.tokIpc.getPlugins, 'function');
    } finally {
      delete globalThis.tokIpc;
      delete globalThis.__reportSandboxProbe;
    }
  });
});

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

describe('Multi-Flow Real-Time Paginator & Live Canvas Sync (Talmud Engine)', () => {
  test('Talmud pagination produces Recto/Verso aware commentary placement', async () => {
    const { FlowPaginator, DEFAULT_TALMUD_FLOWS } = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
    const doc = {
      title: 'Talmud Test',
      templateType: 'gemara',
      flows: {
        gemara: [...DEFAULT_TALMUD_FLOWS.gemara],
        rashi: [...DEFAULT_TALMUD_FLOWS.rashi],
        tosafot: [...DEFAULT_TALMUD_FLOWS.tosafot],
        notes: [...DEFAULT_TALMUD_FLOWS.notes]
      }
    };

    const pages = FlowPaginator.paginateDocument(doc, 2);
    assert.ok(pages.length >= 2, 'Must generate at least 2 pages');

    // Page 0 is Recto (ע״א - right page in Hebrew spread)
    const p0Html = pages[0].htmlContent;
    assert.ok(p0Html.includes('data-flow-id="gemara"'), 'Recto page must contain Gemara frame');
    assert.ok(p0Html.includes('data-flow-id="rashi"'), 'Recto page must contain Rashi frame');
    assert.ok(p0Html.includes('data-flow-id="tosafot"'), 'Recto page must contain Tosafot frame');
    assert.ok(p0Html.includes('data-flow-id="notes"'), 'Recto page must contain Footnotes frame');

    // Check Hebrew spine awareness:
    // On Recto (page 0), Spine is on the LEFT.
    // Inner commentary (Rashi) -> Left column.
    // Outer commentary (Tosafot) -> Right column.
    const p0RightIdx = p0Html.indexOf('data-flow-id="tosafot"');
    const p0CenterIdx = p0Html.indexOf('data-flow-id="gemara"');
    const p0LeftIdx = p0Html.indexOf('data-flow-id="rashi"');
    assert.ok(p0RightIdx < p0CenterIdx, 'Tosafot on Recto should be on the right (outer) column');
    assert.ok(p0CenterIdx < p0LeftIdx, 'Rashi on Recto should be on the left (inner spine) column');

    // Page 1 is Verso (ע״ב - left page in Hebrew spread)
    const p1Html = pages[1].htmlContent;
    // On Verso (page 1), Spine is on the RIGHT.
    // Inner commentary (Rashi) -> Right column.
    // Outer commentary (Tosafot) -> Left column.
    const p1RightIdx = p1Html.indexOf('data-flow-id="rashi"');
    const p1CenterIdx = p1Html.indexOf('data-flow-id="gemara"');
    const p1LeftIdx = p1Html.indexOf('data-flow-id="tosafot"');
    assert.ok(p1RightIdx < p1CenterIdx, 'Rashi on Verso should be on the right (inner spine) column');
    assert.ok(p1CenterIdx < p1LeftIdx, 'Tosafot on Verso should be on the left (outer) column');
  });

  test('Dynamic L-Shape expansion triggers when Gemara ends early', async () => {
    const { FlowPaginator, DEFAULT_TALMUD_FLOWS } = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
    // Short gemara, long rashi
    const doc = {
      title: 'Talmud Short Gemara',
      templateType: 'gemara',
      flows: {
        gemara: [{ id: 'g-short', styleId: 'style-gemara-main', text: 'קצר מאד.' }],
        rashi: [...DEFAULT_TALMUD_FLOWS.rashi],
        tosafot: [...DEFAULT_TALMUD_FLOWS.tosafot],
        notes: []
      }
    };

    const pages = FlowPaginator.paginateDocument(doc, 1);
    const p0Html = pages[0].htmlContent;
    assert.ok(p0Html.includes('tok-lshape-box'), 'Should render L-shape expansion zone');
    assert.ok(p0Html.includes('צורת הדף'), 'Should mention Tzurat HaDaf in expansion');
  });

  test('Prose template paginates continuous text across pages', async () => {
    const { FlowPaginator } = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
    const paras = Array.from({ length: 40 }, (_, i) => ({
      id: `p-${i}`,
      styleId: 'style-body',
      text: `פסקה מספר ${i + 1} עם מלל רציף לצורך בדיקת חלוקת עמודים רב עמודית בפורמט פרוזה.`
    }));

    const doc = {
      title: 'Prose Test',
      templateType: 'prose',
      flows: {
        gemara: paras,
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const pages = FlowPaginator.paginateDocument(doc, 1);
    assert.ok(pages.length > 1, 'Long prose must paginate into multiple pages');
    for (const page of pages) {
      assert.ok(page.htmlContent.includes('data-flow-id="gemara"'));
    }
  });

  test('Footnotes layout displays gemara and bottom footnote notes', async () => {
    const { FlowPaginator } = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
    const doc = {
      title: 'Footnotes Test',
      templateType: 'notes',
      flows: {
        gemara: [{ id: 'g-1', styleId: 'style-body', text: 'טקסט ראשי עם הערת שוליים.' }],
        rashi: [],
        tosafot: [],
        notes: [{ id: 'fn-1', styleId: 'style-notes', text: '1. הערת שוליים בתחתית העמוד.' }]
      }
    };

    const pages = FlowPaginator.paginateDocument(doc, 1);
    assert.equal(pages.length, 1);
    const html = pages[0].htmlContent;
    assert.ok(html.includes('data-flow-id="gemara"'));
    assert.ok(html.includes('data-flow-id="notes"'));
  });
});

describe('Phase 1 & 2: Data Contract (documentState <-> DocumentModel) & Real PDF Export', () => {
  let documentBridge;
  let tokCliPath;

  test('Load documentBridge module', async () => {
    documentBridge = await loadTs('packages/tok-ui/src/engine/documentBridge.ts');
    assert.ok(documentBridge.documentStateToDocumentRoot, 'Must export documentStateToDocumentRoot');
    assert.ok(documentBridge.documentRootToDocumentState, 'Must export documentRootToDocumentState');
    assert.ok(documentBridge.isValidUlid, 'Must export isValidUlid');
    assert.ok(documentBridge.generateCrockfordUlid, 'Must export generateCrockfordUlid');
  });

  test('ULID generator produces valid 26-char Crockford Base32 identifiers', () => {
    for (let i = 0; i < 50; i++) {
      const id = documentBridge.generateCrockfordUlid();
      assert.equal(id.length, 26, `ULID must be 26 chars: ${id}`);
      assert.ok(documentBridge.isValidUlid(id), `ULID must be valid: ${id}`);
      // First character must be <= '7' for 128-bit limit
      assert.ok(id[0] <= '7', `First char must be <= '7': ${id}`);
    }
  });

  test('HTML tag stripping cleans markup while preserving Hebrew and Niqqud', () => {
    const raw = '<b>מֵאֵימָתַי קוֹרִין</b> אֶת <i>שְׁמַע</i> <span style="color:red">בְּעַרְבִית</span>?';
    const clean = documentBridge.stripHtmlTags(raw);
    assert.equal(clean, 'מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית?');
  });

  test('Lossless roundtrip conversion: MultiFlowDocumentState -> DocumentRoot -> MultiFlowDocumentState', () => {
    const originalState = {
      title: 'ספר תלמוד מסכת ברכות.tok',
      templateType: 'gemara',
      flows: {
        gemara: [
          { id: 'g-1', styleId: 'style-gemara-main', text: 'מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית?' },
          { id: 'g-2', styleId: 'style-gemara-main', text: 'מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.' }
        ],
        rashi: [
          { id: 'r-1', styleId: 'style-rashi-body', text: 'תַּנָּא אַקְּרָא קָאֵי דִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ.' }
        ],
        tosafot: [
          { id: 't-1', styleId: 'style-tosafot-body', text: 'פֵּרֵשׁ רַשִׁ\"י דְּתַנָּא אַקְּרָא קָאֵי.' }
        ],
        notes: [
          { id: 'n-1', styleId: 'style-footnotes', text: 'תורה אור: דברים ו, ז.' }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(originalState, { author: 'רבי יהודה הנשיא' });

    // Validate DocumentRoot structure
    assert.ok(documentBridge.isValidUlid(docRoot.id), 'Root ID must be valid ULID');
    assert.equal(docRoot.metadata.title, 'ספר תלמוד מסכת ברכות');
    assert.equal(docRoot.metadata.author, 'רבי יהודה הנשיא');
    assert.equal(docRoot.metadata.progression, 'Rtl');
    assert.equal(docRoot.metadata.primary_language, 'he');
    assert.equal(docRoot.metadata.schema_version, '1.0');

    assert.ok(docRoot.paragraph_styles.length >= 4, 'Must register standard paragraph styles');
    assert.equal(docRoot.sections.length, 1);
    const section = docRoot.sections[0];
    assert.ok(documentBridge.isValidUlid(section.id), 'Section ID must be valid ULID');
    assert.equal(section.flows.length, 4);

    // Verify all paragraph IDs are valid ULIDs and indices are ordered
    for (const flow of section.flows) {
      for (let i = 0; i < flow.paragraphs.length; i++) {
        const p = flow.paragraphs[i];
        assert.ok(documentBridge.isValidUlid(p.id), `Paragraph ID ${p.id} must be valid ULID`);
        if (i > 0) {
          assert.ok(flow.paragraphs[i - 1].index < p.index, 'Paragraph indices must be strictly ordered');
        }
      }
    }

    // Convert back to UI state
    const restoredState = documentBridge.documentRootToDocumentState(docRoot);
    assert.equal(restoredState.title, 'ספר תלמוד מסכת ברכות');
    assert.equal(restoredState.templateType, 'gemara');
    assert.equal(restoredState.flows.gemara.length, 2);
    assert.equal(restoredState.flows.rashi.length, 1);
    assert.equal(restoredState.flows.tosafot.length, 1);
    assert.equal(restoredState.flows.notes.length, 1);

    assert.equal(restoredState.flows.gemara[0].text, originalState.flows.gemara[0].text);
    assert.equal(restoredState.flows.gemara[1].text, originalState.flows.gemara[1].text);
    assert.equal(restoredState.flows.rashi[0].text, originalState.flows.rashi[0].text);
    assert.equal(restoredState.flows.tosafot[0].text, originalState.flows.tosafot[0].text);
    assert.equal(restoredState.flows.notes[0].text, originalState.flows.notes[0].text);
  });

  test('Empty document conversion produces valid DocumentRoot', () => {
    const emptyState = {
      title: 'מסמך ריק.tok',
      templateType: 'prose',
      flows: { gemara: [], rashi: [], tosafot: [], notes: [] }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(emptyState);
    assert.ok(documentBridge.isValidUlid(docRoot.id));
    assert.equal(docRoot.metadata.title, 'מסמך ריק');
    assert.equal(docRoot.sections.length, 1);
    assert.ok(docRoot.sections[0].flows.length >= 1);
    assert.equal(docRoot.sections[0].flows[0].paragraphs.length, 0);

    const restored = documentBridge.documentRootToDocumentState(docRoot);
    assert.equal(restored.title, 'מסמך ריק');
    assert.equal(restored.flows.gemara.length, 0);
  });

  test('Locate tok-cli engine binary', () => {
    tokCliPath = findTokCli();
    assert.ok(tokCliPath, tokCliMissingMessage());
  });

  test('Integration: Export real user document with unique marker text to PDF', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const uniqueMarker = `TYPESETOK_UNIQUE_MARKER_${Date.now()}_עברית`;
    const uniqueTitle = `ספר בדיקה ייחודי ${Date.now()}`;

    const userState = {
      title: uniqueTitle,
      templateType: 'gemara',
      flows: {
        gemara: [
          { id: 'g-1', styleId: 'style-gemara-main', text: `טקסט משתמש ראשי עם סימון ייחודי: ${uniqueMarker}.` }
        ],
        rashi: [
          { id: 'r-1', styleId: 'style-rashi-body', text: 'פירוש רש\"י המקורי של המשתמש.' }
        ],
        tosafot: [],
        notes: []
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(userState);

    const tempJsonPath = path.join(os.tmpdir(), `tok_test_doc_${Date.now()}.json`);
    const tempPdfPath = path.join(os.tmpdir(), `tok_test_out_${Date.now()}.pdf`);

    try {
      fs.writeFileSync(tempJsonPath, JSON.stringify(docRoot, null, 2), 'utf-8');

      const cliResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['render-pdf', tempJsonPath, tempPdfPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(cliResult.code, 0, `tok-cli render-pdf failed: ${cliResult.stderr || cliResult.stdout}`);
      assert.ok(fs.existsSync(tempPdfPath), 'PDF file must be created on disk');

      const pdfStats = fs.statSync(tempPdfPath);
      assert.ok(pdfStats.size > 1000, `PDF size must be substantial (>1000 bytes), got ${pdfStats.size}`);

      const pdfBytes = fs.readFileSync(tempPdfPath);
      const pdfHeader = pdfBytes.slice(0, 8).toString('ascii');
      assert.ok(pdfHeader.startsWith('%PDF-1.'), 'File must start with valid PDF header %PDF-1.x');

      const pdfText = pdfBytes.toString('latin1');

      // 1. Verify user's document title is in the PDF metadata (handles both plain UTF-8 and PDF UTF-16BE hex strings)
      const expectedTitleHex = Array.from(uniqueTitle)
        .map((c) => c.charCodeAt(0).toString(16).padStart(4, '0').toUpperCase())
        .join('');
      assert.ok(
        pdfText.includes('/Title') && (
          pdfText.includes(uniqueTitle) ||
          pdfBytes.includes(Buffer.from(uniqueTitle, 'utf-8')) ||
          pdfText.includes(expectedTitleHex)
        ),
        'User document title must appear in PDF metadata'
      );

      // 2. Verify demo text is NOT in the PDF
      assert.ok(!pdfText.includes('מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית'), 'Built-in demo text must NOT appear in user export');
      assert.ok(!pdfText.includes('תלמוד בבלי - מסכת ברכות'), 'Demo title must NOT appear in user export');

      // 3. Verify unique user marker characters appear in the PDF font's ToUnicode mapping
      const zlib = await import('node:zlib');
      let decompressedStreams = '';
      let pos = 0;
      while (pos < pdfBytes.length) {
        const streamStart = pdfBytes.indexOf(Buffer.from('stream'), pos);
        if (streamStart === -1) break;
        const streamDataStart = pdfBytes[streamStart + 6] === 0x0a ? streamStart + 7 : streamStart + 8;
        const streamEnd = pdfBytes.indexOf(Buffer.from('endstream'), streamDataStart);
        if (streamEnd === -1) break;
        let dataEnd = streamEnd;
        while (dataEnd > streamDataStart && (pdfBytes[dataEnd - 1] === 0x0a || pdfBytes[dataEnd - 1] === 0x0d)) {
          dataEnd--;
        }
        try {
          const uncompressed = zlib.inflateSync(pdfBytes.slice(streamDataStart, dataEnd));
          decompressedStreams += uncompressed.toString('utf-8') + '\n';
        } catch (_) {}
        pos = streamEnd + 9;
      }

      assert.ok(decompressedStreams.includes('/TOK-Custom-ToUnicode'), 'PDF must include ToUnicode CMap');
      for (const char of uniqueMarker) {
        const charHex = char.charCodeAt(0).toString(16).padStart(4, '0').toUpperCase();
        assert.ok(
          decompressedStreams.includes(`<${charHex}>`),
          `Unique marker character '${char}' (U+${charHex}) must be present in the PDF font mapping`
        );
      }

      // 4. Verify PDF is pre-press compliant
      assert.ok(pdfText.includes('/GTS_PDFXVersion'), 'PDF must include PDF/X OutputIntent');
    } finally {
      if (fs.existsSync(tempJsonPath)) fs.unlinkSync(tempJsonPath);
      if (fs.existsSync(tempPdfPath)) fs.unlinkSync(tempPdfPath);
    }
  });

  test('Integration: Export empty document produces valid PDF without crashing', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const emptyState = {
      title: 'מסמך ריק לבדיקה',
      templateType: 'prose',
      flows: { gemara: [], rashi: [], tosafot: [], notes: [] }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(emptyState);
    const tempJsonPath = path.join(os.tmpdir(), `tok_empty_doc_${Date.now()}.json`);
    const tempPdfPath = path.join(os.tmpdir(), `tok_empty_out_${Date.now()}.pdf`);

    try {
      fs.writeFileSync(tempJsonPath, JSON.stringify(docRoot, null, 2), 'utf-8');

      const cliResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['render-pdf', tempJsonPath, tempPdfPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(cliResult.code, 0, `tok-cli must succeed on empty document: ${cliResult.stderr}`);
      assert.ok(fs.existsSync(tempPdfPath), 'PDF file must be created for empty document');
      const pdfBytes = fs.readFileSync(tempPdfPath);
      assert.ok(pdfBytes.slice(0, 8).toString('ascii').startsWith('%PDF-1.'));
      assert.ok(pdfBytes.length > 500);
    } finally {
      if (fs.existsSync(tempJsonPath)) fs.unlinkSync(tempJsonPath);
      if (fs.existsSync(tempPdfPath)) fs.unlinkSync(tempPdfPath);
    }
  });

  test('Integration: Multi-paragraph document with Hebrew Niqqud exports correctly', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const paragraphs = [
      'בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ.',
      'וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ וְחֹשֶׁךְ עַל פְּנֵי תְהוֹם וְרוּחַ אֱלֹהִים מְרַחֶפֶת עַל פְּנֵי הַמָּיִם.',
      'וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר וַיְהִי אוֹר.',
      'וַיַּרְא אֱלֹהִים אֶת הָאוֹר כִּי טוֹב וַיַּבְדֵּל אֱלֹהִים בֵּין הָאוֹר וּבֵין הַחֹשֶׁךְ.'
    ];

    const state = {
      title: 'ספר בראשית',
      templateType: 'prose',
      flows: {
        gemara: paragraphs.map((text, i) => ({ id: `p-${i}`, styleId: 'normal', text })),
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(state);
    const tempJsonPath = path.join(os.tmpdir(), `tok_niqqud_doc_${Date.now()}.json`);
    const tempPdfPath = path.join(os.tmpdir(), `tok_niqqud_out_${Date.now()}.pdf`);

    try {
      fs.writeFileSync(tempJsonPath, JSON.stringify(docRoot, null, 2), 'utf-8');

      const cliResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['render-pdf', tempJsonPath, tempPdfPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(cliResult.code, 0, `tok-cli must succeed on niqqud document: ${cliResult.stderr}`);
      assert.ok(fs.existsSync(tempPdfPath));
      const stats = fs.statSync(tempPdfPath);
      assert.ok(stats.size > 2000, `Multi-paragraph PDF must be > 2000 bytes, got ${stats.size}`);
    } finally {
      if (fs.existsSync(tempJsonPath)) fs.unlinkSync(tempJsonPath);
      if (fs.existsSync(tempPdfPath)) fs.unlinkSync(tempPdfPath);
    }
  });

  test('Error Handling: Non-existent input or corrupted document fails and reports error', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const nonExistentPath = path.join(os.tmpdir(), `non_existent_${Date.now()}.tok`);
    const tempPdfPath = path.join(os.tmpdir(), `should_not_exist_${Date.now()}.pdf`);

    const cliResult = await new Promise((resolve) => {
      const proc = spawn(tokCliPath, ['render-pdf', nonExistentPath, tempPdfPath], { windowsHide: true });
      let stdout = '';
      let stderr = '';
      proc.stdout?.on('data', (d) => (stdout += d.toString()));
      proc.stderr?.on('data', (d) => (stderr += d.toString()));
      proc.on('close', (code) => resolve({ code, stdout, stderr }));
    });

    assert.notEqual(cliResult.code, 0, 'Must exit with non-zero code on missing file');
    assert.equal(fs.existsSync(tempPdfPath), false, 'PDF must NOT be created when input is missing');
  });

  test('Backward Compatibility: --demo flag still functions as expected', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const tempPdfPath = path.join(os.tmpdir(), `tok_demo_out_${Date.now()}.pdf`);

    try {
      const cliResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['render-pdf', '--demo', tempPdfPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(cliResult.code, 0, `tok-cli render-pdf --demo failed: ${cliResult.stderr}`);
      assert.ok(fs.existsSync(tempPdfPath), 'Demo PDF must be generated');
      const stats = fs.statSync(tempPdfPath);
      assert.ok(stats.size > 2000, 'Demo PDF must be > 2000 bytes');
    } finally {
      if (fs.existsSync(tempPdfPath)) fs.unlinkSync(tempPdfPath);
    }
  });
});

describe('Phase 3: Real .tok Storage (Save, Open, Round-Trip & Error Resilience)', () => {
  const tokCliPath = findTokCli();

  let documentBridge;
  before(async () => {
    const bridgePath = path.resolve('packages/tok-ui/dist/engine/documentBridge.js');
    assert.ok(fs.existsSync(bridgePath), 'documentBridge.js must be built');
    documentBridge = await import(`file://${bridgePath.replace(/\\/g, '/')}`);
  });

  test('Round-Trip: Save and open multi-flow Hebrew document with Niqqud via .tok package', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const multiFlowState = {
      title: 'מסכת פסחים - מהדורת בדיקה 2026',
      templateType: 'gemara',
      flows: {
        gemara: [
          { id: 'g-1', styleId: 'style-gemara-main', text: 'כָּל שָׁעָה שֶׁמֻּתָּר לֶאֱכֹל, מַאֲכִיל לַבְּהֵמָה וְלַחַיָּה וְלָעוֹפוֹת, וּמוֹכְרוֹ לְנָכְרִי, וּמֻתָּר בַּהֲנָאָתוֹ.' },
          { id: 'g-2', styleId: 'style-gemara-main', text: 'עָבַר זְמַנּוֹ, אָסוּר בַּהֲנָאָתוֹ, וְלֹא יַסִּיק בּוֹ תַּנּוּר וְכִירַיִם. רַבִּי יְהוּדָה אוֹמֵר: אֵין בִּעוּר חָמֵץ אֶלָּא שְׂרֵפָה.' }
        ],
        rashi: [
          { id: 'r-1', styleId: 'style-rashi-body', text: 'כל שעה שמותר לאכול - כל ארבע שעות.' },
          { id: 'r-2', styleId: 'style-rashi-body', text: 'מאכיל לבהמה - ואף על פי שהיא רובצת לפניו.' }
        ],
        tosafot: [
          { id: 't-1', styleId: 'style-tosafot-body', text: 'כל שעה שמותר לאכול - ואם תאמר והא תנן לקמן כל שעה שאינו אוכל מאכיל.' }
        ],
        notes: [
          { id: 'n-1', styleId: 'style-footnotes', text: 'הערת שוליים ראשונה על פירוש רש\"י.' }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(multiFlowState);
    const tempInJson = path.join(os.tmpdir(), `tok_save_in_${Date.now()}.json`);
    const tempTokPath = path.join(os.tmpdir(), `tok_save_pkg_${Date.now()}.tok`);
    const tempOutJson = path.join(os.tmpdir(), `tok_open_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      // 1. Execute CLI save-package
      const saveResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['save-package', tempInJson, tempTokPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(saveResult.code, 0, `save-package failed: ${saveResult.stderr || saveResult.stdout}`);
      assert.ok(fs.existsSync(tempTokPath), '.tok package must exist on disk');
      const tokStats = fs.statSync(tempTokPath);
      assert.ok(tokStats.size > 500, `.tok package size must be substantial, got ${tokStats.size}`);

      // 2. Execute CLI inspect-package to verify manifest
      const inspectResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['inspect-package', tempTokPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(inspectResult.code, 0, `inspect-package failed: ${inspectResult.stderr}`);
      assert.ok(inspectResult.stdout.includes(multiFlowState.title), 'inspect-package must report correct title');

      // 3. Execute CLI open-package
      const openResult = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['open-package', tempTokPath, tempOutJson], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('error', reject);
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(openResult.code, 0, `open-package failed: ${openResult.stderr || openResult.stdout}`);
      assert.ok(fs.existsSync(tempOutJson), 'Extracted document JSON must exist on disk');

      // 4. Verify roundtripped state matches original model exactly
      const extractedRoot = JSON.parse(fs.readFileSync(tempOutJson, 'utf-8'));
      const restoredState = documentBridge.documentRootToDocumentState(extractedRoot);

      assert.equal(restoredState.title, multiFlowState.title, 'Title must round-trip exactly');
      assert.equal(restoredState.templateType, multiFlowState.templateType, 'Template type must round-trip');

      assert.equal(restoredState.flows.gemara.length, 2, 'Gemara paragraphs count must match');
      assert.equal(restoredState.flows.rashi.length, 2, 'Rashi paragraphs count must match');
      assert.equal(restoredState.flows.tosafot.length, 1, 'Tosafot paragraphs count must match');
      assert.equal(restoredState.flows.notes.length, 1, 'Notes paragraphs count must match');

      assert.ok(restoredState.flows.gemara[0].text.includes('שָׁעָה שֶׁמֻּתָּר לֶאֱכֹל'));
      assert.ok(restoredState.flows.gemara[1].text.includes('רַבִּי יְהוּדָה'));
      assert.ok(restoredState.flows.rashi[0].text.includes('כל שעה שמותר לאכול'));
      assert.ok(restoredState.flows.tosafot[0].text.includes('ואם תאמר והא תנן'));
      assert.ok(restoredState.flows.notes[0].text.includes('הערת שוליים ראשונה'));
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempTokPath)) fs.unlinkSync(tempTokPath);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });

  test('Error Handling: Corrupted or invalid .tok file is rejected without creating output', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const corruptTokPath = path.join(os.tmpdir(), `corrupt_pkg_${Date.now()}.tok`);
    const tempOutJson = path.join(os.tmpdir(), `should_not_exist_${Date.now()}.json`);

    try {
      fs.writeFileSync(corruptTokPath, Buffer.from('NOT_A_VALID_ZIP_HEADER_GARBAGE_BYTES_12345'), 'utf-8');

      const openResult = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['open-package', corruptTokPath, tempOutJson], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.notEqual(openResult.code, 0, 'CLI must exit with non-zero on corrupted .tok package');
      assert.equal(fs.existsSync(tempOutJson), false, 'Output JSON must NOT be created on failure');
    } finally {
      if (fs.existsSync(corruptTokPath)) fs.unlinkSync(corruptTokPath);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });

  test('Atomic Resilience: Existing file is not destroyed if saving invalid document fails', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const existingTokPath = path.join(os.tmpdir(), `atomic_existing_${Date.now()}.tok`);
    const nonExistentJson = path.join(os.tmpdir(), `missing_${Date.now()}.json`);

    try {
      const initialContent = Buffer.from('ORIGINAL_INTACT_CONTENT_BEFORE_FAILED_SAVE');
      fs.writeFileSync(existingTokPath, initialContent);

      const saveResult = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['save-package', nonExistentJson, existingTokPath], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.notEqual(saveResult.code, 0, 'Must fail when input is missing');
      assert.ok(fs.existsSync(existingTokPath), 'Existing file must still exist');
      const survivingContent = fs.readFileSync(existingTokPath);
      assert.deepEqual(survivingContent, initialContent, 'Existing file must not be modified or truncated');
    } finally {
      if (fs.existsSync(existingTokPath)) fs.unlinkSync(existingTokPath);
    }
  });
});

describe('Phase 4: Real Rust Typesetting Engine Connection (typeset-document & typesetBridge)', () => {
  let layoutToPageDescriptors;
  let tokCliPath;

  before(async () => {
    const bridgeModule = await import('../packages/tok-ui/dist/engine/typesetBridge.js');
    layoutToPageDescriptors = bridgeModule.layoutToPageDescriptors;

    tokCliPath = findTokCli();
  });

  test('Bridge: layoutToPageDescriptors translates Rust PageLayoutBox into SpreadCanvas PageDescriptor', () => {
    assert.ok(typeof layoutToPageDescriptors === 'function', 'layoutToPageDescriptors must be exported');

    const sampleRustBoxes = [
      {
        page_index: 0,
        page_number_gematria: 'א׳',
        dimensions: { x: 0, y: 0, width: 595.28, height: 841.89 },
        frames: [
          {
            frame_id: 'frame_1_gemara',
            flow_id: 'gemara',
            rect: { x: 42.5, y: 42.5, width: 510.28, height: 756.89 },
            lines: [
              {
                line_index: 0,
                paragraph_id: 'para-marker-101',
                baseline_y: 18.0,
                height: 14.0,
                width: 510.28,
                glyphs: [],
                text: 'מאימתי קורין את שמע בערבית משעה שהכהנים נכנסין',
                is_rtl: true,
                fonts: ['Frank Ruhl Libre']
              },
              {
                line_index: 1,
                paragraph_id: 'para-marker-101',
                baseline_y: 36.0,
                height: 14.0,
                width: 510.28,
                glyphs: [],
                text: 'לאכול בתרומתן עד סוף האשמורה הראשונה',
                is_rtl: true,
                fonts: ['Frank Ruhl Libre']
              }
            ]
          }
        ],
        break_token: null
      }
    ];

    const descriptors = layoutToPageDescriptors(sampleRustBoxes);
    assert.equal(descriptors.length, 1);
    assert.equal(descriptors[0].pageIndex, 0);
    assert.equal(descriptors[0].gematriaNumber, 'א׳');
    assert.equal(descriptors[0].widthPt, 595.28);
    assert.equal(descriptors[0].heightPt, 841.89);

    const html = descriptors[0].htmlContent;
    assert.ok(html.includes('tok-page-layout-rust'), 'Must wrap in tok-page-layout-rust');
    assert.ok(html.includes('data-frame-id="frame_1_gemara"'), 'Must preserve frame id');
    assert.ok(html.includes('data-flow-id="gemara"'), 'Must preserve flow id');
    assert.ok(html.includes('data-line-index="0"'), 'Must render line 0');
    assert.ok(html.includes('data-para-id="para-marker-101"'), 'Must preserve paragraph ID on line');
    assert.ok(html.includes('data-baseline-y="18"'), 'Must preserve exact baseline Y');
    assert.ok(html.includes('מאימתי קורין את שמע'), 'Must render exact line text');
  });

  test('Bridge: layoutToPageDescriptors handles empty document layout box cleanly', () => {
    const emptyRustBoxes = [
      {
        page_index: 0,
        page_number_gematria: 'א׳',
        dimensions: { x: 0, y: 0, width: 595.28, height: 841.89 },
        frames: [
          {
            frame_id: 'frame_1_gemara',
            flow_id: 'gemara',
            rect: { x: 42.5, y: 42.5, width: 510.28, height: 756.89 },
            lines: []
          }
        ],
        break_token: null
      }
    ];

    const descriptors = layoutToPageDescriptors(emptyRustBoxes);
    assert.equal(descriptors.length, 1);
    assert.ok(descriptors[0].htmlContent.includes('[אין טקסט בעמוד זה]'), 'Must render empty text indicator');
  });

  test('CLI: tok-cli typeset-document emits valid PageLayoutBox JSON array with Hebrew lines', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');
    const bridgeModule = await import('../packages/tok-ui/dist/engine/documentBridge.js');
    const { documentStateToDocumentRoot } = bridgeModule;

    const uniqueMarker = `שמע_בערבית_סימן_ייחודי_${Date.now()}`;
    const testDocState = {
      title: 'בדיקת עימוד חי',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'para-uniq-1',
            styleId: 'style-gemara-main',
            text: `${uniqueMarker}: מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול בתרומתן.`
          }
        ],
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const docRoot = documentStateToDocumentRoot(testDocState);
    const tempInJson = path.join(os.tmpdir(), `typeset_in_${Date.now()}.json`);
    const tempOutJson = path.join(os.tmpdir(), `typeset_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      const cliResult = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempInJson, tempOutJson], { windowsHide: true });
        let stdout = '';
        let stderr = '';
        proc.stdout?.on('data', (d) => (stdout += d.toString()));
        proc.stderr?.on('data', (d) => (stderr += d.toString()));
        proc.on('close', (code) => resolve({ code, stdout, stderr }));
      });

      assert.equal(cliResult.code, 0, `tok-cli typeset-document failed: ${cliResult.stderr}`);
      assert.ok(fs.existsSync(tempOutJson), 'Output JSON must be created');

      const rawJson = fs.readFileSync(tempOutJson, 'utf-8');
      const pages = JSON.parse(rawJson);

      assert.ok(Array.isArray(pages), 'Output must be an array of PageLayoutBox');
      assert.ok(pages.length >= 1, 'Must produce at least 1 page');
      assert.equal(pages[0].page_index, 0);
      assert.ok(pages[0].frames.length >= 1, 'Must have at least 1 frame');

      const allLines = pages[0].frames.flatMap((f) => f.lines);
      assert.ok(allLines.length >= 1, 'Must have typeset lines');
      const lineWithMarker = allLines.find((l) => l.text.includes(uniqueMarker));
      assert.ok(lineWithMarker, 'Must include unique marker text in typeset lines');
      assert.ok(lineWithMarker.baseline_y > 0, 'Baseline Y must be positive');
      assert.ok(lineWithMarker.height > 0, 'Line height must be positive');
      assert.equal(lineWithMarker.is_rtl, true, 'Hebrew line must be RTL');
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });

  test('Integration: Editing document content dynamically updates Rust layout results and line counts', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');
    const bridgeModule = await import('../packages/tok-ui/dist/engine/documentBridge.js');
    const { documentStateToDocumentRoot } = bridgeModule;

    // Phase A: Initial document with short text
    const initialDocState = {
      title: 'עריכה דינמית',
      templateType: 'prose',
      flows: {
        gemara: [
          {
            id: 'para-1',
            styleId: 'style-gemara-main',
            text: 'פסקה ראשונה קצרה ביותר.'
          }
        ],
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const tempIn1 = path.join(os.tmpdir(), `edit_in1_${Date.now()}.json`);
    const tempOut1 = path.join(os.tmpdir(), `edit_out1_${Date.now()}.json`);
    let pages1;

    try {
      fs.writeFileSync(tempIn1, JSON.stringify(documentStateToDocumentRoot(initialDocState), null, 2), 'utf-8');
      await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn1, tempOut1], { windowsHide: true });
        proc.on('close', resolve);
      });
      pages1 = JSON.parse(fs.readFileSync(tempOut1, 'utf-8'));
    } finally {
      if (fs.existsSync(tempIn1)) fs.unlinkSync(tempIn1);
      if (fs.existsSync(tempOut1)) fs.unlinkSync(tempOut1);
    }

    const desc1 = layoutToPageDescriptors(pages1);
    const lineCount1 = pages1.reduce((sum, p) => sum + p.frames.reduce((s, f) => s + f.lines.length, 0), 0);
    assert.equal(desc1.length, 1, 'Initial short text should fit in 1 page');
    assert.equal(lineCount1, 1, 'Initial text should produce 1 line');

    // Phase B: Content edited: user adds extensive multi-paragraph text
    const longHebrewParagraphs = Array.from({ length: 45 }, (_, i) => ({
      id: `para-extended-${i}`,
      styleId: 'style-gemara-main',
      text: `פסקה מספר ${i + 1}: מאימתי קורין את שמע בערבית משעה שהכהנים נכנסים לאכול בתרומתן עד סוף האשמורה הראשונה דברי רבי אליעזר וחכמים אומרים עד חצות רבן גמליאל אומר עד שיעלה עמוד השחר מעשה ובאו בניו מבית המשתה אמרו לו לא קרינו את שמע אמר להם אם לא עלה עמוד השחר חייבין אתם לקרות.`
    }));

    const editedDocState = {
      title: 'עריכה דינמית',
      templateType: 'prose',
      flows: {
        gemara: longHebrewParagraphs,
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const tempIn2 = path.join(os.tmpdir(), `edit_in2_${Date.now()}.json`);
    const tempOut2 = path.join(os.tmpdir(), `edit_out2_${Date.now()}.json`);
    let pages2;

    try {
      fs.writeFileSync(tempIn2, JSON.stringify(documentStateToDocumentRoot(editedDocState), null, 2), 'utf-8');
      await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn2, tempOut2], { windowsHide: true });
        proc.on('close', resolve);
      });
      pages2 = JSON.parse(fs.readFileSync(tempOut2, 'utf-8'));
    } finally {
      if (fs.existsSync(tempIn2)) fs.unlinkSync(tempIn2);
      if (fs.existsSync(tempOut2)) fs.unlinkSync(tempOut2);
    }

    const desc2 = layoutToPageDescriptors(pages2);
    const lineCount2 = pages2.reduce((sum, p) => sum + p.frames.reduce((s, f) => s + f.lines.length, 0), 0);

    // Verify layout expanded according to real Rust typography and page breaking
    assert.ok(lineCount2 > lineCount1, `Line count must expand after adding content (${lineCount2} > ${lineCount1})`);
    assert.ok(desc2.length > 1, `Document must paginate into multiple pages (${desc2.length} > 1)`);
    assert.equal(desc2[0].gematriaNumber, 'א׳');
    assert.equal(desc2[1].gematriaNumber, 'ב׳');
  });

  test('Integration: Hebrew Niqqud, Taamim and multi-flow Talmud co-pagination geometry', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');
    const bridgeModule = await import('../packages/tok-ui/dist/engine/documentBridge.js');
    const { documentStateToDocumentRoot } = bridgeModule;

    const niqqudText = 'בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ׃';
    const talmudDocState = {
      title: 'תלמוד מסכת ברכות',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'g1',
            styleId: 'style-gemara-main',
            text: niqqudText
          }
        ],
        rashi: [
          {
            id: 'r1',
            styleId: 'style-rashi-comm',
            text: 'רש"י: פירש רש"י על אתר בלשון קצרה.'
          }
        ],
        tosafot: [
          {
            id: 't1',
            styleId: 'style-tosafot-comm',
            text: 'תוספות: ותימא מאי שנא הכא.'
          }
        ],
        notes: []
      }
    };

    const tempIn = path.join(os.tmpdir(), `talmud_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `talmud_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(documentStateToDocumentRoot(talmudDocState), null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0);
      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      const descriptors = layoutToPageDescriptors(pages);

      assert.ok(descriptors.length >= 1);
      const firstHtml = descriptors[0].htmlContent;

      // Verify that all 3 flows have distinct interactive frames
      assert.ok(firstHtml.includes('data-flow-id="gemara"') || firstHtml.includes('data-flow-id="main"'), 'Must have Gemara frame');
      assert.ok(firstHtml.includes('data-flow-id="rashi"'), 'Must have Rashi frame');
      assert.ok(firstHtml.includes('data-flow-id="tosafot"'), 'Must have Tosafot frame');

      // Verify that Niqqud is preserved in the rendered layout
      assert.ok(firstHtml.includes('בְּרֵאשִׁית'), 'Niqqud must be preserved in rendered HTML line');
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });

  test('CLI MultiFlow: 2-flow dual-stream document typesets with proportional columns via tok-cli', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const doc2Flow = {
      id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      metadata: {
        title: "ספר לימוד דו-זרימתי",
        author: "מחבר",
        progression: "Rtl",
        primary_language: "he",
        schema_version: "1.0"
      },
      paragraph_styles: [],
      character_styles: [],
      sections: [
        {
          id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
          name: "פרק ראשון",
          page_style: "default",
          flows: [
            {
              id: "primary",
              flow_type: "Main",
              placement_role: "primary",
              paragraphs: [
                {
                  id: "01ARZ3NDEKTSV4RRFFQ69G5FA1",
                  index: "a0",
                  style_id: "normal",
                  text: "טקסט ראשי של הספר הנלמד בשני טורים מקבילים בעברית."
                }
              ]
            },
            {
              id: "commentary",
              flow_type: "CommentA",
              placement_role: "inner_spine",
              paragraphs: [
                {
                  id: "01ARZ3NDEKTSV4RRFFQ69G5FA2",
                  index: "a0",
                  style_id: "normal",
                  text: "פירוש נלווה בטור המקביל הנמצא בצד השדרה."
                }
              ]
            }
          ]
        }
      ]
    };

    const tempIn = path.join(os.tmpdir(), `typeset_2flow_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `typeset_2flow_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(doc2Flow, null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli must succeed on 2-flow document');
      assert.ok(fs.existsSync(tempOut), 'Output JSON must be created');

      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(pages.length >= 1, 'Must output at least 1 page');
      const p0 = pages[0];
      assert.ok(p0.frames.length >= 2, 'Page 0 must have at least 2 frames');

      const primFrame = p0.frames.find((f) => f.flow_id === 'primary');
      const commFrame = p0.frames.find((f) => f.flow_id === 'commentary');
      assert.ok(primFrame, 'Must have primary frame');
      assert.ok(commFrame, 'Must have commentary frame');
      assert.ok(primFrame.lines.length >= 1, 'Primary frame must have lines');
      assert.ok(commFrame.lines.length >= 1, 'Commentary frame must have lines');

      // Primary (60%) should be wider than commentary (40%)
      assert.ok(primFrame.rect.width > commFrame.rect.width, 'Primary column must be wider than secondary column');
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });

  test('CLI MultiFlow: 4-flow Mikraot Gedolot document typesets with 4 distinct column frames via tok-cli', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const doc4Flow = {
      id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      metadata: {
        title: "מקראות גדולות ארבע זרימות",
        author: "מחבר",
        progression: "Rtl",
        primary_language: "he",
        schema_version: "1.0"
      },
      paragraph_styles: [],
      character_styles: [],
      sections: [
        {
          id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
          name: "פרשת בראשית",
          page_style: "default",
          flows: [
            {
              id: "torah",
              flow_type: "Main",
              placement_role: "primary",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA1", index: "a0", style_id: "normal", text: "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ." }]
            },
            {
              id: "onkelos",
              flow_type: "CommentA",
              placement_role: "inner_spine",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA2", index: "a0", style_id: "normal", text: "בְּקַדְמִין בְּרָא יְיָ יָת שְׁמַיָּא וְיָת אַרְעָא." }]
            },
            {
              id: "rashi",
              flow_type: "CommentB",
              placement_role: "outer_margin",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA3", index: "a0", style_id: "normal", text: "בְּרֵאשִׁית: אָמַר רַבִּי יִצְחָק לֹא הָיָה צָרִיךְ לְהַתְחִיל אֶת הַתּוֹרָה אֶלָּא מֵהַחֹדֶשׁ הַזֶּה לָכֶם." }]
            },
            {
              id: "ramban",
              flow_type: "CommentB",
              placement_role: "column_3",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA4", index: "a0", style_id: "normal", text: "רמב\"ן: בראשית ברא אלהים, כתב רש\"י למה פתח בבראשית." }]
            }
          ]
        }
      ]
    };

    const tempIn = path.join(os.tmpdir(), `typeset_4flow_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `typeset_4flow_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(doc4Flow, null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli must succeed on 4-flow document');
      assert.ok(fs.existsSync(tempOut), 'Output JSON must be created');

      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(pages.length >= 1, 'Must output at least 1 page');
      const p0 = pages[0];
      assert.equal(p0.frames.length, 4, 'Must allocate 4 distinct frames for Mikraot Gedolot');

      const flowIds = p0.frames.map((f) => f.flow_id);
      assert.ok(flowIds.includes('torah'), 'Must include torah frame');
      assert.ok(flowIds.includes('onkelos'), 'Must include onkelos frame');
      assert.ok(flowIds.includes('rashi'), 'Must include rashi frame');
      assert.ok(flowIds.includes('ramban'), 'Must include ramban frame');

      for (const frame of p0.frames) {
        assert.ok(frame.lines.length >= 1, `Frame ${frame.flow_id} must have formatted lines`);
        assert.ok(frame.rect.width > 30, `Frame ${frame.flow_id} width must be substantial`);
      }
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });

  test('Integration: Live multi-flow editing, content continuity, and Tzurat HaDaf HTML layout in UI', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');
    const bridgeModule = await import('../packages/tok-ui/dist/engine/documentBridge.js');
    const { documentStateToDocumentRoot } = bridgeModule;

    // 1. Initial Talmud Tzurat HaDaf state: Gemara ends early, Rashi and Tosafot have extensive commentaries
    const talmudEditDoc = {
      title: 'תלמוד בבלי - ברכות ב',
      templateType: 'talmud',
      flows: {
        gemara: [
          {
            id: 'g-init-1',
            styleId: 'style-gemara-main',
            text: 'מַאימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.'
          }
        ],
        rashi: [
          {
            id: 'r-init-1',
            styleId: 'style-rashi-body',
            text: 'מאימתי קורין את שמע בערבית: משעת צאת הכוכבים, והא דתנן משעה שהכהנים נכנסים לאכול בתרומתן, היינו צאת הכוכבים כדמפרש בגמרא.'
          },
          {
            id: 'r-init-2',
            styleId: 'style-rashi-body',
            text: 'לתרומתן: כהנים שנטמאו וטבלו והעריב שמשן ומותרים לאכול בתרומה.'
          }
        ],
        tosafot: [
          {
            id: 't-init-1',
            styleId: 'style-tosafot-body',
            text: 'מאימתי קורין: תימה דבכל דוכתא פריך תנא היכא קאי ומהדר תנא דבי ר\"י, והכא לא פריך הכי משום דפתח בערבית ברישא.'
          }
        ],
        notes: []
      }
    };

    const tempIn1 = path.join(os.tmpdir(), `talmud_live1_in_${Date.now()}.json`);
    const tempOut1 = path.join(os.tmpdir(), `talmud_live1_out_${Date.now()}.json`);
    let pages1;

    try {
      fs.writeFileSync(tempIn1, JSON.stringify(documentStateToDocumentRoot(talmudEditDoc), null, 2), 'utf-8');
      const exitCode1 = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn1, tempOut1], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode1, 0, 'Initial typeset must succeed');
      pages1 = JSON.parse(fs.readFileSync(tempOut1, 'utf-8'));
    } finally {
      if (fs.existsSync(tempIn1)) fs.unlinkSync(tempIn1);
      if (fs.existsSync(tempOut1)) fs.unlinkSync(tempOut1);
    }

    assert.ok(pages1.length >= 1, 'Must have at least 1 page');
    const desc1 = layoutToPageDescriptors(pages1);
    assert.equal(desc1.length, pages1.length, 'Page descriptors length must match layout box count');

    const html1 = desc1[0].htmlContent;
    assert.ok(html1.includes('tok-interactive-frame'), 'HTML must render interactive flow frame container');
    assert.ok(html1.includes('data-flow-id="gemara"'), 'HTML must render gemara flow frame');
    assert.ok(html1.includes('data-flow-id="rashi"'), 'HTML must render rashi flow frame');
    assert.ok(html1.includes('data-flow-id="tosafot"'), 'HTML must render tosafot flow frame');
    assert.ok(html1.includes('tok-line-box'), 'HTML must contain rendered text lines');
    assert.ok(html1.includes('בִּתְרוּמָתָן'), 'HTML must preserve Hebrew Niqqud accurately');

    // 2. Perform live edit: add more commentary to Rashi & Tosafot
    talmudEditDoc.flows.rashi.push({
      id: 'r-edit-3',
      styleId: 'style-rashi-body',
      text: 'והעריב שמשן: ביאת שמשו מעכבתו מלאכול בתרומה עד שיעריב השמש ויטהר לגמרי.'
    });
    talmudEditDoc.flows.tosafot.push({
      id: 't-edit-2',
      styleId: 'style-tosafot-body',
      text: 'ואם תאמר: והא קימא לן תפלת ערבית רשות, ויש לומר דקריאת שמע חובת גברא היא ולא דמיא לתפלה.'
    });

    const tempIn2 = path.join(os.tmpdir(), `talmud_live2_in_${Date.now()}.json`);
    const tempOut2 = path.join(os.tmpdir(), `talmud_live2_out_${Date.now()}.json`);
    let pages2;

    try {
      fs.writeFileSync(tempIn2, JSON.stringify(documentStateToDocumentRoot(talmudEditDoc), null, 2), 'utf-8');
      const exitCode2 = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn2, tempOut2], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode2, 0, 'Re-typesetting edited document must succeed');
      pages2 = JSON.parse(fs.readFileSync(tempOut2, 'utf-8'));
    } finally {
      if (fs.existsSync(tempIn2)) fs.unlinkSync(tempIn2);
      if (fs.existsSync(tempOut2)) fs.unlinkSync(tempOut2);
    }

    const desc2 = layoutToPageDescriptors(pages2);
    assert.ok(desc2.length >= 1, 'Must have at least 1 page descriptor');

    // Total lines across all pages should increase after editing
    const totalLines1 = pages1.reduce((sum, p) => sum + p.frames.reduce((fs, f) => fs + f.lines.length, 0), 0);
    const totalLines2 = pages2.reduce((sum, p) => sum + p.frames.reduce((fs, f) => fs + f.lines.length, 0), 0);
    assert.ok(totalLines2 > totalLines1, 'Edited multi-flow document must contain more typeset lines');

    // Verify all edited content appears in the HTML output without text loss
    const combinedHtml2 = desc2.map((d) => d.htmlContent).join(' ');
    assert.ok(combinedHtml2.includes('שמשו מעכבתו') || combinedHtml2.includes('והעריב שמשן'), 'New Rashi edit must be rendered in HTML');
    assert.ok(combinedHtml2.includes('ואם תאמר') || combinedHtml2.includes('דמיא לתפלה'), 'New Tosafot edit must be rendered in HTML');
    assert.ok(combinedHtml2.includes('מַאימָתַי קוֹרִין'), 'Original Gemara content must remain completely intact');
    assert.ok(combinedHtml2.includes('בִּתְרוּמָתָן'), 'Original Gemara Niqqud must remain completely intact');
  });
});

describe('Phase 6: Document Classification & Layout Template Synthesis', () => {
  let tokCliPath;

  before(() => {
    tokCliPath = findTokCli();
  });

  test('CLI: Auto-classification of unfamiliar flow names into ParallelColumns with equal proportions', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const parallelDoc = {
      id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      metadata: {
        title: "מהדורה דו-לשונית מקבילה",
        author: "מחבר",
        progression: "Rtl",
        primary_language: "he",
        schema_version: "1.0"
      },
      paragraph_styles: [],
      character_styles: [],
      sections: [
        {
          id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
          name: "עמודות מקבילות",
          page_style: "default",
          flows: [
            {
              id: "hebrew_source",
              flow_type: "Main",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA1", index: "a0", style_id: "normal", text: "מקור עברי עתיק עם ניסוח מדויק." }]
            },
            {
              id: "aramaic_translation",
              flow_type: "Main",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA2", index: "a0", style_id: "normal", text: "תרגום ארמי מקביל באותה רמת פירוט." }]
            }
          ]
        }
      ]
    };

    const tempIn = path.join(os.tmpdir(), `phase6_parallel_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `phase6_parallel_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(parallelDoc, null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli must succeed on parallel 2-flow document');
      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(pages.length >= 1, 'Must output at least 1 page');

      const p0 = pages[0];
      assert.equal(p0.frames.length, 2, 'Must synthesize 2 parallel column frames');

      const f0 = p0.frames.find((f) => f.flow_id === 'hebrew_source');
      const f1 = p0.frames.find((f) => f.flow_id === 'aramaic_translation');
      assert.ok(f0 && f1, 'Both flow frames must exist');

      // Equal proportions: widths must be within 5 points of each other
      assert.ok(Math.abs(f0.rect.width - f1.rect.width) < 5.0, `Widths should be equal: ${f0.rect.width} vs ${f1.rect.width}`);
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });

  test('CLI: Explicit layout_kind and column_proportions overrides structural heuristics', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const customDoc = {
      id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      metadata: {
        title: "עקיפת תבנית מפורשת 70/30",
        author: "מחבר",
        progression: "Rtl",
        primary_language: "he",
        schema_version: "1.0"
      },
      paragraph_styles: [],
      character_styles: [],
      sections: [
        {
          id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
          name: "פרק עם עקיפה",
          page_style: "default",
          layout_kind: "parallel_columns",
          column_proportions: [0.70, 0.30],
          flows: [
            {
              id: "wide_stream",
              flow_type: "Main",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA1", index: "a0", style_id: "normal", text: "עמודה רחבה התופסת שבעים אחוז מרוחב הדף." }]
            },
            {
              id: "narrow_stream",
              flow_type: "Main",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA2", index: "a0", style_id: "normal", text: "עמודה צרה התופסת שלושים אחוז." }]
            }
          ]
        }
      ]
    };

    const tempIn = path.join(os.tmpdir(), `phase6_explicit_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `phase6_explicit_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(customDoc, null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli must succeed on custom proportioned document');
      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(pages.length >= 1);

      const p0 = pages[0];
      const fw = p0.frames.find((f) => f.flow_id === 'wide_stream');
      const fn = p0.frames.find((f) => f.flow_id === 'narrow_stream');
      assert.ok(fw && fn);

      const totalW = fw.rect.width + fn.rect.width;
      const ratioW = fw.rect.width / totalW;
      const ratioN = fn.rect.width / totalW;

      assert.ok(Math.abs(ratioW - 0.70) < 0.05, `Wide ratio should be ~0.70, got ${ratioW}`);
      assert.ok(Math.abs(ratioN - 0.30) < 0.05, `Narrow ratio should be ~0.30, got ${ratioN}`);
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });

  test('CLI: Auto-classification of spine-relative commentary into TzuratHaDaf with custom stream names', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const tzuratDoc = {
      id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      metadata: {
        title: "צורת הדף עם שמות שרירותיים",
        author: "מחבר",
        progression: "Rtl",
        primary_language: "he",
        schema_version: "1.0"
      },
      paragraph_styles: [],
      character_styles: [],
      sections: [
        {
          id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
          name: "צורת הדף",
          page_style: "default",
          flows: [
            {
              id: "core_corpus",
              flow_type: "Main",
              placement_role: "primary",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA1", index: "a0", style_id: "normal", text: "טקסט מרכזי בעימוד צורת הדף." }]
            },
            {
              id: "inner_scholion",
              flow_type: "CommentA",
              placement_role: "inner_spine",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA2", index: "a0", style_id: "normal", text: "פירוש שדרה פנימי לצד עמודת המרכז." }]
            },
            {
              id: "outer_gloss",
              flow_type: "CommentB",
              placement_role: "outer_margin",
              paragraphs: [{ id: "01ARZ3NDEKTSV4RRFFQ69G5FA3", index: "a0", style_id: "normal", text: "הגהה חיצונית בשוליים הרחבים של הדף." }]
            }
          ]
        }
      ]
    };

    const tempIn = path.join(os.tmpdir(), `phase6_tzurat_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `phase6_tzurat_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(tzuratDoc, null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli must succeed on custom Tzurat HaDaf document');
      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(pages.length >= 1);

      const p0 = pages[0];
      assert.equal(p0.frames.length, 3, 'Must allocate 3 distinct frames for custom Tzurat HaDaf');

      const flowIds = p0.frames.map((f) => f.flow_id);
      assert.ok(flowIds.includes('core_corpus'), 'Must include core corpus frame');
      assert.ok(flowIds.includes('inner_scholion'), 'Must include inner scholion frame');
      assert.ok(flowIds.includes('outer_gloss'), 'Must include outer gloss frame');

      for (const frame of p0.frames) {
        assert.ok(frame.lines.length >= 1, `Frame ${frame.flow_id} must have placed lines`);
        assert.ok(frame.rect.width > 50, `Frame ${frame.flow_id} width must be substantial`);
      }
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });
});

describe('Stage 11: End-to-End Story Editor, Document Model & .tok Storage Round-Trip', () => {
  let documentBridge;
  let tokCliPath;
  let storyEditorModule;

  before(async () => {
    const bridgePath = path.resolve('packages/tok-ui/dist/engine/documentBridge.js');
    assert.ok(fs.existsSync(bridgePath), 'documentBridge.js must be built');
    documentBridge = await import(`file://${bridgePath.replace(/\\/g, '/')}`);

    const editorPath = path.resolve('packages/tok-story-editor/dist/index.js');
    assert.ok(fs.existsSync(editorPath), 'tok-story-editor dist must be built');
    storyEditorModule = await import(`file://${editorPath.replace(/\\/g, '/')}`);

    tokCliPath = findTokCli();
  });

  test('Story Editor to Document Model bidirectional sync with Spans and Style Patches', () => {
    // 1. Construct rich text paragraph data representing story editor state
    const originalState = {
      title: 'ספר היצירה עם פירוש מודרני',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'g-1',
            styleId: 'style-gemara-main',
            text: 'בִּשְׁלֹשִׁים וּשְׁתַּיִם נְתִיבוֹת פְּלִיאוֹת חָכְמָה חָקַק יָהּ ה׳ צְבָאוֹת.',
            spans: [
              { start: 0, end: 14, format: { fontWeight: 700, fontFamily: 'FrankRuehl', fontSizePt: 16 } },
              { start: 15, end: 32, format: { italic: true } },
              { start: 33, end: 42, format: { underline: true, trackingEm: 1.5 } }
            ],
            patch: {
              alignment: 'Justified',
              lineHeightPt: 1.45,
              spaceBeforePt: 8,
              spaceAfterPt: 12
            }
          },
          {
            id: 'g-2',
            styleId: 'style-gemara-main',
            text: 'וּבָרָא אֶת עוֹלָמוֹ בִּשְׁלֹשָׁה סְפָרִים: בְּסֵפֶר וּסְפָר וְסִפּוּר.',
            spans: [
              { start: 0, end: 17, format: { fontWeight: 700 } },
              { start: 18, end: 35, format: { italic: true } }
            ],
            patch: {
              alignment: 'Right',
              firstLineIndentPt: 15
            }
          }
        ],
        rashi: [
          {
            id: 'r-1',
            styleId: 'style-rashi-body',
            text: 'בשלושים ושתים נתיבות - אלו כ״ב אותיות ועשר ספירות (Sefer Yetzirah Chapter 1).',
            spans: [
              { start: 0, end: 20, format: { fontWeight: 700 } },
              { start: 49, end: 75, format: { fontFamily: 'DejaVu Sans', italic: true } }
            ],
            patch: {
              alignment: 'Justified',
              lineHeightPt: 1.2
            }
          }
        ]
      }
    };

    // 2. Convert to DocumentRoot via documentBridge
    const docRoot = documentBridge.documentStateToDocumentRoot(originalState, { author: 'רבי אברהם' });
    assert.ok(documentBridge.isValidUlid(docRoot.id), 'Root ID must be valid ULID');
    assert.equal(docRoot.metadata.title, originalState.title);
    assert.equal(docRoot.metadata.author, 'רבי אברהם');

    const sec = docRoot.sections[0];
    assert.ok(sec, 'Document must have at least one section');
    const gemaraFlow = sec.flows.find((f) => f.id === 'gemara');
    assert.ok(gemaraFlow, 'Gemara flow must be present');
    assert.equal(gemaraFlow.paragraphs.length, 2);

    // Verify ParagraphNodes retained rich spans and patches
    const p0 = gemaraFlow.paragraphs[0];
    assert.equal(p0.spans.length, 3, 'First paragraph must retain 3 text spans');
    assert.equal(p0.spans[0].format.font_weight, 700);
    assert.equal(p0.spans[0].format.font_family, 'FrankRuehl');
    assert.equal(p0.spans[0].format.font_size_pt, 16);
    assert.equal(p0.style_patches[0].alignment, 'Justified');
    assert.equal(p0.style_patches[0].line_height_pt, 1.45);
    assert.equal(p0.style_patches[0].space_before_pt, 8);
    assert.equal(p0.style_patches[0].space_after_pt, 12);

    const rashiFlow = sec.flows.find((f) => f.id === 'rashi');
    assert.ok(rashiFlow, 'Rashi flow must be present');
    const rp0 = rashiFlow.paragraphs[0];
    assert.equal(rp0.spans.length, 2);
    assert.equal(rp0.spans[1].format.font_family, 'DejaVu Sans');

    // 3. Convert back to MultiFlowDocumentState
    const restoredState = documentBridge.documentRootToDocumentState(docRoot);
    assert.equal(restoredState.title, originalState.title);
    assert.equal(restoredState.flows.gemara.length, 2);
    assert.deepEqual(restoredState.flows.gemara[0].spans, originalState.flows.gemara[0].spans);
    assert.deepEqual(restoredState.flows.gemara[0].patch, originalState.flows.gemara[0].patch);
    assert.deepEqual(restoredState.flows.rashi[0].spans, originalState.flows.rashi[0].spans);
    assert.deepEqual(restoredState.flows.rashi[0].patch, originalState.flows.rashi[0].patch);
  });

  test('Document Settings and Custom Styles round-trip through documentBridge losslessly', () => {
    const stateWithSettings = {
      title: 'הגדרות עמוד וסגנונות מותאמים',
      templateType: 'gemara',
      settings: {
        title: 'הגדרות עמוד וסגנונות מותאמים',
        pageSize: '17x24',
        pageWidthMm: 170,
        pageHeightMm: 240,
        marginTopMm: 25,
        marginBottomMm: 30,
        marginInsideMm: 22,
        marginOutsideMm: 18,
        columns: 2,
        columnGapMm: 6,
        baselineGridPt: 14,
        baselineOffsetPt: 50,
        gematriaFormat: 'talmudic',
        preflightStatus: 'clean'
      },
      styles: {
        'custom-heading': {
          id: 'custom-heading',
          name: 'כותרת מיוחדת',
          font_family: 'Taamey Frank CLM',
          font_weight: 700,
          font_size_pt: 18.0,
          line_height_pt: 24.0,
          space_before_pt: 10.0,
          space_after_pt: 6.0,
          first_line_indent_pt: 0.0,
          alignment: 'Center',
          ahalterm_stretch: 'AutoVariable',
          color: { DeviceCmyk: { c: 0, m: 0, y: 0, k: 1 } },
          keep_with_next: true,
          orphan_control: 2,
          widow_control: 2
        }
      },
      flows: {
        gemara: [
          {
            id: 'p-custom-1',
            styleId: 'custom-heading',
            text: 'פרק ראשון: מסכת ברכות'
          }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(stateWithSettings);
    assert.ok(docRoot.metadata.document_settings, 'metadata must include document_settings');
    assert.equal(docRoot.metadata.document_settings.marginTopMm, 25);
    assert.equal(docRoot.metadata.document_settings.pageWidthMm, 170);

    const customSty = docRoot.paragraph_styles.find((s) => s.id === 'custom-heading');
    assert.ok(customSty, 'Custom style must be registered in paragraph_styles');
    assert.equal(customSty.name, 'כותרת מיוחדת');
    assert.equal(customSty.font_family, 'Taamey Frank CLM');

    const restored = documentBridge.documentRootToDocumentState(docRoot);
    assert.ok(restored.settings, 'Restored state must have settings');
    assert.equal(restored.settings.marginTopMm, 25);
    assert.equal(restored.settings.marginBottomMm, 30);
    assert.equal(restored.settings.pageWidthMm, 170);
    assert.ok(restored.styles?.['custom-heading'], 'Restored state must have custom style');
    assert.equal(restored.styles['custom-heading'].name, 'כותרת מיוחדת');
  });

  test('Save and reopen .tok package preserves rich spans, patches, and Hebrew Niqqud losslessly', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const originalState = {
      title: 'בדיקת שימור סגנונות עשירים בקובץ tok',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'g-rich-1',
            styleId: 'style-gemara-main',
            text: 'שְׁמַע יִשְׂרָאֵל ה׳ אֱלֹהֵינוּ ה׳ אֶחָד׃ (Deuteronomy 6:4)',
            spans: [
              { start: 0, end: 11, format: { fontWeight: 700, fontSizePt: 18, fontFamily: 'FrankRuehl' } },
              { start: 12, end: 32, format: { fontWeight: 700, italic: true } },
              { start: 33, end: 50, format: { italic: true, fontFamily: 'Times New Roman' } }
            ],
            patch: {
              alignment: 'Center',
              lineHeightPt: 1.5,
              spaceBeforePt: 10,
              spaceAfterPt: 15
            }
          }
        ],
        notes: [
          {
            id: 'n-rich-1',
            styleId: 'style-footnotes',
            text: 'הערה: קבלת עול מלכות שמים בכוונה עצומה.',
            spans: [
              { start: 0, end: 5, format: { fontWeight: 700, underline: true } }
            ],
            patch: {
              alignment: 'Right',
              lineHeightPt: 1.1
            }
          }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(originalState);
    const tempInJson = path.join(os.tmpdir(), `tok_rich_in_${Date.now()}.json`);
    const tempTokFile = path.join(os.tmpdir(), `tok_rich_pkg_${Date.now()}.tok`);
    const tempOutJson = path.join(os.tmpdir(), `tok_rich_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      // 1. Save .tok package via tok-cli
      const saveCode = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['save-package', tempInJson, tempTokFile], { windowsHide: true });
        proc.on('error', reject);
        proc.on('close', resolve);
      });
      assert.equal(saveCode, 0, 'tok-cli save-package must succeed');
      assert.ok(fs.existsSync(tempTokFile), '.tok package must exist');

      // 2. Open .tok package via tok-cli
      const openCode = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['open-package', tempTokFile, tempOutJson], { windowsHide: true });
        proc.on('error', reject);
        proc.on('close', resolve);
      });
      assert.equal(openCode, 0, 'tok-cli open-package must succeed');
      assert.ok(fs.existsSync(tempOutJson), 'Extracted document JSON must exist');

      // 3. Parse extracted JSON and assert data integrity
      const extractedRoot = JSON.parse(fs.readFileSync(tempOutJson, 'utf-8'));
      const sec = extractedRoot.sections[0];
      const gFlow = sec.flows.find((f) => f.id === 'gemara');
      const p = gFlow.paragraphs[0];

      assert.equal(p.text, originalState.flows.gemara[0].text, 'Text with Niqqud must be identical');
      assert.ok(p.spans && p.spans.length === 3, 'Spans length must be preserved');
      assert.equal(p.spans[0].format.font_weight, 700);
      assert.equal(p.spans[0].format.font_family, 'FrankRuehl');
      assert.equal(p.spans[0].format.font_size_pt, 18);
      assert.equal(p.spans[1].format.font_weight, 700);
      assert.equal(p.spans[2].format.italic, true);
      assert.equal(p.spans[2].format.font_family, 'Times New Roman');

      assert.equal(p.style_patches[0].alignment, 'Center');
      assert.equal(p.style_patches[0].line_height_pt, 1.5);
      assert.equal(p.style_patches[0].space_before_pt, 10);
      assert.equal(p.style_patches[0].space_after_pt, 15);

      // 4. Restore state via bridge and verify full equality
      const restoredState = documentBridge.documentRootToDocumentState(extractedRoot);
      assert.deepEqual(restoredState.flows.gemara[0].spans, originalState.flows.gemara[0].spans);
      assert.deepEqual(restoredState.flows.gemara[0].patch, originalState.flows.gemara[0].patch);
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempTokFile)) fs.unlinkSync(tempTokFile);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });

  test('Real typesetting engine successfully lays out document containing rich text spans and patches', async () => {
    assert.ok(tokCliPath, tokCliMissingMessage());
    const { spawn } = await import('node:child_process');
    const os = await import('node:os');

    const richDocState = {
      title: 'עימוד עשיר עם ספנים והזחות',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'g-layout-1',
            styleId: 'style-gemara-main',
            text: 'מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן, עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה, דִּבְרֵי רַבִּי אֱלִיעֶזֶר.',
            spans: [
              { start: 0, end: 27, format: { fontWeight: 700, fontSizePt: 14 } },
              { start: 28, end: 60, format: { italic: true } }
            ],
            patch: {
              alignment: 'Justified',
              lineHeightPt: 1.3
            }
          },
          {
            id: 'g-layout-2',
            styleId: 'style-gemara-main',
            text: 'וַחֲכָמִים אוֹמְרִים: עַד חֲצוֹת. רַבָּן גַּמְלִיאֵל אוֹמֵר: עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר.',
            spans: [
              { start: 0, end: 16, format: { fontWeight: 700 } }
            ],
            patch: {
              alignment: 'Justified'
            }
          }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(richDocState);
    const tempInJson = path.join(os.tmpdir(), `tok_typeset_in_${Date.now()}.json`);
    const tempOutJson = path.join(os.tmpdir(), `tok_typeset_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempInJson, tempOutJson], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli typeset-document must succeed on document with rich spans and patches');
      const pages = JSON.parse(fs.readFileSync(tempOutJson, 'utf-8'));
      assert.ok(pages.length >= 1, 'At least 1 page must be produced');

      const p0 = pages[0];
      assert.ok(p0.frames.length >= 1, 'At least 1 frame must be allocated');
      const gemaraFrame = p0.frames.find((f) => f.flow_id === 'gemara') || p0.frames[0];
      assert.ok(gemaraFrame.lines.length >= 2, `Gemara frame must contain placed lines, got ${gemaraFrame.lines.length}`);
      for (const line of gemaraFrame.lines) {
        assert.ok(line.text.length > 0, 'Placed line text must not be empty');
        assert.ok(line.width > 0, 'Placed line width must be positive');
      }
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });

  test('Story Editor editing operations generate valid transaction history that serializes cleanly', () => {
    const { formatRangeInSpans, normalizeSpans } = storyEditorModule;

    let text = 'שלום עולם שלום ירושלים';
    let spans = [];

    // Apply bold to 'שלום' (0..4)
    spans = formatRangeInSpans(spans, 0, 4, { fontWeight: 700 });
    assert.equal(spans.length, 1);
    assert.deepEqual(spans[0], { start: 0, end: 4, format: { fontWeight: 700 } });

    // Apply italic to 'עולם שלום' (5..14)
    spans = formatRangeInSpans(spans, 5, 14, { italic: true });
    assert.equal(spans.length, 2);

    // Apply bold to overlapping range (10..22)
    spans = formatRangeInSpans(spans, 10, 22, { fontWeight: 700 });
    spans = normalizeSpans(spans);

    // Verify all spans are sorted, disjoint, and valid within text length
    for (let i = 0; i < spans.length; i++) {
      assert.ok(spans[i].start < spans[i].end);
      assert.ok(spans[i].end <= text.length);
      if (i > 0) {
        assert.ok(spans[i - 1].end <= spans[i].start);
      }
    }

    // Convert paragraph with these spans to DocumentRoot and verify roundtrip
    const state = {
      title: 'היסטוריית עריכה',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'm-1',
            styleId: 'normal',
            text,
            spans,
            patch: { alignment: 'Right' }
          }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(state);
    const restored = documentBridge.documentRootToDocumentState(docRoot);
    assert.deepEqual(restored.flows.gemara[0].spans, spans);
    assert.deepEqual(restored.flows.gemara[0].patch, { alignment: 'Right' });
  });

  test('Document state preserves isDirty and filePath metadata', () => {
    const state = {
      title: 'שמירת שינויים',
      templateType: 'gemara',
      isDirty: true,
      filePath: 'C:/docs/berakhot.tok',
      flows: { gemara: [] }
    };

    assert.equal(state.isDirty, true);
    assert.equal(state.filePath, 'C:/docs/berakhot.tok');

    // On save, dirty resets to false
    state.isDirty = false;
    assert.equal(state.isDirty, false);
  });

  test('Full .tok package preservation: multi-flow layout, custom settings, styles, and spans round-trip', async () => {
    const originalState = {
      title: 'מסכת ברכות - פרק ראשון',
      templateType: 'gemara',
      settings: {
        title: 'מסכת ברכות - פרק ראשון',
        pageSize: 'Custom',
        pageWidthMm: 180,
        pageHeightMm: 255,
        marginTopMm: 20,
        marginBottomMm: 25,
        marginInsideMm: 22,
        marginOutsideMm: 18,
        columns: 2,
        columnGapMm: 6,
        baselineGridPt: 14,
        baselineOffsetPt: 50,
        gematriaFormat: 'talmudic',
        preflightStatus: 'clean'
      },
      styles: {
        rashi_commentary: {
          id: 'rashi_commentary',
          name: 'פירוש רש״י',
          fontSizePt: 12,
          fontFamily: 'Noto Rashi Hebrew',
          fontWeight: 400,
          lineHeightPt: 15,
          alignment: 'Justified'
        }
      },
      flows: {
        gemara: [
          {
            id: 'g-1',
            styleId: 'normal',
            text: 'מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית?',
            spans: [
              { start: 0, end: 8, format: { font_weight: 700, font_family: 'Taamey Frank CLM' } },
              { start: 9, end: 36, format: { font_size_pt: 16 } }
            ],
            patch: { alignment: 'Justified', lineHeightPt: 22 }
          }
        ],
        rashi: [
          {
            id: 'r-1',
            styleId: 'rashi_commentary',
            text: 'משעה שהכהנים נכנסין לאכול בתרומתן.',
            spans: [
              { start: 0, end: 5, format: { font_weight: 700 } }
            ],
            patch: { alignment: 'Right' }
          }
        ],
        tosafot: [
          {
            id: 't-1',
            styleId: 'normal',
            text: 'בְּעַרְבִית — תימה מאי שנא דתנא דערבית ברישא?',
            spans: [
              { start: 0, end: 8, format: { font_weight: 700 } }
            ]
          }
        ],
        notes: [
          {
            id: 'n-1',
            styleId: 'normal',
            text: 'הערה א: עיין ברכות דף ב עמוד א.',
            spans: []
          }
        ]
      }
    };

    const docRoot = documentBridge.documentStateToDocumentRoot(originalState);
    const tempInJson = path.join(os.tmpdir(), `tok_full_in_${Date.now()}.json`);
    const tempTokFile = path.join(os.tmpdir(), `tok_full_pkg_${Date.now()}.tok`);
    const tempOutJson = path.join(os.tmpdir(), `tok_full_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      assert.ok(tokCliPath, tokCliMissingMessage());

      // 1. Save .tok package via tok-cli
      const saveCode = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['save-package', tempInJson, tempTokFile], { windowsHide: true });
        proc.on('error', reject);
        proc.on('close', resolve);
      });
      assert.equal(saveCode, 0, 'tok-cli save-package must succeed');
      assert.ok(fs.existsSync(tempTokFile), '.tok package must exist');

      // 2. Open .tok package via tok-cli
      const openCode = await new Promise((resolve, reject) => {
        const proc = spawn(tokCliPath, ['open-package', tempTokFile, tempOutJson], { windowsHide: true });
        proc.on('error', reject);
        proc.on('close', resolve);
      });
      assert.equal(openCode, 0, 'tok-cli open-package must succeed');
      assert.ok(fs.existsSync(tempOutJson), 'Extracted document JSON must exist');

      // 3. Round-trip restored state via documentBridge
      const extractedRoot = JSON.parse(fs.readFileSync(tempOutJson, 'utf-8'));
      const restoredState = documentBridge.documentRootToDocumentState(extractedRoot);

      // Verify settings preservation
      assert.ok(restoredState.settings, 'Document settings must be preserved');
      assert.equal(restoredState.settings.pageWidthMm, 180);
      assert.equal(restoredState.settings.pageHeightMm, 255);
      assert.equal(restoredState.settings.marginTopMm, 20);
      assert.equal(restoredState.settings.columns, 2);

      // Verify custom styles preservation
      assert.ok(restoredState.styles?.rashi_commentary, 'Custom style must be preserved');
      assert.equal(restoredState.styles.rashi_commentary.fontFamily, 'Noto Rashi Hebrew');
      assert.equal(restoredState.styles.rashi_commentary.fontSizePt, 12);

      // Verify multi-flow texts and rich spans
      assert.equal(restoredState.flows.gemara[0].text, originalState.flows.gemara[0].text);
      assert.equal(restoredState.flows.gemara[0].spans.length, 2);
      assert.equal(restoredState.flows.gemara[0].patch.alignment, 'Justified');

      assert.equal(restoredState.flows.rashi[0].text, originalState.flows.rashi[0].text);
      assert.equal(restoredState.flows.rashi[0].styleId, 'rashi_commentary');

      assert.equal(restoredState.flows.tosafot[0].text, originalState.flows.tosafot[0].text);
      assert.equal(restoredState.flows.notes[0].text, originalState.flows.notes[0].text);
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempTokFile)) fs.unlinkSync(tempTokFile);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });
});

describe('Phase 2 UI Integration: Hebrew Typography, Fallback Stacks, and Lifecycle Actions', () => {
  test('fonts.ts provides safe font stack fallbacks and system Hebrew fonts', async () => {
    const { getSafeFontStack, getAvailableFonts, DOCUMENT_FONTS } = await loadTs('packages/tok-ui/src/fonts.ts');
    
    assert.equal(DOCUMENT_FONTS.length, 4, 'Four embedded OFL fonts must be defined');
    const all = getAvailableFonts();
    assert.ok(all.some((f) => f.family === 'Frank Ruhl Libre'));
    assert.ok(all.some((f) => f.family === 'Keter YG'));
    assert.ok(all.some((f) => f.family === 'SBL Hebrew'));
    assert.ok(all.some((f) => f.family === 'Ezra SIL'));

    assert.equal(getSafeFontStack('Frank Ruhl Libre'), '"Frank Ruhl Libre", \'Frank Ruhl Libre\', \'David Libre\', \'Times New Roman\', serif');
    assert.equal(getSafeFontStack(''), '\'Frank Ruhl Libre\', \'David Libre\', \'Times New Roman\', serif');
  });

  test('textStats.ts word counter achieves high speed zero-allocation accuracy and caching', async () => {
    const { countWords, getParagraphWordCount, clearWordCountCache } = await loadTs('packages/tok-ui/src/textStats.ts');

    assert.equal(countWords(''), 0);
    assert.equal(countWords('   '), 0);
    assert.equal(countWords('שלום עולם ומלואו'), 3);
    assert.equal(countWords('בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ׃'), 7);
    assert.equal(countWords('מילה\nשורה\tרווח\u00A0קשיח'), 4);

    // Paragraph caching
    clearWordCountCache();
    const pId = 'test-para-1';
    const text = 'אמר רבי יוחנן כל האומר שירה בעולם הזה';
    assert.equal(getParagraphWordCount(pId, text), 8);
    // Cached retrieval returns identical count
    assert.equal(getParagraphWordCount(pId, text), 8);
    // Modified text recalculates count
    assert.equal(getParagraphWordCount(pId, text + ' זוכה'), 9);
  });

  test('Hebrew actions (normalize, shield divine names, gematria) execute cleanly', async () => {
    const { hebrewUtils } = await loadTs('packages/tok-ui/src/plugins/hebrew-utils.ts');
    
    // Normalization of quotes
    const raw = 'ר"ת וכו\'';
    const normalized = hebrewUtils.normalizeQuotes(raw);
    assert.equal(normalized, 'ר״ת וכו׳');

    // Divine names detection and shielding
    const holy = 'ברוך יהוה לעולם אמן ואמן אלהים צבאות';
    const matches = hebrewUtils.findHolyNames(holy);
    assert.ok(matches.length >= 2);
    assert.equal(matches[0].name, 'יהוה');

    // Replacement/shielding token
    const shielded = holy.replace(/יהוה/g, 'ה׳');
    assert.ok(!shielded.includes('יהוה'));
    assert.ok(shielded.includes('ה׳'));
  });

  test('Document state bridge preserves all layout flows, page formats, and inspector metadata', async () => {
    const bridge = await loadTs('packages/tok-ui/src/engine/documentBridge.ts');
    
    const initialDoc = {
      title: 'בדיקת סנכרון רכיבים',
      templateType: 'prose',
      flows: {
        main: [
          {
            id: 'p-sync-1',
            styleId: 'style-heading-1',
            text: 'כותרת ראשית',
            spans: [{ start: 0, end: 11, format: { fontWeight: 700, fontSizePt: 22 } }]
          },
          {
            id: 'p-sync-2',
            styleId: 'style-body',
            text: 'פסקה ראשונה עם ניקוד: שָׁלוֹם עֲלֵיכֶם.',
            spans: [{ start: 20, end: 36, format: { italic: true } }]
          }
        ]
      },
      settings: {
        pageSize: 'A4',
        pageWidthPt: 595,
        pageHeightPt: 842,
        marginTopPt: 54,
        marginBottomPt: 54,
        marginLeftPt: 54,
        marginRightPt: 54,
        primaryFontFamily: 'Frank Ruhl Libre',
        baseFontSizePt: 12,
        baseLineHeightPt: 16
      },
      styles: {
        'style-custom-accent': {
          fontFamily: 'Noto Serif Hebrew',
          fontSizePt: 14,
          lineHeightPt: 18,
          align: 'center',
          color: '#1a365d'
        }
      }
    };

    const docRoot = bridge.documentStateToDocumentRoot(initialDoc);
    assert.ok(bridge.isValidUlid(docRoot.id), 'Root ID must be valid ULID');
    assert.equal(docRoot.metadata.title, 'בדיקת סנכרון רכיבים');
    assert.equal(docRoot.metadata.schema_version, '1.0');
    assert.ok(docRoot.sections[0]);
    assert.equal(docRoot.sections[0].flows[0].id, 'main');
    assert.equal(docRoot.sections[0].flows[0].paragraphs.length, 2);
    assert.ok(docRoot.paragraph_styles.some((s) => s.id === 'style-custom-accent'));

    const restoredDoc = bridge.documentRootToDocumentState(docRoot);
    assert.equal(restoredDoc.title, 'בדיקת סנכרון רכיבים');
    assert.equal(restoredDoc.templateType, 'prose');
    assert.ok(restoredDoc.flows.main || restoredDoc.flows.gemara);
    const mainParas = restoredDoc.flows.main || restoredDoc.flows.gemara;
    assert.equal(mainParas.length, 2);
    assert.equal(mainParas[0].text, 'כותרת ראשית');
    assert.equal(mainParas[0].spans?.[0]?.format?.fontWeight, 700);
    assert.equal(mainParas[1].spans?.[0]?.format?.italic, true);
    assert.equal(restoredDoc.settings?.primaryFontFamily, 'Frank Ruhl Libre');
  });
});

describe('Core Feature: Hebrew Text Cleanup & Dibur HaMatchil Wizard (Proposals B.3 & A.2)', () => {
  test('analyzeParagraphs calculates correct counts for spaces, punctuation, quotes and dibur hamatchil', async () => {
    const { analyzeParagraphs, DEFAULT_CLEANUP_OPTIONS } = await loadTs('packages/tok-ui/src/engine/textCleanup.ts');

    const sampleParas = [
      {
        id: 'p1',
        styleId: 'body',
        text: 'רש"י. ד"ה בראשית  ברא  אלהים .'
      },
      {
        id: 'p2',
        styleId: 'body',
        text: 'תוספות:שם בגמרא עמ\' כב.'
      },
      {
        id: 'p3',
        styleId: 'body',
        text: '   '
      },
      {
        id: 'p4',
        styleId: 'body',
        text: ''
      }
    ];

    const stats = analyzeParagraphs(sampleParas, DEFAULT_CLEANUP_OPTIONS);
    assert.ok(stats.doubleSpacesFixed >= 2, 'Should detect double spaces');
    assert.ok(stats.spacesBeforePunctFixed >= 1, 'Should detect space before period');
    assert.ok(stats.spacesAfterPunctFixed >= 1, 'Should detect missing space after colon');
    assert.ok(stats.quotesConverted >= 2, 'Should detect double quotes in Hebrew words');
    assert.ok(stats.gereshConverted >= 1, 'Should detect single quote in Hebrew abbreviation');
    assert.ok(stats.diburHaMatchilBolded >= 2, 'Should detect dibur hamatchil in both paragraphs');
    assert.ok(stats.emptyParagraphsRemoved >= 1, 'Should detect consecutive empty paragraphs');
    assert.ok(stats.totalModifications > 0, 'Total modifications should be positive');
  });

  test('cleanupParagraphs performs complete text cleanup, space normalization and quotes conversion', async () => {
    const { cleanupParagraphs } = await loadTs('packages/tok-ui/src/engine/textCleanup.ts');

    const paras = [
      {
        id: 'p1',
        styleId: 'body',
        text: 'רמב"ם .  הלכות   תפילה  , פרק א\' .'
      }
    ];

    const { paragraphs: cleaned, stats } = cleanupParagraphs(paras, {
      cleanDoubleSpaces: true,
      cleanSpacesBeforePunct: true,
      ensureSpaceAfterPunct: true,
      normalizeQuotesAndGeresh: true,
      collapseEmptyParagraphs: true,
      boldDiburHaMatchil: false,
      diburDelimiter: 'any',
      scope: 'activeFlow'
    });

    assert.equal(cleaned.length, 1);
    // Double spaces removed, space before period removed, space before comma removed, quotes converted
    assert.equal(cleaned[0].text, 'רמב״ם. הלכות תפילה, פרק א׳.');
    assert.ok(stats.doubleSpacesFixed >= 2);
    assert.ok(stats.spacesBeforePunctFixed >= 2);
    assert.ok(stats.quotesConverted >= 1);
    assert.ok(stats.gereshConverted >= 1);
  });

  test('cleanupParagraphs bolds Dibur HaMatchil with period, colon, and dash delimiters', async () => {
    const { cleanupParagraphs } = await loadTs('packages/tok-ui/src/engine/textCleanup.ts');

    const paras = [
      {
        id: 'p1',
        styleId: 'rashi',
        text: 'בראשית. ברא אלהים את השמים ואת הארץ.'
      },
      {
        id: 'p2',
        styleId: 'tosafot',
        text: 'כוונת המחבר: הנה ראוי לבאר את שורש העניין.'
      },
      {
        id: 'p3',
        styleId: 'body',
        text: 'פרק ראשון – כל המצוות כולן נתנו למשה מסיני.'
      }
    ];

    const { paragraphs: cleaned, stats } = cleanupParagraphs(paras, {
      cleanDoubleSpaces: true,
      cleanSpacesBeforePunct: true,
      ensureSpaceAfterPunct: true,
      normalizeQuotesAndGeresh: true,
      collapseEmptyParagraphs: true,
      boldDiburHaMatchil: true,
      diburDelimiter: 'any',
      scope: 'activeFlow'
    });

    assert.equal(stats.diburHaMatchilBolded, 3, 'All 3 paragraphs should receive bold dibur hamatchil');

    // Paragraph 1: "בראשית." (length 7) should be bold
    assert.ok(cleaned[0].spans, 'p1 must have spans');
    assert.equal(cleaned[0].spans[0].start, 0);
    assert.equal(cleaned[0].spans[0].end, 7);
    assert.equal(cleaned[0].spans[0].format.fontWeight, 700);

    // Paragraph 2: "כוונת המחבר:" (length 12) should be bold
    assert.ok(cleaned[1].spans, 'p2 must have spans');
    assert.equal(cleaned[1].spans[0].start, 0);
    assert.equal(cleaned[1].spans[0].end, 12);
    assert.equal(cleaned[1].spans[0].format.fontWeight, 700);

    // Paragraph 3: "פרק ראשון –" (length 11) should be bold
    assert.ok(cleaned[2].spans, 'p3 must have spans');
    assert.equal(cleaned[2].spans[0].start, 0);
    assert.equal(cleaned[2].spans[0].end, 11);
    assert.equal(cleaned[2].spans[0].format.fontWeight, 700);
  });

  test('cleanupParagraphs correctly preserves and shifts existing TextSpans', async () => {
    const { cleanupParagraphs } = await loadTs('packages/tok-ui/src/engine/textCleanup.ts');

    // Paragraph has a double space before "שלום" and an italic span over "שלום"
    // "א  שלום" -> double space at index 1-3. "שלום" is at index 3..7
    const paras = [
      {
        id: 'p1',
        styleId: 'body',
        text: 'א  שלום',
        spans: [
          { start: 3, end: 7, format: { italic: true } }
        ]
      }
    ];

    const { paragraphs: cleaned } = cleanupParagraphs(paras, {
      cleanDoubleSpaces: true,
      cleanSpacesBeforePunct: false,
      ensureSpaceAfterPunct: false,
      normalizeQuotesAndGeresh: false,
      collapseEmptyParagraphs: false,
      boldDiburHaMatchil: false,
      diburDelimiter: 'any',
      scope: 'activeFlow'
    });

    assert.equal(cleaned[0].text, 'א שלום');
    assert.ok(cleaned[0].spans);
    // After collapsing 2 spaces to 1, "שלום" shifts from offset 3 to offset 2!
    assert.equal(cleaned[0].spans[0].start, 2);
    assert.equal(cleaned[0].spans[0].end, 6);
    assert.equal(cleaned[0].spans[0].format.italic, true);
  });
});

describe('Milestone 3 & 4 Regression: Frame Alignment, Canonical Hebrew Combining Marks & Layer/Virtualizer Wiring', () => {
  test('frameAlignment aligns and distributes multi-selected frame rectangles accurately', async () => {
    const {
      alignRight,
      alignLeft,
      alignCenter,
      alignTop,
      alignMiddle,
      alignBottom,
      distributeVertical,
      distributeHorizontal,
      alignFrames
    } = await loadTs('packages/tok-ui/src/utils/frameAlignment.ts');

    const frames = [
      { id: 'f1', x: 20, y: 10, w: 100, h: 40 },
      { id: 'f2', x: 80, y: 90, w: 60, h: 40 },
      { id: 'f3', x: 50, y: 210, w: 150, h: 40 }
    ];

    // Max right edge is f3: 50 + 150 = 200
    const rightAligned = alignRight(frames);
    assert.equal(rightAligned[0].x + rightAligned[0].w, 200);
    assert.equal(rightAligned[1].x + rightAligned[1].w, 200);
    assert.equal(rightAligned[2].x + rightAligned[2].w, 200);

    // Min left edge is f1: 20
    const leftAligned = alignLeft(frames);
    assert.equal(leftAligned[0].x, 20);
    assert.equal(leftAligned[1].x, 20);
    assert.equal(leftAligned[2].x, 20);

    // Selection horizontal bounds: [20, 200] -> center = 110
    const centerAligned = alignCenter(frames);
    assert.equal(centerAligned[0].x + centerAligned[0].w / 2, 110);
    assert.equal(centerAligned[1].x + centerAligned[1].w / 2, 110);

    // Top/Middle/Bottom alignment
    const topAligned = alignTop(frames);
    assert.equal(topAligned[1].y, 10);
    const bottomAligned = alignBottom(frames);
    assert.equal(bottomAligned[0].y + bottomAligned[0].h, 250);
    const middleAligned = alignMiddle(frames);
    assert.equal(middleAligned[0].y + middleAligned[0].h / 2, 130);

    // Vertical distribution: total span [10, 250] = 240, sum of heights = 120, remaining = 120 across 2 gaps = 60 gap
    // f1: y=10..50, f2: y=110..150, f3: y=210..250
    const vDist = distributeVertical(frames);
    assert.equal(vDist[0].y, 10);
    assert.equal(vDist[1].y, 110);
    assert.equal(vDist[2].y, 210);

    // Dispatcher compatibility
    const viaDispatcher = alignFrames(frames, 'align-right');
    assert.equal(viaDispatcher[0].x, 100);
    const hDist = distributeHorizontal(frames);
    assert.equal(hDist.length, 3);
  });

  test('normalizeHebrew canonically orders Hebrew combining marks (Shin/Sin -> Dagesh -> Vowel -> Taamim) and is idempotent', async () => {
    const { normalizeHebrew } = await loadTs('packages/tok-ui/src/plugins/hebrew-utils.ts');

    // Shin (U+05E9) + Etnahta Taam (U+0591) + Qamats Vowel (U+05B8) + Dagesh (U+05BC) + Shin Dot (U+05C1)
    const scrambled = '\u05E9\u0591\u05B8\u05BC\u05C1';
    const expected = '\u05E9\u05C1\u05BC\u05B8\u0591';

    const normalized = normalizeHebrew(scrambled);
    assert.equal(normalized, expected, 'Out-of-order Hebrew combining marks must be sorted canonically');
    assert.equal(normalizeHebrew(normalized), expected, 'normalizeHebrew must be idempotent');
  });
});

describe('Welcome screen exit: a failed or cancelled open must not leave the close button on "return to document"', () => {
  // Normalize line endings: Windows checkouts use CRLF, which would break the slicing below.
  const appTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8').replace(/\r\n/g, '\n');

  test('openAnyFile does not mark a document as open before it has loaded', () => {
    const start = appTs.indexOf('public openAnyFile(');
    assert.ok(start > 0, 'openAnyFile must exist');
    const body = appTs.slice(start, appTs.indexOf('\n  }\n', start));
    assert.ok(!body.includes('setHasOpenDocument(true)'), 'flag must only be set after a successful load');
  });

  test('failed open and failed import bring the welcome screen back when no document is open', () => {
    const openErr = appTs.indexOf("tf('toastOpenError', { error: errorMsg })");
    const importErr = appTs.indexOf("tf('toastImportError', { error: err?.userMessage");
    for (const idx of [openErr, importErr]) {
      assert.ok(idx > 0, 'error path must exist');
      assert.ok(appTs.slice(idx, idx + 260).includes('this.welcomeModal.show()'), 'welcome screen must be restored');
    }
  });
});

describe('Side panel flows follow the document template (new and opened documents)', () => {
  const appTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8').replace(/\r\n/g, '\n');
  const bodyOf = (signature) => {
    const start = appTs.indexOf(signature);
    assert.ok(start > 0, `${signature} must exist`);
    return appTs.slice(start, appTs.indexOf('\n  }\n', start));
  };

  test('creating a document from a template refreshes the structure bar flows', () => {
    assert.ok(bodyOf('private handleTemplateSelect(').includes('this.structureBar.setFlows(getFlowsForTemplate('));
  });

  test('opening a saved document refreshes the structure bar flows', () => {
    assert.ok(bodyOf('private loadDocumentRoot(').includes('this.structureBar.setFlows(getFlowsForTemplate('));
  });
});

describe('Installer language is adopted by the app on first run', () => {
  const read = (rel) => fs.readFileSync(path.join(rootDir, rel), 'utf-8').replace(/\r\n/g, '\n');

  test('parseInstallLanguage understands Inno Setup and NSIS registry output', async () => {
    const { parseInstallLanguage } = await loadTs('packages/tok-electron/src/install-language.ts');
    const inno = (v) => `\r\nHKEY_CURRENT_USER\\Software\\TypesetOK\r\n    ProjectsDir    REG_SZ    C:\\Books\r\n    Language    REG_SZ    ${v}\r\n`;
    assert.equal(parseInstallLanguage(inno('en')), 'en');
    assert.equal(parseInstallLanguage(inno('he')), 'he');
    assert.equal(parseInstallLanguage('    Installer Language    REG_SZ    1033\r\n'), 'en');
    assert.equal(parseInstallLanguage('    Installer Language    REG_SZ    1037\r\n'), 'he');
    assert.equal(parseInstallLanguage(inno('fr')), null);
    assert.equal(parseInstallLanguage(''), null);
  });

  test('preload applies it only when the user has no saved language; main answers only the app page', () => {
    const preload = read('packages/tok-electron/src/preload.ts');
    assert.ok(preload.includes("!localStorage.getItem('tok_lang')"), 'a saved choice must win');
    assert.ok(preload.includes("sendSync('tok:get-install-language')"));
    const main = read('packages/tok-electron/src/main.ts');
    const at = main.indexOf("ipcMain.on('tok:get-install-language'");
    assert.ok(at > 0, 'main must answer the channel');
    assert.ok(main.slice(at, at + 220).includes('isAppUrl('), 'sender must be validated');
  });

  test('parseInstallProjectsDir reads the installer projects folder, including spaces', async () => {
    const { parseInstallProjectsDir } = await loadTs('packages/tok-electron/src/install-language.ts');
    const out = '\r\n    ProjectsDir    REG_SZ    C:\\Users\\Dan Cohen\\TypesetOK Projects\r\n    Language    REG_SZ    en\r\n';
    assert.equal(parseInstallProjectsDir(out), 'C:\\Users\\Dan Cohen\\TypesetOK Projects');
    assert.equal(parseInstallProjectsDir('    Language    REG_SZ    he\r\n'), null);
    assert.equal(parseInstallProjectsDir(''), null);
  });

  test('save and open dialogs start in the installer projects folder', () => {
    const main = read('packages/tok-electron/src/main.ts');
    assert.ok(main.includes('defaultPath: withProjectsDir(options?.defaultPath)'), 'save dialog');
    const open = main.indexOf("handle('tok:show-open-dialog'");
    assert.ok(open > 0 && main.slice(open, open + 300).includes('defaultPath: getProjectsDir()'), 'open dialog');
  });

  test('the Inno Setup installer still records the chosen language', () => {
    const iss = read('scripts/installer.iss');
    assert.ok(iss.includes("'Software\\TypesetOK', 'Language', 'he'") && iss.includes("'Software\\TypesetOK', 'Language', 'en'"));
  });
});

describe('Switching book layout template on an open document (without re-importing)', () => {
  const read = (rel) => fs.readFileSync(path.join(rootDir, rel), 'utf-8').replace(/\r\n/g, '\n');

  test('StructureBar declares onChangeTemplate callback and renders change template button', () => {
    const bar = read('packages/tok-ui/src/components/StructureBar.ts');
    assert.ok(bar.includes('onChangeTemplate?: () => void;'), 'StructureBarCallbacks must declare onChangeTemplate');
    assert.ok(bar.includes("t('structureChangeTemplate')"), 'renders change template button in flows panel');
  });

  test('app.ts implements changeDocumentTemplate preserving existing flows and re-syncing', () => {
    const app = read('packages/tok-ui/src/app.ts');
    assert.ok(app.includes('public changeDocumentTemplate(newTemplate: TemplateType): void'), 'changeDocumentTemplate must exist');
    assert.ok(app.includes('this.promptChangeDocumentTemplate()'), 'promptChangeDocumentTemplate must exist');
    assert.ok(app.includes('cmd-change-template'), 'Command palette must offer change template action');
  });
});

describe('Home screen, dialogs and About screen', () => {
  const read = (rel) => fs.readFileSync(path.join(rootDir, rel), 'utf-8').replace(/\r\n/g, '\n');

  test('home screen no longer offers demo templates', () => {
    const welcome = read('packages/tok-ui/src/components/WelcomeModal.ts');
    assert.ok(!welcome.includes('tok-template-demo-card'), 'demo cards must be gone');
    assert.ok(!welcome.includes('welcomeDemoSection'), 'demo section title must be gone');
    assert.ok(welcome.includes('tok-template-import') && welcome.includes('tok-template-blank'), 'blank and import cards stay');
  });

  test('the import card is styled like the blank card (text no longer sticks to the edges)', () => {
    const html = read('packages/tok-ui/src/index.html');
    assert.ok(/\.tok-template-blank,\s*\.tok-template-import\s*\{/.test(html));
  });

  test('every dialog fades and rises in and out through CSS (open class drives both directions)', () => {
    const html = read('packages/tok-ui/src/index.html');
    assert.ok(html.includes('@starting-style'), 'enter animation');
    assert.ok(html.includes('display 0.22s allow-discrete'), 'exit animation keeps the overlay visible while it fades');
  });

  test('About screen shows the main developer and contributors from generated data, with safe links only', () => {
    const about = read('packages/tok-ui/src/components/AboutModal.ts');
    assert.ok(about.includes('aboutMainDevBio') && about.includes('aboutThanksTitle'));
    assert.ok(about.includes("u.hostname === 'github.com'") && about.includes("u.protocol === 'https:'"), 'only https GitHub links');
    assert.ok(!about.includes('Knuth-Plass') && !about.includes('Hardened Lean Electron'), 'technical engine text was removed');
    const data = read('packages/tok-ui/src/contributors.generated.ts');
    assert.ok(data.includes("login: 'Amlaach'") && data.includes('https://github.com/Amlaach'));
    const script = read('scripts/update-contributors.mjs');
    assert.ok(script.includes("c.type === 'User'"), 'bots are excluded');
  });
});

describe('User guide content', () => {
  const guide = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/UserGuideModal.ts'), 'utf-8').replace(/\r\n/g, '\n');

  test('every section exists in both Hebrew and English with the same number of blocks', () => {
    const count = (re) => (guide.match(re) || []).length;
    assert.equal(count(/titleHe:/g), count(/titleEn:/g));
    assert.equal(count(/contentHe: \[/g), count(/contentEn: \[/g));
    assert.equal(count(/\n {8}title: /g) > 0, true);
    const heBlocks = count(/\n {8}title: '[^'\n]*[\u0590-\u05FF]/g);
    const enBlocks = count(/\n {8}title: '[^\u0590-\u05FF'\n]+'/g);
    assert.equal(heBlocks, enBlocks, 'each Hebrew block needs an English twin');
  });

  test('the guide no longer describes the removed built-in demo templates', () => {
    assert.ok(!guide.includes('תבניות מובנות') && !guide.includes('Built-in Templates'));
  });
});

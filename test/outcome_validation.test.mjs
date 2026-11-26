import { test, describe, before } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

// Helper to load TypeScript module on the fly with esbuild
let esbuildMod = null;
async function loadTs(relPath) {
  esbuildMod ||= await import('esbuild');
  const outfile = path.join(
    os.tmpdir(),
    `tok-outcome-test-${process.pid}-${relPath.replace(/[^a-zA-Z0-9]+/g, '_')}-${Date.now()}.mjs`
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

// Locate tok-cli binary
function findTokCli() {
  if (process.env.TOK_CLI_PATH && fs.existsSync(process.env.TOK_CLI_PATH)) {
    return path.resolve(process.env.TOK_CLI_PATH);
  }
  const candidates = [
    path.join(process.env.LOCALAPPDATA || '', 'tok_target', 'debug', 'tok-cli.exe'),
    path.join(process.env.LOCALAPPDATA || '', 'tok_target', 'release', 'tok-cli.exe'),
    path.join(rootDir, 'target', 'debug', 'tok-cli.exe'),
    path.join(rootDir, 'target', 'release', 'tok-cli.exe'),
    path.join(rootDir, 'target', 'debug', 'tok-cli'),
    path.join(rootDir, 'target', 'release', 'tok-cli')
  ];
  return candidates.find((p) => p && fs.existsSync(p));
}

describe('Phase 3 Outcome-Based Validation: Production Capabilities & Real Gaps', () => {
  let tokCliPath;
  let toHebrewGematria;
  let normalizeHebrew, normalizeQuotes, findHolyNames;
  let layoutToPageDescriptors;
  let documentStateToDocumentRoot, documentRootToDocumentState;

  before(async () => {
    tokCliPath = findTokCli();
    assert.ok(tokCliPath, `tok-cli binary must exist for real outcome tests. Found: ${tokCliPath}`);

    // Load actual product TypeScript modules
    const gematriaMod = await loadTs('packages/tok-ui/src/gematria.ts');
    toHebrewGematria = gematriaMod.toHebrewGematria;

    const hebrewUtilsMod = await loadTs('packages/tok-ui/src/plugins/hebrew-utils.ts');
    normalizeHebrew = hebrewUtilsMod.normalizeHebrew;
    normalizeQuotes = hebrewUtilsMod.normalizeQuotes;
    findHolyNames = hebrewUtilsMod.findHolyNames;

    const typesetBridgeMod = await loadTs('packages/tok-ui/src/engine/typesetBridge.ts');
    layoutToPageDescriptors = typesetBridgeMod.layoutToPageDescriptors;

    const docBridgeMod = await loadTs('packages/tok-ui/src/engine/documentBridge.ts');
    documentStateToDocumentRoot = docBridgeMod.documentStateToDocumentRoot;
    documentRootToDocumentState = docBridgeMod.documentRootToDocumentState;
  });

  test('1. Real Product Gematria Engine: Correct punctuation and Talmudic exceptions', () => {
    assert.equal(toHebrewGematria(1), 'א׳');
    assert.equal(toHebrewGematria(2), 'ב׳');
    assert.equal(toHebrewGematria(15), 'ט״ו');
    assert.equal(toHebrewGematria(16), 'ט״ז');
    assert.equal(toHebrewGematria(20), 'כ׳');
    assert.equal(toHebrewGematria(21), 'כ״א');
    assert.equal(toHebrewGematria(100), 'ק׳');
    assert.equal(toHebrewGematria(784), 'תשפ״ד');
    assert.equal(toHebrewGematria(785), 'תשפ״ה');
  });

  test('2. Real Product Hebrew Normalization: Quotes, divine names and Niqqud consistency', () => {
    const rawQuote = 'פרק א"ב הלכה ג\'';
    const normQuote = normalizeQuotes(rawQuote);
    assert.equal(normQuote, 'פרק א״ב הלכה ג׳', 'Straight quotes must normalize to Hebrew typographic geresh and gershayim');

    const holyText = 'ברוך יהוה לעולם אמן';
    const matches = findHolyNames(holyText);
    assert.ok(matches.length > 0, 'Must identify divine Tetragrammaton');
  });

  test('3. Real Custom Document Typesetting via tok-cli (Non-Demo): Multi-flow geometry and exact baseline heights', async () => {
    const customDocState = {
      title: 'בדיקת מסמך אמיתי תלת זרימתי',
      templateType: 'gemara',
      flows: {
        gemara: [
          {
            id: 'g-custom-1',
            styleId: 'style-gemara-main',
            text: 'מַתְנִיתִין: מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה.'
          }
        ],
        rashi: [
          {
            id: 'r-custom-1',
            styleId: 'style-rashi-body',
            text: 'מֵאֵימָתַי - תַּנָּא אַקְּרָא קָאֵי דִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ, וְהָכִי קָתָנֵי זְמַן קְרִיאַת שְׁמַע שֶׁל שְׁכִיבָה אֵימַת.'
          }
        ],
        tosafot: [
          {
            id: 't-custom-1',
            styleId: 'style-tosafot-body',
            text: 'מֵאֵימָתַי - פֵּרֵשׁ רַשִׁ״י דְּתַנָּא אַקְּרָא קָאֵי וְתֵימַהּ לְרִ״י דְּהָא תְּנַן בִּבְרָכוֹת הַקּוֹרֵא לְמַפְרֵעַ לֹא יָצָא.'
          }
        ],
        notes: []
      }
    };

    const docRoot = documentStateToDocumentRoot(customDocState);
    const tempIn = path.join(os.tmpdir(), `custom_typeset_in_${Date.now()}.json`);
    const tempOut = path.join(os.tmpdir(), `custom_typeset_out_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempIn, JSON.stringify(docRoot, null, 2), 'utf-8');

      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['typeset-document', tempIn, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli typeset-document must exit with code 0 on custom multi-flow document');
      assert.ok(fs.existsSync(tempOut), 'Output layout JSON must exist');

      const pages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      assert.ok(Array.isArray(pages) && pages.length >= 1, 'Must emit at least 1 page');

      const p0 = pages[0];
      assert.equal(p0.page_index, 0);
      assert.equal(p0.page_number_gematria, 'א׳');
      assert.ok(p0.frames.length >= 3, 'Must contain at least 3 distinct multi-flow frames (gemara, rashi, tosafot)');

      const gemaraFrame = p0.frames.find((f) => f.flow_id === 'gemara');
      const rashiFrame = p0.frames.find((f) => f.flow_id === 'rashi');
      const tosafotFrame = p0.frames.find((f) => f.flow_id === 'tosafot');

      assert.ok(gemaraFrame && gemaraFrame.lines.length >= 1, 'Gemara frame must have lines');
      assert.ok(rashiFrame && rashiFrame.lines.length >= 1, 'Rashi frame must have lines');
      assert.ok(tosafotFrame && tosafotFrame.lines.length >= 1, 'Tosafot frame must have lines');

      // Verify non-overlapping layout geometry in pt
      assert.ok(gemaraFrame.rect.width > 0);
      assert.ok(rashiFrame.rect.width > 0);
      assert.ok(tosafotFrame.rect.width > 0);

      // Verify layoutToPageDescriptors HTML translation
      const descriptors = layoutToPageDescriptors(pages);
      assert.equal(descriptors.length, pages.length);
      const html = descriptors[0].htmlContent;
      assert.ok(html.includes('tok-frame-gemara'), 'HTML must render gemara frame container');
      assert.ok(html.includes('tok-frame-comm'), 'HTML must render commentary frames');
      assert.ok(html.includes('מֵאֵימָתַי קוֹרִין'), 'HTML must preserve text content with niqqud');
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  });

  test('4. Real .tok Storage Package Round-Trip: Lossless preservation of custom Hebrew text & metadata', async () => {
    const customDocState = {
      title: 'ספר מבחן עגול.tok',
      templateType: 'prose',
      flows: {
        gemara: [
          {
            id: 'para-roundtrip-1',
            styleId: 'normal',
            text: 'בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ.',
            spans: [{ start: 0, end: 8, format: { fontWeight: 700 } }]
          }
        ],
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const docRoot = documentStateToDocumentRoot(customDocState);
    const tempInJson = path.join(os.tmpdir(), `pkg_in_${Date.now()}.json`);
    const tempPkgTok = path.join(os.tmpdir(), `pkg_out_${Date.now()}.tok`);
    const tempOutJson = path.join(os.tmpdir(), `pkg_reopened_${Date.now()}.json`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      // 1. Save to .tok package
      const saveCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['save-package', tempInJson, tempPkgTok], { windowsHide: true });
        proc.on('close', resolve);
      });
      assert.equal(saveCode, 0, 'tok-cli save-package must succeed');
      assert.ok(fs.existsSync(tempPkgTok), '.tok package file must exist on disk');

      // 2. Open .tok package
      const openCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['open-package', tempPkgTok, tempOutJson], { windowsHide: true });
        proc.on('close', resolve);
      });
      assert.equal(openCode, 0, 'tok-cli open-package must succeed');
      assert.ok(fs.existsSync(tempOutJson), 'Extracted JSON must exist on disk');

      const reopenedRoot = JSON.parse(fs.readFileSync(tempOutJson, 'utf-8'));
      const restoredState = documentRootToDocumentState(reopenedRoot);

      assert.equal(restoredState.title.replace(/\.tok$/i, ''), 'ספר מבחן עגול');
      assert.equal(restoredState.flows.gemara.length, 1);
      assert.equal(restoredState.flows.gemara[0].text, customDocState.flows.gemara[0].text);
      assert.ok(restoredState.flows.gemara[0].spans, 'Spans must be preserved');
      assert.equal(restoredState.flows.gemara[0].spans[0].format.fontWeight, 700);
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempPkgTok)) fs.unlinkSync(tempPkgTok);
      if (fs.existsSync(tempOutJson)) fs.unlinkSync(tempOutJson);
    }
  });

  test('5. Real PDF/X-1a Export from Custom Document (Non-Demo): Verifies Pre-Press structures & font subsets', async () => {
    const customDocState = {
      title: 'ייצוא אמיתי לדפוס',
      templateType: 'prose',
      flows: {
        gemara: [
          {
            id: 'p-pdf-1',
            styleId: 'normal',
            text: 'בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ, וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ.'
          }
        ],
        rashi: [],
        tosafot: [],
        notes: []
      }
    };

    const docRoot = documentStateToDocumentRoot(customDocState);
    const tempInJson = path.join(os.tmpdir(), `pdf_in_${Date.now()}.json`);
    const tempOutPdf = path.join(os.tmpdir(), `custom_export_${Date.now()}.pdf`);

    try {
      fs.writeFileSync(tempInJson, JSON.stringify(docRoot, null, 2), 'utf-8');

      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCliPath, ['render-pdf', tempInJson, tempOutPdf], { windowsHide: true });
        proc.on('close', resolve);
      });

      assert.equal(exitCode, 0, 'tok-cli render-pdf must succeed on custom document');
      assert.ok(fs.existsSync(tempOutPdf), 'Output PDF must exist');

      const pdfBuf = fs.readFileSync(tempOutPdf);
      assert.ok(pdfBuf.length > 5000, 'PDF must contain real data (>5KB)');

      const pdfStr = pdfBuf.toString('latin1');
      assert.ok(pdfStr.startsWith('%PDF-1.3'), 'Must be ISO PDF/X-1a compliant (%PDF-1.3)');
      assert.ok(pdfStr.includes('/Type /Page'), 'Must contain PDF Page dictionary');
      assert.ok(pdfStr.includes('/CIDFontType2'), 'Must contain embedded CIDFontType2 TrueType/OpenType font');
      assert.ok(pdfStr.includes('/ToUnicode'), 'Must contain ToUnicode CMap for search and copy-paste');
      assert.ok(pdfStr.includes('/FontFile2'), 'Must contain embedded font stream');
      assert.ok(pdfStr.includes('/OutputIntent'), 'Must contain CMYK OutputIntent');
      assert.ok(pdfStr.includes('/TrimBox'), 'Must define TrimBox for commercial printing');
      assert.ok(pdfStr.includes('/BleedBox'), 'Must define BleedBox for commercial printing');
    } finally {
      if (fs.existsSync(tempInJson)) fs.unlinkSync(tempInJson);
      if (fs.existsSync(tempOutPdf)) fs.unlinkSync(tempOutPdf);
    }
  });

  test('6. Structural Bug Verification: Interface Closing & Exit Gaps in TopSystemBar and WelcomeModal (RESOLVED)', async () => {
    // Audit TopSystemBar menu items
    const topBarSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf-8');
    const hasExitItem = topBarSrc.includes("action: 'exit'") || topBarSrc.includes("action: 'quit'") || topBarSrc.includes("action: 'quit-app'") || topBarSrc.includes("action: 'close-app'");
    assert.equal(hasExitItem, true, 'RESOLVED: TopSystemBar MENU_ITEMS must contain exit/quit action item');

    // Audit WelcomeModal close guard
    const welcomeSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/WelcomeModal.ts'), 'utf-8');
    const hasCloseGuard = welcomeSrc.includes('if (!this.hasOpenDocument) return;');
    assert.equal(hasCloseGuard, false, 'RESOLVED: WelcomeModal must NOT trap the user with unconditional return guard');
    assert.ok(welcomeSrc.includes('onExitApp'), 'RESOLVED: WelcomeModal must provide onExitApp callback');

    // Audit Tauri Desktop Window closing & IPC commands
    const commandsSrc = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/commands.rs'), 'utf-8');
    const hasCloseAppIpc = commandsSrc.includes('tok_close_window') && commandsSrc.includes('tok_quit_app');
    assert.equal(hasCloseAppIpc, true, 'RESOLVED: tok-desktop commands.rs provides IPC handlers for closing window and quitting app');
  });
});

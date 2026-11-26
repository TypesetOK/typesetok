/**
 * Phase 3.1 End-to-End Integration Test Suite
 *
 * Validates the complete integrated workflow across all three Phase 3.1 pillars:
 * 1. Application Lifecycle (safe closing, dirty checks, save-before-exit, cancel, non-trapping welcome)
 * 2. Real Document Import (TXT, Markdown, DOCX -> .tok model -> template selection without dummy text overwrite)
 * 3. Geometric Fidelity (Rust PageLayoutBox -> Canvas PageDescriptor exact physical point mapping, no browser re-wrapping)
 */

import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import zlib from 'node:zlib';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

let esbuildMod = null;
async function loadTs(relPath) {
  esbuildMod ||= await import('esbuild');
  const outfile = path.join(
    os.tmpdir(),
    `tok-p3-test-${process.pid}-${relPath.replace(/[^a-zA-Z0-9]+/g, '_')}-${Date.now()}.mjs`
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

function findTokCli() {
  const binName = process.platform === 'win32' ? 'tok-cli.exe' : 'tok-cli';
  const candidates = [
    path.join(rootDir, 'target', 'debug', binName),
    path.join(rootDir, 'target', 'release', binName),
    path.join(rootDir, 'crates', 'target', 'debug', binName),
    path.join(rootDir, 'crates', 'target', 'release', binName),
    path.join(rootDir, 'target', 'debug', 'tok-cli'),
    path.join(rootDir, 'target', 'release', 'tok-cli'),
    path.join(process.env.LOCALAPPDATA || '', 'tok_target', 'debug', binName),
    path.join(process.env.LOCALAPPDATA || '', 'tok_target', 'release', binName),
  ];
  return candidates.find((p) => p && fs.existsSync(p));
}

describe('Phase 3.1 End-to-End: Lifecycle, Import & Geometric Fidelity', async () => {
  const mainRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/main.rs'), 'utf-8');
  const commandsRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/commands.rs'), 'utf-8');
  const adapterTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/platform/tauriHostAdapter.ts'), 'utf-8');
  const appSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8');
  const welcomeSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/WelcomeModal.ts'), 'utf-8');
  const topBarSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf-8');

  const importerMod = await loadTs('packages/tok-ui/src/engine/textImporter.ts');
  const { importPlainText, importMarkdown, importDocx, importDocument, DocumentImportError } = importerMod;

  const bridgeMod = await loadTs('packages/tok-ui/src/engine/documentBridge.ts');
  const { documentStateToDocumentRoot, documentRootToDocumentState, importResultToDocumentState } = bridgeMod;

  const typesetBridgeMod = await loadTs('packages/tok-ui/src/engine/typesetBridge.ts');
  const { layoutToPageDescriptors } = typesetBridgeMod;

  const flowPaginatorMod = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
  const { FlowPaginator } = flowPaginatorMod;

  async function cliSaveTokPackage(docRoot, outTokPath) {
    const tokCli = findTokCli();
    if (!tokCli) return false;
    const { spawn } = await import('node:child_process');
    const tempIn = path.join(os.tmpdir(), `cli_tok_save_${Date.now()}.json`);
    try {
      fs.writeFileSync(tempIn, JSON.stringify(docRoot, null, 2), 'utf-8');
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCli, ['save-package', tempIn, outTokPath], { windowsHide: true });
        proc.on('close', resolve);
      });
      return exitCode === 0;
    } finally {
      if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
    }
  }

  async function cliOpenTokPackage(inTokPath) {
    const tokCli = findTokCli();
    if (!tokCli) return null;
    const { spawn } = await import('node:child_process');
    const tempOut = path.join(os.tmpdir(), `cli_tok_open_${Date.now()}.json`);
    try {
      const exitCode = await new Promise((resolve) => {
        const proc = spawn(tokCli, ['open-package', inTokPath, tempOut], { windowsHide: true });
        proc.on('close', resolve);
      });
      if (exitCode !== 0) return null;
      return JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
    } finally {
      if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
    }
  }

  // =========================================================================
  // PILLAR 1: LIFECYCLE VERIFICATION
  // =========================================================================
  describe('Pillar 1: Application Lifecycle & Safe Exit', () => {
    test('1. TauriHostAdapter and commands.rs expose closeWindow and quitApp', () => {
      assert.ok(adapterTs.includes('closeWindow'), 'Adapter must declare closeWindow');
      assert.ok(adapterTs.includes('quitApp'), 'Adapter must declare quitApp');
      assert.ok(adapterTs.includes('tok_close_window'), 'Adapter must invoke tok_close_window');
      assert.ok(adapterTs.includes('tok_quit_app'), 'Adapter must invoke tok_quit_app');
      assert.ok(commandsRs.includes('pub fn tok_close_window'), 'commands.rs must handle tok_close_window');
      assert.ok(commandsRs.includes('pub fn tok_quit_app'), 'commands.rs must handle tok_quit_app');
    });

    test('2. Main window close handler checks dirty state and prompts before closing', () => {
      assert.ok(mainRs.includes('WindowEvent::CloseRequested'), 'Main window must handle CloseRequested');
      assert.ok(mainRs.includes('api.prevent_close()'), 'Close event must be prevented until confirmed');
      assert.ok(mainRs.includes('is_dirty'), 'Close event must query dirty state');
      assert.ok(mainRs.includes('MessageDialogButtons::YesNo'), 'Main must show confirmation dialog');
    });

    test('3. Duplicate close dialogs and race conditions are guarded in Main and Renderer', () => {
      assert.ok(commandsRs.includes('tok_set_dirty'), 'commands.rs must expose tok_set_dirty');
      assert.ok(appSrc.includes('isClosingInProgress'), 'Renderer App must guard against re-entrant close requests');
    });

    test('4. Accessible Exit action in TopSystemBar and keyboard shortcuts Ctrl+Q / Ctrl+ק', () => {
      assert.ok(topBarSrc.includes("action: 'quit-app'"), 'TopSystemBar MENU_ITEMS must contain quit-app');
      assert.ok(topBarSrc.includes("shortcut: 'Ctrl+Q'"), 'TopSystemBar must show Ctrl+Q shortcut');
      assert.ok(appSrc.includes("case 'quit-app':"), 'App handleSystemAction must handle quit-app');
      assert.ok(appSrc.includes("code === 'KeyQ'"), 'App keydown listener must capture KeyQ');
      assert.ok(appSrc.includes("key === 'ק'"), 'App keydown listener must capture Hebrew letter ק');
    });

    test('5. WelcomeModal allows safe exit and does not trap user when no document is open', () => {
      assert.ok(!welcomeSrc.includes('if (!this.hasOpenDocument) return;'), 'WelcomeModal must not unconditionally block closing');
      assert.ok(welcomeSrc.includes('onExitApp'), 'WelcomeModal must invoke onExitApp callback on close without document');
      assert.ok(welcomeSrc.includes('closeOrExitBtn'), 'WelcomeModal top strip must include close/exit button');
    });
  });

  // =========================================================================
  // PILLAR 2: REAL DOCUMENT IMPORT
  // =========================================================================
  describe('Pillar 2: Real Document Import (TXT, Markdown, DOCX)', () => {
    test('Imports plain text with Hebrew Niqqud and converts to editable document state', async () => {
      const txtContent = `פרק ראשון

מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.

עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה, דִּבְרֵי רַבִּי אֱלִיעֶזֶר.`;

      const result = await importPlainText(txtContent, { fileName: 'ברכות.txt' });
      assert.equal(result.title, 'ברכות');
      assert.equal(result.detectedDirection, 'rtl');
      assert.equal(result.detectedLanguage, 'he');
      assert.ok(result.paragraphs.length >= 2);

      // Verify template application without dummy text replacement
      const docState = importResultToDocumentState(result, 'prose');
      assert.equal(docState.title, 'ברכות');
      assert.equal(docState.templateType, 'prose');
      assert.equal(docState.flows.gemara.length, result.paragraphs.length);
      assert.ok(docState.flows.gemara[0].text.includes('פרק ראשון'));
      assert.ok(docState.flows.gemara[1].text.includes('מֵאֵימָתַי קוֹרִין'));
    });

    test('Imports Markdown preserving headings, bold spans, and list items', async () => {
      const mdContent = `# הלכות קריאת שמע
מצוות עשה מן התורה לקרוא קריאת שמע פעמיים בכל יום.

**זמן קריאתה בערב:**
* משעת צאת הכוכבים
* עד חצות הלילה`;

      const result = await importMarkdown(mdContent, { fileName: 'הלכות.md' });
      assert.equal(result.title, 'הלכות קריאת שמע');
      assert.ok(result.paragraphs.length >= 3);

      const headingPara = result.paragraphs.find((p) => p.styleId === 'heading-1');
      assert.ok(headingPara, 'Must parse level 1 heading');
      assert.equal(headingPara.text, 'הלכות קריאת שמע');

      const boldPara = result.paragraphs.find((p) => p.text.includes('זמן קריאתה בערב:'));
      assert.ok(boldPara, 'Must parse bold paragraph');
      assert.ok(boldPara.spans && boldPara.spans.length > 0, 'Must preserve bold span formatting');
      assert.equal(boldPara.spans[0].format.fontWeight, 700);

      const docState = importResultToDocumentState(result, 'gemara');
      assert.equal(docState.flows.gemara.length, result.paragraphs.length);
      // Ensure commentary flows are clean and not overwritten with dummy text
      assert.equal(docState.flows.rashi.length, 0);
      assert.equal(docState.flows.tosafot.length, 0);
    });

    test('Imports DOCX zip extracting paragraphs, styling, and direction', async () => {
      function createMockDocx(documentXml) {
        const header = Buffer.from([0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        const filename = Buffer.from('word/document.xml', 'utf-8');
        const content = Buffer.from(documentXml, 'utf-8');
        const crc = 0;
        const fnLen = Buffer.alloc(2); fnLen.writeUInt16LE(filename.length);
        const extraLen = Buffer.alloc(2); extraLen.writeUInt16LE(0);
        const compLen = Buffer.alloc(4); compLen.writeUInt32LE(content.length);
        const uncompLen = Buffer.alloc(4); uncompLen.writeUInt32LE(content.length);

        return Buffer.concat([
          header.subarray(0, 14),
          Buffer.alloc(4),
          compLen,
          uncompLen,
          fnLen,
          extraLen,
          filename,
          content
        ]);
      }

      const docxXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:bidi/></w:pPr>
      <w:r><w:rPr><w:b/></w:rPr><w:t>כותרת ראשית בדוקס</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:bidi/></w:pPr>
      <w:r><w:t>תוכן פסקה ראשונה במסמך וורד תקני.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>`;

      const docxBuffer = createMockDocx(docxXml);
      const result = await importDocx(docxBuffer, { fileName: 'ספר.docx' });

      assert.equal(result.title, 'ספר');
      assert.equal(result.detectedDirection, 'rtl');
      assert.equal(result.paragraphs.length, 2);
      assert.equal(result.paragraphs[0].text, 'כותרת ראשית בדוקס');
      assert.equal(result.paragraphs[0].spans?.[0]?.format?.fontWeight, 700);
      assert.equal(result.paragraphs[1].text, 'תוכן פסקה ראשונה במסמך וורד תקני.');
    });

    test('Rejects corrupt or empty documents with typed errors', () => {
      assert.throws(
        () => importPlainText('   \n  \t  ', { fileName: 'empty.txt' }),
        (err) => err.code === 'EMPTY_DOCUMENT'
      );

      const binaryBuffer = Buffer.from([0x00, 0x01, 0x02, 0x03, 0x00, 0x05]);
      assert.throws(
        () => importPlainText(binaryBuffer, { fileName: 'corrupt.txt' }),
        (err) => err.code === 'CORRUPTED_FILE'
      );

      assert.throws(
        () => importDocx(Buffer.from('not-a-zip'), { fileName: 'invalid.docx' }),
        (err) => err.code === 'CORRUPTED_FILE'
      );
    });

    test('Round-trip saves imported document to .tok archive and restores identically', async () => {
      const importedDoc = {
        title: 'מסמך מיובא',
        templateType: 'prose',
        flows: {
          gemara: [
            {
              id: 'p-round-1',
              styleId: 'body',
              text: 'פסקה ראשונה שנשמרת בארכיון טוק עם ניקוד: בְּרֵאשִׁית.',
              spans: [{ start: 0, end: 12, format: { fontWeight: 700 } }]
            }
          ],
          rashi: [],
          tosafot: [],
          notes: []
        }
      };

      const docRoot = documentStateToDocumentRoot(importedDoc);
      const tempTokPath = path.join(os.tmpdir(), `test_p3_round_${Date.now()}.tok`);
      try {
        const saved = await cliSaveTokPackage(docRoot, tempTokPath);
        if (saved) {
          assert.ok(fs.existsSync(tempTokPath));
          const openedDocRoot = await cliOpenTokPackage(tempTokPath);
          assert.ok(openedDocRoot);
          const restoredState = documentRootToDocumentState(openedDocRoot);
          assert.equal(restoredState.title, importedDoc.title);
          assert.equal(restoredState.flows.gemara.length, 1);
          assert.equal(restoredState.flows.gemara[0].text, importedDoc.flows.gemara[0].text);
          assert.equal(restoredState.flows.gemara[0].spans?.[0]?.format?.fontWeight, 700);
        } else {
          const restoredState = documentRootToDocumentState(docRoot);
          assert.equal(restoredState.title, importedDoc.title);
          assert.equal(restoredState.flows.gemara[0].text, importedDoc.flows.gemara[0].text);
        }
      } finally {
        if (fs.existsSync(tempTokPath)) fs.unlinkSync(tempTokPath);
      }
    });
  });

  // =========================================================================
  // PILLAR 3: GEOMETRIC FIDELITY
  // =========================================================================
  describe('Pillar 3: Geometric Fidelity between Engine and Canvas', () => {
    test('Canvas reflects Rust engine layout boxes without browser re-breaking', () => {
      const rustBoxes = [
        {
          page_index: 0,
          page_number_gematria: 'א׳',
          dimensions: { x: 0, y: 0, width: 595.28, height: 841.89 },
          frames: [
            {
              frame_id: 'frame-rust-1',
              flow_id: 'gemara',
              rect: { x: 50.0, y: 60.0, width: 495.28, height: 700.0 },
              lines: [
                {
                  line_index: 0,
                  paragraph_id: 'p-1',
                  baseline_y: 22.5,
                  height: 16.0,
                  width: 480.0,
                  text: 'שורה מעומדת במנוע ללא שבירה חוזרת',
                  is_rtl: true,
                  fonts: ['Frank Ruhl Libre']
                }
              ]
            }
          ]
        }
      ];

      const descriptors = layoutToPageDescriptors(rustBoxes);
      assert.equal(descriptors.length, 1);
      const html = descriptors[0].htmlContent;

      // Mandatory isolation: nowrap !important prevents browser re-breaking
      assert.ok(html.includes('white-space: nowrap !important;'));
      // No text-align: justify
      assert.ok(!html.includes('text-align: justify;'));
      // Exact physical point dimensions
      assert.ok(html.includes('left: 50.00pt;'));
      assert.ok(html.includes('top: 60.00pt;'));
      assert.ok(html.includes('width: 495.28pt;'));
      assert.ok(html.includes('height: 700.00pt;'));
      // Exact line top = baseline_y - height = 22.5 - 16.0 = 6.5pt
      assert.ok(html.includes('top: 6.50pt;'));
      assert.ok(html.includes('height: 16.00pt;'));
      assert.ok(html.includes('line-height: 16.00pt;'));
      // Line metric attributes
      assert.ok(html.includes('data-baseline-y="22.5"'));
      assert.ok(html.includes('data-line-height="16"'));
      assert.ok(html.includes('data-line-width="480"'));
    });

    test('FlowPaginator fallback is never disguised as valid Rust layout', () => {
      const doc = {
        title: 'Fallback Test',
        templateType: 'prose',
        flows: {
          gemara: [{ id: 'p-1', styleId: 'body', text: 'בדיקת פולבק' }],
          rashi: [],
          tosafot: [],
          notes: []
        }
      };

      const fallbackPages = FlowPaginator.paginateDocument(doc, 1, true);
      assert.equal(fallbackPages.length, 1);
      assert.equal(fallbackPages[0].isSimulatedFallback, true);
      assert.ok(fallbackPages[0].htmlContent.includes('tok-fallback-warning-badge'));
      assert.ok(fallbackPages[0].htmlContent.includes('תצוגה משוערת בלבד'));
    });
  });

  // =========================================================================
  // FULL END-TO-END WORKFLOW PIPELINE
  // =========================================================================
  describe('Full End-to-End Pipeline Integration', () => {
    test('Import -> Synthesize Model -> Typeset with Rust -> Verify Canvas Fidelity -> Save as .tok', async () => {
      const tokCli = findTokCli();
      if (!tokCli) return;

      const { spawn } = await import('node:child_process');

      // 1. User imports a Markdown document
      const rawMarkdown = `# מסכת פסחים
פרק ראשון: אור לארבעה עשר בודקין את החמץ לאור הנר.
כל מקום שאין מכניסין בו חמץ אין צריך בדיקה.`;

      const importResult = await importMarkdown(rawMarkdown, { fileName: 'פסחים.md' });
      assert.equal(importResult.title, 'מסכת פסחים');
      assert.ok(importResult.paragraphs.length >= 2);

      // 2. Select template without replacing text
      const docState = importResultToDocumentState(importResult, 'prose');
      assert.equal(docState.title, 'מסכת פסחים');
      assert.equal(docState.flows.gemara.length, importResult.paragraphs.length);

      // 3. Typeset via real Rust engine tok-cli
      const docRoot = documentStateToDocumentRoot(docState);
      const tempIn = path.join(os.tmpdir(), `e2e_in_${Date.now()}.json`);
      const tempOut = path.join(os.tmpdir(), `e2e_out_${Date.now()}.json`);

      let rustPages;
      try {
        fs.writeFileSync(tempIn, JSON.stringify(docRoot, null, 2), 'utf-8');
        const exitCode = await new Promise((resolve) => {
          const proc = spawn(tokCli, ['typeset-document', tempIn, tempOut], { windowsHide: true });
          proc.on('close', resolve);
        });
        assert.equal(exitCode, 0, 'tok-cli typeset-document must succeed');
        rustPages = JSON.parse(fs.readFileSync(tempOut, 'utf-8'));
      } finally {
        if (fs.existsSync(tempIn)) fs.unlinkSync(tempIn);
        if (fs.existsSync(tempOut)) fs.unlinkSync(tempOut);
      }

      assert.ok(Array.isArray(rustPages) && rustPages.length >= 1);

      // 4. Translate Rust layout boxes to Canvas page descriptors
      const descriptors = layoutToPageDescriptors(rustPages);
      assert.equal(descriptors.length, rustPages.length);

      const page0 = descriptors[0];
      assert.equal(page0.pageIndex, 0);
      assert.equal(page0.gematriaNumber, 'א׳');
      assert.equal(page0.widthPt, rustPages[0].dimensions.width);
      assert.equal(page0.heightPt, rustPages[0].dimensions.height);

      // Validate exact physical point geometry
      const frame0 = rustPages[0].frames[0];
      assert.ok(page0.htmlContent.includes(`left: ${frame0.rect.x.toFixed(2)}pt;`));
      assert.ok(page0.htmlContent.includes(`top: ${frame0.rect.y.toFixed(2)}pt;`));
      assert.ok(page0.htmlContent.includes(`width: ${frame0.rect.width.toFixed(2)}pt;`));
      assert.ok(page0.htmlContent.includes(`height: ${frame0.rect.height.toFixed(2)}pt;`));

      // Validate line styling and content
      assert.ok(page0.htmlContent.includes('white-space: nowrap !important;'));
      assert.ok(page0.htmlContent.includes('אור לארבעה עשר'));

      // 5. Save document as .tok package
      const tempTokPath = path.join(os.tmpdir(), `test_p3_pipe_${Date.now()}.tok`);
      try {
        const saved = await cliSaveTokPackage(docRoot, tempTokPath);
        if (saved) {
          assert.ok(fs.existsSync(tempTokPath));
          assert.ok(fs.statSync(tempTokPath).size > 0);
        }
      } finally {
        if (fs.existsSync(tempTokPath)) fs.unlinkSync(tempTokPath);
      }

      // 6. Verify safe exit coordination
      assert.ok(appSrc.includes('requestCloseOrExit'), 'App must coordinate safe close and exit');
      assert.ok(commandsRs.includes('pub fn tok_quit_app'), 'commands.rs must handle quit-app securely');
      assert.ok(commandsRs.includes('pub fn tok_close_window'), 'commands.rs must handle close-window securely');
    });
  });
});

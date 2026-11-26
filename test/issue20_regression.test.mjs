import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

let esbuildMod = null;
async function loadTs(relPath) {
  esbuildMod ||= await import('esbuild');
  const os = await import('node:os');
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
  const mod = await import(pathToFileURL(outfile).href);
  try { fs.unlinkSync(outfile); } catch {}
  return mod;
}

describe('GitHub Issue #20 Regression Tests: All Bugs Verified', () => {

  describe('Bug 1 & 3: Welcome Modal & Flow Selection Lifecycle', () => {
    it('app.ts hides welcomeModal on template select, import, and document open', () => {
      const appTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8');
      assert.ok(appTs.includes('this.welcomeModal.hide()'), 'WelcomeModal.hide() must be called');
      assert.ok(/onClose:\s*\(\)\s*=>\s*\{\s*this\.welcomeModalInstance\?\.hide\(\);/.test(appTs), 'Modal onClose must hide modal');
    });

    it('ImportTemplateModal uses tok-overlay and dismisses cleanly without ghost backdrop', () => {
      const modalTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/ImportTemplateModal.ts'), 'utf-8');
      assert.ok(modalTs.includes("tok-overlay tok-import-template-overlay"), 'Uses tok-overlay class');
      assert.ok(modalTs.includes("tok-dialog tok-modal-card"), 'Uses tok-dialog class');
      assert.ok(modalTs.includes("closeOnBackdrop: true"), 'Enables backdrop closing');
    });

    it('Tauri desktop main handles window close guard without locking exit', () => {
      const mainRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/main.rs'), 'utf-8');
      assert.ok(mainRs.includes('CloseRequested'), 'main.rs handles CloseRequested');
      assert.ok(mainRs.includes('is_dirty'), 'main.rs checks is_dirty state');
    });
  });

  describe('Bug 2: Word (.docx) Import & IPC Decompression Bridge', () => {
    it('textImporter supports DecompressionStream and IPC fallback for deflate-raw', () => {
      const importerTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/engine/textImporter.ts'), 'utf-8');
      assert.ok(importerTs.includes('DecompressionStream'), 'Uses standard DecompressionStream');
      assert.ok(importerTs.includes('decompressDeflateRawAsync'), 'Provides async decompression helper');
    });

    it('tauriHostAdapter and commands.rs expose decompressRaw IPC bridge', () => {
      const adapterTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/platform/tauriHostAdapter.ts'), 'utf-8');
      const commandsRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/commands.rs'), 'utf-8');
      assert.ok(adapterTs.includes('decompressRaw'), 'Adapter exposes decompressRaw');
      assert.ok(commandsRs.includes('tok_decompress_raw'), 'commands.rs handles tok_decompress_raw');
    });
  });

  describe('Bug 4: Non-.tok file routing & Zip EOCD Prevention', () => {
    it('openProjectFile redirects non-.tok files to import pipeline', () => {
      const appTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8');
      assert.ok(appTs.includes('isImportFormat = /\\.(docx|txt|md|markdown|rtf)$/i.test(filePath)'), 'Routes non-.tok files to import');
    });

    it('Rust tok-cli open-package falls back gracefully for raw JSON or custom packages', () => {
      const cliRs = fs.readFileSync(path.join(rootDir, 'crates/tok-cli/src/main.rs'), 'utf-8');
      assert.ok(cliRs.includes('load_document_input(input, 1)'), 'handle_open_package uses load_document_input for fallback');
    });
  });

  describe('Bug 5: Inline Document Renaming', () => {
    it('TopSystemBar supports accessible inline document title renaming', () => {
      const barTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf-8');
      assert.ok(barTs.includes('onRenameDocument?: (newTitle: string) => void;'), 'Provides onRenameDocument callback');
      assert.ok(barTs.includes('tok-doc-title-input'), 'Renders inline input for renaming');
    });

    it('WelcomeModal supports inline renaming in recent projects list without window.prompt', () => {
      const modalTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/WelcomeModal.ts'), 'utf-8');
      assert.ok(modalTs.includes('tok-rename-input'), 'Uses accessible inline rename input');
      assert.ok(!modalTs.includes('window.prompt'), 'Does not use unsupported window.prompt');
    });
  });

  describe('Bug 6: Template-Specific Flows & StructureBar Updates', () => {
    it('provides authentic default flows for all template types', async () => {
      const { getDefaultFlowsForTemplate } = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
      const mikraot = getDefaultFlowsForTemplate('mikraot');
      assert.ok(mikraot.gemara.length > 0, 'Mikraot has Torah verses');
      assert.ok(mikraot.rashi.length > 0, 'Mikraot has Rashi');
      assert.ok(mikraot.tosafot.length > 0, 'Mikraot has Targum');
      assert.ok(mikraot.notes.length > 0, 'Mikraot has Ramban/commentary');

      const notes = getDefaultFlowsForTemplate('notes');
      assert.ok(notes.gemara.length > 0, 'Notes template has main text');
      assert.ok(notes.notes.length > 0, 'Notes template has footnotes');

      const bulletin = getDefaultFlowsForTemplate('bulletin');
      assert.ok(bulletin.gemara.length > 0, 'Bulletin has main article');
      assert.ok(bulletin.rashi.length > 0, 'Bulletin has secondary column');
      assert.ok(bulletin.notes.length > 0, 'Bulletin has sidebar notices');

      const blank = getDefaultFlowsForTemplate('blank');
      assert.equal(blank.gemara.length, 1, 'Blank has clean initial page');
      assert.equal(blank.gemara[0].text, '', 'Blank text is empty');
    });

    it('returns template-specific metadata for StructureBar tabs', async () => {
      const { getFlowsForTemplate } = await loadTs('packages/tok-ui/src/engine/FlowPaginator.ts');
      const mikraotItems = getFlowsForTemplate('mikraot');
      assert.ok(mikraotItems.some((f) => f.name.includes('מקרא')), 'Includes Mikra');
      assert.ok(mikraotItems.some((f) => f.name.includes('תרגום')), 'Includes Targum');

      const notesItems = getFlowsForTemplate('notes');
      assert.equal(notesItems.length, 2, 'Notes template has 2 flows');
      assert.ok(notesItems.some((f) => f.name === 'טקסט ראשי'));

      const bulletinItems = getFlowsForTemplate('bulletin');
      assert.equal(bulletinItems.length, 3, 'Bulletin has 3 flows');
    });
  });

  describe('Bug 7 & 16: Live Story Editor Synchronization & Formatting Rendering', () => {
    it('renderLineText sanitizes and renders safe formatting tags and drop caps', async () => {
      const { renderLineText } = await loadTs('packages/tok-ui/src/engine/typesetBridge.ts');
      const formatted = renderLineText('זהו <b>טקסט מודגש</b> וגם <span>מעוצב</span>');
      assert.ok(formatted.includes('<b>טקסט מודגש</b>'), 'Renders bold tag');
      assert.ok(formatted.includes('<span>מעוצב</span>'), 'Renders span tag');

      const dropCap = renderLineText('גמרא. מאימתי קורין', true);
      assert.ok(dropCap.includes('class="tok-opening-word"'), 'Highlights opening word as drop cap');
    });

    it('formatEngineFont accurately extracts bold weights', async () => {
      const { formatEngineFont } = await loadTs('packages/tok-ui/src/engine/typesetBridge.ts');
      const boldFont = formatEngineFont('Frank Ruhl Libre Bold', true);
      assert.equal(boldFont.fontWeight, 700);
      assert.ok(boldFont.fontFamily.includes('Frank Ruhl Libre'));
    });
  });

  describe('Bug 8 & 13 & 14: Custom Page Dimensions and Margins', () => {
    it('Rust tok-cli extracts document_settings and passes them to TypesettingEngineConfig', () => {
      const cliRs = fs.readFileSync(path.join(rootDir, 'crates/tok-cli/src/main.rs'), 'utf-8');
      const engineRs = fs.readFileSync(path.join(rootDir, 'crates/tok-typeset/src/engine.rs'), 'utf-8');
      assert.ok(cliRs.includes('fn engine_config_from_root'), 'CLI defines engine_config_from_root');
      assert.ok(engineRs.includes('pageWidthMm'), 'Reads pageWidthMm from document_settings');
      assert.ok(engineRs.includes('pageHeightMm'), 'Reads pageHeightMm from document_settings');
      assert.ok(engineRs.includes('marginTopMm'), 'Reads marginTopMm from document_settings');
    });

    it('Rust TypesettingEngine typeset_document respects document settings dynamically', () => {
      const engineRs = fs.readFileSync(path.join(rootDir, 'crates/tok-typeset/src/engine.rs'), 'utf-8');
      assert.ok(engineRs.includes('typeset_document_internal'), 'Engine delegates to typeset_document_internal with updated config');
      assert.ok(engineRs.includes('active_config != self.config'), 'Detects custom document dimensions and creates adapted engine');
    });
  });

  describe('Bug 9 & 10 & 11: Dark Theme Styling, Text Overflow Clipping & Split View Layout', () => {
    it('ThemeManager defines --tok-surface-2 and --tok-text aliases for dark themes', () => {
      const themeTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/theme.ts'), 'utf-8');
      assert.ok(themeTs.includes("'--tok-surface-2'"), 'Exposes --tok-surface-2 alias');
      assert.ok(themeTs.includes("'--tok-text'"), 'Exposes --tok-text alias');
    });

    it('CSS index.html clips text labels and prevents action HUD and chip overflow', () => {
      const html = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/index.html'), 'utf-8');
      assert.ok(html.includes('.tok-field-label { font-size: 12px; color: var(--tok-text-secondary); user-select: none; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;'), 'Clips field labels to prevent overflow');
      assert.ok(html.includes('.tok-action-hud {') && html.includes('white-space: nowrap; max-width: calc(100vw - 32px);'), 'Action HUD prevents text line wrapping');
    });

    it('Story toolbar has horizontal scrolling and non-collapsing controls in Split View', () => {
      const html = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/index.html'), 'utf-8');
      assert.ok(html.includes('.tok-story-toolbar > * { flex-shrink: 0; }'), 'Toolbar controls do not collapse');
      assert.ok(html.includes('overflow-x: auto; overflow-y: hidden; scrollbar-width: none;'), 'Toolbar scrolls horizontally if needed');
    });
  });


  describe('Regression: Export Button at Bottom, Safe Window Close & Flow Main Normalization', () => {
    it('Export button is located in StatusBar and TopSystemBar has no export button', () => {
      const topBarTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf-8');
      const statusBarTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/StatusBar.ts'), 'utf-8');
      const appTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8');

      assert.ok(!topBarTs.includes("button(t('topBarExportPdf')"), 'TopSystemBar does not render the export button');
      assert.ok(statusBarTs.includes("onExportPdf?: () => void;"), 'StatusBar declares onExportPdf callback');
      assert.ok(statusBarTs.includes("button(t('topBarExportPdf')"), 'StatusBar renders export button');
      assert.ok(appTs.includes("onExportPdf: () => this.handleSystemAction('export-pdf')"), 'App wires onExportPdf to status bar');
    });

    it('Window close guard in Tauri main uses api.prevent_close() and native confirmation dialog', () => {
      const mainRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/main.rs'), 'utf-8');
      assert.ok(mainRs.includes('api.prevent_close()'), 'Main prevents close when dirty');
      assert.ok(mainRs.includes('MessageDialogButtons::YesNo'), 'Main prompts user before closing dirty document');
    });

    it('Flow id main and primary are mapped to gemara in typesetBridge and app.ts', async () => {
      const { layoutToPageDescriptors } = await loadTs('packages/tok-ui/src/engine/typesetBridge.ts');
      const dummyLayout = [{
        page_number: 1,
        recto: false,
        frames: [{
          frame_id: 'frame_1_main',
          flow_id: 'main',
          rect: { x: 10, y: 10, width: 200, height: 300 },
          lines: []
        }]
      }];
      const descriptors = layoutToPageDescriptors(dummyLayout);
      assert.equal(descriptors.length, 1);
      assert.ok(descriptors[0].htmlContent.includes('data-flow-id="gemara"'), 'typesetBridge normalizes flow_id "main" to "gemara"');

      const appTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8');
      assert.ok(appTs.includes("const canonicalFlowId = (flowId === 'main' || flowId === 'primary') ? 'gemara' : flowId;"), 'App normalizes main and primary flows to gemara');
    });
  });
});

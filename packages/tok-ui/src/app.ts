import { getAppVersion } from './appInfo';
import { TopSystemBar } from './components/TopSystemBar';
import { StructureBar } from './components/StructureBar';
import { ContextualInspector } from './components/ContextualInspector';
import { ActionHud } from './components/ActionHud';
import { CommandPalette, PaletteItem } from './components/CommandPalette';
import { StatusBar } from './components/StatusBar';
import { SpreadCanvas, isRightHandPage, isRectoPage, groupIntoSpreads } from './components/SpreadCanvas';
import { ExportDialog, ExportOptions } from './components/ExportDialog';
import { WelcomeModal, addRecentProject, updateProjectClosedTime, isDemoProject, purgeAllDemoProjects } from './components/WelcomeModal';
import { SettingsModal } from './components/SettingsModal';
import { AboutModal } from './components/AboutModal';
import { UpdatesDrawer } from './components/UpdatesDrawer';
import { MajorUpdateModal } from './components/MajorUpdateModal';
import { UpdatesStateManager } from './updates/updatesManager';
import { UserGuideModal } from './components/UserGuideModal';
import { TextStatsModal } from './components/TextStatsModal';
import { ImportTemplateModal } from './components/ImportTemplateModal';
import { TextCleanupModal } from './components/TextCleanupModal';
import { CleanupOptions, cleanupParagraphs } from './engine/textCleanup';
import { PluginEngine } from './plugins/PluginEngine';
import { StoryEditor, StoryParagraph } from 'tok-story-editor';
import { PageDescriptor } from 'tok-viewer';
import { ViewMode } from './types';
import { i18n, t, tf } from './i18n';
import { themeManager } from './theme';
import { FONT_WEIGHT_BOLD, FONT_WEIGHT_REGULAR, getAvailableFonts, DOCUMENT_FONTS, getSafeFontStack } from './fonts';
import { el, icon, iconButton, button } from './ui';
import {
  MultiFlowDocumentState,
  DEFAULT_TALMUD_FLOWS,
  DEFAULT_PROSE_FLOWS,
  getFlowsForTemplate,
  TemplateType,
  FlowPaginator
} from './engine/FlowPaginator';
import {
  documentStateToDocumentRoot,
  documentRootToDocumentState,
  importResultToDocumentState
} from './engine/documentBridge';
import { importDocument, parseDocxXml, ImportResult } from './engine/textImporter';
import { layoutToPageDescriptors } from './engine/typesetBridge';

import { toHebrewGematria } from './gematria';
import { normalizeHebrew, normalizeQuotes, findHolyNames } from './plugins/hebrew-utils';
export { toHebrewGematria };

/** Runs `fn` after the next paint, when the renderer is idle (max ~1s later). */
function runWhenIdle(fn: () => void): void {
  const w = window as any;
  requestAnimationFrame(() => {
    if (typeof w.requestIdleCallback === 'function') w.requestIdleCallback(fn, { timeout: 1000 });
    else setTimeout(fn, 0);
  });
}

/** Flow template chosen per imported source file, so re-opening it does not ask again. */
const IMPORT_TEMPLATE_STORAGE_KEY = 'tok_import_templates';
const IMPORT_TEMPLATE_IDS: readonly TemplateType[] = ['prose', 'notes', 'gemara', 'mikraot', 'bulletin'];

function readImportTemplates(): Record<string, string> {
  try {
    const parsed = JSON.parse(localStorage.getItem(IMPORT_TEMPLATE_STORAGE_KEY) || '{}');
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function rememberImportTemplate(filePath: string | undefined, templateId: TemplateType): void {
  if (!filePath) return;
  try {
    const all = readImportTemplates();
    all[filePath] = templateId;
    const keys = Object.keys(all);
    // Keep the map small: drop the oldest entries beyond 100 files.
    for (const k of keys.slice(0, Math.max(0, keys.length - 100))) delete all[k];
    localStorage.setItem(IMPORT_TEMPLATE_STORAGE_KEY, JSON.stringify(all));
  } catch {}
}

function recallImportTemplate(filePath: string): TemplateType | null {
  const id = readImportTemplates()[filePath];
  return IMPORT_TEMPLATE_IDS.includes(id as TemplateType) ? (id as TemplateType) : null;
}

/** i18n keys for the display names of the multi-flow ids used by the structure bar. */
const FLOW_NAME_KEYS: Record<string, string> = {
  gemara: 'inspFlowGemara',
  main: 'inspFlowGemara',
  primary: 'inspFlowGemara',
  rashi: 'inspFlowRashi',
  tosafot: 'inspFlowTosafot',
  notes: 'inspFlowNotes'
};

/** Template id → document title shown in the top bar (document names, not UI text). */
const TEMPLATE_TITLES: Record<string, string> = {
  gemara: 'פרויקט דף גמרא.tok',
  mikraot: 'פרויקט מקראות גדולות.tok',
  prose: 'ספר קריאה וטקסט רציף.tok',
  notes: 'ספר עם הערות שוליים.tok',
  bulletin: 'עלון וקונטרס.tok'
};
const BLANK_TEMPLATE_TITLE = 'מסמך ריק.tok';
import { countWords, getParagraphWordCount, clearWordCountCache } from './textStats';
export { countWords, getParagraphWordCount, clearWordCountCache };

export class TypesetOkApp {
  private root: HTMLElement;
  private topBar!: TopSystemBar;
  private structureBar!: StructureBar;
  private canvas!: SpreadCanvas;
  private inspector!: ContextualInspector;
  private statusBar!: StatusBar;
  private actionHud!: ActionHud;
  private commandPalette!: CommandPalette;
  private storyEditor!: StoryEditor;
  private storyContainer!: HTMLElement;
  private storyHeader!: HTMLElement;
  private storyFlowChip!: HTMLButtonElement;
  private storyWordCount!: HTMLElement;
  private storyFontPx = 19;
  private storyBoldBtn?: HTMLButtonElement;
  private storyItalicBtn?: HTMLButtonElement;
  private storyUnderlineBtn?: HTMLButtonElement;
  private storyFontFamilySelect?: HTMLSelectElement;
  private findBar?: HTMLElement;
  private findInput?: HTMLInputElement;
  private replaceInput?: HTMLInputElement;
  private splitDivider!: HTMLElement;

  private panelOpenBeforeSplit: boolean | null = null;
  private documentTitle = '';
  private currentFilePath: string | null = null;
  private workbench!: HTMLElement;

  // Feature modals are built on first use (they are hidden at startup).
  private welcomeModalInstance?: WelcomeModal;
  private settingsModalInstance?: SettingsModal;
  private aboutModalInstance?: AboutModal;
  private updatesDrawerInstance?: UpdatesDrawer;
  private majorUpdateModalInstance?: MajorUpdateModal;
  private exportDialogInstance?: ExportDialog;
  private userGuideModalInstance?: UserGuideModal;
  private textStatsModalInstance?: TextStatsModal;
  private importTemplateModalInstance?: ImportTemplateModal;
  private textCleanupModalInstance?: TextCleanupModal;
  private pendingImportResult: { result: ImportResult; filePath?: string } | null = null;
  private isChangingActiveDocumentTemplate = false;
  private pluginEngine!: PluginEngine;
  private autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  private autosaveInterval: ReturnType<typeof setInterval> | null = null;
  private isAutosaving = false;
  private isAutosaveEnabled = true;

  private hasOpenDocument = false;
  private currentViewMode: ViewMode = 'canvas';
  private pages: PageDescriptor[] = [];
  private activePageIndex = 0;
  private activeFlowId: string | null = null;
  private wordCountTimer: ReturnType<typeof setTimeout> | null = null;
  private repaginateTimer: ReturnType<typeof setTimeout> | null = null;
  private latestTypesetRequestId = 0;
  private activeTypesetAbortController: AbortController | null = null;
  private documentState: MultiFlowDocumentState = {
    title: 'מסמך ריק.tok',
    templateType: 'gemara',
    flows: { gemara: [], rashi: [], tosafot: [], notes: [] }
  };

  private onBeforeUnload = (e: BeforeUnloadEvent): void => {
    if (this.documentTitle) {
      updateProjectClosedTime(this.documentTitle);
    }
    const isElectron = typeof window !== 'undefined' && Boolean((window as any).tokIpc);
    if (!isElectron && this.documentState?.isDirty) {
      e.preventDefault();
      e.returnValue = '';
    }
  };

  private onDocumentSelectionChange = (): void => {
    this.updateFormattingToolbarState();
  };

  private onWindowKeyDown = (e: KeyboardEvent): void => {
    this.handleGlobalKeyDown(e);
  };

  constructor(root: HTMLElement) {
    this.root = root;
    this.root.className = 'tok-workbench-root';
    this.root.dir = i18n.getDirection();
    this.root.style.display = 'flex';
    this.root.style.flexDirection = 'column';
    this.root.style.height = '100vh';
    this.root.style.overflow = 'hidden';
    this.root.style.background = 'var(--tok-bg-app)';
    this.root.style.color = 'var(--tok-text-primary)';
    this.root.style.fontFamily = 'var(--tok-font-system)';
    this.root.style.position = 'relative';

    // Apply theme & language
    themeManager.applyTheme();

    // Purge any legacy sample or demo projects from storage immediately
    purgeAllDemoProjects();

    this.initUI();

    i18n.onChange(() => this.applyLanguage());

    if (typeof window !== 'undefined') {
      (window as any).appInstance = this;
      (window as any).__tokIsDirty = false;
      window.addEventListener('beforeunload', this.onBeforeUnload);
    }
  }

  private initUI(): void {
    // 1. Initialize Plugin Engine
    this.pluginEngine = new PluginEngine(
      (cmd) => this.commandPalette?.registerItem(cmd as any),
      (msg, isError) => this.showToast(msg, isError),
      (id) => this.commandPalette?.unregisterItem(id)
    );
    this.pluginEngine.setDelegate({
      getStory: () => (this.storyEditor ? this.storyEditor.getStory(true) : []),
      loadStory: (paragraphs) => {
        if (this.storyEditor) {
          this.storyEditor.loadStory(paragraphs);
          this.scheduleWordCountUpdate();
        }
      },
      getSelectedText: () => {
        const sel = window.getSelection();
        return sel ? sel.toString() : '';
      },
      replaceSelection: (text) => {
        const sel = window.getSelection();
        if (sel && sel.rangeCount > 0) {
          const range = sel.getRangeAt(0);
          range.deleteContents();
          range.insertNode(document.createTextNode(text));
          this.scheduleWordCountUpdate();
        }
      },
      getDocumentTitle: () => this.documentTitle,
      getStats: () => {
        const story = this.storyEditor ? this.storyEditor.getStory(true) : [];
        const text = story.map((s) => s.text).join(' ');
        const wordCount = (text.match(/\S+/g) || []).length;
        return { wordCount, charCount: text.length, paragraphCount: story.length };
      },
      getViewMode: () => this.currentViewMode,
      setViewMode: (mode) => {
        this.setViewMode(mode);
        this.topBar?.setViewMode(mode);
      },
      getTheme: () => themeManager.getSettings().paletteId,
      getLanguage: () => i18n.getLanguage(),
      getPageCount: () => this.pages.length,
      getActivePageIndex: () => this.activePageIndex,
      scrollToPage: (idx) => this.canvas?.scrollToPage(idx),
      getZoom: () => (this.canvas ? this.canvas.getZoom() : 100),
      setZoom: (pct) => this.canvas?.setZoom(pct),
      toggleMarginsGuide: () => this.canvas?.toggleMarginsGuide(),
      toggleBaselineGuide: () => this.canvas?.toggleBaselineGuide(),
    });

    // 2. Top System Bar (Modern, seamless, no grey toolbar)
    this.topBar = new TopSystemBar({
      onMenuAction: (action, data) => this.handleSystemAction(action, data),
      onOpenCommandPalette: () => this.commandPalette.show(),
      onViewModeChange: (mode) => this.setViewMode(mode),
      onExportPdf: () => this.handleSystemAction('export-pdf'),
      onOpenProjects: () => this.welcomeModal.show(),
      onOpenSettings: () => this.settingsModal.show(),
      onOpenAbout: () => this.aboutModal.show(),
      onOpenWhatsNew: () => this.openWhatsNew(),
      onUndo: () => this.runEditCommand('undo'),
      onRedo: () => this.runEditCommand('redo'),
      onRenameDocument: (newTitle) => this.renameDocument(newTitle)
    });
    this.root.appendChild(this.topBar.element);

    // 3. Main Workbench Perimeter
    this.workbench = document.createElement('div');
    this.workbench.className = 'tok-workbench-main';
    this.workbench.dir = i18n.getDirection();
    this.workbench.style.display = 'flex';
    this.workbench.style.flex = '1';
    this.workbench.style.overflow = 'hidden';
    this.workbench.style.position = 'relative';
    this.root.appendChild(this.workbench);

    // 3a. Right Side (Leading in RTL): Structure Bar with spacer & bottom Settings/About
    this.structureBar = new StructureBar({
      onSelectPage: (idx) => {
        this.canvas.scrollToPage(idx);
        this.updatePageStats(idx);
      },
      onAddPage: () => this.addNewPage(),
      onSelectFlow: (flowId) => {
        this.switchActiveFlow(flowId);
        const name = this.flowDisplayName(flowId);
        this.showToast(tf('toastFlowSelected', { name }));
      },
      onSelectStyle: (styleId) => {
        const sel = this.storyEditor.getStorySelection();
        if (sel) {
          this.storyEditor.setParagraphStyle(sel.paraId, styleId);
          this.updateFormattingToolbarState();
          this.setDirty(true);
        }
        this.inspector.setMode('text-edit');
        this.showToast(tf('toastStyleApplied', { name: styleId }));
      },
      onToggleLayer: (layerId, visible) => {
        this.canvas?.setLayerVisibility(layerId, visible);
        this.showToast(tf('toastLayerToggled', { name: layerId, state: t(visible ? 'appLayerShown' : 'appLayerHidden') }));
      },
      onLockLayer: (layerId, locked) => {
        this.canvas?.setLayerLocked(layerId, locked);
      },
      onChangeTemplate: () => this.promptChangeDocumentTemplate(),
      onOpenSettings: () => this.settingsModal.show(),
      onOpenAbout: () => this.aboutModal.show(),
      onActiveTabChange: () => this.updateStoryToolbar(),
      onPanelToggle: () => this.updateStoryToolbar()
    });
    this.workbench.appendChild(this.structureBar.element);

    // 3b. Center: Spread Canvas
    this.canvas = new SpreadCanvas({
      onSelectionModeChange: (mode, frameData) => {
        this.inspector.setMode(mode, frameData);
      },
      onRequestActionHud: (x, y, initialValues) => {
        this.actionHud.showAt(x, y, initialValues);
      },
      onDismissActionHud: () => {
        this.actionHud.hide();
      },
      onPageChange: (idx) => {
        this.updatePageStats(idx);
      },
      onZoomChange: (z) => this.statusBar?.updateStats({ zoom: z }),
      onSelectFrameFlow: (flowId, paraId) => this.handleCanvasFrameSelect(flowId, paraId),
      onFrameDoubleClick: (flowId, paraId) => {
        this.handleCanvasFrameSelect(flowId, paraId);
        if (this.currentViewMode === 'canvas') {
          this.setViewMode('split');
          this.topBar?.setViewMode('split');
        }
      }
    });

    // 3c. Continuous story editor (hidden in page view). In RTL the editor sits on the
    // right and the pages on the left in split view, with a draggable divider between.
    this.storyContainer = el('section', 'tok-story-panel', { 'aria-labelledby': 'tok-story-title' });
    this.storyContainer.style.display = 'none';

    this.storyHeader = el('h2', 'tok-visually-hidden', { id: 'tok-story-title' }, t('appStoryEditorTitle'));
    const storyScroll = el('div', 'tok-story-scroll');
    this.storyEditor = new StoryEditor(storyScroll);
    const syncStoryToDocument = (paragraphs?: StoryParagraph[]) => {
      if (this.activeFlowId) {
        const story = paragraphs || this.storyEditor.getStory();
        this.documentState.flows[this.activeFlowId] = story;
        this.setDirty(true);
      }
      this.scheduleWordCountUpdate();
      this.scheduleRepaginate();
    };
    this.storyEditor.onTextChange(() => syncStoryToDocument());
    this.storyEditor.onStoryChange((paras) => syncStoryToDocument(paras));

    this.storyEditor.getElement().addEventListener('keydown', (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F')) {
        e.preventDefault();
        this.toggleFindReplaceBar(true);
      }
    });

    document.addEventListener('selectionchange', this.onDocumentSelectionChange);

    this.storyContainer.appendChild(this.storyHeader);
    this.storyContainer.appendChild(this.buildStoryToolbar());
    this.storyContainer.appendChild(this.buildFindReplaceBar());
    this.storyContainer.appendChild(storyScroll);


    this.splitDivider = this.buildSplitDivider();
    this.splitDivider.style.display = 'none';

    this.workbench.appendChild(this.storyContainer);
    this.workbench.appendChild(this.splitDivider);
    this.workbench.appendChild(this.canvas.element);

    // 3d. Left Side (Trailing in RTL): Contextual Inspector
    this.inspector = new ContextualInspector({
      onDocumentChange: (settings) => {
        this.documentState.settings = { ...(this.documentState.settings || this.inspector.getDocumentSettings()), ...settings };
        if (settings.baselineGridPt || settings.baselineOffsetPt !== undefined) {
          const pt = settings.baselineGridPt || this.documentState.settings.baselineGridPt || 13;
          const offset = settings.baselineOffsetPt !== undefined ? settings.baselineOffsetPt : this.documentState.settings.baselineOffsetPt;
          this.canvas?.setBaselineGrid(pt, offset);
        }
        this.setDirty(true);
        this.scheduleRepaginate();
      },
      onFrameChange: (geometry) => {
        this.canvas?.applyFrameGeometry(geometry);
        this.setDirty(true);
        if (geometry.flowId || geometry.widthMm || geometry.heightMm) {
          this.scheduleRepaginate();
        }
      },
      onTypographyChange: (typo) => {
        const sel = this.storyEditor?.getStorySelection();
        if (sel) {
          const fmt: any = {};
          if (typo.fontFamily) fmt.fontFamily = typo.fontFamily;
          if (typo.fontSizePt) fmt.fontSizePt = typo.fontSizePt;
          if (typo.fontWeight) fmt.fontWeight = typo.fontWeight;
          if (Object.keys(fmt).length > 0 && !sel.isCollapsed) {
            this.storyEditor.formatSelection(fmt);
          }

          const patch: any = {};
          if (typo.fontFamily) patch.fontFamily = typo.fontFamily;
          if (typo.fontSizePt) patch.fontSizePt = typo.fontSizePt;
          if (typo.fontWeight) patch.fontWeight = typo.fontWeight;
          if (typo.lineHeightPt) patch.lineHeightPt = typo.lineHeightPt;
          if (typo.alignment) {
            patch.alignment = typo.alignment === 'justify' ? 'Justified' : typo.alignment === 'center' ? 'Center' : typo.alignment === 'left' ? 'Left' : 'Right';
          }
          if (Object.keys(patch).length > 0) {
            this.storyEditor.setParagraphPatch(sel.paraId, patch);
          }
          this.updateFormattingToolbarState();
        }
        if (this.activeFlowId) {
          this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory();
        }
        this.setDirty(true);
        this.scheduleRepaginate();
      },
      onSyncStyleToken: () => {
        const sel = this.storyEditor?.getStorySelection();
        if (sel) {
          this.storyEditor.setParagraphPatch(sel.paraId, {});
          this.updateFormattingToolbarState();
          if (this.activeFlowId) {
            this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory();
          }
          this.setDirty(true);
          this.scheduleRepaginate();
        }
        this.showToast(t('toastGlobalStyleSynced'));
      },
      onNormalizeNiqqud: () => {
        this.handleSystemAction('normalize-hebrew');
      },
      onOpenTextCleanup: () => {
        this.openTextCleanupWizard();
      },
      onAlignFrames: (alignType) => {
        if (this.canvas) {
          const { before, after } = this.canvas.alignSelectedFrames(alignType);
          if (this.storyEditor && before.length > 0) {
            this.storyEditor.executeTransaction(
              `Align frames (${alignType})`,
              () => {},
              {
                onUndo: () => this.canvas?.applyFrameRects(before),
                onRedo: () => this.canvas?.applyFrameRects(after)
              }
            );
          }
        }
        this.setDirty(true);
        this.showToast(tf('toastAlignApplied', { type: alignType }));
      },
      onOpenSettings: () => this.settingsModal.show(),
      onOpenWhatsNew: () => this.openWhatsNew()
    });
    this.workbench.appendChild(this.inspector.element);

    // 4. Status Bar (26px)
    this.statusBar = new StatusBar({
      onZoomChange: (z) => this.canvas.setZoom(z),
      onPreflightClick: () => {
        this.inspector.setMode('zero');
        this.showToast(t('toastPreflightOk'));
      },
      onWordCountClick: () => this.openTextStatsDialog(),
      onExportPdf: () => this.handleSystemAction('export-pdf')
    });
    this.root.appendChild(this.statusBar.element);
    // The structure bar starts on the Gemara flow; show its localized name.
    this.activeFlowId = 'gemara';
    this.statusBar.updateStats({ activeFlow: this.flowDisplayName(this.activeFlowId) });

    // 5. Action HUD (Floating, anchored)
    this.actionHud = new ActionHud({
      onFontChange: (f) => {
        this.storyEditor.formatSelection({ fontFamily: f });
        this.updateFormattingToolbarState();
      },
      onSizeChange: (s) => {
        this.storyEditor.formatSelection({ fontSizePt: s });
        this.updateFormattingToolbarState();
      },
      onWeightChange: (b) => {
        this.storyEditor.formatSelection({ fontWeight: b ? FONT_WEIGHT_BOLD : FONT_WEIGHT_REGULAR });
        this.updateFormattingToolbarState();
      },
      onAlignChange: (a) => {
        const sel = this.storyEditor.getStorySelection();
        if (sel) {
          const alignment = (a === 'justify' ? 'Justified' : a === 'center' ? 'Center' : a === 'left' ? 'Left' : 'Right') as any;
          this.storyEditor.setParagraphPatch(sel.paraId, { alignment });
          this.updateFormattingToolbarState();
        }
      },
      onStyleChange: (st) => {
        const sel = this.storyEditor.getStorySelection();
        if (sel) {
          this.storyEditor.setParagraphStyle(sel.paraId, st);
          this.updateFormattingToolbarState();
        }
        this.showToast(tf('toastQuickStyle', { name: st }));
      },
      onDismiss: () => this.inspector.setMode('zero')
    });
    this.root.appendChild(this.actionHud.element);

    // 6. Command Palette (Ctrl+K). Its global shortcut must be live from the start.
    this.commandPalette = new CommandPalette(this.buildCommands());
    this.root.appendChild(this.commandPalette.element);

    // Global keyboard shortcuts supporting Hebrew & English layouts
    this.bindKeyboardShortcuts();

    // 7. Welcome / Settings / About are created lazily on first open (see getters below).

    // Load plugins once the first frame is on screen: discovery/compilation happens
    // in the main process and must not compete with the initial paint.
    runWhenIdle(() => {
      this.pluginEngine.loadPlugins().catch(console.error);
    });

    // 8b. Check for major updates popup on startup
    runWhenIdle(() => {
      this.checkStartupUpdatesPrompt();
    });

    // 8. Initialize default multi-flow document state
    this.activeFlowId = 'gemara';
    this.storyEditor.loadStory(this.documentState.flows.gemara);
    this.repaginateAndSync(true);

    // 8c. Continuous Autosave & Draft Recovery initialization
    this.initAutosave();
    runWhenIdle(() => {
      this.checkDraftRecoveryOnStartup();
    });

    // 9. Startup file inspection: if opened with a file argument, load it directly
    const win = window as any;
    if (win?.tokIpc?.onOpenFile) {
      win.tokIpc.onOpenFile((file: string) => {
        if (file) this.openProjectFile(file);
      });
    }

    if (win?.tokIpc?.getStartupFile) {
      win.tokIpc.getStartupFile().then((startupFile: string | null) => {
        if (startupFile) {
          this.openProjectFile(startupFile);
        } else {
          this.welcomeModal.setHasOpenDocument(false);
          this.welcomeModal.show();
        }
      }).catch(() => {
        this.welcomeModal.setHasOpenDocument(false);
        this.welcomeModal.show();
      });
    } else {
      this.welcomeModal.setHasOpenDocument(false);
      this.welcomeModal.show();
    }
  }

  /** Standard keyboard shortcuts supporting both English and Hebrew layouts. */
  private bindKeyboardShortcuts(): void {
    if (typeof window !== 'undefined') {
      window.addEventListener('keydown', this.onWindowKeyDown);
    }
  }

  private handleGlobalKeyDown(e: KeyboardEvent): void {
      const target = e.target as HTMLElement | null;
      const isInput = Boolean(
        target && (
          target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable ||
          target.closest?.('.tok-story-scroll')
        )
      );

      if (e.key === 'Escape') {
        if (this.findBar && this.findBar.style.display !== 'none') {
          e.preventDefault();
          this.toggleFindReplaceBar(false);
          this.storyEditor?.getElement().focus();
          return;
        }
      }

      // F6: Cycle focus between Story Editor and Canvas
      if (e.key === 'F6') {
        e.preventDefault();
        if (this.currentViewMode === 'story' || this.currentViewMode === 'split') {
          const editorEl = this.storyEditor?.getElement();
          if (document.activeElement === editorEl || editorEl?.contains(document.activeElement)) {
            this.canvas.element.focus();
          } else {
            editorEl?.focus();
          }
        }
        return;
      }

      // F2: Rename Document
      if (e.key === 'F2') {
        e.preventDefault();
        this.topBar?.startRenaming();
        return;
      }

      const hasCtrl = e.ctrlKey || e.metaKey;
      if (!hasCtrl) return;

      const code = e.code;
      const key = e.key.toLowerCase();

      // Ctrl+F / Ctrl+כ: Find & Replace
      if (!e.shiftKey && (code === 'KeyF' || key === 'f' || key === 'כ')) {
        e.preventDefault();
        this.toggleFindReplaceBar(true);
        return;
      }

      // Ctrl+H / Ctrl+י: Replace
      if (!e.shiftKey && (code === 'KeyH' || key === 'h' || key === 'י')) {
        e.preventDefault();
        this.toggleFindReplaceBar(true);
        this.replaceInput?.focus();
        this.replaceInput?.select();
        return;
      }

      // Ctrl+Shift+P: Projects / Welcome Modal
      if (e.shiftKey && (code === 'KeyP' || key === 'p' || key === 'פ')) {
        e.preventDefault();
        this.welcomeModal.show();
        return;
      }

      // Ctrl+Shift+S: Save As
      if (e.shiftKey && (code === 'KeyS' || key === 's' || key === 'ד')) {
        e.preventDefault();
        this.handleSystemAction('save-as');
        return;
      }

      // Ctrl+S: Save Document
      if (!e.shiftKey && (code === 'KeyS' || key === 's' || key === 'ד')) {
        e.preventDefault();
        this.handleSystemAction('save-document');
        return;
      }

      // Ctrl+Shift+C: Text Cleanup Wizard
      if (e.shiftKey && (code === 'KeyC' || key === 'c' || key === 'ב')) {
        e.preventDefault();
        this.openTextCleanupWizard();
        return;
      }

      // Ctrl+Shift+N: Normalize Hebrew Niqqud & Cantillation
      if (e.shiftKey && (code === 'KeyN' || key === 'n' || key === 'מ')) {
        e.preventDefault();
        this.handleSystemAction('normalize-hebrew');
        return;
      }

      // Ctrl+Alt+J: Full Hebrew Justification
      if (e.altKey && (code === 'KeyJ' || key === 'j' || key === 'ח')) {
        e.preventDefault();
        this.handleSystemAction('apply-justification');
        return;
      }

      // Ctrl+W: Close Document
      if (!e.shiftKey && (code === 'KeyW' || key === 'w' || key === 'ן')) {
        e.preventDefault();
        this.closeDocument();
        return;
      }

      // Ctrl+Q: Quit / Exit Application
      if (!e.shiftKey && (code === 'KeyQ' || key === 'q' || key === '/' || key === 'ק')) {
        e.preventDefault();
        this.handleSystemAction('quit-app');
        return;
      }

      // Ctrl+N: New Document
      if (!e.shiftKey && (code === 'KeyN' || key === 'n' || key === 'מ')) {
        e.preventDefault();
        this.handleSystemAction('new-document');
        return;
      }

      // Ctrl+O: Open Document
      if (!e.shiftKey && (code === 'KeyO' || key === 'o' || key === 'ם')) {
        e.preventDefault();
        this.handleSystemAction('open-document');
        return;
      }

      // Ctrl+I: Import Document
      if (!e.shiftKey && (code === 'KeyI' || key === 'i' || key === 'ן')) {
        e.preventDefault();
        this.handleSystemAction('import-document');
        return;
      }

      // Ctrl+E or Ctrl+P: Export PDF
      if (!e.shiftKey && (code === 'KeyE' || key === 'e' || key === 'ק' || code === 'KeyP' || key === 'p' || key === 'פ')) {
        e.preventDefault();
        this.handleSystemAction('export-pdf');
        return;
      }

      // Ctrl+, : Settings
      if (code === 'Comma' || key === ',' || key === 'ת') {
        e.preventDefault();
        this.settingsModal.show();
        return;
      }

      // Don't hijack typing or input editing
      if (isInput) return;

      // Undo / Redo outside text inputs
      if (code === 'KeyZ' || key === 'z' || key === 'ז') {
        e.preventDefault();
        if (e.shiftKey) {
          this.runEditCommand('redo');
        } else {
          this.runEditCommand('undo');
        }
        return;
      }
      if (code === 'KeyY' || key === 'y' || key === 'ט') {
        e.preventDefault();
        this.runEditCommand('redo');
        return;
      }

      // View modes: Ctrl+1 (Canvas / Pages), Ctrl+2 (Split), Ctrl+3 (Story / Text Editor)
      if (code === 'Digit1' || key === '1') {
        e.preventDefault();
        this.setViewMode('canvas');
        this.topBar.setViewMode('canvas');
        return;
      }
      if (code === 'Digit2' || key === '2') {
        e.preventDefault();
        this.setViewMode('split');
        this.topBar.setViewMode('split');
        return;
      }
      if (code === 'Digit3' || key === '3') {
        e.preventDefault();
        this.setViewMode('story');
        this.topBar.setViewMode('story');
        return;
      }

      // Zoom: Ctrl+= / Ctrl++ (Zoom In), Ctrl+- (Zoom Out), Ctrl+0 (Fit / 100%)
      if (code === 'Equal' || key === '=' || key === '+') {
        e.preventDefault();
        const nextZoom = Math.min(300, Math.round(this.canvas.getZoom() * 1.2));
        this.canvas.setZoom(nextZoom);
        this.statusBar.updateStats({ zoom: nextZoom });
        return;
      }
      if (code === 'Minus' || key === '-') {
        e.preventDefault();
        const nextZoom = Math.max(25, Math.round(this.canvas.getZoom() / 1.2));
        this.canvas.setZoom(nextZoom);
        this.statusBar.updateStats({ zoom: nextZoom });
        return;
      }
      if (code === 'Digit0' || key === '0') {
        e.preventDefault();
        this.canvas.fitToWindow();
        return;
      }

      // Add new page: Ctrl+Enter
      if (code === 'Enter' || key === 'enter') {
        e.preventDefault();
        this.addNewPage();
        return;
      }
  }

  /**
   * Cleans up all global event listeners, timers, abort controllers, and sub-components.
   */
  public destroy(): void {
    this.cancelPendingRepaginate();
    if (this.wordCountTimer) {
      clearTimeout(this.wordCountTimer);
      this.wordCountTimer = null;
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', this.onWindowKeyDown);
      window.removeEventListener('beforeunload', this.onBeforeUnload);
    }
    if (typeof document !== 'undefined') {
      document.removeEventListener('selectionchange', this.onDocumentSelectionChange);
    }
    this.storyEditor?.destroy();
    this.canvas?.destroy();
  }

  /** Re-applies texts owned by the app shell after a language switch. */
  private applyLanguage(): void {
    const dir = i18n.getDirection();
    this.root.dir = dir;
    this.workbench.dir = dir;
    this.storyHeader.textContent = t('appStoryEditorTitle');
    this.updateStoryToolbar();
    // registerItem replaces by id, so plugin-registered commands are kept.
    for (const cmd of this.buildCommands()) this.commandPalette.registerItem(cmd);
    this.refreshThumbnails();
    this.updatePageStats(this.activePageIndex);
    if (this.activeFlowId) {
      this.statusBar.updateStats({ activeFlow: this.flowDisplayName(this.activeFlowId) });
    }
  }

  private flowDisplayName(flowId: string): string {
    const canonical = (flowId === 'main' || flowId === 'primary') ? 'gemara' : flowId;
    const fromStructure = this.structureBar?.getFlows().find((f) => f.id === canonical || f.id === flowId);
    if (fromStructure?.name) return fromStructure.name;
    const key = FLOW_NAME_KEYS[canonical] || FLOW_NAME_KEYS[flowId];
    return key ? t(key) : canonical;
  }

  private get welcomeModal(): WelcomeModal {
    if (!this.welcomeModalInstance) {
      this.welcomeModalInstance = new WelcomeModal({
        onSelectTemplate: (tmpl) => this.handleTemplateSelect(tmpl),
        onOpenProject: (path) => this.handleSystemAction('open-document', path),
        onImportDocument: () => this.handleSystemAction('import-document'),
        onClose: () => {
          this.welcomeModalInstance?.hide();
        },
        onOpenSettings: () => this.settingsModal.show(),
        onOpenAbout: () => this.aboutModal.show(),
        onOpenGuide: () => this.userGuideModal.show(),
        onExitApp: () => this.handleSystemAction('quit-app')
      });
      this.root.appendChild(this.welcomeModalInstance.element);
    }
    return this.welcomeModalInstance;
  }

  private get settingsModal(): SettingsModal {
    if (!this.settingsModalInstance) {
      this.settingsModalInstance = new SettingsModal({
        onLanguageChange: (lang) => {
          this.showToast(tf('toastLangUpdated', { lang: t(lang === 'he' ? 'appLangHebrew' : 'appLangEnglish') }));
        },
        onClose: () => {
          if (!this.hasOpenDocument) this.welcomeModal.show();
        },
        pluginEngine: this.pluginEngine,
        showToast: (msg) => this.showToast(msg),
        getGuides: () => this.canvas.getGuides(),
        setGuide: (guide, visible) => {
          const current = this.canvas.getGuides();
          if (guide === 'margins' && current.margins !== visible) this.canvas.toggleMarginsGuide();
          if (guide === 'baseline' && current.baseline !== visible) this.canvas.toggleBaselineGuide();
        },
        getZoom: () => this.canvas.getZoom(),
        onOpenWhatsNew: () => this.openWhatsNew(),
        setZoom: (z) => {
          this.canvas.setZoom(z);
          this.statusBar.updateStats({ zoom: z });
        }
      });
      this.root.appendChild(this.settingsModalInstance.element);
    }
    return this.settingsModalInstance;
  }

  private get aboutModal(): AboutModal {
    if (!this.aboutModalInstance) {
      this.aboutModalInstance = new AboutModal({
        onClose: () => {
          if (!this.hasOpenDocument) this.welcomeModal.show();
        },
        onCheckUpdates: () => this.settingsModal.show('updates'),
        onOpenWhatsNew: () => this.openWhatsNew()
      });
      this.root.appendChild(this.aboutModalInstance.element);
    }
    return this.aboutModalInstance;
  }

  private get userGuideModal(): UserGuideModal {
    if (!this.userGuideModalInstance) {
      this.userGuideModalInstance = new UserGuideModal({
        onClose: () => {
          if (!this.hasOpenDocument) this.welcomeModal.show();
        }
      });
      this.root.appendChild(this.userGuideModalInstance.element);
    }
    return this.userGuideModalInstance;
  }

  private get textStatsModal(): TextStatsModal {
    if (!this.textStatsModalInstance) {
      this.textStatsModalInstance = new TextStatsModal({
        onClose: () => {}
      });
      this.root.appendChild(this.textStatsModalInstance.element);
    }
    return this.textStatsModalInstance;
  }

  private get importTemplateModal(): ImportTemplateModal {
    if (!this.importTemplateModalInstance) {
      this.importTemplateModalInstance = new ImportTemplateModal({
        onSelect: (templateId) => {
          if (this.isChangingActiveDocumentTemplate) {
            this.isChangingActiveDocumentTemplate = false;
            this.changeDocumentTemplate(templateId);
            return;
          }
          if (this.pendingImportResult) {
            rememberImportTemplate(this.pendingImportResult.filePath, templateId);
            this.applyImportedDocument(this.pendingImportResult.result, this.pendingImportResult.filePath, templateId);
            this.pendingImportResult = null;
          }
        },
        onCancel: () => {
          this.isChangingActiveDocumentTemplate = false;
          this.pendingImportResult = null;
          if (!this.hasOpenDocument) {
            this.welcomeModal.show();
          }
        }
      });
      this.root.appendChild(this.importTemplateModalInstance.element);
    }
    return this.importTemplateModalInstance;
  }

  private get textCleanupModal(): TextCleanupModal {
    if (!this.textCleanupModalInstance) {
      this.textCleanupModalInstance = new TextCleanupModal({
        onApply: (options) => this.applyTextCleanup(options)
      });
      this.root.appendChild(this.textCleanupModalInstance.element);
    }
    return this.textCleanupModalInstance;
  }

  public openTextCleanupWizard(): void {
    const activeKey = this.activeFlowId || 'gemara';
    this.textCleanupModal.show({
      activeFlowId: activeKey,
      activeFlowDisplayName: this.flowDisplayName(activeKey),
      getParagraphs: (scope) => {
        if (scope === 'activeFlow') {
          return this.storyEditor ? this.storyEditor.getStory(true) : (this.documentState.flows[activeKey] || []);
        } else {
          const all: StoryParagraph[] = [];
          for (const [k, paras] of Object.entries(this.documentState.flows)) {
            if (k === this.activeFlowId && this.storyEditor) {
              all.push(...this.storyEditor.getStory(true));
            } else {
              all.push(...paras);
            }
          }
          return all;
        }
      }
    });
  }

  private applyTextCleanup(options: CleanupOptions): void {
    const activeKey = this.activeFlowId || 'gemara';
    let totalChanges = 0;

    if (options.scope === 'activeFlow') {
      const curParas = this.storyEditor ? this.storyEditor.getStory(true) : (this.documentState.flows[activeKey] || []);
      const { paragraphs: cleaned, stats } = cleanupParagraphs(curParas, options);
      totalChanges = stats.totalModifications;

      if (this.storyEditor) {
        this.storyEditor.executeTransaction(t('textCleanupTitle'), () => {
          this.storyEditor.loadStory(cleaned, false);
        });
      } else {
        this.documentState.flows[activeKey] = cleaned;
      }
    } else {
      // All flows
      for (const [flowId, paras] of Object.entries(this.documentState.flows)) {
        const cur = (flowId === this.activeFlowId && this.storyEditor) ? this.storyEditor.getStory(true) : paras;
        const { paragraphs: cleaned, stats } = cleanupParagraphs(cur, options);
        totalChanges += stats.totalModifications;

        if (flowId === this.activeFlowId && this.storyEditor) {
          this.storyEditor.executeTransaction(t('textCleanupTitle'), () => {
            this.storyEditor.loadStory(cleaned, false);
          });
        } else {
          this.documentState.flows[flowId] = cleaned;
        }
      }
    }

    this.setDirty(true);
    this.scheduleWordCountUpdate();
    this.scheduleRepaginate();
    this.showToast(tf('textCleanupToastSuccess', { count: totalChanges }));
  }

  private openTextStatsDialog(): void {
    const story = this.storyEditor ? this.storyEditor.getStory(true) : [];
    const fullText = story.map((s) => s.text).join('\n');
    const totalWords = (fullText.match(/\S+/g) || []).length;
    const charsWithSpaces = fullText.length;
    const charsWithoutSpaces = fullText.replace(/\s/g, '').length;
    const paragraphs = story.length || 1;
    const linesEstimate = Math.max(1, Math.ceil(totalWords / 9));
    const hebrewChars = (fullText.match(/[\u0590-\u05FF]/g) || []).length;
    const niqqudCount = (fullText.match(/[\u0591-\u05BD\u05BF\u05C1-\u05C2\u05C4-\u05C5\u05C7]/g) || []).length;

    const curPage = this.pages[this.activePageIndex];
    const pageText = curPage?.htmlContent ? curPage.htmlContent.replace(/<[^>]+>/g, ' ') : '';
    const pageWords = (pageText.match(/\S+/g) || []).length || Math.min(totalWords, Math.ceil(totalWords / Math.max(1, this.pages.length)));
    const gematria = curPage?.gematriaNumber || toHebrewGematria(this.activePageIndex + 1);

    this.textStatsModal.show({
      pageWords,
      totalWords,
      charsWithSpaces,
      charsWithoutSpaces,
      paragraphs,
      linesEstimate,
      hebrewChars,
      niqqudCount,
      pageNumber: gematria
    });
  }

  private get exportDialog(): ExportDialog {
    if (!this.exportDialogInstance) {
      this.exportDialogInstance = new ExportDialog({ onExport: (options) => this.runExport(options) });
      this.root.appendChild(this.exportDialogInstance.element);
    }
    return this.exportDialogInstance;
  }

  /** Opens the export dialog for the current document. */
  public openExportDialog(): void {
    const spread = groupIntoSpreads(this.pages.length).find((sp) => sp.includes(this.activePageIndex)) ?? [];
    this.exportDialog.show({
      documentTitle: this.documentTitle,
      pageCount: this.pages.length,
      spreadLabels: spread.map((pos) => {
        const p = this.pages[pos];
        return `${p ? p.gematriaNumber : toHebrewGematria(pos + 1)} ${isRectoPage(pos) ? 'ע״א' : 'ע״ב'}`;
      }),
      preflightStatus: this.statusBar.getPreflightStatus()
    });
  }

  /** Runs the export with the options chosen in the dialog. */
  private runExport(options: ExportOptions): void {
    const win = window as any;
    if (win.tokIpc) {
      this.showToast(t('toastExporting'));
      // Sync active story editor changes to documentState
      if (this.storyEditor && this.activeFlowId) {
        this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
      }
      this.documentState.title = this.documentTitle || this.documentState.title;

      // Convert documentState to Rust DocumentRoot model format
      const docRoot = documentStateToDocumentRoot(this.documentState);
      console.log('[TOK] Exporting real document to PDF:', { title: docRoot.metadata.title, options });

      this.statusBar?.setBusy(true, 'מייצא ל-PDF...');
      win.tokIpc.renderPdf({ document: docRoot, outputPath: options.fileName })
        .then(() => {
          this.statusBar?.setBusy(false);
          this.showToast(t('toastExportDone'));
        })
        .catch((err: any) => {
          this.statusBar?.setBusy(false);
          this.showToast(tf('toastExportError', { error: err?.message ?? String(err) }), true);
        });
    } else {
      this.showToast(t('toastExportSimulated'));
    }
  }

  /** Undo/redo in the story editor. */
  private runEditCommand(command: 'undo' | 'redo'): void {
    if (this.storyEditor) {
      if (command === 'undo') {
        this.storyEditor.undo();
      } else {
        this.storyEditor.redo();
      }
      this.updateFormattingToolbarState();
      this.scheduleWordCountUpdate();
      this.scheduleRepaginate();
    }
  }

  private buildStoryToolbar(): HTMLElement {
    const bar = el('div', 'tok-story-toolbar', { role: 'toolbar', 'aria-label': t('storyToolbar') });
    this.storyFlowChip = el('button', 'tok-chip-select', { type: 'button' });
    this.storyFlowChip.addEventListener('click', () => {
      if (this.structureBar?.isPanelOpen() && this.structureBar?.getActiveTab() === 'flows') {
        this.structureBar.setPanelOpen(false);
      } else {
        this.structureBar?.showTab('flows');
      }
      this.updateStoryToolbar();
    });
    bar.appendChild(this.storyFlowChip);
    bar.appendChild(el('span', 'tok-divider-v', { 'aria-hidden': 'true', style: 'height:18px' }));

    // History controls
    const undoBtn = iconButton('undo', 'ביטול פעולה (Ctrl+Z)', () => this.runEditCommand('undo'), {
      size: 14,
      className: 'tok-btn tok-btn-ghost tok-btn-xs'
    });
    undoBtn.setAttribute('aria-label', 'ביטול פעולה (Ctrl+Z)');
    undoBtn.setAttribute('aria-disabled', 'true');
    const redoBtn = iconButton('redo', 'ביצוע חוזר (Ctrl+Y)', () => this.runEditCommand('redo'), {
      size: 14,
      className: 'tok-btn tok-btn-ghost tok-btn-xs'
    });
    redoBtn.setAttribute('aria-label', 'ביצוע חוזר (Ctrl+Y)');
    redoBtn.setAttribute('aria-disabled', 'true');
    this.storyEditor.onHistoryChange((canUndo, canRedo) => {
      undoBtn.disabled = !canUndo;
      redoBtn.disabled = !canRedo;
      undoBtn.setAttribute('aria-disabled', String(!canUndo));
      redoBtn.setAttribute('aria-disabled', String(!canRedo));
      undoBtn.style.opacity = canUndo ? '1' : '0.4';
      redoBtn.style.opacity = canRedo ? '1' : '0.4';
      this.topBar?.setHistoryState(canUndo, canRedo);
    });
    bar.appendChild(undoBtn);
    bar.appendChild(redoBtn);
    bar.appendChild(el('span', 'tok-divider-v', { 'aria-hidden': 'true', style: 'height:18px' }));

    // Text formatting controls
    this.storyBoldBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'הדגשה (Ctrl+B)',
      'aria-label': 'הדגשה (Ctrl+B)',
      'aria-pressed': 'false',
      style: 'font-weight:700;min-width:24px;height:24px;padding:0;font-size:13px;'
    }, 'B');
    this.storyBoldBtn.addEventListener('click', () => {
      this.storyEditor.toggleFormatSelection('bold');
      this.updateFormattingToolbarState();
    });
    bar.appendChild(this.storyBoldBtn);

    this.storyItalicBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'נטוי (Ctrl+I)',
      'aria-label': 'נטוי (Ctrl+I)',
      'aria-pressed': 'false',
      style: 'font-style:italic;min-width:24px;height:24px;padding:0;font-size:13px;'
    }, 'I');
    this.storyItalicBtn.addEventListener('click', () => {
      this.storyEditor.toggleFormatSelection('italic');
      this.updateFormattingToolbarState();
    });
    bar.appendChild(this.storyItalicBtn);

    this.storyUnderlineBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'קו תחתי (Ctrl+U)',
      'aria-label': 'קו תחתי (Ctrl+U)',
      'aria-pressed': 'false',
      style: 'text-decoration:underline;min-width:24px;height:24px;padding:0;font-size:13px;'
    }, 'U');
    this.storyUnderlineBtn.addEventListener('click', () => {
      this.storyEditor.toggleFormatSelection('underline');
      this.updateFormattingToolbarState();
    });
    bar.appendChild(this.storyUnderlineBtn);

    const clearFmtBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'ניקוי עיצוב תווים (Ctrl+\\)',
      'aria-label': 'ניקוי עיצוב תווים (Ctrl+\\)',
      style: 'min-width:24px;height:24px;padding:0;font-size:11px;font-weight:600;'
    }, 'Tx');
    clearFmtBtn.addEventListener('click', () => {
      this.storyEditor.clearFormattingSelection();
      this.updateFormattingToolbarState();
    });
    bar.appendChild(clearFmtBtn);

    bar.appendChild(el('span', 'tok-divider-v', { 'aria-hidden': 'true', style: 'height:18px' }));

    // Font Family selector
    this.storyFontFamilySelect = el('select', 'tok-select tok-select-xs', {
      title: 'גופן מקטע / פיסקה',
      'aria-label': 'גופן מקטע / פיסקה',
      style: 'height:24px;font-size:11px;padding:0 6px;min-width:90px;max-width:140px;background:var(--tok-bg-surface-2);color:var(--tok-text-primary);border:1px solid var(--tok-border-strong);border-radius:4px;'
    }) as HTMLSelectElement;
    for (const font of getAvailableFonts()) {
      const opt = el('option', undefined, { value: font.family }, font.family) as HTMLOptionElement;
      this.storyFontFamilySelect.appendChild(opt);
    }
    this.storyFontFamilySelect.addEventListener('change', () => {
      const chosenFont = this.storyFontFamilySelect?.value;
      if (!chosenFont) return;
      const sel = this.storyEditor?.getStorySelection();
      if (sel) {
        if (!sel.isCollapsed) {
          this.storyEditor.formatSelection({ fontFamily: chosenFont });
        } else {
          this.storyEditor.setParagraphPatch(sel.paraId, { fontFamily: chosenFont });
        }
        this.setDirty(true);
        this.scheduleRepaginate();
        this.updateFormattingToolbarState();
      }
    });
    bar.appendChild(this.storyFontFamilySelect);

    // Alignment controls
    const alignRightBtn = iconButton('alignRight', 'יישור לימין', () => {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        this.storyEditor.setParagraphPatch(sel.paraId, { alignment: 'Right' });
        this.updateFormattingToolbarState();
      }
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    const alignCenterBtn = iconButton('alignCenter', 'יישור למרכז', () => {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        this.storyEditor.setParagraphPatch(sel.paraId, { alignment: 'Center' });
        this.updateFormattingToolbarState();
      }
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    const alignLeftBtn = iconButton('alignLeft', 'יישור לשמאל', () => {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        this.storyEditor.setParagraphPatch(sel.paraId, { alignment: 'Left' });
        this.updateFormattingToolbarState();
      }
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    const alignJustifyBtn = iconButton('alignJustify', 'יישור לשני הצדדים', () => {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        this.storyEditor.setParagraphPatch(sel.paraId, { alignment: 'Justified' });
        this.updateFormattingToolbarState();
      }
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    bar.appendChild(alignRightBtn);
    bar.appendChild(alignCenterBtn);
    bar.appendChild(alignLeftBtn);
    bar.appendChild(alignJustifyBtn);

    const outdentBtn = iconButton('chevronRight', 'הזחה החוצה (Shift+Tab)', () => {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        this.storyEditor.outdentParagraph(sel.paraId);
        this.updateFormattingToolbarState();
      }
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    const indentBtn = iconButton('chevronLeft', 'הזחה פנימה (Tab)', () => {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        this.storyEditor.indentParagraph(sel.paraId);
        this.updateFormattingToolbarState();
      }
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    bar.appendChild(outdentBtn);
    bar.appendChild(indentBtn);

    bar.appendChild(el('span', 'tok-divider-v', { 'aria-hidden': 'true', style: 'height:18px' }));

    // Find & replace button
    const findToggleBtn = iconButton('search', 'חיפוש והחלפה (Ctrl+F)', () => {
      this.toggleFindReplaceBar();
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });
    bar.appendChild(findToggleBtn);

    // Text cleanup & Dibur HaMatchil wizard button
    const cleanupBtn = iconButton('sparkle', 'אשף ניקוי טקסט ודיבור המתחיל (Ctrl+Shift+C)', () => {
      this.openTextCleanupWizard();
    }, { size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });
    bar.appendChild(cleanupBtn);

    bar.appendChild(el('span', 'tok-divider-v', { 'aria-hidden': 'true', style: 'height:18px' }));

    // Typography size buttons
    const fontSmallerBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'הקטנת גודל טקסט בעורך (Ctrl+-)',
      style: 'min-width:26px;height:26px;padding:0 5px;font-size:12px;font-weight:700;line-height:1;display:flex;align-items:center;justify-content:center;'
    }, 'א⁻');
    fontSmallerBtn.addEventListener('click', () => this.setStoryFont(this.storyFontPx - 1));
    bar.appendChild(fontSmallerBtn);

    const fontLargerBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'הגדלת גודל טקסט בעורך (Ctrl++)',
      style: 'min-width:26px;height:26px;padding:0 5px;font-size:12px;font-weight:700;line-height:1;display:flex;align-items:center;justify-content:center;'
    }, 'א⁺');
    fontLargerBtn.addEventListener('click', () => this.setStoryFont(this.storyFontPx + 1));
    bar.appendChild(fontLargerBtn);

    bar.appendChild(el('span', 'tok-grow'));
    this.storyWordCount = el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted)' });
    bar.appendChild(this.storyWordCount);
    bar.appendChild(button(t('storyShowOnPage'), {
      className: 'tok-btn tok-btn-ghost tok-btn-sm tok-flip-rtl',
      onClick: () => {
        this.setViewMode('canvas');
        this.topBar.setViewMode('canvas');
        this.canvas.scrollToPage(this.activePageIndex);
      }
    }));
    this.updateStoryToolbar();
    return bar;
  }

  private buildFindReplaceBar(): HTMLElement {
    this.findBar = el('div', 'tok-story-find-bar', {
      style: 'display:none;padding:6px 10px;background:var(--tok-surface-2);border-bottom:1px solid var(--tok-border-subtle);flex-wrap:wrap;gap:6px;align-items:center;font-size:12px;'
    });

    this.findInput = el('input', 'tok-input', {
      type: 'text',
      'aria-label': 'חיפוש טקסט',
      placeholder: 'חיפוש טקסט...',
      style: 'width:140px;height:24px;font-size:12px;padding:0 6px;'
    }) as HTMLInputElement;

    this.replaceInput = el('input', 'tok-input', {
      type: 'text',
      'aria-label': 'החלפה בטקסט',
      placeholder: 'החלפה ב...',
      style: 'width:130px;height:24px;font-size:12px;padding:0 6px;'
    }) as HTMLInputElement;

    let ignoreNiqqud = true;
    const matchBadge = el('span', 'tok-find-badge', {
      role: 'status',
      'aria-live': 'polite',
      style: 'font-size:11px;color:var(--tok-text-muted);padding:0 4px;min-width:60px;'
    });

    const updateMatchBadge = () => {
      const q = this.findInput?.value || '';
      if (!q) {
        matchBadge.textContent = '';
        return;
      }
      const matches = this.storyEditor.findAll({ query: q, ignoreNiqqud });
      if (matches.length === 0) {
        matchBadge.textContent = '0 נמצאו';
        matchBadge.style.color = 'var(--tok-status-error, #ef4444)';
      } else {
        matchBadge.textContent = `${matches.length} נמצאו`;
        matchBadge.style.color = 'var(--tok-text-muted)';
      }
    };

    const niqqudBtn = el('button', 'tok-btn tok-btn-xs tok-btn-primary', {
      type: 'button',
      title: 'התעלם מניקוד ומטעמים בחיפוש',
      style: 'height:24px;padding:0 6px;'
    }, 'ללא ניקוד');
    niqqudBtn.addEventListener('click', () => {
      ignoreNiqqud = !ignoreNiqqud;
      if (ignoreNiqqud) {
        niqqudBtn.classList.add('tok-btn-primary');
        niqqudBtn.classList.remove('tok-btn-ghost');
      } else {
        niqqudBtn.classList.remove('tok-btn-primary');
        niqqudBtn.classList.add('tok-btn-ghost');
      }
      updateMatchBadge();
    });

    const prevBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'הקודם',
      style: 'height:24px;padding:0 6px;'
    }, '↑ הקודם');
    prevBtn.addEventListener('click', () => {
      const q = this.findInput?.value;
      if (q) {
        this.storyEditor.findPrev({ query: q, ignoreNiqqud });
        updateMatchBadge();
      }
    });

    const nextBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'הבא',
      style: 'height:24px;padding:0 6px;'
    }, '↓ הבא');
    nextBtn.addEventListener('click', () => {
      const q = this.findInput?.value;
      if (q) {
        this.storyEditor.findNext({ query: q, ignoreNiqqud });
        updateMatchBadge();
      }
    });

    this.findInput.addEventListener('input', updateMatchBadge);
    this.findInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const q = this.findInput?.value;
        if (!q) return;
        if (e.shiftKey) {
          this.storyEditor.findPrev({ query: q, ignoreNiqqud });
        } else {
          this.storyEditor.findNext({ query: q, ignoreNiqqud });
        }
        updateMatchBadge();
      } else if (e.key === 'Escape') {
        this.toggleFindReplaceBar(false);
      }
    });

    this.replaceInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const q = this.findInput?.value;
        if (q && this.replaceInput) {
          this.storyEditor.replaceNext(this.replaceInput.value, { query: q, ignoreNiqqud });
          this.updateFormattingToolbarState();
          this.scheduleRepaginate();
          updateMatchBadge();
        }
      }
    });

    const replaceBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'החלף מופע נוכחי',
      style: 'height:24px;padding:0 6px;'
    }, 'החלף');
    replaceBtn.addEventListener('click', () => {
      const q = this.findInput?.value;
      if (q && this.replaceInput) {
        const replaced = this.storyEditor.replaceNext(this.replaceInput.value, { query: q, ignoreNiqqud });
        this.updateFormattingToolbarState();
        this.scheduleRepaginate();
        updateMatchBadge();
        if (replaced) this.showToast('מופע הוחלף בהצלחה');
      }
    });

    const replaceAllBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'החלף את כל המופעים',
      style: 'height:24px;padding:0 6px;'
    }, 'החלף הכל');
    replaceAllBtn.addEventListener('click', () => {
      const q = this.findInput?.value;
      if (q && this.replaceInput) {
        const count = this.storyEditor.replaceAll(this.replaceInput.value, { query: q, ignoreNiqqud });
        this.updateFormattingToolbarState();
        this.scheduleRepaginate();
        updateMatchBadge();
        this.showToast(`הוחלפו ${count} מופעים`);
      }
    });

    const closeBtn = iconButton('close', 'סגור (Esc)', () => {
      this.toggleFindReplaceBar(false);
    }, { size: 12, className: 'tok-btn tok-btn-ghost tok-btn-xs' });

    this.findBar.appendChild(this.findInput);
    this.findBar.appendChild(matchBadge);
    this.findBar.appendChild(niqqudBtn);
    this.findBar.appendChild(prevBtn);
    this.findBar.appendChild(nextBtn);
    this.findBar.appendChild(el('span', 'tok-divider-v', { 'aria-hidden': 'true', style: 'height:16px' }));
    this.findBar.appendChild(this.replaceInput);
    this.findBar.appendChild(replaceBtn);
    this.findBar.appendChild(replaceAllBtn);
    this.findBar.appendChild(el('span', 'tok-grow'));
    this.findBar.appendChild(closeBtn);

    return this.findBar;
  }

  private toggleFindReplaceBar(force?: boolean): void {
    if (!this.findBar) return;
    const isVisible = this.findBar.style.display !== 'none';
    const show = force !== undefined ? force : !isVisible;
    this.findBar.style.display = show ? 'flex' : 'none';
    if (show) {
      const sel = this.storyEditor?.getStorySelection();
      if (sel && !sel.isCollapsed) {
        const p = this.storyEditor.getParagraph(sel.paraId);
        if (p) {
          const text = Array.from(p.text).slice(sel.start, sel.end).join('');
          if (text && this.findInput) {
            this.findInput.value = text;
          }
        }
      }
      this.findInput?.focus();
      this.findInput?.select();
    }
  }

  private updateFormattingToolbarState(): void {
    if (!this.storyBoldBtn || !this.storyItalicBtn || !this.storyUnderlineBtn || !this.storyEditor) return;
    const fmt = this.storyEditor.getSelectionFormat();
    const isBold = (fmt.fontWeight || 400) >= 600;
    const isItalic = Boolean(fmt.italic);
    const isUnderline = Boolean(fmt.underline);

    this.storyBoldBtn.style.background = isBold ? 'var(--tok-bg-surface-hover)' : '';
    this.storyItalicBtn.style.background = isItalic ? 'var(--tok-bg-surface-hover)' : '';
    this.storyUnderlineBtn.style.background = isUnderline ? 'var(--tok-bg-surface-hover)' : '';
    this.storyBoldBtn.setAttribute('aria-pressed', String(isBold));
    this.storyItalicBtn.setAttribute('aria-pressed', String(isItalic));
    this.storyUnderlineBtn.setAttribute('aria-pressed', String(isUnderline));

    if (this.storyFontFamilySelect) {
      const sel = this.storyEditor.getStorySelection();
      const p = sel ? this.storyEditor.getParagraph(sel.paraId) : null;
      const targetFamily = fmt.fontFamily || p?.patch?.fontFamily || (this.activeFlowId === 'gemara' ? 'Frank Ruhl Libre' : 'Noto Rashi Hebrew');
      if (targetFamily) {
        this.storyFontFamilySelect.value = targetFamily;
      }
    }

    if (this.inspector && this.currentViewMode !== 'canvas') {
      const sel = this.storyEditor.getStorySelection();
      if (sel) {
        const p = this.storyEditor.getParagraph(sel.paraId);
        const patch = p?.patch || {};
        const isBoldSel = isBold || ((patch.fontWeight || 400) >= 600);
        const typoData: any = {
          fontFamily: fmt.fontFamily || patch.fontFamily || (this.activeFlowId === 'gemara' ? 'Frank Ruhl Libre' : 'Noto Rashi Hebrew'),
          fontSizePt: fmt.fontSizePt || patch.fontSizePt || (this.activeFlowId === 'gemara' ? 15 : 11),
          fontWeight: isBoldSel ? FONT_WEIGHT_BOLD : FONT_WEIGHT_REGULAR,
          lineHeightPt: patch.lineHeightPt || (this.activeFlowId === 'gemara' ? 20 : 15),
          alignment: (patch.alignment?.toLowerCase() as any) || 'justify',
          styleTokenId: p?.styleId || 'main-text',
          styleTokenName: p?.styleId || 'טקסט ראשי',
          isOverride: Boolean(p?.patch && Object.keys(p.patch).length > 0)
        };
        this.inspector.setMode('text-edit', undefined, typoData);
      }
    }
  }


  private updateStoryToolbar(): void {
    if (!this.storyFlowChip) return;
    const flow = this.structureBar?.getFlows().find((f) => f.id === this.activeFlowId);
    const name = this.flowDisplayName(this.activeFlowId || 'gemara');
    this.storyFlowChip.replaceChildren();
    const sw = el('span', 'tok-swatch', { 'aria-hidden': 'true', style: 'width:8px;height:8px;border-radius:2px' });
    sw.style.background = flow?.color ?? 'var(--tok-accent-text)';
    this.storyFlowChip.appendChild(sw);
    this.storyFlowChip.appendChild(el('span', undefined, undefined, name));

    // Item 10: When flows panel is open, do not show opening arrow
    const isFlowsOpen = Boolean(this.structureBar?.isPanelOpen() && this.structureBar?.getActiveTab() === 'flows');
    if (!isFlowsOpen) {
      this.storyFlowChip.appendChild(icon('chevronDown', 12));
    }
    this.storyFlowChip.setAttribute('aria-label', tf('storyFlowChip', { name }));
    this.storyFlowChip.title = t('structureFlowsTitle');
    const words = this.storyEditor ? this.storyEditor.getStory().reduce((sum: number, p: StoryParagraph) => sum + getParagraphWordCount(p.id, p.text), 0) : 0;
    this.storyWordCount.textContent = tf('storyWordsCount', { n: words.toLocaleString() });
  }

  private setStoryFont(px: number): void {
    this.storyFontPx = Math.max(13, Math.min(32, px));
    this.storyEditor.setFontSize(this.storyFontPx);
  }

  /** Vertical divider between editor and pages in split view (drag or arrow keys). */
  private buildSplitDivider(): HTMLElement {
    const divider = el('div', 'tok-split-divider', { role: 'separator', 'aria-orientation': 'vertical', tabindex: '0', 'aria-label': t('splitResize') });
    const grip = el('span', undefined, { 'aria-hidden': 'true' });
    grip.appendChild(icon('grip', 14));
    divider.appendChild(grip);
    const setWidth = (px: number) => {
      const total = this.workbench.clientWidth;
      const w = Math.max(320, Math.min(total - 520, px));
      this.storyContainer.style.flex = `0 0 ${w}px`;
    };
    divider.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      divider.setPointerCapture(e.pointerId);
      const startX = e.clientX;
      const startW = this.storyContainer.getBoundingClientRect().width;
      const rtl = i18n.getDirection() === 'rtl';
      const move = (ev: PointerEvent) => setWidth(startW + (rtl ? startX - ev.clientX : ev.clientX - startX));
      const up = () => {
        divider.removeEventListener('pointermove', move);
        divider.removeEventListener('pointerup', up);
      };
      divider.addEventListener('pointermove', move);
      divider.addEventListener('pointerup', up);
    });
    divider.addEventListener('keydown', (e) => {
      if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
      e.preventDefault();
      const rtl = i18n.getDirection() === 'rtl';
      const grow = (e.key === 'ArrowLeft') === rtl;
      setWidth(this.storyContainer.getBoundingClientRect().width + (grow ? 24 : -24));
    });
    return divider;
  }

  public openWelcome(): void {
    this.welcomeModal.show();
  }

  public openSettings(): void {
    this.settingsModal.show();
  }

  public openAbout(): void {
    this.aboutModal.show();
  }
  public get updatesDrawer(): UpdatesDrawer {
    if (!this.updatesDrawerInstance) {
      this.updatesDrawerInstance = new UpdatesDrawer({
        onClose: () => {},
        onCheckUpdates: () => this.settingsModal.show('updates'),
        onOpenSettings: () => this.settingsModal.show('updates')
      });
      this.root.appendChild(this.updatesDrawerInstance.element);
    }
    return this.updatesDrawerInstance;
  }

  public get majorUpdateModal(): MajorUpdateModal {
    if (!this.majorUpdateModalInstance) {
      this.majorUpdateModalInstance = new MajorUpdateModal({
        onClose: () => {},
        onExploreAll: () => this.updatesDrawer.show()
      });
      this.root.appendChild(this.majorUpdateModalInstance.element);
    }
    return this.majorUpdateModalInstance;
  }

  public openWhatsNew(version?: string): void {
    this.updatesDrawer.show(version);
  }

  public openMajorUpdate(version?: string): void {
    this.majorUpdateModal.show(version);
  }

  private checkStartupUpdatesPrompt(): void {
    const updatesManager = UpdatesStateManager.getInstance();
    getAppVersion().then((ver: string) => {
      const version = ver || updatesManager.getLatestEntry()?.version;
      if (version && updatesManager.shouldPromptMajorUpdate(version)) {
        this.openMajorUpdate(version);
      }
    }).catch((e: any) => {
      console.warn('[UPDATES] Startup check error:', e);
    });
  }


  /** Built-in palette commands in the current UI language (page & layout modifying only). */
  private buildCommands(): PaletteItem[] {
    return [
      {
        id: 'cmd-whats-new',
        icon: 'sparkle',
        category: t('sidebarAbout'),
        title: t('commandPaletteWhatsNew'),
        subtitle: t('whatsNewSubtitle'),
        action: () => this.handleSystemAction('open-whats-new')
      },
      {
        id: 'cmd-full-justify',
        icon: 'alignJustify',
        category: t('cmdCatTypography'),
        title: t('cmdJustifyTitle'),
        subtitle: t('cmdJustifySub'),
        shortcut: 'Ctrl+Alt+J',
        action: () => this.handleSystemAction('apply-justification')
      },
      {
        id: 'cmd-text-cleanup',
        icon: 'sparkle',
        category: t('cmdCatTypography'),
        title: t('cmdTextCleanupTitle'),
        subtitle: t('cmdTextCleanupSub'),
        shortcut: 'Ctrl+Shift+C',
        action: () => this.openTextCleanupWizard()
      },
      {
        id: 'cmd-norm-niqqud',
        icon: 'sparkle',
        category: t('cmdCatTypography'),
        title: t('cmdNormalizeTitle'),
        subtitle: t('cmdNormalizeSub'),
        shortcut: 'Ctrl+Shift+N',
        action: () => this.handleSystemAction('normalize-hebrew')
      },
      {
        id: 'cmd-shield-divine',
        icon: 'lock',
        category: t('cmdCatTypography'),
        title: t('cmdShieldTitle'),
        subtitle: t('cmdShieldSub'),
        action: () => this.handleSystemAction('shield-divine-names')
      },
      {
        id: 'cmd-recalc-gematria',
        icon: 'refresh',
        category: t('cmdCatTypography'),
        title: t('cmdGematriaTitle'),
        subtitle: t('cmdGematriaSub'),
        action: () => this.handleSystemAction('recalculate-gematria')
      },
      {
        id: 'cmd-export-pdf',
        icon: 'export',
        category: t('cmdCatPrint'),
        title: t('cmdExportTitle'),
        subtitle: t('cmdExportSub'),
        shortcut: 'Ctrl+E',
        action: () => this.handleSystemAction('export-pdf')
      },
      {
        id: 'cmd-import-doc',
        icon: 'file',
        category: t('cmdCatProjects'),
        title: t('importDocument'),
        subtitle: t('importDocumentDesc'),
        shortcut: 'Ctrl+I',
        action: () => this.handleSystemAction('import-document')
      },
      {
        id: 'cmd-change-template',
        icon: 'templateBook',
        category: t('cmdCatPages'),
        title: t('structureChangeTemplate'),
        subtitle: 'החלפת פריסת העמודים ותזרימי הספר',
        action: () => this.promptChangeDocumentTemplate()
      },
      {
        id: 'cmd-close-document',
        icon: 'close',
        category: t('cmdCatProjects'),
        title: 'סגירת מסמך',
        subtitle: 'סגירת המסמך הפעיל וחזרה למסך הפתיחה',
        shortcut: 'Ctrl+W',
        action: () => this.closeDocument()
      },
      {
        id: 'cmd-quit-app',
        icon: 'close',
        category: t('cmdCatProjects'),
        title: t('menuExit'),
        subtitle: 'יציאה בטוחה מהתוכנה',
        shortcut: 'Ctrl+Q',
        action: () => this.handleSystemAction('quit-app')
      },
      {
        id: 'cmd-new-page',
        icon: 'pages',
        category: t('cmdCatPages'),
        title: t('cmdNewPageTitle'),
        shortcut: 'Ctrl+Enter',
        action: () => this.addNewPage()
      },
      {
        id: 'cmd-toggle-margins',
        icon: 'frame',
        category: t('cmdCatView'),
        title: t('cmdMarginsTitle'),
        action: () => {
          this.canvas.toggleMarginsGuide();
          this.showToast(t('toastMarginsToggled'));
        }
      },
      {
        id: 'cmd-toggle-baseline',
        icon: 'ruler',
        category: t('cmdCatView'),
        title: t('cmdBaselineTitle'),
        action: () => {
          this.canvas.toggleBaselineGuide();
          this.showToast(t('toastBaselineToggled'));
        }
      },
      {
        id: 'cmd-zoom-100',
        icon: 'search',
        category: t('cmdCatView'),
        title: t('cmdZoom100Title'),
        shortcut: 'Ctrl+0',
        action: () => {
          this.canvas.setZoom(100);
          this.statusBar.updateStats({ zoom: 100 });
        }
      },
      {
        id: 'cmd-zoom-fit',
        category: t('cmdCatView'),
        title: t('canvasFit'),
        icon: 'fit',
        action: () => this.canvas.fitToWindow()
      },
      {
        id: 'cmd-view-canvas',
        category: t('cmdCatView'),
        title: t('topBarViewCanvas'),
        icon: 'canvas',
        action: () => {
          this.setViewMode('canvas');
          this.topBar.setViewMode('canvas');
        }
      },
      {
        id: 'cmd-view-split',
        category: t('cmdCatView'),
        title: t('topBarViewSplit'),
        icon: 'split',
        action: () => {
          this.setViewMode('split');
          this.topBar.setViewMode('split');
        }
      },
      {
        id: 'cmd-view-story',
        category: t('cmdCatView'),
        title: t('topBarViewStory'),
        icon: 'story',
        action: () => {
          this.setViewMode('story');
          this.topBar.setViewMode('story');
        }
      }
    ];
  }

  public loadDocumentPages(pages: PageDescriptor[]): void {
    this.pages = pages;
    this.canvas.setPages(pages);
    this.refreshThumbnails();
    this.updatePageStats(0);
  }

  /** Loads paragraphs into the continuous Story Editor panel. */
  public loadStory(paragraphs: StoryParagraph[]): void {
    this.storyEditor.loadStory(paragraphs);
    if (this.activeFlowId) {
      this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory();
    }
    this.updateStoryToolbar();
    this.scheduleWordCountUpdate();
    this.scheduleRepaginate();
  }

  public getDocumentState(): MultiFlowDocumentState {
    if (this.storyEditor && this.activeFlowId) {
      this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
    }
    return this.documentState;
  }

  public getStoryEditor(): StoryEditor {
    return this.storyEditor;
  }

  private refreshThumbnails(): void {
    this.structureBar.setPages(
      this.pages.map((p, idx) => ({
        pageIndex: p.pageIndex,
        gematria: p.gematriaNumber,
        label: tf('appPageThumb', { page: p.gematriaNumber }),
        isSpreadRight: isRightHandPage(idx)
      })),
      this.activePageIndex
    );
  }

  /** Switches active flow, syncing the story editor and structure bar. */
  public switchActiveFlow(flowId: string, scrollToParaId?: string): void {
    const canonicalFlowId = (flowId === 'main' || flowId === 'primary') ? 'gemara' : flowId;
    if (this.storyEditor && this.activeFlowId) {
      this.storyEditor.flush();
      this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
    }
    this.activeFlowId = canonicalFlowId;

    if (canonicalFlowId === 'gemara') {
      const rawFlows = this.documentState.flows as Record<string, StoryParagraph[] | undefined>;
      if ((!rawFlows.gemara || rawFlows.gemara.length === 0) && rawFlows.main && rawFlows.main.length > 0) {
        rawFlows.gemara = rawFlows.main;
      }
    }

    if (!this.documentState.flows[canonicalFlowId]) {
      this.documentState.flows[canonicalFlowId] = [];
    }
    const targetFlowParas = this.documentState.flows[canonicalFlowId];
    this.storyEditor.loadStory(targetFlowParas);

    const name = this.flowDisplayName(canonicalFlowId);
    this.statusBar.updateStats({ activeFlow: name });
    this.structureBar.setActiveFlow(canonicalFlowId);
    this.updateStoryToolbar();

    if (scrollToParaId) {
      setTimeout(() => {
        const editorEl = this.storyEditor.getElement();
        const paraEl = editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(scrollToParaId)}"]`);
        if (paraEl) {
          editorEl.querySelectorAll('p.tok-current').forEach((p) => p.classList.remove('tok-current'));
          paraEl.classList.add('tok-current');
          paraEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      }, 50);
    }
  }

  private handleCanvasFrameSelect(flowId: string, paraId?: string): void {
    const canonicalFlowId = (flowId === 'main' || flowId === 'primary') ? 'gemara' : flowId;
    if (canonicalFlowId !== this.activeFlowId) {
      this.switchActiveFlow(canonicalFlowId, paraId);
      const name = this.flowDisplayName(canonicalFlowId);
      this.showToast(tf('toastFlowSelected', { name }));
    } else if (paraId) {
      const editorEl = this.storyEditor.getElement();
      const paraEl = editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
      if (paraEl) {
        editorEl.querySelectorAll('p.tok-current').forEach((p) => p.classList.remove('tok-current'));
        paraEl.classList.add('tok-current');
        paraEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
        this.storyEditor.setStorySelection({
          paraId,
          start: 0,
          end: 0,
          isCollapsed: true
        });
        this.updateFormattingToolbarState();
      }
    }
  }

  public cancelPendingRepaginate(): void {
    if (this.repaginateTimer) {
      clearTimeout(this.repaginateTimer);
      this.repaginateTimer = null;
    }
    if (this.activeTypesetAbortController) {
      this.activeTypesetAbortController.abort();
      this.activeTypesetAbortController = null;
    }
    this.statusBar?.setBusy(false);
    this.latestTypesetRequestId++;
  }

  public scheduleRepaginate(delayMs = 150, forceReset = false): void {
    if (this.repaginateTimer) {
      clearTimeout(this.repaginateTimer);
      this.repaginateTimer = null;
    }
    if (delayMs <= 0) {
      this.repaginateAndSync(forceReset);
      return;
    }
    this.repaginateTimer = setTimeout(() => {
      this.repaginateTimer = null;
      this.repaginateAndSync(forceReset);
    }, delayMs);
  }

  public async repaginateAndSync(forceReset = false): Promise<void> {
    if (this.storyEditor && this.activeFlowId) {
      this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
    }
    if (this.activeTypesetAbortController) {
      this.activeTypesetAbortController.abort();
    }
    const abortCtrl = new AbortController();
    this.activeTypesetAbortController = abortCtrl;

    const requestId = ++this.latestTypesetRequestId;
    this.statusBar?.setBusy(true, 'מעמד מחדש...');
    const win = window as any;
    let newPages: PageDescriptor[] = [];

    try {
      if (win.tokIpc?.typesetDocument) {
        try {
          const docRoot = documentStateToDocumentRoot(this.documentState);
          const res = await win.tokIpc.typesetDocument({ document: docRoot });
          // Discard stale response if a newer typeset request was dispatched while this was in flight
          if (requestId !== this.latestTypesetRequestId || abortCtrl.signal.aborted) {
            return;
          }
          if (res && res.success && Array.isArray(res.pages)) {
            newPages = layoutToPageDescriptors(res.pages);
          }
        } catch (err: any) {
          if (requestId !== this.latestTypesetRequestId || abortCtrl.signal.aborted) {
            return;
          }
          console.error('[TOK] Rust live typesetting failed:', err);
          this.showToast('עימוד Rust נכשל — מוצגת תצוגה משוערת בלבד', true);
        }
      }

      if (requestId !== this.latestTypesetRequestId || abortCtrl.signal.aborted) {
        return;
      }

      if (newPages.length === 0) {
        const minPages = Math.max(1, this.pages.length);
        const isExplicitFallback = Boolean(win.tokIpc?.typesetDocument);
        newPages = FlowPaginator.paginateDocument(this.documentState, minPages, isExplicitFallback);
      }

      if (requestId !== this.latestTypesetRequestId || abortCtrl.signal.aborted) {
        return;
      }

      this.pages = newPages;
      if (this.documentState?.settings?.baselineGridPt) {
        this.canvas.setBaselineGrid(this.documentState.settings.baselineGridPt, this.documentState.settings.baselineOffsetPt);
      }
      this.canvas.setPages(newPages, this.activePageIndex);
      this.refreshThumbnails();
      if (forceReset || this.activePageIndex >= newPages.length) {
        this.updatePageStats(0);
      } else {
        this.updatePageStats(this.activePageIndex);
      }

      // Update flow word counts across all flows
      const counts: Record<string, number> = {};
      let totalWords = 0;
      for (const [fid, paras] of Object.entries(this.documentState.flows)) {
        const w = paras.reduce((sum, p) => sum + getParagraphWordCount(p.id, p.text), 0);
        counts[fid] = w;
        totalWords += w;
      }
      this.structureBar.updateFlowWordCounts(counts);
      this.statusBar.updateStats({ wordCount: totalWords });
      this.updateStoryToolbar();
    } finally {
      if (this.activeTypesetAbortController === abortCtrl) {
        this.activeTypesetAbortController = null;
        this.statusBar?.setBusy(false);
      }
    }
  }

  /** Story edits update the word count in the status bar (debounced while typing). */
  private scheduleWordCountUpdate(): void {
    if (this.wordCountTimer) clearTimeout(this.wordCountTimer);
    this.wordCountTimer = setTimeout(() => {
      this.wordCountTimer = null;
      let totalWords = 0;
      for (const paras of Object.values(this.documentState.flows)) {
        totalWords += paras.reduce((sum, p) => sum + getParagraphWordCount(p.id, p.text), 0);
      }
      this.statusBar.updateStats({ wordCount: totalWords });
      this.updateStoryToolbar();
    }, 250);
  }

  /**
   * Resets active asynchronous operations, timers, and editor state prior to loading a new document session.
   */
  public resetDocumentSession(): void {
    this.cancelPendingRepaginate();
    if (this.wordCountTimer) {
      clearTimeout(this.wordCountTimer);
      this.wordCountTimer = null;
    }
    if (this.findBar) {
      this.toggleFindReplaceBar(false);
    }
    this.inspector?.setMode('zero');
    this.actionHud?.hide();
  }

  /**
   * Closes the active document after confirming unsaved changes, returning to the welcome project picker.
   */
  public closeDocument(): boolean {
    if (!this.confirmDiscardUnsaved()) return false;
    if (this.documentTitle) {
      updateProjectClosedTime(this.documentTitle);
    }
    this.resetDocumentSession();
    this.hasOpenDocument = false;
    this.setDirty(false);
    this.welcomeModal.setHasOpenDocument(false);
    this.welcomeModal.show();
    return true;
  }

  private handleTemplateSelect(templateId: string): void {
    this.resetDocumentSession();
    const isBlank = templateId === 'blank';
    const name = isBlank ? BLANK_TEMPLATE_TITLE : (TEMPLATE_TITLES[templateId] || BLANK_TEMPLATE_TITLE);
    this.documentTitle = name;
    this.documentState.title = name;
    this.documentState.templateType = (templateId in TEMPLATE_TITLES ? templateId : 'gemara') as TemplateType;
    this.currentFilePath = '';

    this.documentState.flows = { gemara: [], rashi: [], tosafot: [], notes: [] };

    this.activeFlowId = 'gemara';
    this.storyEditor.loadStory(this.documentState.flows.gemara);
    // The side panel must list the flows of this template, not whatever was loaded before.
    this.structureBar.setFlows(getFlowsForTemplate(this.documentState.templateType));
    this.activePageIndex = 0;
    this.structureBar.setActivePage(0);
    this.repaginateAndSync(true);

    this.hasOpenDocument = true;
    this.setDirty(false);
    this.showDocumentTitle(name, false);
    this.welcomeModal.setHasOpenDocument(true);
    this.welcomeModal.hide();

    this.showToast(tf('toastProjectCreated', { name }));
  }

    private async saveDocumentToFile(filePath: string, options: { silent?: boolean } = {}): Promise<boolean> {
    if (!options.silent) {
      this.statusBar?.setBusy(true, 'שומר מסמך...');
    }
    try {
      const win = window as any;
      if (this.storyEditor && this.activeFlowId) {
        this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
      }
      let targetPath = filePath;
      if (!targetPath.toLowerCase().endsWith('.tok')) {
        targetPath = targetPath.replace(/\.[^/\\.]+$/, '') + '.tok';
      }
      const baseName = targetPath.split(/[\\/]/).pop() || targetPath;
      this.documentState.title = this.documentTitle || baseName.replace(/\.tok$/i, '');
      this.documentState.filePath = targetPath;

      const docRoot = documentStateToDocumentRoot(this.documentState);

      if (win.tokIpc?.saveDocument) {
        try {
          await win.tokIpc.saveDocument({ document: docRoot, filePath: targetPath });
          this.currentFilePath = targetPath;
          this.documentTitle = baseName.replace(/\.tok$/i, '');
          this.setDirty(false);
          this.showDocumentTitle(this.documentTitle, false);
          addRecentProject({
            name: baseName,
            path: targetPath,
            pages: this.pages.length || 1,
            lastSavedAt: new Date().toISOString()
          });
          this.clearDraftRecovery();
          if (!options.silent) {
            this.showToast(t('toastSaved'));
          } else {
            this.statusBar?.setTemporaryStatus(t('autosavedStatus'), 2000);
          }
          return true;
        } catch (err: any) {
          console.error('[TOK] Failed to save document:', err);
          if (!options.silent) {
            this.showToast(tf('toastSaveError', { error: err?.message ?? String(err) }), true);
          }
          return false;
        }
      } else {
        try {
          if (typeof document !== 'undefined' && typeof Blob !== 'undefined') {
            const jsonStr = JSON.stringify(docRoot, null, 2);
            const blob = new Blob([jsonStr], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = baseName;
            document.body.appendChild(a);
            a.click();
            a.remove();
            URL.revokeObjectURL(url);
          }
          this.currentFilePath = targetPath;
          this.documentTitle = baseName.replace(/\.tok$/i, '');
          this.setDirty(false);
          this.showDocumentTitle(this.documentTitle, false);
          addRecentProject({
            name: baseName,
            path: targetPath,
            pages: this.pages.length || 1,
            lastSavedAt: new Date().toISOString()
          });
          this.clearDraftRecovery();
          if (!options.silent) {
            this.showToast(t('toastSaved'));
          } else {
            this.statusBar?.setTemporaryStatus(t('autosavedStatus'), 2000);
          }
          return true;
        } catch (err: any) {
          console.error('[TOK] Failed to export/save file:', err);
          if (!options.silent) {
            this.showToast(tf('toastSaveError', { error: err?.message ?? String(err) }), true);
          }
          return false;
        }
      }
    } finally {
      if (!options.silent) {
        this.statusBar?.setBusy(false);
      }
    }
  }

  private loadDocumentRoot(docRoot: any, filePath: string, title?: string): Promise<void> {
    this.resetDocumentSession();
    this.statusBar?.setBusy(true, 'טוען מסמך...');
    const baseName = filePath.split(/[\\/]/).pop() || filePath;
    const newState = documentRootToDocumentState(docRoot);
    this.documentState = newState;
    this.currentFilePath = filePath;
    const displayTitle = title || docRoot.metadata?.title || baseName;
    this.documentTitle = displayTitle;
    this.setDirty(false);
    this.showDocumentTitle(displayTitle, false);

    if (newState.settings) {
      this.inspector.setDocumentSettings({ ...newState.settings, title: displayTitle });
    }

    const flowKeys = Object.keys(newState.flows);
    const activeKey = (this.activeFlowId && newState.flows[this.activeFlowId])
      ? this.activeFlowId
      : (flowKeys[0] || 'gemara');
    this.activeFlowId = activeKey;
    this.structureBar.setFlows(getFlowsForTemplate(newState.templateType));
    this.structureBar.setActiveFlow(activeKey);
    const activeStory = newState.flows[activeKey] || [];
    this.storyEditor.loadStory(activeStory);

    this.activePageIndex = 0;
    this.structureBar.setActivePage(0);

    return this.repaginateAndSync(true).then(() => {
      this.hasOpenDocument = true;
      this.welcomeModal.setHasOpenDocument(true);
      this.welcomeModal.hide();
      this.canvas.scrollToPage(0);
      addRecentProject({
        name: displayTitle,
        path: filePath,
        pages: this.pages.length || 1,
        lastSavedAt: new Date().toISOString()
      });
      this.showToast(tf('toastOpenFile', { path: displayTitle }));
    }).finally(() => {
      this.statusBar?.setBusy(false);
    });
  }

  public openAnyFile(filePath: string): void {
    const ext = filePath.split('.').pop()?.toLowerCase();
    // The "document open" flag is set only once loading succeeds (loadDocumentRoot / applyImportedDocument),
    // otherwise a failed or cancelled open leaves the welcome screen's close button stuck on "return to document".
    this.welcomeModal.hide();
    if (ext === 'docx' || ext === 'md' || ext === 'txt' || ext === 'markdown') {
      this.promptAndImportDocument(filePath);
    } else {
      this.openProjectFile(filePath);
    }
  }

  private openProjectFile(filePath: string): void {
    const isImportFormat = /\.(docx|txt|md|markdown|rtf)$/i.test(filePath);
    if (isImportFormat) {
      this.promptAndImportDocument(filePath);
      return;
    }
    const win = window as any;
    const baseName = filePath.split(/[\\/]/).pop() || filePath;

    if (win.tokIpc?.openDocument) {
      win.tokIpc.openDocument(filePath)
        .then((docRoot: any) => {
          return this.loadDocumentRoot(docRoot, filePath, docRoot.metadata?.title || baseName);
        })
        .catch((err: any) => {
          console.error('[TOK] Failed to open document:', err);
          const rawMsg = err?.message ?? String(err);
          const isZipError = rawMsg.includes('Zip error') || rawMsg.includes('EOCD');
          const errorMsg = isZipError
            ? 'הקובץ אינו חבילת פרויקט (.tok) תקינה. אם זהו מסמך Word או Markdown, השתמש ב"ייבוא מסמך".'
            : (err?.userMessage || rawMsg);
          this.showToast(tf('toastOpenError', { error: errorMsg }), true);
          if (!this.hasOpenDocument) this.welcomeModal.show();
        });
      return;
    }

    this.documentTitle = baseName;
    this.setDirty(false);
    this.showDocumentTitle(baseName, false);
    this.currentFilePath = filePath;
    if (this.pages.length === 0) {
      this.pages = [{
        pageIndex: 0,
        gematriaNumber: 'א׳',
        widthPt: 480,
        heightPt: 678,
        htmlContent: ''
      }];
      this.loadDocumentPages(this.pages);
      this.storyEditor.loadStory([]);
    }
    this.hasOpenDocument = true;
    addRecentProject({ name: baseName, path: filePath, pages: this.pages.length || 1, lastSavedAt: new Date().toISOString() });
    this.welcomeModal.setHasOpenDocument(true);
    this.welcomeModal.hide();
    this.showToast(tf('toastOpenFile', { path: baseName }));
  }

  public async promptAndImportDocument(explicitPath?: string): Promise<void> {
    const win = window as any;
    let targetPath = explicitPath;

    if (!targetPath) {
      if (win.tokIpc?.showImportDialog) {
        targetPath = await win.tokIpc.showImportDialog();
      } else if (win.tokIpc?.showOpenDialog) {
        targetPath = await win.tokIpc.showOpenDialog({
          filters: [
            { name: 'כל המסמכים הנתמכים (*.docx, *.md, *.txt)', extensions: ['docx', 'md', 'txt'] },
            { name: 'Microsoft Word (*.docx)', extensions: ['docx'] },
            { name: 'Markdown (*.md)', extensions: ['md'] },
            { name: 'טקסט פשוט (*.txt)', extensions: ['txt'] }
          ]
        });
      }
    }

    if (!targetPath) {
      if (typeof document !== 'undefined' && !win.tokIpc) {
        this.promptWebFileImport();
      }
      return;
    }

    try {
      let result: ImportResult;
      if (win.tokIpc?.readImportFile) {
        const fileInfo = await win.tokIpc.readImportFile(targetPath);
        if (fileInfo.docxXml) {
          result = parseDocxXml(fileInfo.docxXml, { fileName: fileInfo.fileName });
        } else if (fileInfo.isBinary) {
          const binStr = atob(fileInfo.dataBase64);
          const len = binStr.length;
          const bytes = new Uint8Array(len);
          for (let i = 0; i < len; i++) {
            bytes[i] = binStr.charCodeAt(i);
          }
          result = importDocument(bytes, { fileName: fileInfo.fileName });
        } else {
          result = importDocument(fileInfo.text || '', { fileName: fileInfo.fileName });
        }
      } else {
        throw new Error('IPC reader not available');
      }

      // Re-opening a known file (recent projects, OS file association) reuses the flow chosen the first
      // time; the picker still appears for a first import and for an explicit "Import document".
      const remembered = explicitPath ? recallImportTemplate(targetPath) : null;
      if (remembered) {
        this.applyImportedDocument(result, targetPath, remembered);
        return;
      }
      this.pendingImportResult = { result, filePath: targetPath };
      this.importTemplateModal.show(result);
    } catch (err: any) {
      console.error('[TOK] Import error:', err);
      this.showToast(tf('toastImportError', { error: err?.userMessage || err?.message || String(err) }), true);
      if (!this.hasOpenDocument) this.welcomeModal.show();
    }
  }

  private promptWebFileImport(): void {
    if (typeof document === 'undefined') return;
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.docx,.md,.txt';
    input.style.display = 'none';
    input.addEventListener('change', () => {
      const file = input.files?.[0];
      if (!file) return;
      const isDocx = file.name.endsWith('.docx');
      const reader = new FileReader();
      reader.onload = () => {
        try {
          let content: string | Uint8Array;
          if (isDocx) {
            content = new Uint8Array(reader.result as ArrayBuffer);
          } else {
            content = reader.result as string;
          }
          const result = importDocument(content, { fileName: file.name });
          this.pendingImportResult = { result, filePath: file.name };
          this.importTemplateModal.show(result);
        } catch (e: any) {
          this.showToast(tf('toastImportError', { error: e?.userMessage || e?.message || String(e) }), true);
        }
      };
      if (isDocx) {
        reader.readAsArrayBuffer(file);
      } else {
        reader.readAsText(file);
      }
    });
    document.body.appendChild(input);
    input.click();
    setTimeout(() => input.remove(), 1000);
  }

  public applyImportedDocument(result: ImportResult, filePath?: string, templateType: TemplateType = 'prose'): void {
    this.resetDocumentSession();
    this.documentTitle = result.title;
    this.currentFilePath = filePath || '';
    this.showDocumentTitle(this.documentTitle, true);
    this.setDirty(true);
    this.hasOpenDocument = true;
    this.welcomeModal.setHasOpenDocument(true);
    this.welcomeModal.hide();

    this.documentState = importResultToDocumentState(result, templateType);
    this.activeFlowId = 'gemara';
    this.storyEditor.loadStory(this.documentState.flows.gemara);
    this.structureBar.setFlows(getFlowsForTemplate(templateType));
    this.activePageIndex = 0;
    this.structureBar.setActivePage(0);
    this.repaginateAndSync(true);

    addRecentProject({
      name: (this.documentTitle || 'imported') + '.tok',
      path: filePath,
      pages: this.pages.length || 1,
      lastSavedAt: new Date().toISOString()
    });

    this.showToast(tf('toastImportSuccess', { title: result.title, count: result.paragraphs.length }));
  }

  /**
   * Opens the template chooser dialog for the currently active document,
   * allowing the user to seamlessly switch between book layouts (prose, talmud, notes, etc.)
   * without losing any content.
   */
  public promptChangeDocumentTemplate(): void {
    if (!this.hasOpenDocument) {
      this.welcomeModal.show();
      return;
    }
    this.isChangingActiveDocumentTemplate = true;
    const currentStory = this.storyEditor?.getStory() || [];
    const totalParas = Object.values(this.documentState.flows).reduce((acc, f) => acc + (f?.length || 0), 0);
    const mockResult: ImportResult = {
      title: this.documentTitle || 'מסמך פעיל',
      paragraphs: currentStory,
      paragraphCount: totalParas || currentStory.length || 1,
      wordCount: 0,
      format: 'txt',
      detectedDirection: i18n.getDirection(),
      detectedLanguage: 'he'
    };
    this.importTemplateModal.show(mockResult);
  }

  /**
   * Applies a new template layout to the active document while preserving all existing texts.
   */
  public changeDocumentTemplate(newTemplate: TemplateType): void {
    if (!this.documentState) return;
    this.documentState.templateType = newTemplate;
    // Ensure all standard flows for the new template exist in documentState
    const templateFlows = getFlowsForTemplate(newTemplate);
    for (const f of templateFlows) {
      if (!this.documentState.flows[f.id]) {
        this.documentState.flows[f.id] = [];
      }
    }
    // Update structure bar and ensure active flow is valid
    this.structureBar.setFlows(templateFlows);
    if (!this.activeFlowId || !templateFlows.some((f) => f.id === this.activeFlowId)) {
      this.activeFlowId = templateFlows[0]?.id || 'gemara';
    }
    const targetFlow = this.activeFlowId;
    this.structureBar.setActiveFlow(targetFlow);
    this.storyEditor.loadStory(this.documentState.flows[targetFlow] || []);

    // If there is an associated import path, remember the new choice
    if (this.currentFilePath) {
      rememberImportTemplate(this.currentFilePath, newTemplate);
    }

    this.setDirty(true);
    this.repaginateAndSync(true);

    const templateNames: Record<string, string> = {
      gemara: t('templateGemara'),
      prose: t('templateProse'),
      notes: t('templateNotes'),
      mikraot: t('templateMikraot'),
      bulletin: t('templateBulletin'),
      blank: t('templateBlank')
    };
    const name = templateNames[newTemplate] || newTemplate;
    this.showToast(tf('toastTemplateSwitched', { name }));
  }

  public setViewMode(mode: ViewMode): void {
    const wasSplit = this.currentViewMode === 'split';
    this.currentViewMode = mode;
    this.workbench.classList.toggle('tok-split', mode === 'split');
    this.canvas.element.style.display = mode === 'story' ? 'none' : 'flex';
    this.storyContainer.style.display = mode === 'canvas' ? 'none' : 'flex';
    this.splitDivider.style.display = mode === 'split' ? 'flex' : 'none';
    // Story view fills the space; split starts at half and can be dragged.
    this.storyContainer.style.flex = mode === 'split' ? '1 1 0' : '1';

    // Split view needs the room: fold the side panel away, and bring it back after.
    if (mode === 'split' && !wasSplit) {
      this.panelOpenBeforeSplit = this.structureBar.isPanelOpen();
      this.structureBar.setPanelOpen(false);
    } else if (mode !== 'split' && wasSplit && this.panelOpenBeforeSplit !== null) {
      this.structureBar.setPanelOpen(this.panelOpenBeforeSplit);
      this.panelOpenBeforeSplit = null;
    }
    if (mode !== 'canvas') this.updateStoryToolbar();
  }

  public getViewMode(): ViewMode {
    return this.currentViewMode;
  }

  private addNewPage(): void {
    const activeKey = this.activeFlowId || 'gemara';
    const flowParas = this.documentState.flows[activeKey] || [];
    flowParas.push({
      id: `p-${Date.now()}`,
      text: '',
      styleId: activeKey === 'gemara' ? 'style-gemara-main' : 'style-rashi-comm'
    });
    this.documentState.flows[activeKey] = flowParas;
    if (this.storyEditor) {
      this.storyEditor.loadStory(flowParas);
    }
    this.setDirty(true);
    this.repaginateAndSync(false).then(() => {
      const newIdx = Math.max(0, this.pages.length - 1);
      this.canvas.scrollToPage(newIdx);
      this.updatePageStats(newIdx);
      const newGematria = this.pages[newIdx]?.gematriaNumber || toHebrewGematria(newIdx + 1);
      this.showToast(tf('toastPageAdded', { page: newGematria, index: newIdx + 1 }));
    });
  }

  private updatePageStats(idx: number): void {
    const maxIdx = Math.max(0, this.pages.length - 1);
    const validIdx = Math.min(Math.max(0, idx), maxIdx);
    this.activePageIndex = validIdx;
    const p = this.pages[validIdx];
    const gematria = p ? p.gematriaNumber : toHebrewGematria(validIdx + 1);
    this.statusBar.updateStats({
      pageLabel: tf('appPageStatus', { page: gematria, index: validIdx + 1, total: this.pages.length })
    });
    this.topBar.setPageLabel(`${tf('structurePageLabel', { g: gematria })} ${isRectoPage(validIdx) ? 'ע״א' : 'ע״ב'}`);
    this.structureBar.setActivePage(validIdx);
  }

  /**
   * Single entry point for app-level actions: top bar, command palette, the
   * native menu (via IPC) and `tok-action` DOM events all route through here.
   */
  public handleSystemAction(action: string, data?: unknown): Promise<any> | void {
    switch (action) {
      case 'quit-app':
      case 'exit':
        this.requestCloseOrExit('quit');
        break;
      case 'close-window':
        this.requestCloseOrExit('close');
        break;
      case 'close-document':
        this.closeDocument();
        break;
      case 'new-document':
        if (!this.confirmDiscardUnsaved()) return;
        if (this.documentTitle) {
          updateProjectClosedTime(this.documentTitle);
        }
        this.resetDocumentSession();
        this.welcomeModal.show();
        break;
      case 'open-projects':
      case 'open-welcome':
        if (this.documentTitle) {
          updateProjectClosedTime(this.documentTitle);
        }
        this.welcomeModal.show();
        break;
      case 'open-settings':
        this.settingsModal.show();
        break;
      case 'open-about':
        this.aboutModal.show();
        break;
      case 'open-whats-new':
        this.openWhatsNew();
        break;
      case 'open-file-at-path':
        if (typeof data === 'string' && data) {
          if (!this.confirmDiscardUnsaved()) return;
          this.openAnyFile(data);
        }
        break;
      case 'open-document': {
        if (!this.confirmDiscardUnsaved()) return;
        const win = window as any;
        if (typeof data === 'string' && data) {
          this.openAnyFile(data);
        } else if (win.tokIpc?.showOpenDialog) {
          win.tokIpc.showOpenDialog({ filters: [{ name: 'TypesetOK Document', extensions: ['tok', 'json'] }] })
            .then((filePath: string | null) => {
              if (filePath) this.openProjectFile(filePath);
            });
        } else if (typeof document !== 'undefined') {
          const input = document.createElement('input');
          input.type = 'file';
          input.accept = '.tok,.json';
          input.style.display = 'none';
          input.addEventListener('change', () => {
            const file = input.files?.[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = () => {
              try {
                const text = reader.result as string;
                const docRoot = JSON.parse(text);
                this.loadDocumentRoot(docRoot, file.name, file.name.replace(/\.(tok|json)$/i, ''));
              } catch (e: any) {
                this.showToast(tf('toastOpenError', { error: e?.message || 'Invalid format' }), true);
              }
            };
            reader.readAsText(file);
            input.remove();
          });
          document.body.appendChild(input);
          input.click();
        } else {
          this.showToast(tf('toastOpenFile', { path: '' }));
        }
        break;
      }
      case 'import-document': {
        if (!this.confirmDiscardUnsaved()) return;
        this.promptAndImportDocument(typeof data === 'string' ? data : undefined);
        break;
      }
      case 'save-document': {
        if (this.currentFilePath && this.currentFilePath.toLowerCase().endsWith('.tok')) {
          return this.saveDocumentToFile(this.currentFilePath);
        } else {
          return this.handleSystemAction('save-as');
        }
        break;
      }
      case 'save-as': {
        const win = window as any;
        const defaultName = (this.documentTitle || 'document').replace(/\.tok$/i, '') + '.tok';
        if (win.tokIpc?.showSaveDialog) {
          return win.tokIpc.showSaveDialog({
            defaultPath: defaultName,
            filters: [{ name: 'TypesetOK Document', extensions: ['tok', 'json'] }]
          }).then((filePath: string | null) => {
            if (filePath) {
              return this.saveDocumentToFile(filePath);
            }
          }).catch((err: any) => {
            console.error('[TOK] Save dialog error:', err);
            this.showToast(tf('toastSaveError', { error: err?.message || String(err) }), true);
          });
        } else {
          return this.saveDocumentToFile(defaultName);
        }
        break;
      }
      case 'export-pdf':
        this.openExportDialog();
        break;
      case 'rename-document':
        this.topBar?.startRenaming();
        break;
      case 'open-text-cleanup':
      case 'text-cleanup':
        this.openTextCleanupWizard();
        break;
      case 'normalize-hebrew': {
        const activeKey = this.activeFlowId || 'gemara';
        if (this.storyEditor && this.activeFlowId) {
          this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
        }
        const flowParas = this.documentState.flows[activeKey] || [];
        let modified = false;
        for (const p of flowParas) {
          const norm = normalizeQuotes(normalizeHebrew(p.text));
          if (norm !== p.text) {
            p.text = norm;
            modified = true;
          }
        }
        if (modified) {
          if (this.storyEditor) {
            this.storyEditor.loadStory(flowParas);
          }
          this.setDirty(true);
          this.scheduleWordCountUpdate();
          this.scheduleRepaginate();
        }
        this.showToast(t('toastNormalized'));
        break;
      }
      case 'shield-divine-names': {
        const activeKey = this.activeFlowId || 'gemara';
        if (this.storyEditor && this.activeFlowId) {
          this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
        }
        const flowParas = this.documentState.flows[activeKey] || [];
        let modified = false;
        for (const p of flowParas) {
          let nextText = p.text.replace(/יהוה/g, 'ה׳');
          const matches = findHolyNames(nextText);
          if (matches.length > 0) {
            let offset = 0;
            for (const m of matches) {
              const start = m.offset + offset;
              const end = start + m.len;
              const before = nextText.slice(0, start);
              const inside = nextText.slice(start, end);
              const after = nextText.slice(end);
              if (!inside.includes('\u2060')) {
                nextText = `${before}\u2060${inside}\u2060${after}`;
                offset += 2;
              }
            }
          }
          if (nextText !== p.text) {
            p.text = nextText;
            modified = true;
          }
        }
        if (modified) {
          if (this.storyEditor) {
            this.storyEditor.loadStory(flowParas);
          }
          this.setDirty(true);
          this.scheduleWordCountUpdate();
          this.scheduleRepaginate();
        }
        this.showToast(t('toastShieldOn'));
        break;
      }
      case 'recalculate-gematria': {
        for (let idx = 0; idx < this.pages.length; idx++) {
          this.pages[idx].gematriaNumber = toHebrewGematria(idx + 1);
        }
        this.canvas.setPages(this.pages, this.activePageIndex);
        this.refreshThumbnails();
        this.updatePageStats(this.activePageIndex);
        this.setDirty(true);
        this.showToast(t('toastGematriaSynced'));
        break;
      }
      case 'apply-justification': {
        const activeKey = this.activeFlowId || 'gemara';
        if (this.storyEditor && this.activeFlowId) {
          this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
        }
        const flowParas = this.documentState.flows[activeKey] || [];
        for (const p of flowParas) {
          p.patch = { ...(p.patch || {}), alignment: 'Justified' };
        }
        if (this.storyEditor) {
          this.storyEditor.loadStory(flowParas);
        }
        this.setDirty(true);
        this.scheduleRepaginate();
        this.updateFormattingToolbarState();
        this.showToast(t('toastJustified'));
        break;
      }
      default:
        console.log('[TOK] Action:', action);
    }
  }

  public showToast(msg: string, isError = false): void {
    // One toast at a time; a new message replaces the previous one.
    document.querySelectorAll('.tok-toast').forEach((n) => n.remove());
    const toast = el('div', isError ? 'tok-toast tok-toast-error' : 'tok-toast', { role: isError ? 'alert' : 'status' });
    toast.dir = i18n.getDirection();
    toast.appendChild(icon(isError ? 'warning' : 'check', 15));
    // Messages can carry file paths, CLI stderr or plugin text: never parse them as HTML.
    toast.appendChild(el('span', undefined, undefined, msg));
    document.body.appendChild(toast);
    setTimeout(() => {
      toast.classList.add('tok-toast-out');
      setTimeout(() => toast.remove(), 250);
    }, 3200);
  }

  /** Shows the document title in the top bar and in the inspector header, so both always agree. */
  private showDocumentTitle(title: string, isDirty: boolean): void {
    this.topBar.setDocumentTitle(title, isDirty);
    this.inspector?.setDocumentSettings({ title });
  }

  public renameDocument(newTitle: string): void {
    const trimmed = newTitle.trim();
    if (!trimmed || trimmed === this.documentTitle) return;
    this.documentTitle = trimmed;
    this.documentState.title = trimmed;
    this.showDocumentTitle(trimmed, true);
    this.setDirty(true);
    this.showToast(tf('toastTitleRenamed', { title: trimmed }));
  }

  public setDirty(dirty: boolean): void {
    if (!this.documentState) return;
    this.documentState.isDirty = dirty;
    if (typeof window !== 'undefined') {
      (window as any).__tokIsDirty = dirty;
    }
    this.topBar?.setDirty(dirty);
    const cleanTitle = (this.documentTitle || 'TypesetOK').replace(/\.tok$/i, '');
    if (typeof document !== 'undefined') {
      document.title = dirty ? `• ${cleanTitle} - TypesetOK` : `${cleanTitle} - TypesetOK`;
    }
    if (dirty) {
      this.scheduleAutosave();
    }
  }

  private initAutosave(): void {
    try {
      this.isAutosaveEnabled = typeof localStorage !== 'undefined'
        ? localStorage.getItem('tok_autosave_enabled') !== 'false'
        : true;
    } catch {
      this.isAutosaveEnabled = true;
    }
    if (typeof window !== 'undefined') {
      this.autosaveInterval = setInterval(() => {
        if (this.documentState?.isDirty && !this.isAutosaving) {
          this.triggerAutosave();
        }
      }, 30000);
    }
  }

  private scheduleAutosave(): void {
    if (!this.isAutosaveEnabled || !this.documentState) return;
    if (this.autosaveTimer) {
      clearTimeout(this.autosaveTimer);
    }
    this.autosaveTimer = setTimeout(() => {
      this.triggerAutosave();
    }, 2500);
  }

  public async triggerAutosave(): Promise<void> {
    if (!this.documentState || !this.documentState.isDirty || this.isAutosaving) return;
    this.isAutosaving = true;
    try {
      if (this.currentFilePath && this.currentFilePath.toLowerCase().endsWith('.tok')) {
        await this.saveDocumentToFile(this.currentFilePath, { silent: true });
      } else {
        this.saveDraftRecovery();
      }
    } catch (err) {
      console.warn('[TOK] Autosave failure:', err);
    } finally {
      this.isAutosaving = false;
    }
  }

  private saveDraftRecovery(): void {
    try {
      if (!this.documentState) return;
      if (this.storyEditor && this.activeFlowId) {
        this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory(true);
      }
      const docRoot = documentStateToDocumentRoot(this.documentState);
      const snapshot = {
        savedAt: new Date().toISOString(),
        title: this.documentTitle || 'פרויקט ללא שם',
        templateType: this.documentState.templateType,
        document: docRoot
      };
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('tok_draft_recovery', JSON.stringify(snapshot));
      }
      this.setDirty(false);
      this.statusBar?.setTemporaryStatus(t('draftSavedStatus'), 2000);
    } catch (err) {
      console.warn('[TOK] Failed to save draft recovery snapshot:', err);
    }
  }

  public clearDraftRecovery(): void {
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.removeItem('tok_draft_recovery');
      }
    } catch {}
  }

  private checkDraftRecoveryOnStartup(): void {
    try {
      if (typeof localStorage === 'undefined') return;
      const raw = localStorage.getItem('tok_draft_recovery');
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (parsed && parsed.document && parsed.savedAt) {
        if (parsed.title && isDemoProject(parsed.title, parsed.filePath)) {
          this.clearDraftRecovery();
          return;
        }
        const savedTime = new Date(parsed.savedAt).toLocaleTimeString();
        if (typeof window !== 'undefined' && typeof window.confirm === 'function' && !(window as any).__TOK_TEST_ENV__) {
          if (window.confirm(tf('draftRecoveryFound', { time: savedTime }))) {
            this.loadDocumentRoot(parsed.document, '', parsed.title || 'טיוטה משוחזרת');
            this.showToast(t('draftRecoveredToast'));
            return;
          } else {
            this.clearDraftRecovery();
          }
        }
      }
    } catch (err) {
      console.warn('[TOK] Error checking draft recovery:', err);
    }
  }

  public confirmDiscardUnsaved(): boolean {
    if (!this.documentState?.isDirty) return true;
    if (typeof window !== 'undefined' && typeof window.confirm === 'function') {
      return window.confirm(t('confirmUnsavedChanges'));
    }
    return true;
  }

  private isClosingInProgress = false;

  /**
   * Safe application close/exit coordination supporting save-before-exit, discard, and cancel.
   */
  public async requestCloseOrExit(action: 'close' | 'quit' = 'quit'): Promise<boolean> {
    if (this.isClosingInProgress) return false;
    this.isClosingInProgress = true;
    try {
      const win = typeof window !== 'undefined' ? (window as any) : null;
      if (this.documentTitle) {
        updateProjectClosedTime(this.documentTitle);
      }
      if (win?.tokIpc?.quitApp && action === 'quit') {
        await win.tokIpc.quitApp();
        return true;
      }
      if (win?.tokIpc?.closeWindow && action === 'close') {
        await win.tokIpc.closeWindow();
        return true;
      }
      // Web environment fallback
      if (!this.confirmDiscardUnsaved()) return false;
      return this.closeDocument();
    } finally {
      this.isClosingInProgress = false;
    }
  }
}

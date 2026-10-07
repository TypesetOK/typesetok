import { TopSystemBar } from './components/TopSystemBar';
import { StructureBar } from './components/StructureBar';
import { ContextualInspector } from './components/ContextualInspector';
import { ActionHud } from './components/ActionHud';
import { CommandPalette, PaletteItem } from './components/CommandPalette';
import { StatusBar } from './components/StatusBar';
import { SpreadCanvas, isRightHandPage, isRectoPage, groupIntoSpreads } from './components/SpreadCanvas';
import { ExportDialog, ExportOptions } from './components/ExportDialog';
import { WelcomeModal, addRecentProject, updateProjectClosedTime } from './components/WelcomeModal';
import { SettingsModal } from './components/SettingsModal';
import { AboutModal } from './components/AboutModal';
import { UserGuideModal } from './components/UserGuideModal';
import { TextStatsModal } from './components/TextStatsModal';
import { PluginEngine } from './plugins/PluginEngine';
import { StoryEditor, StoryParagraph } from 'tok-story-editor';
import { PageDescriptor } from 'tok-viewer';
import { ViewMode } from './types';
import { i18n, t, tf } from './i18n';
import { themeManager } from './theme';
import { FONT_WEIGHT_BOLD, FONT_WEIGHT_REGULAR } from './fonts';
import { el, icon, iconButton, button } from './ui';
import {
  FlowPaginator,
  MultiFlowDocumentState,
  DEFAULT_TALMUD_FLOWS,
  DEFAULT_PROSE_FLOWS,
  TemplateType
} from './engine/FlowPaginator';

import { toHebrewGematria } from './gematria';
export { toHebrewGematria };

/** Runs `fn` after the next paint, when the renderer is idle (max ~1s later). */
function runWhenIdle(fn: () => void): void {
  const w = window as any;
  requestAnimationFrame(() => {
    if (typeof w.requestIdleCallback === 'function') w.requestIdleCallback(fn, { timeout: 1000 });
    else setTimeout(fn, 0);
  });
}

/** i18n keys for the display names of the multi-flow ids used by the structure bar. */
const FLOW_NAME_KEYS: Record<string, string> = {
  gemara: 'inspFlowGemara',
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

const countWords = (text: string) => (text.match(/\S+/g) || []).length;

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
  private splitDivider!: HTMLElement;
  private panelOpenBeforeSplit: boolean | null = null;
  private documentTitle = '';
  private workbench!: HTMLElement;

  // Feature modals are built on first use (they are hidden at startup).
  private welcomeModalInstance?: WelcomeModal;
  private settingsModalInstance?: SettingsModal;
  private aboutModalInstance?: AboutModal;
  private exportDialogInstance?: ExportDialog;
  private userGuideModalInstance?: UserGuideModal;
  private textStatsModalInstance?: TextStatsModal;
  private pluginEngine!: PluginEngine;

  private hasOpenDocument = false;
  private currentViewMode: ViewMode = 'canvas';
  private pages: PageDescriptor[] = [];
  private activePageIndex = 0;
  private activeFlowId: string | null = null;
  private wordCountTimer: ReturnType<typeof setTimeout> | null = null;
  private repaginateTimer: ReturnType<typeof setTimeout> | null = null;
  private documentState: MultiFlowDocumentState = {
    title: 'פרויקט דף גמרא.tok',
    templateType: 'gemara',
    flows: JSON.parse(JSON.stringify(DEFAULT_TALMUD_FLOWS))
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

    this.initUI();

    i18n.onChange(() => this.applyLanguage());

    // Record exit/closing timestamp on window close
    window.addEventListener('beforeunload', () => {
      if (this.documentTitle) {
        updateProjectClosedTime(this.documentTitle);
      }
    });
  }

  private initUI(): void {
    // 1. Initialize Plugin Engine
    this.pluginEngine = new PluginEngine(
      (cmd) => this.commandPalette?.registerItem(cmd as any),
      (msg, isError) => this.showToast(msg, isError),
      (id) => this.commandPalette?.unregisterItem(id)
    );
    this.pluginEngine.setDelegate({
      getStory: () => (this.storyEditor ? this.storyEditor.getStory() : []),
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
        const story = this.storyEditor ? this.storyEditor.getStory() : [];
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
      onUndo: () => this.runEditCommand('undo'),
      onRedo: () => this.runEditCommand('redo')
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
        this.inspector.setMode('text-edit');
        this.showToast(tf('toastStyleApplied', { name: styleId }));
      },
      onToggleLayer: (layerId, visible) => {
        this.showToast(tf('toastLayerToggled', { name: layerId, state: t(visible ? 'appLayerShown' : 'appLayerHidden') }));
      },
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
      onSelectFrameFlow: (flowId, paraId) => this.handleCanvasFrameSelect(flowId, paraId)
    });

    // 3c. Continuous story editor (hidden in page view). In RTL the editor sits on the
    // right and the pages on the left in split view, with a draggable divider between.
    this.storyContainer = el('section', 'tok-story-panel', { 'aria-labelledby': 'tok-story-title' });
    this.storyContainer.style.display = 'none';

    this.storyHeader = el('h2', 'tok-visually-hidden', { id: 'tok-story-title' }, t('appStoryEditorTitle'));
    this.storyContainer.appendChild(this.storyHeader);
    this.storyContainer.appendChild(this.buildStoryToolbar());

    const storyScroll = el('div', 'tok-story-scroll');
    this.storyContainer.appendChild(storyScroll);
    this.storyEditor = new StoryEditor(storyScroll);
    this.storyEditor.onTextChange(() => {
      if (this.activeFlowId) {
        this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory();
      }
      this.scheduleWordCountUpdate();
      this.scheduleRepaginate();
    });

    this.splitDivider = this.buildSplitDivider();
    this.splitDivider.style.display = 'none';

    this.workbench.appendChild(this.storyContainer);
    this.workbench.appendChild(this.splitDivider);
    this.workbench.appendChild(this.canvas.element);

    // 3d. Left Side (Trailing in RTL): Contextual Inspector
    this.inspector = new ContextualInspector({
      onDocumentChange: (settings) => {
        console.log('[TOK] Document settings changed:', settings);
      },
      onFrameChange: (geometry) => {
        console.log('[TOK] Frame geometry changed:', geometry);
      },
      onTypographyChange: (typo) => {
        console.log('[TOK] Typography changed:', typo);
      },
      onSyncStyleToken: () => {
        this.showToast(t('toastGlobalStyleSynced'));
      },
      onNormalizeNiqqud: () => {
        this.handleSystemAction('normalize-hebrew');
      },
      onAlignFrames: (alignType) => {
        this.showToast(tf('toastAlignApplied', { type: alignType }));
      }
    });
    this.workbench.appendChild(this.inspector.element);

    // 4. Status Bar (26px)
    this.statusBar = new StatusBar({
      onZoomChange: (z) => this.canvas.setZoom(z),
      onPreflightClick: () => {
        this.inspector.setMode('zero');
        this.showToast(t('toastPreflightOk'));
      },
      onWordCountClick: () => this.openTextStatsDialog()
    });
    this.root.appendChild(this.statusBar.element);
    // The structure bar starts on the Gemara flow; show its localized name.
    this.activeFlowId = 'gemara';
    this.statusBar.updateStats({ activeFlow: this.flowDisplayName(this.activeFlowId) });

    // 5. Action HUD (Floating, anchored)
    this.actionHud = new ActionHud({
      onFontChange: (f) => this.inspector.setMode('text-edit', undefined, { fontFamily: f }),
      onSizeChange: (s) => this.inspector.setMode('text-edit', undefined, { fontSizePt: s }),
      onWeightChange: (b) => this.inspector.setMode('text-edit', undefined, { fontWeight: b ? FONT_WEIGHT_BOLD : FONT_WEIGHT_REGULAR }),
      onAlignChange: (a) => this.inspector.setMode('text-edit', undefined, { alignment: a }),
      onStyleChange: (st) => this.showToast(tf('toastQuickStyle', { name: st })),
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

    // 8. Initialize default multi-flow document state
    this.activeFlowId = 'gemara';
    this.storyEditor.loadStory(this.documentState.flows.gemara);
    this.repaginateAndSync(true);

    // 9. Main startup: Open project picker unconditionally as the primary screen
    this.welcomeModal.setHasOpenDocument(false);
    this.welcomeModal.show();
  }

  /** Standard keyboard shortcuts supporting both English and Hebrew layouts. */
  private bindKeyboardShortcuts(): void {
    window.addEventListener('keydown', (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isInput = Boolean(
        target && (
          target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable ||
          target.closest?.('.tok-story-scroll')
        )
      );

      const hasCtrl = e.ctrlKey || e.metaKey;
      if (!hasCtrl) return;

      const code = e.code;
      const key = e.key.toLowerCase();

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
    });
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
    const key = FLOW_NAME_KEYS[flowId];
    return key ? t(key) : flowId;
  }

  private get welcomeModal(): WelcomeModal {
    if (!this.welcomeModalInstance) {
      this.welcomeModalInstance = new WelcomeModal({
        onSelectTemplate: (tmpl) => this.handleTemplateSelect(tmpl),
        onOpenProject: (path) => this.handleSystemAction('open-document', path),
        onClose: () => {},
        onOpenSettings: () => this.settingsModal.show(),
        onOpenAbout: () => this.aboutModal.show(),
        onOpenGuide: () => this.userGuideModal.show()
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
        onCheckUpdates: () => this.settingsModal.show('updates')
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

  private openTextStatsDialog(): void {
    const story = this.storyEditor ? this.storyEditor.getStory() : [];
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

  /** Runs the export with the options chosen in the dialog (same tok-cli path as before). */
  private runExport(options: ExportOptions): void {
    const win = window as any;
    if (win.tokIpc) {
      this.showToast(t('toastExporting'));
      // The CLI currently renders its built-in sample document and takes only an
      // output path; the other options are recorded for when the engine accepts them.
      console.log('[TOK] Export options:', options);
      win.tokIpc.renderPdf('--demo', options.fileName)
        .then(() => {
          this.showToast(t('toastExportDone'));
        })
        .catch((err: any) => {
          this.showToast(tf('toastExportError', { error: err?.message ?? String(err) }), true);
        });
    } else {
      this.showToast(t('toastExportSimulated'));
    }
  }

  /** Undo/redo in the story editor (the only editable text surface for now). */
  private runEditCommand(command: 'undo' | 'redo'): void {
    const editor = this.storyEditor.getElement();
    if (!editor.contains(document.activeElement)) editor.focus();
    document.execCommand(command);
    this.scheduleWordCountUpdate();
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

    // Item 17: Compact typography size buttons with Hebrew letters
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
    const words = this.storyEditor ? this.storyEditor.getStory().reduce((sum: number, p: StoryParagraph) => sum + countWords(p.text), 0) : 0;
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

  /** Built-in palette commands in the current UI language (page & layout modifying only). */
  private buildCommands(): PaletteItem[] {
    return [
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
    this.updateStoryToolbar();
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
    if (this.storyEditor && this.activeFlowId) {
      this.documentState.flows[this.activeFlowId] = this.storyEditor.getStory();
    }
    this.activeFlowId = flowId;
    const targetFlowParas = this.documentState.flows[flowId] || [];
    this.storyEditor.loadStory(targetFlowParas);

    const name = this.flowDisplayName(flowId);
    this.statusBar.updateStats({ activeFlow: name });
    this.structureBar.setActiveFlow(flowId);
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
    if (flowId !== this.activeFlowId) {
      this.switchActiveFlow(flowId, paraId);
      const name = this.flowDisplayName(flowId);
      this.showToast(tf('toastFlowSelected', { name }));
    } else if (paraId) {
      const editorEl = this.storyEditor.getElement();
      const paraEl = editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
      if (paraEl) {
        editorEl.querySelectorAll('p.tok-current').forEach((p) => p.classList.remove('tok-current'));
        paraEl.classList.add('tok-current');
        paraEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }

  private scheduleRepaginate(): void {
    if (this.repaginateTimer) clearTimeout(this.repaginateTimer);
    this.repaginateTimer = setTimeout(() => {
      this.repaginateTimer = null;
      this.repaginateAndSync(false);
    }, 120);
  }

  private repaginateAndSync(forceReset = false): void {
    const minPages = Math.max(1, this.pages.length);
    const newPages = FlowPaginator.paginateDocument(this.documentState, minPages);
    this.pages = newPages;
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
      const w = paras.reduce((sum, p) => sum + countWords(p.text), 0);
      counts[fid] = w;
      totalWords += w;
    }
    this.structureBar.updateFlowWordCounts(counts);
    this.statusBar.updateStats({ wordCount: totalWords });
    this.updateStoryToolbar();
  }

  /** Story edits update the word count in the status bar (debounced while typing). */
  private scheduleWordCountUpdate(): void {
    if (this.wordCountTimer) clearTimeout(this.wordCountTimer);
    this.wordCountTimer = setTimeout(() => {
      this.wordCountTimer = null;
      let totalWords = 0;
      for (const paras of Object.values(this.documentState.flows)) {
        totalWords += paras.reduce((sum, p) => sum + countWords(p.text), 0);
      }
      this.statusBar.updateStats({ wordCount: totalWords });
      this.updateStoryToolbar();
    }, 250);
  }

  private handleTemplateSelect(templateId: string): void {
    const name = TEMPLATE_TITLES[templateId] || BLANK_TEMPLATE_TITLE;
    this.documentTitle = name;
    this.topBar.setDocumentTitle(name);
    this.documentState.title = name;
    this.documentState.templateType = (templateId in TEMPLATE_TITLES ? templateId : 'gemara') as TemplateType;

    if (templateId === 'prose') {
      this.documentState.flows = JSON.parse(JSON.stringify(DEFAULT_PROSE_FLOWS));
    } else {
      this.documentState.flows = JSON.parse(JSON.stringify(DEFAULT_TALMUD_FLOWS));
    }

    this.activeFlowId = 'gemara';
    this.storyEditor.loadStory(this.documentState.flows.gemara);
    this.repaginateAndSync(true);

    this.hasOpenDocument = true;
    addRecentProject({ name, pages: this.pages.length, lastSavedAt: new Date().toISOString() });
    this.welcomeModal.setHasOpenDocument(true);
    this.showToast(tf('toastProjectCreated', { name }));
  }

  private openProjectFile(filePath: string): void {
    const baseName = filePath.split(/[\\/]/).pop() || filePath;
    this.documentTitle = baseName;
    this.topBar.setDocumentTitle(baseName);
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
    this.showToast(tf('toastOpenFile', { path: baseName }));
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
    const minP = this.pages.length + 1;
    const newPages = FlowPaginator.paginateDocument(this.documentState, minP);
    this.pages = newPages;
    this.canvas.setPages(newPages, this.activePageIndex);
    this.refreshThumbnails();
    const newIdx = this.pages.length - 1;
    this.canvas.scrollToPage(newIdx);
    this.updatePageStats(newIdx);
    const newGematria = this.pages[newIdx].gematriaNumber;
    this.showToast(tf('toastPageAdded', { page: newGematria, index: newIdx + 1 }));
  }

  private updatePageStats(idx: number): void {
    this.activePageIndex = idx;
    const p = this.pages[idx];
    const gematria = p ? p.gematriaNumber : toHebrewGematria(idx + 1);
    this.statusBar.updateStats({
      pageLabel: tf('appPageStatus', { page: gematria, index: idx + 1, total: this.pages.length })
    });
    this.topBar.setPageLabel(`${tf('structurePageLabel', { g: gematria })} ${isRectoPage(idx) ? 'ע״א' : 'ע״ב'}`);
    this.structureBar.setActivePage(idx);
  }

  /**
   * Single entry point for app-level actions: top bar, command palette, the
   * native menu (via IPC) and `tok-action` DOM events all route through here.
   */
  public handleSystemAction(action: string, data?: unknown): void {
    switch (action) {
      case 'new-document':
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
      case 'open-document': {
        const win = window as any;
        if (typeof data === 'string' && data) {
          this.openProjectFile(data);
        } else if (win.tokIpc?.showOpenDialog) {
          win.tokIpc.showOpenDialog({ filters: [{ name: 'TypesetOK Document', extensions: ['tok'] }] })
            .then((filePath: string | null) => {
              if (filePath) this.openProjectFile(filePath);
            });
        } else {
          this.showToast(tf('toastOpenFile', { path: '' }));
        }
        break;
      }
      case 'save-document': {
        const nowIso = new Date().toISOString();
        addRecentProject({
          name: this.documentTitle,
          pages: this.pages.length || 1,
          lastSavedAt: nowIso
        });
        this.showToast(t('toastSaved'));
        break;
      }
      case 'save-as': {
        const win = window as any;
        const nowIso = new Date().toISOString();
        if (win.tokIpc?.showSaveDialog) {
          win.tokIpc.showSaveDialog({
            defaultPath: this.documentTitle,
            filters: [{ name: 'TypesetOK Document', extensions: ['tok'] }]
          }).then((filePath: string | null) => {
            if (filePath) {
              const baseName = filePath.split(/[\\/]/).pop() || filePath;
              this.documentTitle = baseName;
              this.topBar.setDocumentTitle(baseName);
              addRecentProject({
                name: baseName,
                path: filePath,
                pages: this.pages.length || 1,
                lastSavedAt: nowIso
              });
              this.showToast(t('toastSaved'));
            }
          });
        } else {
          addRecentProject({
            name: this.documentTitle,
            pages: this.pages.length || 1,
            lastSavedAt: nowIso
          });
          this.showToast(t('toastSaved'));
        }
        break;
      }
      case 'export-pdf':
        this.openExportDialog();
        break;
      case 'normalize-hebrew':
        this.showToast(t('toastNormalized'));
        break;
      case 'shield-divine-names':
        this.showToast(t('toastShieldOn'));
        break;
      case 'recalculate-gematria':
        this.showToast(t('toastGematriaSynced'));
        break;
      case 'apply-justification':
        this.showToast(t('toastJustified'));
        break;
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
      toast.style.transition = 'opacity 0.25s ease';
      toast.style.opacity = '0';
      setTimeout(() => toast.remove(), 250);
    }, 3200);
  }
}

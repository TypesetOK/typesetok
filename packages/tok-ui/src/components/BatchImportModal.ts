import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { getHostBridge } from '../platform/hostBridge.js';
import {
  LayoutRecommendation,
  analyzePageLayout,
  formatConfidencePercent,
} from '../services/layoutAnalyzerBridge';
import {
  BatchQueuedFile,
  BatchSortMode,
  applyCustomOrderList,
  getFileExtension,
  isSupportedImportFileName,
  reorderBatchItem,
  sortBatchFiles,
} from '../engine/batchFileOrder';

export type BatchImportTarget = 'new' | 'append';

export interface BatchImportLaunchOptions {
  files: BatchQueuedFile[];
  mode: 'stitch_continuous' | 'split_sections';
  insertFileHeading: boolean;
  continueOnError: boolean;
  target: BatchImportTarget;
}

export interface BatchImportModalCallbacks {
  onStartImport: (options: BatchImportLaunchOptions) => Promise<void>;
  onCancel?: () => void;
}

const PAGE_SIZE = 120;

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

let nextQueueId = 1;
export function createQueuedFileFromPath(
  filePath: string,
  meta?: { fileName?: string; relativePath?: string; size?: number; modifiedMs?: number; ext?: string }
): BatchQueuedFile {
  const fileName = meta?.fileName || filePath.split(/[\\/]/).pop() || filePath;
  return {
    id: `bq_${nextQueueId++}`,
    fileName,
    filePath,
    relativePath: meta?.relativePath || fileName,
    size: meta?.size ?? 0,
    modifiedMs: meta?.modifiedMs,
    ext: meta?.ext || getFileExtension(fileName),
  };
}

export function createQueuedFileFromWebFile(file: File, relativePath?: string): BatchQueuedFile {
  const rel = relativePath || (file as any).webkitRelativePath || file.name;
  return {
    id: `bq_${nextQueueId++}`,
    fileName: file.name,
    relativePath: rel,
    size: file.size,
    modifiedMs: file.lastModified,
    ext: getFileExtension(file.name),
    webFile: file,
  };
}

export class BatchImportModal {
  public element: HTMLElement;
  private modal: ModalController;
  private callbacks: BatchImportModalCallbacks;
  private isVisible = false;
  private hasOpenDocument = false;

  private files: BatchQueuedFile[] = [];
  private sortMode: BatchSortMode = 'natural';
  private filterQuery = '';
  private currentPage = 0;
  private mergeMode: 'stitch_continuous' | 'split_sections' = 'split_sections';
  private insertFileHeading = true;
  private continueOnError = true;
  private target: BatchImportTarget = 'new';

  private showPasteOrder = false;
  private pasteOrderText = '';
  private statusNotice = '';
  private isBusy = false;
  private progressPercent = 0;
  private detectedLayout: LayoutRecommendation | null = null;
  private progressText = '';
  private dragSourceIndex: number | null = null;

  private listContainer?: HTMLElement;
  private statsEl?: HTMLElement;
  private importBtn?: HTMLButtonElement;
  private progressWrap?: HTMLElement;
  private progressBarFill?: HTMLElement;
  private progressLabel?: HTMLElement;

  constructor(callbacks: BatchImportModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-batch-import-overlay');
    this.modal = new ModalController(this.element, () => {
      if (!this.isBusy) this.cancel();
    }, { closeOnBackdrop: false });

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      if (this.isVisible) this.render();
    });
  }

  public show(options?: { initialFiles?: BatchQueuedFile[]; hasOpenDocument?: boolean }): void {
    this.hasOpenDocument = Boolean(options?.hasOpenDocument);
    this.target = 'new';
    this.isBusy = false;
    this.progressPercent = 0;
    this.progressText = '';
    this.statusNotice = '';
    this.showPasteOrder = false;
    this.filterQuery = '';
    this.currentPage = 0;

    if (options?.initialFiles && options.initialFiles.length > 0) {
      this.files = sortBatchFiles(options.initialFiles, this.sortMode);
      void this.refreshLayoutRecommendation();
    } else {
      this.files = [];
      this.detectedLayout = null;
    }

    this.isVisible = true;
    this.element.dir = i18n.getDirection();
    this.render();
    this.element.classList.add('tok-open');
    this.modal.opened();
  }

  public hide(): void {
    if (!this.isVisible) return;
    this.isVisible = false;
    this.element.classList.remove('tok-open');
    this.modal.closed();
  }

  public setProgress(processed: number, total: number, label: string): void {
    this.progressPercent = total > 0 ? Math.min(100, Math.round((processed / total) * 100)) : 0;
    this.progressText = label;
    if (this.progressWrap) {
      this.progressWrap.style.display = 'flex';
    }
    if (this.progressBarFill) {
      this.progressBarFill.style.width = `${this.progressPercent}%`;
    }
    if (this.progressLabel) {
      this.progressLabel.textContent = label;
    }
  }

  private cancel(): void {
    this.hide();
    this.callbacks.onCancel?.();
  }

  private async refreshLayoutRecommendation(): Promise<void> {
    const candidate = this.files.find((f) => f.filePath && /\.(pdf|pgm)$/i.test(f.filePath));
    if (!candidate?.filePath) {
      this.detectedLayout = null;
      return;
    }
    try {
      const rec = await analyzePageLayout(candidate.filePath, 1, 10);
      this.detectedLayout = rec;
      if (this.isVisible) {
        this.render();
      }
    } catch {
      this.detectedLayout = null;
    }
  }

  private addQueuedFiles(incoming: BatchQueuedFile[]): void {
    if (incoming.length === 0) return;
    const existingKeys = new Set(
      this.files.map((f) => f.filePath || `${f.relativePath || f.fileName}:${f.size}`)
    );
    const added: BatchQueuedFile[] = [];
    for (const item of incoming) {
      if (!isSupportedImportFileName(item.fileName)) continue;
      if (item.fileName.startsWith('~$')) continue;
      const key = item.filePath || `${item.relativePath || item.fileName}:${item.size}`;
      if (existingKeys.has(key)) continue;
      existingKeys.add(key);
      added.push(item);
    }
    if (added.length === 0) return;

    const combined = [...this.files, ...added];
    this.files = this.sortMode === 'custom' ? combined : sortBatchFiles(combined, this.sortMode);
    this.currentPage = 0;
    this.render();
  }

  private async handlePickFiles(): Promise<void> {
    if (this.isBusy) return;
    const host = getHostBridge();
    if (host.has('showImportManyDialog') && host.showImportManyDialog) {
      try {
        const paths = await host.showImportManyDialog();
        if (paths && paths.length > 0) {
          this.addQueuedFiles(paths.map((p) => createQueuedFileFromPath(p)));
        }
      } catch (err) {
        console.error('[TOK] showImportManyDialog error:', err);
      }
      return;
    }

    this.openWebPicker(false);
  }

  private async handlePickFolder(): Promise<void> {
    if (this.isBusy) return;
    const host = getHostBridge();
    if (host.has('showImportFolderDialog') && host.showImportFolderDialog && host.scanImportFolder) {
      try {
        const folderPath = await host.showImportFolderDialog();
        if (!folderPath) return;
        this.statusNotice = t('batchScanningFolder');
        this.render();
        const scanned = await host.scanImportFolder(folderPath, true);
        this.statusNotice = '';
        this.addQueuedFiles(
          scanned.map((s) =>
            createQueuedFileFromPath(s.filePath, {
              fileName: s.fileName,
              relativePath: s.relativePath,
              size: s.size,
              modifiedMs: s.modifiedMs,
              ext: s.ext,
            })
          )
        );
      } catch (err) {
        console.error('[TOK] folder scan error:', err);
        this.statusNotice = '';
        this.render();
      }
      return;
    }

    this.openWebPicker(true);
  }

  private openWebPicker(isDirectory: boolean): void {
    if (typeof document === 'undefined') return;
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = true;
    input.accept = '.docx,.md,.markdown,.txt,.tag';
    if (isDirectory) {
      (input as any).webkitdirectory = true;
      (input as any).directory = true;
    }
    input.style.display = 'none';
    input.addEventListener('change', () => {
      const list = Array.from(input.files || []);
      if (list.length > 0) {
        this.addQueuedFiles(list.map((f) => createQueuedFileFromWebFile(f)));
      }
    });
    document.body.appendChild(input);
    input.click();
    setTimeout(() => input.remove(), 1500);
  }

  private async handleModalDrop(dataTransfer: DataTransfer): Promise<void> {
    const collected: BatchQueuedFile[] = [];
    const MAX_DROP_FILES = 10_000;
    const items = dataTransfer.items ? Array.from(dataTransfer.items) : [];
    const entries = items
      .map((item) => (typeof (item as any).webkitGetAsEntry === 'function' ? (item as any).webkitGetAsEntry() : null))
      .filter(Boolean);

    if (entries.length > 0) {
      const readEntry = async (entry: any, prefix: string): Promise<void> => {
        if (collected.length >= MAX_DROP_FILES) return;
        if (entry.isFile) {
          const file: File = await new Promise((resolve, reject) => entry.file(resolve, reject));
          const rel = prefix ? `${prefix}/${file.name}` : file.name;
          collected.push(createQueuedFileFromWebFile(file, rel));
        } else if (entry.isDirectory) {
          const reader = entry.createReader();
          const readBatch = (): Promise<any[]> =>
            new Promise((resolve, reject) => reader.readEntries(resolve, reject));
          let batch: any[] = await readBatch();
          while (batch.length > 0 && collected.length < MAX_DROP_FILES) {
            for (const child of batch) {
              const nextPrefix = prefix ? `${prefix}/${entry.name}` : entry.name;
              await readEntry(child, nextPrefix);
              if (collected.length >= MAX_DROP_FILES) break;
            }
            batch = await readBatch();
          }
        }
      };

      for (const entry of entries) {
        await readEntry(entry, '');
        if (collected.length >= MAX_DROP_FILES) break;
      }
      this.addQueuedFiles(collected);
      return;
    }

    const flat = Array.from(dataTransfer.files || []).map((f) => createQueuedFileFromWebFile(f));
    this.addQueuedFiles(flat);
  }

  private getFilteredIndices(): number[] {
    const q = this.filterQuery.trim().toLowerCase();
    if (!q) {
      return this.files.map((_, idx) => idx);
    }
    const indices: number[] = [];
    for (let i = 0; i < this.files.length; i++) {
      const f = this.files[i];
      if (
        f.fileName.toLowerCase().includes(q) ||
        (f.relativePath && f.relativePath.toLowerCase().includes(q))
      ) {
        indices.push(i);
      }
    }
    return indices;
  }

  private async startImport(): Promise<void> {
    if (this.isBusy || this.files.length === 0) return;
    this.isBusy = true;
    this.progressPercent = 0;
    this.progressText = tf('batchReadingProgress', { processed: 0, total: this.files.length });
    this.render();
    try {
      await this.callbacks.onStartImport({
        files: [...this.files],
        mode: this.mergeMode,
        insertFileHeading: this.insertFileHeading,
        continueOnError: this.continueOnError,
        target: this.hasOpenDocument ? this.target : 'new',
      });
      this.isBusy = false;
      this.hide();
    } catch (err) {
      this.isBusy = false;
      this.render();
    }
  }

  private render(): void {
    this.element.innerHTML = '';

    const dialog = el('div', 'tok-dialog tok-modal-card', {
      role: 'dialog',
      'aria-labelledby': 'tok-batch-import-h',
      style: 'max-width:860px;width:94%;max-height:88vh;margin:auto;display:flex;flex-direction:column;',
    });

    // Header
    const head = el('div', 'tok-modal-header', {
      style: 'display:flex;align-items:center;justify-content:space-between;padding:16px 22px;border-bottom:1px solid var(--tok-border-subtle);flex-shrink:0;',
    });
    const titleWrap = el('div');
    titleWrap.appendChild(
      el('h2', undefined, { id: 'tok-batch-import-h', style: 'margin:0 0 3px 0;font-size:18px;font-weight:700;' }, t('batchImportTitle'))
    );
    titleWrap.appendChild(
      el('p', undefined, { style: 'margin:0;font-size:12px;color:var(--tok-text-muted);' }, t('batchImportSubtitle'))
    );
    if (this.detectedLayout) {
      const rec = this.detectedLayout;
      titleWrap.appendChild(
        el('div', 'tok-batch-layout-badge', {
          style: 'display:inline-flex;align-items:center;gap:6px;margin-top:6px;font-size:11px;font-weight:600;padding:2px 10px;border-radius:12px;background:#10B98122;color:#10B981;'
        }, `${rec.displayNameHe} · ${tf('layoutAutoDetectedBadge', { pct: formatConfidencePercent(rec.confidence) })}`)
      );
    }
    head.appendChild(titleWrap);
    const closeBtn = iconButton('close', t('paletteHintClose'), () => {
      if (!this.isBusy) this.cancel();
    }, { size: 16 });
    closeBtn.disabled = this.isBusy;
    head.appendChild(closeBtn);
    dialog.appendChild(head);

    // Toolbar: Add Files, Add Folder, Sort, Reverse, Paste Order, Clear
    const toolbar = el('div', undefined, {
      style: 'display:flex;flex-wrap:wrap;align-items:center;gap:8px;padding:12px 22px;border-bottom:1px solid var(--tok-border-subtle);background:var(--tok-bg-surface-2, rgba(15,23,42,0.35));flex-shrink:0;',
    });

    const addFilesBtn = button(t('batchAddFiles'), {
      className: 'tok-btn tok-btn-sm tok-btn-primary',
      onClick: () => void this.handlePickFiles(),
    });
    addFilesBtn.disabled = this.isBusy;
    toolbar.appendChild(addFilesBtn);

    const addFolderBtn = button(t('batchAddFolder'), {
      className: 'tok-btn tok-btn-sm',
      onClick: () => void this.handlePickFolder(),
    });
    addFolderBtn.disabled = this.isBusy;
    toolbar.appendChild(addFolderBtn);

    const sortWrap = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px;margin-inline-start:auto;' });
    sortWrap.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);' }, t('batchSortLabel')));

    const sortSelect = el('select', 'tok-select', {
      style: 'font-size:12px;padding:4px 8px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-1, #0f172a);color:var(--tok-text-primary);',
    }) as HTMLSelectElement;
    const sortOptions: { value: BatchSortMode; label: string }[] = [
      { value: 'natural', label: t('batchSortNatural') },
      { value: 'hebrew-gematria', label: t('batchSortGematria') },
      { value: 'name-asc', label: t('batchSortNameAsc') },
      { value: 'name-desc', label: t('batchSortNameDesc') },
      { value: 'path-natural', label: t('batchSortPath') },
      { value: 'date-asc', label: t('batchSortDateAsc') },
      { value: 'date-desc', label: t('batchSortDateDesc') },
      { value: 'size-asc', label: t('batchSortSizeAsc') },
      { value: 'size-desc', label: t('batchSortSizeDesc') },
      { value: 'custom', label: t('batchSortCustom') },
    ];
    for (const opt of sortOptions) {
      const o = el('option', undefined, { value: opt.value }, opt.label) as HTMLOptionElement;
      if (opt.value === this.sortMode) o.selected = true;
      sortSelect.appendChild(o);
    }
    sortSelect.disabled = this.isBusy;
    sortSelect.addEventListener('change', () => {
      this.sortMode = sortSelect.value as BatchSortMode;
      if (this.sortMode !== 'custom') {
        this.files = sortBatchFiles(this.files, this.sortMode);
      }
      this.currentPage = 0;
      this.renderFileList();
    });
    sortWrap.appendChild(sortSelect);

    const revBtn = button(t('batchReverseOrder'), {
      className: 'tok-btn tok-btn-sm',
      onClick: () => {
        this.files = [...this.files].reverse();
        this.sortMode = 'custom';
        this.currentPage = 0;
        this.render();
      },
    });
    revBtn.disabled = this.isBusy || this.files.length < 2;
    sortWrap.appendChild(revBtn);

    const pasteBtn = button(t('batchPasteOrderBtn'), {
      className: 'tok-btn tok-btn-sm',
      onClick: () => {
        this.showPasteOrder = !this.showPasteOrder;
        this.render();
      },
    });
    pasteBtn.disabled = this.isBusy || this.files.length === 0;
    sortWrap.appendChild(pasteBtn);

    const clearBtn = button(t('batchClearAll'), {
      className: 'tok-btn tok-btn-sm',
      onClick: () => {
        this.files = [];
        this.currentPage = 0;
        this.render();
      },
    });
    clearBtn.disabled = this.isBusy || this.files.length === 0;
    sortWrap.appendChild(clearBtn);

    toolbar.appendChild(sortWrap);
    dialog.appendChild(toolbar);

    // Optional Paste Order Drawer
    if (this.showPasteOrder) {
      const drawer = el('div', undefined, {
        style: 'padding:12px 22px;border-bottom:1px solid var(--tok-border-subtle);background:rgba(59,130,246,0.06);display:flex;flex-direction:column;gap:8px;flex-shrink:0;',
      });
      drawer.appendChild(el('div', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);' }, t('batchPasteOrderHint')));
      const ta = el('textarea', undefined, {
        rows: '4',
        style: 'width:100%;box-sizing:border-box;padding:8px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-1, #0f172a);color:var(--tok-text-primary);font-family:inherit;font-size:12px;resize:vertical;',
      }) as HTMLTextAreaElement;
      ta.value = this.pasteOrderText;
      ta.addEventListener('input', () => {
        this.pasteOrderText = ta.value;
      });
      drawer.appendChild(ta);

      const drawerActions = el('div', undefined, { style: 'display:flex;justify-content:flex-end;gap:8px;' });
      drawerActions.appendChild(
        button(t('batchApplyOrderBtn'), {
          className: 'tok-btn tok-btn-sm tok-btn-primary',
          onClick: () => {
            const res = applyCustomOrderList(this.files, this.pasteOrderText);
            this.files = res.ordered;
            this.sortMode = 'custom';
            this.showPasteOrder = false;
            this.statusNotice = tf('batchOrderApplied', { matched: res.matchedCount });
            this.currentPage = 0;
            this.render();
          },
        })
      );
      drawer.appendChild(drawerActions);
      dialog.appendChild(drawer);
    }

    // Search + Stats Bar
    const filterRow = el('div', undefined, {
      style: 'display:flex;align-items:center;justify-content:space-between;gap:12px;padding:10px 22px;border-bottom:1px solid var(--tok-border-subtle);flex-shrink:0;',
    });
    const searchInput = el('input', undefined, {
      type: 'search',
      placeholder: t('batchSearchPlaceholder'),
      style: 'flex:1;max-width:360px;padding:6px 10px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-1, #0f172a);color:var(--tok-text-primary);font-size:12px;',
    }) as HTMLInputElement;
    searchInput.value = this.filterQuery;
    searchInput.disabled = this.isBusy;
    searchInput.addEventListener('input', () => {
      this.filterQuery = searchInput.value;
      this.currentPage = 0;
      this.renderFileList();
    });
    filterRow.appendChild(searchInput);

    this.statsEl = el('div', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);' });
    filterRow.appendChild(this.statsEl);
    dialog.appendChild(filterRow);

    // Status notice (if any)
    if (this.statusNotice) {
      dialog.appendChild(
        el('div', undefined, {
          style: 'padding:6px 22px;font-size:12px;color:var(--tok-accent, #60a5fa);background:rgba(59,130,246,0.08);flex-shrink:0;',
        }, this.statusNotice)
      );
    }

    // File List (dropzone + paginated rows)
    this.listContainer = el('div', undefined, {
      style: 'flex:1;min-height:220px;overflow-y:auto;padding:10px 22px;display:flex;flex-direction:column;gap:4px;',
    });
    this.listContainer.addEventListener('dragover', (e) => {
      if (e.dataTransfer && e.dataTransfer.types.includes('Files')) {
        e.preventDefault();
        e.stopPropagation();
      }
    });
    this.listContainer.addEventListener('drop', (e) => {
      if (e.dataTransfer && (e.dataTransfer.files?.length > 0 || e.dataTransfer.items?.length > 0)) {
        e.preventDefault();
        e.stopPropagation();
        void this.handleModalDrop(e.dataTransfer);
      }
    });
    dialog.appendChild(this.listContainer);
    this.renderFileList();

    // Options Row
    const optsRow = el('div', undefined, {
      style: 'display:flex;flex-wrap:wrap;align-items:center;gap:14px;padding:12px 22px;border-top:1px solid var(--tok-border-subtle);background:var(--tok-bg-surface-2, rgba(15,23,42,0.35));font-size:12px;flex-shrink:0;',
    });

    const modeWrap = el('label', undefined, { style: 'display:flex;align-items:center;gap:6px;' });
    modeWrap.appendChild(el('span', undefined, undefined, t('batchModeLabel')));
    const modeSel = el('select', undefined, {
      style: 'padding:4px 8px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-1, #0f172a);color:var(--tok-text-primary);font-size:12px;',
    }) as HTMLSelectElement;
    const optSec = el('option', undefined, { value: 'split_sections' }, t('batchModeSections')) as HTMLOptionElement;
    const optCont = el('option', undefined, { value: 'stitch_continuous' }, t('batchModeContinuous')) as HTMLOptionElement;
    if (this.mergeMode === 'split_sections') optSec.selected = true;
    else optCont.selected = true;
    modeSel.appendChild(optSec);
    modeSel.appendChild(optCont);
    modeSel.disabled = this.isBusy;
    modeSel.addEventListener('change', () => {
      this.mergeMode = modeSel.value as 'split_sections' | 'stitch_continuous';
    });
    modeWrap.appendChild(modeSel);
    optsRow.appendChild(modeWrap);

    const headingCbWrap = el('label', undefined, { style: 'display:flex;align-items:center;gap:6px;cursor:pointer;' });
    const headingCb = el('input', undefined, { type: 'checkbox' }) as HTMLInputElement;
    headingCb.checked = this.insertFileHeading;
    headingCb.disabled = this.isBusy;
    headingCb.addEventListener('change', () => {
      this.insertFileHeading = headingCb.checked;
    });
    headingCbWrap.appendChild(headingCb);
    headingCbWrap.appendChild(el('span', undefined, undefined, t('batchInsertHeading')));
    optsRow.appendChild(headingCbWrap);

    const errCbWrap = el('label', undefined, { style: 'display:flex;align-items:center;gap:6px;cursor:pointer;' });
    const errCb = el('input', undefined, { type: 'checkbox' }) as HTMLInputElement;
    errCb.checked = this.continueOnError;
    errCb.disabled = this.isBusy;
    errCb.addEventListener('change', () => {
      this.continueOnError = errCb.checked;
    });
    errCbWrap.appendChild(errCb);
    errCbWrap.appendChild(el('span', undefined, undefined, t('batchContinueOnError')));
    optsRow.appendChild(errCbWrap);

    if (this.hasOpenDocument) {
      const targetWrap = el('label', undefined, { style: 'display:flex;align-items:center;gap:6px;' });
      const targetSel = el('select', undefined, {
        style: 'padding:4px 8px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-1, #0f172a);color:var(--tok-text-primary);font-size:12px;',
      }) as HTMLSelectElement;
      const tNew = el('option', undefined, { value: 'new' }, t('batchTargetNew')) as HTMLOptionElement;
      const tAppend = el('option', undefined, { value: 'append' }, t('batchTargetAppend')) as HTMLOptionElement;
      if (this.target === 'new') tNew.selected = true;
      else tAppend.selected = true;
      targetSel.appendChild(tNew);
      targetSel.appendChild(tAppend);
      targetSel.disabled = this.isBusy;
      targetSel.addEventListener('change', () => {
        this.target = targetSel.value as BatchImportTarget;
      });
      targetWrap.appendChild(targetSel);
      optsRow.appendChild(targetWrap);
    }

    dialog.appendChild(optsRow);

    // Progress Bar (hidden until busy)
    this.progressWrap = el('div', undefined, {
      style: `display:${this.isBusy ? 'flex' : 'none'};flex-direction:column;gap:6px;padding:10px 22px;border-top:1px solid var(--tok-border-subtle);flex-shrink:0;`,
    });
    this.progressLabel = el('div', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);' }, this.progressText);
    const barBg = el('div', undefined, {
      style: 'width:100%;height:6px;border-radius:999px;background:rgba(148,163,184,0.2);overflow:hidden;',
    });
    this.progressBarFill = el('div', undefined, {
      style: `width:${this.progressPercent}%;height:100%;background:var(--tok-accent, #3b82f6);transition:width 0.12s ease;`,
    });
    barBg.appendChild(this.progressBarFill);
    this.progressWrap.appendChild(this.progressLabel);
    this.progressWrap.appendChild(barBg);
    dialog.appendChild(this.progressWrap);

    // Footer
    const footer = el('div', 'tok-modal-footer', {
      style: 'display:flex;justify-content:flex-end;gap:10px;padding:14px 22px;border-top:1px solid var(--tok-border-subtle);flex-shrink:0;',
    });
    const cancelBtn = button(t('cancel') || 'ביטול', {
      className: 'tok-btn tok-btn-sm',
      onClick: () => {
        if (!this.isBusy) this.cancel();
      },
    });
    cancelBtn.disabled = this.isBusy;
    footer.appendChild(cancelBtn);

    this.importBtn = button(tf('batchStartImport', { count: this.files.length }), {
      className: 'tok-btn tok-btn-sm tok-btn-primary',
      onClick: () => void this.startImport(),
    }) as HTMLButtonElement;
    this.importBtn.disabled = this.isBusy || this.files.length === 0;
    footer.appendChild(this.importBtn);
    dialog.appendChild(footer);

    this.element.appendChild(dialog);
  }

  private renderFileList(): void {
    const container = this.listContainer;
    if (!container) return;
    container.innerHTML = '';

    const totalSize = this.files.reduce((acc, f) => acc + (f.size || 0), 0);
    const filteredIndices = this.getFilteredIndices();
    if (this.statsEl) {
      this.statsEl.textContent = this.filterQuery.trim()
        ? tf('batchStatsFiltered', { shown: filteredIndices.length, total: this.files.length })
        : tf('batchStatsSummary', { count: this.files.length, size: formatBytes(totalSize) });
    }
    if (this.importBtn) {
      this.importBtn.textContent = tf('batchStartImport', { count: this.files.length });
      this.importBtn.disabled = this.isBusy || this.files.length === 0;
    }

    if (this.files.length === 0) {
      const empty = el('div', undefined, {
        style: 'margin:auto;padding:36px 20px;text-align:center;color:var(--tok-text-muted);border:2px dashed var(--tok-border-subtle);border-radius:12px;font-size:13px;max-width:480px;',
      }, t('batchEmptyDropHint'));
      container.appendChild(empty);
      return;
    }

    const totalPages = Math.max(1, Math.ceil(filteredIndices.length / PAGE_SIZE));
    if (this.currentPage >= totalPages) this.currentPage = totalPages - 1;
    const start = this.currentPage * PAGE_SIZE;
    const end = Math.min(filteredIndices.length, start + PAGE_SIZE);

    for (let pos = start; pos < end; pos++) {
      const idx = filteredIndices[pos];
      const item = this.files[idx];
      const row = el('div', 'tok-batch-file-row', {
        draggable: this.isBusy ? 'false' : 'true',
        style: 'display:flex;align-items:center;gap:10px;padding:6px 10px;border-radius:8px;border:1px solid var(--tok-border-subtle);background:var(--tok-bg-surface-1, rgba(15,23,42,0.5));font-size:12px;',
      });

      row.addEventListener('dragstart', () => {
        this.dragSourceIndex = idx;
      });
      row.addEventListener('dragover', (e) => {
        if (this.dragSourceIndex !== null) e.preventDefault();
      });
      row.addEventListener('drop', (e) => {
        e.preventDefault();
        if (this.dragSourceIndex !== null && this.dragSourceIndex !== idx) {
          this.files = reorderBatchItem(this.files, this.dragSourceIndex, idx);
          this.sortMode = 'custom';
          this.dragSourceIndex = null;
          this.render();
        }
      });

      row.appendChild(
        el('span', undefined, { style: 'min-width:34px;color:var(--tok-text-muted);font-variant-numeric:tabular-nums;' }, `${idx + 1}.`)
      );
      row.appendChild(
        el('span', undefined, {
          style: 'font-size:10px;font-weight:700;text-transform:uppercase;padding:2px 6px;border-radius:4px;background:rgba(59,130,246,0.16);color:#60a5fa;',
        }, item.ext || 'FILE')
      );

      const nameCol = el('div', undefined, { style: 'flex:1;min-width:0;display:flex;flex-direction:column;' });
      nameCol.appendChild(
        el('span', undefined, { style: 'overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-weight:500;' }, item.fileName)
      );
      if (item.relativePath && item.relativePath !== item.fileName) {
        nameCol.appendChild(
          el('span', undefined, { style: 'font-size:10px;color:var(--tok-text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;' }, item.relativePath)
        );
      }
      row.appendChild(nameCol);

      if (item.size > 0) {
        row.appendChild(
          el('span', undefined, { style: 'color:var(--tok-text-muted);font-size:11px;white-space:nowrap;' }, formatBytes(item.size))
        );
      }

      if (!this.isBusy) {
        const actions = el('div', undefined, { style: 'display:flex;align-items:center;gap:4px;' });
        const mkMoveBtn = (label: string, targetIdx: number, disabled: boolean) => {
          const b = el('button', 'tok-btn tok-btn-sm', {
            type: 'button',
            style: 'padding:2px 6px;font-size:11px;min-width:24px;',
          }, label) as HTMLButtonElement;
          b.disabled = disabled;
          b.addEventListener('click', () => {
            this.files = reorderBatchItem(this.files, idx, targetIdx);
            this.sortMode = 'custom';
            this.render();
          });
          return b;
        };

        actions.appendChild(mkMoveBtn('⤒', 0, idx === 0));
        actions.appendChild(mkMoveBtn('↑', idx - 1, idx === 0));
        actions.appendChild(mkMoveBtn('↓', idx + 1, idx === this.files.length - 1));
        actions.appendChild(mkMoveBtn('⤓', this.files.length - 1, idx === this.files.length - 1));

        const rmBtn = iconButton('close', 'הסר', () => {
          this.files.splice(idx, 1);
          this.render();
        }, { size: 12 });
        actions.appendChild(rmBtn);
        row.appendChild(actions);
      }

      container.appendChild(row);
    }

    if (totalPages > 1) {
      const pager = el('div', undefined, {
        style: 'display:flex;align-items:center;justify-content:center;gap:10px;padding:10px 0 4px 0;font-size:12px;',
      });
      const prev = button('‹', {
        className: 'tok-btn tok-btn-sm',
        onClick: () => {
          if (this.currentPage > 0) {
            this.currentPage--;
            this.renderFileList();
          }
        },
      });
      prev.disabled = this.currentPage === 0;
      const next = button('›', {
        className: 'tok-btn tok-btn-sm',
        onClick: () => {
          if (this.currentPage < totalPages - 1) {
            this.currentPage++;
            this.renderFileList();
          }
        },
      });
      next.disabled = this.currentPage >= totalPages - 1;
      pager.appendChild(prev);
      pager.appendChild(el('span', undefined, undefined, `${this.currentPage + 1} / ${totalPages}`));
      pager.appendChild(next);
      container.appendChild(pager);
    }
  }
}

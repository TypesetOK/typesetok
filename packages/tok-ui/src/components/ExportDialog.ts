import { t, tf, i18n } from '../i18n';
import type { PreflightReport } from '../engine/typesetBridge';
import { el, iconButton, button, kbd, switchRow, segmented, group, selectField } from '../ui';
import { ModalController } from './ModalController';
import { getHostBridge } from '../platform/hostBridge.js';

export type ExportFormat = 'pdfx1a' | 'pdfx4' | 'pdf';
export type ExportRange = 'all' | 'spread' | 'range';

export interface ExportOptions {
  format: ExportFormat;
  range: ExportRange;
  fromPage: number;
  toPage: number;
  bleed: boolean;
  cropMarks: boolean;
  registrationMarks: boolean;
  colorProfile: 'fogra39' | 'fogra51' | 'iso-coated-v2' | 'gracol' | 'swop';
  fileName: string;
}

export interface ExportDialogContext {
  documentTitle: string;
  pageCount: number;
  /** Labels of the pages of the current spread, e.g. ["ב׳", "ג׳"]. */
  spreadLabels: string[];
  preflightStatus: 'clean' | 'warning' | 'error';
  preflightReport?: PreflightReport | null;
  /**
   * The pages of the current spread exactly as the page viewer draws them (detached copies),
   * right-hand page first. Missing when the viewer cannot provide them.
   */
  previewSheets?: HTMLElement[];
}

export interface ExportDialogCallbacks {
  onExport: (options: ExportOptions) => void;
  /** Renders the real PDF with these options and shows it before anything is saved. */
  onPreviewPdf?: (options: ExportOptions) => void;
  /** Opens the full print-check dialog (the export dialog is closed first and reopened after). */
  onShowPreflight?: () => void;
}

/** Folder part of a path, or '' when it has none. */
function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return i > 0 ? path.slice(0, i) : '';
}

function baseName(path: string): string {
  return path.slice(Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1);
}

function joinPath(dir: string, name: string): string {
  if (!dir) return name;
  const sep = dir.includes('\\') && !dir.includes('/') ? '\\' : '/';
  return dir.endsWith('/') || dir.endsWith('\\') ? `${dir}${name}` : `${dir}${sep}${name}`;
}

/** "פרויקט.tok" -> "פרויקט.pdf" */
export function defaultExportFileName(documentTitle: string): string {
  const base = documentTitle.replace(/\.tok$/i, '').replace(/[\\/:*?"<>|]+/g, '').replace(/[\s—–-]+/g, '-').replace(/^-+|-+$/g, '');
  return `${base || 'TypesetOK'}.pdf`;
}

const FORMATS: { id: ExportFormat; code: string; titleKey: string; descKey: string; recommended?: boolean }[] = [
  { id: 'pdfx1a', code: 'PDF/X-1a', titleKey: 'exportX1aTitle', descKey: 'exportX1aDesc', recommended: true },
  { id: 'pdfx4', code: 'PDF/X-4', titleKey: 'exportX4Title', descKey: 'exportX4Desc' },
  { id: 'pdf', code: 'PDF', titleKey: 'exportScreenTitle', descKey: 'exportScreenDesc' }
];

/** Pixel width the real page preview is fitted to. */
const PREVIEW_WIDTH_PX = 252;

export class ExportDialog {
  public element: HTMLElement;
  private callbacks: ExportDialogCallbacks;
  private modal: ModalController;
  private isVisible = false;
  /** Folder the PDF is written to unless the user picks another one (the projects folder). */
  private saveDir = '';
  private context: ExportDialogContext = { documentTitle: '', pageCount: 0, spreadLabels: [], preflightStatus: 'clean', preflightReport: null };
  private options: ExportOptions = {
    format: 'pdfx1a',
    range: 'all',
    fromPage: 1,
    toPage: 1,
    bleed: true,
    cropMarks: true,
    registrationMarks: false,
    colorProfile: 'fogra39',
    fileName: ''
  };

  constructor(callbacks: ExportDialogCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-export-overlay');
    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });
    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(context: ExportDialogContext): void {
    const docChanged = context.documentTitle !== this.context.documentTitle;
    this.context = context;
    if (docChanged || !this.options.fileName) this.options.fileName = defaultExportFileName(context.documentTitle);
    this.options.toPage = Math.min(Math.max(this.options.toPage, 1), Math.max(1, context.pageCount));
    if (docChanged) this.options.toPage = Math.max(1, context.pageCount);
    this.isVisible = true;
    this.element.classList.add('tok-open');
    this.render();
    this.modal.opened();
    if (!this.saveDir) void this.loadDefaultSaveDir();
  }

  /** The PDF is saved in the TypesetOK projects folder by default. */
  private async loadDefaultSaveDir(): Promise<void> {
    try {
      const host = getHostBridge();
      if (!host.has('getProjectsDir')) return;
      const dir = await host.getProjectsDir();
      if (dir && !this.saveDir) {
        this.saveDir = dir;
        if (this.isVisible) this.render();
      }
    } catch {
      /* without a known folder the host decides where a bare file name goes */
    }
  }

  /** Final output path: the chosen folder (projects folder by default) plus the file name. */
  private outputPath(): string {
    const o = this.options;
    let name = o.fileName.trim() || defaultExportFileName(this.context.documentTitle);
    if (!/\.pdf$/i.test(name)) name += '.pdf';
    o.fileName = name;
    o.fromPage = Math.min(o.fromPage, o.toPage);
    o.toPage = Math.max(o.fromPage, o.toPage);
    return joinPath(this.saveDir, name);
  }

  public hide(): void {
    if (!this.isVisible) return;
    this.isVisible = false;
    this.element.classList.remove('tok-open');
    this.modal.closed();
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.renderContent();
    this.modal.afterRender(focusKey);
  }

  private renderContent(): void {
    this.element.innerHTML = '';
    this.element.dir = i18n.getDirection();
    const o = this.options;

    const card = el('div', 'tok-dialog');
    card.style.width = '820px';
    card.style.maxWidth = '96vw';
    card.style.maxHeight = '92vh';

    // Header
    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, t('exportTitle')));
    headText.appendChild(el('p', undefined, undefined, tf('exportSubtitle', { doc: this.context.documentTitle.replace(/\.tok$/i, ''), n: this.context.pageCount })));
    head.appendChild(headText);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', t('aboutClose'), () => this.hide(), { attrs: { 'data-focus-key': 'close' } }));
    card.appendChild(head);

    const body = el('div', 'tok-dialog-body');

    // ---- Options (short and in one column) ----
    const form = el('div', 'tok-export-form');
    form.style.gap = '14px';

    // Format: one segmented control and a one-line description of the chosen format.
    const fmtBox = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:6px' });
    fmtBox.appendChild(segmented(t('exportFormat'), FORMATS.map((f) => ({ id: f.id, label: f.recommended ? `${f.code} ★` : f.code })), o.format, (id) => {
      o.format = id;
      this.render();
    }, { fill: true, focusPrefix: 'format' }));
    const chosen = FORMATS.find((f) => f.id === o.format) ?? FORMATS[0];
    fmtBox.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, `${t(chosen.titleKey)} — ${t(chosen.descKey)}`));
    form.appendChild(group(t('exportFormat'), fmtBox));

    // Page range
    const rangeRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:12px;flex-wrap:wrap' });
    rangeRow.appendChild(segmented(t('exportPages'), [
      { id: 'all', label: t('exportAllPages') },
      { id: 'spread', label: t('exportCurrentSpread') },
      { id: 'range', label: t('exportRange') }
    ], o.range, (id) => {
      o.range = id;
      this.render();
    }, { focusPrefix: 'range' }));
    if (o.range === 'range') {
      const num = (labelKey: string, value: number, set: (v: number) => void, key: string) => {
        const wrap = el('label', undefined, { style: 'display:flex;align-items:center;gap:6px;font-size:12px;color:var(--tok-text-secondary)' });
        wrap.appendChild(el('span', undefined, undefined, t(labelKey)));
        const input = el('input', 'tok-input tok-input-number', { type: 'number', min: '1', max: String(this.context.pageCount), 'data-focus-key': key });
        input.value = String(value);
        input.style.width = '72px';
        input.addEventListener('change', () => {
          const v = Math.max(1, Math.min(this.context.pageCount, parseInt(input.value, 10) || 1));
          input.value = String(v);
          set(v);
        });
        wrap.appendChild(input);
        return wrap;
      };
      rangeRow.appendChild(num('exportRangeFrom', o.fromPage, (v) => {
        o.fromPage = v;
        if (o.fromPage > o.toPage) o.toPage = o.fromPage;
        this.render();
      }, 'range-from'));
      rangeRow.appendChild(num('exportRangeTo', o.toPage, (v) => {
        o.toPage = v;
        if (o.toPage < o.fromPage) o.fromPage = o.toPage;
        this.render();
      }, 'range-to'));
    } else {
      const summary = o.range === 'all'
        ? tf('welcomePagesCount', { n: this.context.pageCount })
        : this.context.spreadLabels.join(' – ');
      rangeRow.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, summary));
    }
    form.appendChild(group(t('exportPages'), rangeRow));

    // Print marks and colour profile, side by side.
    const two = el('div', undefined, { style: 'display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:20px;align-items:start' });
    const marks = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:2px' });
    marks.appendChild(switchRow(t('exportBleed'), o.bleed, (v) => (o.bleed = v), { focusKey: 'bleed' }));
    marks.appendChild(switchRow(t('exportCropMarks'), o.cropMarks, (v) => (o.cropMarks = v), { focusKey: 'crop' }));
    marks.appendChild(switchRow(t('exportRegMarks'), o.registrationMarks, (v) => (o.registrationMarks = v), { focusKey: 'reg' }));
    two.appendChild(group(t('exportMarks'), marks));
    two.appendChild(group(t('exportProfile'), selectField(t('exportProfile'), [
      { value: 'fogra39', label: t('exportProfileFogra39') },
      { value: 'fogra51', label: t('exportProfileFogra51') },
      { value: 'iso-coated-v2', label: t('exportProfileIsoCoated') },
      { value: 'gracol', label: t('exportProfileGracol') },
      { value: 'swop', label: t('exportProfileSwop') }
    ], o.colorProfile, (v) => (o.colorProfile = v as ExportOptions['colorProfile']), { 'data-focus-key': 'profile', disabled: o.format === 'pdf' })));
    form.appendChild(two);

    // File name and folder (the projects folder unless another one is chosen).
    const fileBox = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:6px' });
    const nameWrap = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px' });
    const nameInput = el('input', 'tok-input', { id: 'tok-export-name', type: 'text', 'data-focus-key': 'file-name', spellcheck: 'false', 'aria-label': t('exportFileName') });
    nameInput.value = o.fileName;
    nameInput.style.flex = '1';
    nameInput.style.minWidth = '0';
    nameInput.addEventListener('input', () => (o.fileName = nameInput.value));
    nameWrap.appendChild(nameInput);
    nameWrap.appendChild(button(t('exportBrowse'), {
      className: 'tok-btn tok-btn-sm',
      icon: 'folder',
      attrs: { 'data-focus-key': 'browse-export-file', title: t('exportChooseLocation') },
      onClick: async () => {
        const host = getHostBridge();
        if (!host.has('showSaveDialog')) return;
        const path = await host.showSaveDialog({
          defaultPath: joinPath(this.saveDir, o.fileName.trim() || defaultExportFileName(this.context.documentTitle)),
          filters: [{ name: 'PDF Document', extensions: ['pdf'] }]
        });
        if (path) {
          this.saveDir = dirName(path) || this.saveDir;
          o.fileName = baseName(path);
          this.render();
        }
      }
    }));
    fileBox.appendChild(nameWrap);
    if (this.saveDir) {
      fileBox.appendChild(el('span', undefined, {
        dir: 'ltr',
        title: this.saveDir,
        style: 'font-size:12px;color:var(--tok-text-secondary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;text-align:start'
      }, `${t('exportSaveIn')}: ${this.saveDir}`));
    }
    form.appendChild(group(t('exportFileName'), fileBox));
    body.appendChild(form);

    // ---- Real page preview and a short print check ----
    const side = el('aside', 'tok-export-side', { 'aria-label': t('exportCheckAria') });
    side.style.width = '290px';
    const preview = el('div', 'tok-export-preview', { role: 'img', 'aria-label': `${t('exportPreview')}: ${this.context.spreadLabels.join(' – ')}` });
    preview.appendChild(this.buildPagePreview());
    side.appendChild(preview);
    side.appendChild(el('span', undefined, { style: 'text-align:center;font-size:12px;color:var(--tok-text-secondary)' }, this.context.spreadLabels.join(' – ')));
    side.appendChild(this.buildCheckSummary());
    body.appendChild(side);
    card.appendChild(body);

    // ---- Footer: actions only ----
    const foot = el('div', 'tok-dialog-foot');
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(t('exportCancel'), { attrs: { 'data-focus-key': 'cancel' }, onClick: () => this.hide() }));
    if (this.callbacks.onPreviewPdf) {
      foot.appendChild(button(t('exportPdfPreviewBtn'), {
        icon: 'eye',
        attrs: { 'data-focus-key': 'preview-pdf' },
        onClick: () => {
          const path = this.outputPath();
          this.hide();
          this.callbacks.onPreviewPdf?.({ ...o, fileName: path });
        }
      }));
    }
    foot.appendChild(button(t('exportGo'), {
      className: 'tok-btn tok-btn-primary',
      icon: 'export',
      attrs: { 'data-focus-key': 'export' },
      onClick: () => {
        const path = this.outputPath();
        this.hide();
        this.callbacks.onExport({ ...o, fileName: path });
      }
    }));
    card.appendChild(foot);

    this.element.appendChild(card);
  }

  /** The current spread as the page viewer draws it, scaled to the side column. */
  private buildPagePreview(): HTMLElement {
    const sheets = this.context.previewSheets ?? [];
    const sizes = sheets.map((s) => ({
      w: Number(s.dataset.widthPt) || s.offsetWidth || 1,
      h: Number(s.dataset.heightPt) || s.offsetHeight || 1
    }));
    if (sheets.length === 0) {
      return el('div', undefined, { style: 'color:var(--tok-text-secondary);font-size:12px;padding:24px 0' }, t('exportPreview'));
    }
    const PT_TO_PX = 96 / 72;
    const totalWidthPx = sizes.reduce((n, s) => n + s.w * PT_TO_PX, 0);
    const scale = Math.min(1, PREVIEW_WIDTH_PX / totalWidthPx);
    const spread = el('div');
    // Right-hand page first: the book reads right to left.
    spread.dir = 'rtl';
    sheets.forEach((sheet, i) => {
      const holder = el('div', undefined, {
        style: `position:relative;flex:none;overflow:hidden;width:${(sizes[i].w * PT_TO_PX * scale).toFixed(2)}px;height:${(sizes[i].h * PT_TO_PX * scale).toFixed(2)}px;`
      });
      sheet.style.position = 'absolute';
      sheet.style.top = '0';
      sheet.style.left = '0';
      sheet.style.margin = '0';
      sheet.style.transformOrigin = '0 0';
      sheet.style.transform = `scale(${scale})`;
      holder.appendChild(sheet);
      spread.appendChild(holder);
    });
    return spread;
  }

  /** One badge, the error/warning counts and a link to the full print-check dialog. */
  private buildCheckSummary(): HTMLElement {
    const report = this.context.preflightReport;
    const status: 'clean' | 'warning' | 'error' = report
      ? report.overall_status === 'Fail'
        ? 'error'
        : report.overall_status === 'Warning'
        ? 'warning'
        : 'clean'
      : this.context.preflightStatus;
    const card = el('div', 'tok-card', { style: 'display:flex;flex-direction:column;gap:8px' });
    const head = el('div', 'tok-check-head');
    head.appendChild(el('span', undefined, undefined, t('exportCheckTitle')));
    const badgeClass = status === 'clean' ? 'tok-badge-success' : status === 'warning' ? 'tok-badge-warning' : 'tok-badge-error';
    const badge = el('span', `tok-badge ${badgeClass}`);
    badge.appendChild(el('span', 'tok-dot', { 'aria-hidden': 'true' }));
    badge.appendChild(el('span', undefined, undefined, status === 'clean' ? t('inspReadyForPrint') : status === 'warning' ? t('exportCheckWarnings') : t('exportCheckErrors')));
    head.appendChild(badge);
    card.appendChild(head);
    if (report && Array.isArray(report.checks) && status !== 'clean') {
      const e = report.checks.filter((c) => c.status === 'Fail').length;
      const w = report.checks.filter((c) => c.status === 'Warning').length;
      card.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, tf('exportCheckCounts', { e, w })));
    }
    if (this.callbacks.onShowPreflight) {
      card.appendChild(button(t('exportCheckDetails'), {
        className: 'tok-btn tok-btn-sm',
        attrs: { 'data-focus-key': 'show-preflight' },
        onClick: () => {
          this.hide();
          this.callbacks.onShowPreflight?.();
        }
      }));
    }
    return card;
  }
}

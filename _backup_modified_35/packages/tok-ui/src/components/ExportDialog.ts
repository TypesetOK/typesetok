import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button, kbd, switchRow, segmented, group, selectField } from '../ui';
import { ModalController } from './ModalController';

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
}

export interface ExportDialogCallbacks {
  onExport: (options: ExportOptions) => void;
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

/** Document page drawing for the preview (same proportions as the start screen). */
function previewPage(rightHand: boolean): HTMLElement {
  const page = el('div', 'tok-page-art', { 'aria-hidden': 'true' });
  page.style.width = '96px';
  page.style.height = '136px';
  page.style.boxShadow = 'none';
  const outer = rightHand ? 'right' : 'left';
  page.innerHTML = `
    <i class="tr" style="top:8%;right:10%;width:80%"></i>
    <i class="tg2" style="top:18%;right:10%;width:80%;height:68%"></i>
    <i class="tr" style="top:90%;${outer}:10%;width:25%"></i>`;
  return page;
}

export class ExportDialog {
  public element: HTMLElement;
  private callbacks: ExportDialogCallbacks;
  private modal: ModalController;
  private isVisible = false;
  private context: ExportDialogContext = { documentTitle: '', pageCount: 0, spreadLabels: [], preflightStatus: 'clean' };
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
    card.style.width = '920px';
    card.style.height = '700px';

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

    // ---- Options ----
    const form = el('div', 'tok-export-form');

    const formats = el('div', undefined, { role: 'radiogroup', 'aria-label': t('exportFormat'), style: 'display:flex;flex-direction:column;gap:8px' });
    for (const f of FORMATS) {
      const on = o.format === f.id;
      const b = el('button', 'tok-radio-card', { type: 'button', role: 'radio', 'aria-checked': String(on), 'data-focus-key': `format-${f.id}` });
      b.appendChild(el('span', 'tok-radio-dot', { 'aria-hidden': 'true' }));
      const text = el('span', undefined, { style: 'flex:1;display:flex;flex-direction:column;gap:3px' });
      const title = el('span', 'tok-format-title');
      title.appendChild(el('b', undefined, undefined, f.code));
      title.appendChild(el('span', undefined, undefined, `· ${t(f.titleKey)}`));
      if (f.recommended) title.appendChild(el('span', 'tok-badge tok-badge-success', { style: 'font-size:11px;font-weight:600' }, t('exportRecommended')));
      text.appendChild(title);
      text.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, t(f.descKey)));
      b.appendChild(text);
      b.addEventListener('click', () => {
        o.format = f.id;
        this.render();
      });
      formats.appendChild(b);
    }
    form.appendChild(group(t('exportFormat'), formats));

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

    // Marks & color profile
    const two = el('div', undefined, { style: 'display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px' });
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
    body.appendChild(form);

    // ---- Preview & print check ----
    const side = el('aside', 'tok-export-side', { 'aria-label': t('exportCheckAria') });
    const preview = el('div', 'tok-export-preview', { role: 'img', 'aria-label': `${t('exportPreview')}: ${this.context.spreadLabels.join(' – ')}` });
    const spread = el('div');
    // Right-hand page first (the preview is laid out right to left like the book).
    spread.dir = 'rtl';
    if (this.context.spreadLabels.length > 1) spread.appendChild(previewPage(true));
    spread.appendChild(previewPage(false));
    preview.appendChild(spread);
    side.appendChild(preview);
    side.appendChild(el('span', undefined, { style: 'text-align:center;font-size:12px;color:var(--tok-text-secondary)' }, this.context.spreadLabels.join(' – ')));

    const status = this.context.preflightStatus;
    const check = el('div', 'tok-card', { style: 'display:flex;flex-direction:column;gap:10px' });
    const checkHead = el('div', 'tok-check-head');
    checkHead.appendChild(el('span', undefined, undefined, t('exportCheckTitle')));
    const badgeClass = status === 'clean' ? 'tok-badge-success' : status === 'warning' ? 'tok-badge-warning' : 'tok-badge-error';
    const badge = el('span', `tok-badge ${badgeClass}`);
    badge.appendChild(el('span', 'tok-dot', { 'aria-hidden': 'true' }));
    badge.appendChild(el('span', undefined, undefined, status === 'clean' ? t('inspReadyForPrint') : status === 'warning' ? t('exportCheckWarnings') : t('exportCheckErrors')));
    checkHead.appendChild(badge);
    check.appendChild(checkHead);
    const list = el('ul', 'tok-check-list');
    for (const key of ['exportCheckK', 'exportCheckFonts', 'exportCheckOverset', 'exportCheckImages']) {
      const ok = status === 'clean';
      const li = el('li');
      const mark = el('span', ok ? 'tok-ok' : 'tok-warn');
      mark.appendChild(icon(ok ? 'check' : 'warning', 15));
      li.appendChild(mark);
      li.appendChild(el('span', undefined, undefined, t(key)));
      list.appendChild(li);
    }
    check.appendChild(list);
    side.appendChild(check);
    body.appendChild(side);
    card.appendChild(body);

    // ---- Footer ----
    const foot = el('div', 'tok-dialog-foot');
    const nameLabel = el('label', undefined, { for: 'tok-export-name', style: 'font-size:12px;color:var(--tok-text-secondary)' }, t('exportFileName'));
    const nameWrap = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px' });
    const nameInput = el('input', 'tok-input', { id: 'tok-export-name', type: 'text', 'data-focus-key': 'file-name', spellcheck: 'false' });
    nameInput.value = o.fileName;
    nameInput.style.width = '240px';
    nameInput.addEventListener('input', () => (o.fileName = nameInput.value));
    nameWrap.appendChild(nameInput);
    nameWrap.appendChild(button(t('exportBrowse'), {
      className: 'tok-btn tok-btn-sm',
      icon: 'folder',
      attrs: { 'data-focus-key': 'browse-export-file', title: t('exportChooseLocation') },
      onClick: async () => {
        const win = window as any;
        if (win.tokIpc?.showSaveDialog) {
          const path = await win.tokIpc.showSaveDialog({
            defaultPath: o.fileName || defaultExportFileName(this.context.documentTitle),
            filters: [{ name: 'PDF Document', extensions: ['pdf'] }]
          });
          if (path) {
            o.fileName = path;
            nameInput.value = path;
          }
        }
      }
    }));
    foot.appendChild(nameWrap);
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(t('exportCancel'), { attrs: { 'data-focus-key': 'cancel' }, onClick: () => this.hide() }));
    foot.appendChild(button(t('exportGo'), {
      className: 'tok-btn tok-btn-primary',
      icon: 'export',
      attrs: { 'data-focus-key': 'export' },
      onClick: () => {
        let name = o.fileName.trim() || defaultExportFileName(this.context.documentTitle);
        if (!/\.pdf$/i.test(name)) name += '.pdf';
        o.fileName = name;
        o.fromPage = Math.min(o.fromPage, o.toPage);
        o.toPage = Math.max(o.fromPage, o.toPage);
        this.hide();
        this.callbacks.onExport({ ...o });
      }
    }));
    card.appendChild(foot);

    this.element.appendChild(card);
  }
}

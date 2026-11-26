import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button, kbd, switchRow } from '../ui';
import { ModalController } from './ModalController';
import { CleanupOptions, DEFAULT_CLEANUP_OPTIONS, analyzeParagraphs } from '../engine/textCleanup';
import type { StoryParagraph } from 'tok-story-editor';

export interface TextCleanupContext {
  activeFlowId: string;
  activeFlowDisplayName: string;
  getParagraphs: (scope: 'activeFlow' | 'allFlows') => StoryParagraph[];
  initialOptions?: Partial<CleanupOptions>;
}

export interface TextCleanupModalCallbacks {
  onApply: (options: CleanupOptions) => void;
  onOptionsChange?: (options: CleanupOptions) => void;
  onClose?: () => void;
}

export class TextCleanupModal {
  public element: HTMLElement;
  private callbacks: TextCleanupModalCallbacks;
  private isVisible = false;
  private modal: ModalController;
  private context: TextCleanupContext | null = null;
  private options: CleanupOptions = { ...DEFAULT_CLEANUP_OPTIONS };
  private summaryEl: HTMLElement | null = null;

  constructor(callbacks: TextCleanupModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-cleanup-overlay');
    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(context: TextCleanupContext): void {
    this.context = context;
    this.options = {
      ...DEFAULT_CLEANUP_OPTIONS,
      ...(context.initialOptions || {})
    };
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
    this.callbacks.onOptionsChange?.({ ...this.options });
    this.callbacks.onClose?.();
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.renderContent();
    this.modal.afterRender(focusKey);
  }

  private renderContent(): void {
    this.element.innerHTML = '';
    const isEn = i18n.getLanguage() === 'en';
    this.element.dir = isEn ? 'ltr' : 'rtl';

    const card = el('div', 'tok-dialog tok-cleanup-card');
    card.style.width = '520px';
    card.style.maxHeight = '90vh';
    card.style.display = 'flex';
    card.style.flexDirection = 'column';

    // Header
    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, t('textCleanupTitle')));
    headText.appendChild(el('p', 'tok-dialog-subtitle', undefined, t('textCleanupSubtitle')));
    head.appendChild(headText);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', t('textCleanupCancel'), () => this.hide(), {
      attrs: { 'data-focus-key': 'close' }
    }));
    card.appendChild(head);

    // Body
    const body = el('div', 'tok-dialog-body', {
      style: 'padding:16px 22px;display:flex;flex-direction:column;gap:14px;overflow-y:auto;'
    });

    // 1. Scope selection
    const scopeBox = el('div', 'tok-cleanup-section', {
      style: 'background:var(--tok-bg-surface-2, #1E293B);border:1px solid var(--tok-border-subtle);border-radius:6px;padding:10px 14px;display:flex;flex-direction:column;gap:8px;'
    });
    scopeBox.appendChild(el('span', undefined, { style: 'font-weight:600;font-size:12px;color:var(--tok-text-primary);' }, t('textCleanupScope')));

    const scopeRow = el('div', undefined, { style: 'display:flex;gap:16px;font-size:12px;' });

    const activeFlowLabel = `${t('textCleanupScopeActive')} (${this.context?.activeFlowDisplayName || ''})`;
    const activeRadio = this.createRadio('tok-cleanup-scope', 'activeFlow', this.options.scope === 'activeFlow', activeFlowLabel, (v) => {
      if (v) {
        this.options.scope = 'activeFlow';
        this.updateLiveSummary();
      }
    });

    const allRadio = this.createRadio('tok-cleanup-scope', 'allFlows', this.options.scope === 'allFlows', t('textCleanupScopeAll'), (v) => {
      if (v) {
        this.options.scope = 'allFlows';
        this.updateLiveSummary();
      }
    });

    scopeRow.appendChild(activeRadio);
    scopeRow.appendChild(allRadio);
    scopeBox.appendChild(scopeRow);
    body.appendChild(scopeBox);

    // 2. Standard typography cleanups
    const cleanupsBox = el('div', 'tok-cleanup-section', {
      style: 'display:flex;flex-direction:column;gap:6px;'
    });

    cleanupsBox.appendChild(
      switchRow(t('textCleanupDoubleSpaces'), this.options.cleanDoubleSpaces, (val) => {
        this.options.cleanDoubleSpaces = val;
        this.updateLiveSummary();
      })
    );

    cleanupsBox.appendChild(
      switchRow(t('textCleanupSpacesBeforePunct'), this.options.cleanSpacesBeforePunct, (val) => {
        this.options.cleanSpacesBeforePunct = val;
        this.updateLiveSummary();
      })
    );

    cleanupsBox.appendChild(
      switchRow(t('textCleanupSpacesAfterPunct'), this.options.ensureSpaceAfterPunct, (val) => {
        this.options.ensureSpaceAfterPunct = val;
        this.updateLiveSummary();
      })
    );

    cleanupsBox.appendChild(
      switchRow(t('textCleanupQuotes'), this.options.normalizeQuotesAndGeresh, (val) => {
        this.options.normalizeQuotesAndGeresh = val;
        this.updateLiveSummary();
      })
    );

    cleanupsBox.appendChild(
      switchRow(t('textCleanupEmptyParagraphs'), this.options.collapseEmptyParagraphs, (val) => {
        this.options.collapseEmptyParagraphs = val;
        this.updateLiveSummary();
      })
    );

    body.appendChild(cleanupsBox);

    // 3. Dibur HaMatchil section
    const diburBox = el('div', 'tok-cleanup-section', {
      style: 'background:var(--tok-bg-surface-2, #1E293B);border:1px solid var(--tok-border-subtle);border-radius:6px;padding:12px 14px;display:flex;flex-direction:column;gap:10px;'
    });

    diburBox.appendChild(
      switchRow(t('textCleanupDiburTitle'), this.options.boldDiburHaMatchil, (val) => {
        this.options.boldDiburHaMatchil = val;
        delimRow.style.display = val ? 'flex' : 'none';
        this.updateLiveSummary();
      }, { description: t('textCleanupDiburDesc') })
    );

    const delimRow = el('div', undefined, {
      style: `display:${this.options.boldDiburHaMatchil ? 'flex' : 'none'};align-items:center;gap:10px;font-size:12px;padding-top:4px;`
    });
    delimRow.appendChild(el('span', undefined, { style: 'color:var(--tok-text-muted);' }, t('textCleanupDelimiter')));

    const delimSelect = el('select', 'tok-select tok-select-sm', {
      style: 'background:var(--tok-surface-1);color:var(--tok-text-primary);border:1px solid var(--tok-border);border-radius:4px;padding:2px 8px;font-size:12px;'
    }) as HTMLSelectElement;

    const delims: { id: CleanupOptions['diburDelimiter']; label: string }[] = [
      { id: 'any', label: t('textCleanupDelimAny') },
      { id: 'period', label: t('textCleanupDelimPeriod') },
      { id: 'colon', label: t('textCleanupDelimColon') },
      { id: 'dash', label: t('textCleanupDelimDash') }
    ];

    for (const d of delims) {
      const opt = el('option', undefined, { value: d.id }, d.label) as HTMLOptionElement;
      if (d.id === this.options.diburDelimiter) opt.selected = true;
      delimSelect.appendChild(opt);
    }

    delimSelect.addEventListener('change', () => {
      this.options.diburDelimiter = delimSelect.value as CleanupOptions['diburDelimiter'];
      this.updateLiveSummary();
    });

    delimRow.appendChild(delimSelect);
    diburBox.appendChild(delimRow);
    body.appendChild(diburBox);

    // 4. Live summary status
    this.summaryEl = el('div', 'tok-cleanup-summary', {
      style: 'font-size:12px;color:var(--tok-text-secondary);background:var(--tok-surface-3, rgba(255,255,255,0.05));border-radius:4px;padding:8px 12px;display:flex;align-items:center;gap:8px;'
    });
    body.appendChild(this.summaryEl);

    card.appendChild(body);

    // Footer actions
    const footer = el('div', 'tok-dialog-foot', {
      style: 'padding:14px 22px;border-top:1px solid var(--tok-border-subtle);display:flex;justify-content:flex-end;gap:10px;'
    });

    footer.appendChild(button(t('textCleanupCancel'), {
      className: 'tok-btn tok-btn-ghost tok-btn-sm',
      onClick: () => this.hide()
    }));

    const applyBtn = button(t('textCleanupApply'), {
      className: 'tok-btn tok-btn-primary tok-btn-sm',
      icon: 'sparkle',
      iconSize: 14,
      onClick: () => {
        this.callbacks.onApply({ ...this.options });
        this.hide();
      }
    });
    footer.appendChild(applyBtn);

    card.appendChild(footer);
    this.element.appendChild(card);

    this.updateLiveSummary();
  }

  private createRadio(name: string, value: string, checked: boolean, label: string, onChange: (checked: boolean) => void): HTMLElement {
    const wrapper = el('label', undefined, { style: 'display:inline-flex;align-items:center;gap:6px;cursor:pointer;' });
    const input = el('input', undefined, {
      type: 'radio',
      name,
      value
    }) as HTMLInputElement;
    input.checked = checked;
    input.addEventListener('change', () => onChange(input.checked));
    wrapper.appendChild(input);
    wrapper.appendChild(el('span', undefined, undefined, label));
    return wrapper;
  }

  private updateLiveSummary(): void {
    if (!this.summaryEl || !this.context) return;
    const paras = this.context.getParagraphs(this.options.scope);
    const stats = analyzeParagraphs(paras, this.options);

    this.summaryEl.innerHTML = '';
    const iconSpan = icon('sparkle', 14);
    this.summaryEl.appendChild(iconSpan);

    if (stats.totalModifications === 0) {
      this.summaryEl.appendChild(el('span', undefined, undefined, t('textCleanupSummaryZero')));
    } else {
      const parts: string[] = [];
      if (stats.doubleSpacesFixed > 0) parts.push(`${stats.doubleSpacesFixed} ${t('textCleanupDoubleSpaces')}`);
      if (stats.quotesConverted + stats.gereshConverted > 0) parts.push(`${stats.quotesConverted + stats.gereshConverted} ${t('textCleanupQuotes')}`);
      if (stats.diburHaMatchilBolded > 0) parts.push(`${stats.diburHaMatchilBolded} ${t('textCleanupDiburTitle')}`);

      const summaryText = tf('textCleanupSummary', { count: stats.totalModifications }) + (parts.length > 0 ? ` (${parts.join(', ')})` : '');
      this.summaryEl.appendChild(el('span', undefined, undefined, summaryText));
    }
  }
}

import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button, kbd } from '../ui';
import { ModalController } from './ModalController';

export interface TextStatsData {
  pageWords: number;
  totalWords: number;
  charsWithSpaces: number;
  charsWithoutSpaces: number;
  paragraphs: number;
  linesEstimate: number;
  hebrewChars: number;
  niqqudCount: number;
  pageNumber: string;
}

export interface TextStatsModalCallbacks {
  onClose: () => void;
}

export class TextStatsModal {
  public element: HTMLElement;
  private callbacks: TextStatsModalCallbacks;
  private isVisible = false;
  private modal: ModalController;
  private currentData: TextStatsData | null = null;

  constructor(callbacks: TextStatsModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-stats-overlay');

    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(data: TextStatsData): void {
    this.currentData = data;
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
    this.callbacks.onClose();
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

    const card = el('div', 'tok-dialog tok-stats-card');
    card.style.width = '460px';
    card.style.maxHeight = '90vh';
    card.style.display = 'flex';
    card.style.flexDirection = 'column';

    // Header
    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, isEn ? 'Text & Word Statistics' : 'סטטיסטיקת טקסט ומילים'));
    head.appendChild(headText);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', isEn ? 'Close' : 'סגירה', () => this.hide(), {
      attrs: { 'data-focus-key': 'close' }
    }));
    card.appendChild(head);

    // Body
    const body = el('div', 'tok-dialog-body', {
      style: 'padding:20px 24px;display:flex;flex-direction:column;gap:16px;'
    });

    const d = this.currentData || {
      pageWords: 0,
      totalWords: 0,
      charsWithSpaces: 0,
      charsWithoutSpaces: 0,
      paragraphs: 0,
      linesEstimate: 0,
      hebrewChars: 0,
      niqqudCount: 0,
      pageNumber: 'א׳'
    };

    // Quick stats grid
    const grid = el('div', undefined, {
      style: 'display:grid;grid-template-columns:1fr 1fr;gap:10px;'
    });

    const statBox = (label: string, value: number | string, sub?: string) => {
      const b = el('div', undefined, {
        style: 'background:var(--tok-bg-surface-1);border:1px solid var(--tok-border-subtle);border-radius:10px;padding:12px;display:flex;flex-direction:column;gap:4px;'
      });
      b.appendChild(el('span', undefined, { style: 'font-size:11px;color:var(--tok-text-muted);font-weight:500' }, label));
      b.appendChild(el('strong', undefined, { style: 'font-size:20px;font-weight:700;color:var(--tok-accent-text)' }, typeof value === 'number' ? value.toLocaleString() : value));
      if (sub) {
        b.appendChild(el('span', undefined, { style: 'font-size:11px;color:var(--tok-text-secondary)' }, sub));
      }
      return b;
    };

    grid.appendChild(statBox(isEn ? 'Words on current page' : 'מילים בעמוד הנוכחי', d.pageWords, `${isEn ? 'Page' : 'דף'} ${d.pageNumber}`));
    grid.appendChild(statBox(isEn ? 'Total document words' : 'סה״כ מילים במסמך', d.totalWords));
    grid.appendChild(statBox(isEn ? 'Characters (with spaces)' : 'תווים כולל רווחים', d.charsWithSpaces));
    grid.appendChild(statBox(isEn ? 'Characters (no spaces)' : 'תווים ללא רווחים', d.charsWithoutSpaces));
    grid.appendChild(statBox(isEn ? 'Paragraphs' : 'מספר פסקאות', d.paragraphs));
    grid.appendChild(statBox(isEn ? 'Estimated lines' : 'מספר שורות משוער', d.linesEstimate));
    grid.appendChild(statBox(isEn ? 'Hebrew letters' : 'אותיות עבריות', d.hebrewChars));
    grid.appendChild(statBox(isEn ? 'Niqqud & Cantillation' : 'סימני ניקוד וטעמים', d.niqqudCount));

    body.appendChild(grid);
    card.appendChild(body);

    // Footer
    const foot = el('div', 'tok-dialog-foot');
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(isEn ? 'Close' : 'סגירה', {
      className: 'tok-btn tok-btn-primary',
      attrs: { 'data-focus-key': 'done' },
      onClick: () => this.hide()
    }));
    card.appendChild(foot);

    this.element.appendChild(card);
  }
}

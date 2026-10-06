import { t, tf, i18n } from '../i18n';
import { el, icon } from '../ui';

export interface StatusBarCallbacks {
  onZoomChange: (zoomPercent: number) => void;
  onPageClick?: () => void;
  onPreflightClick: () => void;
  onWordCountClick?: () => void;
}

export const ZOOM_PRESETS = [50, 75, 100, 125, 150, 200];

export class StatusBar {
  public element: HTMLElement;
  private callbacks: StatusBarCallbacks;
  private pageLabel = '';
  private zoom = 100;
  private wordCount = 0;
  private activeFlow = '';
  private preflightStatus: 'clean' | 'warning' | 'error' = 'clean';

  // Live nodes updated in place by updateStats().
  private pageText!: HTMLElement;
  private countText!: HTMLElement;
  private flowText!: HTMLElement;
  private pfItem!: HTMLElement;
  private pfText!: HTMLElement;
  private zoomSelect!: HTMLSelectElement;

  constructor(callbacks: StatusBarCallbacks) {
    this.callbacks = callbacks;
    this.element = document.createElement('footer');
    this.element.className = 'tok-status-bar';
    this.element.setAttribute('role', 'status');
    this.element.dir = i18n.getDirection();

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      this.build();
    });

    this.build();
  }

  public updateStats(params: {
    pageLabel?: string;
    zoom?: number;
    wordCount?: number;
    activeFlow?: string;
    preflightStatus?: 'clean' | 'warning' | 'error';
  }): void {
    if (params.pageLabel !== undefined) this.pageLabel = params.pageLabel;
    if (params.zoom !== undefined) this.zoom = params.zoom;
    if (params.wordCount !== undefined) this.wordCount = params.wordCount;
    if (params.activeFlow !== undefined) this.activeFlow = params.activeFlow;
    if (params.preflightStatus !== undefined) this.preflightStatus = params.preflightStatus;
    this.refresh();
  }

  public getPreflightStatus(): 'clean' | 'warning' | 'error' {
    return this.preflightStatus;
  }

  /** Writes the current state into the existing nodes. All values go in as text, never HTML. */
  private refresh(): void {
    this.pageText.textContent = this.pageLabel;
    this.countText.textContent = `${this.wordCount.toLocaleString()} ${t('statusWords')}`;
    this.countText.title = t('statusTextLength');
    this.flowText.textContent = this.activeFlow;

    const cls = { clean: 'tok-status-ok', warning: 'tok-status-warn', error: 'tok-status-err' }[this.preflightStatus];
    this.pfItem.className = `tok-status-btn ${cls}`;
    this.pfText.textContent =
      this.preflightStatus === 'clean' ? t('statusPreflightClean')
        : this.preflightStatus === 'warning' ? t('statusPreflightWarnings')
          : t('statusPreflightErrors');

    // A zoom value that is not a preset (e.g. "fit to window") still needs an option to show.
    this.zoomSelect.querySelectorAll('option[data-custom]').forEach((o) => {
      if ((o as HTMLOptionElement).value !== String(this.zoom)) o.remove();
    });
    if (!ZOOM_PRESETS.includes(this.zoom) && !this.zoomSelect.querySelector(`option[value="${this.zoom}"]`)) {
      const opt = this.createZoomOption(this.zoom);
      opt.dataset.custom = '';
      const after = Array.from(this.zoomSelect.options).find((o) => parseInt(o.value, 10) > this.zoom);
      this.zoomSelect.insertBefore(opt, after ?? null);
    }
    this.zoomSelect.value = String(this.zoom);
  }

  private createZoomOption(z: number): HTMLOptionElement {
    return el('option', undefined, { value: String(z) }, `${z}%`);
  }

  private build(): void {
    this.element.innerHTML = '';

    // Page indicator (display only, does NOT open command search)
    const pageItem = el('div', 'tok-status-btn tok-status-static', { title: tf('structurePageLabel', { g: this.pageLabel }) });
    pageItem.appendChild(icon('pages', 14));
    this.pageText = el('span');
    pageItem.appendChild(this.pageText);
    this.element.appendChild(pageItem);

    // Word count button (opens detailed text statistics modal)
    const countBtn = el('button', 'tok-status-btn', {
      type: 'button',
      title: 'סטטיסטיקת מילים ותווים במסמך',
      style: 'cursor:pointer'
    });
    this.countText = el('span');
    countBtn.appendChild(this.countText);
    countBtn.addEventListener('click', () => {
      this.callbacks.onWordCountClick?.();
    });
    this.element.appendChild(countBtn);

    const flowItem = el('span');
    flowItem.appendChild(el('span', undefined, undefined, `${t('statusActiveFlow')}: `));
    this.flowText = el('strong', undefined, { style: 'font-weight:600;color:var(--tok-text-primary)' });
    flowItem.appendChild(this.flowText);
    this.element.appendChild(flowItem);

    this.element.appendChild(el('span', 'tok-grow'));

    this.element.appendChild(el('span', undefined, undefined, t('statusJustificationRules')));

    this.pfItem = el('button', 'tok-status-btn', { type: 'button' });
    this.pfItem.appendChild(el('span', 'tok-dot', { 'aria-hidden': 'true' }));
    this.pfText = el('span', undefined, { style: 'font-weight:500' });
    this.pfItem.appendChild(this.pfText);
    this.pfItem.addEventListener('click', () => this.callbacks.onPreflightClick());
    this.element.appendChild(this.pfItem);

    this.zoomSelect = el('select', 'tok-status-btn', { 'aria-label': t('statusZoom'), title: t('statusZoom'), style: 'cursor:pointer' });
    for (const z of ZOOM_PRESETS) this.zoomSelect.appendChild(this.createZoomOption(z));
    this.zoomSelect.addEventListener('change', () => {
      const z = parseInt(this.zoomSelect.value, 10);
      if (!Number.isFinite(z)) return;
      this.zoom = z;
      this.callbacks.onZoomChange(z);
    });
    this.element.appendChild(this.zoomSelect);

    this.refresh();
  }
}

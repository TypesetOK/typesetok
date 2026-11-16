import { t, tf, i18n } from '../i18n';
import { el, icon, button } from '../ui';

export interface StatusBarCallbacks {
  onZoomChange: (zoomPercent: number) => void;
  onPageClick?: () => void;
  onPreflightClick: () => void;
  onWordCountClick?: () => void;
  onExportPdf?: () => void;
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
  private busyItem!: HTMLElement;
  private busyText!: HTMLElement;
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

  private temporaryStatusTimer: ReturnType<typeof setTimeout> | null = null;

  public setBusy(busy: boolean, label?: string): void {
    if (!this.busyItem || !this.busyText) return;
    if (this.temporaryStatusTimer) {
      clearTimeout(this.temporaryStatusTimer);
      this.temporaryStatusTimer = null;
    }
    this.busyItem.style.display = busy ? 'inline-flex' : 'none';
    if (label) {
      this.busyText.textContent = label;
    }
  }

  public setTemporaryStatus(label: string, durationMs = 2000): void {
    if (!this.busyItem || !this.busyText) return;
    if (this.temporaryStatusTimer) {
      clearTimeout(this.temporaryStatusTimer);
    }
    this.busyText.textContent = label;
    this.busyItem.style.display = 'inline-flex';
    this.temporaryStatusTimer = setTimeout(() => {
      if (this.busyItem) {
        this.busyItem.style.display = 'none';
      }
      this.temporaryStatusTimer = null;
    }, durationMs);
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

    // Page indicator: opens the page view (it does NOT open command search)
    const pageItem = el('button', 'tok-status-btn', {
      type: 'button',
      title: tf('structurePageLabel', { g: this.pageLabel }),
      style: 'cursor:pointer'
    });
    pageItem.appendChild(icon('pages', 14));
    this.pageText = el('span');
    pageItem.appendChild(this.pageText);
    pageItem.addEventListener('click', () => {
      this.callbacks.onPageClick?.();
    });
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

    this.busyItem = el('span', 'tok-status-btn', {
      style: 'display:none;align-items:center;gap:4px;color:var(--tok-accent-text);'
    });
    const spinner = el('span', 'tok-spinner', {
      'aria-hidden': 'true',
      style: 'width:10px;height:10px;border:2px solid currentColor;border-top-color:transparent;border-radius:50%;display:inline-block;animation:spin 0.8s linear infinite;'
    });
    this.busyText = el('span', undefined, { style: 'font-size:11px;' }, 'מעבד...');
    this.busyItem.appendChild(spinner);
    this.busyItem.appendChild(this.busyText);
    this.element.appendChild(this.busyItem);

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

    if (this.callbacks.onExportPdf) {
      const exportBtn = button(t('topBarExportPdf'), {
        className: 'tok-btn tok-btn-primary tok-btn-sm tok-status-export-btn',
        icon: 'export',
        attrs: { 'aria-keyshortcuts': 'Control+E', title: `${t('topBarExportPdf')} (Ctrl+E)` },
        onClick: () => this.callbacks.onExportPdf?.()
      });
      exportBtn.style.height = '24px';
      exportBtn.style.padding = '0 10px';
      exportBtn.style.fontSize = '12px';
      this.element.appendChild(exportBtn);
    }

    this.refresh();
  }
}

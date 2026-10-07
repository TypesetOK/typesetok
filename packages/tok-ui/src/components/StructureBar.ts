import { MultiFlowItem, StyleToken, PageThumbnailItem } from '../types';
import { t, tf, i18n } from '../i18n';
import { IconName } from '../icons';
import { el, icon, iconButton } from '../ui';
import { groupIntoSpreads, isRectoPage, isRightHandPage } from './SpreadCanvas';
import { FONT_WEIGHT_BOLD, FONT_WEIGHT_REGULAR } from '../fonts';

export interface StructureBarCallbacks {
  onSelectPage: (pageIndex: number) => void;
  onAddPage: () => void;
  onSelectFlow: (flowId: string) => void;
  onSelectStyle: (styleId: string) => void;
  onToggleLayer: (layerId: string, visible: boolean) => void;
  onOpenSettings?: () => void;
  onOpenAbout?: () => void;
  /** The side panel was opened or closed from the rail. */
  onPanelToggle?: (open: boolean) => void;
  onActiveTabChange?: (tab: TabId, open: boolean) => void;
}

type TabId = 'pages' | 'flows' | 'styles' | 'layers';

/** Mini drawing of a page sheet with header rule and text frame boundaries. */
function pageThumbDrawing(_rightHandPage: boolean): string {
  return `
    <i class="tr" style="top:5px;left:5px;right:5px;height:1px;opacity:0.6"></i>
    <div style="position:absolute;top:10px;bottom:6px;left:6px;right:6px;border:1px dashed var(--tok-border-strong);border-radius:2px;opacity:0.4;"></div>`;
}

/**
 * Leading side of the workbench: an icon rail (pages, flows, styles, layers, settings,
 * about) and the panel it controls. Clicking the active rail item hides the panel.
 */
export class StructureBar {
  public element: HTMLElement;
  private callbacks: StructureBarCallbacks;
  private activeTab: TabId = 'pages';
  private panelOpen = true;
  private activePageIndex = 0;
  private activeFlowId = 'gemara';
  private pages: PageThumbnailItem[] = [];
  private rail!: HTMLElement;
  private panel!: HTMLElement;

  private flows: MultiFlowItem[] = [
    { id: 'gemara', name: 'גמרא (ראשי)', color: '#1E4A9E', role: 'מרכז העמוד', wordCount: 0, isActive: true },
    { id: 'rashi', name: 'רש"י', color: '#B45309', role: 'פירוש פנימי', wordCount: 0, isActive: false },
    { id: 'tosafot', name: 'תוספות', color: '#15803D', role: 'פירוש חיצוני', wordCount: 0, isActive: false },
    { id: 'notes', name: 'הערות שוליים', color: '#7C3AED', role: 'תחתית העמוד', wordCount: 0, isActive: false }
  ];

  private styles: StyleToken[] = [
    { id: 'style-gemara-heading', name: 'כותרת פרק', fontFamily: 'Frank Ruhl Libre', fontSizePt: 20, fontWeight: FONT_WEIGHT_BOLD, flowId: 'gemara' },
    { id: 'style-gemara-main', name: 'גמרא ראשי', fontFamily: 'Frank Ruhl Libre', fontSizePt: 15, fontWeight: FONT_WEIGHT_BOLD, flowId: 'gemara' },
    { id: 'style-rashi-body', name: 'רש"י רציף', fontFamily: 'Noto Rashi Hebrew', fontSizePt: 12, fontWeight: FONT_WEIGHT_REGULAR, flowId: 'rashi' },
    { id: 'style-tosafot-body', name: 'תוספות רציף', fontFamily: 'Noto Rashi Hebrew', fontSizePt: 11.5, fontWeight: FONT_WEIGHT_REGULAR, flowId: 'tosafot' },
    { id: 'style-dibur-hamatchil', name: 'דיבור המתחיל', fontFamily: 'Frank Ruhl Libre', fontSizePt: 12.5, fontWeight: FONT_WEIGHT_BOLD, flowId: 'rashi' },
    { id: 'style-footnotes', name: 'הערות שוליים', fontFamily: 'Frank Ruhl Libre', fontSizePt: 10, fontWeight: FONT_WEIGHT_REGULAR, flowId: 'notes' }
  ];

  private layers: { id: string; name: string; visible: boolean; locked: boolean }[] = [
    { id: 'text-main', name: 'שכבת טקסט ראשי', visible: true, locked: false },
    { id: 'text-commentary', name: 'שכבת פירושים', visible: true, locked: false },
    { id: 'decorations', name: 'עיטורים ומסגרות', visible: true, locked: false },
    { id: 'guides', name: 'קווי עזר ושוליים', visible: true, locked: true }
  ];

  constructor(callbacks: StructureBarCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-structure');
    this.element.style.display = 'flex';
    this.element.style.flex = 'none';
    this.element.style.minHeight = '0';
    this.element.dir = i18n.getDirection();

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      this.render();
    });

    this.render();
  }

  public getActiveTab(): TabId {
    return this.activeTab;
  }

  public setPages(pages: PageThumbnailItem[], activeIndex = 0): void {
    this.pages = pages;
    this.activePageIndex = activeIndex;
    if (this.activeTab === 'pages') this.renderPanel();
  }

  public setActivePage(pageIndex: number): void {
    this.activePageIndex = pageIndex;
    if (this.activeTab === 'pages') this.highlightActivePage();
  }

  /** Mirrors the flow chosen elsewhere (e.g. the story editor's flow chip). */
  public setActiveFlow(flowId: string): void {
    this.activeFlowId = flowId;
    this.flows.forEach((f) => (f.isActive = f.id === flowId));
    if (this.activeTab === 'flows') this.renderPanel();
  }

  public updateFlowWordCounts(counts: Record<string, number>): void {
    for (const f of this.flows) {
      if (typeof counts[f.id] === 'number') {
        f.wordCount = counts[f.id];
      }
    }
    if (this.activeTab === 'flows') this.renderPanel();
  }

  public getFlows(): MultiFlowItem[] {
    return this.flows.map((f) => ({ ...f }));
  }

  /** Opens/closes the side panel (the rail always stays). */
  public setPanelOpen(open: boolean): void {
    this.panelOpen = open;
    this.panel.hidden = !open;
    this.updateRail();
  }

  public isPanelOpen(): boolean {
    return this.panelOpen;
  }

  /** Opens the panel on a given tab. */
  public showTab(tab: TabId): void {
    this.activeTab = tab;
    this.setPanelOpen(true);
    this.renderPanel();
  }

  private render(): void {
    this.element.innerHTML = '';

    // ---- Rail ----
    this.rail = el('nav', 'tok-rail', { 'aria-label': t('railLabel') });
    const tabs: { id: TabId; label: string; icon: IconName }[] = [
      { id: 'pages', label: t('sidebarPages'), icon: 'pages' },
      { id: 'flows', label: t('sidebarFlows'), icon: 'flows' },
      { id: 'styles', label: t('sidebarStyles'), icon: 'typography' },
      { id: 'layers', label: t('sidebarLayers'), icon: 'layers' }
    ];
    for (const tab of tabs) {
      const b = el('button', 'tok-rail-btn', { type: 'button', 'data-tab': tab.id, 'aria-controls': 'tok-side-panel' });
      b.appendChild(icon(tab.icon, 20));
      b.appendChild(el('span', undefined, undefined, tab.label));
      b.addEventListener('click', () => {
        if (this.activeTab === tab.id && this.panelOpen) {
          this.setPanelOpen(false);
          this.callbacks.onPanelToggle?.(false);
          this.callbacks.onActiveTabChange?.(this.activeTab, false);
          return;
        }
        const wasOpen = this.panelOpen;
        this.activeTab = tab.id;
        this.setPanelOpen(true);
        this.renderPanel();
        if (!wasOpen) this.callbacks.onPanelToggle?.(true);
        this.callbacks.onActiveTabChange?.(this.activeTab, true);
      });
      this.rail.appendChild(b);
    }
    this.rail.appendChild(el('div', 'tok-rail-spacer'));

    const bottom = el('div', 'tok-structure-bottom');

    const settings = el('button', 'tok-rail-btn', {
      type: 'button',
      title: t('sidebarSettings'),
      'aria-label': t('sidebarSettings')
    });
    settings.appendChild(icon('settings', 20));
    settings.appendChild(el('span', undefined, undefined, t('sidebarSettings')));
    settings.addEventListener('click', () => this.callbacks.onOpenSettings?.());
    bottom.appendChild(settings);

    const about = el('button', 'tok-rail-btn', {
      type: 'button',
      title: t('sidebarAbout'),
      'aria-label': t('sidebarAbout')
    });
    about.appendChild(icon('info', 20));
    about.appendChild(el('span', undefined, undefined, t('sidebarAbout')));
    about.addEventListener('click', () => this.callbacks.onOpenAbout?.());
    bottom.appendChild(about);
    this.rail.appendChild(bottom);

    this.element.appendChild(this.rail);

    // ---- Panel ----
    this.panel = el('aside', 'tok-structure-bar', { id: 'tok-side-panel' });
    this.panel.hidden = !this.panelOpen;
    this.element.appendChild(this.panel);

    this.updateRail();
    this.renderPanel();
  }

  private updateRail(): void {
    this.rail?.querySelectorAll<HTMLElement>('.tok-rail-btn[data-tab]').forEach((b) => {
      const on = this.panelOpen && b.dataset.tab === this.activeTab;
      b.setAttribute('aria-pressed', String(on));
      b.setAttribute('aria-expanded', String(on));
    });
  }

  private panelHeader(title: string, count?: number, action?: HTMLElement): HTMLElement {
    const head = el('div', 'tok-panel-head');
    head.appendChild(el('h2', undefined, undefined, title));
    if (count !== undefined) head.appendChild(el('span', 'tok-badge-count', undefined, String(count)));
    head.appendChild(el('span', 'tok-grow'));
    if (action) head.appendChild(action);
    head.appendChild(iconButton('close', t('railHidePanel'), () => {
      this.setPanelOpen(false);
      this.callbacks.onPanelToggle?.(false);
      this.callbacks.onActiveTabChange?.(this.activeTab, false);
    }, { size: 15, attrs: { style: 'width:28px;height:28px' } }));
    return head;
  }

  private renderPanel(): void {
    if (!this.panel) return;
    this.panel.innerHTML = '';
    const titles: Record<TabId, string> = {
      pages: t('sidebarPages'),
      flows: t('structureFlowsTitle'),
      styles: t('structureStylesTitle'),
      layers: t('structureLayersTitle')
    };
    this.panel.setAttribute('aria-label', titles[this.activeTab]);

    const body = el('div', 'tok-structure-body');
    if (this.activeTab === 'pages') {
      const add = iconButton('plus', t('sidebarAddPage'), () => this.callbacks.onAddPage(), { size: 17, attrs: { 'aria-keyshortcuts': 'Control+Enter', style: 'width:30px;height:30px' } });
      this.panel.appendChild(this.panelHeader(titles.pages, this.pages.length, add));
      this.renderPagesView(body);
    } else if (this.activeTab === 'flows') {
      this.panel.appendChild(this.panelHeader(titles.flows, this.flows.length));
      this.renderFlowsView(body);
    } else if (this.activeTab === 'styles') {
      this.panel.appendChild(this.panelHeader(titles.styles, this.styles.length));
      this.renderStylesView(body);
    } else {
      this.panel.appendChild(this.panelHeader(titles.layers, this.layers.length));
      this.renderLayersView(body);
    }
    this.panel.appendChild(body);
  }

  /** Thumbnail label: "דף א׳" + position and amud (same helpers as the canvas). */
  private pageDescription(p: PageThumbnailItem): string {
    const side = t(isRightHandPage(p.pageIndex) ? 'structureSpreadRight' : 'structureSpreadLeft');
    const amud = isRectoPage(p.pageIndex) ? 'ע"א' : 'ע"ב';
    return `${p.label || tf('structurePageLabel', { g: p.gematria })} · ${tf('structurePageNumber', { n: p.pageIndex + 1 })} · ${side} (${amud})`;
  }

  private renderPagesView(container: HTMLElement): void {
    const list = el('div', 'tok-pages-list', { role: 'list' });
    list.style.display = 'flex';
    list.style.flexDirection = 'column';
    list.style.gap = '4px';

    for (const spread of groupIntoSpreads(this.pages.length)) {
      const items = spread.map((pos) => this.pages[pos]).filter(Boolean);
      const card = el('div', 'tok-spread-card', { role: 'listitem' });
      const sheets = el('div', 'tok-spread-pages');
      // A lone first page is a recto and sits on the left: keep an empty slot on the right.
      if (spread.length === 1 && isRectoPage(spread[0])) {
        sheets.appendChild(el('span', 'tok-thumb tok-thumb-empty', { 'aria-hidden': 'true' }));
      }
      for (const p of items) {
        const thumb = el('button', 'tok-thumb tok-page-card', {
          type: 'button',
          'data-page-index': p.pageIndex,
          'aria-label': this.pageDescription(p),
          title: this.pageDescription(p)
        });
        thumb.innerHTML = pageThumbDrawing(isRightHandPage(p.pageIndex));
        thumb.addEventListener('click', () => {
          this.activePageIndex = p.pageIndex;
          this.highlightActivePage();
          this.callbacks.onSelectPage(p.pageIndex);
        });
        sheets.appendChild(thumb);
      }
      card.appendChild(sheets);
      card.appendChild(el('span', 'tok-spread-label', undefined, items.map((p) => p.gematria).join(' – ')));
      list.appendChild(card);
    }
    container.appendChild(list);
    this.highlightActivePage();
  }

  private highlightActivePage(): void {
    this.panel.querySelectorAll<HTMLElement>('.tok-thumb[data-page-index]').forEach((thumb) => {
      const on = parseInt(thumb.dataset.pageIndex || '-1', 10) === this.activePageIndex;
      if (on) thumb.setAttribute('aria-current', 'page');
      else thumb.removeAttribute('aria-current');
    });
    this.panel.querySelectorAll<HTMLElement>('.tok-spread-card').forEach((card) => {
      const on = !!card.querySelector('[aria-current="page"]');
      card.classList.toggle('tok-active', on);
      if (on) card.scrollIntoView?.({ block: 'nearest' });
    });
  }

  private renderFlowsView(container: HTMLElement): void {
    for (const f of this.flows) {
      const on = f.id === this.activeFlowId;
      const row = el('button', 'tok-list-row', { type: 'button', 'aria-pressed': String(on) });
      const sw = el('span', 'tok-swatch', { 'aria-hidden': 'true' });
      sw.style.background = f.color;
      row.appendChild(sw);
      const main = el('span', 'tok-list-main');
      main.appendChild(el('span', 'tok-list-title', undefined, f.name));
      main.appendChild(el('span', 'tok-list-sub', undefined, `${t('structureFlowPosition')}: ${f.role}`));
      row.appendChild(main);
      row.appendChild(el('span', 'tok-list-meta', undefined, `${f.wordCount.toLocaleString()} ${t('statusWords')}`));
      row.addEventListener('click', () => {
        this.setActiveFlow(f.id);
        this.callbacks.onSelectFlow(f.id);
      });
      container.appendChild(row);
    }
    container.appendChild(el('p', 'tok-panel-note', undefined, t('structureFlowsDesc')));
  }

  private renderStylesView(container: HTMLElement): void {
    for (const s of this.styles) {
      const row = el('button', 'tok-list-row', { type: 'button' });
      const main = el('span', 'tok-list-main');
      main.appendChild(el('span', 'tok-list-title', { style: `font-weight:${s.fontWeight}` }, s.name));
      main.appendChild(el('span', 'tok-list-sub', { dir: 'ltr', style: 'text-align:start' }, `${s.fontFamily} · ${s.fontWeight} · ${s.fontSizePt}pt`));
      row.appendChild(main);
      const flow = this.flows.find((f) => f.id === s.flowId);
      if (flow) {
        const sw = el('span', 'tok-swatch', { title: flow.name, 'aria-hidden': 'true' });
        sw.style.background = flow.color;
        row.appendChild(sw);
      }
      row.addEventListener('click', () => this.callbacks.onSelectStyle(s.id));
      container.appendChild(row);
    }
  }

  private renderLayersView(container: HTMLElement): void {
    for (const l of this.layers) {
      const row = el('div', 'tok-list-row');

      const eye = iconButton(l.visible ? 'eye' : 'eyeOff', '', undefined, { size: 16, attrs: { style: 'width:28px;height:28px' } });
      const syncEye = () => {
        const label = t(l.visible ? 'structureHideLayer' : 'structureShowLayer');
        eye.replaceChildren(icon(l.visible ? 'eye' : 'eyeOff', 16));
        eye.title = label;
        eye.setAttribute('aria-label', `${label}: ${l.name}`);
        eye.setAttribute('aria-pressed', String(l.visible));
        eye.style.color = l.visible ? 'var(--tok-text-primary)' : 'var(--tok-text-muted)';
      };
      eye.addEventListener('click', () => {
        l.visible = !l.visible;
        syncEye();
        this.callbacks.onToggleLayer(l.id, l.visible);
      });
      syncEye();
      row.appendChild(eye);

      const main = el('span', 'tok-list-main');
      main.appendChild(el('span', 'tok-list-title', undefined, l.name));
      row.appendChild(main);

      const lock = iconButton(l.locked ? 'lock' : 'unlock', '', undefined, { size: 16, attrs: { style: 'width:28px;height:28px' } });
      const syncLock = () => {
        const label = t(l.locked ? 'structureUnlockLayer' : 'structureLockLayer');
        lock.replaceChildren(icon(l.locked ? 'lock' : 'unlock', 16));
        lock.title = label;
        lock.setAttribute('aria-label', `${label}: ${l.name}`);
        lock.setAttribute('aria-pressed', String(l.locked));
        lock.style.color = l.locked ? 'var(--tok-status-warning)' : 'var(--tok-text-muted)';
      };
      lock.addEventListener('click', () => {
        l.locked = !l.locked;
        syncLock();
      });
      syncLock();
      row.appendChild(lock);

      container.appendChild(row);
    }
  }
}

import { drawLineGuides } from './baselineGuides';
import { PageDescriptor, PageDomVirtualizer, computeActiveWindow } from 'tok-viewer';
import { SelectionMode, TextFrameData } from '../types';
import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { alignFrames, FrameAlignType, FrameRect } from '../utils/frameAlignment';

/**
 * Spread geometry for a right-to-left bound book.
 *
 * The first page (index 0) is a recto and stands alone; after it pages pair up as
 * (1,2), (3,4), ... In an RTL book the recto (front of the leaf, amud aleph) is the
 * LEFT page of an open spread and the verso (amud bet) is the RIGHT page, so:
 *   - even index -> recto, ע״א, left side
 *   - odd index  -> verso, ע״ב, right side
 * These helpers are the single source of truth for the canvas and the page list.
 */
export function isRectoPage(pageIndex: number): boolean {
  return pageIndex % 2 === 0;
}

export function isRightHandPage(pageIndex: number): boolean {
  return !isRectoPage(pageIndex);
}

/** Groups page positions into spreads, each listed right-to-left: [[0], [1, 2], [3, 4], ...]. */
export function groupIntoSpreads(pageCount: number): number[][] {
  const spreads: number[][] = [];
  if (pageCount <= 0) return spreads;
  spreads.push([0]);
  for (let i = 1; i < pageCount; i += 2) {
    spreads.push(i + 1 < pageCount ? [i, i + 1] : [i]);
  }
  return spreads;
}

/** Page sheet size in CSS px (17×24 cm proportions). */
export const PAGE_WIDTH_PX = 480;
export const PAGE_HEIGHT_PX = 678;
export const MIN_ZOOM = 25;
export const MAX_ZOOM = 400;

/** Zoom (percent) at which a full spread fits the visible canvas area. */
export function fitZoom(viewportWidth: number, viewportHeight: number, pageWidth = PAGE_WIDTH_PX, pageHeight = PAGE_HEIGHT_PX): number {
  const z = Math.min((viewportWidth - 80) / (2 * pageWidth), (viewportHeight - 136) / pageHeight) * 100;
  if (!Number.isFinite(z)) return 100;
  return Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, Math.floor(z)));
}

/** Escapes HTML entities to prevent XSS injection when interpolating strings into innerHTML. */
export function escapeHtml(str: string): string {
  if (!str) return '';
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

type FlowId = 'gemara' | 'rashi' | 'tosafot' | 'notes';

interface FrameParams {
  id: string;
  title: string;
  flowId: FlowId;
  className: string;
  label?: string;
  text: string;
}

export interface SpreadCanvasCallbacks {
  onSelectionModeChange: (mode: SelectionMode, frame?: TextFrameData) => void;
  onRequestActionHud: (x: number, y: number, initialValues?: any) => void;
  onDismissActionHud: () => void;
  onPageChange: (pageIndex: number) => void;
  /** The zoom changed from the canvas's own controls. */
  onZoomChange?: (zoomPercent: number) => void;
  /** User clicked on an interactive text frame on the canvas. */
  onSelectFrameFlow?: (flowId: string, paraId?: string) => void;
  /** User double clicked on an interactive text frame on the canvas. */
  onFrameDoubleClick?: (flowId: string, paraId?: string) => void;
}

export class SpreadCanvas {
  /** Outer shell (scroll area + floating zoom control). */
  public element: HTMLElement;
  private scroller: HTMLElement;
  private callbacks: SpreadCanvasCallbacks;
  private zoomPercent = 100;
  private hasAutoFitted = false;
  private showMargins = true;
  private showBaseline = false;
  private activePageIndex = 0;
  private pages: PageDescriptor[] = [];
  private selectedFrameId: string | null = null;
  private selectedFrameIds: Set<string> = new Set();
  private frameRects: Map<string, FrameRect> = new Map();
  private lockedFrameIds: Set<string> = new Set();
  private virtualizer: PageDomVirtualizer;
  private layerVisibility: Record<string, boolean> = {
    'text-main': true,
    'text-commentary': true,
    decorations: true,
    guides: true
  };
  private layerLocks: Record<string, boolean> = {
    'text-main': false,
    'text-commentary': false,
    decorations: false,
    guides: true
  };
  private innerContainer!: HTMLElement;
  /** Takes the scaled size of the pages so the scroll area matches what is drawn. */
  private zoomBox!: HTMLElement;
  private lastReportedPage = -1;
  private scrollRaf = 0;
  private programmaticScroll = false;
  private scrollSettleTimer = 0;
  private baselineGridPt = 13;
  private baselineOffsetPt = 48;

  constructor(callbacks: SpreadCanvasCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-canvas-shell');
    this.scroller = el('main', 'tok-canvas-container', { dir: 'rtl', tabindex: '-1' });
    this.element.appendChild(this.scroller);

    const virtualizerHost = el('div', 'tok-virtualizer-host', { 'aria-hidden': 'true', style: 'display:none;' });
    this.virtualizer = new PageDomVirtualizer(virtualizerHost);

    this.renderContainer();
    this.bindEvents();

    try {
      const savedGuides = localStorage.getItem('tok_canvas_guides');
      if (savedGuides) {
        const parsed = JSON.parse(savedGuides);
        if (typeof parsed.margins === 'boolean') this.showMargins = parsed.margins;
        if (typeof parsed.baseline === 'boolean') this.showBaseline = parsed.baseline;
      }
      const savedZoom = localStorage.getItem('tok_canvas_zoom');
      if (savedZoom) {
        const z = parseInt(savedZoom, 10);
        if (z >= MIN_ZOOM && z <= MAX_ZOOM) this.zoomPercent = z;
      }
    } catch {}

    i18n.onChange(() => {
      this.scroller.setAttribute('aria-label', t('canvasAria'));
      if (this.pages.length) this.renderSpreads();
    });
  }

  public destroy(): void {
    if (this.scrollSettleTimer) {
      clearTimeout(this.scrollSettleTimer);
      this.scrollSettleTimer = 0;
    }
    if (this.scrollRaf) {
      cancelAnimationFrame(this.scrollRaf);
      this.scrollRaf = 0;
    }
    this.virtualizer.destroy();
    this.element.remove();
  }

  public getVirtualizer(): PageDomVirtualizer {
    return this.virtualizer;
  }

  public getActiveWindow(): { start: number; end: number } {
    return computeActiveWindow(this.activePageIndex, this.pages.length);
  }

  private syncActiveVirtualWindow(): void {
    const win = computeActiveWindow(this.activePageIndex, this.pages.length);
    this.innerContainer.querySelectorAll<HTMLElement>('.tok-page-sheet[data-page-index]').forEach((sheet) => {
      const idx = parseInt(sheet.dataset.pageIndex || '-1', 10);
      const inWindow = idx >= win.start && idx <= win.end;
      sheet.classList.toggle('tok-page-mounted', inWindow);
      sheet.dataset.virtualMounted = String(inWindow);
    });
  }

  public setPages(pages: PageDescriptor[], activeIndex = 0): void {
    this.pages = pages;
    this.activePageIndex = activeIndex;
    this.lastReportedPage = activeIndex;
    this.virtualizer.setPages(pages);
    this.virtualizer.updateActiveWindow(activeIndex);
    this.renderSpreads();
    // After layout: fit the first spread to the window once, then reveal the active page.
    requestAnimationFrame(() => {
      if (!this.hasAutoFitted && this.scroller.clientWidth > 0) {
        this.hasAutoFitted = true;
        this.fitToWindow();
      }
      if (!this.programmaticScroll) this.revealPage(this.activePageIndex, false, false);
    });
  }

  public scrollToPage(pageIndex: number): void {
    this.activePageIndex = pageIndex;
    this.virtualizer.updateActiveWindow(pageIndex);
    this.syncActiveVirtualWindow();
    // While the smooth scroll runs, the pages it passes must not be reported; the
    // target is reported right away and tracking resumes once scrolling settles.
    this.programmaticScroll = true;
    this.armScrollSettle();
    if (this.lastReportedPage !== pageIndex) {
      this.lastReportedPage = pageIndex;
      this.callbacks.onPageChange(pageIndex);
    }
    this.revealPage(pageIndex, true, true);
  }

  private armScrollSettle(): void {
    if (this.scrollSettleTimer) clearTimeout(this.scrollSettleTimer);
    this.scrollSettleTimer = window.setTimeout(() => {
      this.scrollSettleTimer = 0;
      this.programmaticScroll = false;
    }, 200);
  }

  /**
   * Scrolls only the canvas so that the page is vertically centered (optional) and
   * horizontally fully visible.
   */
  private revealPage(pageIndex: number, smooth: boolean, centerVertically: boolean): void {
    const sheet = this.innerContainer.querySelector<HTMLElement>(`.tok-page-sheet[data-page-index="${pageIndex}"]`);
    if (!sheet) return;
    const s = sheet.getBoundingClientRect();
    const c = this.scroller.getBoundingClientRect();
    const margin = 16;
    let dx = 0;
    if (s.width + 2 * margin >= c.width) {
      dx = s.left + s.width / 2 - (c.left + c.width / 2);
    } else if (s.left < c.left + margin) {
      dx = s.left - (c.left + margin);
    } else if (s.right > c.right - margin) {
      dx = s.right - (c.right - margin);
    }
    let dy = 0;
    if (centerVertically) {
      dy = s.top + s.height / 2 - (c.top + c.height / 2);
    } else if (s.top < c.top) {
      dy = s.top - c.top - margin;
    } else if (s.bottom > c.bottom) {
      dy = Math.min(s.bottom - c.bottom + margin, s.top - c.top - margin);
    }
    if (dx || dy) this.scroller.scrollBy({ left: dx, top: dy, behavior: smooth ? 'smooth' : 'auto' });
  }

  private zoomHud!: HTMLElement;
  private zoomDisplayBtn!: HTMLButtonElement;

  public setZoom(zoom: number): void {
    this.zoomPercent = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, Math.round(zoom)));
    try {
      localStorage.setItem('tok_canvas_zoom', String(this.zoomPercent));
    } catch {}
    if (this.innerContainer) {
      this.innerContainer.style.transform = `scale(${this.zoomPercent / 100})`;
      this.updateZoomBox();
    }
    this.updateZoomHud();
  }

  public getZoom(): number {
    return this.zoomPercent;
  }

  /** Zooms so one full spread fits the visible area. */
  public fitToWindow(): void {
    const firstPage = this.pages[0];
    const pageWidth = firstPage?.widthPt || PAGE_WIDTH_PX;
    const pageHeight = firstPage?.heightPt || PAGE_HEIGHT_PX;
    this.applyUserZoom(fitZoom(this.scroller.clientWidth, this.scroller.clientHeight, pageWidth, pageHeight));
  }

  private applyUserZoom(z: number): void {
    this.setZoom(z);
    this.callbacks.onZoomChange?.(this.zoomPercent);
    requestAnimationFrame(() => this.revealPage(this.activePageIndex, false, true));
  }

  public setBaselineGrid(pt: number, offsetPt?: number): void {
    if (pt > 0) this.baselineGridPt = pt;
    if (typeof offsetPt === 'number' && offsetPt >= 0) this.baselineOffsetPt = offsetPt;
    this.updateGuides();
  }

  public toggleMarginsGuide(): void {
    this.showMargins = !this.showMargins;
    this.saveGuides();
    this.updateGuides();
  }

  public toggleBaselineGuide(): void {
    this.showBaseline = !this.showBaseline;
    this.saveGuides();
    this.updateGuides();
  }

  private saveGuides(): void {
    try {
      localStorage.setItem('tok_canvas_guides', JSON.stringify({ margins: this.showMargins, baseline: this.showBaseline }));
    } catch {}
  }

  public getGuides(): { margins: boolean; baseline: boolean } {
    return { margins: this.showMargins, baseline: this.showBaseline };
  }

  /** Updates the active page text frames in real time as the user types in the editor. */
  public updateActivePageText(paragraphs: { id: string; styleId: string; text: string }[], flowId = 'gemara'): void {
    const pageIndex = this.activePageIndex;
    const page = this.pages[pageIndex];
    if (!page) return;

    const sheet = this.innerContainer.querySelector<HTMLElement>(`.tok-page-sheet[data-page-index="${pageIndex}"]`);
    if (!sheet) return;

    const html = paragraphs
      .map((p) => `<p data-para-id="${escapeHtml(p.id)}" class="${p.styleId ? `tok-style-${escapeHtml(p.styleId)}` : ''}" style="margin:0 0 0.6em;line-height:1.55;">${escapeHtml(p.text) || '&nbsp;'}</p>`)
      .join('');

    page.htmlContent = html;

    const frameInner = sheet.querySelector<HTMLElement>(`.tok-interactive-frame[data-flow-id="${flowId}"] .tok-frame-text`);
    if (frameInner) {
      frameInner.innerHTML = html;
      return;
    }

    let contentEl = sheet.querySelector<HTMLElement>('.tok-page-content');
    if (!contentEl) {
      contentEl = el('div', 'tok-page-content', { style: 'flex: 1; min-height: 0; overflow: hidden; padding: 24px 28px;' });
      const foot = sheet.querySelector('.tok-page-foot');
      sheet.insertBefore(contentEl, foot);
    }
    contentEl.innerHTML = html;
  }

  private renderContainer(): void {
    this.scroller.innerHTML = '';
    this.scroller.setAttribute('aria-label', t('canvasAria'));
    this.zoomBox = el('div', 'tok-zoom-box');
    this.innerContainer = el('div', 'tok-spreads-wrapper');
    this.zoomBox.appendChild(this.innerContainer);
    this.scroller.appendChild(this.zoomBox);

    // Floating Zoom Controls HUD directly on the pages canvas
    this.zoomHud = el('div', 'tok-canvas-zoom-hud', {
      role: 'toolbar',
      'aria-label': 'בקרת תקריב וזום',
      style: 'position:absolute;bottom:20px;inset-inline-end:24px;z-index:25;display:flex;align-items:center;gap:3px;padding:3px 6px;background:var(--tok-bg-surface-1);border:1px solid var(--tok-border-strong);border-radius:24px;box-shadow:0 4px 18px rgba(0,0,0,0.18);backdrop-filter:blur(8px);user-select:none;'
    });

    // Drag handle for Zoom HUD
    const zoomDrag = el('div', 'tok-zoom-drag-handle', {
      title: 'גרור להזזת סרגל הזום',
      style: 'cursor:grab;display:flex;align-items:center;justify-content:center;padding:0 3px;color:var(--tok-text-muted);'
    });
    zoomDrag.appendChild(icon('layers', 12));

    let isZoomDragging = false;
    let zStartX = 0;
    let zStartY = 0;
    let zInitLeft = 0;
    let zInitTop = 0;

    zoomDrag.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      isZoomDragging = true;
      zStartX = e.clientX;
      zStartY = e.clientY;
      const rect = this.zoomHud.getBoundingClientRect();
      const parentRect = this.element.getBoundingClientRect();
      zInitLeft = rect.left - parentRect.left;
      zInitTop = rect.top - parentRect.top;
      this.zoomHud.style.insetInlineEnd = 'auto';
      this.zoomHud.style.bottom = 'auto';
      this.zoomHud.style.left = `${zInitLeft}px`;
      this.zoomHud.style.top = `${zInitTop}px`;
      zoomDrag.style.cursor = 'grabbing';
      e.preventDefault();
    });

    window.addEventListener('pointermove', (e) => {
      if (!isZoomDragging) return;
      const dx = e.clientX - zStartX;
      const dy = e.clientY - zStartY;
      const parentRect = this.element.getBoundingClientRect();
      const newL = Math.max(10, Math.min(parentRect.width - 150, zInitLeft + dx));
      const newT = Math.max(10, Math.min(parentRect.height - 45, zInitTop + dy));
      this.zoomHud.style.left = `${newL}px`;
      this.zoomHud.style.top = `${newT}px`;
    });

    window.addEventListener('pointerup', () => {
      if (isZoomDragging) {
        isZoomDragging = false;
        zoomDrag.style.cursor = 'grab';
      }
    });

    this.zoomHud.appendChild(zoomDrag);

    const zoomOut = iconButton('minus', 'הקטנת תצוגה (Ctrl+-)', () => this.applyUserZoom(this.zoomPercent - 10), {
      size: 13,
      attrs: { style: 'width:24px;height:24px;border-radius:50%;' }
    });
    this.zoomHud.appendChild(zoomOut);

    this.zoomDisplayBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'איפוס ל-100% (Ctrl+0)',
      style: 'font-size:11px;font-weight:600;min-width:40px;padding:0 3px;height:24px;'
    }, `${this.zoomPercent}%`) as HTMLButtonElement;
    this.zoomDisplayBtn.addEventListener('click', () => {
      if (this.zoomPercent === 100) this.fitToWindow();
      else this.applyUserZoom(100);
    });
    this.zoomHud.appendChild(this.zoomDisplayBtn);

    const zoomIn = iconButton('plus', 'הגדלת תצוגה (Ctrl++)', () => this.applyUserZoom(this.zoomPercent + 10), {
      size: 13,
      attrs: { style: 'width:24px;height:24px;border-radius:50%;' }
    });
    this.zoomHud.appendChild(zoomIn);

    const fitBtn = button('התאם', {
      className: 'tok-btn tok-btn-ghost tok-btn-xs',
      attrs: { title: 'התאמה מלאה לחלון', style: 'font-size:11px;padding:0 5px;height:24px;' },
      onClick: () => this.fitToWindow()
    });
    this.zoomHud.appendChild(fitBtn);

    this.element.appendChild(this.zoomHud);
  }

  private updateZoomHud(): void {
    if (this.zoomDisplayBtn) {
      this.zoomDisplayBtn.textContent = `${this.zoomPercent}%`;
    }
  }

  /** offsetWidth/Height are the unscaled layout size (transforms don't change them). */
  private updateZoomBox(): void {
    const z = this.zoomPercent / 100;
    this.zoomBox.style.width = `${this.innerContainer.offsetWidth * z}px`;
    this.zoomBox.style.height = `${this.innerContainer.offsetHeight * z}px`;
  }

  private renderSpreads(): void {
    this.innerContainer.innerHTML = '';
    if (this.pages.length === 0) {
      const emptyState = el('div', 'tok-canvas-empty', {
        style: 'display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:300px;gap:12px;color:var(--tok-text-muted);font-size:14px;padding:40px;'
      });
      emptyState.appendChild(icon('pages', 32));
      emptyState.appendChild(el('div', undefined, { style: 'font-weight:600;' }, 'אין עמודים להצגה'));
      this.innerContainer.appendChild(emptyState);
      this.updateZoomBox();
      return;
    }

    for (const spread of groupIntoSpreads(this.pages.length)) {
      const spreadRow = el('div', spread.length === 1 ? 'tok-spread-row tok-single' : 'tok-spread-row');
      // The canvas is dir="rtl", so the first sheet appended lands on the right.
      // The lone first page is a recto and therefore belongs on the left.
      if (spread.length === 1 && isRectoPage(spread[0])) {
        const firstPage = this.pages[spread[0]];
        const pageWidth = firstPage?.widthPt || PAGE_WIDTH_PX;
        spreadRow.style.justifyContent = 'flex-end';
        spreadRow.style.width = `${2 * pageWidth}pt`;
      }
      for (const pos of spread) {
        spreadRow.appendChild(this.createPageSheet(this.pages[pos], pos));
      }
      this.innerContainer.appendChild(spreadRow);
    }

    this.updateZoomBox();
    this.updateGuides();
    this.syncActiveVirtualWindow();
    this.applyLayerStates();
  }

  private flowToLayerId(flowId: string): string {
    return flowId === 'gemara' || flowId === 'main' ? 'text-main' : 'text-commentary';
  }

  private defaultFrameRect(frameId: string, flowId: string): FrameRect {
    switch (flowId) {
      case 'rashi':
        return { id: frameId, x: 150, y: 45, w: 55, h: 240, width: 55, height: 240 };
      case 'tosafot':
        return { id: frameId, x: 10, y: 45, w: 55, h: 240, width: 55, height: 240 };
      case 'notes':
        return { id: frameId, x: 35, y: 290, w: 110, h: 45, width: 110, height: 45 };
      case 'gemara':
      default:
        return { id: frameId, x: 35, y: 45, w: 110, h: 240, width: 110, height: 240 };
    }
  }

  private ensureFrameRect(frame: HTMLElement, frameId: string, flowId: string): FrameRect {
    const existing = this.frameRects.get(frameId);
    if (existing) {
      this.syncFrameElementRect(frame, existing);
      return existing;
    }
    const parseStyleNum = (val: string | undefined, fallback: number): number => {
      if (!val) return fallback;
      const n = parseFloat(val);
      return Number.isFinite(n) ? n : fallback;
    };
    const defaults = this.defaultFrameRect(frameId, flowId);
    const w = parseStyleNum(frame.style.width || frame.dataset.rectW, defaults.w);
    const h = parseStyleNum(frame.style.height || frame.dataset.rectH, defaults.h);
    const rect: FrameRect = {
      id: frameId,
      x: parseStyleNum(frame.style.left || frame.dataset.rectX, defaults.x),
      y: parseStyleNum(frame.style.top || frame.dataset.rectY, defaults.y),
      w,
      h,
      width: w,
      height: h
    };
    this.frameRects.set(frameId, rect);
    this.syncFrameElementRect(frame, rect);
    return rect;
  }

  private syncFrameElementRect(frame: HTMLElement, rect: FrameRect): void {
    frame.dataset.rectX = String(rect.x);
    frame.dataset.rectY = String(rect.y);
    frame.dataset.rectW = String(rect.w);
    frame.dataset.rectH = String(rect.h);
  }

  private createPageSheet(page: PageDescriptor, position: number): HTMLElement {
    const isRecto = isRectoPage(position);
    const isRightPage = isRightHandPage(position);
    const amud = isRecto ? 'ע״א' : 'ע״ב';
    const sheet = el('article', 'tok-page-sheet', { 'data-page-index': page.pageIndex, 'aria-label': `${tf('structurePageLabel', { g: page.gematriaNumber })} ${amud}` });

    const pageWidthPt = page.widthPt || PAGE_WIDTH_PX;
    const pageHeightPt = page.heightPt || PAGE_HEIGHT_PX;
    sheet.style.width = `${pageWidthPt}pt`;
    sheet.style.height = `${pageHeightPt}pt`;
    sheet.dataset.widthPt = String(pageWidthPt);
    sheet.dataset.heightPt = String(pageHeightPt);

    // Click on page background clears frame selection (zero selection).
    sheet.addEventListener('click', (e) => {
      if (e.target === sheet) {
        this.clearFrameSelection();
        this.callbacks.onSelectionModeChange('zero');
        this.callbacks.onDismissActionHud();
      }
    });

    // Shading along the spine (the inner edge).
    const spine = el('div', 'tok-page-spine', { 'aria-hidden': 'true', 'data-layer-id': 'decorations' });
    spine.style[isRightPage ? 'left' : 'right'] = '0';
    spine.style.background = `linear-gradient(to ${isRightPage ? 'left' : 'right'}, rgba(40,30,10,.10), rgba(40,30,10,0))`;
    sheet.appendChild(spine);

    // 1. Running head: page number on the outer corner.
    const head = el('div', 'tok-page-head', { 'data-layer-id': 'decorations' });
    const pageNo = el('span', undefined, undefined, `דף ${page.gematriaNumber} ${amud}`);
    if (isRightPage) head.append(pageNo);
    else head.append(pageNo);
    sheet.appendChild(head);

    // 2. Page body: rendered document content or clean interactive editable text frame.
    if (page.htmlContent && page.htmlContent.trim().length > 0) {
      const isEngineLayout = page.htmlContent.includes('tok-page-layout-rust');
      if (isEngineLayout) {
        sheet.style.padding = '0';
        sheet.classList.add('tok-page-engine-layout');
        head.style.position = 'absolute';
        head.style.top = '14pt';
        head.style.left = isRightPage ? 'auto' : '24pt';
        head.style.right = isRightPage ? '24pt' : 'auto';
        head.style.borderBottom = 'none';
        head.style.zIndex = '5';
      }
      const content = el('div', 'tok-page-content', {
        style: isEngineLayout
          ? 'position: absolute; inset: 0; overflow: hidden;'
          : 'flex: 1; min-height: 0; overflow: hidden;'
      });
      content.innerHTML = page.htmlContent;
      sheet.appendChild(content);

      // Bind interactive clicks on all frames inside the rendered multi-flow content
      const frames = content.querySelectorAll<HTMLElement>('.tok-interactive-frame');
      frames.forEach((fr, idx) => {
        const rawFlow = fr.dataset.flowId || 'gemara';
        const flowId = (rawFlow === 'main' || rawFlow === 'primary') ? 'gemara' : (rawFlow as FlowId);
        const frameId = fr.dataset.frameId || `frame-${page.pageIndex}-${flowId}-${idx}`;
        fr.dataset.frameId = frameId;
        fr.dataset.layerId = this.flowToLayerId(flowId);
        this.ensureFrameRect(fr, frameId, flowId);

        fr.addEventListener('dblclick', (e) => {
          e.stopPropagation();
          const target = e.target as HTMLElement | null;
          const paraEl = target?.closest('[data-para-id]') as HTMLElement | null;
          const paraId = paraEl?.dataset?.paraId;
          this.callbacks.onFrameDoubleClick?.(flowId, paraId);
        });

        fr.addEventListener('click', (e) => {
          e.stopPropagation();
          const layerId = fr.dataset.layerId || this.flowToLayerId(flowId);
          if (this.isLayerLocked(layerId) || !this.isLayerVisible(layerId)) return;

          const target = e.target as HTMLElement | null;
          const paraEl = target?.closest('[data-para-id]') as HTMLElement | null;
          const paraId = paraEl?.dataset?.paraId;

          this.selectFrame(
            fr,
            {
              id: frameId,
              title: this.flowTitle(flowId),
              flowId,
              className: '',
              text: ''
            },
            e.shiftKey
          );

          if (this.selectedFrameIds.size > 1) {
            this.callbacks.onSelectionModeChange('multi');
            return;
          }

          this.callbacks.onSelectFrameFlow?.(flowId, paraId);

          const rect = this.ensureFrameRect(fr, frameId, flowId);
          const sel = window.getSelection();
          if (sel && sel.toString().trim().length > 0) {
            this.callbacks.onSelectionModeChange('text-edit');
            this.callbacks.onRequestActionHud(e.clientX, e.clientY, {
              size: flowId === 'gemara' ? 14 : 11,
              bold: flowId === 'gemara',
              align: 'justify',
              style: flowId
            });
          } else {
            this.callbacks.onSelectionModeChange('text-frame', {
              id: frameId,
              xMm: rect.x,
              yMm: rect.y,
              widthMm: rect.w,
              heightMm: rect.h,
              rotationDeg: 0,
              flowId,
              columns: 1,
              columnGapMm: 0,
              insetTopMm: 3,
              insetBottomMm: 3,
              insetRightMm: 4,
              insetLeftMm: 4,
              verticalAlign: (fr.dataset.verticalAlign as TextFrameData['verticalAlign']) || 'top',
              isLocked: this.isFrameLocked(frameId),
              text: ''
            });
          }
        });
      });
    } else {
      const bodyFrame = this.createInteractiveFrame({
        id: `frame-main-${page.pageIndex}`,
        title: t('appMainText'),
        flowId: 'gemara',
        className: 'tok-frame-gemara',
        text: ''
      });
      bodyFrame.style.flex = '1';
      bodyFrame.style.minHeight = '0';
      sheet.appendChild(bodyFrame);
    }

    // 3. Running foot.
    sheet.appendChild(el('div', 'tok-page-foot', { 'aria-hidden': 'true', 'data-layer-id': 'decorations' }, `- ${page.gematriaNumber} -`));

    // Guides (margin rectangle, baseline grid).
    const guides = el('div', 'tok-guides-layer', { 'aria-hidden': 'true', 'data-layer-id': 'guides' });
    const marginGuide = el('div', 'tok-guide-margin-rect');
    marginGuide.style.inset = '36px 38px 40px';
    guides.appendChild(marginGuide);
    const baseline = el('div', 'tok-guide-baseline');
    baseline.style.inset = '36px 38px 40px';
    guides.appendChild(baseline);
    sheet.appendChild(guides);

    return sheet;
  }

  private createInteractiveFrame(params: FrameParams): HTMLElement {
    const layerId = this.flowToLayerId(params.flowId);
    const frame = el('div', 'tok-interactive-frame', {
      'data-frame-id': params.id,
      'data-flow-id': params.flowId,
      'data-layer-id': layerId
    });
    const inner = el('div', `tok-frame-text ${params.className}`);
    inner.innerHTML = (params.label ? `<div class="tok-frame-label" data-layer-id="decorations">${escapeHtml(params.label)}</div>` : '') + (params.text ? escapeHtml(params.text) : '');
    frame.appendChild(inner);
    this.ensureFrameRect(frame, params.id, params.flowId);

    frame.addEventListener('click', (e) => {
      e.stopPropagation();
      if (this.isLayerLocked(layerId) || !this.isLayerVisible(layerId)) return;

      this.selectFrame(frame, params, e.shiftKey);

      if (this.selectedFrameIds.size > 1) {
        this.callbacks.onSelectionModeChange('multi');
        return;
      }

      const rect = this.ensureFrameRect(frame, params.id, params.flowId);

      const sel = window.getSelection();
      if (sel && sel.toString().trim().length > 0) {
        // Text selected: floating text toolbar + text-edit inspector.
        this.callbacks.onSelectionModeChange('text-edit');
        this.callbacks.onRequestActionHud(e.clientX, e.clientY, {
          size: params.flowId === 'gemara' ? 15 : 11,
          bold: params.flowId === 'gemara',
          align: 'justify',
          style: params.flowId
        });
      } else {
        this.callbacks.onSelectionModeChange('text-frame', {
          id: params.id,
          xMm: rect.x,
          yMm: rect.y,
          widthMm: rect.w,
          heightMm: rect.h,
          rotationDeg: 0,
          flowId: params.flowId,
          columns: 1,
          columnGapMm: 0,
          insetTopMm: 3,
          insetBottomMm: 3,
          insetRightMm: 4,
          insetLeftMm: 4,
          verticalAlign: (frame.dataset.verticalAlign as TextFrameData['verticalAlign']) || 'top',
          isLocked: this.isFrameLocked(params.id),
          text: params.text
        });
      }
    });

    return frame;
  }

  public isFrameLocked(frameId: string): boolean {
    return this.lockedFrameIds.has(frameId);
  }

  public setFrameLocked(frameId: string, locked: boolean): void {
    if (locked) {
      this.lockedFrameIds.add(frameId);
    } else {
      this.lockedFrameIds.delete(frameId);
    }
    const frame = this.innerContainer.querySelector<HTMLElement>(`.tok-interactive-frame[data-frame-id="${frameId}"]`);
    if (frame) {
      frame.dataset.frameLocked = String(locked);
      frame.classList.toggle('tok-frame-locked', locked);
      const flowId = (frame.dataset.flowId || 'gemara') as FlowId;
      this.updateFrameTagAndHandles(frame, frameId, this.flowTitle(flowId), flowId, locked);
    }
  }

  public toggleFrameLock(frameId: string): boolean {
    const next = !this.isFrameLocked(frameId);
    this.setFrameLocked(frameId, next);
    const rect = this.frameRects.get(frameId);
    this.callbacks.onSelectionModeChange('text-frame', {
      id: frameId,
      xMm: rect?.x || 10,
      yMm: rect?.y || 45,
      widthMm: rect?.w || 110,
      heightMm: rect?.h || 240,
      rotationDeg: 0,
      flowId: 'gemara',
      columns: 1,
      columnGapMm: 0,
      insetTopMm: 3,
      insetBottomMm: 3,
      insetRightMm: 4,
      insetLeftMm: 4,
      verticalAlign: 'top',
      text: '',
      isLocked: next
    });
    return next;
  }

  private updateFrameTagAndHandles(
    frame: HTMLElement,
    frameId: string,
    title: string,
    flowId: FlowId,
    isLocked: boolean
  ): void {
    frame.querySelectorAll('.tok-frame-tag, .tok-resize-handle').forEach((el) => el.remove());

    // Frame tag with drag ability and lock toggle button
    const tag = el('span', 'tok-frame-tag', { 'data-frame-tag-id': frameId });
    const tagTitle = el('span', undefined, undefined, title);
    tag.appendChild(tagTitle);

    const lockBtn = el('button', 'tok-frame-lock-btn', {
      type: 'button',
      title: isLocked ? 'שחרר נעילת תיבה (ניתן להזזה)' : 'נעל מיקום תיבה',
      'aria-label': isLocked ? 'שחרר נעילת תיבה' : 'נעל תיבה'
    });
    lockBtn.appendChild(icon(isLocked ? 'lock' : 'unlock', 12));
    lockBtn.addEventListener('click', (ev) => {
      ev.stopPropagation();
      ev.preventDefault();
      this.toggleFrameLock(frameId);
    });
    tag.appendChild(lockBtn);
    frame.appendChild(tag);

    if (!isLocked) {
      this.attachFrameDrag(frame, tag, frameId);

      // Corner handles
      const corners: [string, string, string][] = [
        ['top', 'left', 'nwse-resize'],
        ['top', 'right', 'nesw-resize'],
        ['bottom', 'left', 'nesw-resize'],
        ['bottom', 'right', 'nwse-resize']
      ];
      for (const [v, h, cursor] of corners) {
        const handle = el('div', 'tok-resize-handle');
        handle.style[v as 'top' | 'bottom'] = '-7px';
        handle.style[h as 'left' | 'right'] = '-7px';
        handle.style.cursor = cursor;
        this.attachHandleResize(handle, v, h, frame, frameId);
        frame.appendChild(handle);
      }
    }
  }

  private attachFrameDrag(frame: HTMLElement, handleEl: HTMLElement, frameId: string): void {
    handleEl.addEventListener('pointerdown', (e: PointerEvent) => {
      if (this.isFrameLocked(frameId)) return;
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();

      const sheet = frame.closest<HTMLElement>('.tok-page-sheet');
      if (!sheet) return;

      const z = this.zoomPercent / 100;
      const sheetRect = sheet.getBoundingClientRect();
      const frameRect = frame.getBoundingClientRect();

      const initLeft = (frameRect.left - sheetRect.left) / z;
      const initTop = (frameRect.top - sheetRect.top) / z;
      const width = frameRect.width / z;
      const height = frameRect.height / z;

      frame.style.position = 'absolute';
      frame.style.left = `${initLeft.toFixed(1)}pt`;
      frame.style.top = `${initTop.toFixed(1)}pt`;
      frame.style.width = `${width.toFixed(1)}pt`;
      frame.style.height = `${height.toFixed(1)}pt`;
      frame.style.margin = '0';
      frame.style.zIndex = '15';

      const startX = e.clientX;
      const startY = e.clientY;

      const onPointerMove = (ev: PointerEvent) => {
        const dx = (ev.clientX - startX) / z;
        const dy = (ev.clientY - startY) / z;
        const curLeft = Math.max(0, initLeft + dx);
        const curTop = Math.max(0, initTop + dy);

        frame.style.left = `${curLeft.toFixed(1)}pt`;
        frame.style.top = `${curTop.toFixed(1)}pt`;

        const rect = this.ensureFrameRect(frame, frameId, frame.dataset.flowId || 'gemara');
        rect.x = Math.round(curLeft);
        rect.y = Math.round(curTop);
        this.frameRects.set(frameId, rect);
        this.syncFrameElementRect(frame, rect);

        this.callbacks.onSelectionModeChange('text-frame', {
          id: frameId,
          xMm: rect.x,
          yMm: rect.y,
          widthMm: rect.w,
          heightMm: rect.h,
          rotationDeg: 0,
          flowId: (frame.dataset.flowId || 'gemara') as any,
          columns: 1,
          columnGapMm: 0,
          insetTopMm: 3,
          insetBottomMm: 3,
          insetRightMm: 4,
          insetLeftMm: 4,
          verticalAlign: (frame.dataset.verticalAlign as any) || 'top',
          text: '',
          isLocked: false
        });
      };

      const onPointerUp = () => {
        window.removeEventListener('pointermove', onPointerMove);
        window.removeEventListener('pointerup', onPointerUp);
        frame.style.zIndex = '';
      };

      window.addEventListener('pointermove', onPointerMove);
      window.addEventListener('pointerup', onPointerUp);
    });
  }

  private attachHandleResize(handle: HTMLElement, v: string, h: string, frame: HTMLElement, frameId: string): void {
    handle.addEventListener('pointerdown', (e: PointerEvent) => {
      if (this.isFrameLocked(frameId)) return;
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();

      const sheet = frame.closest<HTMLElement>('.tok-page-sheet');
      if (!sheet) return;

      const z = this.zoomPercent / 100;
      const sheetRect = sheet.getBoundingClientRect();
      const frameRect = frame.getBoundingClientRect();

      const initLeft = (frameRect.left - sheetRect.left) / z;
      const initTop = (frameRect.top - sheetRect.top) / z;
      const initW = frameRect.width / z;
      const initH = frameRect.height / z;

      frame.style.position = 'absolute';
      frame.style.left = `${initLeft.toFixed(1)}pt`;
      frame.style.top = `${initTop.toFixed(1)}pt`;
      frame.style.width = `${initW.toFixed(1)}pt`;
      frame.style.height = `${initH.toFixed(1)}pt`;
      frame.style.margin = '0';
      frame.style.zIndex = '15';

      const startX = e.clientX;
      const startY = e.clientY;

      const onPointerMove = (ev: PointerEvent) => {
        const dx = (ev.clientX - startX) / z;
        const dy = (ev.clientY - startY) / z;

        let curLeft = initLeft;
        let curTop = initTop;
        let curW = initW;
        let curH = initH;

        if (h === 'right') {
          curW = Math.max(30, initW + dx);
        } else {
          const potentialW = initW - dx;
          if (potentialW >= 30) {
            curW = potentialW;
            curLeft = initLeft + dx;
          }
        }

        if (v === 'bottom') {
          curH = Math.max(25, initH + dy);
        } else {
          const potentialH = initH - dy;
          if (potentialH >= 25) {
            curH = potentialH;
            curTop = initTop + dy;
          }
        }

        frame.style.left = `${curLeft.toFixed(1)}pt`;
        frame.style.top = `${curTop.toFixed(1)}pt`;
        frame.style.width = `${curW.toFixed(1)}pt`;
        frame.style.height = `${curH.toFixed(1)}pt`;

        const rect = this.ensureFrameRect(frame, frameId, frame.dataset.flowId || 'gemara');
        rect.x = Math.round(curLeft);
        rect.y = Math.round(curTop);
        rect.w = Math.round(curW);
        rect.h = Math.round(curH);
        rect.width = rect.w;
        rect.height = rect.h;
        this.frameRects.set(frameId, rect);
        this.syncFrameElementRect(frame, rect);

        this.callbacks.onSelectionModeChange('text-frame', {
          id: frameId,
          xMm: rect.x,
          yMm: rect.y,
          widthMm: rect.w,
          heightMm: rect.h,
          rotationDeg: 0,
          flowId: (frame.dataset.flowId || 'gemara') as any,
          columns: 1,
          columnGapMm: 0,
          insetTopMm: 3,
          insetBottomMm: 3,
          insetRightMm: 4,
          insetLeftMm: 4,
          verticalAlign: (frame.dataset.verticalAlign as any) || 'top',
          text: '',
          isLocked: false
        });
      };

      const onPointerUp = () => {
        window.removeEventListener('pointermove', onPointerMove);
        window.removeEventListener('pointerup', onPointerUp);
        frame.style.zIndex = '';
      };

      window.addEventListener('pointermove', onPointerMove);
      window.addEventListener('pointerup', onPointerUp);
    });
  }

  private selectFrame(frame: HTMLElement, params: FrameParams, additive = false): void {
    if (additive) {
      if (this.selectedFrameIds.has(params.id)) {
        this.selectedFrameIds.delete(params.id);
        frame.classList.remove('tok-selected');
        frame.querySelectorAll('.tok-resize-handle, .tok-frame-tag').forEach((h) => h.remove());
        const remaining = Array.from(this.selectedFrameIds);
        this.selectedFrameId = remaining[remaining.length - 1] || null;
        return;
      }
    } else {
      this.clearFrameSelection();
    }
    this.selectedFrameId = params.id;
    this.selectedFrameIds.add(params.id);
    frame.classList.add('tok-selected');
    if (!frame.querySelector('.tok-frame-tag')) {
      frame.appendChild(el('span', 'tok-frame-tag', { 'aria-hidden': 'true' }, params.title));
    }

    // Corner handles
    const corners: [string, string, string][] = [
      ['top', 'left', 'nwse-resize'],
      ['top', 'right', 'nesw-resize'],
      ['bottom', 'left', 'nesw-resize'],
      ['bottom', 'right', 'nwse-resize']
    ];
    for (const [v, h, cursor] of corners) {
      const handle = el('div', 'tok-resize-handle');
      handle.style[v as 'top' | 'bottom'] = '-7px';
      handle.style[h as 'left' | 'right'] = '-7px';
      handle.style.cursor = cursor;
      frame.appendChild(handle);
    }
  }

  private clearFrameSelection(): void {
    this.innerContainer.querySelectorAll<HTMLElement>('.tok-interactive-frame.tok-selected').forEach((f) => {
      f.classList.remove('tok-selected');
      f.querySelectorAll('.tok-resize-handle, .tok-frame-tag').forEach((h) => h.remove());
    });
    this.selectedFrameId = null;
    this.selectedFrameIds.clear();
  }

  public getSelectedFrameIds(): string[] {
    return Array.from(this.selectedFrameIds);
  }

  public selectFramesByIds(frameIds: string[]): void {
    this.clearFrameSelection();
    for (const id of frameIds) {
      const frame = this.innerContainer.querySelector<HTMLElement>(`.tok-interactive-frame[data-frame-id="${id}"]`);
      if (frame) {
        const flowId = (frame.dataset.flowId || 'gemara') as FlowId;
        this.selectFrame(
          frame,
          { id, title: this.flowTitle(flowId), flowId, className: '', text: '' },
          true
        );
      } else {
        this.selectedFrameIds.add(id);
        this.selectedFrameId = id;
      }
    }
    if (this.selectedFrameIds.size > 1) {
      this.callbacks.onSelectionModeChange('multi');
    }
  }

  public alignSelectedFrames(alignType: FrameAlignType | string): { before: FrameRect[]; after: FrameRect[] } {
    let targetFrames: HTMLElement[] = [];
    if (this.selectedFrameIds.size >= 2) {
      for (const id of this.selectedFrameIds) {
        const f = this.innerContainer.querySelector<HTMLElement>(`.tok-interactive-frame[data-frame-id="${id}"]`);
        if (f) targetFrames.push(f);
      }
    }
    if (targetFrames.length < 2) {
      const activeSheet = this.innerContainer.querySelector<HTMLElement>(`.tok-page-sheet[data-page-index="${this.activePageIndex}"]`);
      if (activeSheet) {
        targetFrames = Array.from(activeSheet.querySelectorAll<HTMLElement>('.tok-interactive-frame')).filter((f) => {
          const lid = f.dataset.layerId || this.flowToLayerId(f.dataset.flowId || 'gemara');
          return this.isLayerVisible(lid) && !this.isLayerLocked(lid);
        });
      }
    }
    if (targetFrames.length < 2) {
      targetFrames = Array.from(this.innerContainer.querySelectorAll<HTMLElement>('.tok-interactive-frame')).filter((f) => {
        const lid = f.dataset.layerId || this.flowToLayerId(f.dataset.flowId || 'gemara');
        return this.isLayerVisible(lid) && !this.isLayerLocked(lid);
      });
    }

    const before: FrameRect[] = targetFrames.map((f, idx) => {
      const flowId = f.dataset.flowId || 'gemara';
      const id = f.dataset.frameId || `frame-${this.activePageIndex}-${flowId}-${idx}`;
      f.dataset.frameId = id;
      const rect = this.ensureFrameRect(f, id, flowId);
      return { ...rect };
    });

    const after = alignFrames(before, alignType);
    this.applyFrameRects(after);
    return { before, after };
  }

  public applyFrameRects(rects: FrameRect[]): void {
    for (const r of rects) {
      const w = r.w ?? r.width ?? 110;
      const h = r.h ?? r.height ?? 240;
      const normalized: FrameRect = { ...r, w, h, width: w, height: h };
      this.frameRects.set(r.id, normalized);
      const frame = this.innerContainer.querySelector<HTMLElement>(`.tok-interactive-frame[data-frame-id="${r.id}"]`);
      if (frame) {
        this.syncFrameElementRect(frame, normalized);
        frame.style.left = `${r.x}pt`;
        frame.style.top = `${r.y}pt`;
        frame.style.width = `${w}pt`;
        frame.style.height = `${h}pt`;
      }
    }
  }

  public applyFrameGeometry(geometry: Partial<TextFrameData>): void {
    const targetId = geometry.id || this.selectedFrameId;
    let frame: HTMLElement | null = null;
    if (targetId) {
      frame = this.innerContainer.querySelector<HTMLElement>(`.tok-interactive-frame[data-frame-id="${targetId}"]`);
    }
    if (!frame) {
      const activeSheet = this.innerContainer.querySelector<HTMLElement>(`.tok-page-sheet[data-page-index="${this.activePageIndex}"]`);
      frame = activeSheet?.querySelector<HTMLElement>('.tok-interactive-frame') || null;
    }
    if (!frame) return;

    if (geometry.isLocked !== undefined && targetId) {
      this.setFrameLocked(targetId, geometry.isLocked);
    }

    const frameId = frame.dataset.frameId || targetId || `frame-main-${this.activePageIndex}`;
    const flowId = frame.dataset.flowId || 'gemara';
    const rect = { ...this.ensureFrameRect(frame, frameId, flowId) };

    if (typeof geometry.xMm === 'number') {
      rect.x = geometry.xMm;
      frame.style.left = `${geometry.xMm}pt`;
    }
    if (typeof geometry.yMm === 'number') {
      rect.y = geometry.yMm;
      frame.style.top = `${geometry.yMm}pt`;
    }
    if (typeof geometry.widthMm === 'number') {
      rect.w = geometry.widthMm;
      rect.width = geometry.widthMm;
      frame.style.width = `${geometry.widthMm}pt`;
    }
    if (typeof geometry.heightMm === 'number') {
      rect.h = geometry.heightMm;
      rect.height = geometry.heightMm;
      frame.style.height = `${geometry.heightMm}pt`;
    }
    this.frameRects.set(frameId, rect);
    this.syncFrameElementRect(frame, rect);

    const textEl = frame.querySelector<HTMLElement>('.tok-frame-text') || frame;
    if (typeof geometry.insetTopMm === 'number') textEl.style.paddingTop = `${geometry.insetTopMm}mm`;
    if (typeof geometry.insetBottomMm === 'number') textEl.style.paddingBottom = `${geometry.insetBottomMm}mm`;
    if (typeof geometry.insetRightMm === 'number') textEl.style.paddingRight = `${geometry.insetRightMm}mm`;
    if (typeof geometry.insetLeftMm === 'number') textEl.style.paddingLeft = `${geometry.insetLeftMm}mm`;

    if (geometry.verticalAlign) {
      frame.dataset.verticalAlign = geometry.verticalAlign;
      const justifyMap: Record<TextFrameData['verticalAlign'], string> = {
        top: 'flex-start',
        center: 'center',
        bottom: 'flex-end',
        justify: 'space-between'
      };
      const justify = justifyMap[geometry.verticalAlign] || 'flex-start';
      frame.style.display = 'flex';
      frame.style.flexDirection = 'column';
      frame.style.justifyContent = justify;
      textEl.style.display = 'flex';
      textEl.style.flexDirection = 'column';
      textEl.style.justifyContent = justify;
      textEl.style.flex = '1';
    }
  }

  public setLayerVisibility(layerId: string, visible: boolean): void {
    this.layerVisibility[layerId] = visible;
    if (!visible) {
      this.innerContainer.querySelectorAll<HTMLElement>(`.tok-interactive-frame[data-layer-id="${layerId}"].tok-selected`).forEach((f) => {
        const fid = f.dataset.frameId;
        if (fid) this.selectedFrameIds.delete(fid);
        f.classList.remove('tok-selected');
        f.querySelectorAll('.tok-resize-handle, .tok-frame-tag').forEach((h) => h.remove());
      });
    }
    this.applyLayerStates();
  }

  public setLayerLocked(layerId: string, locked: boolean): void {
    this.layerLocks[layerId] = locked;
    if (locked) {
      this.innerContainer.querySelectorAll<HTMLElement>(`.tok-interactive-frame[data-layer-id="${layerId}"].tok-selected`).forEach((f) => {
        const fid = f.dataset.frameId;
        if (fid) this.selectedFrameIds.delete(fid);
        f.classList.remove('tok-selected');
        f.querySelectorAll('.tok-resize-handle, .tok-frame-tag').forEach((h) => h.remove());
      });
    }
    this.applyLayerStates();
  }

  public isLayerVisible(layerId: string): boolean {
    return this.layerVisibility[layerId] !== false;
  }

  public isLayerLocked(layerId: string): boolean {
    return Boolean(this.layerLocks[layerId]);
  }

  private applyLayerStates(): void {
    this.innerContainer.querySelectorAll<HTMLElement>('[data-layer-id]').forEach((elem) => {
      const lid = elem.dataset.layerId || '';
      const visible = this.isLayerVisible(lid);
      const locked = this.isLayerLocked(lid);
      elem.classList.toggle('tok-layer-hidden', !visible);
      elem.classList.toggle('tok-layer-locked', locked);
      elem.style.display = visible ? '' : 'none';
      if (elem.classList.contains('tok-interactive-frame')) {
        elem.style.pointerEvents = locked ? 'none' : '';
        elem.setAttribute('aria-disabled', String(locked));
      }
    });
  }

  private updateGuides(): void {
    this.innerContainer.querySelectorAll<HTMLElement>('.tok-guide-margin-rect').forEach((g) => {
      g.style.display = this.showMargins ? 'block' : 'none';
    });

    const stepPx = (this.baselineGridPt * 96) / 72;
    const offsetPx = (this.baselineOffsetPt * 96) / 72;
    const line = 'var(--tok-guide-baseline)';
    this.innerContainer.querySelectorAll<HTMLElement>('.tok-guide-baseline').forEach((g) => {
      g.style.display = this.showBaseline ? 'block' : 'none';
      if (!this.showBaseline) return;

      const sheet = g.closest<HTMLElement>('.tok-page-sheet');
      // With engine lines on the page, draw one rule per real line (exact alignment with the text).
      if (drawLineGuides(g, sheet, line)) {
        g.style.inset = '0';
        g.style.backgroundImage = 'none';
        return;
      }
      // Blank page: show a plain fixed grid inside the margins.
      g.style.top = `${Math.max(0, offsetPx)}px`;
      g.style.bottom = '40px';
      g.style.left = '38px';
      g.style.right = '36px';
      g.style.backgroundImage =
        `repeating-linear-gradient(to bottom, ${line} 0, ${line} 1px, transparent 1px, transparent ${stepPx}px)`;
    });
  }

  /** Index (PageDescriptor.pageIndex) of the page sheet closest to the viewport's vertical center. */
  private findCenteredPage(): number | null {
    const sheets = this.innerContainer.querySelectorAll<HTMLElement>('.tok-page-sheet[data-page-index]');
    if (sheets.length === 0) return null;
    const box = this.scroller.getBoundingClientRect();
    const centerY = box.top + box.height / 2;
    let best: number | null = null;
    let bestDist = Infinity;
    sheets.forEach((sheet) => {
      const r = sheet.getBoundingClientRect();
      const dist = centerY < r.top ? r.top - centerY : centerY > r.bottom ? centerY - r.bottom : 0;
      const idx = parseInt(sheet.dataset.pageIndex as string, 10);
      // Both pages of a spread share a row: keep the current page if it is one of
      // them, otherwise the first (right-hand) one wins the tie.
      if (dist < bestDist || (dist === bestDist && idx === this.activePageIndex)) {
        bestDist = dist;
        best = idx;
      }
    });
    return best;
  }

  private bindEvents(): void {
    // Canvas background click clears selection.
    this.scroller.addEventListener('click', (e) => {
      if (e.target === this.scroller || e.target === this.innerContainer || e.target === this.zoomBox) {
        this.clearFrameSelection();
        this.callbacks.onSelectionModeChange('zero');
        this.callbacks.onDismissActionHud();
      }
    });

    let lastSwipeTime = 0;

    // Trackpad gestures & mouse wheel:
    // 1. Two-finger pinch-to-zoom (wheel with e.ctrlKey)
    // 2. Two-finger horizontal swipe for page turning (when cursor is on pages, not on toolbars)
    this.scroller.addEventListener('wheel', (e: WheelEvent) => {
      if (e.ctrlKey) {
        e.preventDefault();
        let delta = 0;
        if (Math.abs(e.deltaY) < 25) {
          delta = e.deltaY < 0 ? 3 : -3;
        } else {
          delta = e.deltaY < 0 ? 10 : -10;
        }
        this.applyUserZoom(this.zoomPercent + delta);
        return;
      }

      // Trackpad two-finger swipe: only if cursor is over pages/canvas area and not on interactive bars
      const target = e.target as HTMLElement | null;
      const isOverPages = target && (target.closest('.tok-page-sheet') || target.closest('.tok-canvas-container'));
      if (isOverPages && Math.abs(e.deltaX) > 35 && Math.abs(e.deltaX) > Math.abs(e.deltaY) * 1.4) {
        const now = Date.now();
        if (now - lastSwipeTime > 350) {
          lastSwipeTime = now;
          const isRtl = i18n.getDirection() === 'rtl';
          const goNext = isRtl ? e.deltaX > 0 : e.deltaX < 0;
          if (goNext && this.activePageIndex + 1 < this.pages.length) {
            e.preventDefault();
            this.scrollToPage(this.activePageIndex + 1);
          } else if (!goNext && this.activePageIndex > 0) {
            e.preventDefault();
            this.scrollToPage(this.activePageIndex - 1);
          }
        }
      }
    }, { passive: false });

    // Report the page under the viewport center so the status bar and page list follow scrolling.
    this.scroller.addEventListener('scroll', () => {
      if (this.programmaticScroll) {
        this.armScrollSettle();
        return;
      }
      if (this.scrollRaf) return;
      this.scrollRaf = requestAnimationFrame(() => {
        this.scrollRaf = 0;
        const page = this.findCenteredPage();
        if (page === null || page === this.lastReportedPage) return;
        this.lastReportedPage = page;
        this.activePageIndex = page;
        this.virtualizer.updateActiveWindow(page);
        this.syncActiveVirtualWindow();
        this.callbacks.onPageChange(page);
      });
    }, { passive: true });
  }

  private flowTitle(flowId: string): string {
    const canonical = (flowId === 'main' || flowId === 'primary') ? 'gemara' : flowId;
    switch (canonical) {
      case 'gemara':
        return 'טקסט ראשי';
      case 'rashi':
        return 'רש"י';
      case 'tosafot':
        return 'תוספות';
      case 'notes':
        return 'הערות ומסורת';
      default:
        return flowId;
    }
  }
}


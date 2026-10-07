import { PageDescriptor } from 'tok-viewer';
import { SelectionMode, TextFrameData } from '../types';
import { t, tf, i18n } from '../i18n';
import { el, iconButton, button } from '../ui';

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
export function fitZoom(viewportWidth: number, viewportHeight: number): number {
  const z = Math.min((viewportWidth - 80) / (2 * PAGE_WIDTH_PX), (viewportHeight - 136) / PAGE_HEIGHT_PX) * 100;
  if (!Number.isFinite(z)) return 100;
  return Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, Math.floor(z)));
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
  private innerContainer!: HTMLElement;
  /** Takes the scaled size of the pages so the scroll area matches what is drawn. */
  private zoomBox!: HTMLElement;
  private lastReportedPage = -1;
  private scrollRaf = 0;
  private programmaticScroll = false;
  private scrollSettleTimer = 0;
  private baselineGridPt = 13;

  constructor(callbacks: SpreadCanvasCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-canvas-shell');
    this.scroller = el('main', 'tok-canvas-container', { dir: 'rtl', tabindex: '-1' });
    this.element.appendChild(this.scroller);

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

  public setPages(pages: PageDescriptor[], activeIndex = 0): void {
    this.pages = pages;
    this.activePageIndex = activeIndex;
    this.lastReportedPage = activeIndex;
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
    this.applyUserZoom(fitZoom(this.scroller.clientWidth, this.scroller.clientHeight));
  }

  private applyUserZoom(z: number): void {
    this.setZoom(z);
    this.callbacks.onZoomChange?.(this.zoomPercent);
    requestAnimationFrame(() => this.revealPage(this.activePageIndex, false, true));
  }

  public setBaselineGrid(pt: number): void {
    if (!(pt > 0)) return;
    this.baselineGridPt = pt;
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
      .map((p) => `<p data-para-id="${p.id}" class="${p.styleId ? `tok-style-${p.styleId}` : ''}" style="margin:0 0 0.6em;line-height:1.55;">${p.text || '&nbsp;'}</p>`)
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
      style: 'position:absolute;bottom:20px;inset-inline-end:24px;z-index:25;display:flex;align-items:center;gap:4px;padding:4px 6px;background:var(--tok-bg-surface-1);border:1px solid var(--tok-border-strong);border-radius:24px;box-shadow:0 4px 18px rgba(0,0,0,0.18);backdrop-filter:blur(8px);'
    });

    const zoomOut = iconButton('minus', 'הקטנת תצוגה (Ctrl+-)', () => this.applyUserZoom(this.zoomPercent - 10), {
      size: 13,
      attrs: { style: 'width:26px;height:26px;border-radius:50%;' }
    });
    this.zoomHud.appendChild(zoomOut);

    this.zoomDisplayBtn = el('button', 'tok-btn tok-btn-ghost tok-btn-xs', {
      type: 'button',
      title: 'איפוס ל-100% (Ctrl+0)',
      style: 'font-size:11px;font-weight:600;min-width:44px;padding:0 4px;height:26px;'
    }, `${this.zoomPercent}%`) as HTMLButtonElement;
    this.zoomDisplayBtn.addEventListener('click', () => {
      if (this.zoomPercent === 100) this.fitToWindow();
      else this.applyUserZoom(100);
    });
    this.zoomHud.appendChild(this.zoomDisplayBtn);

    const zoomIn = iconButton('plus', 'הגדלת תצוגה (Ctrl++)', () => this.applyUserZoom(this.zoomPercent + 10), {
      size: 13,
      attrs: { style: 'width:26px;height:26px;border-radius:50%;' }
    });
    this.zoomHud.appendChild(zoomIn);

    const fitBtn = button('התאם', {
      className: 'tok-btn tok-btn-ghost tok-btn-xs',
      attrs: { title: 'התאמה מלאה לחלון', style: 'font-size:11px;padding:0 6px;height:26px;' },
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
    if (this.pages.length === 0) return;

    for (const spread of groupIntoSpreads(this.pages.length)) {
      const spreadRow = el('div', spread.length === 1 ? 'tok-spread-row tok-single' : 'tok-spread-row');
      // The canvas is dir="rtl", so the first sheet appended lands on the right.
      // The lone first page is a recto and therefore belongs on the left.
      if (spread.length === 1 && isRectoPage(spread[0])) {
        spreadRow.style.justifyContent = 'flex-end';
        spreadRow.style.width = `${2 * PAGE_WIDTH_PX}px`;
      }
      for (const pos of spread) {
        spreadRow.appendChild(this.createPageSheet(this.pages[pos], pos));
      }
      this.innerContainer.appendChild(spreadRow);
    }

    this.updateZoomBox();
    this.updateGuides();
  }

  private createPageSheet(page: PageDescriptor, position: number): HTMLElement {
    const isRecto = isRectoPage(position);
    const isRightPage = isRightHandPage(position);
    const amud = isRecto ? 'ע״א' : 'ע״ב';
    const sheet = el('article', 'tok-page-sheet', { 'data-page-index': page.pageIndex, 'aria-label': `${tf('structurePageLabel', { g: page.gematriaNumber })} ${amud}` });

    // Click on page background clears frame selection (zero selection).
    sheet.addEventListener('click', (e) => {
      if (e.target === sheet) {
        this.clearFrameSelection();
        this.callbacks.onSelectionModeChange('zero');
        this.callbacks.onDismissActionHud();
      }
    });

    // Shading along the spine (the inner edge).
    const spine = el('div', 'tok-page-spine', { 'aria-hidden': 'true' });
    spine.style[isRightPage ? 'left' : 'right'] = '0';
    spine.style.background = `linear-gradient(to ${isRightPage ? 'left' : 'right'}, rgba(40,30,10,.10), rgba(40,30,10,0))`;
    sheet.appendChild(spine);

    // 1. Running head: page number on the outer corner.
    const head = el('div', 'tok-page-head');
    const pageNo = el('span', undefined, undefined, `דף ${page.gematriaNumber} ${amud}`);
    if (isRightPage) head.append(pageNo);
    else head.append(pageNo);
    sheet.appendChild(head);

    // 2. Page body: rendered document content or clean interactive editable text frame.
    if (page.htmlContent && page.htmlContent.trim().length > 0) {
      const content = el('div', 'tok-page-content', { style: 'flex: 1; min-height: 0; overflow: hidden;' });
      content.innerHTML = page.htmlContent;
      sheet.appendChild(content);

      // Bind interactive clicks on all frames inside the rendered multi-flow content
      const frames = content.querySelectorAll<HTMLElement>('.tok-interactive-frame');
      frames.forEach((fr) => {
        fr.addEventListener('click', (e) => {
          e.stopPropagation();
          const flowId = (fr.dataset.flowId || 'gemara') as FlowId;
          const frameId = fr.dataset.frameId || `frame-${page.pageIndex}-${flowId}`;
          const target = e.target as HTMLElement | null;
          const paraEl = target?.closest('[data-para-id]') as HTMLElement | null;
          const paraId = paraEl?.dataset?.paraId;

          this.selectFrame(fr, {
            id: frameId,
            title: this.flowTitle(flowId),
            flowId,
            className: '',
            text: ''
          });

          this.callbacks.onSelectFrameFlow?.(flowId, paraId);

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
              xMm: 35,
              yMm: 45,
              widthMm: 110,
              heightMm: 240,
              rotationDeg: 0,
              flowId,
              columns: 1,
              columnGapMm: 0,
              insetTopMm: 3,
              insetBottomMm: 3,
              insetRightMm: 4,
              insetLeftMm: 4,
              verticalAlign: 'top',
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
    sheet.appendChild(el('div', 'tok-page-foot', { 'aria-hidden': 'true' }, `- ${page.gematriaNumber} -`));

    // Guides (margin rectangle, baseline grid).
    const guides = el('div', 'tok-guides-layer', { 'aria-hidden': 'true' });
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
    const frame = el('div', 'tok-interactive-frame', { 'data-frame-id': params.id, 'data-flow-id': params.flowId });
    const inner = el('div', `tok-frame-text ${params.className}`);
    inner.innerHTML = (params.label ? `<div class="tok-frame-label">${params.label}</div>` : '') + (params.text || '');
    frame.appendChild(inner);

    frame.addEventListener('click', (e) => {
      e.stopPropagation();
      this.selectFrame(frame, params);

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
          xMm: 35,
          yMm: 45,
          widthMm: 110,
          heightMm: 240,
          rotationDeg: 0,
          flowId: params.flowId,
          columns: 1,
          columnGapMm: 0,
          insetTopMm: 3,
          insetBottomMm: 3,
          insetRightMm: 4,
          insetLeftMm: 4,
          verticalAlign: 'top',
          text: params.text
        });
      }
    });

    return frame;
  }

  private selectFrame(frame: HTMLElement, params: FrameParams): void {
    this.clearFrameSelection();
    this.selectedFrameId = params.id;
    frame.classList.add('tok-selected');
    frame.appendChild(el('span', 'tok-frame-tag', { 'aria-hidden': 'true' }, params.title));

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
  }

  private updateGuides(): void {
    this.innerContainer.querySelectorAll<HTMLElement>('.tok-guide-margin-rect').forEach((g) => {
      g.style.display = this.showMargins ? 'block' : 'none';
    });

    const stepPx = (this.baselineGridPt * 96) / 72;
    const line = 'var(--tok-guide-baseline)';
    this.innerContainer.querySelectorAll<HTMLElement>('.tok-guide-baseline').forEach((g) => {
      g.style.display = this.showBaseline ? 'block' : 'none';
      g.style.backgroundImage =
        `repeating-linear-gradient(to bottom, transparent 0, transparent ${stepPx - 1}px, ${line} ${stepPx - 1}px, ${line} ${stepPx}px)`;
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
        this.callbacks.onPageChange(page);
      });
    }, { passive: true });
  }

  private flowTitle(flowId: string): string {
    switch (flowId) {
      case 'gemara':
        return 'גמרא (טקסט ראשי)';
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


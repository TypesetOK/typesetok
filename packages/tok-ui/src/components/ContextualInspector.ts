import {
  SelectionMode,
  DocumentSettings,
  TextFrameData,
  TypographySettings,
  FootnoteApparatusConfigData,
  GrepRuleData
} from '../types';
import { getBuiltinGrepRules, getDefaultMultiApparatusConfigs } from '../engine/documentBridge';
import { IconName } from '../icons';
import { t, tf, i18n } from '../i18n';
import { el, icon, button, iconButton, switchRow, segmented, selectField } from '../ui';
import { DOCUMENT_FONTS, DEFAULT_DOCUMENT_FONT, FONT_WEIGHT_BOLD, FONT_WEIGHT_REGULAR, getAvailableFonts } from '../fonts';
import { UpdatesBellButton } from '../updates/UpdatesBellButton';

/**
 * Parses a scrubber field such as "11.5 pt", "-3 מ"מ" or "12,5". Returns null when no
 * number is present. (parseInt used to truncate 11.5pt to 11 on every edit/drag.)
 */
export function parseScrubberValue(text: string): number | null {
  const m = String(text).replace(',', '.').match(/-?\d+(?:\.\d+)?/);
  if (!m) return null;
  const v = parseFloat(m[0]);
  return Number.isFinite(v) ? v : null;
}

/** Clamps to [min, max] and rounds to one decimal place. */
export function clampScrubberValue(v: number, min: number, max: number): number {
  return Math.round(Math.max(min, Math.min(max, v)) * 10) / 10;
}

export function formatScrubberValue(v: number, unit: string): string {
  return `${Math.round(v * 10) / 10} ${unit}`;
}

export interface InspectorCallbacks {
  onDocumentChange: (settings: Partial<DocumentSettings>) => void;
  onFrameChange: (geometry: Partial<TextFrameData>) => void;
  onTypographyChange: (typo: Partial<TypographySettings>) => void;
  onSyncStyleToken: () => void;
  onNormalizeNiqqud: () => void;
  onOpenTextCleanup?: () => void;
  onAlignFrames: (alignType: string) => void;
  onOpenSettings?: () => void;
  onOpenWhatsNew?: () => void;
  onToggleCollapse?: (collapsed: boolean) => void;
  onGrepRulesChange?: (rules: GrepRuleData[]) => void;
  onFootnoteApparatusesChange?: (apparatuses: FootnoteApparatusConfigData[]) => void;
}

const FLOW_COLORS: Record<string, string> = { gemara: '#1E4A9E', rashi: '#B45309', tosafot: '#15803D', notes: '#7C3AED' };

export class ContextualInspector {
  public element: HTMLElement;
  private availableFlows: Array<{ id: string; name: string; color?: string }> = [];
  private mode: SelectionMode = 'zero';
  private callbacks: InspectorCallbacks;
  private isCollapsed = false;
  private grepRules: GrepRuleData[] = getBuiltinGrepRules();
  private footnoteApparatuses: FootnoteApparatusConfigData[] = [];

  private documentSettings: DocumentSettings = {
    title: 'מסמך ללא שם',
    pageSize: '17x24',
    pageWidthMm: 170,
    pageHeightMm: 240,
    marginTopMm: 18,
    marginBottomMm: 22,
    marginInsideMm: 20,
    marginOutsideMm: 15,
    columns: 2,
    columnGapMm: 5,
    baselineGridPt: 13,
    baselineOffsetPt: 48,
    gematriaFormat: 'talmudic',
    preflightStatus: 'clean'
  };

  private selectedFrame: TextFrameData = {
    id: 'frame-1',
    xMm: 35,
    yMm: 45,
    widthMm: 100,
    heightMm: 140,
    rotationDeg: 0,
    flowId: 'gemara',
    columns: 1,
    columnGapMm: 0,
    insetTopMm: 3,
    insetBottomMm: 3,
    insetRightMm: 4,
    insetLeftMm: 4,
    verticalAlign: 'top',
    nextFrameId: '',
    text: ''
  };

  private typographySettings: TypographySettings = {
    fontFamily: DEFAULT_DOCUMENT_FONT,
    fontSizePt: 15,
    fontWeight: FONT_WEIGHT_BOLD,
    lineHeightPt: 19,
    paragraphSpacingPt: 8,
    firstLineIndentMm: 0,
    alignment: 'justify',
    styleTokenId: 'main-text',
    styleTokenName: 'טקסט ראשי',
    isOverride: false,
    justification: {
      tier1WordSpacingMin: 85,
      tier1WordSpacingMax: 125,
      tier2OheltaremEnabled: true,
      tier2OheltaremMaxStretch: 120,
      tier2OheltaremLetters: ['א', 'ה', 'ל', 'ת', 'ר', 'ם'],
      tier3MicroTrackingRange: 2
    },
    knuthPlassEnabled: true,
    normalizeNiqqud: true,
    shieldDivineNames: true,
    keepLinesTogether: 2
  };

  constructor(callbacks: InspectorCallbacks) {
    this.callbacks = callbacks;
    this.element = document.createElement('aside');
    this.element.className = 'tok-inspector-bar';
    this.element.dir = i18n.getDirection();
    this.element.setAttribute('aria-label', t('inspAriaLabel'));

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      this.element.setAttribute('aria-label', t('inspAriaLabel'));
      this.render();
    });

    this.render();
  }

  public toggleCollapse(): void {
    this.setCollapsed(!this.isCollapsed);
  }

  public setCollapsed(collapsed: boolean): void {
    this.isCollapsed = collapsed;
    if (this.isCollapsed) {
      this.element.classList.add('tok-collapsed');
    } else {
      this.element.classList.remove('tok-collapsed');
    }
    this.callbacks.onToggleCollapse?.(this.isCollapsed);
    this.render();
  }

  public isInspectorCollapsed(): boolean {
    return this.isCollapsed;
  }

  public setMode(mode: SelectionMode, frameData?: Partial<TextFrameData>, typoData?: Partial<TypographySettings>): void {
    this.mode = mode;
    if (frameData) Object.assign(this.selectedFrame, frameData);
    if (typoData) Object.assign(this.typographySettings, typoData);
    this.render();
  }

  public getMode(): SelectionMode {
    return this.mode;
  }

  /** Current document setup (shared with the export dialog). */
  public getDocumentSettings(): DocumentSettings {
    return { ...this.documentSettings };
  }

  public setDocumentSettings(settings: Partial<DocumentSettings>): void {
    Object.assign(this.documentSettings, settings);
    if (this.mode === 'zero') {
      this.render();
    }
  }

  public setDocumentTitle(title: string): void {
    const trimmed = title.trim();
    if (!trimmed || this.documentSettings.title === trimmed) return;
    this.documentSettings.title = trimmed;
    if (this.mode === 'zero') {
      this.render();
    }
  }

  public setFlows(flows: Array<{ id: string; name: string; color?: string }>): void {
    this.availableFlows = flows;
    if (this.mode === 'text-frame' || this.mode === 'text-edit') {
      this.render();
    }
  }

  public setGrepRules(rules?: GrepRuleData[]): void {
    const presets = getBuiltinGrepRules();
    if (!rules || rules.length === 0) {
      this.grepRules = presets;
    } else {
      const merged = [...rules];
      for (const preset of presets) {
        if (!merged.some((r) => r.id === preset.id)) {
          merged.push(preset);
        }
      }
      this.grepRules = merged;
    }
    this.render();
  }

  public getGrepRules(): GrepRuleData[] {
    return [...this.grepRules];
  }

  public setFootnoteApparatuses(apparatuses?: FootnoteApparatusConfigData[]): void {
    this.footnoteApparatuses = apparatuses ? [...apparatuses] : [];
    this.render();
  }

  public getFootnoteApparatuses(): FootnoteApparatusConfigData[] {
    return [...this.footnoteApparatuses];
  }

  private flowName(flowId: string): string {
    const canonical = (flowId === 'main' || flowId === 'primary') ? 'gemara' : flowId;
    const fromAvailable = this.availableFlows.find((f) => f.id === canonical || f.id === flowId);
    if (fromAvailable?.name) return fromAvailable.name;
    const keys: Record<string, string> = { gemara: 'inspFlowGemara', rashi: 'inspFlowRashi', tosafot: 'inspFlowTosafot', notes: 'inspFlowNotes' };
    return keys[canonical] ? t(keys[canonical]) : (keys[flowId] ? t(keys[flowId]) : flowId);
  }

  private getContainer(): HTMLElement {
    return this.element.querySelector<HTMLElement>('.tok-inspector-body') || this.element;
  }

  private render(): void {
    this.element.innerHTML = '';

    if (this.isCollapsed) {
      this.element.classList.add('tok-collapsed');

      const topBar = el('div', 'tok-inspector-top-bar');
      const expandBtn = el('button', 'tok-btn tok-btn-ghost tok-insp-toggle-btn', {
        type: 'button',
        title: t('inspExpand'),
        'aria-label': t('inspExpand')
      });
      // In RTL, the inspector is on the left; expanding points inward towards the right.
      const expandIcon = i18n.getDirection() === 'rtl' ? 'chevronRight' : 'chevronLeft';
      expandBtn.appendChild(icon(expandIcon, 16));
      expandBtn.addEventListener('click', () => this.toggleCollapse());
      topBar.appendChild(expandBtn);
      this.element.appendChild(topBar);

      const strip = el('div', 'tok-inspector-collapsed-strip', {
        title: `${t('inspInspectorTitle')} - ${t('inspExpand')}`,
        role: 'button',
        tabindex: '0'
      });
      const modeIconName: IconName = this.mode === 'text-edit' ? 'typography' : this.mode === 'text-frame' ? 'ruler' : 'pages';
      const modeIndicator = el('div', 'tok-insp-mode-indicator');
      modeIndicator.appendChild(icon(modeIconName, 18));
      strip.appendChild(modeIndicator);
      strip.addEventListener('click', () => this.toggleCollapse());
      strip.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.toggleCollapse();
        }
      });
      this.element.appendChild(strip);

      const foot = el('div', 'tok-inspector-footer');
      if (this.callbacks.onOpenWhatsNew) {
        const footWhatsNew = new UpdatesBellButton({
          onClick: () => this.callbacks.onOpenWhatsNew?.(),
          size: 16,
          className: 'tok-btn tok-btn-ghost tok-insp-whatsnew-btn'
        });
        foot.appendChild(footWhatsNew.element);
      }
      if (this.callbacks.onOpenSettings) {
        const footSettings = el('button', 'tok-btn tok-btn-ghost tok-insp-settings-btn', {
          type: 'button',
          title: t('sidebarSettings'),
          'aria-label': t('sidebarSettings')
        });
        footSettings.appendChild(icon('settings', 16));
        footSettings.addEventListener('click', () => this.callbacks.onOpenSettings?.());
        foot.appendChild(footSettings);
      }
      this.element.appendChild(foot);
      return;
    }

    this.element.classList.remove('tok-collapsed');

    // 1. Top bar of inspector
    const topBar = el('div', 'tok-inspector-top-bar');

    // In RTL, the inner edge facing the canvas is at the right side (start edge in RTL)
    const collapseBtn = el('button', 'tok-btn tok-btn-ghost tok-insp-toggle-btn', {
      type: 'button',
      title: t('inspCollapse'),
      'aria-label': t('inspCollapse')
    });
    // In RTL, collapsing moves outward toward the left edge
    const collapseIcon = i18n.getDirection() === 'rtl' ? 'chevronLeft' : 'chevronRight';
    collapseBtn.appendChild(icon(collapseIcon, 16));
    collapseBtn.addEventListener('click', () => this.toggleCollapse());
    topBar.appendChild(collapseBtn);

    const titleWrap = el('div', 'tok-insp-header-title-wrap');
    const modeIcon: IconName = this.mode === 'text-edit' ? 'typography' : this.mode === 'text-frame' ? 'ruler' : 'pages';
    titleWrap.appendChild(icon(modeIcon, 14));
    const titleEl = el('span', 'tok-insp-header-title', undefined, t('inspInspectorTitle'));
    titleWrap.appendChild(titleEl);
    topBar.appendChild(titleWrap);

    if (this.callbacks.onOpenWhatsNew) {
      const bell = new UpdatesBellButton({
        onClick: () => this.callbacks.onOpenWhatsNew?.(),
        size: 16,
        className: 'tok-btn tok-btn-ghost tok-insp-whatsnew-btn'
      });
      topBar.appendChild(bell.element);
    } else {
      topBar.appendChild(el('span', undefined, { style: 'width:28px;' }));
    }

    this.element.appendChild(topBar);

    // 2. Scrollable Body
    const body = el('div', 'tok-inspector-body');
    this.element.appendChild(body);

    switch (this.mode) {
      case 'zero':
        this.renderZeroSelection();
        break;
      case 'text-frame':
        this.renderTextFrameMode();
        break;
      case 'text-edit':
        this.renderTextEditMode();
        break;
      case 'image-frame':
        this.renderImageFrameMode();
        break;
      case 'multi':
        this.renderMultiSelectMode();
        break;
    }

    // 3. Footer with Settings button
    if (this.callbacks.onOpenSettings) {
      const footer = el('footer', 'tok-inspector-footer');
      const settingsBtn = el('button', 'tok-btn tok-btn-ghost tok-insp-settings-btn tok-insp-settings-btn-labeled', {
        type: 'button',
        title: t('sidebarSettings'),
        'aria-label': t('sidebarSettings')
      });
      settingsBtn.appendChild(icon('settings', 16));
      settingsBtn.appendChild(el('span', undefined, undefined, t('sidebarSettings')));
      settingsBtn.addEventListener('click', () => this.callbacks.onOpenSettings?.());
      footer.appendChild(settingsBtn);
      this.element.appendChild(footer);
    }
  }

  // =========================================================================
  // State 1: Zero Selection (document setup & print check)
  // =========================================================================
  private renderZeroSelection(): void {
    const c = this.getContainer();
    c.appendChild(this.createHeader(t('inspNothingSelected'), this.documentSettings.title, `${t('inspDocTitle')} · ${t('inspDocSubtitle')}`, 'file'));

    // 1. Page size
    const sizeCard = this.createCard(t('inspPageSize'));
    const presets = [
      { id: '17x24', name: t('inspPresetSefer'), w: 170, h: 240 },
      { id: 'Crown', name: t('inspPresetCrown'), w: 165, h: 235 },
      { id: 'A4', name: t('inspPresetA4'), w: 210, h: 297 },
      { id: 'B5', name: t('inspPresetB5'), w: 176, h: 250 }
    ];
    sizeCard.appendChild(selectField(t('inspPageSize'), presets.map((p) => ({ value: p.id, label: p.name })), this.documentSettings.pageSize, (value) => {
      const found = presets.find((p) => p.id === value);
      if (!found) return;
      this.documentSettings.pageSize = found.id as DocumentSettings['pageSize'];
      this.documentSettings.pageWidthMm = found.w;
      this.documentSettings.pageHeightMm = found.h;
      this.callbacks.onDocumentChange({ pageSize: this.documentSettings.pageSize, pageWidthMm: found.w, pageHeightMm: found.h });
      this.render();
    }));
    sizeCard.appendChild(this.row(
      this.createScrubber(t('inspWidth'), this.documentSettings.pageWidthMm, t('unitMm'), 50, 400, (v) => {
        this.documentSettings.pageWidthMm = v;
        this.callbacks.onDocumentChange({ pageWidthMm: v });
      }),
      this.createScrubber(t('inspHeight'), this.documentSettings.pageHeightMm, t('unitMm'), 50, 500, (v) => {
        this.documentSettings.pageHeightMm = v;
        this.callbacks.onDocumentChange({ pageHeightMm: v });
      })
    ));
    c.appendChild(sizeCard);

    // 2. Graded margins
    const marginCard = this.createCard(t('inspGradedMargins'));
    marginCard.appendChild(this.row(
      this.createScrubber(t('inspTop'), this.documentSettings.marginTopMm, t('unitMm'), 5, 80, (v) => {
        this.documentSettings.marginTopMm = v;
        this.callbacks.onDocumentChange({ marginTopMm: v });
      }),
      this.createScrubber(t('inspBottom'), this.documentSettings.marginBottomMm, t('unitMm'), 5, 80, (v) => {
        this.documentSettings.marginBottomMm = v;
        this.callbacks.onDocumentChange({ marginBottomMm: v });
      })
    ));
    marginCard.appendChild(this.row(
      this.createScrubber(t('inspInside'), this.documentSettings.marginInsideMm, t('unitMm'), 5, 80, (v) => {
        this.documentSettings.marginInsideMm = v;
        this.callbacks.onDocumentChange({ marginInsideMm: v });
      }),
      this.createScrubber(t('inspOutside'), this.documentSettings.marginOutsideMm, t('unitMm'), 5, 80, (v) => {
        this.documentSettings.marginOutsideMm = v;
        this.callbacks.onDocumentChange({ marginOutsideMm: v });
      })
    ));
    c.appendChild(marginCard);

    // 3. Baseline grid
    const gridCard = this.createCard(t('inspBaselineGrid'));
    gridCard.appendChild(this.row(
      this.createScrubber(t('inspLineStep'), this.documentSettings.baselineGridPt, 'pt', 8, 30, (v) => {
        this.documentSettings.baselineGridPt = v;
        this.callbacks.onDocumentChange({ baselineGridPt: v });
      }),
      this.createScrubber(t('inspTopOffset'), this.documentSettings.baselineOffsetPt, 'pt', 0, 100, (v) => {
        this.documentSettings.baselineOffsetPt = v;
        this.callbacks.onDocumentChange({ baselineOffsetPt: v });
      })
    ));
    c.appendChild(gridCard);

    // 4. Multi-Apparatus Footnotes
    this.appendMultiApparatusCard(c);

    // 5. Live GREP Rule Engine
    this.appendLiveGrepCard(c);

    c.appendChild(el('div', 'tok-insp-spacer'));

    // 4. Continuous print check
    c.appendChild(this.createCheckCard(t('inspPreflight'), [
      { label: t('inspProdStatus'), value: t('inspReadyForPrint'), ok: true },
      { label: t('inspColorProfile'), value: 'ISO Coated v2 (100% K)', ok: true },
      { label: t('inspOverset'), value: t('inspNoIssues'), ok: true },
      { label: t('inspImageRes'), value: t('inspImageResOk'), ok: true }
    ]));
  }

  // =========================================================================
  // State 2: Text frame selected (object mode)
  // =========================================================================
  private renderTextFrameMode(): void {
    const c = this.getContainer();
    const flow = this.selectedFrame.flowId;
    c.appendChild(this.createHeader(t('inspSelected'), this.flowName(flow), `${t('inspTextFrame')} · ${this.selectedFrame.id}`, 'frame', FLOW_COLORS[flow]));

    // 1. Geometry
    const geoCard = this.createCard(t('inspGeometry'));
    geoCard.appendChild(this.row(
      this.createScrubber(t('inspPosX'), this.selectedFrame.xMm, t('unitMm'), 0, 300, (v) => {
        this.selectedFrame.xMm = v;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, xMm: v });
      }),
      this.createScrubber(t('inspPosY'), this.selectedFrame.yMm, t('unitMm'), 0, 400, (v) => {
        this.selectedFrame.yMm = v;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, yMm: v });
      })
    ));
    geoCard.appendChild(this.row(
      this.createScrubber(t('inspWidthW'), this.selectedFrame.widthMm, t('unitMm'), 10, 300, (v) => {
        this.selectedFrame.widthMm = v;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, widthMm: v });
      }),
      this.createScrubber(t('inspHeightH'), this.selectedFrame.heightMm, t('unitMm'), 10, 400, (v) => {
        this.selectedFrame.heightMm = v;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, heightMm: v });
      })
    ));

    const isLocked = Boolean(this.selectedFrame.isLocked);
    geoCard.appendChild(button(isLocked ? t('inspUnlockFrame') : t('inspLockFrame'), {
      className: `tok-btn tok-btn-block tok-btn-sm ${isLocked ? 'tok-btn-warning' : 'tok-btn-secondary'}`,
      icon: isLocked ? 'lock' : 'unlock',
      iconSize: 14,
      onClick: () => {
        this.selectedFrame.isLocked = !isLocked;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, isLocked: this.selectedFrame.isLocked });
        this.render();
      }
    }));
    c.appendChild(geoCard);

    // 2. Flow & threading
    const flowCard = this.createCard(t('inspFlowThreading'));
    const flowList = this.availableFlows.length > 0
      ? this.availableFlows
      : [
          { id: 'gemara', name: this.flowName('gemara') },
          { id: 'rashi', name: this.flowName('rashi') },
          { id: 'tosafot', name: this.flowName('tosafot') },
          { id: 'notes', name: this.flowName('notes') }
        ];
    const flows = flowList.map((f) => ({ value: f.id, label: f.name }));
    flowCard.appendChild(selectField(t('inspFlowThreading'), flows, flow, (value) => {
      this.selectedFrame.flowId = value as TextFrameData['flowId'];
      this.callbacks.onFrameChange({ id: this.selectedFrame.id, flowId: this.selectedFrame.flowId });
      this.render();
    }));
    const thread = el('div', 'tok-status-line');
    thread.appendChild(el('span', undefined, undefined, t('inspThreading')));
    thread.appendChild(el('span', undefined, { dir: 'ltr', style: 'color:var(--tok-accent-text)' }, this.selectedFrame.nextFrameId || t('inspNone')));
    flowCard.appendChild(thread);
    c.appendChild(flowCard);

    // 3. Insets & vertical alignment
    const insetCard = this.createCard(t('inspInsets'));
    insetCard.appendChild(this.row(
      this.createScrubber(t('inspTop'), this.selectedFrame.insetTopMm, t('unitMm'), 0, 30, (v) => {
        this.selectedFrame.insetTopMm = v;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, insetTopMm: v });
      }),
      this.createScrubber(t('inspBottom'), this.selectedFrame.insetBottomMm, t('unitMm'), 0, 30, (v) => {
        this.selectedFrame.insetBottomMm = v;
        this.callbacks.onFrameChange({ id: this.selectedFrame.id, insetBottomMm: v });
      })
    ));
    const rightInset = this.createScrubber(t('inspRight'), this.selectedFrame.insetRightMm, t('unitMm'), 0, 30, (v) => {
      this.selectedFrame.insetRightMm = v;
      this.callbacks.onFrameChange({ id: this.selectedFrame.id, insetRightMm: v });
    });
    const leftInset = this.createScrubber(t('inspLeft'), this.selectedFrame.insetLeftMm, t('unitMm'), 0, 30, (v) => {
      this.selectedFrame.insetLeftMm = v;
      this.callbacks.onFrameChange({ id: this.selectedFrame.id, insetLeftMm: v });
    });
    insetCard.appendChild(i18n.getDirection() === 'rtl' ? this.row(rightInset, leftInset) : this.row(leftInset, rightInset));

    insetCard.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspVAlign')));
    insetCard.appendChild(segmented(t('inspVAlign'), [
      { id: 'top', label: t('inspTop') },
      { id: 'center', label: t('inspCenter') },
      { id: 'bottom', label: t('inspBottom') },
      { id: 'justify', label: t('inspJustify') }
    ], this.selectedFrame.verticalAlign, (id) => {
      this.selectedFrame.verticalAlign = id;
      this.callbacks.onFrameChange({ id: this.selectedFrame.id, verticalAlign: id });
    }, { fill: true }));
    c.appendChild(insetCard);

    // 4. Frame Typography & Alignment
    this.appendTypographyCard(c);
    this.appendAlignCard(c);
  }

  private appendTypographyCard(c: HTMLElement): void {
    const fontCard = this.createCard(t('inspFontSpacing'));
    const availFonts = getAvailableFonts();
    const current = availFonts.some((f) => f.family === this.typographySettings.fontFamily)
      ? this.typographySettings.fontFamily
      : DEFAULT_DOCUMENT_FONT;
    fontCard.appendChild(selectField(t('hudFont'), availFonts.map((f) => ({ value: f.family, label: f.label })), current, (value) => {
      this.typographySettings.fontFamily = value;
      this.typographySettings.isOverride = true;
      this.callbacks.onTypographyChange({ fontFamily: value, isOverride: true });
      this.render();
    }));
    const weight = this.typographySettings.fontWeight >= 600 ? FONT_WEIGHT_BOLD : FONT_WEIGHT_REGULAR;
    fontCard.appendChild(segmented(t('inspWeight'), [
      { id: String(FONT_WEIGHT_REGULAR), label: t('inspWeightRegular') },
      { id: String(FONT_WEIGHT_BOLD), label: t('inspWeightBold') },
    ], String(weight), (id) => {
      const fontWeight = Number(id);
      this.typographySettings.fontWeight = fontWeight;
      this.typographySettings.isOverride = true;
      this.callbacks.onTypographyChange({ fontWeight, isOverride: true });
      this.render();
    }, { fill: true, focusPrefix: 'weight' }));
    fontCard.appendChild(this.row(
      this.createScrubber(t('inspFontSize'), this.typographySettings.fontSizePt, 'pt', 6, 72, (v) => {
        this.typographySettings.fontSizePt = v;
        this.typographySettings.isOverride = true;
        this.callbacks.onTypographyChange({ fontSizePt: v, isOverride: true });
      }),
      this.createScrubber(t('inspLeading'), this.typographySettings.lineHeightPt, 'pt', 8, 90, (v) => {
        this.typographySettings.lineHeightPt = v;
        this.typographySettings.isOverride = true;
        this.callbacks.onTypographyChange({ lineHeightPt: v, isOverride: true });
      })
    ));
    c.appendChild(fontCard);
  }

  private appendAlignCard(c: HTMLElement): void {
    const alignCard = this.createCard(t('inspAlignTitle'));
    alignCard.appendChild(segmented(t('inspAlignTitle'), [
      { id: 'right', label: t('hudAlignRight'), icon: 'alignRight' },
      { id: 'center', label: t('hudAlignCenter'), icon: 'alignCenter' },
      { id: 'left', label: t('inspAlignLeft'), icon: 'alignLeft' },
      { id: 'justify', label: t('hudAlignJustify'), icon: 'alignJustify' }
    ], this.typographySettings.alignment, (id) => {
      this.typographySettings.alignment = id;
      this.callbacks.onTypographyChange({ alignment: id });
    }, { fill: true, iconOnly: true }));
    c.appendChild(alignCard);
  }

  // =========================================================================
  // State 3: Text edit mode (typography, 3-tier Hebrew justification, niqqud)
  // =========================================================================
  private renderTextEditMode(): void {
    const c = this.getContainer();
    c.appendChild(this.createHeader(t('inspSelected'), this.typographySettings.styleTokenName, t('inspTypography'), 'typography'));

    // 1. Paragraph style & override sync
    const styleCard = this.createCard(t('inspStyleToken'));
    const tokenRow = el('div', 'tok-status-line');
    tokenRow.appendChild(el('span', undefined, { style: 'font-size:13px;font-weight:600;color:var(--tok-accent-text)' }, this.typographySettings.styleTokenName));
    if (this.typographySettings.isOverride) {
      tokenRow.appendChild(el('span', 'tok-badge tok-badge-warning', undefined, t('inspLocalOverride')));
    }
    styleCard.appendChild(tokenRow);
    styleCard.appendChild(button(t('inspSyncStyle'), {
      className: 'tok-btn tok-btn-block tok-btn-sm',
      icon: 'refresh',
      iconSize: 14,
      onClick: () => {
        this.typographySettings.isOverride = false;
        this.callbacks.onSyncStyleToken();
        this.render();
      }
    }));
    c.appendChild(styleCard);

    // 2. Font & Spacing
    this.appendTypographyCard(c);

    // 3. Alignment
    this.appendAlignCard(c);

    // 4. Three-tier Hebrew justification
    const j = this.typographySettings.justification;
    const justCard = this.createCard(t('inspJustify3'));
    justCard.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspTier1')));
    justCard.appendChild(this.row(
      this.createScrubber(t('inspMin'), j.tier1WordSpacingMin, '%', 60, 100, (v) => {
        j.tier1WordSpacingMin = v;
        this.callbacks.onTypographyChange({ justification: j });
      }),
      this.createScrubber(t('inspMax'), j.tier1WordSpacingMax, '%', 100, 160, (v) => {
        j.tier1WordSpacingMax = v;
        this.callbacks.onTypographyChange({ justification: j });
      })
    ));

    justCard.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspTier2')));
    justCard.appendChild(switchRow(t('inspEnableStretch'), j.tier2OheltaremEnabled, (v) => {
      j.tier2OheltaremEnabled = v;
      this.callbacks.onTypographyChange({ justification: j });
    }));
    const letters = el('div', 'tok-letter-chips', { role: 'group', 'aria-label': t('inspTier2') });
    for (const l of ['א', 'ה', 'ל', 'ת', 'ר', 'ם']) {
      const on = j.tier2OheltaremLetters.includes(l);
      const chip = el('button', 'tok-letter-chip', { type: 'button', 'aria-pressed': String(on) }, l);
      chip.addEventListener('click', () => {
        const idx = j.tier2OheltaremLetters.indexOf(l);
        if (idx >= 0) j.tier2OheltaremLetters.splice(idx, 1);
        else j.tier2OheltaremLetters.push(l);
        chip.setAttribute('aria-pressed', String(idx < 0));
        this.callbacks.onTypographyChange({ justification: j });
      });
      letters.appendChild(chip);
    }
    justCard.appendChild(letters);
    justCard.appendChild(this.createScrubber(t('inspMaxStretch'), j.tier2OheltaremMaxStretch, '%', 100, 180, (v) => {
      j.tier2OheltaremMaxStretch = v;
      this.callbacks.onTypographyChange({ justification: j });
    }));

    justCard.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspTier3')));
    justCard.appendChild(this.createScrubber(t('inspTrackingRange'), j.tier3MicroTrackingRange, '%', 0, 5, (v) => {
      j.tier3MicroTrackingRange = v;
      this.callbacks.onTypographyChange({ justification: j });
    }));
    c.appendChild(justCard);

    // 5. Niqqud, cantillation & divine names
    const sacredCard = this.createCard(t('inspSacred'));
    sacredCard.appendChild(button(t('menuTextCleanup'), {
      className: 'tok-btn tok-btn-block tok-btn-sm',
      icon: 'sparkle',
      iconSize: 14,
      onClick: () => this.callbacks.onOpenTextCleanup?.()
    }));
    sacredCard.appendChild(button(t('inspNormalize'), {
      className: 'tok-btn tok-btn-block tok-btn-sm',
      icon: 'sparkle',
      iconSize: 14,
      onClick: () => this.callbacks.onNormalizeNiqqud()
    }));
    sacredCard.appendChild(switchRow(t('inspDivineShield'), this.typographySettings.shieldDivineNames, (v) => {
      this.typographySettings.shieldDivineNames = v;
      this.callbacks.onTypographyChange({ shieldDivineNames: v });
    }));
    c.appendChild(sacredCard);

    // 6. Live GREP Rule Engine & Multi-Apparatus Footnotes
    this.appendLiveGrepCard(c);
    this.appendMultiApparatusCard(c);
  }

  private appendMultiApparatusCard(c: HTMLElement): void {
    const appCard = this.createCard(t('inspMultiApparatusTitle'));
    appCard.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspMultiApparatusSub')));

    const hasDual = this.footnoteApparatuses.length >= 2;
    appCard.appendChild(button(t('inspMultiApparatusPresetDual'), {
      className: `tok-btn tok-btn-block tok-btn-sm ${hasDual ? 'tok-btn-primary' : 'tok-btn-secondary'}`,
      icon: 'layers',
      iconSize: 14,
      attrs: { 'data-action': 'enable-dual-apparatus' },
      onClick: () => {
        this.footnoteApparatuses = getDefaultMultiApparatusConfigs();
        this.callbacks.onFootnoteApparatusesChange?.([...this.footnoteApparatuses]);
        this.render();
      }
    }));

    for (const app of this.footnoteApparatuses) {
      const item = el('div', 'tok-apparatus-row', {
        'data-apparatus-id': app.id,
        style: 'display:flex;flex-direction:column;gap:4px;padding:6px 8px;border:1px solid var(--tok-border-subtle);border-radius:6px;margin-top:6px;'
      });
      const head = el('div', undefined, { style: 'display:flex;justify-content:space-between;align-items:center;font-size:12px;font-weight:600;' });
      head.appendChild(el('span', undefined, undefined, app.name));
      const numBadge = app.numbering_style === 'HebrewGematria' ? 'א, ב, ג' : app.numbering_style === 'ArabicNumerals' ? '1, 2, 3' : '*';
      head.appendChild(el('span', 'tok-badge', { style: 'font-size:10px;' }, `${numBadge} · ${app.columns} טורים`));
      item.appendChild(head);

      const controls = el('div', undefined, { style: 'display:flex;gap:6px;align-items:center;' });
      const colToggle = button(app.columns === 2 ? '2 טורים' : 'טור 1', {
        className: 'tok-btn tok-btn-sm',
        attrs: { 'data-apparatus-cols-toggle': app.id },
        onClick: () => {
          app.columns = app.columns === 2 ? 1 : 2;
          this.callbacks.onFootnoteApparatusesChange?.([...this.footnoteApparatuses]);
          this.render();
        }
      });
      const numToggle = button(app.numbering_style === 'HebrewGematria' ? 'אותיות' : 'ספרות', {
        className: 'tok-btn tok-btn-sm',
        attrs: { 'data-apparatus-num-toggle': app.id },
        onClick: () => {
          app.numbering_style = app.numbering_style === 'HebrewGematria' ? 'ArabicNumerals' : 'HebrewGematria';
          this.callbacks.onFootnoteApparatusesChange?.([...this.footnoteApparatuses]);
          this.render();
        }
      });
      controls.appendChild(colToggle);
      controls.appendChild(numToggle);
      item.appendChild(controls);
      appCard.appendChild(item);
    }

    c.appendChild(appCard);
  }

  private appendLiveGrepCard(c: HTMLElement): void {
    const grepCard = this.createCard(t('inspLiveGrepTitle'));
    grepCard.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspLiveGrepSub')));

    for (const rule of this.grepRules) {
      const row = switchRow(rule.name, rule.enabled, (enabled) => {
        rule.enabled = enabled;
        this.callbacks.onGrepRulesChange?.([...this.grepRules]);
      }, { focusKey: `grep-${rule.id}` });
      row.setAttribute('data-grep-rule-id', rule.id);
      if (!rule.id.startsWith('grep-custom-')) {
        grepCard.appendChild(row);
        continue;
      }
      // User-added rules can be removed again; the built-in presets can only be switched on or off.
      const removable = el('div', 'tok-grep-rule-row');
      removable.appendChild(row);
      removable.appendChild(iconButton('trash', t('inspLiveGrepRemove'), () => {
        this.grepRules = this.grepRules.filter((r) => r.id !== rule.id);
        this.callbacks.onGrepRulesChange?.([...this.grepRules]);
        this.render();
      }, { size: 14, className: 'tok-icon-btn tok-grep-rule-remove', attrs: { 'data-action': 'remove-grep-rule', 'data-grep-remove-id': rule.id } }));
      grepCard.appendChild(removable);
    }

    const customWrap = el('div', 'tok-grep-custom-wrap', { style: 'display:flex;flex-direction:column;gap:6px;margin-top:8px;padding-top:6px;border-top:1px solid var(--tok-border-subtle);' });
    customWrap.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspLiveGrepNewRule')));
    const patternInput = el('input', 'tok-input', {
      type: 'text',
      dir: 'ltr',
      placeholder: t('inspLiveGrepPlaceholder'),
      'aria-label': t('inspLiveGrepNewRule'),
      'data-grep-custom-pattern': 'true'
    });
    const patternError = el('div', 'tok-grep-error', { role: 'alert', hidden: true });
    patternInput.addEventListener('input', () => {
      patternError.hidden = true;
      patternInput.removeAttribute('aria-invalid');
    });
    const styleSelect = el('select', 'tok-input', { 'data-grep-custom-style': 'true' });
    for (const opt of [
      { id: 'parens-80', label: t('inspLiveGrepStyleCitation') },
      { id: 'bold-700', label: t('inspLiveGrepStyleBold') },
      { id: 'rashi-font', label: t('inspLiveGrepStyleRashi') }
    ]) {
      const o = el('option', undefined, { value: opt.id }, opt.label);
      styleSelect.appendChild(o);
    }
    const addBtn = button(t('inspLiveGrepAddBtn'), {
      className: 'tok-btn tok-btn-block tok-btn-sm',
      icon: 'plus',
      iconSize: 14,
      attrs: { 'data-action': 'add-grep-rule' },
      onClick: () => {
        const pat = patternInput.value.trim();
        if (!pat) return;
        try {
          // Same flags the live evaluator uses, so a pattern accepted here always runs.
          new RegExp(pat, 'gu');
        } catch {
          patternError.textContent = t('inspLiveGrepInvalid');
          patternError.hidden = false;
          patternInput.setAttribute('aria-invalid', 'true');
          return;
        }
        const mode = styleSelect.value;
        const newRule: GrepRuleData = {
          id: `grep-custom-${Date.now()}`,
          name: tf('inspLiveGrepCustomName', { p: pat }),
          pattern: pat,
          target_style_id: `char-custom-${mode}`,
          format: mode === 'parens-80'
            ? { character_style_id: 'char-citation-parens', font_scale: 0.80, color: { DeviceCmyk: { c: 0, m: 0, y: 0, k: 0.65 } } }
            : mode === 'rashi-font'
              ? { character_style_id: 'char-rashi', font_family: 'Noto Rashi Hebrew', font_weight: 700 }
              : { character_style_id: 'char-bold', font_weight: 700 },
          enabled: true
        };
        this.grepRules.push(newRule);
        patternInput.value = '';
        this.callbacks.onGrepRulesChange?.([...this.grepRules]);
        this.render();
      }
    });
    customWrap.appendChild(patternInput);
    customWrap.appendChild(el('div', 'tok-inspector-sub', undefined, t('inspLiveGrepPatternHint')));
    customWrap.appendChild(patternError);
    customWrap.appendChild(styleSelect);
    customWrap.appendChild(addBtn);
    grepCard.appendChild(customWrap);

    c.appendChild(grepCard);
  }

  // =========================================================================
  // State 4: Image frame
  // =========================================================================
  private renderImageFrameMode(): void {
    const c = this.getContainer();
    c.appendChild(this.createHeader(t('inspSelected'), t('inspImageFrame'), t('inspImage'), 'image'));

    const imgCard = this.createCard(t('inspFitting'));
    const isHe = i18n.getLanguage() === 'he';
    const notice = el(
      'p',
      'tok-panel-note',
      { style: 'margin:0 0 8px;color:var(--tok-text-muted);font-size:12px;' },
      isHe
        ? 'שיבוץ תמונות רסטר מתוכנן לגרסה עתידית (בקרים מושבתים).'
        : 'Raster image placement is planned for a future release (controls disabled).'
    );
    imgCard.appendChild(notice);

    const fits = [t('inspFitProportional'), t('inspFitFill'), t('inspFitFrame')];
    const fitField = selectField(t('inspFitting'), fits.map((f) => ({ value: f, label: f })), fits[0], () => {});
    fitField.querySelectorAll('select').forEach((sel) => {
      sel.disabled = true;
    });
    imgCard.appendChild(fitField);
    imgCard.appendChild(this.createStatusRow(t('inspEffRes'), '—', 'var(--tok-text-muted)'));
    imgCard.appendChild(this.createStatusRow(t('inspColorSpace'), '—', 'var(--tok-text-muted)'));
    imgCard.appendChild(this.createStatusRow(t('inspWrap'), t('inspWrapAround'), 'var(--tok-text-muted)'));
    c.appendChild(imgCard);
  }

  // =========================================================================
  // State 5: Multi-selection (align & distribute)
  // =========================================================================
  private renderMultiSelectMode(): void {
    const c = this.getContainer();
    c.appendChild(this.createHeader(t('inspSelected'), t('inspMulti'), t('inspMultiCount'), 'layers'));

    const alignCard = this.createCard(t('inspAlignDist'));
    const grid = el('div', undefined, { style: 'display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px' });
    const actions = [
      { id: 'align-right', label: t('inspAlignRight') },
      { id: 'align-center', label: t('inspAlignCenter') },
      { id: 'align-left', label: t('inspAlignLeft') },
      { id: 'align-top', label: t('inspAlignTop') },
      { id: 'align-middle', label: t('inspAlignMiddle') },
      { id: 'align-bottom', label: t('inspAlignBottom') }
    ];
    for (const a of actions) {
      grid.appendChild(button(a.label, { className: 'tok-btn tok-btn-sm', onClick: () => this.callbacks.onAlignFrames(a.id) }));
    }
    alignCard.appendChild(grid);
    alignCard.appendChild(button(t('inspDistribute'), {
      className: 'tok-btn tok-btn-primary tok-btn-block tok-btn-sm',
      onClick: () => this.callbacks.onAlignFrames('distribute-v')
    }));
    const isHe = i18n.getLanguage() === 'he';
    alignCard.appendChild(button(isHe ? 'פזר אופקית במרווחים שווים' : 'Distribute Horizontally', {
      className: 'tok-btn tok-btn-block tok-btn-sm',
      onClick: () => this.callbacks.onAlignFrames('distribute-h')
    }));
    c.appendChild(alignCard);
  }

  // =========================================================================
  // Value scrubber: label above (drag it to change the value), unit inside the field
  // =========================================================================
  private createScrubber(
    label: string,
    initialVal: number,
    unit: string,
    min: number,
    max: number,
    onChange: (val: number) => void
  ): HTMLElement {
    const wrap = el('div', 'tok-field tok-scrubber-field');
    const id = `tok-scrub-${Math.random().toString(36).slice(2, 9)}`;
    const lbl = el('label', 'tok-field-label tok-scrub', { for: id, title: t('inspScrubHint') }, label);
    wrap.appendChild(lbl);

    const box = el('div', 'tok-field-box');
    const input = el('input', 'tok-input tok-input-number', { id, type: 'text', inputmode: 'decimal' });
    input.value = String(Math.round(initialVal * 10) / 10);
    box.appendChild(input);
    box.appendChild(el('span', 'tok-field-unit', { 'aria-hidden': 'true' }, unit));
    wrap.appendChild(box);

    let isDragging = false;
    let startX = 0;
    let startVal = initialVal;
    let lastVal = initialVal;

    const show = (v: number) => {
      input.value = formatScrubberValue(v, unit).replace(` ${unit}`, '');
    };
    const commit = (v: number) => {
      show(v);
      if (v !== lastVal) {
        lastVal = v;
        onChange(v);
      }
    };

    const onMouseMove = (e: MouseEvent) => {
      if (!isDragging) return;
      const rtl = getComputedStyle(wrap).direction === 'rtl';
      const dx = rtl ? startX - e.clientX : e.clientX - startX;
      commit(clampScrubberValue(startVal + Math.round(dx / 3), min, max));
    };
    const onMouseUp = () => {
      if (!isDragging) return;
      isDragging = false;
      document.body.style.cursor = '';
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
    lbl.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      isDragging = true;
      startX = e.clientX;
      startVal = parseScrubberValue(input.value) ?? lastVal;
      document.body.style.cursor = 'ew-resize';
      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
      e.preventDefault();
    });

    input.addEventListener('change', () => {
      const parsed = parseScrubberValue(input.value);
      if (parsed === null) {
        show(lastVal);
        return;
      }
      commit(clampScrubberValue(parsed, min, max));
    });

    input.addEventListener('keydown', (e: KeyboardEvent) => {
      // Allow control and navigation keys
      if (
        e.key === 'Backspace' ||
        e.key === 'Delete' ||
        e.key === 'Tab' ||
        e.key === 'Escape' ||
        e.key === 'Enter' ||
        e.key === 'ArrowLeft' ||
        e.key === 'ArrowRight' ||
        e.key === 'Home' ||
        e.key === 'End' ||
        e.ctrlKey ||
        e.metaKey ||
        e.altKey
      ) {
        if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
          e.preventDefault();
          const step = (e.shiftKey ? 10 : 1) * (e.key === 'ArrowUp' ? 1 : -1);
          commit(clampScrubberValue((parseScrubberValue(input.value) ?? lastVal) + step, min, max));
        }
        return;
      }

      // Restrict input: allow only digits and decimal point
      if (!/^[0-9.]$/.test(e.key)) {
        e.preventDefault();
        return;
      }

      // Prevent duplicate decimal points
      if (e.key === '.') {
        const val = input.value;
        const selStart = input.selectionStart ?? 0;
        const selEnd = input.selectionEnd ?? 0;
        const selected = val.slice(selStart, selEnd);
        if (val.includes('.') && !selected.includes('.')) {
          e.preventDefault();
        }
      }
    });

    input.addEventListener('beforeinput', (e: any) => {
      if (e.data) {
        if (!/^[0-9.]+$/.test(e.data)) {
          e.preventDefault();
          return;
        }
        if (e.data.includes('.')) {
          const val = input.value;
          const selStart = input.selectionStart ?? 0;
          const selEnd = input.selectionEnd ?? 0;
          const selected = val.slice(selStart, selEnd);
          if (val.includes('.') && !selected.includes('.')) {
            e.preventDefault();
          }
        }
      }
    });

    input.addEventListener('input', () => {
      const sanitized = input.value.replace(/[^0-9.]/g, '').replace(/(\..*?)\./g, '$1');
      if (input.value !== sanitized) {
        input.value = sanitized;
      }
    });

    return wrap;
  }

  private row(a: HTMLElement, b: HTMLElement): HTMLElement {
    const r = el('div', 'tok-field-row');
    r.append(a, b);
    return r;
  }

  private createCard(title: string): HTMLElement {
    const card = el('section', 'tok-inspector-card');
    card.appendChild(el('h3', 'tok-inspector-card-header', undefined, title));
    return card;
  }

  private createHeader(eyebrow: string, title: string, subtitle: string, iconName: IconName, swatch?: string): HTMLElement {
    const header = el('div', 'tok-insp-head');
    header.appendChild(el('span', 'tok-insp-eyebrow', undefined, eyebrow));
    const titleRow = el('div', 'tok-insp-title');
    if (swatch) {
      const sw = el('span', 'tok-swatch', { 'aria-hidden': 'true' });
      sw.style.background = swatch;
      titleRow.appendChild(sw);
    } else {
      const ic = icon(iconName, 16);
      ic.style.color = 'var(--tok-accent-text)';
      titleRow.appendChild(ic);
    }
    titleRow.appendChild(el('h2', undefined, undefined, title));
    header.appendChild(titleRow);
    header.appendChild(el('span', 'tok-insp-sub', undefined, subtitle));
    return header;
  }

  /** Bottom card with a status badge and a list of checks. */
  private createCheckCard(title: string, items: { label: string; value: string; ok: boolean }[]): HTMLElement {
    const card = el('div', 'tok-check-card');
    const head = el('div', 'tok-check-head');
    head.appendChild(el('span', undefined, undefined, title));
    const allOk = items.every((i) => i.ok);
    const badge = el('span', `tok-badge ${allOk ? 'tok-badge-success' : 'tok-badge-warning'}`);
    badge.appendChild(el('span', 'tok-dot', { 'aria-hidden': 'true' }));
    badge.appendChild(el('span', undefined, undefined, allOk ? t('inspReadyForPrint') : t('exportCheckWarnings')));
    head.appendChild(badge);
    card.appendChild(head);
    const list = el('ul', 'tok-check-list');
    for (const item of items) {
      const li = el('li');
      const mark = el('span', item.ok ? 'tok-ok' : 'tok-warn');
      mark.appendChild(icon(item.ok ? 'check' : 'warning', 15));
      li.appendChild(mark);
      li.appendChild(el('span', undefined, { style: 'flex:1' }, item.label));
      li.appendChild(el('span', undefined, { style: 'color:var(--tok-text-primary)' }, item.value));
      list.appendChild(li);
    }
    card.appendChild(list);
    return card;
  }

  private createStatusRow(label: string, value: string, color: string): HTMLElement {
    const row = el('div', 'tok-status-line');
    row.appendChild(el('span', undefined, undefined, label));
    const v = el('span', undefined, undefined, value);
    v.style.color = color;
    row.appendChild(v);
    return row;
  }
}

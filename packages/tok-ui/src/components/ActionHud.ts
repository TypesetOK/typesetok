import { t, i18n } from '../i18n';
import { el, icon, iconButton, selectField } from '../ui';
import { DOCUMENT_FONTS, DEFAULT_DOCUMENT_FONT, getAvailableFonts } from '../fonts';

export interface ActionHudCallbacks {
  onFontChange: (fontFamily: string) => void;
  onSizeChange: (sizePt: number) => void;
  onWeightChange: (isBold: boolean, fontWeight?: number) => void;
  onAlignChange: (align: 'right' | 'center' | 'left' | 'justify') => void;
  onStyleChange: (styleToken: string) => void;
  onDismiss: () => void;
}

export class ActionHud {
  public element: HTMLElement;
  private callbacks: ActionHudCallbacks;
  private isVisible = false;
  private currentSize = 14;
  private isBold = false;
  private currentAlign: 'right' | 'center' | 'left' | 'justify' = 'right';
  private currentStyle = 'style-gemara-main';
  private currentFont = '';
  private availableStyles: Array<{ value: string; label: string }> = [
    { value: 'style-gemara-main', label: 'גמרא ראשי' },
    { value: 'style-rashi-body', label: 'רש"י' },
    { value: 'style-tosafot-body', label: 'תוספות' },
    { value: 'style-gemara-heading', label: 'כותרת פרק' }
  ];

  // Drag state
  private isDragging = false;
  private dragStartX = 0;
  private dragStartY = 0;
  private initialLeft = 0;
  private initialTop = 0;
  private userHasMoved = false;
  private customLeft: number | null = null;
  private customTop: number | null = null;

  constructor(callbacks: ActionHudCallbacks) {
    this.callbacks = callbacks;
    this.element = document.createElement('div');
    this.element.className = 'tok-action-hud';
    this.element.setAttribute('role', 'toolbar');
    this.element.setAttribute('aria-label', t('hudQuickStyle'));
    this.element.style.direction = i18n.getDirection();
    i18n.onChange(() => {
      this.element.style.direction = i18n.getDirection();
      this.element.setAttribute('aria-label', t('hudQuickStyle'));
      if (this.isVisible) this.render();
    });
    this.element.style.display = 'none';

    this.render();

    // Dismiss on Escape key
    window.addEventListener('keydown', (e) => {
      if (this.isVisible && e.key === 'Escape') {
        this.hide();
        this.callbacks.onDismiss();
      }
    });

    // Window drag event listeners
    window.addEventListener('pointermove', (e) => this.onPointerMove(e));
    window.addEventListener('pointerup', () => this.onPointerUp());
    window.addEventListener('pointercancel', () => this.onPointerUp());
  }

  public setAvailableStyles(styles: Array<{ value: string; label: string }>): void {
    if (styles && styles.length > 0) {
      this.availableStyles = styles;
      if (this.isVisible) this.render();
    }
  }

  public showAt(x: number, y: number, initialValues?: {
    font?: string;
    size?: number;
    bold?: boolean;
    fontWeight?: 'normal' | 'bold' | '600' | '700' | number;
    font_weight?: number;
    align?: 'right' | 'center' | 'left' | 'justify';
    style?: string;
    availableStyles?: Array<{ value: string; label: string }>;
  }): void {
    if (initialValues?.availableStyles && initialValues.availableStyles.length > 0) {
      this.availableStyles = initialValues.availableStyles;
    }
    if (initialValues?.size) this.currentSize = initialValues.size;
    if (initialValues?.bold !== undefined) {
      this.isBold = initialValues.bold;
    } else if (initialValues?.fontWeight !== undefined) {
      this.isBold = initialValues.fontWeight === 'bold' || initialValues.fontWeight === '700' || (typeof initialValues.fontWeight === 'number' && initialValues.fontWeight >= 700);
    } else if (initialValues?.font_weight !== undefined) {
      this.isBold = initialValues.font_weight >= 700;
    }
    if (initialValues?.align) this.currentAlign = initialValues.align;
    if (initialValues?.style) this.currentStyle = initialValues.style;
    if (initialValues?.font) this.currentFont = initialValues.font;

    this.render();

    this.element.style.display = 'flex';
    const width = this.element.offsetWidth || 380;
    const height = this.element.offsetHeight || 42;

    if (this.userHasMoved && this.customLeft !== null && this.customTop !== null) {
      // Keep at the user's preferred location so it doesn't obstruct their view
      const maxLeft = Math.max(10, window.innerWidth - width - 10);
      const maxTop = Math.max(45, window.innerHeight - height - 10);
      this.element.style.left = `${Math.min(maxLeft, Math.max(10, this.customLeft))}px`;
      this.element.style.top = `${Math.min(maxTop, Math.max(45, this.customTop))}px`;
    } else {
      // Position smartly: if selection is high up, put HUD below it; otherwise put it well above with clearance
      const maxLeft = Math.max(10, window.innerWidth - width - 10);
      const targetLeft = Math.min(maxLeft, Math.max(10, x - width / 2));
      let targetTop = y - 56;
      if (targetTop < 50) {
        // Place below the click so it never disappears off-screen or covers text
        targetTop = y + 32;
      }
      this.element.style.left = `${targetLeft}px`;
      this.element.style.top = `${targetTop}px`;
    }

    this.isVisible = true;
  }

  public hide(): void {
    this.element.style.display = 'none';
    this.isVisible = false;
  }

  public getIsOpen(): boolean {
    return this.isVisible;
  }

  private onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    this.isDragging = true;
    this.dragStartX = e.clientX;
    this.dragStartY = e.clientY;
    this.initialLeft = this.element.offsetLeft;
    this.initialTop = this.element.offsetTop;
    this.element.style.cursor = 'grabbing';
    e.preventDefault();
  }

  private onPointerMove(e: PointerEvent): void {
    if (!this.isDragging) return;
    const dx = e.clientX - this.dragStartX;
    const dy = e.clientY - this.dragStartY;
    const width = this.element.offsetWidth || 380;
    const height = this.element.offsetHeight || 42;

    const newLeft = Math.min(window.innerWidth - width - 5, Math.max(5, this.initialLeft + dx));
    const newTop = Math.min(window.innerHeight - height - 5, Math.max(40, this.initialTop + dy));

    this.element.style.left = `${newLeft}px`;
    this.element.style.top = `${newTop}px`;
    this.userHasMoved = true;
    this.customLeft = newLeft;
    this.customTop = newTop;
  }

  private onPointerUp(): void {
    if (this.isDragging) {
      this.isDragging = false;
      this.element.style.cursor = 'default';
    }
  }

  private render(): void {
    this.element.innerHTML = '';

    // 0. Draggable handle
    const dragHandle = el('div', 'tok-hud-drag-handle', {
      title: 'גרור להזזת הסרגל לכל מקום במסך',
      style: 'cursor:grab;display:flex;align-items:center;justify-content:center;padding:0 4px;color:var(--tok-text-muted);user-select:none;height:100%;'
    });
    dragHandle.appendChild(icon('layers', 14));
    dragHandle.addEventListener('pointerdown', (e) => this.onPointerDown(e));
    this.element.appendChild(dragHandle);

    this.element.appendChild(this.createDivider());

    // 1. Font family
    const fontSelect = selectField(t('hudFont'), getAvailableFonts().map((f) => ({ value: f.family, label: f.label })), this.currentFont || DEFAULT_DOCUMENT_FONT, (value) => {
      this.currentFont = value;
      this.callbacks.onFontChange(value);
    }, { title: t('hudFont') });
    fontSelect.style.minWidth = '115px';
    fontSelect.style.flexShrink = '0';
    this.element.appendChild(fontSelect);

    this.element.appendChild(this.createDivider());

    // 2. Font size stepper
    const sizeInput = el('input', 'tok-input tok-input-number', { type: 'text', 'aria-label': t('inspFontSize') });
    sizeInput.value = `${this.currentSize}pt`;
    sizeInput.style.width = '48px';
    sizeInput.style.textAlign = 'center';
    sizeInput.style.padding = '0 2px';
    const step = (delta: number) => {
      this.currentSize = Math.max(6, Math.min(150, this.currentSize + delta));
      sizeInput.value = `${this.currentSize}pt`;
      this.callbacks.onSizeChange(this.currentSize);
    };
    sizeInput.addEventListener('change', () => {
      const parsed = parseInt(sizeInput.value, 10);
      if (!isNaN(parsed) && parsed >= 6 && parsed <= 150) {
        this.currentSize = parsed;
        this.callbacks.onSizeChange(this.currentSize);
      }
      sizeInput.value = `${this.currentSize}pt`;
    });
    this.element.appendChild(iconButton('minus', t('hudSmaller'), () => step(-1), { size: 14, className: 'tok-hud-btn' }));
    this.element.appendChild(sizeInput);
    this.element.appendChild(iconButton('plus', t('hudLarger'), () => step(1), { size: 14, className: 'tok-hud-btn' }));

    this.element.appendChild(this.createDivider());

    // 3. Bold
    const boldBtn = el('button', 'tok-hud-btn', { type: 'button', title: t('hudBold'), 'aria-label': t('hudBold'), 'aria-pressed': String(this.isBold) });
    boldBtn.appendChild(icon('bold', 15));
    boldBtn.addEventListener('click', () => {
      this.isBold = !this.isBold;
      boldBtn.setAttribute('aria-pressed', String(this.isBold));
      this.callbacks.onWeightChange(this.isBold, this.isBold ? 700 : 400);
    });
    this.element.appendChild(boldBtn);

    // 4. Alignment button (Icon only - prevents text overflow!)
    const alignIcon = () => (this.currentAlign === 'justify' ? 'alignJustify' : this.currentAlign === 'center' ? 'alignCenter' : this.currentAlign === 'left' ? 'alignLeft' : 'alignRight');
    const alignLabel = () => t(this.currentAlign === 'justify' ? 'hudAlignJustify' : this.currentAlign === 'center' ? 'hudAlignCenter' : this.currentAlign === 'left' ? 'inspAlignLeft' : 'hudAlignRight');
    const alignBtn = el('button', 'tok-hud-btn', { type: 'button' });
    const paintAlign = () => {
      alignBtn.replaceChildren(icon(alignIcon(), 15));
      const lbl = alignLabel();
      alignBtn.title = `${t('hudAlignTitle')}: ${lbl}`;
      alignBtn.setAttribute('aria-label', `${t('hudAlignTitle')}: ${lbl}`);
    };
    paintAlign();
    alignBtn.addEventListener('click', () => {
      this.currentAlign = this.currentAlign === 'right' ? 'justify' : this.currentAlign === 'justify' ? 'center' : this.currentAlign === 'center' ? 'left' : 'right';
      paintAlign();
      this.callbacks.onAlignChange(this.currentAlign);
    });
    this.element.appendChild(alignBtn);

    this.element.appendChild(this.createDivider());

    // 5. Quick style (synced to active document template styles)
    const styles = this.availableStyles.length > 0
      ? this.availableStyles
      : [
          { value: 'style-gemara-main', label: 'גמרא ראשי' },
          { value: 'style-rashi-body', label: 'רש"י' },
          { value: 'style-tosafot-body', label: 'תוספות' },
          { value: 'style-gemara-heading', label: 'כותרת פרק' }
        ];
    const selectedStyle = styles.some((s) => s.value === this.currentStyle) ? this.currentStyle : styles[0].value;
    this.element.appendChild(selectField(t('hudQuickStyle'), styles, selectedStyle, (value) => {
      this.currentStyle = value;
      this.callbacks.onStyleChange(value);
    }, { title: t('hudQuickStyle') }));

    this.element.appendChild(this.createDivider());

    // 6. Close button
    const closeBtn = iconButton('close', 'סגירת סרגל צף (Esc)', () => {
      this.hide();
      this.callbacks.onDismiss();
    }, { size: 14, className: 'tok-hud-btn', attrs: { style: 'color:var(--tok-text-muted);' } });
    this.element.appendChild(closeBtn);
  }

  private createDivider(): HTMLElement {
    const div = document.createElement('div');
    div.className = 'tok-hud-divider';
    return div;
  }
}

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
  private currentStyle = 'gemara';
  private currentFont = '';

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
  }

  public showAt(x: number, y: number, initialValues?: {
    font?: string;
    size?: number;
    bold?: boolean;
    fontWeight?: 'normal' | 'bold' | '600' | '700' | number;
    font_weight?: number;
    align?: 'right' | 'center' | 'left' | 'justify';
    style?: string;
  }): void {
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

    // Position HUD centered above (x, y), kept fully inside the window
    this.element.style.display = 'flex';
    const width = this.element.offsetWidth || 360;
    const maxLeft = Math.max(10, window.innerWidth - width - 10);
    this.element.style.left = `${Math.min(maxLeft, Math.max(10, x - width / 2))}px`;
    this.element.style.top = `${Math.max(50, y - 48)}px`;
    this.isVisible = true;
  }

  public hide(): void {
    this.element.style.display = 'none';
    this.isVisible = false;
  }

  public getIsOpen(): boolean {
    return this.isVisible;
  }

  private render(): void {
    this.element.innerHTML = '';

    // 1. Font family
    const fontSelect = selectField(t('hudFont'), getAvailableFonts().map((f) => ({ value: f.family, label: f.label })), this.currentFont || DEFAULT_DOCUMENT_FONT, (value) => {
      this.currentFont = value;
      this.callbacks.onFontChange(value);
    }, { title: t('hudFont') });
    this.element.appendChild(fontSelect);

    this.element.appendChild(this.createDivider());

    // 2. Font size stepper
    const sizeInput = el('input', 'tok-input tok-input-number', { type: 'text', 'aria-label': t('inspFontSize') });
    sizeInput.value = `${this.currentSize}pt`;
    sizeInput.style.width = '52px';
    sizeInput.style.textAlign = 'center';
    sizeInput.style.padding = '0 4px';
    const step = (delta: number) => {
      this.currentSize = Math.max(6, Math.min(150, this.currentSize + delta));
      sizeInput.value = `${this.currentSize}pt`;
      this.callbacks.onSizeChange(this.currentSize);
    };
    sizeInput.addEventListener('change', () => {
      const parsed = parseInt(sizeInput.value, 10);
      // Same 6–150pt range the −/+ steppers enforce.
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

    // 4. Alignment (cycles right → justify → center)
    const alignIcon = () => (this.currentAlign === 'justify' ? 'alignJustify' : this.currentAlign === 'center' ? 'alignCenter' : 'alignRight');
    const alignLabel = () => t(this.currentAlign === 'justify' ? 'hudAlignJustify' : this.currentAlign === 'center' ? 'hudAlignCenter' : 'hudAlignRight');
    const alignBtn = el('button', 'tok-hud-btn', { type: 'button', title: t('hudAlignTitle') });
    const paintAlign = () => {
      alignBtn.replaceChildren(icon(alignIcon(), 15), el('span', undefined, undefined, alignLabel()));
      alignBtn.setAttribute('aria-label', `${t('hudAlignTitle')}: ${alignLabel()}`);
    };
    paintAlign();
    alignBtn.addEventListener('click', () => {
      this.currentAlign = this.currentAlign === 'right' ? 'justify' : this.currentAlign === 'justify' ? 'center' : 'right';
      paintAlign();
      this.callbacks.onAlignChange(this.currentAlign);
    });
    this.element.appendChild(alignBtn);

    this.element.appendChild(this.createDivider());

    // 5. Quick style
    const styles = [
      { value: 'gemara', label: 'גמרא' },
      { value: 'rashi', label: 'רש"י' },
      { value: 'tosafot', label: 'תוספות' },
      { value: 'heading', label: 'כותרת' }
    ];
    this.element.appendChild(selectField(t('hudQuickStyle'), styles, this.currentStyle, (value) => {
      this.currentStyle = value;
      this.callbacks.onStyleChange(value);
    }, { title: t('hudQuickStyle') }));
  }

  private createDivider(): HTMLElement {
    const div = document.createElement('div');
    div.className = 'tok-hud-divider';
    return div;
  }
}

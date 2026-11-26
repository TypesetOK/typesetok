import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { TemplateType } from '../engine/FlowPaginator';
import { ImportResult } from '../engine/textImporter';
import { getAvailableFonts } from '../fonts';
import {
  CustomTemplate,
  BaseTemplateType,
  getCustomTemplates,
  addCustomTemplate,
  deleteCustomTemplate,
} from '../services/customTemplatesStore';

export interface ImportTemplateModalCallbacks {
  onSelect: (templateId: TemplateType, customTemplate?: CustomTemplate) => void;
  onCancel: () => void;
}

interface TemplateOption {
  id: TemplateType;
  titleKey: string;
  descKey: string;
  badge?: string;
  badgeColor?: string;
}

const TEMPLATE_OPTIONS: TemplateOption[] = [
  {
    id: 'prose',
    titleKey: 'templateProse',
    descKey: 'templateProseDesc',
    badge: 'מומלץ',
    badgeColor: '#10B981'
  },
  {
    id: 'notes',
    titleKey: 'templateNotes',
    descKey: 'templateNotesDesc'
  },
  {
    id: 'gemara',
    titleKey: 'templateGemara',
    descKey: 'templateGemaraDesc'
  },
  {
    id: 'mikraot',
    titleKey: 'templateMikraot',
    descKey: 'templateMikraotDesc'
  },
  {
    id: 'bulletin',
    titleKey: 'templateBulletin',
    descKey: 'templateBulletinDesc'
  }
];

export class ImportTemplateModal {
  public element: HTMLElement;
  private modal: ModalController;
  private callbacks: ImportTemplateModalCallbacks;
  private currentResult: ImportResult | null = null;
  private isVisible = false;
  private isNewDocument = false;
  private isCreatingCustom = false;

  constructor(callbacks: ImportTemplateModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-import-template-overlay');
    this.modal = new ModalController(this.element, () => this.cancel(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      if (this.isVisible) this.render();
    });
  }

  public show(result: ImportResult, options?: { isNewDocument?: boolean }): void {
    this.isNewDocument = Boolean(options?.isNewDocument);
    this.isCreatingCustom = false;
    this.currentResult = result;
    this.isVisible = true;
    this.element.dir = i18n.getDirection();
    this.render();
    this.element.classList.add('tok-open');
    this.modal.opened();
  }

  public hide(): void {
    if (!this.isVisible) return;
    this.isVisible = false;
    this.element.classList.remove('tok-open');
    this.modal.closed();
  }

  private cancel(): void {
    this.hide();
    this.callbacks.onCancel();
  }

  private render(): void {
    this.element.innerHTML = '';
    const res = this.currentResult;
    if (!res) return;

    const dialog = el('div', 'tok-dialog tok-modal-card', {
      role: 'dialog',
      'aria-labelledby': 'tok-import-tmpl-h',
      style: 'max-width:580px;width:90%;margin:auto;'
    });

    // Header
    const head = el('div', 'tok-modal-header', { style: 'display:flex;align-items:center;justify-content:space-between;padding:18px 24px;border-bottom:1px solid var(--tok-border-subtle);' });
    const titleWrap = el('div');
    const title = el('h2', undefined, { id: 'tok-import-tmpl-h', style: 'margin:0 0 4px 0;font-size:18px;font-weight:700;' }, this.isNewDocument ? t('newDocChooseTemplateTitle') : tf('importChooseTemplateTitle', { title: res.title }));
    const sub = el('p', undefined, { style: 'margin:0;font-size:12px;color:var(--tok-text-muted);' }, this.isNewDocument ? t('newDocChooseTemplateSub') : tf('importChooseTemplateSub', { count: res.paragraphCount }));
    titleWrap.appendChild(title);
    titleWrap.appendChild(sub);
    head.appendChild(titleWrap);

    const closeBtn = iconButton('close', t('paletteHintClose'), () => this.cancel(), { size: 16 });
    head.appendChild(closeBtn);
    dialog.appendChild(head);

    // Body: template grid or custom template creator
    const body = el('div', 'tok-modal-body', { style: 'padding:16px 22px;display:flex;flex-direction:column;gap:8px;max-height:56vh;overflow-y:auto;' });
    const isEn = i18n.getLanguage() === 'en';

    if (this.isCreatingCustom) {
      this.renderCustomCreatorForm(body, isEn);
      dialog.appendChild(body);
      const footer = el('div', 'tok-modal-footer', { style: 'display:flex;justify-content:flex-end;gap:10px;padding:14px 24px;border-top:1px solid var(--tok-border-subtle);' });
      footer.appendChild(button(isEn ? 'Back' : 'חזרה', {
        className: 'tok-btn tok-btn-sm',
        onClick: () => {
          this.isCreatingCustom = false;
          this.render();
        }
      }));
      dialog.appendChild(footer);
      this.element.appendChild(dialog);
      return;
    }

    for (const opt of TEMPLATE_OPTIONS) {
      const card = el('button', 'tok-template-option-btn', {
        type: 'button',
        style: 'display:flex;align-items:center;justify-content:space-between;gap:12px;padding:12px 16px;border-radius:10px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-2, #1E293B);color:var(--tok-text-primary);cursor:pointer;text-align:start;transition:all 0.15s ease;flex-shrink:0;'
      });

      const info = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:3px;flex:1;min-width:0;' });
      const optTitleRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:8px;' });
      optTitleRow.appendChild(el('strong', undefined, { style: 'font-size:14px;font-weight:700;' }, t(opt.titleKey)));
      if (opt.badge) {
        optTitleRow.appendChild(el('span', undefined, {
          style: `font-size:11px;font-weight:600;padding:2px 8px;border-radius:12px;background:${opt.badgeColor}22;color:${opt.badgeColor};`
        }, isEn && opt.badge === 'מומלץ' ? 'Recommended' : opt.badge));
      }
      info.appendChild(optTitleRow);
      info.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);line-height:1.4;' }, t(opt.descKey)));
      card.appendChild(info);

      const arrow = el('span', 'tok-flip-rtl', { style: 'opacity:0.6;font-size:16px;flex:none;' });
      arrow.appendChild(icon('arrowForward', 16));
      card.appendChild(arrow);

      card.addEventListener('mouseenter', () => {
        card.style.borderColor = 'var(--tok-accent-primary, #3B82F6)';
        card.style.background = 'var(--tok-accent-soft, rgba(59, 130, 246, 0.1))';
      });
      card.addEventListener('mouseleave', () => {
        card.style.borderColor = 'var(--tok-border-strong)';
        card.style.background = 'var(--tok-bg-surface-2, #1E293B)';
      });

      card.addEventListener('click', () => {
        this.hide();
        try {
          this.callbacks.onSelect(opt.id);
        } catch (err) {
          console.error('[TOK] Error applying template:', err);
        }
      });

      body.appendChild(card);
    }

    // Custom templates
    const customTemplates = getCustomTemplates();
    if (customTemplates.length > 0) {
      body.appendChild(el('div', undefined, {
        style: 'font-size:11px;font-weight:700;color:var(--tok-text-muted);margin:8px 2px 2px;'
      }, isEn ? 'Custom Templates' : 'תבניות מותאמות אישית'));

      for (const ct of customTemplates) {
        const card = el('div', 'tok-template-option-btn', {
          role: 'button',
          tabindex: '0',
          style: 'display:flex;align-items:center;justify-content:space-between;gap:12px;padding:12px 16px;border-radius:10px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-2, #1E293B);color:var(--tok-text-primary);cursor:pointer;text-align:start;transition:all 0.15s ease;flex-shrink:0;'
        });

        const info = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:3px;flex:1;min-width:0;' });
        const optTitleRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:8px;' });
        optTitleRow.appendChild(el('strong', undefined, { style: 'font-size:14px;font-weight:700;' }, ct.name));
        optTitleRow.appendChild(el('span', undefined, {
          style: 'font-size:11px;font-weight:600;padding:2px 8px;border-radius:12px;background:rgba(139,92,246,0.16);color:#8B5CF6;'
        }, isEn ? 'Custom' : 'מותאמת אישית'));
        info.appendChild(optTitleRow);
        const descText = ct.description || `${ct.pageSize} · ${ct.columns} ${isEn ? 'col' : 'טורים'} · ${ct.fontFamily} ${ct.fontSizePt}pt`;
        info.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);line-height:1.4;' }, descText));
        card.appendChild(info);

        const actions = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px;flex:none;' });
        const delBtn = iconButton('close', isEn ? 'Delete template' : 'מחק תבנית', (e?: Event) => {
          e?.stopPropagation();
          deleteCustomTemplate(ct.id);
          this.render();
        }, { size: 13, attrs: { style: 'width:24px;height:24px;padding:0;opacity:0.7;color:var(--tok-status-error);' } });
        delBtn.addEventListener('click', (e) => e.stopPropagation());
        actions.appendChild(delBtn);

        const arrow = el('span', 'tok-flip-rtl', { style: 'opacity:0.6;font-size:16px;' });
        arrow.appendChild(icon('arrowForward', 16));
        actions.appendChild(arrow);
        card.appendChild(actions);

        card.addEventListener('mouseenter', () => {
          card.style.borderColor = 'var(--tok-accent-primary, #3B82F6)';
          card.style.background = 'var(--tok-accent-soft, rgba(59, 130, 246, 0.1))';
        });
        card.addEventListener('mouseleave', () => {
          card.style.borderColor = 'var(--tok-border-strong)';
          card.style.background = 'var(--tok-bg-surface-2, #1E293B)';
        });

        const selectCustom = () => {
          this.hide();
          try {
            this.callbacks.onSelect(ct.baseTemplate, ct);
          } catch (err) {
            console.error('[TOK] Error applying custom template:', err);
          }
        };
        card.addEventListener('click', selectCustom);
        card.addEventListener('keydown', (e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            selectCustom();
          }
        });
        body.appendChild(card);
      }
    }

    // Button to create a new custom template
    const addCustomBtn = el('button', 'tok-btn tok-btn-ghost', {
      type: 'button',
      style: 'margin-top:6px;display:flex;align-items:center;justify-content:center;gap:8px;padding:10px 14px;border:1px dashed var(--tok-border-strong);border-radius:10px;width:100%;font-weight:600;'
    });
    addCustomBtn.appendChild(icon('plus', 15));
    addCustomBtn.appendChild(el('span', undefined, undefined, isEn ? 'Create Custom Template...' : 'צור והוסף תבנית מותאמת אישית...'));
    addCustomBtn.addEventListener('click', () => {
      this.isCreatingCustom = true;
      this.render();
    });
    body.appendChild(addCustomBtn);

    dialog.appendChild(body);

    // Footer with cancel
    const footer = el('div', 'tok-modal-footer', { style: 'display:flex;justify-content:flex-end;gap:10px;padding:14px 24px;border-top:1px solid var(--tok-border-subtle);' });
    const cancelBtn = button(t('paletteHintClose'), {
      className: 'tok-btn tok-btn-sm',
      onClick: () => this.cancel()
    });
    footer.appendChild(cancelBtn);
    dialog.appendChild(footer);

    this.element.appendChild(dialog);
  }

  private renderCustomCreatorForm(container: HTMLElement, isEn: boolean): void {
    const fieldStyle = 'display:flex;flex-direction:column;gap:4px;font-size:12px;';
    const inputStyle = 'padding:8px 10px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-2, #1E293B);color:var(--tok-text-primary);font-family:inherit;font-size:13px;';

    const nameWrap = el('label', undefined, { style: fieldStyle });
    nameWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Template Name *' : 'שם התבנית *'));
    const nameInput = el('input', undefined, {
      type: 'text',
      placeholder: isEn ? 'e.g. My Commentary Book' : 'לדוגמה: ספר שו״ת דו-טורי',
      style: inputStyle
    }) as HTMLInputElement;
    nameWrap.appendChild(nameInput);
    container.appendChild(nameWrap);

    const descWrap = el('label', undefined, { style: fieldStyle });
    descWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Description (optional)' : 'תיאור קצר (אופציונלי)'));
    const descInput = el('input', undefined, {
      type: 'text',
      placeholder: isEn ? 'Short description of layout' : 'תיאור קצר של הפריסה',
      style: inputStyle
    }) as HTMLInputElement;
    descWrap.appendChild(descInput);
    container.appendChild(descWrap);

    const row1 = el('div', undefined, { style: 'display:grid;grid-template-columns:1fr 1fr;gap:10px;' });

    const baseWrap = el('label', undefined, { style: fieldStyle });
    baseWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Base Layout' : 'מבנה בסיס'));
    const baseSelect = el('select', undefined, { style: inputStyle }) as HTMLSelectElement;
    for (const opt of TEMPLATE_OPTIONS) {
      const o = el('option', undefined, { value: opt.id }, t(opt.titleKey)) as HTMLOptionElement;
      baseSelect.appendChild(o);
    }
    baseWrap.appendChild(baseSelect);
    row1.appendChild(baseWrap);

    const pageWrap = el('label', undefined, { style: fieldStyle });
    pageWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Page Size' : 'גודל עמוד'));
    const pageSelect = el('select', undefined, { style: inputStyle }) as HTMLSelectElement;
    for (const sz of ['17x24', 'A4', 'B5', 'Crown'] as const) {
      pageSelect.appendChild(el('option', undefined, { value: sz }, sz));
    }
    pageWrap.appendChild(pageSelect);
    row1.appendChild(pageWrap);
    container.appendChild(row1);

    const row2 = el('div', undefined, { style: 'display:grid;grid-template-columns:1fr 1fr 1fr;gap:10px;' });

    const colWrap = el('label', undefined, { style: fieldStyle });
    colWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Columns' : 'מספר טורים'));
    const colInput = el('input', undefined, { type: 'number', min: '1', max: '4', value: '1', style: inputStyle }) as HTMLInputElement;
    colWrap.appendChild(colInput);
    row2.appendChild(colWrap);

    const fontWrap = el('label', undefined, { style: fieldStyle });
    fontWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Font' : 'גופן ראשי'));
    const fontSelect = el('select', undefined, { style: inputStyle }) as HTMLSelectElement;
    for (const f of getAvailableFonts()) {
      fontSelect.appendChild(el('option', undefined, { value: f.family }, f.label));
    }
    fontWrap.appendChild(fontSelect);
    row2.appendChild(fontWrap);

    const sizeWrap = el('label', undefined, { style: fieldStyle });
    sizeWrap.appendChild(el('span', undefined, { style: 'font-weight:600;' }, isEn ? 'Font Size (pt)' : 'גודל גופן (pt)'));
    const sizeInput = el('input', undefined, { type: 'number', min: '8', max: '28', value: '12', style: inputStyle }) as HTMLInputElement;
    sizeWrap.appendChild(sizeInput);
    row2.appendChild(sizeWrap);
    container.appendChild(row2);

    const errorEl = el('div', undefined, { style: 'color:var(--tok-status-error);font-size:12px;display:none;' });
    container.appendChild(errorEl);

    const saveBtn = button(isEn ? 'Save & Use Template' : 'שמור והחל תבנית', {
      className: 'tok-btn tok-btn-primary',
      attrs: { style: 'margin-top:8px;align-self:flex-start;' },
      onClick: () => {
        const name = nameInput.value.trim();
        if (!name) {
          errorEl.textContent = isEn ? 'Please enter a template name.' : 'יש להזין שם לתבנית.';
          errorEl.style.display = 'block';
          nameInput.focus();
          return;
        }
        const fontSizePt = Math.max(8, Math.min(28, Number(sizeInput.value) || 12));
        const columns = Math.max(1, Math.min(4, Math.round(Number(colInput.value) || 1)));
        const created = addCustomTemplate({
          name,
          description: descInput.value.trim() || undefined,
          baseTemplate: (baseSelect.value as BaseTemplateType) || 'prose',
          pageSize: (pageSelect.value as CustomTemplate['pageSize']) || '17x24',
          columns,
          fontFamily: fontSelect.value || 'Frank Ruhl Libre',
          fontSizePt,
          lineHeightPt: Math.round(fontSizePt * 1.4),
        });
        this.isCreatingCustom = false;
        this.hide();
        this.callbacks.onSelect(created.baseTemplate, created);
      }
    });
    container.appendChild(saveBtn);
  }
}

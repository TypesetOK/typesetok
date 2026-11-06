import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { TemplateType } from '../engine/FlowPaginator';
import { ImportResult } from '../engine/textImporter';

export interface ImportTemplateModalCallbacks {
  onSelect: (templateId: TemplateType) => void;
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

    // Body: template grid
    const body = el('div', 'tok-modal-body', { style: 'padding:16px 22px;display:flex;flex-direction:column;gap:8px;max-height:56vh;overflow-y:auto;' });

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
        }, opt.badge));
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
}

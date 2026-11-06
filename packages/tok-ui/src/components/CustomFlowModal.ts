import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { IconName } from '../icons';
import { ModalController } from './ModalController';

export type FlowPlacementType = 'split-edges' | 'bottom' | 'margin-outer' | 'free';

export interface CustomFlowData {
  name: string;
  role: string;
  color: string;
  placement: FlowPlacementType;
}

export interface CustomFlowModalCallbacks {
  onSubmit: (flow: CustomFlowData) => void;
  onCancel: () => void;
}

const PRESET_COLORS = [
  { hex: '#7C3AED', name: 'סגול' },
  { hex: '#059669', name: 'ירוק אזמרגד' },
  { hex: '#D97706', name: 'ענבר' },
  { hex: '#E11D48', name: 'שני' },
  { hex: '#2563EB', name: 'כחול מלכותי' },
  { hex: '#0891B2', name: 'טורקיז' }
];

export class CustomFlowModal {
  public element: HTMLElement;
  private modal: ModalController;
  private callbacks: CustomFlowModalCallbacks;
  private isVisible = false;

  private selectedColor = PRESET_COLORS[0].hex;
  private selectedPlacement: FlowPlacementType = 'split-edges';

  constructor(callbacks: CustomFlowModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-custom-flow-overlay');
    this.modal = new ModalController(this.element, () => this.cancel(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      if (this.isVisible) this.render();
    });
  }

  public show(): void {
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

    const dialog = el('div', 'tok-dialog tok-modal-card', {
      role: 'dialog',
      'aria-labelledby': 'tok-custom-flow-h',
      style: 'max-width:560px;width:92%;margin:auto;'
    });

    // Header
    const head = el('div', 'tok-modal-header', {
      style: 'display:flex;align-items:center;justify-content:space-between;padding:16px 22px;border-bottom:1px solid var(--tok-border-subtle);'
    });
    const titleWrap = el('div');
    titleWrap.appendChild(el('h2', undefined, { id: 'tok-custom-flow-h', style: 'margin:0 0 4px 0;font-size:17px;font-weight:700;' }, t('customFlowModalTitle')));
    titleWrap.appendChild(el('p', undefined, { style: 'margin:0;font-size:12px;color:var(--tok-text-muted);' }, t('customFlowModalSub')));
    head.appendChild(titleWrap);

    const closeBtn = iconButton('close', t('paletteHintClose'), () => this.cancel(), { size: 16 });
    head.appendChild(closeBtn);
    dialog.appendChild(head);

    // Body
    const body = el('div', 'tok-modal-body', {
      style: 'padding:18px 22px;display:flex;flex-direction:column;gap:14px;max-height:60vh;overflow-y:auto;'
    });

    // 1. Flow Name
    const nameGroup = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:5px;' });
    nameGroup.appendChild(el('label', undefined, { style: 'font-size:12px;font-weight:600;' }, t('customFlowNameLabel')));
    const nameInput = el('input', 'tok-input', {
      type: 'text',
      value: 'הערות שוליים נוספות',
      style: 'padding:8px 10px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-2);color:var(--tok-text-primary);font-size:13px;'
    }) as HTMLInputElement;
    nameGroup.appendChild(nameInput);
    body.appendChild(nameGroup);

    // 2. Role / Description
    const roleGroup = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:5px;' });
    roleGroup.appendChild(el('label', undefined, { style: 'font-size:12px;font-weight:600;' }, t('customFlowRoleLabel')));
    const roleInput = el('input', 'tok-input', {
      type: 'text',
      value: 'הערות תחתית ושוליים',
      style: 'padding:8px 10px;border-radius:6px;border:1px solid var(--tok-border-strong);background:var(--tok-bg-surface-2);color:var(--tok-text-primary);font-size:13px;'
    }) as HTMLInputElement;
    roleGroup.appendChild(roleInput);
    body.appendChild(roleGroup);

    // 3. Color Swatches
    const colorGroup = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:6px;' });
    colorGroup.appendChild(el('label', undefined, { style: 'font-size:12px;font-weight:600;' }, t('customFlowColorLabel')));
    const swatchesWrap = el('div', undefined, { style: 'display:flex;gap:8px;align-items:center;' });
    for (const c of PRESET_COLORS) {
      const sw = el('button', undefined, {
        type: 'button',
        title: c.name,
        style: `width:26px;height:26px;border-radius:50%;background:${c.hex};cursor:pointer;border:2px solid ${this.selectedColor === c.hex ? 'var(--tok-text-primary)' : 'transparent'};transition:transform 0.15s;`
      });
      sw.addEventListener('click', () => {
        this.selectedColor = c.hex;
        this.render();
      });
      swatchesWrap.appendChild(sw);
    }
    colorGroup.appendChild(swatchesWrap);
    body.appendChild(colorGroup);

    // 4. Layout Placement Mode (Including Split Edges)
    const placeGroup = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:8px;' });
    placeGroup.appendChild(el('label', undefined, { style: 'font-size:12px;font-weight:600;' }, t('customFlowPlacementLabel')));

    const placementOptions: { id: FlowPlacementType; labelKey: string; iconName: IconName }[] = [
      { id: 'split-edges', labelKey: 'customFlowPlaceSplitEdges', iconName: 'templateColumns' },
      { id: 'bottom', labelKey: 'customFlowPlaceBottom', iconName: 'layers' },
      { id: 'margin-outer', labelKey: 'customFlowPlaceMargin', iconName: 'flows' },
      { id: 'free', labelKey: 'customFlowPlaceFree', iconName: 'settings' }
    ];

    for (const opt of placementOptions) {
      const isSel = this.selectedPlacement === opt.id;
      const optBtn = el('button', 'tok-placement-opt-btn', {
        type: 'button',
        style: `display:flex;align-items:center;gap:10px;padding:10px 14px;border-radius:8px;border:1.5px solid ${isSel ? 'var(--tok-accent-primary)' : 'var(--tok-border-strong)'};background:${isSel ? 'var(--tok-accent-soft, rgba(59,130,246,0.1))' : 'var(--tok-bg-surface-2)'};color:var(--tok-text-primary);cursor:pointer;text-align:start;transition:all 0.15s;`
      });
      const iconWrap = el('span', undefined, { style: 'display:inline-flex;align-items:center;opacity:0.85;' });
      iconWrap.appendChild(icon(opt.iconName, 18));
      optBtn.appendChild(iconWrap);
      optBtn.appendChild(el('span', undefined, { style: 'font-size:12.5px;font-weight:500;line-height:1.35;flex:1;' }, t(opt.labelKey)));

      optBtn.addEventListener('click', () => {
        this.selectedPlacement = opt.id;
        this.render();
      });
      placeGroup.appendChild(optBtn);
    }
    body.appendChild(placeGroup);

    dialog.appendChild(body);

    // Footer
    const footer = el('div', 'tok-modal-footer', {
      style: 'display:flex;justify-content:flex-end;gap:10px;padding:14px 22px;border-top:1px solid var(--tok-border-subtle);'
    });
    const cancelBtn = button(t('paletteHintClose'), {
      className: 'tok-btn tok-btn-sm',
      onClick: () => this.cancel()
    });
    const submitBtn = button(t('customFlowSubmit'), {
      className: 'tok-btn tok-btn-primary tok-btn-sm',
      onClick: () => {
        const name = nameInput.value.trim() || 'תזרים מותאם';
        const role = roleInput.value.trim() || 'פירוש נוסף';
        this.hide();
        this.callbacks.onSubmit({
          name,
          role,
          color: this.selectedColor,
          placement: this.selectedPlacement
        });
      }
    });
    footer.appendChild(cancelBtn);
    footer.appendChild(submitBtn);
    dialog.appendChild(footer);

    this.element.appendChild(dialog);
  }
}

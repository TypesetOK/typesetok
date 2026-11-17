import { t, tf, i18n } from '../i18n';
import { el, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { getAvailableFonts, registerFontFamily } from '../fonts';

export interface MissingFontsModalCallbacks {
  onApplySubstitutions: (mapping: Record<string, string>) => void;
  onCancel?: () => void;
}

export class MissingFontsModal {
  public element: HTMLElement;
  private modal: ModalController;
  private callbacks: MissingFontsModalCallbacks;
  private isVisible = false;
  private missingFamilies: string[] = [];
  private substitutions: Map<string, string> = new Map();
  private statusByFamily: Map<string, { message: string; isError: boolean }> = new Map();

  constructor(callbacks: MissingFontsModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay', { 'data-modal': 'missing-fonts' });
    this.modal = new ModalController(this.element, () => this.cancel(), { closeOnBackdrop: false });

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      if (this.isVisible) this.render();
    });
  }

  public show(missingFamilies: string[]): void {
    this.missingFamilies = [...new Set(missingFamilies.map((f) => f.trim()).filter(Boolean))];
    this.substitutions.clear();
    this.statusByFamily.clear();
    const avail = getAvailableFonts();
    const fallbackFamily = avail[0]?.family || 'Frank Ruhl Libre';
    for (const fam of this.missingFamilies) {
      this.substitutions.set(fam, fallbackFamily);
    }
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
    this.callbacks.onCancel?.();
  }

  private async installTtfForFamily(missingFamily: string): Promise<void> {
    const win = typeof window !== 'undefined' ? (window as any) : {};
    if (win.tokIpc?.showOpenDialog && win.tokIpc?.installFont) {
      const filePath: string | null = await win.tokIpc.showOpenDialog({
        filters: [{ name: 'TrueType Fonts (.ttf)', extensions: ['ttf'] }]
      });
      if (!filePath) return;
      const res = await win.tokIpc.installFont(filePath);
      if (!res?.success || !res?.report) {
        const msg = res?.message || res?.error || 'Invalid font';
        this.statusByFamily.set(missingFamily, {
          message: tf('missingFontsRejected', { error: msg }),
          isError: true
        });
        this.render();
        return;
      }
      const installedName: string = res.report.family_name || missingFamily;
      if (res.base64Data) {
        this.injectBrowserFontFace(installedName, res.base64Data);
      }
      registerFontFamily(installedName);
      this.substitutions.set(missingFamily, installedName);
      this.statusByFamily.set(missingFamily, {
        message: tf('missingFontsInstalledOk', { family: installedName }),
        isError: false
      });
      this.render();
      return;
    }

    // Browser fallback without Electron IPC: file picker into @font-face
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.ttf';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const bytes = new Uint8Array(await file.arrayBuffer());
      let bin = '';
      for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
      const fam = file.name.replace(/\.ttf$/i, '') || missingFamily;
      this.injectBrowserFontFace(fam, btoa(bin));
      registerFontFamily(fam);
      this.substitutions.set(missingFamily, fam);
      this.statusByFamily.set(missingFamily, {
        message: tf('missingFontsInstalledOk', { family: fam }),
        isError: false
      });
      this.render();
    };
    input.click();
  }

  private injectBrowserFontFace(family: string, base64Ttf: string): void {
    try {
      const styleId = `tok-custom-font-${family.replace(/[^a-z0-9_-]/gi, '_')}`;
      if (document.getElementById(styleId)) return;
      const style = document.createElement('style');
      style.id = styleId;
      style.textContent = `@font-face { font-family: '${family.replace(/'/g, "\\'")}'; src: url(data:font/ttf;base64,${base64Ttf}) format('truetype'); font-display: swap; }`;
      document.head.appendChild(style);
    } catch {
      // Ignore in non-DOM tests
    }
  }

  public getSubstitutions(): Record<string, string> {
    const mapping: Record<string, string> = {};
    for (const [k, v] of this.substitutions.entries()) {
      mapping[k] = v;
    }
    return mapping;
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.element.innerHTML = '';

    const dialog = el('div', 'tok-dialog', { style: 'max-width:540px;width:90%;margin:auto;' });

    // Header
    const head = el('div', 'tok-dialog-head');
    head.appendChild(el('h2', undefined, { 'data-modal-title': 'true' }, t('missingFontsTitle')));
    head.appendChild(iconButton('close', t('missingFontsSkip'), () => this.cancel()));
    dialog.appendChild(head);

    // Body
    const body = el('div', 'tok-dialog-body');
    const wrap = el('div', undefined, {
      dir: i18n.getDirection(),
      style: 'display:flex;flex-direction:column;gap:12px;'
    });
    wrap.appendChild(el('p', undefined, { style: 'margin:0;font-size:13px;color:var(--tok-text-secondary);line-height:1.5;' }, t('missingFontsDesc')));

    const avail = getAvailableFonts();
    for (const fam of this.missingFamilies) {
      const card = el('div', undefined, {
        'data-missing-font': fam,
        style: 'display:flex;flex-direction:column;gap:8px;padding:10px 12px;border:1px solid var(--tok-border-subtle);border-radius:8px;background:var(--tok-bg-surface-2);'
      });

      const top = el('div', undefined, { style: 'display:flex;justify-content:space-between;align-items:center;gap:8px;' });
      top.appendChild(el('strong', undefined, { style: 'font-size:13px;' }, fam));
      top.appendChild(button(t('missingFontsInstallTtf'), {
        className: 'tok-btn tok-btn-sm',
        attrs: { 'data-install-font-for': fam },
        onClick: () => {
          void this.installTtfForFamily(fam);
        }
      }));
      card.appendChild(top);

      const selRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:8px;' });
      selRow.appendChild(el('label', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);white-space:nowrap;' }, t('missingFontsReplaceWith')));
      const select = document.createElement('select');
      select.className = 'tok-input';
      select.setAttribute('data-replace-font-for', fam);
      const selectedTarget = this.substitutions.get(fam) || avail[0]?.family || 'Frank Ruhl Libre';
      for (const f of avail) {
        const opt = document.createElement('option');
        opt.value = f.family;
        opt.textContent = f.label || f.family;
        if (f.family === selectedTarget) opt.selected = true;
        select.appendChild(opt);
      }
      select.addEventListener('change', () => {
        this.substitutions.set(fam, select.value);
      });
      selRow.appendChild(select);
      card.appendChild(selRow);

      const status = this.statusByFamily.get(fam);
      if (status) {
        card.appendChild(el('div', undefined, {
          style: `font-size:12px;color:${status.isError ? 'var(--tok-status-error)' : 'var(--tok-status-success)'};`
        }, status.message));
      }

      wrap.appendChild(card);
    }

    body.appendChild(wrap);
    dialog.appendChild(body);

    // Footer
    const foot = el('div', 'tok-dialog-foot');
    foot.appendChild(button(t('missingFontsSkip'), {
      className: 'tok-btn tok-btn-ghost',
      attrs: { 'data-action': 'missing-fonts-skip' },
      onClick: () => this.cancel()
    }));
    foot.appendChild(button(t('missingFontsApply'), {
      className: 'tok-btn tok-btn-primary',
      attrs: { 'data-action': 'missing-fonts-apply', 'data-autofocus': 'true' },
      onClick: () => {
        const mapping = this.getSubstitutions();
        this.hide();
        this.callbacks.onApplySubstitutions(mapping);
      }
    }));
    dialog.appendChild(foot);

    this.element.appendChild(dialog);
    this.modal.afterRender(focusKey);
  }
}

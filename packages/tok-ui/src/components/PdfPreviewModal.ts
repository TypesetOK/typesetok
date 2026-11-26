import { t, i18n } from '../i18n';
import { el, iconButton, button, kbd } from '../ui';
import { ModalController } from './ModalController';
import { getHostBridge } from '../platform/hostBridge.js';

export interface PdfPreviewModalCallbacks {
  /** Back to the export settings (also what Esc / the close button do). */
  onBack: () => void;
  /** Export the file with the settings that were previewed. */
  onExport: () => void;
}

function tauriPreviewAssetUrl(): string {
  const isWin = typeof navigator !== 'undefined' && /win/i.test(navigator.platform || navigator.userAgent || '');
  const base = isWin ? 'http://tok-asset.localhost/preview/latest.pdf' : 'tok-asset://localhost/preview/latest.pdf';
  return `${base}?t=${Date.now()}`;
}

function base64ToBlobUrl(base64: string): string {
  const bin = atob(base64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return URL.createObjectURL(new Blob([bytes], { type: 'application/pdf' }));
}

type PreviewState = { kind: 'loading' } | { kind: 'ready'; url: string; revokeOnClose: boolean } | { kind: 'error'; message: string };

/**
 * Shows the real PDF the engine would export (same bytes as the exported file),
 * rendered by the system's PDF viewer inside the window, before anything is saved.
 */
export class PdfPreviewModal {
  public element: HTMLElement;
  private callbacks: PdfPreviewModalCallbacks;
  private modal: ModalController;
  private isVisible = false;
  private state: PreviewState = { kind: 'loading' };
  private title = '';

  constructor(callbacks: PdfPreviewModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-pdf-preview-overlay');
    this.modal = new ModalController(this.element, () => this.back(), { closeOnBackdrop: false });
    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  /** Opens the dialog in its "rendering…" state. */
  public showLoading(title: string): void {
    this.releaseUrl();
    this.title = title;
    this.state = { kind: 'loading' };
    this.isVisible = true;
    this.element.classList.add('tok-open');
    this.render();
    this.modal.opened();
  }

  public setPdf(base64: string): void {
    if (!this.isVisible) return;
    this.releaseUrl();
    try {
      const isTauri = getHostBridge().kind === 'tauri';
      this.state = isTauri
        ? { kind: 'ready', url: tauriPreviewAssetUrl(), revokeOnClose: false }
        : { kind: 'ready', url: base64ToBlobUrl(base64), revokeOnClose: true };
    } catch (err: any) {
      this.state = { kind: 'error', message: err?.message ?? String(err) };
    }
    this.render();
  }

  public setError(message: string): void {
    if (!this.isVisible) return;
    this.releaseUrl();
    this.state = { kind: 'error', message };
    this.render();
  }

  public isOpen(): boolean {
    return this.isVisible;
  }

  /** Closes without notifying (used when a newer preview request replaces this one). */
  public hide(): void {
    if (!this.isVisible) return;
    this.isVisible = false;
    this.releaseUrl();
    this.element.classList.remove('tok-open');
    this.element.innerHTML = '';
    this.modal.closed();
  }

  private back(): void {
    this.hide();
    this.callbacks.onBack();
  }

  private releaseUrl(): void {
    if (this.state.kind === 'ready' && this.state.revokeOnClose) URL.revokeObjectURL(this.state.url);
    this.state = { kind: 'loading' };
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.renderContent();
    this.modal.afterRender(focusKey);
  }

  private renderContent(): void {
    this.element.innerHTML = '';
    this.element.dir = i18n.getDirection();

    const card = el('div', 'tok-dialog', {
      style: 'width:min(1000px,96vw);height:min(860px,94vh);display:flex;flex-direction:column;'
    });

    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, t('pdfPreviewTitle')));
    headText.appendChild(el('p', undefined, undefined, this.title));
    head.appendChild(headText);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', t('pdfPreviewBack'), () => this.back(), { attrs: { 'data-focus-key': 'close' } }));
    card.appendChild(head);

    const body = el('div', 'tok-dialog-body', {
      style: 'flex:1;min-height:0;padding:0;display:flex;align-items:stretch;justify-content:center;background:var(--tok-bg-surface-1);'
    });
    if (this.state.kind === 'ready') {
      body.appendChild(
        el('iframe', undefined, {
          src: `${this.state.url}#toolbar=1&navpanes=0&view=FitH`,
          title: t('pdfPreviewTitle'),
          style: 'flex:1;border:0;width:100%;height:100%;background:#fff;'
        })
      );
    } else {
      const msg = this.state.kind === 'loading' ? t('pdfPreviewRendering') : `${t('pdfPreviewError')} ${this.state.message}`;
      body.appendChild(
        el('div', undefined, {
          role: this.state.kind === 'error' ? 'alert' : 'status',
          style: 'margin:auto;padding:24px;text-align:center;color:var(--tok-text-secondary);max-width:520px;'
        }, msg)
      );
    }
    card.appendChild(body);

    const foot = el('div', 'tok-dialog-foot');
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(t('pdfPreviewBack'), { attrs: { 'data-focus-key': 'back' }, onClick: () => this.back() }));
    const exportBtn = button(t('exportGo'), {
      className: 'tok-btn tok-btn-primary',
      icon: 'export',
      attrs: { 'data-focus-key': 'export' },
      onClick: () => {
        this.hide();
        this.callbacks.onExport();
      }
    });
    if (this.state.kind !== 'ready') exportBtn.disabled = true;
    foot.appendChild(exportBtn);
    card.appendChild(foot);

    this.element.appendChild(card);
  }
}

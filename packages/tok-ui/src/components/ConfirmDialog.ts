import { el, button } from '../ui';
import { i18n } from '../i18n';

/**
 * In-app confirmation dialog (replaces the native window.confirm box, which ignores the app theme).
 * One shared overlay; resolves true on confirm and false on cancel, Escape or a backdrop click.
 * Keyboard handling runs in the capture phase so a modal underneath (e.g. Welcome) never sees the keys.
 */
export interface ConfirmDialogOptions {
  title: string;
  message: string;
  detail?: string;
  confirmLabel: string;
  cancelLabel: string;
  /** Styles the confirm button as destructive. */
  danger?: boolean;
}

let overlay: HTMLElement | null = null;
let settle: ((value: boolean) => void) | null = null;
let previousFocus: HTMLElement | null = null;
let cancelButton: HTMLButtonElement | null = null;
let confirmButton: HTMLButtonElement | null = null;
let titleSeq = 0;

function onKeyDown(e: KeyboardEvent): void {
  if (!overlay || !cancelButton || !confirmButton) return;
  if (e.key === 'Escape') {
    e.preventDefault();
    e.stopPropagation();
    finish(false);
  } else if (e.key === 'Tab') {
    e.preventDefault();
    e.stopPropagation();
    const order = [cancelButton, confirmButton];
    const idx = order.indexOf(document.activeElement as HTMLButtonElement);
    const next = e.shiftKey ? (idx <= 0 ? order.length - 1 : idx - 1) : (idx < 0 || idx === order.length - 1 ? 0 : idx + 1);
    order[next].focus();
  }
}

function finish(result: boolean): void {
  if (!overlay) return;
  document.removeEventListener('keydown', onKeyDown, true);
  // Let the exit transition play, then drop the element.
  const closing = overlay;
  closing.classList.remove('tok-open');
  setTimeout(() => closing.remove(), 260);
  overlay = null;
  cancelButton = null;
  confirmButton = null;
  const resolve = settle;
  settle = null;
  const prev = previousFocus;
  previousFocus = null;
  if (prev && prev.isConnected) prev.focus();
  resolve?.(result);
}

export function confirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  if (overlay) finish(false);

  const active = document.activeElement;
  previousFocus = active instanceof HTMLElement && active !== document.body ? active : null;

  const titleId = `tok-confirm-title-${++titleSeq}`;
  const messageId = `tok-confirm-msg-${titleSeq}`;
  overlay = el('div', 'tok-overlay tok-open tok-confirm-overlay', {
    role: 'alertdialog',
    'aria-modal': 'true',
    'aria-labelledby': titleId,
    'aria-describedby': messageId
  });
  overlay.dir = i18n.getDirection();
  overlay.addEventListener('mousedown', (e) => {
    if (e.target === overlay) finish(false);
  });

  const dialog = el('div', 'tok-dialog', { style: 'width:460px;max-width:calc(100vw - 48px);' });

  const head = el('div', 'tok-dialog-head');
  const headText = el('div', 'tok-dialog-head-text');
  headText.appendChild(el('h2', undefined, { id: titleId }, options.title));
  head.appendChild(headText);
  dialog.appendChild(head);

  const body = el('div', 'tok-dialog-body', { style: 'flex-direction:column;gap:8px;padding:24px 28px;' });
  body.appendChild(el('p', undefined, { id: messageId, style: 'font-size:14px;line-height:1.55;' }, options.message));
  if (options.detail) {
    body.appendChild(el('p', undefined, { style: 'font-size:13px;line-height:1.5;color:var(--tok-text-secondary);' }, options.detail));
  }
  dialog.appendChild(body);

  const foot = el('div', 'tok-dialog-foot');
  foot.appendChild(el('span', 'tok-grow'));
  cancelButton = button(options.cancelLabel, { className: 'tok-btn', onClick: () => finish(false) });
  confirmButton = button(options.confirmLabel, {
    className: options.danger ? 'tok-btn tok-btn-danger' : 'tok-btn tok-btn-primary',
    onClick: () => finish(true)
  });
  foot.appendChild(cancelButton);
  foot.appendChild(confirmButton);
  dialog.appendChild(foot);

  overlay.appendChild(dialog);
  document.body.appendChild(overlay);
  document.addEventListener('keydown', onKeyDown, true);
  // Cancel is the safe default for a destructive prompt.
  cancelButton.focus();

  return new Promise<boolean>((resolve) => {
    settle = resolve;
  });
}

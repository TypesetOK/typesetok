import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { fillAppVersion } from '../appInfo';
import { MAIN_DEVELOPER, CONTRIBUTORS, Contributor } from '../contributors.generated';

export interface AboutModalCallbacks {
  onClose: () => void;
  onCheckUpdates?: () => void;
  onOpenWhatsNew?: () => void;
}

const REPO_URL = 'https://github.com/TypesetOK/typesetok';

/** Only https GitHub profile links are ever opened from this screen. */
function safeProfileUrl(url: string): string | null {
  try {
    const u = new URL(url);
    return u.protocol === 'https:' && u.hostname === 'github.com' ? u.href : null;
  } catch {
    return null;
  }
}

function openExternalUrl(url: string): void {
  const win = window as any;
  if (win.tokIpc && win.tokIpc.openExternal) {
    Promise.resolve(win.tokIpc.openExternal(url)).catch((e: any) => {
      console.warn('[ABOUT] openExternal failed:', e?.message ?? e);
    });
  } else {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}

/** Profile picture (embedded data: URI) or, when there is none, a round badge with the first letter. */
function avatarEl(c: Contributor, size: number): HTMLElement {
  const round = `width:${size}px;height:${size}px;border-radius:50%;flex:none;`;
  if (c.avatar.startsWith('data:image/')) {
    return el('img', undefined, { src: c.avatar, alt: '', width: String(size), height: String(size), style: `${round}object-fit:cover;background:var(--tok-bg-surface-2);` });
  }
  return el('span', undefined, {
    'aria-hidden': 'true',
    style: `${round}display:flex;align-items:center;justify-content:center;font-weight:700;font-size:${Math.round(size * 0.42)}px;background:var(--tok-accent-soft);color:var(--tok-accent-text);`
  }, (c.name || c.login).charAt(0).toUpperCase());
}

export class AboutModal {
  public element: HTMLElement;
  private callbacks: AboutModalCallbacks;
  private isVisible = false;
  private modal: ModalController;

  constructor(callbacks: AboutModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-about-overlay');

    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(): void {
    this.isVisible = true;
    this.element.classList.add('tok-open');
    this.render();
    this.modal.opened();
  }

  public hide(): void {
    if (!this.isVisible) return;
    this.isVisible = false;
    this.element.classList.remove('tok-open');
    this.modal.closed();
    this.callbacks.onClose();
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.renderContent();
    this.modal.afterRender(focusKey);
  }

  private renderContent(): void {
    this.element.innerHTML = '';
    this.element.dir = i18n.getDirection();

    const card = el('div', 'tok-dialog tok-about-card');
    card.style.width = '680px';
    card.style.maxHeight = '88vh';

    // Header
    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, t('aboutTitle')));
    head.appendChild(headText);
    head.appendChild(iconButton('close', t('aboutClose'), () => this.hide(), { attrs: { 'data-focus-key': 'x' } }));
    card.appendChild(head);

    // Scrollable Body
    const body = el('div', 'tok-about-body', {
      style: 'flex:1;overflow-y:auto;padding:24px 28px;display:flex;flex-direction:column;gap:20px;'
    });

    // 1. Hero brand banner
    const hero = el('div', 'tok-card', {
      style: 'align-self:stretch;display:flex;align-items:center;gap:18px;padding:18px 20px;background:var(--tok-bg-surface-2);border-radius:12px;'
    });
    const brandMark = el('span', 'tok-brand-mark', {
      'aria-hidden': 'true',
      style: 'width:56px;height:56px;border-radius:14px;box-shadow:0 3px 10px rgba(30,74,158,0.25);flex-shrink:0;'
    });
    brandMark.appendChild(icon('brand', 32));
    hero.appendChild(brandMark);

    const heroDetails = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:4px;flex:1;' });
    const titleRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:10px;flex-wrap:wrap;' });
    titleRow.appendChild(el('h1', undefined, { style: 'font-size:20px;font-weight:800;letter-spacing:-0.02em;' }, 'TypesetOK (TOK)'));
    const verBadge = el('span', 'tok-badge tok-badge-success', { dir: 'ltr' }, 'Stable');
    verBadge.setAttribute('data-app-version', '');
    fillAppVersion(verBadge, (v) => `v${v} Stable`);
    titleRow.appendChild(verBadge);
    heroDetails.appendChild(titleRow);
    heroDetails.appendChild(el('p', undefined, {
      style: 'font-size:14px;color:var(--tok-text-secondary);line-height:1.5;'
    }, t('aboutAppDesc')));
    hero.appendChild(heroDetails);
    body.appendChild(hero);

    // 2. Main developer
    const dev = el('div', 'tok-card', { style: 'align-self:stretch;display:flex;align-items:center;gap:18px;padding:20px 22px;' });
    dev.appendChild(avatarEl(MAIN_DEVELOPER, 76));
    const devText = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:3px;flex:1;min-width:0;text-align:start;' });
    devText.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);' }, t('aboutDevelopedBy')));
    devText.appendChild(el('h2', undefined, { style: 'font-size:20px;font-weight:700;' }, t('aboutMainDevName')));
    devText.appendChild(el('p', undefined, { style: 'font-size:14px;color:var(--tok-text-secondary);line-height:1.5;' }, t('aboutMainDevBio')));
    const devUrl = safeProfileUrl(MAIN_DEVELOPER.url);
    if (devUrl) {
      const link = button(t('aboutProfileLink'), {
        className: 'tok-btn tok-btn-sm',
        icon: 'brand',
        attrs: { 'data-focus-key': 'main-dev' },
        onClick: () => openExternalUrl(devUrl)
      });
      link.style.alignSelf = 'flex-start';
      link.style.marginTop = '6px';
      devText.appendChild(link);
    }
    dev.appendChild(devText);
    body.appendChild(dev);

    // 3. Other contributors (smaller)
    const people = CONTRIBUTORS.filter((c) => safeProfileUrl(c.url));
    if (people.length > 0) {
      const thanks = el('div', undefined, { style: 'align-self:stretch;display:flex;flex-direction:column;gap:10px;padding:2px 4px;text-align:start;' });
      thanks.appendChild(el('span', undefined, { style: 'font-size:13px;font-weight:700;color:var(--tok-text-primary);' }, t('aboutThanksTitle')));
      thanks.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);margin-top:-6px;' }, t('aboutThanksSub')));
      const grid = el('div', undefined, { style: 'display:flex;flex-wrap:wrap;gap:8px;' });
      for (const c of people) {
        const url = safeProfileUrl(c.url)!;
        const chip = el('button', 'tok-btn tok-btn-sm', {
          type: 'button',
          title: `${c.name} (@${c.login})`,
          'data-focus-key': `contrib-${c.login}`,
          style: 'height:36px;padding:0 12px 0 6px;gap:8px;border-radius:18px;'
        });
        chip.appendChild(avatarEl(c, 24));
        chip.appendChild(el('span', undefined, { style: 'font-size:12px;' }, c.name || c.login));
        chip.addEventListener('click', () => openExternalUrl(url));
        grid.appendChild(chip);
      }
      thanks.appendChild(grid);
      body.appendChild(thanks);
    }

    card.appendChild(body);

    // Footer with buttons
    const foot = el('div', 'tok-dialog-foot');

    foot.appendChild(button(t('aboutGithubBtn'), {
      className: 'tok-btn',
      icon: 'brand',
      attrs: { 'data-focus-key': 'github' },
      onClick: () => openExternalUrl(REPO_URL)
    }));

    foot.appendChild(button(t('whatsNewTitle'), {
      className: 'tok-btn',
      icon: 'sparkle',
      attrs: { 'data-focus-key': 'whats-new' },
      onClick: () => {
        this.hide();
        if (this.callbacks.onOpenWhatsNew) {
          this.callbacks.onOpenWhatsNew();
        } else {
          (window as any).tokApp?.openWhatsNew?.();
        }
      }
    }));

    foot.appendChild(el('span', 'tok-grow'));

    foot.appendChild(button(t('aboutClose'), {
      className: 'tok-btn tok-btn-primary',
      attrs: { 'data-focus-key': 'close' },
      onClick: () => this.hide()
    }));
    card.appendChild(foot);

    this.element.appendChild(card);
  }
}

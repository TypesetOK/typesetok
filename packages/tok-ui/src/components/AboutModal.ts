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

function isSafeGitHubUrl(url: string): boolean {
  try {
    const u = new URL(url);
    return u.hostname === 'github.com' && u.protocol === 'https:';
  } catch {
    return false;
  }
}

function openSafeLink(url: string): void {
  if (!isSafeGitHubUrl(url)) return;
  const win = window as any;
  if (win.tokIpc && win.tokIpc.openExternal) {
    Promise.resolve(win.tokIpc.openExternal(url)).catch((e: any) => {
      console.warn('[ABOUT] openExternal failed:', e?.message ?? e);
    });
  } else {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
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
    card.style.width = '620px';
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
      style: 'flex:1;overflow-y:auto;padding:24px 28px;display:flex;flex-direction:column;gap:18px;'
    });

    // 1. Hero Brand Banner
    const hero = el('div', 'tok-card', {
      style: 'display:flex;align-items:center;gap:18px;padding:18px 20px;background:var(--tok-bg-surface-2);border-radius:12px;'
    });

    const brandMark = el('span', 'tok-brand-mark', {
      'aria-hidden': 'true',
      style: 'width:56px;height:56px;border-radius:14px;box-shadow:0 3px 10px rgba(30,74,158,0.25);flex-shrink:0;'
    });
    brandMark.appendChild(icon('brand', 32));
    hero.appendChild(brandMark);

    const heroDetails = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:4px;flex:1;' });
    const titleRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:10px;flex-wrap:wrap;' });
    titleRow.appendChild(el('h1', undefined, { style: 'font-size:20px;font-weight:800;letter-spacing:-0.02em;margin:0;' }, 'TypesetOK (TOK)'));

    const verBadge = el('span', 'tok-badge tok-badge-success', { dir: 'ltr' }, 'Stable');
    verBadge.setAttribute('data-app-version', '');
    fillAppVersion(verBadge, (v) => `v${v} Stable`);
    titleRow.appendChild(verBadge);
    heroDetails.appendChild(titleRow);

    heroDetails.appendChild(el('p', undefined, {
      style: 'font-size:13px;color:var(--tok-text-secondary);line-height:1.45;margin:0;'
    }, t('aboutAppDesc')));

    hero.appendChild(heroDetails);
    body.appendChild(hero);

    // 2. Main Developer Section
    const devCard = el('div', 'tok-card', { style: 'display:flex;flex-direction:column;gap:12px;padding:16px 18px;' });
    const devHead = el('div', undefined, { style: 'font-size:12px;font-weight:600;color:var(--tok-text-muted);text-transform:uppercase;letter-spacing:0.05em;' }, t('aboutDevelopedBy'));
    devCard.appendChild(devHead);

    const devRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:14px;' });
    if (MAIN_DEVELOPER.avatar) {
      const avatarImg = el('img', undefined, {
        src: MAIN_DEVELOPER.avatar,
        alt: MAIN_DEVELOPER.name,
        style: 'width:48px;height:48px;border-radius:50%;object-fit:cover;border:1px solid var(--tok-border-strong);flex-shrink:0;'
      });
      devRow.appendChild(avatarImg);
    } else {
      const avatarPlaceholder = el('div', undefined, {
        style: 'width:48px;height:48px;border-radius:50%;background:var(--tok-accent-soft);color:var(--tok-accent-text);display:flex;align-items:center;justify-content:center;font-weight:700;font-size:18px;flex-shrink:0;'
      }, MAIN_DEVELOPER.name ? MAIN_DEVELOPER.name.charAt(0) : 'A');
      devRow.appendChild(avatarPlaceholder);
    }

    const devInfo = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:3px;flex:1;min-width:0;' });
    const devNameRow = el('div', undefined, { style: 'display:flex;align-items:center;gap:8px;' });
    devNameRow.appendChild(el('strong', undefined, { style: 'font-size:15px;font-weight:700;' }, t('aboutMainDevName')));
    devNameRow.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);' }, `@${MAIN_DEVELOPER.login}`));
    devInfo.appendChild(devNameRow);

    devInfo.appendChild(el('span', undefined, { style: 'font-size:13px;color:var(--tok-text-secondary);' }, t('aboutMainDevBio')));
    devRow.appendChild(devInfo);

    if (MAIN_DEVELOPER.url) {
      const profileBtn = button(t('aboutProfileLink'), {
        className: 'tok-btn tok-btn-sm',
        icon: 'brand',
        onClick: () => openSafeLink(MAIN_DEVELOPER.url)
      });
      devRow.appendChild(profileBtn);
    }
    devCard.appendChild(devRow);
    body.appendChild(devCard);

    // 3. Contributors Section
    if (CONTRIBUTORS && CONTRIBUTORS.length > 0) {
      const contribCard = el('div', 'tok-card', { style: 'display:flex;flex-direction:column;gap:12px;padding:16px 18px;' });
      const contribHead = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:2px;' });
      contribHead.appendChild(el('strong', undefined, { style: 'font-size:14px;font-weight:700;' }, t('aboutThanksTitle')));
      contribHead.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);' }, t('aboutThanksSub')));
      contribCard.appendChild(contribHead);

      const contribList = el('div', undefined, {
        style: 'display:grid;grid-template-columns:repeat(auto-fill, minmax(180px, 1fr));gap:10px;'
      });

      for (const contrib of CONTRIBUTORS) {
        const item = el('button', 'tok-card', {
          type: 'button',
          style: 'display:flex;align-items:center;gap:10px;padding:8px 12px;border:1px solid var(--tok-border-subtle);background:var(--tok-bg-surface-2);border-radius:8px;text-align:start;cursor:pointer;transition:all 0.15s ease;'
        });

        if (contrib.avatar) {
          const img = el('img', undefined, {
            src: contrib.avatar,
            alt: contrib.name,
            style: 'width:32px;height:32px;border-radius:50%;object-fit:cover;flex-shrink:0;'
          });
          item.appendChild(img);
        } else {
          const initials = el('div', undefined, {
            style: 'width:32px;height:32px;border-radius:50%;background:var(--tok-accent-soft);color:var(--tok-accent-text);display:flex;align-items:center;justify-content:center;font-weight:600;font-size:13px;flex-shrink:0;'
          }, contrib.name ? contrib.name.charAt(0) : 'U');
          item.appendChild(initials);
        }

        const nameCol = el('div', undefined, { style: 'display:flex;flex-direction:column;overflow:hidden;' });
        nameCol.appendChild(el('span', undefined, {
          style: 'font-size:13px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;'
        }, contrib.name || contrib.login));
        nameCol.appendChild(el('span', undefined, {
          style: 'font-size:11px;color:var(--tok-text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;'
        }, `@${contrib.login}`));
        item.appendChild(nameCol);

        item.addEventListener('click', () => {
          if (contrib.url) openSafeLink(contrib.url);
        });

        contribList.appendChild(item);
      }
      contribCard.appendChild(contribList);
      body.appendChild(contribCard);
    }

    card.appendChild(body);

    // Footer with buttons
    const foot = el('div', 'tok-dialog-foot');

    foot.appendChild(button(t('aboutGithubBtn'), {
      className: 'tok-btn',
      icon: 'brand',
      attrs: { 'data-focus-key': 'github' },
      onClick: () => openSafeLink(REPO_URL)
    }));

    foot.appendChild(button(t('updatesCheckNow'), {
      className: 'tok-btn',
      icon: 'refresh',
      attrs: { 'data-focus-key': 'updates' },
      onClick: () => {
        this.hide();
        if (this.callbacks.onCheckUpdates) {
          this.callbacks.onCheckUpdates();
        } else {
          (window as any).tokApp?.openSettings();
        }
      }
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


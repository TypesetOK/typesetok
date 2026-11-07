import { t, i18n } from '../i18n';
import { el, icon } from '../ui';
import { UpdatesStateManager } from './updatesManager';

export interface UpdatesBellButtonOptions {
  onClick: () => void;
  className?: string;
  size?: number;
}

export class UpdatesBellButton {
  public element: HTMLButtonElement;
  private stateManager: UpdatesStateManager;
  private badgeDot: HTMLElement;
  private unsubscribe?: () => void;
  private onClickHandler: () => void;

  constructor(options: UpdatesBellButtonOptions, customManager?: UpdatesStateManager) {
    this.onClickHandler = options.onClick;
    this.stateManager = customManager || UpdatesStateManager.getInstance();

    this.element = el('button', options.className || 'tok-btn tok-btn-ghost tok-btn-icon tok-updates-bell-btn', {
      type: 'button',
      'aria-label': t('whatsNewTitle'),
      title: t('whatsNewTitle')
    }) as HTMLButtonElement;

    // Vector Icon
    const iconEl = icon('sparkle', options.size || 18);
    this.element.appendChild(iconEl);

    // Unread indicator dot
    this.badgeDot = el('span', 'tok-updates-unread-dot', {
      'aria-hidden': 'true'
    });
    this.element.appendChild(this.badgeDot);

    this.element.addEventListener('click', () => {
      this.onClickHandler();
    });

    this.updateStatus();

    this.unsubscribe = this.stateManager.subscribe(() => {
      this.updateStatus();
    });

    i18n.onChange(() => {
      this.element.setAttribute('aria-label', t('whatsNewTitle'));
      this.element.setAttribute('title', t('whatsNewTitle'));
    });
  }

  public updateStatus(): void {
    const unreadCount = this.stateManager.getUnreadCount();
    const hasUnread = unreadCount > 0;

    if (hasUnread) {
      this.badgeDot.classList.add('tok-visible');
      this.badgeDot.textContent = unreadCount > 99 ? '99+' : String(unreadCount);
      this.element.classList.add('tok-has-unread');
      this.element.setAttribute('aria-label', `${t('whatsNewTitle')} (${unreadCount} ${t('whatsNewUnreadAria')})`);
    } else {
      this.badgeDot.classList.remove('tok-visible');
      this.badgeDot.textContent = '';
      this.element.classList.remove('tok-has-unread');
      this.element.setAttribute('aria-label', t('whatsNewTitle'));
    }
  }

  public destroy(): void {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = undefined;
    }
  }
}

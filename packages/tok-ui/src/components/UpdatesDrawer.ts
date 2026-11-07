import { t, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { UpdatesStateManager } from '../updates/updatesManager';
import { ChangelogEntryCard } from '../updates/ChangelogEntryCard';
import { UpdatesFilterOptions, ChangeCategory, UpdateImportance } from '../updates/types';

export interface UpdatesDrawerCallbacks {
  onClose: () => void;
  onCheckUpdates?: () => void;
  onOpenSettings?: () => void;
}

export class UpdatesDrawer {
  public element: HTMLElement;
  private callbacks: UpdatesDrawerCallbacks;
  private isVisible = false;
  private modal: ModalController;
  private stateManager: UpdatesStateManager;
  private filterOptions: UpdatesFilterOptions = { category: 'all', importance: 'all', searchQuery: '' };
  private cardsContainer!: HTMLElement;
  private searchInput!: HTMLInputElement;

  constructor(callbacks: UpdatesDrawerCallbacks, customManager?: UpdatesStateManager) {
    this.callbacks = callbacks;
    this.stateManager = customManager || UpdatesStateManager.getInstance();
    this.element = el('div', 'tok-overlay tok-updates-overlay');

    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(targetVersion?: string): void {
    this.isVisible = true;
    this.element.classList.add('tok-open');

    // Mark current updates as seen when user opens the drawer
    if (targetVersion) {
      this.stateManager.markVersionAsSeen(targetVersion);
    } else {
      this.stateManager.markAllAsSeen();
    }

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

  public isOpen(): boolean {
    return this.isVisible;
  }

  public render(): void {
    const focusKey = this.modal.captureFocus();
    this.element.innerHTML = '';
    this.element.dir = i18n.getDirection();

    // Drawer container
    const drawer = el('div', 'tok-drawer tok-updates-drawer');
    drawer.setAttribute('role', 'dialog');
    drawer.setAttribute('aria-modal', 'true');
    drawer.setAttribute('aria-label', t('whatsNewTitle'));

    // 1. Drawer Header
    const head = el('header', 'tok-drawer-head');
    const headIconWrap = el('div', 'tok-drawer-head-icon');
    headIconWrap.appendChild(icon('sparkle', 22));
    head.appendChild(headIconWrap);

    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, t('whatsNewTitle')));
    headText.appendChild(el('p', undefined, undefined, t('whatsNewSubtitle')));
    head.appendChild(headText);

    head.appendChild(iconButton('close', t('aboutClose'), () => this.hide(), {
      attrs: { 'data-focus-key': 'close-x' }
    }));
    drawer.appendChild(head);

    // 2. Filter & Search Controls Bar
    const filterSection = el('div', 'tok-updates-filter-bar');

    // Search input
    const searchWrap = el('div', 'tok-updates-search-wrap');
    searchWrap.appendChild(icon('search', 16));
    this.searchInput = el('input', 'tok-input tok-updates-search-input', {
      type: 'search',
      placeholder: t('whatsNewSearchPlaceholder'),
      'aria-label': t('whatsNewSearchPlaceholder'),
      value: this.filterOptions.searchQuery || ''
    }) as HTMLInputElement;

    this.searchInput.addEventListener('input', () => {
      this.filterOptions.searchQuery = this.searchInput.value;
      this.populateCards();
    });
    searchWrap.appendChild(this.searchInput);
    filterSection.appendChild(searchWrap);

    // Filter Chips
    const chipsRow = el('div', 'tok-updates-filter-chips', { role: 'tablist' });

    const filterPresets: Array<{ id: string; label: string; importance?: UpdateImportance | 'all'; category?: ChangeCategory | 'all' }> = [
      { id: 'all', label: t('whatsNewFilterAll'), importance: 'all', category: 'all' },
      { id: 'major', label: t('whatsNewFilterMajor'), importance: 'major', category: 'all' },
      { id: 'improvements', label: t('whatsNewFilterMinor'), importance: 'all', category: 'improvements' },
      { id: 'fixes', label: t('whatsNewFilterFixes'), importance: 'all', category: 'fixes' },
      { id: 'typography', label: t('whatsNewFilterTypography'), importance: 'all', category: 'typography' }
    ];

    for (const preset of filterPresets) {
      const isSelected = (preset.importance ? this.filterOptions.importance === preset.importance : true) &&
                         (preset.category ? this.filterOptions.category === preset.category : true);

      const chip = el('button', `tok-filter-chip ${isSelected ? 'tok-filter-chip-active' : ''}`, {
        type: 'button',
        role: 'tab',
        'aria-selected': String(isSelected)
      }, preset.label);

      chip.addEventListener('click', () => {
        this.filterOptions.importance = preset.importance;
        this.filterOptions.category = preset.category;
        drawer.querySelectorAll('.tok-filter-chip').forEach((c) => {
          c.classList.remove('tok-filter-chip-active');
          c.setAttribute('aria-selected', 'false');
        });
        chip.classList.add('tok-filter-chip-active');
        chip.setAttribute('aria-selected', 'true');
        this.populateCards();
      });

      chipsRow.appendChild(chip);
    }

    filterSection.appendChild(chipsRow);
    drawer.appendChild(filterSection);

    // 3. Scrollable Changelog Cards List
    const body = el('div', 'tok-drawer-body tok-updates-body');
    this.cardsContainer = el('div', 'tok-updates-cards-list');
    body.appendChild(this.cardsContainer);
    drawer.appendChild(body);

    // 4. Drawer Footer
    const foot = el('footer', 'tok-drawer-foot');

    foot.appendChild(el('span', 'tok-grow'));

    const closeBtn = button(t('whatsNewGotIt'), {
      className: 'tok-btn tok-btn-primary',
      attrs: { 'data-focus-key': 'close-btn' },
      onClick: () => this.hide()
    });
    foot.appendChild(closeBtn);

    drawer.appendChild(foot);
    this.element.appendChild(drawer);

    this.populateCards();
    this.modal.afterRender(focusKey);
  }

  private populateCards(): void {
    this.cardsContainer.innerHTML = '';
    const lang = i18n.getLanguage();
    const entries = this.stateManager.filterEntries(this.filterOptions, lang);

    if (entries.length === 0) {
      const empty = el('div', 'tok-updates-empty-state');
      empty.appendChild(icon('info', 28));
      empty.appendChild(el('p', undefined, undefined, t('whatsNewNoResults')));
      this.cardsContainer.appendChild(empty);
      return;
    }

    for (let idx = 0; idx < entries.length; idx++) {
      const entry = entries[idx];
      const isCurrent = idx === 0;
      const card = new ChangelogEntryCard(entry, { isCurrent, defaultExpanded: isCurrent });
      this.cardsContainer.appendChild(card.element);
    }
  }
}

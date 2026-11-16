import { ChangelogEntry, LocalizedString } from './types';
import { i18n, Language } from '../i18n';
import { el, icon } from '../ui';
import { createVersionBadge, createCategoryPill } from './updatePills';

export interface ChangelogCardOptions {
  isCurrent?: boolean;
  defaultExpanded?: boolean;
}

/**
 * "Current" and "previous" are decided by the card from the list position, never by the data.
 * Older changelog entries carry these words as their badge, so they are ignored as custom badges.
 */
const LIFECYCLE_BADGE_TEXTS = new Set(['גרסה נוכחית', 'גרסה קודמת', 'current version', 'previous version']);

function isLifecycleBadge(badge?: LocalizedString): boolean {
  if (!badge) return false;
  return LIFECYCLE_BADGE_TEXTS.has((badge.he || '').trim()) || LIFECYCLE_BADGE_TEXTS.has((badge.en || '').trim().toLowerCase());
}

export class ChangelogEntryCard {
  public element: HTMLElement;
  private entry: ChangelogEntry;
  private isExpanded: boolean;
  private isCurrent: boolean;

  constructor(entry: ChangelogEntry, options?: ChangelogCardOptions) {
    this.entry = entry;
    this.isCurrent = Boolean(options?.isCurrent);
    this.isExpanded = options?.defaultExpanded ?? this.isCurrent;
    this.element = el('article', `tok-changelog-card tok-changelog-${this.entry.importance}${this.isExpanded ? ' tok-changelog-expanded' : ' tok-changelog-collapsed'}`);
    this.render();
  }

  public toggle(): void {
    this.isExpanded = !this.isExpanded;
    this.element.classList.toggle('tok-changelog-expanded', this.isExpanded);
    this.element.classList.toggle('tok-changelog-collapsed', !this.isExpanded);
    this.render();
  }

  public render(): void {
    const lang: Language = i18n.getLanguage();
    this.element.innerHTML = '';
    this.element.setAttribute('data-version', this.entry.version);

    // Header row: version, badge, toggle button, date
    const header = el('header', 'tok-changelog-card-header');
    const titleGroup = el('div', 'tok-changelog-version-group');

    const versionTitle = el('h3', 'tok-changelog-version-tag', undefined, `v${this.entry.version}`);
    titleGroup.appendChild(versionTitle);

    // "Current Version" is shown only on the running version; a real custom badge (e.g. "Major Release") is kept next to it.
    if (this.isCurrent) {
      titleGroup.appendChild(el('span', 'tok-badge tok-badge-success', undefined, lang === 'he' ? 'גרסה נוכחית' : 'Current Version'));
    }
    if (this.entry.badge && !isLifecycleBadge(this.entry.badge)) {
      titleGroup.appendChild(createVersionBadge(this.entry.importance, this.entry.badge));
    }

    header.appendChild(titleGroup);

    const rightGroup = el('div', 'tok-changelog-header-right');

    const dateSpan = el('time', 'tok-changelog-date', {
      datetime: this.entry.releaseDate,
      dir: 'ltr'
    }, this.entry.releaseDate);
    rightGroup.appendChild(dateSpan);

    const toggleBtn = el('button', 'tok-changelog-toggle-btn', {
      type: 'button',
      'aria-expanded': String(this.isExpanded),
      title: this.isExpanded ? (lang === 'he' ? 'צמצם פירוט' : 'Collapse details') : (lang === 'he' ? 'הצג פירוט' : 'Show details')
    });
    toggleBtn.appendChild(icon(this.isExpanded ? 'chevronUp' : 'chevronDown', 13));
    const toggleText = el('span', 'tok-changelog-toggle-label', undefined,
      this.isExpanded
        ? (lang === 'he' ? 'צמצם' : 'Collapse')
        : (lang === 'he' ? 'פירוט השינויים' : 'Details')
    );
    toggleBtn.appendChild(toggleText);
    toggleBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.toggle();
    });
    rightGroup.appendChild(toggleBtn);

    header.appendChild(rightGroup);
    header.style.cursor = 'pointer';
    header.addEventListener('click', () => this.toggle());

    this.element.appendChild(header);

    // Entry Title & Summary
    const titleText = this.entry.title[lang] || this.entry.title.en;
    const summaryText = this.entry.summary[lang] || this.entry.summary.en;

    const entryTitle = el('h4', 'tok-changelog-entry-title', undefined, titleText);
    this.element.appendChild(entryTitle);

    const summaryPara = el('p', 'tok-changelog-entry-summary', undefined, summaryText);
    this.element.appendChild(summaryPara);

    // Changes List (accordion expandable)
    if (this.isExpanded && this.entry.changes && this.entry.changes.length > 0) {
      const expandable = el('div', 'tok-changelog-expandable');
      const changesList = el('ul', 'tok-changelog-changes-list');

      for (const item of this.entry.changes) {
        const li = el('li', 'tok-changelog-change-item');
        const pill = createCategoryPill(item.category, item.tag);
        li.appendChild(pill);

        const textSpan = el('span', 'tok-changelog-change-text', undefined, item.text[lang] || item.text.en);
        li.appendChild(textSpan);

        changesList.appendChild(li);
      }

      expandable.appendChild(changesList);
      this.element.appendChild(expandable);
    }
  }

  public updateLanguage(): void {
    this.render();
  }
}

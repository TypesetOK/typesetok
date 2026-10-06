import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button } from '../ui';
import { ModalController } from './ModalController';
import { fillAppVersion } from '../appInfo';

export interface WelcomeModalCallbacks {
  onSelectTemplate: (templateId: string) => void;
  /** Open a .tok file; `path` is set when a file was dropped on the screen or selected. */
  onOpenProject: (path?: string) => void;
  onClose: () => void;
  onOpenSettings?: () => void;
  onOpenAbout?: () => void;
  onOpenGuide?: () => void;
}

/** A block of a page drawing: top/right/width/height in % of the page, and its kind. */
type ArtBlock = [number, number, number, number, 'tl' | 'tg' | 'tg2' | 'th' | 'th2' | 'tr'];

interface Template {
  id: string;
  titleKey: string;
  descKey: string;
  metaKey: string;
  art: ArtBlock[];
}

const TEMPLATES: Template[] = [
  {
    id: 'gemara', titleKey: 'templateGemara', descKey: 'templateGemaraDesc', metaKey: 'templateMetaGemara',
    art: [[6, 8, 84, 0, 'tr'], [10, 8, 24, 84, 'tl'], [10, 36, 28, 48, 'tg'], [10, 68, 24, 84, 'tl'], [61, 36, 56, 33, 'tl']]
  },
  {
    id: 'mikraot', titleKey: 'templateMikraot', descKey: 'templateMikraotDesc', metaKey: 'templateMetaMikraot',
    art: [[6, 8, 84, 0, 'tr'], [10, 8, 58, 22, 'tg'], [10, 70, 22, 22, 'tl'], [35, 8, 41, 58, 'tl'], [35, 51, 41, 27, 'tl'], [66, 51, 41, 27, 'tl']]
  },
  {
    id: 'prose', titleKey: 'templateProse', descKey: 'templateProseDesc', metaKey: 'templateMetaProse',
    art: [[14, 32, 36, 3, 'th'], [20, 42, 16, 0, 'tr'], [26, 14, 72, 62, 'tg2']]
  },
  {
    id: 'notes', titleKey: 'templateNotes', descKey: 'templateNotesDesc', metaKey: 'templateMetaNotes',
    art: [[10, 14, 72, 60, 'tg2'], [75, 58, 28, 0, 'tr'], [78, 14, 72, 12, 'tl']]
  },
  {
    id: 'bulletin', titleKey: 'templateBulletin', descKey: 'templateBulletinDesc', metaKey: 'templateMetaBulletin',
    art: [[9, 10, 80, 6, 'th'], [18, 26, 48, 2, 'th2'], [25, 10, 38, 64, 'tg2'], [25, 52, 38, 64, 'tg2']]
  }
];

export interface RecentProject {
  name: string;
  path?: string;
  time?: string;
  lastSavedAt?: string;
  lastClosedAt?: string;
  pages: number;
  template: number;
}

const DUMMY_PROJECT_NAMES = new Set([
  'מסכת ברכות — מהדורת מופת.tok',
  'ספר תהילים עם פירוש המילות.tok',
  'עלון שבת קודש — גיליון ק״מ.tok'
]);

/** Format accurate project timestamp into readable Hebrew/English date and time. */
export function formatProjectTimestamp(isoOrDate?: string): string {
  if (!isoOrDate || isoOrDate === 'זה עתה') {
    const d = new Date();
    const timeStr = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
    return `${d.getDate().toString().padStart(2, '0')}/${(d.getMonth() + 1).toString().padStart(2, '0')}/${d.getFullYear()} ${timeStr}`;
  }
  try {
    const d = new Date(isoOrDate);
    if (isNaN(d.getTime())) return isoOrDate;
    const now = new Date();
    const isToday = d.toDateString() === now.toDateString();
    const timeStr = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
    if (isToday) {
      return i18n.getLanguage() === 'en' ? `Today ${timeStr}` : `היום ${timeStr}`;
    }
    const dateStr = `${d.getDate().toString().padStart(2, '0')}/${(d.getMonth() + 1).toString().padStart(2, '0')}/${d.getFullYear()}`;
    return `${dateStr} ${timeStr}`;
  } catch {
    return isoOrDate;
  }
}

export function getRecentProjects(): RecentProject[] {
  try {
    const raw = localStorage.getItem('tok_recent_projects');
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed
          .filter((p) => p && typeof p.name === 'string' && !DUMMY_PROJECT_NAMES.has(p.name))
          .map((p) => {
            const accurateTime = p.lastSavedAt || p.lastClosedAt;
            return {
              ...p,
              time: formatProjectTimestamp(accurateTime || p.time)
            };
          });
      }
    }
  } catch {}
  return [];
}

export function addRecentProject(entry: {
  name: string;
  path?: string;
  pages?: number;
  template?: number;
  lastSavedAt?: string;
  lastClosedAt?: string;
}): void {
  try {
    const nowIso = new Date().toISOString();
    const saved = entry.lastSavedAt || nowIso;
    const list = getRecentProjects().filter((p) => p.name !== entry.name && p.path !== entry.path);
    list.unshift({
      name: entry.name,
      path: entry.path,
      lastSavedAt: saved,
      lastClosedAt: entry.lastClosedAt,
      time: formatProjectTimestamp(saved),
      pages: entry.pages ?? 1,
      template: entry.template ?? 0
    });
    localStorage.setItem('tok_recent_projects', JSON.stringify(list.slice(0, 50)));
  } catch {}
}

export function updateProjectClosedTime(nameOrPath: string): void {
  try {
    const list = getRecentProjects();
    const nowIso = new Date().toISOString();
    for (const p of list) {
      if (p.name === nameOrPath || p.path === nameOrPath) {
        p.lastClosedAt = nowIso;
        break;
      }
    }
    localStorage.setItem('tok_recent_projects', JSON.stringify(list));
  } catch {}
}

export function removeRecentProject(name: string): void {
  try {
    const list = getRecentProjects().filter((p) => p.name !== name);
    localStorage.setItem('tok_recent_projects', JSON.stringify(list));
  } catch {}
}

export function renameRecentProject(oldName: string, newName: string): void {
  try {
    const cleanNew = newName.trim();
    if (!cleanNew) return;
    const list = getRecentProjects().map((p) => {
      if (p.name === oldName) {
        return { ...p, name: cleanNew };
      }
      return p;
    });
    localStorage.setItem('tok_recent_projects', JSON.stringify(list));
  } catch {}
}

function pageArt(blocks: ArtBlock[], w: number, h: number): HTMLElement {
  const page = el('div', 'tok-page-art', { 'aria-hidden': 'true' });
  page.style.width = `${w}px`;
  page.style.height = `${h}px`;
  for (const [top, right, width, height, kind] of blocks) {
    const b = el('i', kind);
    b.style.top = `${top}%`;
    b.style.right = `${right}%`;
    b.style.width = `${width}%`;
    if (kind !== 'tr') b.style.height = `${height}%`;
    page.appendChild(b);
  }
  return page;
}

/**
 * Start screen: full-screen primary launcher for TypesetOK.
 */
export class WelcomeModal {
  public element: HTMLElement;
  private callbacks: WelcomeModalCallbacks;
  private isVisible = false;
  private hasOpenDocument = false;
  private modal: ModalController;
  private showAllProjects = false;

  constructor(callbacks: WelcomeModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-welcome-overlay');
    this.element.dir = i18n.getDirection();

    this.modal = new ModalController(this.element, () => this.close());

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      if (this.isVisible) this.render();
    });

    // Dropping a .tok file anywhere on the screen opens it.
    this.element.addEventListener('dragover', (e) => {
      if (!e.dataTransfer?.types.includes('Files')) return;
      e.preventDefault();
      this.element.querySelector('.tok-dropzone')?.classList.add('tok-drag');
    });
    this.element.addEventListener('dragleave', (e) => {
      if (e.target === this.element || !this.element.contains(e.relatedTarget as Node)) {
        this.element.querySelector('.tok-dropzone')?.classList.remove('tok-drag');
      }
    });
    this.element.addEventListener('drop', (e) => {
      const file = e.dataTransfer?.files?.[0];
      this.element.querySelector('.tok-dropzone')?.classList.remove('tok-drag');
      if (!file) return;
      e.preventDefault();
      const path = (file as File & { path?: string }).path || file.name;
      this.hide();
      this.callbacks.onOpenProject(path);
    });
  }

  public setHasOpenDocument(hasDoc: boolean): void {
    this.hasOpenDocument = hasDoc;
    if (this.isVisible) this.render();
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
  }

  private close(): void {
    if (!this.hasOpenDocument) return;
    this.hide();
    this.callbacks.onClose();
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.renderContent();
    this.modal.afterRender(focusKey);
  }

  private renderContent(): void {
    this.element.innerHTML = '';
    const isEn = i18n.getLanguage() === 'en';

    // ---- Top strip: Brand only (no redundant return buttons or settings) ----
    const topStrip = el('div', 'tok-welcome-top');
    const brandMark = el('span', 'tok-brand-mark', { 'aria-hidden': 'true' });
    brandMark.appendChild(icon('brand', 18));
    topStrip.appendChild(brandMark);
    topStrip.appendChild(el('span', 'tok-brand-name', undefined, 'TypesetOK'));
    topStrip.appendChild(el('span', 'tok-grow'));
    this.element.appendChild(topStrip);

    // ---- Body ----
    const body = el('div', 'tok-welcome-body');
    const card = el('div', 'tok-welcome-card');

    // New project section
    const main = el('section', 'tok-welcome-main', { 'aria-labelledby': 'tok-welcome-h' });
    const head = el('div');
    const h1 = el('h1', undefined, { id: 'tok-welcome-h', 'data-modal-title': '' }, t('welcomeTitle'));
    head.appendChild(h1);
    head.appendChild(el('p', 'tok-lead', undefined, t('welcomeSubtitle')));
    main.appendChild(head);

    const grid = el('div', 'tok-template-grid', { role: 'list', 'aria-label': t('newProject') });
    for (const tmpl of TEMPLATES) {
      const b = el('button', 'tok-template-card', { type: 'button', role: 'listitem', 'data-focus-key': `template-${tmpl.id}` });
      const art = el('span', 'tok-template-art');
      art.appendChild(pageArt(tmpl.art, 84, 118));
      b.appendChild(art);
      const text = el('span', 'tok-template-text');
      text.appendChild(el('span', 'tok-template-title', undefined, t(tmpl.titleKey)));
      text.appendChild(el('span', 'tok-template-desc', undefined, t(tmpl.descKey)));
      text.appendChild(el('span', 'tok-template-meta', undefined, t(tmpl.metaKey)));
      b.appendChild(text);
      b.addEventListener('click', () => {
        this.hide();
        this.callbacks.onSelectTemplate(tmpl.id);
      });
      grid.appendChild(b);
    }
    const blank = el('button', 'tok-template-card tok-template-blank', { type: 'button', role: 'listitem', 'data-focus-key': 'template-blank' });
    const plus = el('span', 'tok-plus', { 'aria-hidden': 'true' });
    plus.appendChild(icon('plus', 20));
    blank.appendChild(plus);
    blank.appendChild(el('span', 'tok-template-title', undefined, t('templateBlank')));
    blank.appendChild(el('span', 'tok-template-desc', undefined, t('templateBlankDesc')));
    blank.addEventListener('click', () => {
      this.hide();
      this.callbacks.onSelectTemplate('blank');
    });
    grid.appendChild(blank);
    main.appendChild(grid);
    card.appendChild(main);

    // Recent projects section (with delete, rename, view all)
    const side = el('aside', 'tok-recent', { 'aria-labelledby': 'tok-recent-h' });
    const sideHead = el('div', 'tok-recent-head');
    sideHead.appendChild(el('h2', undefined, { id: 'tok-recent-h' }, t('recentProjects')));

    const headActions = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px;' });

    const openBtn = button(t('openProject'), {
      className: 'tok-btn tok-btn-sm',
      icon: 'folder',
      iconSize: 15,
      attrs: { 'data-focus-key': 'open' },
      onClick: () => {
        this.hide();
        this.callbacks.onOpenProject();
      }
    });
    headActions.appendChild(openBtn);
    sideHead.appendChild(headActions);
    side.appendChild(sideHead);

    const recent = getRecentProjects();
    const list = el('div', undefined, { role: 'list', style: 'display:flex;flex-direction:column;gap:4px;flex:1;overflow-y:auto;max-height:360px;' });
    if (recent.length === 0) {
      const emptyBox = el('div', 'tok-recent-empty', { style: 'display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;padding:36px 12px;margin:auto 0' });
      const emptyIcon = el('div', undefined, { style: 'color:var(--tok-text-muted);opacity:0.6' });
      emptyIcon.appendChild(icon('history', 32));
      emptyBox.appendChild(emptyIcon);
      emptyBox.appendChild(el('strong', undefined, { style: 'font-size:13px;color:var(--tok-text-primary)' }, t('noRecentProjects')));
      emptyBox.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);text-align:center' }, t('noRecentProjectsSub')));
      list.appendChild(emptyBox);
    } else {
      const displayProjects = this.showAllProjects ? recent : recent.slice(0, 6);
      for (const rec of displayProjects) {
        const row = el('div', 'tok-recent-row', {
          role: 'listitem',
          style: 'display:flex;align-items:center;gap:8px;padding:8px 10px;border-radius:8px;cursor:pointer;position:relative;'
        });

        const tpl = TEMPLATES[rec.template] || TEMPLATES[0];
        row.appendChild(pageArt(tpl.art, 32, 44));

        const info = el('div', 'tok-recent-main', { style: 'flex:1;min-width:0;display:flex;flex-direction:column;gap:2px;' });
        const nameSpan = el('span', 'tok-recent-name', { title: rec.name, style: 'font-weight:600;font-size:13px;' }, rec.name.replace(/\.tok$/i, ''));
        info.appendChild(nameSpan);
        info.appendChild(el('span', 'tok-recent-sub', undefined, `${t(tpl.titleKey)} · ${tf('welcomePagesCount', { n: rec.pages })}`));
        row.appendChild(info);

        const timeSpan = el('span', 'tok-recent-when', { style: 'font-size:11px;color:var(--tok-text-muted);white-space:nowrap;margin-inline-end:4px;' }, rec.time || '');
        row.appendChild(timeSpan);

        // Action buttons container: Rename and Delete
        const actions = el('div', 'tok-recent-actions', {
          style: 'display:flex;align-items:center;gap:2px;'
        });
        actions.addEventListener('click', (e: MouseEvent) => e.stopPropagation());

        // Rename button
        const renameBtn = iconButton('typography', isEn ? 'Rename' : 'שינוי שם', () => {
          const currentClean = rec.name.replace(/\.tok$/i, '');
          const newName = window.prompt(isEn ? 'Enter new project name:' : 'הזן שם חדש לפרויקט:', currentClean);
          if (newName && newName.trim() && newName.trim() !== currentClean) {
            renameRecentProject(rec.name, `${newName.trim()}.tok`);
            this.render();
          }
        }, { size: 13, attrs: { style: 'width:24px;height:24px;padding:0;border-radius:4px;opacity:0.75;' } });
        actions.appendChild(renameBtn);

        // Delete button
        const delBtn = iconButton('close', isEn ? 'Remove from history' : 'מחיקה מההיסטוריה', () => {
          const confirmed = window.confirm(isEn ? `Remove "${rec.name}" from recent projects?` : `להסיר את "${rec.name}" מרשימת הפרויקטים?`);
          if (confirmed) {
            removeRecentProject(rec.name);
            this.render();
          }
        }, { size: 13, attrs: { style: 'width:24px;height:24px;padding:0;border-radius:4px;opacity:0.75;color:var(--tok-status-error);' } });
        actions.appendChild(delBtn);

        row.appendChild(actions);

        row.addEventListener('click', () => {
          this.hide();
          this.callbacks.onOpenProject(rec.path || rec.name);
        });
        list.appendChild(row);
      }
    }
    side.appendChild(list);

    // Toggle button for all projects if list has more than 6
    if (recent.length > 6) {
      const toggleAllBtn = button(this.showAllProjects ? (isEn ? 'Show less' : 'הצג פחות') : (isEn ? `View all (${recent.length})` : `הצג את כל הפרויקטים (${recent.length})`), {
        className: 'tok-btn tok-btn-ghost tok-btn-xs',
        onClick: () => {
          this.showAllProjects = !this.showAllProjects;
          this.render();
        }
      });
      toggleAllBtn.style.alignSelf = 'center';
      toggleAllBtn.style.marginTop = '4px';
      side.appendChild(toggleAllBtn);
    }

    const drop = el('div', 'tok-dropzone');
    const dropIcon = icon('folder', 22);
    dropIcon.style.color = 'var(--tok-text-muted)';
    drop.appendChild(dropIcon);
    drop.appendChild(el('span', undefined, undefined, t('welcomeDropHint')));
    side.appendChild(drop);
    card.appendChild(side);

    body.appendChild(card);
    this.element.appendChild(body);

    // ---- Footer: User Guide, Version, and Settings/About on bottom left ----
    const foot = el('footer', 'tok-welcome-foot', {
      style: 'height:46px;display:flex;align-items:center;padding:0 24px;border-top:1px solid var(--tok-border-subtle);'
    });

    const guide = el('button', 'tok-link', {
      type: 'button',
      style: 'display:flex;align-items:center;gap:6px;font-weight:500;cursor:pointer;'
    });
    guide.appendChild(icon('brand', 15));
    guide.appendChild(el('span', undefined, undefined, isEn ? 'User Guide' : 'מדריך למשתמש'));
    guide.addEventListener('click', () => {
      this.callbacks.onOpenGuide?.();
    });
    foot.appendChild(guide);

    foot.appendChild(el('span', 'tok-grow'));

    // Version label
    const verSpan = el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted);margin-inline-end:16px;' });
    fillAppVersion(verSpan, (v) => tf('welcomeVersion', { v }));
    foot.appendChild(verSpan);

    // Settings and About placed in the bottom left (matching the position inside a project)
    const bottomControls = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px;' });
    const settingsBtn = iconButton('settings', t('sidebarSettings'), () => {
      this.callbacks.onOpenSettings?.();
    }, { size: 16, attrs: { 'data-focus-key': 'settings', title: t('sidebarSettings'), style: 'width:32px;height:32px;' } });
    bottomControls.appendChild(settingsBtn);

    const aboutBtn = iconButton('info', t('sidebarAbout'), () => {
      this.callbacks.onOpenAbout?.();
    }, { size: 16, attrs: { 'data-focus-key': 'about', title: t('sidebarAbout'), style: 'width:32px;height:32px;' } });
    bottomControls.appendChild(aboutBtn);
    foot.appendChild(bottomControls);

    this.element.appendChild(foot);
  }
}

import { UpdatesBellButton } from '../updates/UpdatesBellButton';
import { ViewMode } from '../types';
import { t, i18n } from '../i18n';
import { el, icon, iconButton, kbd, button } from '../ui';

export interface TopSystemBarCallbacks {
  onMenuAction: (action: string, data?: unknown) => void;
  onOpenCommandPalette: () => void;
  onViewModeChange: (mode: ViewMode) => void;
  onExportPdf?: () => void;
  onOpenProjects?: () => void;
  onToggleLanguage?: () => void;
  onOpenSettings?: () => void;
  onOpenAbout?: () => void;
  onOpenWhatsNew?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onRenameDocument?: (newTitle: string) => void;
}

type MenuItem = { type: 'separator' } | { labelKey: string; shortcut?: string; action: string };

// Only shortcuts that are actually bound (Electron menu accelerators) are shown.
const MENU_ITEMS: MenuItem[] = [
  { labelKey: 'topBarProjects', shortcut: 'Ctrl+Shift+P', action: 'open-projects' },
  { labelKey: 'menuNewDocument', shortcut: 'Ctrl+N', action: 'new-document' },
  { labelKey: 'menuOpenDocument', shortcut: 'Ctrl+O', action: 'open-document' },
  { labelKey: 'menuImportDocument', shortcut: 'Ctrl+I', action: 'import-document' },
  { labelKey: 'menuSave', shortcut: 'Ctrl+S', action: 'save-document' },
  { labelKey: 'menuSaveAs', shortcut: 'Ctrl+Shift+S', action: 'save-as' },
  { labelKey: 'menuRenameDocument', shortcut: 'F2', action: 'rename-document' },
  { type: 'separator' },
  { labelKey: 'menuTextCleanup', shortcut: 'Ctrl+Shift+C', action: 'open-text-cleanup' },
  { labelKey: 'menuNormalizeNiqqud', action: 'normalize-hebrew' },
  { labelKey: 'menuShieldDivineNames', action: 'shield-divine-names' },
  { labelKey: 'menuRecalcGematria', action: 'recalculate-gematria' },
  { type: 'separator' },
  { labelKey: 'menuExportPdf', shortcut: 'Ctrl+E', action: 'export-pdf' },
  { type: 'separator' },
  { labelKey: 'sidebarSettings', shortcut: 'Ctrl+,', action: 'open-settings' },
  { labelKey: 'menuWhatsNew', action: 'open-whats-new' },
  { labelKey: 'sidebarAbout', action: 'open-about' },
  { type: 'separator' },
  { labelKey: 'menuExit', shortcut: 'Ctrl+Q', action: 'quit-app' }
];

/**
 * App slim title bar & system controls:
 * - Center: Brand icon + TypesetOK + ' - ' + Document Title (with inline rename and dirty status)
 * - Window controls: Minimize, Maximize/Restore, Close
 * - Side actions: Quick file menu, View switcher, Search pill, Undo/Redo
 */
export class TopSystemBar {
  public element: HTMLElement;
  private currentMode: ViewMode = 'canvas';
  private callbacks: TopSystemBarCallbacks;
  private activeDropdown: HTMLElement | null = null;
  private activeAnchor: HTMLElement | null = null;
  private documentTitle = '';
  private isDirty = false;
  private pageLabel = '';
  private pageLabelEl: HTMLElement | null = null;
  private undoBtn?: HTMLButtonElement;
  private redoBtn?: HTMLButtonElement;

  constructor(callbacks: TopSystemBarCallbacks) {
    this.callbacks = callbacks;
    this.element = document.createElement('header');
    this.element.className = 'tok-top-bar';
    this.element.dir = i18n.getDirection();

    i18n.onChange(() => {
      this.element.dir = i18n.getDirection();
      this.render();
    });

    this.element.addEventListener('dblclick', (e) => {
      const target = e.target as HTMLElement;
      if (target === this.element || target.classList.contains('tok-top-center') || target.classList.contains('tok-top-bar')) {
        this.toggleMaximize();
      }
    });

    this.render();

    // Close an open dropdown on click outside or Escape.
    document.addEventListener('pointerdown', (e) => {
      if (this.activeDropdown && !this.element.contains(e.target as Node)) this.closeDropdown();
    }, true);
    document.addEventListener('keydown', (e) => {
      if (this.activeDropdown && e.key === 'Escape') this.closeDropdown();
    });
  }

  public setDocumentTitle(title: string, isDirty: boolean = false): void {
    this.documentTitle = title;
    this.isDirty = isDirty;
    this.render();
  }

  public setDirty(isDirty: boolean): void {
    if (this.isDirty === isDirty) return;
    this.isDirty = isDirty;
    const nameEl = this.element.querySelector('.tok-doc-title-name');
    if (nameEl && this.documentTitle) {
      const cleanTitle = this.documentTitle.replace(/\.tok$/i, '');
      nameEl.textContent = isDirty ? `• ${cleanTitle}` : cleanTitle;
    }
  }

  /** Current page shown next to the document name (e.g. "דף ב׳ ע״ב"). */
  public setPageLabel(label: string): void {
    this.pageLabel = label;
    if (this.pageLabelEl) {
      this.pageLabelEl.textContent = label;
      this.pageLabelEl.hidden = !label;
    }
  }

  public setViewMode(mode: ViewMode): void {
    this.currentMode = mode;
    this.element.querySelectorAll<HTMLButtonElement>('.tok-seg-btn').forEach((b) => {
      const on = b.dataset.view === mode;
      b.setAttribute('aria-pressed', String(on));
    });
  }

  public setHistoryState(canUndo: boolean, canRedo: boolean): void {
    if (this.undoBtn) {
      this.undoBtn.disabled = !canUndo;
      this.undoBtn.style.opacity = canUndo ? '1' : '0.4';
    }
    if (this.redoBtn) {
      this.redoBtn.disabled = !canRedo;
      this.redoBtn.style.opacity = canRedo ? '1' : '0.4';
    }
  }

  public setUndoRedo(canUndo: boolean, canRedo: boolean): void {
    this.setHistoryState(canUndo, canRedo);
  }

  public startRenaming(): void {
    const titleEl = this.element.querySelector<HTMLElement>('.tok-doc-title-name');
    if (!titleEl || titleEl.querySelector('input')) return;

    const currentTitle = this.documentTitle.replace(/\.tok$/i, '');
    const input = document.createElement('input');
    input.type = 'text';
    input.value = currentTitle;
    input.className = 'tok-input tok-doc-title-input';
    input.setAttribute('aria-label', t('renameDocumentTooltip'));
    input.style.width = `${Math.max(120, currentTitle.length * 9 + 20)}px`;
    input.style.height = '22px';
    input.style.fontSize = '12px';
    input.style.padding = '0 6px';

    let committed = false;
    const commit = () => {
      if (committed) return;
      committed = true;
      const next = input.value.trim();
      if (next && next !== currentTitle) {
        this.callbacks.onRenameDocument?.(next);
      } else {
        titleEl.textContent = this.isDirty ? `• ${currentTitle}` : currentTitle;
      }
    };

    const cancel = () => {
      if (committed) return;
      committed = true;
      titleEl.textContent = this.isDirty ? `• ${currentTitle}` : currentTitle;
    };

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        commit();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        cancel();
      }
    });

    input.addEventListener('blur', () => commit());

    titleEl.replaceChildren(input);
    input.focus();
    input.select();
  }

  private render(): void {
    this.closeDropdown();
    this.element.innerHTML = '';

    // ---- Start side: Quick menu, view switcher, search, undo/redo ----
    const start = el('div', 'tok-top-side');

    const menuBtn = el('button', 'tok-btn tok-menu-btn', {
      type: 'button',
      'aria-haspopup': 'menu',
      'aria-expanded': 'false',
      title: t('topBarFileAndMenu')
    });
    menuBtn.appendChild(icon('folder', 14));
    menuBtn.appendChild(el('span', undefined, undefined, t('topBarFileAndMenu')));
    menuBtn.appendChild(icon('chevronDown', 10));
    menuBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.toggleQuickMenu(menuBtn);
    });
    start.appendChild(menuBtn);

    const views = el('div', 'tok-seg tok-seg-compact', { role: 'group', 'aria-label': t('topBarViewMode') });
    const modes: { id: ViewMode; label: string }[] = [
      { id: 'canvas', label: t('topBarViewCanvas') },
      { id: 'split', label: t('topBarViewSplit') },
      { id: 'story', label: t('topBarViewStory') }
    ];
    for (const m of modes) {
      const b = el('button', 'tok-seg-btn', { type: 'button', 'aria-pressed': String(this.currentMode === m.id), 'data-view': m.id }, m.label);
      b.addEventListener('click', () => {
        if (this.currentMode === m.id) return;
        this.setViewMode(m.id);
        this.callbacks.onViewModeChange(m.id);
      });
      views.appendChild(b);
    }
    start.appendChild(views);

    const search = el('button', 'tok-search-pill', { type: 'button', 'aria-keyshortcuts': 'Control+K' });
    search.appendChild(icon('search', 13));
    search.appendChild(el('span', undefined, undefined, t('topBarSearchPlaceholder')));
    search.appendChild(kbd('Ctrl K'));
    search.addEventListener('click', () => this.callbacks.onOpenCommandPalette());
    start.appendChild(search);

    const undo = iconButton('undo', t('topBarUndo'), () => this.callbacks.onUndo?.(), { attrs: { 'aria-keyshortcuts': 'Control+Z' }, size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });
    const redo = iconButton('redo', t('topBarRedo'), () => this.callbacks.onRedo?.(), { attrs: { 'aria-keyshortcuts': 'Control+Y' }, size: 14, className: 'tok-btn tok-btn-ghost tok-btn-xs' });
    undo.disabled = true;
    undo.style.opacity = '0.4';
    redo.disabled = true;
    redo.style.opacity = '0.4';
    this.undoBtn = undo;
    this.redoBtn = redo;
    if (i18n.getDirection() === 'rtl') {
      undo.replaceChildren(icon('redo', 14));
      redo.replaceChildren(icon('undo', 14));
    }
    start.appendChild(undo);
    start.appendChild(redo);

    if (this.callbacks.onOpenWhatsNew) {
      const bell = new UpdatesBellButton({
        onClick: () => this.callbacks.onOpenWhatsNew?.()
      });
      start.appendChild(bell.element);
    }

    this.element.appendChild(start);

    // ---- Center: Brand logo + "TypesetOK" + " - " + Project Title ----
    const center = el('div', 'tok-top-center');
    const brand = el('button', 'tok-brand-btn', {
      type: 'button',
      title: t('topBarProjects'),
      'aria-label': `TypesetOK · ${t('topBarProjects')}`
    });
    const mark = el('span', 'tok-brand-mark', { 'aria-hidden': 'true' });
    mark.appendChild(icon('brand', 13));
    brand.appendChild(mark);
    brand.appendChild(el('span', 'tok-brand-name', undefined, 'TypesetOK'));
    brand.addEventListener('click', () => this.callbacks.onOpenProjects?.());
    center.appendChild(brand);

    const cleanTitle = (this.documentTitle || 'ללא שם').replace(/\.tok$/i, '');
    const sep = el('span', 'tok-doc-title-sep', { 'aria-hidden': 'true' }, ' - ');
    center.appendChild(sep);

    const titleSpan = el('span', 'tok-doc-title-name', {
      tabindex: '0',
      role: 'button',
      title: t('renameDocumentTooltip'),
      style: 'cursor: pointer; border-radius: 3px; padding: 1px 4px;'
    }, this.isDirty ? `• ${cleanTitle}` : cleanTitle);
    titleSpan.addEventListener('dblclick', (e) => {
      e.stopPropagation();
      this.startRenaming();
    });
    titleSpan.addEventListener('keydown', (e) => {
      if (e.key === 'F2' || e.key === 'Enter') {
        e.preventDefault();
        this.startRenaming();
      }
    });
    center.appendChild(titleSpan);

    if (this.pageLabel) {
      const pageSep = el('span', 'tok-doc-title-sep', { 'aria-hidden': 'true' }, ' / ');
      center.appendChild(pageSep);
      this.pageLabelEl = el('span', 'tok-doc-title-page', undefined, this.pageLabel);
      center.appendChild(this.pageLabelEl);
    }

    this.element.appendChild(center);

    // ---- End side: Window control buttons (Minimize, Maximize/Restore, Close) ----
    const winControls = el('div', 'tok-window-controls');

    const minBtn = el('button', 'tok-win-btn tok-win-min', { type: 'button', title: 'מזער', 'aria-label': 'Minimize' });
    minBtn.innerHTML = '<svg viewBox="0 0 10 10"><line x1="1" y1="5" x2="9" y2="5" stroke-width="1.2" stroke-linecap="round"/></svg>';
    minBtn.addEventListener('click', () => this.minimizeWindow());

    const maxBtn = el('button', 'tok-win-btn tok-win-max', { type: 'button', title: 'הגדל/שחזר', 'aria-label': 'Maximize' });
    maxBtn.innerHTML = '<svg viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" fill="none" stroke-width="1.1"/></svg>';
    maxBtn.addEventListener('click', () => this.toggleMaximize());

    const closeBtn = el('button', 'tok-win-btn tok-win-close', { type: 'button', title: 'סגור', 'aria-label': 'Close' });
    closeBtn.innerHTML = '<svg viewBox="0 0 10 10"><line x1="2" y1="2" x2="8" y2="8" stroke-width="1.2" stroke-linecap="round"/><line x1="8" y1="2" x2="2" y2="8" stroke-width="1.2" stroke-linecap="round"/></svg>';
    closeBtn.addEventListener('click', () => this.closeWindow());

    winControls.appendChild(minBtn);
    winControls.appendChild(maxBtn);
    winControls.appendChild(closeBtn);

    this.element.appendChild(winControls);
  }

  private async minimizeWindow(): Promise<void> {
    const tokIpc = (window as any).tokIpc;
    if (tokIpc?.minimizeWindow) {
      await tokIpc.minimizeWindow();
    }
  }

  private async toggleMaximize(): Promise<void> {
    const tokIpc = (window as any).tokIpc;
    if (tokIpc?.maximizeWindow) {
      await tokIpc.maximizeWindow();
    }
  }

  private async closeWindow(): Promise<void> {
    const tokIpc = (window as any).tokIpc;
    if (tokIpc?.closeWindow) {
      await tokIpc.closeWindow();
    } else {
      this.callbacks.onMenuAction('quit-app');
    }
  }

  private toggleQuickMenu(anchorBtn: HTMLElement): void {
    if (this.activeDropdown) {
      this.closeDropdown();
      return;
    }

    const dropdown = el('div', 'tok-quick-menu', { role: 'menu', 'aria-label': t('topBarFileAndMenu') });
    if (i18n.getDirection() === 'rtl') {
      dropdown.style.right = `${Math.max(0, this.element.clientWidth - (anchorBtn.offsetLeft + anchorBtn.offsetWidth))}px`;
    } else {
      dropdown.style.left = `${Math.max(0, anchorBtn.offsetLeft)}px`;
    }

    const rows: HTMLButtonElement[] = [];
    for (const item of MENU_ITEMS) {
      if ('type' in item) {
        dropdown.appendChild(el('div', 'tok-quick-menu-sep', { role: 'separator' }));
        continue;
      }
      const row = el('button', 'tok-quick-menu-item', { type: 'button', role: 'menuitem', tabindex: '-1' });
      row.appendChild(el('span', undefined, undefined, t(item.labelKey)));
      if (item.shortcut) row.appendChild(kbd(item.shortcut));
      row.addEventListener('mouseenter', () => row.focus());
      row.addEventListener('click', () => {
        this.closeDropdown();
        this.callbacks.onMenuAction(item.action);
      });
      rows.push(row);
      dropdown.appendChild(row);
    }

    dropdown.addEventListener('keydown', (e) => {
      const idx = rows.indexOf(document.activeElement as HTMLButtonElement);
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const delta = e.key === 'ArrowDown' ? 1 : -1;
        rows[(idx + delta + rows.length) % rows.length].focus();
      } else if (e.key === 'Home' || e.key === 'End') {
        e.preventDefault();
        rows[e.key === 'Home' ? 0 : rows.length - 1].focus();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        this.closeDropdown();
        anchorBtn.focus();
      } else if (e.key === 'Tab') {
        this.closeDropdown();
      }
    });

    this.element.appendChild(dropdown);
    this.activeDropdown = dropdown;
    anchorBtn.setAttribute('aria-expanded', 'true');
    this.activeAnchor = anchorBtn;
    if (document.activeElement === anchorBtn) rows[0]?.focus();
  }

  private closeDropdown(): void {
    if (this.activeDropdown) {
      this.activeDropdown.remove();
      this.activeDropdown = null;
    }
    if (this.activeAnchor) {
      this.activeAnchor.setAttribute('aria-expanded', 'false');
      this.activeAnchor = null;
    }
  }
}

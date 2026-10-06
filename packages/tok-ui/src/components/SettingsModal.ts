import { t, tf, onOff, i18n, Language } from '../i18n';
import {
  themeManager, ACCENT_PRESETS, CANVAS_TONE_PRESETS, FONT_SCALES, THEME_PALETTES, SYSTEM_THEME,
  ThemePalette, Density
} from '../theme';
import { PluginEngine } from '../plugins/PluginEngine';
import { IconName } from '../icons';
import { el, icon, iconButton, button, kbd, switchRow, segmented, group, selectField } from '../ui';
import { ModalController } from './ModalController';
import { fillAppVersion, getAppVersion } from '../appInfo';

export interface SettingsModalCallbacks {
  onLanguageChange: (lang: Language) => void;
  onClose: () => void;
  pluginEngine: PluginEngine;
  showToast: (msg: string) => void;
  /** Page guides shown on the canvas (margins / baseline grid). */
  getGuides?: () => { margins: boolean; baseline: boolean };
  setGuide?: (guide: 'margins' | 'baseline', visible: boolean) => void;
  getZoom?: () => number;
  setZoom?: (zoom: number) => void;
}

type SettingsTab = 'appearance' | 'accessibility' | 'logs' | 'updates' | 'plugins';

/**
 * Preset names are stored bilingually as "עברית (English)". Show the half that matches
 * the UI language.
 */
export function localizedPresetName(name: string, lang: Language = i18n.getLanguage()): string {
  const m = name.match(/^(.*?)\s*\(([^)]*)\)\s*$/);
  if (!m) return name.trim();
  return (lang === 'en' ? m[2] : m[1]).trim() || name.trim();
}

/** Miniature of the app in a palette's colors (used on the theme cards). */
function appMiniature(p: ThemePalette, width: number): HTMLElement {
  const box = el('span', undefined, { 'aria-hidden': 'true' });
  box.style.cssText = `width:${width}px;height:92px;flex:none;display:flex;flex-direction:column;overflow:hidden;background:${p.appBg}`;
  const bar = el('span');
  bar.style.cssText = `height:11px;flex:none;border-bottom:1px solid ${p.borderSubtle};display:flex;align-items:center;gap:4px;padding:0 6px`;
  const mark = el('span');
  mark.style.cssText = `width:6px;height:6px;border-radius:2px;background:${p.dark ? '#3D6FD6' : '#1E4A9E'}`;
  const line = el('span');
  line.style.cssText = `width:22px;height:3px;border-radius:2px;background:${p.borderStrong}`;
  bar.append(mark, line);
  const bodyRow = el('span');
  bodyRow.style.cssText = 'flex:1;display:flex';
  const part = (css: string) => {
    const s = el('span');
    s.style.cssText = css;
    return s;
  };
  const desk = part(`flex:1;background:${p.canvas};display:flex;align-items:center;justify-content:center`);
  desk.appendChild(part('width:44%;height:72%;background:#FFFEFB;box-shadow:0 1px 3px rgba(0,0,0,.2)'));
  bodyRow.append(
    part(`width:12%;background:${p.appBg};border-left:1px solid ${p.borderSubtle}`),
    part(`width:20%;background:${p.surface1};border-left:1px solid ${p.borderSubtle}`),
    desk,
    part(`width:24%;background:${p.surface1};border-right:1px solid ${p.borderSubtle}`)
  );
  box.append(bar, bodyRow);
  return box;
}

export class SettingsModal {
  public element: HTMLElement;
  private callbacks: SettingsModalCallbacks;
  private activeTab: SettingsTab = 'appearance';
  private isVisible = false;
  private logRetentionDays = 14;
  private modal: ModalController;
  private scrollPanel?: HTMLElement;

  constructor(callbacks: SettingsModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-settings-overlay');

    try {
      const savedRetention = localStorage.getItem('tok_log_retention');
      const parsed = savedRetention ? parseInt(savedRetention, 10) : NaN;
      if (parsed >= 1 && parsed <= 365) this.logRetentionDays = parsed;
    } catch {}

    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
    // Theme changes (also from the palette or the OS) are reflected while open.
    themeManager.onChange(() => {
      if (this.isVisible && (this.activeTab === 'appearance' || this.activeTab === 'accessibility') && this.scrollPanel) {
        this.renderTabContent(this.scrollPanel);
      }
    });
  }

  public show(initialTab?: SettingsTab): void {
    if (initialTab && initialTab !== ('language' as any)) this.activeTab = initialTab;
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

    const card = el('div', 'tok-dialog tok-settings-card');
    card.style.width = '880px';
    card.style.height = '690px';

    // Header
    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, t('settingsTitle')));
    head.appendChild(headText);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', t('aboutClose'), () => this.hide(), { attrs: { 'data-focus-key': 'close' } }));
    card.appendChild(head);

    const body = el('div', 'tok-dialog-body');

    // Content container created first so tab buttons can update it directly
    const content = el('div', 'tok-settings-content');
    const scroll = el('div', 'tok-settings-scroll', { id: 'tok-settings-panel', role: 'tabpanel' });
    this.scrollPanel = scroll;

    // Category navigation (without language tab)
    const nav = el('nav', 'tok-settings-nav', { role: 'tablist', 'aria-orientation': 'vertical', 'aria-label': t('settingsCategories') });
    const tabs: { id: SettingsTab; label: string; icon: IconName }[] = [
      { id: 'appearance', label: t('settingsTabAppearance'), icon: 'appearance' },
      { id: 'accessibility', label: t('settingsTabAccessibility'), icon: 'accessibility' },
      { id: 'plugins', label: t('settingsTabPlugins'), icon: 'plugin' },
      { id: 'updates', label: t('settingsTabUpdates'), icon: 'refresh' },
      { id: 'logs', label: t('settingsTabLogs'), icon: 'file' }
    ];
    const tabButtons: HTMLButtonElement[] = [];
    const tabButtonMap = new Map<HTMLButtonElement, SettingsTab>();

    for (const tab of tabs) {
      const on = this.activeTab === tab.id;
      const b = el('button', 'tok-settings-nav-btn', {
        type: 'button', role: 'tab', 'aria-selected': String(on), tabindex: on ? '0' : '-1',
        'aria-controls': 'tok-settings-panel', 'data-focus-key': `tab-${tab.id}`
      });
      b.appendChild(icon(tab.icon, 17));
      b.appendChild(el('span', undefined, undefined, tab.label));
      b.addEventListener('click', () => {
        if (this.activeTab === tab.id) return;
        this.activeTab = tab.id;
        for (const [btn, tId] of tabButtonMap) {
          const isSelected = tId === tab.id;
          btn.setAttribute('aria-selected', String(isSelected));
          btn.setAttribute('tabindex', isSelected ? '0' : '-1');
        }
        this.renderTabContent(scroll);
      });
      tabButtons.push(b);
      tabButtonMap.set(b, tab.id);
      nav.appendChild(b);
    }
    // Arrow keys move between categories.
    nav.addEventListener('keydown', (e) => {
      if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
      e.preventDefault();
      const idx = tabButtons.indexOf(document.activeElement as HTMLButtonElement);
      const next = tabButtons[(idx + (e.key === 'ArrowDown' ? 1 : -1) + tabButtons.length) % tabButtons.length];
      next.click();
    });
    const versionSpan = el('span', 'tok-settings-version', undefined, 'TypesetOK');
    fillAppVersion(versionSpan, (v) => `TypesetOK · ${tf('welcomeVersion', { v })}`);
    nav.appendChild(versionSpan);
    body.appendChild(nav);

    // Initial tab content render
    this.renderTabContent(scroll);
    content.appendChild(scroll);

    const foot = el('div', 'tok-dialog-foot');
    const note = el('span', 'tok-dialog-note');
    note.appendChild(icon('check', 15));
    note.appendChild(el('span', undefined, undefined, t('settingsAutoSaved')));
    foot.appendChild(note);
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(t('settingsDone'), { className: 'tok-btn tok-btn-primary', attrs: { 'data-focus-key': 'done' }, onClick: () => this.hide() }));
    content.appendChild(foot);

    body.appendChild(content);
    card.appendChild(body);
    this.element.appendChild(card);
  }

  private renderTabContent(scroll: HTMLElement): void {
    scroll.replaceChildren();
    switch (this.activeTab) {
      case 'appearance': this.renderAppearanceTab(scroll); break;
      case 'accessibility': this.renderAccessibilityTab(scroll); break;
      case 'logs': this.renderLogsTab(scroll); break;
      case 'updates': this.renderUpdatesTab(scroll); break;
      case 'plugins': this.renderPluginsTab(scroll); break;
    }
  }

  private themeCard(id: string, name: string, preview: HTMLElement, selected: boolean): HTMLElement {
    const b = el('button', 'tok-radio-card tok-theme-card', { type: 'button', role: 'radio', 'aria-checked': String(selected), 'data-focus-key': `theme-${id}` });
    const pv = el('span', 'tok-theme-preview');
    pv.appendChild(preview);
    b.appendChild(pv);
    const label = el('span', 'tok-theme-card-label');
    label.appendChild(el('span', 'tok-radio-dot', { 'aria-hidden': 'true' }));
    label.appendChild(el('span', undefined, undefined, name));
    b.appendChild(label);
    b.addEventListener('click', () => {
      themeManager.setPalette(id);
      this.callbacks.showToast(`${t('appearanceTheme')}: ${name}`);
    });
    return b;
  }

  // --- Appearance ---
  private renderAppearanceTab(container: HTMLElement): void {
    const settings = themeManager.getSettings();
    const light = THEME_PALETTES.find((p) => p.id === 'light')!;
    const dark = THEME_PALETTES.find((p) => p.id === 'dark')!;

    // Main themes: light / dark / follow the system
    const themes = el('div', 'tok-theme-grid', { role: 'radiogroup', 'aria-label': t('appearanceTheme') });
    themes.appendChild(this.themeCard('light', localizedPresetName(light.name), appMiniature(light, 150), settings.paletteId === 'light'));
    themes.appendChild(this.themeCard('dark', localizedPresetName(dark.name), appMiniature(dark, 150), settings.paletteId === 'dark'));
    const auto = el('span', undefined, { style: 'display:flex' });
    auto.append(appMiniature(light, 75), appMiniature(dark, 75));
    themes.appendChild(this.themeCard(SYSTEM_THEME, t('themeSystem'), auto, settings.paletteId === SYSTEM_THEME));
    container.appendChild(group(t('appearanceTheme'), themes));

    // The additional color themes
    const more = el('div', 'tok-theme-grid', { role: 'radiogroup', 'aria-label': t('appearanceMoreThemes'), style: 'grid-template-columns:repeat(4,minmax(0,1fr))' });
    for (const p of THEME_PALETTES.filter((x) => x.id !== 'light' && x.id !== 'dark')) {
      const cardEl = this.themeCard(p.id, localizedPresetName(p.name), appMiniature(p, 120), settings.paletteId === p.id);
      cardEl.title = p.desc;
      more.appendChild(cardEl);
    }
    container.appendChild(group(t('appearanceMoreThemes'), more));

    // Accent color
    const current = ACCENT_PRESETS.find((a) => a.value.toLowerCase() === settings.accentColor.toLowerCase());
    const swRow = el('div', 'tok-swatches');
    const swatches = el('div', 'tok-swatches', { role: 'radiogroup', 'aria-label': t('appearanceAccentColor') });
    for (const a of ACCENT_PRESETS) {
      const on = a === current;
      const name = localizedPresetName(a.name);
      const b = el('button', 'tok-swatch-btn', { type: 'button', role: 'radio', 'aria-checked': String(on), 'aria-label': name, title: name, 'data-focus-key': `accent-${a.id}` });
      b.style.background = a.value;
      b.style.setProperty('--tok-swatch', a.value);
      b.addEventListener('click', () => {
        themeManager.setAccentColor(a.value);
        this.callbacks.showToast(`${t('appearanceAccentColor')}: ${name}`);
      });
      swatches.appendChild(b);
    }
    swRow.appendChild(swatches);
    swRow.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, current ? localizedPresetName(current.name) : settings.accentColor));
    container.appendChild(group(t('appearanceAccentColor'), swRow, t('appearanceAccentHint')));

    // Desk color & density
    const two = el('div', undefined, { style: 'display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px' });
    const toneSelect = selectField(
      t('appearanceCanvasTone'),
      CANVAS_TONE_PRESETS.map((p) => ({ value: p.value, label: localizedPresetName(p.name) })),
      CANVAS_TONE_PRESETS.some((p) => p.value === settings.canvasTone) ? settings.canvasTone : '',
      (value) => {
        themeManager.setCanvasTone(value);
        const preset = CANVAS_TONE_PRESETS.find((p) => p.value === value);
        if (preset) this.callbacks.showToast(`${t('appearanceCanvasTone')}: ${localizedPresetName(preset.name)}`);
      },
      { 'data-focus-key': 'canvas-tone' }
    );
    two.appendChild(group(t('appearanceCanvasTone'), toneSelect));
    const densities: { id: Density; label: string }[] = [
      { id: 'spacious', label: t('densitySpacious') },
      { id: 'comfortable', label: t('densityComfortable') },
      { id: 'compact', label: t('densityCompact') }
    ];
    two.appendChild(group(t('appearanceDensity'), segmented(t('appearanceDensity'), densities, settings.density, (id) => {
      themeManager.setDensity(id);
      this.callbacks.showToast(`${t('appearanceDensity')}: ${densities.find((d) => d.id === id)?.label}`);
    }, { fill: true, focusPrefix: 'density' })));
    container.appendChild(two);

    // Page view & guides
    const pageViewSection = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:12px;max-width:420px' });
    if (this.callbacks.getZoom && this.callbacks.setZoom) {
      const curZoom = this.callbacks.getZoom();
      const zoomSelect = selectField(
        t('appearanceZoom'),
        [50, 75, 100, 125, 150, 200].map((z) => ({ value: String(z), label: `${z}%` })),
        String(curZoom),
        (v) => {
          const z = parseInt(v, 10);
          if (Number.isFinite(z)) {
            this.callbacks.setZoom!(z);
            this.callbacks.showToast(`${t('appearanceZoom')}: ${z}%`);
          }
        },
        { 'data-focus-key': 'appearance-zoom' }
      );
      pageViewSection.appendChild(group(t('appearanceZoom'), zoomSelect));
    }
    if (this.callbacks.getGuides && this.callbacks.setGuide) {
      const g = this.callbacks.getGuides();
      const guideRows = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:4px' });
      guideRows.appendChild(switchRow(t('appearanceShowMargins'), g.margins, (v) => this.callbacks.setGuide!('margins', v), { focusKey: 'guide-margins' }));
      guideRows.appendChild(switchRow(t('appearanceShowBaseline'), g.baseline, (v) => this.callbacks.setGuide!('baseline', v), { focusKey: 'guide-baseline' }));
      pageViewSection.appendChild(guideRows);
    }
    if (pageViewSection.children.length > 0) {
      container.appendChild(group(t('appearancePageView'), pageViewSection));
    }
  }

  // --- Accessibility ---
  private renderAccessibilityTab(container: HTMLElement): void {
    const settings = themeManager.getSettings();
    container.appendChild(el('h2', undefined, undefined, t('settingsTabAccessibility')));

    const toggles = el('div', 'tok-card', { style: 'display:flex;flex-direction:column;gap:14px' });
    toggles.appendChild(switchRow(t('accessHighContrast'), settings.highContrast, (val) => {
      themeManager.setHighContrast(val);
      this.callbacks.showToast(`${t('accessHighContrast')}: ${onOff(val)}`);
    }, { description: t('accessHighContrastDesc'), focusKey: 'a11y-contrast' }));
    toggles.appendChild(switchRow(t('accessReducedMotion'), settings.reducedMotion, (val) => {
      themeManager.setReducedMotion(val);
      this.callbacks.showToast(`${t('accessReducedMotion')}: ${onOff(val)}`);
    }, { description: t('accessReducedMotionDesc'), focusKey: 'a11y-motion' }));
    toggles.appendChild(switchRow(t('accessEnhancedFocus'), settings.enhancedFocus, (val) => {
      themeManager.setEnhancedFocus(val);
      this.callbacks.showToast(`${t('accessEnhancedFocus')}: ${onOff(val)}`);
    }, { description: t('accessEnhancedFocusDesc'), focusKey: 'a11y-focus' }));
    toggles.appendChild(switchRow(t('accessDyslexicFont'), settings.accessibleFont, (val) => {
      themeManager.setAccessibleFont(val);
      this.callbacks.showToast(`${t('accessDyslexicFont')}: ${onOff(val)}`);
    }, { description: t('accessDyslexicFontDesc'), focusKey: 'a11y-font' }));
    container.appendChild(toggles);

    container.appendChild(group(t('accessFontScale'), segmented(
      t('accessFontScale'),
      FONT_SCALES.map((s) => ({ id: String(s), label: `${s}%` })),
      String(settings.fontScale),
      (id) => {
        themeManager.setFontScale(parseInt(id, 10));
        this.callbacks.showToast(`${t('accessFontScale')}: ${id}%`);
      },
      { focusPrefix: 'scale' }
    ), t('accessFontScaleDesc')));
  }

  // --- Logs & maintenance ---
  private renderLogsTab(container: HTMLElement): void {
    container.appendChild(el('h2', undefined, undefined, t('settingsTabLogs')));

    const options = [3, 7, 14, 30, 60];
    // Keep a stored value that is not one of the presets visible.
    if (!options.includes(this.logRetentionDays)) {
      options.push(this.logRetentionDays);
      options.sort((a, b) => a - b);
    }
    const retention = el('div', 'tok-card tok-card-row');
    retention.appendChild(el('span', undefined, { style: 'font-size:13px' }, t('logsRetentionLabel')));
    const select = selectField(t('logsRetentionLabel'), options.map((o) => ({ value: String(o), label: `${o} ${t('logsRetentionDays')}` })), String(this.logRetentionDays), (value) => {
      this.logRetentionDays = parseInt(value, 10);
      try {
        localStorage.setItem('tok_log_retention', value);
      } catch {}
      const win = window as any;
      if (win.tokIpc && win.tokIpc.setLogRetention) {
        Promise.resolve(win.tokIpc.setLogRetention(this.logRetentionDays)).catch((e: any) => {
          console.warn('[SETTINGS] setLogRetention failed:', e?.message ?? e);
        });
      }
      this.callbacks.showToast(tf('logsRetentionUpdated', { n: this.logRetentionDays }));
    }, { 'data-focus-key': 'log-retention' });
    select.style.width = '160px';
    retention.appendChild(select);
    container.appendChild(retention);

    const actions = el('div', undefined, { style: 'display:flex;gap:10px' });
    actions.appendChild(button(t('logsOpenFolder'), {
      icon: 'folder',
      attrs: { 'data-focus-key': 'logs-folder' },
      onClick: async () => {
        const win = window as any;
        if (win.tokIpc && win.tokIpc.openLogsFolder) {
          try {
            await win.tokIpc.openLogsFolder();
          } catch (e: any) {
            this.callbacks.showToast(`${t('actionFailed')}: ${e?.message ?? e}`);
          }
        } else {
          this.callbacks.showToast(t('desktopOnlyFeature'));
        }
      }
    }));
    actions.appendChild(button(t('logsCleanNow'), {
      icon: 'trash',
      attrs: { 'data-focus-key': 'logs-clean' },
      onClick: async () => {
        const win = window as any;
        if (win.tokIpc && win.tokIpc.cleanOldLogs) {
          try {
            const deleted = await win.tokIpc.cleanOldLogs(this.logRetentionDays);
            this.callbacks.showToast(tf('logsCleanedCount', { n: Number(deleted) || 0 }));
          } catch (e: any) {
            this.callbacks.showToast(`${t('actionFailed')}: ${e?.message ?? e}`);
          }
          if (this.isVisible && this.activeTab === 'logs') this.render();
        } else {
          this.callbacks.showToast(t('logsCleaned'));
        }
      }
    }));
    container.appendChild(actions);

    // Log lines are English/ISO text: lay them out LTR so the "[timestamp] [LEVEL]" brackets stay in order.
    const logBox = el('div', 'tok-log-box', { dir: 'ltr', tabindex: '0', role: 'log', 'aria-label': t('logsRecentTitle') }, t('logsLoading'));
    const win = window as any;
    if (win.tokIpc && win.tokIpc.getRecentLogs) {
      win.tokIpc.getRecentLogs().then((lines: string[]) => {
        logBox.textContent = Array.isArray(lines) && lines.length ? lines.join('\n') : t('logsEmpty');
      }).catch((e: any) => {
        logBox.textContent = `[${t('logsLoadFailed')}: ${e?.message ?? e}]`;
      });
    } else {
      logBox.textContent = `[${new Date().toISOString()}] [INFO] TypesetOK Desktop Publishing Platform.\n[${new Date().toISOString()}] [INFO] Knuth-Plass Hebrew Breaker and Pre-press pipeline active.`;
    }
    container.appendChild(group(t('logsRecentTitle'), logBox));
  }

  // --- Software updates ---
  private renderUpdatesTab(container: HTMLElement): void {
    container.appendChild(el('h2', undefined, undefined, t('settingsTabUpdates')));

    const current = el('div', 'tok-card tok-card-row');
    const info = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:2px' });
    const versionTitle = el('span', undefined, { style: 'font-size:13px;font-weight:600' }, 'TypesetOK');
    fillAppVersion(versionTitle, (v) => `TypesetOK v${v}`);
    info.appendChild(versionTitle);
    info.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, t('updatesCurrentChannel')));
    current.appendChild(info);
    const checkBtn = button(t('updatesCheckNow'), { className: 'tok-btn tok-btn-primary', icon: 'refresh', attrs: { 'data-focus-key': 'check-updates' } });
    current.appendChild(checkBtn);
    container.appendChild(current);

    const resultBox = el('div', 'tok-card', { 'aria-live': 'polite', style: 'font-size:13px;color:var(--tok-text-secondary)' }, t('updatesHint'));
    container.appendChild(resultBox);

    const showLatest = (version?: string) => {
      resultBox.innerHTML = '';
      const line = el('span', undefined, { style: 'display:inline-flex;align-items:center;gap:6px;color:var(--tok-status-success)' });
      line.appendChild(icon('check', 15));
      line.appendChild(el('span', undefined, undefined, version ? `${t('updatesStatusLatest')} (v${version})` : t('updatesStatusLatest')));
      resultBox.appendChild(line);
    };
    const setChecking = (on: boolean) => {
      checkBtn.disabled = on;
      const label = checkBtn.querySelector('span:last-child');
      if (label) label.textContent = on ? t('updatesStatusChecking') : t('updatesCheckNow');
    };

    checkBtn.addEventListener('click', async () => {
      if (checkBtn.disabled) return;
      setChecking(true);
      const win = window as any;
      if (win.tokIpc && win.tokIpc.checkForUpdates) {
        try {
          const res = await win.tokIpc.checkForUpdates();
          if (res && res.error) {
            // The updater resolves (never rejects) on network/HTTP/parse failures.
            resultBox.textContent = `${t('updatesCheckFailed')}: ${res.error}`;
          } else if (res && res.hasUpdate) {
            // Release name/notes come from the GitHub API: text only, never HTML.
            resultBox.innerHTML = '';
            const headline = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px;font-weight:600;color:var(--tok-status-success);margin-bottom:6px' });
            headline.appendChild(icon('sparkle', 15));
            headline.appendChild(el('span', undefined, undefined, `${t('updatesStatusAvailable')} (v${String(res.latestVersion ?? '')})`));
            resultBox.appendChild(headline);
            resultBox.appendChild(el('div', undefined, { style: 'white-space:pre-wrap;max-height:160px;overflow-y:auto;margin-bottom:10px;color:var(--tok-text-primary)' }, String(res.releaseNotes ?? '')));
            resultBox.appendChild(button(t('updatesDownload'), {
              className: 'tok-btn tok-btn-primary tok-btn-sm',
              icon: 'export',
              onClick: () => {
                Promise.resolve(win.tokIpc.openReleaseUrl(res.downloadUrl || res.releaseUrl)).catch((e: any) => {
                  this.callbacks.showToast(`${t('updatesCheckFailed')}: ${e?.message ?? e}`);
                });
              }
            }));
          } else {
            showLatest(typeof res?.currentVersion === 'string' && res.currentVersion ? res.currentVersion : await getAppVersion());
          }
        } catch (err: any) {
          resultBox.textContent = `${t('updatesCheckFailed')}: ${err?.message ?? err}`;
        }
      } else {
        await new Promise((r) => setTimeout(r, 300));
        showLatest(await getAppVersion());
      }
      setChecking(false);
    });
  }

  // --- Plugins ---
  private renderPluginsTab(container: HTMLElement): void {
    container.appendChild(el('h2', undefined, undefined, t('pluginsInstalled')));

    // 1. Safer Environment Banner (if running on a Safer-managed workstation)
    if (this.callbacks.pluginEngine.getIsSaferActive()) {
      const saferBanner = el('div', 'tok-card', {
        style: 'background:rgba(37,99,235,0.12);border-color:rgba(59,130,246,0.35);padding:14px 18px;margin-bottom:14px;display:flex;gap:12px;align-items:flex-start;'
      });
      const iconWrap = el('div', undefined, { style: 'color:var(--tok-accent-primary);margin-top:2px;' });
      iconWrap.appendChild(icon('lock', 20));
      saferBanner.appendChild(iconWrap);
      const textWrap = el('div');
      textWrap.appendChild(el('strong', undefined, { style: 'font-size:13px;color:var(--tok-text-primary);display:block;margin-bottom:4px;' }, t('pluginsSaferActive')));
      textWrap.appendChild(el('p', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);line-height:1.45;' }, t('pluginsSaferActiveDesc')));
      saferBanner.appendChild(textWrap);
      container.appendChild(saferBanner);
    }

    // 2. Safe Mode (מצב סייפר / בטוח) Toggle Card
    const isSafe = this.callbacks.pluginEngine.getIsSafeMode();
    const safeModeCard = el('div', 'tok-card', {
      style: 'padding:14px 18px;margin-bottom:16px;display:flex;justify-content:space-between;align-items:center;background:var(--tok-bg-surface-1);'
    });
    const safeInfo = el('div');
    safeInfo.appendChild(el('strong', undefined, { style: 'font-size:13px;display:block;margin-bottom:4px;' }, t('pluginsSafeMode')));
    safeInfo.appendChild(el('p', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary);' }, t('pluginsSafeModeDesc')));
    safeModeCard.appendChild(safeInfo);

    const safeSwitch = el('button', 'tok-switch', {
      type: 'button',
      role: 'switch',
      'aria-checked': String(isSafe),
      'aria-label': t('pluginsSafeMode'),
      'data-focus-key': 'plugins-safe-mode'
    });
    safeSwitch.addEventListener('click', async () => {
      const nextState = safeSwitch.getAttribute('aria-checked') !== 'true';
      safeSwitch.disabled = true;
      try {
        await this.callbacks.pluginEngine.setSafeMode(nextState);
        safeSwitch.setAttribute('aria-checked', String(nextState));
        this.callbacks.showToast(`${t('pluginsSafeMode')}: ${onOff(nextState)}`);
        if (this.isVisible && this.activeTab === 'plugins') this.render();
      } catch (err: any) {
        this.callbacks.showToast(`${t('actionFailed')}: ${err?.message ?? err}`);
      } finally {
        safeSwitch.disabled = false;
      }
    });
    safeModeCard.appendChild(safeSwitch);
    container.appendChild(safeModeCard);

    // 3. Action Toolbar (Open Folder, Reload, Emergency Disable)
    const actions = el('div', undefined, { style: 'display:flex;gap:10px;margin-bottom:14px;flex-wrap:wrap;' });
    actions.appendChild(button(t('pluginsOpenFolder'), {
      icon: 'folder',
      attrs: { 'data-focus-key': 'plugins-folder' },
      onClick: () => {
        this.callbacks.pluginEngine.openPluginsFolder().catch((e: any) => {
          this.callbacks.showToast(`${t('actionFailed')}: ${e?.message ?? e}`);
        });
      }
    }));
    const reload = button(t('pluginsReload'), { icon: 'refresh', attrs: { 'data-focus-key': 'plugins-reload' } });
    reload.addEventListener('click', async () => {
      reload.disabled = true;
      try {
        await this.callbacks.pluginEngine.loadPlugins();
        this.callbacks.showToast(t('pluginsReloaded'));
      } catch (e: any) {
        this.callbacks.showToast(`${t('actionFailed')}: ${e?.message ?? e}`);
      }
      if (this.isVisible && this.activeTab === 'plugins') this.render();
      else reload.disabled = false;
    });
    actions.appendChild(reload);

    // Emergency Disable All button
    const disableAll = button(t('pluginsDisableAll'), {
      className: 'tok-btn tok-btn-ghost tok-btn-sm',
      onClick: async () => {
        const plugins = this.callbacks.pluginEngine.getPlugins();
        for (const p of plugins) {
          if (p.enabled) {
            await this.callbacks.pluginEngine.togglePlugin(p.id, false);
          }
        }
        this.callbacks.showToast(t('pluginsAllDisabled'));
        if (this.isVisible && this.activeTab === 'plugins') this.render();
      }
    });
    actions.appendChild(disableAll);
    container.appendChild(actions);

    const plugins = this.callbacks.pluginEngine.getPlugins();
    if (plugins.length === 0) {
      const emptyCard = el('div', 'tok-card', {
        style: 'text-align:center;padding:36px 20px;display:flex;flex-direction:column;align-items:center;gap:12px;color:var(--tok-text-secondary)'
      });
      const iconWrap = el('div', undefined, { style: 'color:var(--tok-text-muted);opacity:0.7' });
      iconWrap.appendChild(icon('plugin', 32));
      emptyCard.appendChild(iconWrap);
      emptyCard.appendChild(el('strong', undefined, { style: 'font-size:14px;color:var(--tok-text-primary)' }, t('pluginsNoPlugins')));
      emptyCard.appendChild(el('p', undefined, { style: 'font-size:12px;max-width:420px;line-height:1.5;color:var(--tok-text-muted)' },
        'מערכת התוספים של TypesetOK תומכת בהרחבות מותאמות אישית ב-TypeScript (.ts) ו-JavaScript (.js) עם API רשמי עשיר, ארגז חול מאובטח ותמיכה מלאה בסביבות סייפר (Offline).'
      ));
      emptyCard.appendChild(button(t('pluginsOpenFolder'), {
        className: 'tok-btn tok-btn-primary tok-btn-sm',
        icon: 'folder',
        onClick: () => {
          this.callbacks.pluginEngine.openPluginsFolder().catch((e: any) => {
            this.callbacks.showToast(`${t('actionFailed')}: ${e?.message ?? e}`);
          });
        }
      }));
      container.appendChild(emptyCard);
      return;
    }

    const list = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:10px' });
    for (const plugin of plugins) {
      const card = el('div', 'tok-card tok-plugin-card');
      const main = el('div', 'tok-plugin-main');
      const name = el('div', 'tok-plugin-name');
      name.appendChild(el('span', undefined, undefined, plugin.name));
      name.appendChild(el('span', 'tok-tag', undefined, plugin.sourceType.toUpperCase()));
      name.appendChild(el('span', undefined, { style: 'font-size:12px;font-weight:400;color:var(--tok-text-muted)', dir: 'ltr' }, `v${plugin.version}`));

      if (plugin.safeModeBlocked) {
        name.appendChild(el('span', 'tok-tag tok-tag-warning', { style: 'background:rgba(239,68,68,0.15);color:var(--tok-status-error);' }, t('pluginsBlockedBySafeMode')));
      }

      main.appendChild(name);
      main.appendChild(el('div', 'tok-plugin-desc', undefined, plugin.description));

      // Display declared permissions as tags
      if (plugin.permissions && plugin.permissions.length > 0) {
        const permsWrap = el('div', undefined, { style: 'display:flex;gap:6px;flex-wrap:wrap;margin-top:6px;' });
        for (const perm of plugin.permissions) {
          let label: string = perm;
          if (perm === 'document:read') label = t('pluginsPermDocRead');
          else if (perm === 'document:write') label = t('pluginsPermDocWrite');
          else if (perm === 'ui:commands') label = t('pluginsPermCommands');
          else if (perm === 'ui:notifications') label = t('pluginsPermNotifications');
          else if (perm === 'ui:modals') label = t('pluginsPermModals');
          else if (perm === 'canvas:read' || perm === 'canvas:navigate') label = t('pluginsPermCanvas');
          else if (perm === 'network:fetch') label = t('pluginsPermNetwork');

          permsWrap.appendChild(el('span', 'tok-tag', { style: 'font-size:11px;opacity:0.85;' }, label));
        }
        main.appendChild(permsWrap);
      }

      if (plugin.error) {
        main.appendChild(el('div', 'tok-plugin-error', undefined, `${t('pluginsLoadError')}: ${plugin.error}`));
      }
      card.appendChild(main);

      const isCurrentEnabled = plugin.enabled && !plugin.safeModeBlocked;
      const state = el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, isCurrentEnabled ? t('pluginsEnabled') : t('pluginsDisabled'));
      card.appendChild(state);

      const sw = el('button', 'tok-switch', {
        type: 'button',
        role: 'switch',
        'aria-checked': String(isCurrentEnabled),
        'aria-label': plugin.name,
        disabled: !!plugin.error || plugin.safeModeBlocked,
        'data-focus-key': `plugin-${plugin.id}`
      });
      sw.addEventListener('click', async () => {
        const wanted = sw.getAttribute('aria-checked') !== 'true';
        sw.disabled = true;
        try {
          await this.callbacks.pluginEngine.togglePlugin(plugin.id, wanted);
          sw.setAttribute('aria-checked', String(wanted));
          state.textContent = wanted ? t('pluginsEnabled') : t('pluginsDisabled');
          this.callbacks.showToast(`${plugin.name}: ${onOff(wanted)}`);
        } catch (e: any) {
          this.callbacks.showToast(`${t('actionFailed')}: ${e?.message ?? e}`);
        } finally {
          sw.disabled = false;
        }
      });
      card.appendChild(sw);
      list.appendChild(card);
    }
    container.appendChild(list);
  }
}

/**
 * Theme system.
 *
 * Every color the UI uses is a CSS custom property set here on <html>; components only
 * reference `var(--tok-…)`. A theme is a palette (light or dark, or "system" which follows
 * the OS) plus an accent color. The page itself is always paper-colored, in both themes.
 */

export interface ThemePalette {
  id: string;
  /** Dark palettes use the dark accent shades and a dark color-scheme. */
  dark: boolean;
  /** Accent applied when the palette is picked (an ACCENT_PRESETS id). */
  defaultAccent: string;
  name: string;
  desc: string;
  appBg: string;
  surface1: string;
  surface2: string;
  surfaceHover: string;
  elevated: string;
  canvas: string;
  borderSubtle: string;
  borderStrong: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  segBg: string;
  segActive: string;
  segShadow: string;
  kbdBg: string;
  switchOff: string;
  success: string;
  successSoft: string;
  warning: string;
  warningSoft: string;
  error: string;
  errorSoft: string;
  scrim: string;
  shadowPage: string;
  shadowFloat: string;
  shadowModal: string;
  previewColors: string[];
}

type DarkBase = Pick<ThemePalette, 'appBg' | 'surface1' | 'surface2' | 'surfaceHover' | 'elevated' | 'canvas' | 'borderSubtle'
  | 'borderStrong' | 'textPrimary' | 'textSecondary' | 'textMuted' | 'segBg' | 'segActive' | 'kbdBg' | 'switchOff'>;

/** A dark palette: shared feedback colors and shadows, its own surfaces. */
function darkPalette(id: string, name: string, desc: string, defaultAccent: string, base: DarkBase): ThemePalette {
  return {
    id, name, desc, defaultAccent, dark: true, ...base,
    segShadow: '0 1px 2px rgba(0, 0, 0, 0.4)',
    success: '#5BC48A', successSoft: '#1D3328',
    warning: '#E8B45A', warningSoft: '#3A2E1A',
    error: '#F28B82', errorSoft: '#3D1F1D',
    scrim: 'rgba(0, 0, 0, 0.58)',
    shadowPage: '0 2px 4px rgba(0, 0, 0, 0.4), 0 16px 48px rgba(0, 0, 0, 0.55)',
    shadowFloat: '0 6px 20px rgba(0, 0, 0, 0.45)',
    shadowModal: '0 24px 64px rgba(0, 0, 0, 0.65)',
    previewColors: [base.appBg, base.surface1, base.canvas, base.borderStrong]
  };
}

export const THEME_PALETTES: ThemePalette[] = [
  {
    id: 'light',
    dark: false,
    defaultAccent: 'ink',
    name: 'בהיר (Light)',
    desc: 'גוונים חמימים ושקטים, כמו נייר בבית דפוס',
    appBg: '#F4F3EF',
    surface1: '#FFFFFF',
    surface2: '#F8F7F4',
    surfaceHover: '#EEECE7',
    elevated: '#FFFFFF',
    canvas: '#E4E2DB',
    borderSubtle: '#DDDAD2',
    borderStrong: '#C9C5BB',
    textPrimary: '#1B1A17',
    textSecondary: '#57534B',
    textMuted: '#6B675E',
    segBg: '#EAE8E2',
    segActive: '#FFFFFF',
    segShadow: '0 1px 2px rgba(27, 26, 23, 0.12)',
    kbdBg: '#ECEAE4',
    switchOff: '#8F897D',
    success: '#1D6B45',
    successSoft: '#E6F2EA',
    warning: '#8A5300',
    warningSoft: '#FBF0DC',
    error: '#B42318',
    errorSoft: '#FDECEA',
    scrim: 'rgba(27, 26, 23, 0.42)',
    shadowPage: '0 1px 2px rgba(27, 26, 23, 0.10), 0 12px 32px rgba(40, 32, 16, 0.16)',
    shadowFloat: '0 4px 16px rgba(27, 26, 23, 0.12)',
    shadowModal: '0 24px 64px rgba(27, 26, 23, 0.30)',
    previewColors: ['#F4F3EF', '#FFFFFF', '#E4E2DB', '#1E4A9E']
  },
  {
    id: 'dark',
    dark: true,
    defaultAccent: 'ink',
    name: 'כהה (Dark)',
    desc: 'גרפיט רגוע לעבודה ממושכת, הדף נשאר בצבע נייר',
    appBg: '#18191C',
    surface1: '#1F2024',
    surface2: '#18191C',
    surfaceHover: '#2A2C31',
    elevated: '#26272C',
    canvas: '#111214',
    borderSubtle: '#2E3036',
    borderStrong: '#3D4047',
    textPrimary: '#EDEDEF',
    textSecondary: '#B4B5BC',
    textMuted: '#8E909A',
    segBg: '#141518',
    segActive: '#2C2E34',
    segShadow: '0 1px 2px rgba(0, 0, 0, 0.4)',
    kbdBg: '#2A2C31',
    switchOff: '#5A5D66',
    success: '#5BC48A',
    successSoft: '#1D3328',
    warning: '#E8B45A',
    warningSoft: '#3A2E1A',
    error: '#F28B82',
    errorSoft: '#3D1F1D',
    scrim: 'rgba(0, 0, 0, 0.58)',
    shadowPage: '0 2px 4px rgba(0, 0, 0, 0.4), 0 16px 48px rgba(0, 0, 0, 0.55)',
    shadowFloat: '0 6px 20px rgba(0, 0, 0, 0.45)',
    shadowModal: '0 24px 64px rgba(0, 0, 0, 0.65)',
    previewColors: ['#18191C', '#1F2024', '#111214', '#3D6FD6']
  },
  // The four color themes of the previous UI, rebuilt on the full token set.
  darkPalette('navy', 'כחול לילה (Deep Navy)', 'כחול לילה עמוק עם ניגודיות מלוטשת', 'ink', {
    appBg: '#0B132B', surface1: '#0F172A', surface2: '#0B132B', surfaceHover: '#1E293B', elevated: '#1E293B',
    canvas: '#060B18', borderSubtle: '#1E293B', borderStrong: '#334155',
    textPrimary: '#F8FAFC', textSecondary: '#A3B1C6', textMuted: '#8A9AB0',
    segBg: '#0B132B', segActive: '#1E293B', kbdBg: '#1E293B', switchOff: '#475569'
  }),
  darkPalette('parchment', 'קלף עברי (Warm Parchment)', 'השראת דפוסי וילנא: ספיה, עץ וקלף חמים', 'amber', {
    appBg: '#231F1A', surface1: '#2D2822', surface2: '#231F1A', surfaceHover: '#3C352E', elevated: '#342E27',
    canvas: '#181512', borderSubtle: '#453D35', borderStrong: '#64584C',
    textPrimary: '#FAF5EF', textSecondary: '#D1C7BD', textMuted: '#A99D90',
    segBg: '#1C1915', segActive: '#3C352E', kbdBg: '#3C352E', switchOff: '#6E6255'
  }),
  darkPalette('graphite', 'גרפיט (Graphite)', 'סטודיו מודרני: פחם עמוק עם נגיעות חמות', 'amber', {
    appBg: '#18181B', surface1: '#27272A', surface2: '#18181B', surfaceHover: '#3F3F46', elevated: '#2D2D32',
    canvas: '#09090B', borderSubtle: '#2E2E33', borderStrong: '#52525B',
    textPrimary: '#FAFAFA', textSecondary: '#B4B4BB', textMuted: '#93939B',
    segBg: '#18181B', segActive: '#3F3F46', kbdBg: '#3F3F46', switchOff: '#5B5B63'
  }),
  darkPalette('indigo', 'אינדיגו (Royal Indigo)', 'אינדיגו עמוק עם נגיעות ירוקות', 'green', {
    appBg: '#1E1B4B', surface1: '#2E285F', surface2: '#1E1B4B', surfaceHover: '#3B347A', elevated: '#352D70',
    canvas: '#0F0D2B', borderSubtle: '#3730A3', borderStrong: '#6366F1',
    textPrimary: '#EEF2FF', textSecondary: '#C7D2FE', textMuted: '#A5B4FC',
    segBg: '#1E1B4B', segActive: '#3B347A', kbdBg: '#3B347A', switchOff: '#5B57A6'
  })
];

/** 'system' follows the operating system's light/dark preference. */
export type ThemeMode = string;
export const SYSTEM_THEME = 'system';
export const THEME_MODES: ThemeMode[] = [...THEME_PALETTES.map((p) => p.id), SYSTEM_THEME];

export type Density = 'spacious' | 'comfortable' | 'compact';
export const DENSITIES: Density[] = ['spacious', 'comfortable', 'compact'];

export interface ThemeSettings {
  paletteId: ThemeMode;
  accentColor: string;
  /** Optional override of the desk color behind the pages ('' = the palette's). */
  canvasTone: string;
  density: Density;
  // Accessibility
  highContrast: boolean;
  fontScale: number; // 100, 110, 120, 130
  reducedMotion: boolean;
  enhancedFocus: boolean;
  accessibleFont: boolean;
}

interface AccentShades {
  /** Button / selection fill (white text on it passes 4.5:1). */
  fill: string;
  hover: string;
  /** Accent used as text or a thin line on the panel color. */
  text: string;
  /** Tinted background for selected rows and chips. */
  soft: string;
}

export interface AccentPreset {
  id: string;
  name: string;
  /** Persisted value (the light-theme fill). */
  value: string;
  hover: string;
  light: AccentShades;
  dark: AccentShades;
}

export const ACCENT_PRESETS: AccentPreset[] = [
  {
    id: 'ink', name: 'כחול דיו (Ink Blue)', value: '#1E4A9E', hover: '#163A7D',
    light: { fill: '#1E4A9E', hover: '#163A7D', text: '#1E4A9E', soft: '#E8EEF8' },
    dark: { fill: '#3D6FD6', hover: '#4F7FE0', text: '#9DBDFF', soft: '#243149' }
  },
  {
    id: 'indigo', name: 'אינדיגו (Indigo)', value: '#4338CA', hover: '#3730A3',
    light: { fill: '#4338CA', hover: '#3730A3', text: '#4338CA', soft: '#EEEDFB' },
    dark: { fill: '#5B50E0', hover: '#6B61E8', text: '#B4AEFC', soft: '#2B2A4F' }
  },
  {
    id: 'teal', name: 'טורקיז (Teal)', value: '#0E7490', hover: '#0B5E75',
    light: { fill: '#0E7490', hover: '#0B5E75', text: '#0E7490', soft: '#E3F3F6' },
    dark: { fill: '#0B7A90', hover: '#0E8AA2', text: '#67D3E6', soft: '#173539' }
  },
  {
    id: 'green', name: 'ירוק (Green)', value: '#15803D', hover: '#116A32',
    light: { fill: '#15803D', hover: '#116A32', text: '#15803D', soft: '#E6F4EA' },
    dark: { fill: '#1A7A3E', hover: '#1F8A47', text: '#6FD394', soft: '#1A3324' }
  },
  {
    id: 'violet', name: 'סגול (Violet)', value: '#6D28D9', hover: '#5B21B6',
    light: { fill: '#6D28D9', hover: '#5B21B6', text: '#6D28D9', soft: '#F1EAFD' },
    dark: { fill: '#7C3AED', hover: '#8B5CF6', text: '#C4B5FD', soft: '#2E2347' }
  },
  {
    id: 'burgundy', name: 'בורדו (Burgundy)', value: '#9F1239', hover: '#82102F',
    light: { fill: '#9F1239', hover: '#82102F', text: '#9F1239', soft: '#FBE8ED' },
    dark: { fill: '#B5174A', hover: '#C42257', text: '#F7A1B8', soft: '#3D1A24' }
  },
  {
    id: 'amber', name: 'ענבר (Amber)', value: '#B45309', hover: '#934407',
    light: { fill: '#B45309', hover: '#934407', text: '#9A4708', soft: '#FBEFDF' },
    dark: { fill: '#A44C08', hover: '#B45309', text: '#F2B66B', soft: '#3A2A17' }
  }
];

/** Desk color behind the pages. '' follows the theme. */
export const CANVAS_TONE_PRESETS = [
  { id: 'theme', name: 'לפי ערכת הנושא (Theme default)', value: '' },
  { id: 'stone', name: 'אבן בהירה (Light Stone)', value: '#E4E2DB' },
  { id: 'gray', name: 'אפור (Gray)', value: '#BDBAB2' },
  { id: 'navy', name: 'כחול לילה (Deep Navy)', value: '#0B132B' },
  { id: 'charcoal', name: 'פחם (Charcoal)', value: '#121212' },
  { id: 'zinc', name: 'אבץ כהה (Dark Zinc)', value: '#18181B' },
];

export const FONT_SCALES = [100, 110, 120, 130];

/** Bumped when the stored format changes; older entries keep only their accessibility choices. */
export const THEME_SETTINGS_VERSION = 2;

const CSS_COLOR = /^#[0-9a-fA-F]{3,8}$/;

export const DEFAULT_THEME_SETTINGS: ThemeSettings = {
  paletteId: 'light',
  accentColor: ACCENT_PRESETS[0].value,
  canvasTone: '',
  density: 'comfortable',
  highContrast: false,
  fontScale: 100,
  reducedMotion: false,
  enhancedFocus: false,
  accessibleFont: false,
};

/**
 * Merges persisted settings over the defaults field by field, dropping anything of the
 * wrong type or out of range (e.g. fontScale "abc", an unknown density, a color string
 * containing CSS).
 */
export function sanitizeThemeSettings(raw: unknown, defaults: ThemeSettings): ThemeSettings {
  const out: ThemeSettings = { ...defaults };
  if (!raw || typeof raw !== 'object') return out;
  const r = raw as Record<string, unknown>;
  if (typeof r.paletteId === 'string' && (THEME_MODES as string[]).includes(r.paletteId)) out.paletteId = r.paletteId as ThemeMode;
  if (typeof r.accentColor === 'string' && CSS_COLOR.test(r.accentColor)) out.accentColor = r.accentColor;
  if (typeof r.canvasTone === 'string' && (r.canvasTone === '' || CSS_COLOR.test(r.canvasTone))) out.canvasTone = r.canvasTone;
  if (typeof r.density === 'string' && (DENSITIES as string[]).includes(r.density)) out.density = r.density as Density;
  if (typeof r.fontScale === 'number' && FONT_SCALES.includes(r.fontScale)) out.fontScale = r.fontScale;
  for (const key of ['highContrast', 'reducedMotion', 'enhancedFocus', 'accessibleFont'] as const) {
    if (typeof r[key] === 'boolean') out[key] = r[key] as boolean;
  }
  return out;
}

/** Settings migration: preserve user theme choices and accessibility preferences safely. */
function migrateThemeSettings(raw: unknown): unknown {
  if (!raw || typeof raw !== 'object') return raw;
  return raw;
}

function systemPrefersDark(): boolean {
  try {
    return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      && window.matchMedia('(prefers-color-scheme: dark)').matches;
  } catch {
    return false;
  }
}

export function findAccentPreset(value: string): AccentPreset | undefined {
  return ACCENT_PRESETS.find((p) => p.value.toLowerCase() === value.toLowerCase());
}

export class ThemeManager {
  private settings: ThemeSettings;
  private listeners: ((settings: ThemeSettings) => void)[] = [];

  constructor() {
    this.settings = { ...DEFAULT_THEME_SETTINGS };

    try {
      const saved = localStorage.getItem('tok_theme_settings');
      if (saved) {
        this.settings = sanitizeThemeSettings(migrateThemeSettings(JSON.parse(saved)), this.settings);
      }
    } catch {}

    // "system" mode follows OS theme changes while the app is open.
    try {
      if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
        const mq = window.matchMedia('(prefers-color-scheme: dark)');
        mq.addEventListener?.('change', () => {
          if (this.settings.paletteId === SYSTEM_THEME) {
            this.applyTheme();
            for (const l of this.listeners) l({ ...this.settings });
          }
        });
      }
    } catch {}

    this.applyTheme();
  }

  public getSettings(): ThemeSettings {
    return { ...this.settings };
  }

  /** The palette actually on screen ('system' resolved). */
  public getResolvedPalette(): ThemePalette {
    const id = this.settings.paletteId === SYSTEM_THEME ? (systemPrefersDark() ? 'dark' : 'light') : this.settings.paletteId;
    return THEME_PALETTES.find((p) => p.id === id) || THEME_PALETTES[0];
  }

  public setAccentColor(color: string): void {
    this.settings.accentColor = color;
    this.saveAndApply();
  }

  public setCanvasTone(tone: string): void {
    this.settings.canvasTone = tone;
    this.saveAndApply();
  }

  public setDensity(density: Density): void {
    this.settings.density = density;
    this.saveAndApply();
  }

  public setHighContrast(enabled: boolean): void {
    this.settings.highContrast = enabled;
    this.saveAndApply();
  }

  public setFontScale(scale: number): void {
    this.settings.fontScale = scale;
    this.saveAndApply();
  }

  public setReducedMotion(enabled: boolean): void {
    this.settings.reducedMotion = enabled;
    this.saveAndApply();
  }

  public setEnhancedFocus(enabled: boolean): void {
    this.settings.enhancedFocus = enabled;
    this.saveAndApply();
  }

  public setAccessibleFont(enabled: boolean): void {
    this.settings.accessibleFont = enabled;
    this.saveAndApply();
  }

  public setPalette(paletteId: ThemeMode): void {
    if (!THEME_MODES.includes(paletteId)) return;
    this.settings.paletteId = paletteId;
    // As before: a named color theme brings its own accent.
    const palette = THEME_PALETTES.find((p) => p.id === paletteId);
    const accent = palette && ACCENT_PRESETS.find((a) => a.id === palette.defaultAccent);
    if (palette && accent && palette.id !== 'light' && palette.id !== 'dark') this.settings.accentColor = accent.value;
    this.saveAndApply();
  }

  private saveAndApply(): void {
    try {
      localStorage.setItem('tok_theme_settings', JSON.stringify({ ...this.settings, version: THEME_SETTINGS_VERSION }));
    } catch {}
    this.applyTheme();
    for (const l of this.listeners) l({ ...this.settings });
  }

  public applyTheme(): void {
    const root = document.documentElement;
    const set = (name: string, value: string) => root.style.setProperty(name, value);
    const palette = this.getResolvedPalette();
    const isDark = palette.dark;
    const accent = findAccentPreset(this.settings.accentColor);
    const shades: AccentShades = accent
      ? (isDark ? accent.dark : accent.light)
      : { fill: this.settings.accentColor, hover: this.settings.accentColor, text: this.settings.accentColor, soft: 'transparent' };

    if (this.settings.highContrast) {
      set('--tok-bg-app', '#000000');
      set('--tok-bg-surface-1', '#050505');
      set('--tok-bg-surface-2', '#000000');
      set('--tok-bg-surface-hover', '#222222');
      set('--tok-bg-elevated', '#0A0A0A');
      set('--tok-bg-canvas', '#000000');
      set('--tok-border-subtle', '#8A8A8A');
      set('--tok-border-strong', '#FFFFFF');
      set('--tok-text-primary', '#FFFFFF');
      set('--tok-text-secondary', '#F0F0F0');
      set('--tok-text-muted', '#E0E0E0');
      set('--tok-accent-primary', '#FFE600');
      set('--tok-accent-hover', '#FFF27A');
      set('--tok-accent-text', '#FFE600');
      set('--tok-accent-soft', '#3A3500');
      set('--tok-on-accent', '#000000');
      set('--tok-border-focus', '#FFE600');
      set('--tok-seg-bg', '#000000');
      set('--tok-seg-active', '#333333');
      set('--tok-kbd-bg', '#222222');
      set('--tok-switch-off', '#8A8A8A');
      set('--tok-surface-2', '#000000');
      set('--tok-text', '#FFFFFF');
      root.dataset.theme = 'dark';
      root.style.colorScheme = 'dark';
    } else {
      set('--tok-bg-app', palette.appBg);
      set('--tok-bg-surface-1', palette.surface1);
      set('--tok-bg-surface-2', palette.surface2);
      set('--tok-bg-surface-hover', palette.surfaceHover);
      set('--tok-bg-elevated', palette.elevated);
      set('--tok-bg-canvas', this.settings.canvasTone || palette.canvas);
      set('--tok-border-subtle', palette.borderSubtle);
      set('--tok-border-strong', palette.borderStrong);
      set('--tok-text-primary', palette.textPrimary);
      set('--tok-text-secondary', palette.textSecondary);
      set('--tok-text-muted', palette.textMuted);
      set('--tok-accent-primary', shades.fill);
      set('--tok-accent-hover', shades.hover);
      set('--tok-accent-text', shades.text);
      set('--tok-accent-soft', shades.soft);
      set('--tok-on-accent', '#FFFFFF');
      set('--tok-border-focus', shades.text);
      set('--tok-seg-bg', palette.segBg);
      set('--tok-seg-active', palette.segActive);
      set('--tok-surface-2', palette.surface2);
      set('--tok-text', palette.textPrimary);
      set('--tok-kbd-bg', palette.kbdBg);
      set('--tok-switch-off', palette.switchOff);
      root.dataset.theme = isDark ? 'dark' : 'light';
      root.style.colorScheme = isDark ? 'dark' : 'light';
    }

    set('--tok-seg-shadow', palette.segShadow);
    set('--tok-selection-frame', shades.text);
    set('--tok-status-success', palette.success);
    set('--tok-status-warning', palette.warning);
    set('--tok-status-error', palette.error);
    set('--tok-success-soft', palette.successSoft);
    set('--tok-warning-soft', palette.warningSoft);
    set('--tok-error-soft', palette.errorSoft);
    set('--tok-scrim', palette.scrim);
    set('--tok-shadow-page', palette.shadowPage);
    set('--tok-shadow-float', palette.shadowFloat);
    set('--tok-shadow-modal', palette.shadowModal);

    // Font scaling
    const baseFontSize = (13 * (this.settings.fontScale / 100)).toFixed(1);
    root.style.fontSize = `${baseFontSize}px`;

    // Density: control heights and panel widths
    const density = {
      spacious: { top: '56px', input: '36px', panel: '256px', inspector: '316px', row: '40px' },
      comfortable: { top: '52px', input: '32px', panel: '236px', inspector: '300px', row: '36px' },
      compact: { top: '44px', input: '28px', panel: '216px', inspector: '280px', row: '30px' },
    }[this.settings.density];
    set('--tok-top-bar-height', density.top);
    set('--tok-titlebar-height', density.top);
    set('--tok-input-height', density.input);
    set('--tok-structure-width', density.panel);
    set('--tok-inspector-width', density.inspector);
    set('--tok-row-height', density.row);
    root.dataset.density = this.settings.density;

    root.classList[this.settings.reducedMotion ? 'add' : 'remove']('tok-reduced-motion');
    root.classList[this.settings.enhancedFocus ? 'add' : 'remove']('tok-enhanced-focus');

    // Assistant is a highly legible Hebrew UI face when installed; IBM Plex is bundled.
    if (this.settings.accessibleFont) {
      set('--tok-font-system', "Assistant, 'IBM Plex Sans Hebrew', 'Segoe UI', Arial, sans-serif");
    } else {
      root.style.removeProperty('--tok-font-system');
    }
  }

  public onChange(listener: (settings: ThemeSettings) => void): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter(l => l !== listener);
    };
  }
}

export const themeManager = new ThemeManager();

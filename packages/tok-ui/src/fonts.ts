/**
 * Document fonts that ship with TypesetOK or are available on the system.
 * The UI supports built-in fonts, local operating system fonts (via Local Font Access API),
 * and custom dynamically loaded fonts (.ttf, .otf, .woff, .woff2).
 */

export interface FontEntry {
  family: string;
  label: string;
  isCustom?: boolean;
}

/**
 * Embedded document fonts that ship with TypesetOK.
 * The UI loads them with @font-face (index.html) and the Rust engine embeds the same glyphs (tok-typeset font.rs).
 */
export const DOCUMENT_FONTS: FontEntry[] = [
  { family: 'Frank Ruhl Libre', label: 'פרנק רוהל (Frank Ruhl Libre)' },
  { family: 'Noto Rashi Hebrew', label: 'כתב רש"י (Noto Rashi Hebrew)' },
  { family: 'David Libre', label: 'דוד (David Libre)' },
  { family: 'Noto Serif Hebrew', label: 'נוטו סריף, טעמים (Noto Serif Hebrew)' },
];

export const DEFAULT_DOCUMENT_FONT = DOCUMENT_FONTS[0].family;

/** Same values as ParagraphStyle.font_weight in tok-core. */
export const FONT_WEIGHT_REGULAR = 400;
export const FONT_WEIGHT_BOLD = 700;

/** Common system fonts available across platforms */
export const EXTENDED_SYSTEM_FONTS: FontEntry[] = [
  { family: 'Arial', label: 'אריאל (Arial)' },
  { family: 'Times New Roman', label: 'טיימס ניו רומן (Times New Roman)' },
  { family: 'Segoe UI', label: 'סגו יו איי (Segoe UI)' },
  { family: 'Tahoma', label: 'טהומה (Tahoma)' },
  { family: 'Calibri', label: 'קליברי (Calibri)' },
  { family: 'Georgia', label: 'ג׳ורג׳יה (Georgia)' },
  { family: 'David', label: 'דוד מערכת (David)' },
  { family: 'Miriam', label: 'מרים (Miriam)' },
  { family: 'Narkisim', label: 'נרקיסים (Narkisim)' },
  { family: 'Hadassah Friedlaender', label: 'הדסה (Hadassah Friedlaender)' },
];

/** Live registered custom and discovered fonts */
const customFontsList: FontEntry[] = [];
let localFontsDiscovered = false;

/**
 * Returns all currently available fonts (embedded, common system fonts, discovered, and custom loaded).
 */
export function getAvailableFonts(): FontEntry[] {
  const map = new Map<string, FontEntry>();
  for (const f of DOCUMENT_FONTS) map.set(f.family, f);
  for (const f of EXTENDED_SYSTEM_FONTS) map.set(f.family, f);
  for (const f of customFontsList) map.set(f.family, f);
  return Array.from(map.values());
}

/**
 * Registers an arbitrary font family by name (e.g. system font typed by the user).
 */
export function registerFontFamily(family: string, label?: string): FontEntry {
  const trimmed = family.trim();
  if (!trimmed) return DOCUMENT_FONTS[0];
  const existing = getAvailableFonts().find((f) => f.family.toLowerCase() === trimmed.toLowerCase());
  if (existing) return existing;

  const entry: FontEntry = {
    family: trimmed,
    label: label || `${trimmed} (מותאם אישית)`,
    isCustom: true
  };
  customFontsList.push(entry);
  return entry;
}

/**
 * Loads a custom font file (.ttf, .otf, .woff, .woff2) into the document session.
 */
export async function loadCustomFontFile(file: File): Promise<FontEntry> {
  const familyName = file.name.replace(/\.[^/.]+$/, '').trim();
  const buffer = await file.arrayBuffer();
  const fontFace = new FontFace(familyName, buffer);
  await fontFace.load();
  document.fonts.add(fontFace);

  return registerFontFamily(familyName, `${familyName} (קובץ גופן)`);
}

/**
 * Attempts to discover local operating system fonts using the Local Font Access API
 * (supported in modern Chromium/Electron).
 */
export async function discoverSystemFonts(): Promise<FontEntry[]> {
  if (localFontsDiscovered) return getAvailableFonts();
  const win = window as any;
  if (typeof win.queryLocalFonts === 'function') {
    try {
      const fonts = await win.queryLocalFonts();
      const seen = new Set<string>();
      for (const f of fonts) {
        if (f.family && !seen.has(f.family)) {
          seen.add(f.family);
          registerFontFamily(f.family, `${f.family} (מערכת)`);
        }
      }
      localFontsDiscovered = true;
    } catch {
      // User dismissed prompt or API unavailable
    }
  }
  return getAvailableFonts();
}

// Mirrors crates/tok-typeset/src/gematria.rs so page numbers in the UI match the engine.

const GERESH = '׳';
const GERSHAYIM = '״';

const TABOO: Record<number, string> = {
  270: 'ער',
  272: 'ערב',
  275: 'ערה',
  298: 'חרצ',
  304: 'דש',
  344: 'שדמ',
  359: 'נטש',
  698: 'תרחצ',
  744: 'תשדמ',
};

const TENS = ['', 'י', 'כ', 'ל', 'מ', 'נ', 'ס', 'ע', 'פ', 'צ'];
const UNITS = ['', 'א', 'ב', 'ג', 'ד', 'ה', 'ו', 'ז', 'ח', 'ט'];

function sub1000(num: number): string {
  if (TABOO[num]) return TABOO[num];
  let s = '';
  while (num >= 400) {
    s += 'ת';
    num -= 400;
  }
  if (num >= 300) { s += 'ש'; num -= 300; }
  else if (num >= 200) { s += 'ר'; num -= 200; }
  else if (num >= 100) { s += 'ק'; num -= 100; }
  if (num === 15) return s + 'טו';
  if (num === 16) return s + 'טז';
  return s + TENS[Math.floor(num / 10)] + UNITS[num % 10];
}

function punctuate(letters: string): string {
  if (letters.length <= 1) return letters ? letters + GERESH : '';
  return letters.slice(0, -1) + GERSHAYIM + letters.slice(-1);
}

/**
 * Hebrew numeral with geresh/gershayim and taboo substitutions:
 * 1 → א׳, 15 → ט״ו, 115 → קט״ו, 270 → ע״ר, 304 → ד״ש, 5784 → ה׳תשפ״ד.
 */
export function toHebrewGematria(num: number, options?: { plain?: boolean }): string {
  if (!Number.isFinite(num) || num < 1) return '';
  const n = Math.floor(num);
  let thousands = Math.floor(n / 1000);
  const rest = n % 1000;
  const plain = Boolean(options?.plain);
  let out = '';
  if (thousands > 0) {
    while (thousands >= 1000) {
      out += 'תתר';
      thousands -= 1000;
    }
    out += sub1000(thousands) + (plain ? '' : GERESH);
  }
  if (rest > 0) {
    const letters = sub1000(rest);
    out += plain ? letters : punctuate(letters);
  }
  return out;
}

/**
 * Formats a Gemara Daf & Amud page number (always starts at Daf ב,
 * Amud Aleph marked with `.` e.g. `ב.`, Amud Bet marked with `:` e.g. `ב:`).
 */
export function formatGemaraDafAmud(pageIndex: number, includeGeresh = false): string {
  const safeIdx = Math.max(0, Math.floor(pageIndex));
  const dafNum = Math.floor(safeIdx / 2) + 2;
  const isAmudBet = safeIdx % 2 === 1;
  const dafStr = toHebrewGematria(dafNum, { plain: !includeGeresh });
  return `${dafStr}${isAmudBet ? ':' : '.'}`;
}

/** Converts a positive integer to Roman numerals. */
export function toRomanNumeral(num: number, upper = true): string {
  if (!Number.isFinite(num) || num < 1) return '';
  let n = Math.floor(num);
  const table: [number, string][] = [
    [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
    [100, 'C'], [90, 'XC'], [50, 'L'], [40, 'XL'],
    [10, 'X'], [9, 'IX'], [5, 'V'], [4, 'IV'], [1, 'I']
  ];
  let res = '';
  for (const [val, sym] of table) {
    while (n >= val) {
      res += sym;
      n -= val;
    }
  }
  return upper ? res : res.toLowerCase();
}

export interface PageNumberFormatInput {
  enabled?: boolean;
  method?: 'none' | 'hebrew-gematria' | 'hebrew-plain' | 'arabic' | 'gemara-daf-amud' | 'roman-upper' | 'roman-lower';
  startNumber?: number;
  prefix?: string;
  suffix?: string;
}

/** Formats a 0-based page index according to the active page numbering settings. */
export function formatPageNumber(pageIndex: number, cfg?: PageNumberFormatInput | null): string {
  if (!cfg || !cfg.enabled || !cfg.method || cfg.method === 'none') return '';
  const start = Number.isFinite(cfg.startNumber) && (cfg.startNumber ?? 0) >= 1
    ? Math.floor(cfg.startNumber!)
    : 1;
  const ordinal = pageIndex + start;
  let core = '';
  switch (cfg.method) {
    case 'hebrew-gematria':
      core = toHebrewGematria(ordinal);
      break;
    case 'hebrew-plain':
      core = toHebrewGematria(ordinal, { plain: true });
      break;
    case 'arabic':
      core = String(ordinal);
      break;
    case 'gemara-daf-amud': {
      // Gemara always starts at Daf ב (startNumber 2 -> offset 0 -> ב.)
      const dafOffset = Math.max(0, pageIndex + Math.max(0, start - 2) * 2);
      core = formatGemaraDafAmud(dafOffset, false);
      break;
    }
    case 'roman-upper':
      core = toRomanNumeral(ordinal, true);
      break;
    case 'roman-lower':
      core = toRomanNumeral(ordinal, false);
      break;
    default:
      return '';
  }
  return `${cfg.prefix ?? ''}${core}${cfg.suffix ?? ''}`;
}

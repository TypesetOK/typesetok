/**
 * TypesetOK (TOK) - Pure Hebrew Typography Utilities for Plugins
 *
 * Implements standard Hebrew string utilities, gematria, niqqud stripping,
 * holy divine name detection, and Canonical Unicode Hebrew mark normalization.
 * Zero external dependencies, O(n) runtime, microsecond latency.
 */

import { toHebrewGematria } from '../gematria';
import { HolyNameMatch, TokHebrewAPI } from './plugin-types';

/** Sacred names recognised by the divine name protection system */
export const HOLY_NAMES: readonly string[] = [
  'יהוה',
  'אלהים',
  'אלוהים',
  'אהיה',
  'שדי',
  'צבאות',
  'אדני'
];

/** Hebrew Niqqud unicode range regex */
const NIQQUD_REGEX = /[\u05B0-\u05C2\u05C4\u05C5\u05C7]/g;

/** Hebrew Ta'amim (cantillation) unicode range regex */
const TAAMIM_REGEX = /[\u0591-\u05AF]/g;

/** All Hebrew combining marks (Niqqud + Ta'amim) */
const ALL_MARKS_REGEX = /[\u0591-\u05BD\u05BF-\u05C2\u05C4\u05C5\u05C7]/g;

/** Hebrew characters range */
const HEBREW_REGEX = /[\u0590-\u05FF]/;

/**
 * Strips Hebrew Niqqud (vowel points) while preserving consonants and cantillation.
 */
export function stripNiqqud(text: string): string {
  if (!text) return '';
  return text.replace(NIQQUD_REGEX, '');
}

/**
 * Strips Hebrew Ta'amim (cantillation marks) while preserving vowels (Niqqud).
 */
export function stripTaamim(text: string): string {
  if (!text) return '';
  return text.replace(TAAMIM_REGEX, '');
}

/**
 * Strips all combining marks (both vowels and cantillation).
 */
export function stripAllMarks(text: string): string {
  if (!text) return '';
  return text.replace(ALL_MARKS_REGEX, '');
}

/**
 * Tests whether a character is a Hebrew combining mark.
 */
function isHebrewMark(ch: string): boolean {
  const code = ch.charCodeAt(0);
  return (code >= 0x0591 && code <= 0x05bd) || code === 0x05bf || (code >= 0x05c1 && code <= 0x05c2) || code === 0x05c4 || code === 0x05c5 || code === 0x05c7;
}

/**
 * Finds all occurrences of holy divine names in pointed or unpointed text.
 * Matches are returned with their precise character offset and length in the original string.
 */
export function findHolyNames(text: string): HolyNameMatch[] {
  if (!text) return [];

  // Build a mapped array of bare letters with original string indices
  const letters: { char: string; index: number }[] = [];
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (!isHebrewMark(ch)) {
      letters.push({ char: ch, index: i });
    }
  }

  const results: HolyNameMatch[] = [];

  for (const name of HOLY_NAMES) {
    const nameChars = Array.from(name);
    const targetLen = nameChars.length;

    for (let i = 0; i <= letters.length - targetLen; i++) {
      let matches = true;
      for (let j = 0; j < targetLen; j++) {
        if (letters[i + j].char !== nameChars[j]) {
          matches = false;
          break;
        }
      }

      if (matches) {
        const startIdx = letters[i].index;
        // End index is either the start of the next letter or the end of the text
        const lastLetterIdx = letters[i + targetLen - 1].index;
        let endIdx = lastLetterIdx + 1;
        while (endIdx < text.length && isHebrewMark(text[endIdx])) {
          endIdx++;
        }

        results.push({
          name,
          offset: startIdx,
          len: endIdx - startIdx
        });
      }
    }
  }

  // Sort by offset, then descending length
  return results.sort((a, b) => a.offset - b.offset || b.len - a.len);
}

export const HEBREW_GERSHAYIM_RE = /([\u0590-\u05FF])"([\u0590-\u05FF])/g;
export const HEBREW_GERESH_RE = /([\u0590-\u05FF])'(?=\s|[,.:;!?׃\-]|$|[^\u0590-\u05FF])/g;

/**
 * Converts ASCII quotes (" and ') inside Hebrew text to proper typographic
 * Gershayim (״, U+05F4) and Geresh (׳, U+05F3).
 */
export function normalizeQuotes(text: string): string {
  if (!text) return '';
  return text
    // Replace ASCII double quote between Hebrew letters with Gershayim (e.g. ר"ת -> ר״ת)
    .replace(HEBREW_GERSHAYIM_RE, '$1״$2')
    // Replace ASCII single quote after Hebrew letter with Geresh (e.g. ע' -> ע׳)
    .replace(HEBREW_GERESH_RE, '$1׳');
}

/**
 * Returns the canonical Hebrew combining mark priority class:
 * 1. Shin/Sin dots (U+05C1, U+05C2)
 * 2. Dagesh/Mapiq (U+05BC) and Rafe (U+05BF)
 * 3. Vowel points / Niqqud (U+05B0..U+05BB, U+05C7, U+05BD, U+05C4, U+05C5)
 * 4. Cantillation marks / Taamim (U+0591..U+05AF)
 */
function hebrewMarkPriority(code: number): number {
  if (code === 0x05c1 || code === 0x05c2) return 10;
  if (code === 0x05bc || code === 0x05bf) return 20;
  if ((code >= 0x05b0 && code <= 0x05bb) || code === 0x05c7) return 30;
  if (code === 0x05bd || code === 0x05c4 || code === 0x05c5) return 35;
  if (code >= 0x0591 && code <= 0x05af) return 40;
  return 50;
}

const HEBREW_MARK_SEQUENCE_REGEX = /[\u0591-\u05BD\u05BF-\u05C2\u05C4\u05C5\u05C7]{2,}/g;

/**
 * Normalizes Hebrew text combining mark ordering according to Canonical Unicode Hebrew Normalization:
 * 1. Shin/Sin dots (U+05C1, U+05C2)
 * 2. Dagesh/Mapiq (U+05BC) and Rafe (U+05BF)
 * 3. Vowel points / Niqqud (U+05B0..U+05BB, U+05BD, U+05C7)
 * 4. Cantillation marks / Taamim (U+0591..U+05AF)
 */
export function normalizeHebrew(text: string): string {
  if (!text) return '';
  // Unicode NFC normalization decomposes singleton Hebrew presentation forms (U+FB1D..U+FB4F),
  // followed by canonical Hebrew combining mark ordering.
  const nfc = text.normalize('NFC');
  return nfc.replace(HEBREW_MARK_SEQUENCE_REGEX, (seq) =>
    Array.from(seq)
      .sort((a, b) => hebrewMarkPriority(a.charCodeAt(0)) - hebrewMarkPriority(b.charCodeAt(0)))
      .join('')
  );
}

/**
 * Checks whether the text contains any Hebrew characters.
 */
export function isHebrew(text: string): boolean {
  return HEBREW_REGEX.test(text);
}

/**
 * Exported implementation of TokHebrewAPI.
 */
export const hebrewUtils: TokHebrewAPI = {
  toHebrewGematria,
  stripNiqqud,
  stripTaamim,
  stripAllMarks,
  normalizeHebrew,
  findHolyNames,
  normalizeQuotes,
  isHebrew
};

/**
 * Smart file sorting & custom ordering utilities for batch document import.
 *
 * Supports natural numeric sorting, Hebrew gematria / chapter-aware sorting,
 * path/date/size sorting, and manifest-based custom ordering for hundreds or
 * thousands of files.
 */

export type BatchSortMode =
  | 'natural'
  | 'hebrew-gematria'
  | 'name-asc'
  | 'name-desc'
  | 'path-natural'
  | 'date-asc'
  | 'date-desc'
  | 'size-asc'
  | 'size-desc'
  | 'custom';

export interface BatchQueuedFile {
  id: string;
  fileName: string;
  filePath?: string;
  relativePath?: string;
  size: number;
  modifiedMs?: number;
  ext: string;
  webFile?: File;
}

const SUPPORTED_EXT_RE = /\.(docx|md|markdown|txt|tag)$/i;

export function isSupportedImportFileName(fileName: string): boolean {
  return SUPPORTED_EXT_RE.test(fileName);
}

export function getFileExtension(fileName: string): string {
  const dot = fileName.lastIndexOf('.');
  return dot >= 0 ? fileName.slice(dot + 1).toLowerCase() : '';
}

const HEBREW_GEMATRIA_MAP: Record<string, number> = {
  'א': 1, 'ב': 2, 'ג': 3, 'ד': 4, 'ה': 5, 'ו': 6, 'ז': 7, 'ח': 8, 'ט': 9,
  'י': 10, 'כ': 20, 'ך': 20, 'ל': 30, 'מ': 40, 'ם': 40, 'נ': 50, 'ן': 50,
  'ס': 60, 'ע': 70, 'פ': 80, 'ף': 80, 'צ': 90, 'ץ': 90,
  'ק': 100, 'ר': 200, 'ש': 300, 'ת': 400
};

/**
 * Parses a Hebrew numeral token (e.g. `א`, `י"א`, `טו`, `קכג`) into a numeric value.
 * Returns `null` if the token is not a valid Hebrew numeral.
 */
export function parseHebrewGematriaToken(raw: string): number | null {
  const cleaned = raw.replace(/['"׳״\-_]/g, '').trim();
  if (!cleaned || cleaned.length > 6) return null;
  let total = 0;
  for (const ch of cleaned) {
    const val = HEBREW_GEMATRIA_MAP[ch];
    if (!val) return null;
    total += val;
  }
  return total > 0 ? total : null;
}

const NATURAL_TOKEN_RE = /(\d+|\D+)/g;

/**
 * Natural numeric comparator (`file 2.docx` < `file 10.docx`).
 */
export function naturalCompare(a: string, b: string): number {
  const tokensA = a.match(NATURAL_TOKEN_RE) || [];
  const tokensB = b.match(NATURAL_TOKEN_RE) || [];
  const len = Math.min(tokensA.length, tokensB.length);

  for (let i = 0; i < len; i++) {
    const ta = tokensA[i];
    const tb = tokensB[i];
    const isNumA = /^\d+$/.test(ta);
    const isNumB = /^\d+$/.test(tb);

    if (isNumA && isNumB) {
      const diff = Number(ta) - Number(tb);
      if (diff !== 0) return diff;
      if (ta.length !== tb.length) return ta.length - tb.length;
    } else if (isNumA !== isNumB) {
      return isNumA ? -1 : 1;
    } else {
      const cmp = ta.localeCompare(tb, ['he', 'en'], { sensitivity: 'base' });
      if (cmp !== 0) return cmp;
    }
  }
  return tokensA.length - tokensB.length;
}

const WORD_OR_NUM_TOKEN_RE = /(\d+|[\u0590-\u05FF'"׳״]+|[^\d\u0590-\u05FF'"׳״]+)/g;
const HEBREW_PREFIX_WORDS = new Set([
  'פרק', 'סימן', 'דף', 'אות', 'חלק', 'מאמר', 'שיעור', 'הלכה', 'סעיף', 'עמוד', 'כרך', 'שער', 'משנה', 'תורה'
]);

/**
 * Smart Hebrew Gematria + natural numeric comparator.
 * Recognizes Hebrew chapter/section numerals (e.g. `פרק ב` < `פרק י` < `פרק יא`)
 * as well as Arabic numerals (`1, 2, 10`).
 */
export function hebrewGematriaCompare(a: string, b: string): number {
  const stripExt = (s: string) => s.replace(/\.[^/.\\]+$/, '');
  const tokensA = stripExt(a).match(WORD_OR_NUM_TOKEN_RE) || [];
  const tokensB = stripExt(b).match(WORD_OR_NUM_TOKEN_RE) || [];
  const len = Math.min(tokensA.length, tokensB.length);

  for (let i = 0; i < len; i++) {
    const ta = tokensA[i];
    const tb = tokensB[i];
    if (ta === tb) continue;

    const numA = /^\d+$/.test(ta)
      ? Number(ta)
      : (!HEBREW_PREFIX_WORDS.has(ta) ? parseHebrewGematriaToken(ta) : null);
    const numB = /^\d+$/.test(tb)
      ? Number(tb)
      : (!HEBREW_PREFIX_WORDS.has(tb) ? parseHebrewGematriaToken(tb) : null);

    if (numA !== null && numB !== null) {
      if (numA !== numB) return numA - numB;
    } else if (numA !== null || numB !== null) {
      // Keep mixed comparisons deterministic
      const cmp = ta.localeCompare(tb, ['he', 'en'], { numeric: true, sensitivity: 'base' });
      if (cmp !== 0) return cmp;
    } else {
      const cmp = ta.localeCompare(tb, ['he', 'en'], { numeric: true, sensitivity: 'base' });
      if (cmp !== 0) return cmp;
    }
  }
  return tokensA.length - tokensB.length || naturalCompare(a, b);
}

/**
 * Returns a new sorted array of `BatchQueuedFile` items according to `mode`.
 */
export function sortBatchFiles<T extends BatchQueuedFile>(files: readonly T[], mode: BatchSortMode): T[] {
  if (mode === 'custom') return [...files];

  const copy = [...files];
  copy.sort((a, b) => {
    switch (mode) {
      case 'natural':
        return naturalCompare(a.fileName, b.fileName);
      case 'hebrew-gematria':
        return hebrewGematriaCompare(a.fileName, b.fileName);
      case 'name-asc':
        return a.fileName.localeCompare(b.fileName, ['he', 'en'], { sensitivity: 'base' });
      case 'name-desc':
        return b.fileName.localeCompare(a.fileName, ['he', 'en'], { sensitivity: 'base' });
      case 'path-natural':
        return naturalCompare(a.relativePath || a.filePath || a.fileName, b.relativePath || b.filePath || b.fileName);
      case 'date-asc':
        return (a.modifiedMs ?? 0) - (b.modifiedMs ?? 0) || naturalCompare(a.fileName, b.fileName);
      case 'date-desc':
        return (b.modifiedMs ?? 0) - (a.modifiedMs ?? 0) || naturalCompare(a.fileName, b.fileName);
      case 'size-asc':
        return a.size - b.size || naturalCompare(a.fileName, b.fileName);
      case 'size-desc':
        return b.size - a.size || naturalCompare(a.fileName, b.fileName);
      default:
        return 0;
    }
  });
  return copy;
}

export interface CustomOrderApplyResult<T extends BatchQueuedFile> {
  ordered: T[];
  matchedCount: number;
  unmatchedLines: string[];
}

/**
 * Applies a user-provided newline-separated order list (manifest) to `files`.
 * Each line is matched against relativePath -> fileName -> clean title -> substring.
 * Unmatched files are appended at the end in their existing relative order.
 */
export function applyCustomOrderList<T extends BatchQueuedFile>(
  files: readonly T[],
  orderText: string
): CustomOrderApplyResult<T> {
  const lines = orderText
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith('#'));

  const used = new Set<number>();
  const ordered: T[] = [];
  const unmatchedLines: string[] = [];

  const stripExt = (s: string) => s.replace(/\.[^/.\\]+$/, '').toLowerCase();

  for (const line of lines) {
    const needle = line.toLowerCase();
    const needleNoExt = stripExt(line);

    let matchIdx = files.findIndex(
      (f, idx) => !used.has(idx) && (f.relativePath?.toLowerCase() === needle || f.fileName.toLowerCase() === needle)
    );
    if (matchIdx < 0) {
      matchIdx = files.findIndex((f, idx) => !used.has(idx) && stripExt(f.fileName) === needleNoExt);
    }
    if (matchIdx < 0) {
      matchIdx = files.findIndex(
        (f, idx) =>
          !used.has(idx) &&
          (f.fileName.toLowerCase().includes(needle) || (f.relativePath?.toLowerCase().includes(needle) ?? false))
      );
    }

    if (matchIdx >= 0) {
      used.add(matchIdx);
      ordered.push(files[matchIdx]);
    } else {
      unmatchedLines.push(line);
    }
  }

  const matchedCount = ordered.length;
  for (let i = 0; i < files.length; i++) {
    if (!used.has(i)) {
      ordered.push(files[i]);
    }
  }

  return { ordered, matchedCount, unmatchedLines };
}

/**
 * Moves an item from `fromIndex` to `toIndex` and returns a new array.
 */
export function reorderBatchItem<T>(items: readonly T[], fromIndex: number, toIndex: number): T[] {
  if (
    fromIndex === toIndex ||
    fromIndex < 0 ||
    toIndex < 0 ||
    fromIndex >= items.length ||
    toIndex >= items.length
  ) {
    return [...items];
  }
  const copy = [...items];
  const [moved] = copy.splice(fromIndex, 1);
  copy.splice(toIndex, 0, moved);
  return copy;
}

/**
 * TypesetOK (TOK) - Professional Hebrew Text Cleanup & Typography Preparation Engine.
 *
 * Implements robust Hebrew text normalization, typographical quotes/geresh conversion,
 * space and punctuation cleanup, and automatic Dibur HaMatchil run-in heading styling.
 */

import type { StoryParagraph, TextSpan, CharacterFormat } from 'tok-story-editor';

function isFormatEmpty(fmt?: CharacterFormat | null): boolean {
  if (!fmt) return true;
  return (
    !fmt.characterStyleId &&
    !fmt.fontFamily &&
    fmt.fontSizePt === undefined &&
    fmt.fontWeight === undefined &&
    !fmt.italic &&
    !fmt.underline &&
    fmt.trackingEm === undefined &&
    !fmt.color &&
    !fmt.linkUrl
  );
}

function areFormatsEqual(a?: CharacterFormat | null, b?: CharacterFormat | null): boolean {
  if (!a && !b) return true;
  if (!a || !b) return false;
  return (
    a.characterStyleId === b.characterStyleId &&
    a.fontFamily === b.fontFamily &&
    a.fontSizePt === b.fontSizePt &&
    a.fontWeight === b.fontWeight &&
    Boolean(a.italic) === Boolean(b.italic) &&
    Boolean(a.underline) === Boolean(b.underline) &&
    a.trackingEm === b.trackingEm &&
    a.color === b.color &&
    a.linkUrl === b.linkUrl
  );
}

export function normalizeSpans(spans: TextSpan[], textLen: number = Infinity): TextSpan[] {
  let out: TextSpan[] = [];
  for (const span of spans) {
    const start = Math.max(0, Math.min(textLen, span.start));
    const end = Math.max(0, Math.min(textLen, span.end));
    if (start >= end || isFormatEmpty(span.format)) continue;

    const next: TextSpan[] = [];
    for (const s of out) {
      if (s.end <= start || s.start >= end) {
        next.push(s);
        continue;
      }
      if (s.start < start) {
        next.push({ start: s.start, end: start, format: { ...s.format } });
      }
      if (s.end > end) {
        next.push({ start: end, end: s.end, format: { ...s.format } });
      }
    }
    next.push({ start, end, format: { ...span.format } });
    out = next;
  }

  out.sort((a, b) => a.start - b.start);
  const merged: TextSpan[] = [];
  for (const s of out) {
    const last = merged[merged.length - 1];
    if (last && last.end === s.start && areFormatsEqual(last.format, s.format)) {
      last.end = s.end;
    } else {
      merged.push({ start: s.start, end: s.end, format: { ...s.format } });
    }
  }
  return merged;
}

export function formatRangeInSpans(
  spans: TextSpan[],
  start: number,
  end: number,
  format: CharacterFormat,
  textLen: number = Infinity
): TextSpan[] {
  if (start >= end || isFormatEmpty(format)) return [...spans];
  const copy = spans.map((s) => ({ ...s, format: { ...s.format } }));
  copy.push({ start, end, format });
  return normalizeSpans(copy, textLen);
}

export interface CleanupOptions {
  cleanDoubleSpaces: boolean;
  cleanSpacesBeforePunct: boolean;
  ensureSpaceAfterPunct: boolean;
  normalizeQuotesAndGeresh: boolean;
  collapseEmptyParagraphs: boolean;
  boldDiburHaMatchil: boolean;
  diburDelimiter: 'any' | 'period' | 'colon' | 'dash';
  scope: 'activeFlow' | 'allFlows';
}

export interface CleanupStats {
  doubleSpacesFixed: number;
  spacesBeforePunctFixed: number;
  spacesAfterPunctFixed: number;
  quotesConverted: number;
  gereshConverted: number;
  emptyParagraphsRemoved: number;
  diburHaMatchilBolded: number;
  totalModifications: number;
}

export const DEFAULT_CLEANUP_OPTIONS: CleanupOptions = {
  cleanDoubleSpaces: true,
  cleanSpacesBeforePunct: true,
  ensureSpaceAfterPunct: true,
  normalizeQuotesAndGeresh: true,
  collapseEmptyParagraphs: true,
  boldDiburHaMatchil: true,
  diburDelimiter: 'any',
  scope: 'activeFlow'
};

/**
 * Finds the index of the Dibur HaMatchil delimiter in a paragraph text.
 * Returns -1 if no delimiter is found within the threshold.
 */
export function findDiburDelimiter(text: string, delimiter: CleanupOptions['diburDelimiter'], maxChars = 140): number {
  if (!text || text.length === 0) return -1;
  const searchLimit = Math.min(text.length, maxChars);

  let bestIdx = -1;

  for (let i = 0; i < searchLimit; i++) {
    const ch = text[i];

    if (delimiter === 'period' || delimiter === 'any') {
      if (ch === '.') {
        bestIdx = i;
        break;
      }
    }

    if (delimiter === 'colon' || delimiter === 'any') {
      if (ch === ':' || ch === '׃') {
        bestIdx = i;
        break;
      }
    }

    if (delimiter === 'dash' || delimiter === 'any') {
      if (ch === '–' || ch === '—' || ch === '-') {
        // Ensure it's not a hyphen inside a compound word (requires following space or trailing)
        if (ch === '-' && i + 1 < text.length && text[i + 1] !== ' ') {
          // might be hyphenated word like בית-אל, skip
        } else {
          bestIdx = i;
          break;
        }
      }
    }
  }

  return bestIdx;
}


/**
 * Replaces patterns in text while maintaining accurate TextSpan offset tracking.
 */
function applyReplacementWithSpans(
  text: string,
  regex: RegExp,
  replacer: (match: string, ...args: any[]) => string,
  spans: TextSpan[]
): { newText: string; newSpans: TextSpan[]; count: number } {
  let count = 0;
  let resultText = '';
  let lastIndex = 0;
  let currentSpans = spans ? spans.map((s) => ({ ...s, format: { ...s.format } })) : [];

  const matches: { index: number; matchLen: number; replacement: string }[] = [];

  let m: RegExpExecArray | null;
  // Ensure global flag is set
  const re = new RegExp(regex.source, regex.flags.includes('g') ? regex.flags : regex.flags + 'g');

  while ((m = re.exec(text)) !== null) {
    const matchStr = m[0];
    const matchIdx = m.index;
    const repStr = replacer(matchStr, ...m.slice(1));
    matches.push({ index: matchIdx, matchLen: matchStr.length, replacement: repStr });
    count++;
  }

  if (matches.length === 0) {
    return { newText: text, newSpans: spans || [], count: 0 };
  }

  // Apply replacements from left to right and adjust span coordinates
  let offsetDelta = 0;
  for (const item of matches) {
    resultText += text.slice(lastIndex, item.index) + item.replacement;
    lastIndex = item.index + item.matchLen;

    const delta = item.replacement.length - item.matchLen;
    const cutStart = item.index + offsetDelta;
    const cutEnd = cutStart + item.matchLen;

    if (delta !== 0) {
      currentSpans = currentSpans
        .map((s) => {
          if (s.end <= cutStart) {
            return s;
          } else if (s.start >= cutEnd) {
            return { ...s, start: s.start + delta, end: s.end + delta };
          } else {
            // Span overlaps the replacement point
            const newStart = Math.min(s.start, cutStart);
            const newEnd = Math.max(newStart, s.end + delta);
            return { ...s, start: newStart, end: newEnd };
          }
        })
        .filter((s) => s.start < s.end);
      offsetDelta += delta;
    }
  }

  resultText += text.slice(lastIndex);
  return {
    newText: resultText,
    newSpans: normalizeSpans(currentSpans, resultText.length),
    count
  };
}


/**
 * Analyzes paragraphs and computes how many cleanup fixes and heading bolds would be made.
 */
export function analyzeParagraphs(paragraphs: StoryParagraph[], options: CleanupOptions): CleanupStats {
  const stats: CleanupStats = {
    doubleSpacesFixed: 0,
    spacesBeforePunctFixed: 0,
    spacesAfterPunctFixed: 0,
    quotesConverted: 0,
    gereshConverted: 0,
    emptyParagraphsRemoved: 0,
    diburHaMatchilBolded: 0,
    totalModifications: 0
  };

  let prevEmpty = false;

  for (const p of paragraphs) {
    const text = p.text || '';
    const isEmpty = text.trim().length === 0;

    if (isEmpty) {
      if (options.collapseEmptyParagraphs && prevEmpty) {
        stats.emptyParagraphsRemoved++;
      }
      prevEmpty = true;
      continue;
    }
    prevEmpty = false;

    if (options.cleanDoubleSpaces) {
      const m = text.match(/[ \t]{2,}/g);
      if (m) stats.doubleSpacesFixed += m.length;
    }

    if (options.cleanSpacesBeforePunct) {
      const m = text.match(/[ \t]+([,.:;!?׃])/g);
      if (m) stats.spacesBeforePunctFixed += m.length;
    }

    if (options.ensureSpaceAfterPunct) {
      const m1 = text.match(/([,.;!?׃])([\u0590-\u05FF])/g);
      if (m1) stats.spacesAfterPunctFixed += m1.length;
      const m2 = text.match(/([:])([\u0590-\u05FF]{2,})/g);
      if (m2) stats.spacesAfterPunctFixed += m2.length;
    }

    if (options.normalizeQuotesAndGeresh) {
      const qm = text.match(/([\u0590-\u05FF])"([\u0590-\u05FF])/g);
      if (qm) stats.quotesConverted += qm.length;

      const gm = text.match(/([\u0590-\u05FF])'(?=\s|[,.:;!?׃\-]|$|[^\u0590-\u05FF])/g);
      if (gm) stats.gereshConverted += gm.length;
    }

    if (options.boldDiburHaMatchil) {
      const delimIdx = findDiburDelimiter(text, options.diburDelimiter);
      if (delimIdx > 0) {
        // Check if not already bold
        const alreadyBold = (p.spans || []).some((s) => s.start === 0 && s.end >= delimIdx + 1 && s.format.fontWeight === 700);
        if (!alreadyBold) {
          stats.diburHaMatchilBolded++;
        }
      }
    }
  }

  stats.totalModifications =
    stats.doubleSpacesFixed +
    stats.spacesBeforePunctFixed +
    stats.spacesAfterPunctFixed +
    stats.quotesConverted +
    stats.gereshConverted +
    stats.emptyParagraphsRemoved +
    stats.diburHaMatchilBolded;

  return stats;
}


/**
 * Cleans a list of paragraphs according to the specified cleanup options.
 * Modifies text, recalculates spans, and automatically bolds Dibur HaMatchil.
 */
export function cleanupParagraphs(
  paragraphs: StoryParagraph[],
  options: CleanupOptions
): { paragraphs: StoryParagraph[]; stats: CleanupStats } {
  const stats: CleanupStats = {
    doubleSpacesFixed: 0,
    spacesBeforePunctFixed: 0,
    spacesAfterPunctFixed: 0,
    quotesConverted: 0,
    gereshConverted: 0,
    emptyParagraphsRemoved: 0,
    diburHaMatchilBolded: 0,
    totalModifications: 0
  };

  const result: StoryParagraph[] = [];
  let prevEmpty = false;

  for (const original of paragraphs) {
    const rawText = original.text || '';
    const isBlank = rawText.trim().length === 0;

    if (isBlank) {
      if (options.collapseEmptyParagraphs && prevEmpty) {
        stats.emptyParagraphsRemoved++;
        continue;
      }
      prevEmpty = true;
      result.push({
        id: original.id,
        styleId: original.styleId,
        text: '',
        spans: [],
        patch: original.patch ? { ...original.patch } : undefined
      });
      continue;
    }
    prevEmpty = false;

    let curText = rawText;
    let curSpans = original.spans ? original.spans.map((s) => ({ ...s, format: { ...s.format } })) : [];

    // 1. Clean double spaces
    if (options.cleanDoubleSpaces) {
      const r = applyReplacementWithSpans(curText, /[ \t]{2,}/g, () => ' ', curSpans);
      curText = r.newText;
      curSpans = r.newSpans;
      stats.doubleSpacesFixed += r.count;
    }

    // 2. Clean spaces before punctuation
    if (options.cleanSpacesBeforePunct) {
      const r = applyReplacementWithSpans(curText, /[ \t]+([,.:;!?׃])/g, (_m, p1) => p1, curSpans);
      curText = r.newText;
      curSpans = r.newSpans;
      stats.spacesBeforePunctFixed += r.count;
    }

    // 3. Ensure space after punctuation when directly touching Hebrew letters
    if (options.ensureSpaceAfterPunct) {
      const r1 = applyReplacementWithSpans(curText, /([,.;!?׃])([\u0590-\u05FF])/g, (_m, p1, p2) => `${p1} ${p2}`, curSpans);
      curText = r1.newText;
      curSpans = r1.newSpans;
      stats.spacesAfterPunctFixed += r1.count;

      const r2 = applyReplacementWithSpans(curText, /([:])([\u0590-\u05FF]{2,})/g, (_m, p1, p2) => `${p1} ${p2}`, curSpans);
      curText = r2.newText;
      curSpans = r2.newSpans;
      stats.spacesAfterPunctFixed += r2.count;
    }

    // 4. Normalize quotes to Hebrew Gershayim (״) and Geresh (׳)
    if (options.normalizeQuotesAndGeresh) {
      // Double quote between Hebrew characters -> Gershayim U+05F4
      const rq = applyReplacementWithSpans(curText, /([\u0590-\u05FF])"([\u0590-\u05FF])/g, (_m, p1, p2) => `${p1}״${p2}`, curSpans);
      curText = rq.newText;
      curSpans = rq.newSpans;
      stats.quotesConverted += rq.count;

      // Single quote at end of Hebrew word/abbreviation -> Geresh U+05F3
      const rg = applyReplacementWithSpans(
        curText,
        /([\u0590-\u05FF])'(?=\s|[,.:;!?׃\-]|$|[^\u0590-\u05FF])/g,
        (_m, p1) => `${p1}׳`,
        curSpans
      );
      curText = rg.newText;
      curSpans = rg.newSpans;
      stats.gereshConverted += rg.count;
    }

    // 5. Automatic Dibur HaMatchil run-in bold styling
    if (options.boldDiburHaMatchil) {
      const delimIdx = findDiburDelimiter(curText, options.diburDelimiter);
      if (delimIdx > 0) {
        const boldEnd = delimIdx + 1; // Include delimiter in bold formatting
        curSpans = formatRangeInSpans(curSpans, 0, boldEnd, { fontWeight: 700 }, curText.length);
        stats.diburHaMatchilBolded++;
      }
    }

    result.push({
      id: original.id,
      styleId: original.styleId,
      text: curText,
      spans: curSpans.length > 0 ? curSpans : undefined,
      patch: original.patch ? { ...original.patch } : undefined
    });
  }

  stats.totalModifications =
    stats.doubleSpacesFixed +
    stats.spacesBeforePunctFixed +
    stats.spacesAfterPunctFixed +
    stats.quotesConverted +
    stats.gereshConverted +
    stats.emptyParagraphsRemoved +
    stats.diburHaMatchilBolded;

  return { paragraphs: result, stats };
}

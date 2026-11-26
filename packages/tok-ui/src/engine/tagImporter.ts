/**
 * Legacy "Tag" (תג) Flat-Code Lexer & Semantic Importer (נושא 5.2).
 *
 * Converts historical Israeli/Torah typesetting Tag codes (`@1`, `@2`, `@00`, `@כותרת`,
 * `<<B>>...<</B>>`, `<<I>>...<</I>>`, `<<U>>...<</U>>`, `<<S:10>>...<</S>>`,
 * `{{FN:...}}` / `@FN{...}`) into semantic `StoryParagraph`s, `TextSpan`s, and footnotes.
 */

import { CharacterFormat, StoryParagraph, TextSpan } from 'tok-story-editor';
import {
  cleanDocumentTitle,
  createStoryParagraph,
  detectTextDirection,
  detectTextLanguage,
  ImportOptions,
  ImportResult
} from './textImporter.js';

export interface TagTokenResult {
  cleanText: string;
  spans: TextSpan[];
  footnotes: string[];
}

const TAG_STYLE_MAP: Record<string, string> = {
  '0': 'default-body',
  '00': 'default-body',
  '1': 'default-heading',
  '2': 'default-subheading',
  '3': 'default-subheading',
  'כותרת': 'default-heading',
  'כותרת1': 'default-heading',
  'כותרת2': 'default-subheading',
  'גוף': 'default-body',
  'ציטוט': 'quote',
  'הערה': 'footnote',
  'מקור': 'footnote'
};

/**
 * Heuristic check for whether raw text contains legacy Tag (`תג`) flat codes.
 */
export function isLegacyTagFormat(text: string): boolean {
  if (!text) return false;
  return /(?:^|\r?\n)@[0-9א-ת_]{1,24}(?:\s|$)/m.test(text) || /<<[BIUS](?::\d+(?:\.\d+)?)?>>/.test(text);
}

/**
 * Lexes inline Tag formatting markers (`<<B>>...<</B>>`, `<<I>>...<</I>>`, `<<U>>...<</U>>`,
 * `<<S:pt>>...<</S>>`, `{{FN:...}}`, `@FN{...}`) into clean text, disjoint `TextSpan`s, and
 * extracted footnote bodies.
 */
export function lexTagInlineFormatting(raw: string): TagTokenResult {
  let cleanText = '';
  const spans: TextSpan[] = [];
  const footnotes: string[] = [];

  const activeStack: Array<{ tag: string; start: number; sizePt?: number }> = [];
  let i = 0;

  while (i < raw.length) {
    // Footnote marker: {{FN:...}} or @FN{...}
    if (raw.startsWith('{{FN:', i)) {
      const endIdx = raw.indexOf('}}', i + 5);
      if (endIdx !== -1) {
        const fnBody = raw.slice(i + 5, endIdx).trim();
        if (fnBody) footnotes.push(fnBody);
        i = endIdx + 2;
        continue;
      }
    }
    if (raw.startsWith('@FN{', i)) {
      const endIdx = raw.indexOf('}', i + 4);
      if (endIdx !== -1) {
        const fnBody = raw.slice(i + 4, endIdx).trim();
        if (fnBody) footnotes.push(fnBody);
        i = endIdx + 1;
        continue;
      }
    }

    // Inline open/close tags: <<B>>, <</B>>, <<S:9.5>>, <</S>>
    if (raw[i] === '<' && raw[i + 1] === '<') {
      const closeBracket = raw.indexOf('>>', i + 2);
      if (closeBracket !== -1) {
        const token = raw.slice(i + 2, closeBracket).trim();
        if (token.startsWith('/')) {
          const tagName = token.slice(1).toUpperCase();
          for (let k = activeStack.length - 1; k >= 0; k--) {
            if (activeStack[k].tag === tagName) {
              const entry = activeStack.splice(k, 1)[0];
              const endOffset = cleanText.length;
              if (endOffset > entry.start) {
                const fmt: CharacterFormat = {};
                if (tagName === 'B') fmt.fontWeight = 700;
                else if (tagName === 'I') fmt.italic = true;
                else if (tagName === 'U') fmt.underline = true;
                else if (tagName === 'S' && entry.sizePt) fmt.fontSizePt = entry.sizePt;
                spans.push({ start: entry.start, end: endOffset, format: fmt });
              }
              break;
            }
          }
          i = closeBracket + 2;
          continue;
        } else {
          const [tagPart, argPart] = token.split(':');
          const tagName = tagPart.toUpperCase();
          if (tagName === 'B' || tagName === 'I' || tagName === 'U' || tagName === 'S') {
            const sizePt = tagName === 'S' && argPart ? parseFloat(argPart) : undefined;
            activeStack.push({
              tag: tagName,
              start: cleanText.length,
              sizePt: sizePt && Number.isFinite(sizePt) ? sizePt : undefined
            });
            i = closeBracket + 2;
            continue;
          }
        }
      }
    }

    cleanText += raw[i];
    i++;
  }

  return { cleanText, spans, footnotes };
}

/**
 * Parses a legacy Tag (`תג`) flat-code text document into a full `ImportResult`.
 */
export function parseLegacyTagDocument(rawText: string, options?: ImportOptions): ImportResult {
  const normalized = rawText.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const lines = normalized.split('\n');

  const paragraphs: StoryParagraph[] = [];
  const footnotes: StoryParagraph[] = [];
  let currentStyleId = options?.defaultStyleId || 'default-body';
  let fnCounter = 1;

  for (const rawLine of lines) {
    const trimmed = rawLine.trim();
    if (!trimmed) continue;

    let body = trimmed;
    const tagMatch = trimmed.match(/^@([0-9א-ת_]{1,24})(?:\s+(.*))?$/);
    if (tagMatch) {
      const code = tagMatch[1];
      currentStyleId = TAG_STYLE_MAP[code] || `tag-${code}`;
      body = (tagMatch[2] || '').trim();
      if (!body) {
        // Standalone style-switch directive line
        continue;
      }
    }

    const lexed = lexTagInlineFormatting(body);
    if (!lexed.cleanText.trim() && lexed.footnotes.length === 0) {
      continue;
    }

    const p = createStoryParagraph(
      lexed.cleanText,
      currentStyleId,
      lexed.spans.length > 0 ? lexed.spans : undefined
    );
    paragraphs.push(p);

    for (const fnText of lexed.footnotes) {
      footnotes.push(createStoryParagraph(`[${fnCounter}] ${fnText}`, 'footnote'));
      fnCounter++;
    }
  }

  const allText = paragraphs.map((p) => p.text).join(' ');
  const wordCount = allText.trim() ? allText.trim().split(/\s+/).length : 0;

  return {
    title: cleanDocumentTitle(options?.fileName),
    paragraphs,
    footnotes: footnotes.length > 0 ? footnotes : undefined,
    format: 'tag',
    detectedLanguage: detectTextLanguage(allText),
    detectedDirection: detectTextDirection(allText),
    wordCount,
    paragraphCount: paragraphs.length
  };
}

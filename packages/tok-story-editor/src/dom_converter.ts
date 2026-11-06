/**
 * TypesetOK (TOK) - Semantic DOM Converter for Story Editor.
 *
 * Losslessly converts between semantic `StoryParagraph` (with `TextSpan`s and
 * `ParagraphStylePatch`) and contentEditable DOM elements without HTML tag pollution.
 */

import {
  CharacterFormat,
  ParagraphStylePatch,
  StoryParagraph,
  TextSpan,
  isFormatEmpty,
  normalizeSpans
} from './types.js';

/** Converts points to CSS px string (1 pt = 1.333 px) */
export function ptToPx(pt: number): number {
  return Math.round(pt * 1.3333 * 10) / 10;
}

/** Applies a ParagraphStylePatch to a paragraph DOM element */
export function applyPatchToElement(pEl: HTMLElement, patch?: ParagraphStylePatch): void {
  if (!patch) return;
  if (patch.alignment) {
    pEl.style.textAlign = patch.alignment === 'Justified' ? 'justify' : patch.alignment.toLowerCase();
  }
  if (patch.direction) {
    pEl.dir = patch.direction.toLowerCase();
  }
  if (patch.firstLineIndentPt !== undefined) {
    pEl.style.textIndent = `${patch.firstLineIndentPt}pt`;
  }
  if (patch.indentStartPt !== undefined) {
    pEl.style.marginInlineStart = `${patch.indentStartPt}pt`;
  }
  if (patch.indentEndPt !== undefined) {
    pEl.style.marginInlineEnd = `${patch.indentEndPt}pt`;
  }
  if (patch.spaceBeforePt !== undefined) {
    pEl.style.marginTop = `${patch.spaceBeforePt}pt`;
  }
  if (patch.spaceAfterPt !== undefined) {
    pEl.style.marginBottom = `${patch.spaceAfterPt}pt`;
  }
  if (patch.fontFamily) {
    pEl.style.fontFamily = patch.fontFamily;
  }
  if (patch.fontSizePt !== undefined) {
    pEl.style.fontSize = `${patch.fontSizePt}pt`;
  }
  if (patch.fontWeight !== undefined) {
    pEl.style.fontWeight = String(patch.fontWeight);
  }
  if (patch.lineHeightPt !== undefined) {
    pEl.style.lineHeight = `${patch.lineHeightPt}pt`;
  }
  if (patch.color) {
    pEl.style.color = patch.color;
  }
}

/** Extracts a ParagraphStylePatch from a paragraph DOM element */
export function extractPatchFromElement(pEl: HTMLElement): ParagraphStylePatch | undefined {
  const patch: ParagraphStylePatch = {};
  let hasPatch = false;

  const textAlign = pEl.style.textAlign;
  if (textAlign) {
    if (textAlign === 'justify') patch.alignment = 'Justified';
    else if (textAlign === 'right') patch.alignment = 'Right';
    else if (textAlign === 'left') patch.alignment = 'Left';
    else if (textAlign === 'center') patch.alignment = 'Center';
    hasPatch = true;
  }

  const dir = pEl.dir || pEl.getAttribute('dir');
  if (dir) {
    patch.direction = dir.toLowerCase() === 'ltr' ? 'Ltr' : 'Rtl';
    hasPatch = true;
  }

  const textIndent = pEl.style.textIndent;
  if (textIndent && textIndent.endsWith('pt')) {
    patch.firstLineIndentPt = parseFloat(textIndent);
    hasPatch = true;
  }

  const marginStart = pEl.style.marginInlineStart;
  if (marginStart && marginStart.endsWith('pt')) {
    patch.indentStartPt = parseFloat(marginStart);
    hasPatch = true;
  }

  const marginEnd = pEl.style.marginInlineEnd;
  if (marginEnd && marginEnd.endsWith('pt')) {
    patch.indentEndPt = parseFloat(marginEnd);
    hasPatch = true;
  }

  const marginTop = pEl.style.marginTop;
  if (marginTop && marginTop.endsWith('pt')) {
    patch.spaceBeforePt = parseFloat(marginTop);
    hasPatch = true;
  }

  const marginBottom = pEl.style.marginBottom;
  if (marginBottom && marginBottom.endsWith('pt')) {
    patch.spaceAfterPt = parseFloat(marginBottom);
    hasPatch = true;
  }

  const fontFamily = pEl.style.fontFamily;
  if (fontFamily) {
    patch.fontFamily = fontFamily.replace(/^['"]|['"]$/g, '');
    hasPatch = true;
  }

  const fontSize = pEl.style.fontSize;
  if (fontSize && (fontSize.endsWith('pt') || fontSize.endsWith('px'))) {
    const val = parseFloat(fontSize);
    patch.fontSizePt = fontSize.endsWith('px') ? Math.round(val * 0.75 * 10) / 10 : val;
    hasPatch = true;
  }

  const fontWeight = pEl.style.fontWeight;
  if (fontWeight) {
    const val = parseInt(fontWeight, 10);
    patch.fontWeight = isNaN(val) ? (fontWeight === 'bold' ? 700 : 400) : val;
    hasPatch = true;
  }

  const lineHeight = pEl.style.lineHeight;
  if (lineHeight && (lineHeight.endsWith('pt') || lineHeight.endsWith('px'))) {
    const val = parseFloat(lineHeight);
    patch.lineHeightPt = lineHeight.endsWith('px') ? Math.round(val * 0.75 * 10) / 10 : val;
    hasPatch = true;
  }

  const color = pEl.style.color;
  if (color) {
    patch.color = color;
    hasPatch = true;
  }

  return hasPatch ? patch : undefined;
}

/** Renders a StoryParagraph into an HTMLParagraphElement */
export function renderParagraphDom(p: StoryParagraph): HTMLParagraphElement {
  const pEl = document.createElement('p');
  pEl.dataset.paraId = p.id;
  pEl.dataset.styleId = p.styleId;

  if (p.patch) {
    applyPatchToElement(pEl, p.patch);
  }

  const text = p.text || '';
  const textChars = Array.from(text);
  const spans = normalizeSpans(p.spans || [], textChars.length);

  if (spans.length === 0) {
    pEl.textContent = text;
    return pEl;
  }

  // Slice text by span boundaries
  const boundaries = new Set<number>([0, textChars.length]);
  for (const s of spans) {
    boundaries.add(s.start);
    boundaries.add(s.end);
  }
  const points = Array.from(boundaries).sort((a, b) => a - b);

  const frag = document.createDocumentFragment();
  for (let i = 0; i < points.length - 1; i++) {
    const start = points[i];
    const end = points[i + 1];
    if (start >= end) continue;

    const sliceText = textChars.slice(start, end).join('');
    // Find active formatting span covering this slice
    const activeSpan = spans.find((s) => s.start <= start && s.end >= end);

    if (activeSpan && !isFormatEmpty(activeSpan.format)) {
      const spanEl = document.createElement('span');
      const fmt = activeSpan.format;

      if (fmt.fontWeight && fmt.fontWeight >= 600) {
        spanEl.style.fontWeight = 'bold';
      }
      if (fmt.italic) {
        spanEl.style.fontStyle = 'italic';
      }
      if (fmt.underline) {
        spanEl.style.textDecoration = 'underline';
      }
      if (fmt.fontFamily) {
        spanEl.style.fontFamily = fmt.fontFamily;
      }
      if (fmt.fontSizePt !== undefined) {
        spanEl.style.fontSize = `${fmt.fontSizePt}pt`;
      }
      if (fmt.color) {
        spanEl.style.color = fmt.color;
      }
      spanEl.textContent = sliceText;
      frag.appendChild(spanEl);
    } else {
      frag.appendChild(document.createTextNode(sliceText));
    }
  }

  pEl.appendChild(frag);
  return pEl;
}

/** Parses a paragraph DOM element into a semantic StoryParagraph */
export function parseParagraphDom(pEl: HTMLElement): StoryParagraph {
  const id = pEl.dataset.paraId || '';
  const styleId = pEl.dataset.styleId || '';
  const patch = extractPatchFromElement(pEl);

  let rawText = '';
  const spans: TextSpan[] = [];

  function traverse(node: Node, currentFormat: CharacterFormat) {
    if (node.nodeType === Node.TEXT_NODE) {
      const nodeText = node.textContent || '';
      if (!nodeText) return;

      const charStart = Array.from(rawText).length;
      rawText += nodeText;
      const charEnd = Array.from(rawText).length;

      if (!isFormatEmpty(currentFormat)) {
        spans.push({
          start: charStart,
          end: charEnd,
          format: { ...currentFormat }
        });
      }
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as HTMLElement;
      const childFmt: CharacterFormat = { ...currentFormat };

      const tag = el.tagName.toUpperCase();
      if (tag === 'B' || tag === 'STRONG' || el.style.fontWeight === 'bold' || parseInt(el.style.fontWeight || '0', 10) >= 600) {
        childFmt.fontWeight = 700;
      }
      if (tag === 'I' || tag === 'EM' || el.style.fontStyle === 'italic') {
        childFmt.italic = true;
      }
      if (tag === 'U' || el.style.textDecoration?.includes('underline')) {
        childFmt.underline = true;
      }
      if (el.style.fontFamily) {
        childFmt.fontFamily = el.style.fontFamily;
      }
      if (el.style.fontSize && el.style.fontSize.endsWith('pt')) {
        childFmt.fontSizePt = parseFloat(el.style.fontSize);
      }
      if (el.style.color) {
        childFmt.color = el.style.color;
      }

      for (let i = 0; i < el.childNodes.length; i++) {
        traverse(el.childNodes[i], childFmt);
      }
    }
  }

  if (pEl.childNodes && pEl.childNodes.length > 0) {
    for (let i = 0; i < pEl.childNodes.length; i++) {
      traverse(pEl.childNodes[i], {});
    }
  } else if (pEl.textContent) {
    rawText = pEl.textContent;
  }

  const charCount = Array.from(rawText).length;
  const canonicalSpans = normalizeSpans(spans, charCount);

  return {
    id,
    styleId,
    text: rawText,
    spans: canonicalSpans.length > 0 ? canonicalSpans : undefined,
    patch
  };
}

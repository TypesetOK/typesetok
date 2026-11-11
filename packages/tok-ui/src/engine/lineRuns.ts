/**
 * Line runs
 *
 * The Rust engine lays out every line and reports its text, but not the
 * character formatting inside it. This module recovers that formatting from the
 * paragraph's spans, so page view can paint italic, underline, colour, weight,
 * family and size on the part of the line that has them, the same way the
 * story editor shows them.
 *
 * Layout is never changed here: the engine already measured the line with the
 * span fonts and sizes. Only the painting of the line text is richer.
 */

import type { CharacterFormat, TextSpan } from 'tok-story-editor';

/** What page view needs to know about one paragraph of the document. */
export interface LineRunSource {
  /** Paragraph text exactly as it was sent to the engine. */
  text: string;
  spans: TextSpan[];
  /** Paragraph-level text colour (style patch), applied to the whole line. */
  color?: string;
}

export type LineRunLookup = (paragraphId: string) => LineRunSource | undefined;

/** Resolves a font family from a span to a CSS `font-family` value. */
export type FamilyToCss = (family: string) => string;

interface LineRef {
  text: string;
  paragraph_id?: string | null;
  is_drop_cap?: boolean;
}

export interface LocatedLine {
  source: LineRunSource;
  /** Character offset of the line's first character in the paragraph text. */
  start: number;
}

// Plain colour notations only: this value ends up inside a style attribute.
const SAFE_COLOR = /^(#[0-9a-f]{3,8}|rgba?\([\d\s.,%]+\)|hsla?\([\d\s.,%a-z]+\)|[a-z]{3,20})$/i;

export function safeCssColor(value: string | undefined): string | undefined {
  if (!value) return undefined;
  const v = value.trim();
  return SAFE_COLOR.test(v) ? v : undefined;
}

function escapeText(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/** True when a span changes how text looks (a link or a style id alone does not). */
function hasVisibleFormat(f: CharacterFormat | undefined): boolean {
  if (!f) return false;
  return Boolean(
    f.italic ||
    f.underline ||
    safeCssColor(f.color) ||
    f.fontWeight !== undefined ||
    f.fontFamily ||
    (f.fontSizePt !== undefined && f.fontSizePt > 0)
  );
}

/**
 * Finds where each engine line starts inside its paragraph.
 *
 * A paragraph's lines arrive in reading order and each line's text is the next
 * slice of the paragraph, so a moving cursor locates them. A line whose text
 * cannot be found (for example when the engine collapsed repeated spaces) is
 * left out and keeps its plain rendering.
 */
export function locateLines<T extends LineRef>(lines: Iterable<T>, lookup: LineRunLookup): Map<T, LocatedLine> {
  const located = new Map<T, LocatedLine>();
  const cursors = new Map<string, number>();
  for (const line of lines) {
    const pid = line.paragraph_id;
    if (!pid || line.is_drop_cap || !line.text) continue;
    const source = lookup(String(pid));
    if (!source) continue;
    const from = cursors.get(String(pid)) ?? 0;
    const at = source.text.indexOf(line.text, from);
    if (at < 0) continue;
    located.set(line, { source, start: at });
    cursors.set(String(pid), at + line.text.length);
  }
  return located;
}

function cssForFormat(f: CharacterFormat, familyToCss: FamilyToCss): string {
  const css: string[] = [];
  if (f.fontFamily) css.push(`font-family: ${familyToCss(f.fontFamily)};`);
  if (f.fontSizePt !== undefined && f.fontSizePt > 0 && Number.isFinite(f.fontSizePt)) {
    css.push(`font-size: ${f.fontSizePt}pt;`);
  }
  if (f.fontWeight !== undefined) css.push(`font-weight: ${f.fontWeight >= 600 ? 700 : 400};`);
  if (f.italic) css.push('font-style: italic;');
  if (f.underline) css.push('text-decoration: underline;');
  const color = safeCssColor(f.color);
  if (color) css.push(`color: ${color};`);
  return css.join(' ');
}

/**
 * Line text as HTML with one `<span>` per run of identical formatting.
 * Returns null when nothing in the line is formatted, so the caller keeps the
 * existing plain rendering.
 */
export function renderStyledLine(lineText: string, located: LocatedLine, familyToCss: FamilyToCss): string | null {
  const start = located.start;
  const end = start + lineText.length;
  const spans = located.source.spans.filter((s) => s.end > start && s.start < end && hasVisibleFormat(s.format));
  if (spans.length === 0) return null;

  const cuts = new Set<number>([start, end]);
  for (const s of spans) {
    cuts.add(Math.max(start, s.start));
    cuts.add(Math.min(end, s.end));
  }
  const points = [...cuts].sort((a, b) => a - b);

  let html = '';
  for (let i = 0; i + 1 < points.length; i++) {
    const a = points[i];
    const b = points[i + 1];
    const piece = escapeText(lineText.slice(a - start, b - start));
    // Later spans win over earlier ones where they overlap.
    const merged: CharacterFormat = {};
    for (const s of spans) {
      if (s.start <= a && s.end >= b) Object.assign(merged, s.format);
    }
    const css = cssForFormat(merged, familyToCss);
    html += css ? `<span style="${css}">${piece}</span>` : piece;
  }
  return html;
}

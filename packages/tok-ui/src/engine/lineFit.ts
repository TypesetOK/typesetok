/**
 * Preview-side line fitting.
 *
 * The engine justifies every full line to the column measure and reports the final width in
 * `data-line-width`. The browser, however, paints each line at its natural width, so the page
 * view showed ragged lines (and lines wider than their frame got clipped mid-word).
 *
 * `fitJustifiedLines` reproduces the engine's three tiers on the rendered lines:
 *   1. inter-word space within [80%, 130%] of the natural space,
 *   2. widening of the Ahalterm letters (א ה ל ת ר ם ד ך) for the rest,
 *   3. micro-tracking over all letters when there are no such letters.
 * Only extra advance is added or removed; glyph shapes are never scaled.
 */

const PT_TO_PX = 96 / 72;
/** Mirrors crates/tok-typeset/src/engine.rs and hebrew_justify.rs. */
const SPACE_EM = 0.28;
const SPACE_STRETCH = 0.3;
const SPACE_SHRINK = 0.2;
const MAX_TRACKING_EM = 0.02;
/** Beyond the engine's own limit we still pull a too-wide line in, rather than let the frame clip it. */
const MAX_SQUEEZE_EM = 0.06;
const AHALTERM = new Set(['א', 'ה', 'ל', 'ת', 'ר', 'ם', 'ד', 'ך']);
const IS_MARK = /[\u0591-\u05C7]/;

function unwrapLetters(line: HTMLElement): void {
  line.style.wordSpacing = '';
  line.style.letterSpacing = '';
  const spans = line.querySelectorAll<HTMLElement>('span.tok-ah');
  if (spans.length === 0) return;
  spans.forEach((s) => s.replaceWith(document.createTextNode(s.textContent || '')));
  line.normalize();
}

function textNodes(line: HTMLElement): Text[] {
  const out: Text[] = [];
  const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) out.push(n as Text);
  return out;
}

function naturalWidthPx(line: HTMLElement): number {
  const range = document.createRange();
  range.selectNodeContents(line);
  const rect = range.getBoundingClientRect();
  const box = line.getBoundingClientRect();
  // The canvas can be zoomed with a CSS transform: convert back to unscaled px.
  const scale = line.offsetWidth > 0 && box.width > 0 ? box.width / line.offsetWidth : 1;
  return rect.width / (scale || 1);
}

/** Wraps each Ahalterm letter (with its niqqud/te'amim) in a span and returns the spans. */
function wrapAhalterm(line: HTMLElement): HTMLElement[] {
  const spans: HTMLElement[] = [];
  for (const node of textNodes(line)) {
    const text = node.data;
    if (![...text].some((ch) => AHALTERM.has(ch))) continue;
    const frag = document.createDocumentFragment();
    let plain = '';
    const chars = [...text];
    for (let i = 0; i < chars.length; i++) {
      if (AHALTERM.has(chars[i])) {
        let cluster = chars[i];
        while (i + 1 < chars.length && IS_MARK.test(chars[i + 1])) cluster += chars[++i];
        if (plain) {
          frag.appendChild(document.createTextNode(plain));
          plain = '';
        }
        const span = document.createElement('span');
        span.className = 'tok-ah';
        span.textContent = cluster;
        frag.appendChild(span);
        spans.push(span);
      } else {
        plain += chars[i];
      }
    }
    if (plain) frag.appendChild(document.createTextNode(plain));
    node.replaceWith(frag);
  }
  return spans;
}

function fitLine(line: HTMLElement): void {
  unwrapLetters(line);
  const targetPt = Number(line.dataset.lineWidth);
  if (!Number.isFinite(targetPt) || targetPt <= 0) return;
  const fontPx = parseFloat(getComputedStyle(line).fontSize) || 12;
  const target = targetPt * PT_TO_PX;
  const natural = naturalWidthPx(line);
  if (!Number.isFinite(natural) || natural <= 0) return;
  const deficit = target - natural;
  if (Math.abs(deficit) < 0.3) return;

  const text = line.textContent || '';
  const spaceCount = (text.match(/ /g) || []).length;
  const spaceW = fontPx * SPACE_EM;
  const maxStretch = spaceCount * spaceW * SPACE_STRETCH;
  const maxShrink = spaceCount * spaceW * SPACE_SHRINK;

  // Tier 1: word spacing.
  let remaining = deficit;
  if (spaceCount > 0) {
    const used = Math.max(-maxShrink, Math.min(maxStretch, deficit));
    line.style.wordSpacing = `${(used / spaceCount).toFixed(3)}px`;
    remaining = deficit - used;
  }
  if (Math.abs(remaining) < 0.3) return;

  if (remaining > 0) {
    // Tier 2: widen the Ahalterm letters.
    const spans = wrapAhalterm(line);
    if (spans.length > 0) {
      const each = remaining / spans.length;
      spans.forEach((s) => {
        s.style.letterSpacing = `${each.toFixed(3)}px`;
      });
      return;
    }
  }

  // Tier 3: micro-tracking over every letter.
  const letters = [...text].filter((ch) => ch !== ' ' && !IS_MARK.test(ch)).length;
  if (letters > 1) {
    const limit = (remaining > 0 ? MAX_TRACKING_EM : MAX_SQUEEZE_EM) * fontPx;
    const each = Math.max(-limit, Math.min(limit, remaining / letters));
    line.style.letterSpacing = `${each.toFixed(3)}px`;
  }
}

/** Fits every justified line below `root` (a page body) to the width the engine computed. */
export function fitJustifiedLines(root: HTMLElement): void {
  const lines = root.querySelectorAll<HTMLElement>('.tok-line-box[data-justify="1"]');
  lines.forEach((line) => {
    try {
      fitLine(line);
    } catch {
      /* a line that cannot be measured is left as the browser painted it */
    }
  });
}

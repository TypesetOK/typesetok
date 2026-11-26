/**
 * Preview-side line fitting, driven by the engine's own numbers.
 *
 * The Rust engine justifies each full line and reports, per glyph, its final advance. The
 * browser paints text at its natural width, so the page view showed ragged lines. This module
 * replays what the engine decided, instead of estimating it:
 *
 *   Tier 1  every inter-word space gets exactly the width the engine gave it (`data-sp`).
 *   Tier 2  every widened Ahalterm letter (א ה ל ת ר ם ד ך) gets the engine's advance and is
 *           drawn horizontally widened by the engine's `stretch`, from its origin, exactly like
 *           the PDF text-matrix scale (`data-ah`).
 *   Tier 3  whatever small difference is left (micro-tracking, or font metrics that differ
 *           slightly from the engine's) is spread as letter-spacing, so the line ends where the
 *           engine's line ends.
 *
 * `data-sp` / `data-ah` are lists in logical (cluster) order, so they map one-to-one onto the
 * spaces and Ahalterm letters in the line text. If the counts do not match, that tier is skipped
 * and the residual step still closes the gap.
 */

const PT_TO_PX = 96 / 72;
/** Residual tracking limit, as a fraction of the font size (engine Tier 3 allows 2%). */
const MAX_RESIDUAL_EM = 0.06;
const AHALTERM = new Set(['א', 'ה', 'ל', 'ת', 'ר', 'ם', 'ד', 'ך']);
const IS_MARK = /[\u0591-\u05C7]/;

function resetLine(line: HTMLElement): void {
  line.style.letterSpacing = '';
  line.style.wordSpacing = '';
  const spaces = line.querySelectorAll<HTMLElement>('span.tok-sp');
  spaces.forEach((s) => s.replaceWith(document.createTextNode(' ')));
  const letters = line.querySelectorAll<HTMLElement>('span.tok-ah');
  letters.forEach((s) => s.replaceWith(document.createTextNode(s.textContent || '')));
  if (spaces.length > 0 || letters.length > 0) line.normalize();
}

function textNodes(line: HTMLElement): Text[] {
  const out: Text[] = [];
  const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) out.push(n as Text);
  return out;
}

function parseList(raw: string | undefined): number[][] {
  if (!raw) return [];
  return raw.split(';').map((item) => item.split(',').map(Number));
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

/** Replaces each space, in logical order, by a box as wide as the engine's justified space. */
function applySpaces(line: HTMLElement, widthsPt: number[][]): void {
  const nodes = textNodes(line);
  const total = nodes.reduce((n, node) => n + (node.data.match(/ /g) || []).length, 0);
  if (total !== widthsPt.length) return;
  let i = 0;
  for (const node of nodes) {
    if (!node.data.includes(' ')) continue;
    const frag = document.createDocumentFragment();
    node.data.split(/( )/).forEach((part) => {
      if (part === ' ') {
        const span = document.createElement('span');
        span.className = 'tok-sp';
        span.textContent = ' ';
        span.style.display = 'inline-block';
        span.style.whiteSpace = 'pre';
        span.style.letterSpacing = '0';
        span.style.width = `${widthsPt[i++][0]}pt`;
        frag.appendChild(span);
      } else if (part) {
        frag.appendChild(document.createTextNode(part));
      }
    });
    node.replaceWith(frag);
  }
}

/** Widens each Ahalterm letter (with its niqqud/te'amim) by the engine's factor, from its origin. */
function applyLetters(line: HTMLElement, letters: number[][]): void {
  const nodes = textNodes(line).filter((n) => !n.parentElement?.closest('span.tok-sp'));
  const total = nodes.reduce((n, node) => n + [...node.data].filter((ch) => AHALTERM.has(ch)).length, 0);
  if (total !== letters.length) return;
  let i = 0;
  for (const node of nodes) {
    const chars = [...node.data];
    if (!chars.some((ch) => AHALTERM.has(ch))) continue;
    const frag = document.createDocumentFragment();
    let plain = '';
    for (let k = 0; k < chars.length; k++) {
      if (!AHALTERM.has(chars[k])) {
        plain += chars[k];
        continue;
      }
      let cluster = chars[k];
      while (k + 1 < chars.length && IS_MARK.test(chars[k + 1])) cluster += chars[++k];
      if (plain) {
        frag.appendChild(document.createTextNode(plain));
        plain = '';
      }
      const [widthPt, stretch] = letters[i++];
      // The outer box is as wide as the engine's advance; the glyph sits at its left origin
      // (the engine lays glyphs out left to right) and is widened from there.
      const outer = document.createElement('span');
      outer.className = 'tok-ah';
      outer.style.display = 'inline-block';
      outer.style.width = `${widthPt}pt`;
      outer.style.direction = 'ltr';
      outer.style.textAlign = 'left';
      outer.style.whiteSpace = 'nowrap';
      outer.style.letterSpacing = '0';
      const inner = document.createElement('span');
      inner.style.display = 'inline-block';
      inner.style.transformOrigin = '0 50%';
      inner.style.transform = `scaleX(${(1 + stretch).toFixed(4)})`;
      inner.textContent = cluster;
      outer.appendChild(inner);
      frag.appendChild(outer);
    }
    if (plain) frag.appendChild(document.createTextNode(plain));
    node.replaceWith(frag);
  }
}

function fitLine(line: HTMLElement): void {
  resetLine(line);
  const targetPt = Number(line.dataset.lineWidth);
  if (!Number.isFinite(targetPt) || targetPt <= 0) return;

  const spaces = parseList(line.dataset.sp);
  if (spaces.length > 0) applySpaces(line, spaces);
  const letters = parseList(line.dataset.ah);
  if (letters.length > 0) applyLetters(line, letters);

  // Residual: micro-tracking or sub-point differences in font metrics.
  const fontPx = parseFloat(getComputedStyle(line).fontSize) || 12;
  const residual = targetPt * PT_TO_PX - naturalWidthPx(line);
  if (!Number.isFinite(residual) || Math.abs(residual) < 0.3) return;
  const loose = textNodes(line)
    .filter((n) => !n.parentElement?.closest('span.tok-sp, span.tok-ah'))
    .reduce((n, node) => n + [...node.data].filter((ch) => ch !== ' ' && !IS_MARK.test(ch)).length, 0);
  if (loose < 2) return;
  const limit = MAX_RESIDUAL_EM * fontPx;
  const each = Math.max(-limit, Math.min(limit, residual / loose));
  line.style.letterSpacing = `${each.toFixed(3)}px`;
}

/** Fits every justified line below `root` (a page body) to the widths the engine computed. */
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

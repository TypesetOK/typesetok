/**
 * Baseline guides of one page sheet, drawn from the lines the typesetting engine actually placed.
 * One thin rule per rendered line, spanning that line's own frame, so the guide always sits exactly
 * where the text sits (a fixed-step grid never matched the real line pitch of mixed-size columns).
 *
 * Positions are measured from the rendered lines, not computed from the engine numbers, so the page
 * padding and the zoom level (CSS scale) can never shift the guides away from the text.
 * Returns false when the sheet has no engine lines yet, so the caller can show a plain fixed grid.
 */
export function drawLineGuides(layer: HTMLElement, sheet: HTMLElement | null, color: string): boolean {
  layer.replaceChildren();
  if (!sheet) return false;
  layer.style.inset = '0';

  const origin = layer.getBoundingClientRect();
  const scale = layer.offsetWidth > 0 ? origin.width / layer.offsetWidth : 1;
  if (!(scale > 0)) return false;

  // Read everything first, then write, so the browser lays out only once.
  const rules: { x: number; y: number; w: number }[] = [];
  sheet.querySelectorAll<HTMLElement>('.tok-interactive-frame').forEach((frame) => {
    const frameRect = frame.getBoundingClientRect();
    frame.querySelectorAll<HTMLElement>('.tok-line[data-baseline-y]').forEach((line) => {
      const bottom = line.getBoundingClientRect().bottom;
      if (bottom > frameRect.bottom + 0.5) return; // clipped by the frame, so not visible
      rules.push({
        x: (frameRect.left - origin.left) / scale,
        y: (bottom - origin.top) / scale,
        w: frameRect.width / scale
      });
    });
  });

  const fragment = document.createDocumentFragment();
  for (const r of rules) {
    const rule = document.createElement('div');
    rule.className = 'tok-guide-baseline-line';
    rule.style.cssText = `position:absolute;left:${r.x}px;top:${r.y}px;width:${r.w}px;height:1px;background:${color};`;
    fragment.appendChild(rule);
  }
  layer.appendChild(fragment);
  return rules.length > 0;
}

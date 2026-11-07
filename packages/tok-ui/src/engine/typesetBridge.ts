/**
 * Typeset Bridge
 *
 * Maps Rust `PageLayoutBox` geometry produced by `tok-cli typeset-document`
 * (and `TypesettingEngine::typeset_document`) to UI `PageDescriptor` objects
 * rendered natively in SpreadCanvas.
 */

import { PageDescriptor } from 'tok-viewer';
import { toHebrewGematria } from '../gematria.js';

export interface PhysicalPoint {
  x: number;
  y: number;
}

export interface PhysicalRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface GlyphBox {
  glyph_id: number;
  cluster: number;
  x: number;
  y: number;
  width: number;
  height: number;
  character?: string;
  font_index?: number;
}

export interface LineBox {
  line_index: number;
  paragraph_id?: string | null;
  baseline_y: number;
  height: number;
  width: number;
  glyphs?: GlyphBox[];
  text: string;
  is_rtl: boolean;
  fonts?: string[];
  is_drop_cap?: boolean;
  x_offset?: number;
}

export interface TextFrameBox {
  frame_id: string;
  flow_id: string;
  rect: PhysicalRect;
  lines: LineBox[];
}

export interface BreakToken {
  section_index: number;
  paragraph_index: number;
  char_offset: number;
}

export interface PageLayoutBox {
  page_index: number;
  page_number_gematria: string;
  dimensions: PhysicalRect;
  frames: TextFrameBox[];
  break_token?: BreakToken | null;
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

export const ENGINE_FONT_STACK = "'Frank Ruhl Libre', 'Noto Rashi Hebrew', 'Noto Serif Hebrew', 'David Libre', serif";

export function formatEngineFont(family?: string, isPrimary = true): { fontFamily: string; fontWeight?: number } {
  if (!family) {
    return {
      fontFamily: isPrimary
        ? `'Frank Ruhl Libre', ${ENGINE_FONT_STACK}`
        : `'Noto Rashi Hebrew', ${ENGINE_FONT_STACK}`
    };
  }

  const isBold = family.endsWith(' Bold') || family.includes('Bold');
  const baseFamily = family.replace(/\s+Bold$/i, '').trim();

  return {
    fontFamily: `'${baseFamily}', ${ENGINE_FONT_STACK}`,
    fontWeight: isBold ? 700 : undefined
  };
}

export function renderLineText(text: string, isDropCap = false): string {
  if (/<(b|strong|em|i|span)\b[^>]*>/i.test(text)) {
    return text.replace(/<(\/?)(\w+)([^>]*)>/g, (match, close, tag, attrs) => {
      const lower = tag.toLowerCase();
      if (['b', 'strong', 'em', 'i', 'span', 'mark'].includes(lower)) {
        const safeAttrs = attrs.replace(/on\w+="[^"]*"/gi, '');
        return `<${close}${lower}${safeAttrs}>`;
      }
      return '';
    });
  }

  let escaped = escapeHtml(text);
  if (isDropCap) {
    escaped = escaped.replace(/^([^\s]+)/, '<strong class="tok-opening-word" style="font-weight: 800; font-size: 1.12em;">$1</strong>');
  }
  return escaped;
}

/**
 * Converts Rust PageLayoutBox array to tok-viewer PageDescriptor array.
 * Uses exact text positions, line heights, baselines, paragraph IDs,
 * and page dimensions calculated by the Rust typesetting engine.
 */
export function layoutToPageDescriptors(pages: PageLayoutBox[]): PageDescriptor[] {
  if (!Array.isArray(pages) || pages.length === 0) {
    return [];
  }

  return pages.map((pageBox) => {
    const pageIndex = typeof pageBox.page_index === 'number' ? pageBox.page_index : 0;
    const gematriaNumber = pageBox.page_number_gematria || toHebrewGematria(pageIndex + 1);
    const pageWidth = pageBox.dimensions?.width || 595.28;
    const pageHeight = pageBox.dimensions?.height || 841.89;

    // Identify footnote/apparatus frames to draw separator rules (33% rule above primary apparatus, 18% rule between apparatuses)
    const footnoteFrameIdsOrdered = (pageBox.frames || [])
      .filter((f) => {
        const fid = (f.flow_id || '').toLowerCase();
        const frid = (f.frame_id || '').toLowerCase();
        return (
          fid === 'notes' ||
          fid === 'footnote' ||
          fid === 'footnotes' ||
          fid === 'heorot' ||
          fid === 'biurim' ||
          fid === 'mekorot' ||
          frid.includes('_footnote_')
        );
      })
      .sort((a, b) => a.rect.y - b.rect.y)
      .map((f) => f.frame_id);

    const framesHtml = (pageBox.frames || []).map((frame) => {
      const leftPt = frame.rect.x;
      const topPt = frame.rect.y;
      const widthPt = frame.rect.width;
      const heightPt = frame.rect.height;

      const rawFlowId = frame.flow_id || 'gemara';
      const flowId = (rawFlowId === 'main' || rawFlowId === 'primary') ? 'gemara' : rawFlowId;
      const frameId = frame.frame_id || `frame_${pageIndex}_${flowId}`;

      const isExpansion = flowId.endsWith('_expansion');
      const rawBaseFlowId = isExpansion ? flowId.replace(/_expansion$/, '') : flowId;
      const baseFlowId = (rawBaseFlowId === 'main' || rawBaseFlowId === 'primary') ? 'gemara' : rawBaseFlowId;

      const fnOrderIdx = footnoteFrameIdsOrdered.indexOf(frame.frame_id);
      const isFootnoteFrame =
        fnOrderIdx >= 0 ||
        baseFlowId === 'notes' ||
        baseFlowId === 'footnote' ||
        baseFlowId === 'heorot' ||
        baseFlowId === 'biurim' ||
        baseFlowId === 'mekorot';

      let flowClass = 'tok-frame-comm';
      if (baseFlowId === 'gemara' || baseFlowId === 'main' || baseFlowId === 'primary') {
        flowClass = 'tok-frame-gemara';
      } else if (isFootnoteFrame) {
        flowClass = 'tok-frame-notes';
      } else if (isExpansion) {
        flowClass = 'tok-frame-comm tok-frame-expansion tok-frame-l-shape';
      }

      const separatorRuleHtml = isFootnoteFrame && frame.lines && frame.lines.length > 0
        ? `<div class="tok-footnote-separator-rule" data-rule-tier="${fnOrderIdx <= 0 ? 'primary' : 'inter-apparatus'}" style="position: absolute; top: 0; right: 0; width: ${fnOrderIdx <= 0 ? '33%' : '18%'}; height: ${fnOrderIdx <= 0 ? '0.75pt' : '0.5pt'}; background: var(--tok-ink-rule, #222);"></div>`
        : '';

      const isPrimary = baseFlowId === 'gemara' || baseFlowId === 'main' || baseFlowId === 'primary';

      const linesHtml = (frame.lines || []).map((line) => {
        // Exact physical top offset from Rust layout engine: baseline_y - height
        const topOffsetPt = Math.max(0, line.baseline_y - line.height);
        const lineHPt = Math.max(1, line.height);

        const engineFont = formatEngineFont(line.fonts?.[0], isPrimary);
        const isBoldLine = line.is_drop_cap || engineFont.fontWeight === 700;
        const lineFontSizePt = (line.glyphs && line.glyphs.length > 0 && line.glyphs[0].height > 0)
          ? line.glyphs[0].height
          : Math.max(7, Math.round(line.height * 0.76 * 10) / 10);
        const fontStyle = `font-family: ${engineFont.fontFamily}; font-size: ${lineFontSizePt.toFixed(2)}pt;` + (isBoldLine ? ` font-weight: 700;` : '');
        const paraAttr = line.paragraph_id ? ` data-para-id="${escapeHtml(String(line.paragraph_id))}"` : '';
        const offsetStyle = line.x_offset ? (line.is_rtl ? ` margin-right: ${line.x_offset.toFixed(2)}pt;` : ` margin-left: ${line.x_offset.toFixed(2)}pt;`) : '';
        const contentHtml = renderLineText(line.text, line.is_drop_cap);

        return `<div class="tok-line-box tok-line" data-line-index="${line.line_index}"${paraAttr} data-baseline-y="${line.baseline_y}" data-line-height="${line.height}" data-line-width="${line.width}" style="position: absolute; top: ${topOffsetPt.toFixed(2)}pt; left: 0; right: 0; height: ${lineHPt.toFixed(2)}pt; line-height: ${lineHPt.toFixed(2)}pt; ${fontStyle}${offsetStyle} direction: ${line.is_rtl ? 'rtl' : 'ltr'}; font-feature-settings: 'kern' 1, 'mark' 1, 'mkmk' 1; text-rendering: optimizeLegibility; -webkit-font-smoothing: antialiased; white-space: nowrap !important; overflow: visible; contain: layout style;">${contentHtml}</div>`;
      }).join('\n');

      const emptyNotice = (!frame.lines || frame.lines.length === 0)
        ? `<div class="tok-frame-empty" style="font-size: 10px; color: var(--tok-text-muted); font-style: italic; text-align: center; padding-top: 15px;">[אין טקסט בעמוד זה]</div>`
        : '';

      return `
        <div class="tok-interactive-frame tok-frame-rust ${flowClass}"
             data-frame-id="${escapeHtml(frameId)}"
             data-flow-id="${escapeHtml(flowId)}"
             data-base-flow-id="${escapeHtml(baseFlowId)}"
             data-is-expansion="${isExpansion ? 'true' : 'false'}"
             data-rect-x="${frame.rect.x}"
             data-rect-y="${frame.rect.y}"
             data-rect-w="${frame.rect.width}"
             data-rect-h="${frame.rect.height}"
             style="position: absolute; left: ${leftPt.toFixed(2)}pt; top: ${topPt.toFixed(2)}pt; width: ${widthPt.toFixed(2)}pt; height: ${heightPt.toFixed(2)}pt; overflow: hidden;">
          <div class="tok-frame-inner" style="position: relative; width: 100%; height: 100%;">
            ${separatorRuleHtml}
            ${linesHtml}
            ${emptyNotice}
          </div>
        </div>
      `;
    }).join('\n');

    const htmlContent = `
      <div class="tok-page-layout-rust" data-page-index="${pageIndex}" style="position: relative; width: 100%; height: 100%; overflow: hidden;">
        ${framesHtml}
      </div>
    `;

    return {
      pageIndex,
      gematriaNumber,
      widthPt: pageWidth,
      heightPt: pageHeight,
      htmlContent
    };
  });
}

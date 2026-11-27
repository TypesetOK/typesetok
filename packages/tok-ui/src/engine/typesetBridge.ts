/**
 * Typeset Bridge
 *
 * Maps Rust `PageLayoutBox` geometry produced by `tok-cli typeset-document`
 * (and `TypesettingEngine::typeset_document`) to UI `PageDescriptor` objects
 * rendered natively in SpreadCanvas.
 */

import { PageDescriptor } from 'tok-viewer';
import { toHebrewGematria } from '../gematria.js';
import { ColorJson, colorJsonToCss } from './documentBridge.js';
import { LineRunLookup, locateLines, renderStyledLine, safeCssColor } from './lineRuns.js';

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
  /** Horizontal widening of an Ahalterm letter's shape (0.12 = 12% wider), set by justification Tier 2. */
  stretch?: number;
}

export interface VisualLineRun {
  text: string;
  font_family: string;
  font_size_pt: number;
  font_weight: number;
  italic?: boolean;
  underline?: boolean;
  color_css: string;
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
  alignment?: 'Justified' | 'Right' | 'Left' | 'Center';
  visual_runs?: VisualLineRun[];
  adjustment_ratio?: number | null;
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

export interface PlacedImageBox {
  asset_path: string;
  rect: PhysicalRect;
  intrinsic_width_px: number;
  intrinsic_height_px: number;
  effective_dpi_x: number;
  effective_dpi_y: number;
  paragraph_id?: string | null;
}

export interface PlacedOrnamentBox {
  ornament_id: string;
  asset_path?: string | null;
  rect: PhysicalRect;
  color: ColorJson;
}

export type DiagnosticSeverity = 'Info' | 'Warning' | 'Error';

export interface WaiverRecord {
  code: string;
  reason: string;
  approved_by?: string | null;
}

export interface LayoutDiagnostic {
  code: string;
  severity: DiagnosticSeverity;
  message: string;
  page_index: number;
  bottom_delta_pt?: number | null;
  waiver?: WaiverRecord | null;
}

export type PreflightStatus = 'Pass' | 'Warning' | 'Fail';

export interface PreflightCheckItem {
  id: string;
  category: string;
  title: string;
  status: PreflightStatus;
  summary: string;
  details?: string[];
}

export interface MissingFontDiagnostic {
  family: string;
  missing_family: boolean;
  missing_hebrew_coverage: boolean;
  missing_niqqud_coverage?: boolean;
  missing_taamim_coverage?: boolean;
  permission_error?: string | null;
}

export interface PreflightIssue {
  code: string;
  check_id: string;
  severity: PreflightStatus;
  page_number?: number | null;
  location?: string | null;
  message: string;
  message_he: string;
  suggestion?: string | null;
  suggestion_he?: string | null;
}

export interface PreflightReport {
  overall_status: PreflightStatus;
  profile_name: string;
  pdfx_standard: string;
  total_pages: number;
  checks: PreflightCheckItem[];
  missing_fonts?: MissingFontDiagnostic[];
  low_dpi_images_count: number;
  balance_warnings_count: number;
  issues?: PreflightIssue[];
  error_count?: number;
  warning_count?: number;
  affected_pages?: number[];
}

export interface PageLayoutBox {
  page_index: number;
  page_number_gematria: string;
  dimensions: PhysicalRect;
  frames: TextFrameBox[];
  break_token?: BreakToken | null;
  is_blank?: boolean;
  column_rules?: [number, number, number, number][];
  images?: PlacedImageBox[];
  ornaments?: PlacedOrnamentBox[];
  diagnostics?: LayoutDiagnostic[];
}

/**
 * Reads a typeset IPC response: `layoutJson` is the engine output as text (either a page
 * array or `{ pages, preflight }`); older shells sent already-parsed `pages`/`preflight`.
 */
export function readTypesetResponse(res: { layoutJson?: string; pages?: unknown; preflight?: unknown }): {
  pages: PageLayoutBox[];
  preflight: PreflightReport | null;
} {
  const parsed: any = typeof res.layoutJson === 'string' ? JSON.parse(res.layoutJson) : res;
  if (Array.isArray(parsed)) return { pages: parsed, preflight: null };
  return {
    pages: Array.isArray(parsed?.pages) ? parsed.pages : [],
    preflight: (parsed?.preflight ?? null) as PreflightReport | null,
  };
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

function renderVisualRunsHtml(runs: VisualLineRun[], isPrimary: boolean): string {
  return runs.map((run) => {
    const css: string[] = [];
    if (run.font_family) {
      css.push(`font-family: ${formatEngineFont(run.font_family, isPrimary).fontFamily}`);
    }
    if (run.font_size_pt && Number.isFinite(run.font_size_pt)) {
      css.push(`font-size: ${run.font_size_pt.toFixed(2)}pt`);
    }
    if (run.font_weight) {
      css.push(`font-weight: ${run.font_weight >= 600 ? 700 : 400}`);
    }
    if (run.italic) css.push('font-style: italic');
    if (run.underline) css.push('text-decoration: underline');
    const safeColor = safeCssColor(run.color_css);
    if (safeColor) css.push(`color: ${safeColor}`);
    const piece = escapeHtml(run.text || '');
    return css.length > 0 ? `<span style="${css.join('; ')};">${piece}</span>` : piece;
  }).join('');
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
export function layoutToPageDescriptors(pages: PageLayoutBox[], lineRuns?: LineRunLookup): PageDescriptor[] {
  if (!Array.isArray(pages) || pages.length === 0) {
    return [];
  }

  // The engine reports line text only. Where each line sits inside its paragraph tells us
  // which formatting (italic, underline, colour, ...) to paint on which part of it.
  const locatedLines = lineRuns
    ? locateLines(pages.flatMap((p) => (p.frames || []).flatMap((f) => f.lines || [])), lineRuns)
    : null;

  return pages.map((pageBox) => {
    const pageIndex = typeof pageBox.page_index === 'number' ? pageBox.page_index : 0;
    const gematriaNumber = pageBox.page_number_gematria || toHebrewGematria(pageIndex + 1);
    const pageWidth = pageBox.dimensions?.width || 595.28;
    const pageHeight = pageBox.dimensions?.height || 841.89;

    if (pageBox.is_blank) {
      return {
        pageIndex,
        gematriaNumber: '',
        widthPt: pageWidth,
        heightPt: pageHeight,
        htmlContent: `
          <div class="tok-page-layout-rust tok-page-blank" data-page-index="${pageIndex}" style="position: relative; width: 100%; height: 100%; overflow: hidden;">
            <div class="tok-blank-page-notice" style="position: absolute; top: 50%; left: 0; right: 0; text-align: center; color: var(--tok-text-muted, #888); font-size: 11px; font-style: italic; opacity: 0.5;">[עמוד חלק]</div>
          </div>
        `
      };
    }

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

      // Lines that reach the column measure are justified; the page view stretches them to match.
      const frameMaxLineW = Math.max(0, ...(frame.lines || []).map((l) => l.width || 0));
      const linesHtml = (frame.lines || []).map((line) => {
        const lineMeasure = Math.max(1, widthPt - (line.x_offset || 0));
        const lineAlign = line.alignment ?? 'Justified';
        const isJustifiedLine =
          lineAlign === 'Justified' &&
          (line.width || 0) > 0 &&
          (line.width >= frameMaxLineW - 0.5 || line.width >= lineMeasure * 0.985) &&
          /\S\s+\S/.test(line.text || '');
        const textAlignCss =
          lineAlign === 'Center'
            ? 'center'
            : lineAlign === 'Left'
            ? 'left'
            : lineAlign === 'Right'
            ? 'right'
            : line.is_rtl
            ? 'right'
            : 'left';
        // Engine-exact widths for the page view: justified spaces and widened Ahalterm letters,
        // both in logical (cluster) order. See engine/lineFit.ts.
        let fitAttrs = '';
        if (isJustifiedLine && line.glyphs && line.glyphs.length > 0) {
          const round = (n: number, d: number) => String(Math.round(n * 10 ** d) / 10 ** d);
          const byCluster = (a: GlyphBox, b: GlyphBox) => a.cluster - b.cluster;
          const sp = line.glyphs.filter((g) => g.character === ' ').sort(byCluster).map((g) => round(g.width, 3));
          const ah = line.glyphs
            .filter((g) => (g.stretch || 0) > 0)
            .sort(byCluster)
            .map((g) => `${round(g.width, 3)},${round(g.stretch || 0, 4)}`);
          if (sp.length > 0) fitAttrs += ` data-sp="${sp.join(';')}"`;
          if (ah.length > 0) fitAttrs += ` data-ah="${ah.join(';')}"`;
        }
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
        const boxBoundsStyle = line.x_offset
          ? `left: ${line.x_offset.toFixed(2)}pt; width: ${lineMeasure.toFixed(2)}pt;`
          : 'left: 0; right: 0;';
        const located = line.visual_runs?.length ? undefined : locatedLines?.get(line);
        const contentHtml = (line.visual_runs && line.visual_runs.length > 0)
          ? renderVisualRunsHtml(line.visual_runs, isPrimary)
          : ((located
            ? renderStyledLine(line.text, located, (family) => formatEngineFont(family, isPrimary).fontFamily)
            : null) ?? renderLineText(line.text, line.is_drop_cap));
        const paraColor = line.visual_runs?.length ? undefined : safeCssColor(located?.source.color);
        const colorStyle = paraColor ? ` color: ${paraColor};` : '';
        let hjAttrs = '';
        if (typeof line.adjustment_ratio === 'number' && Number.isFinite(line.adjustment_ratio)) {
          const absR = Math.abs(line.adjustment_ratio);
          const tier = absR <= 0.5 ? 'good' : absR <= 1.2 ? 'warn' : 'bad';
          hjAttrs = ` data-adj-ratio="${line.adjustment_ratio.toFixed(3)}" data-hj-tier="${tier}"`;
        }

        return `<div class="tok-line-box tok-line" data-line-index="${line.line_index}"${paraAttr} data-baseline-y="${line.baseline_y}" data-line-height="${line.height}" data-line-width="${line.width}" data-align="${lineAlign}"${isJustifiedLine ? ' data-justify="1"' : ''}${hjAttrs}${fitAttrs} style="position: absolute; top: ${topOffsetPt.toFixed(2)}pt; ${boxBoundsStyle} height: ${lineHPt.toFixed(2)}pt; line-height: ${lineHPt.toFixed(2)}pt; text-align: ${textAlignCss}; ${fontStyle}${colorStyle} direction: ${line.is_rtl ? 'rtl' : 'ltr'}; font-feature-settings: 'kern' 1, 'mark' 1, 'mkmk' 1; text-rendering: optimizeLegibility; -webkit-font-smoothing: antialiased; white-space: nowrap !important; overflow: visible; contain: layout style;">${contentHtml}</div>`;
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

const BUILTIN_ORNAMENT_SVGS: Record<string, string> = {
  crown_torah: `<svg viewBox="0 0 120 40" width="100%" height="100%" aria-hidden="true"><path d="M18 32 L10 14 L34 22 L60 6 L86 22 L110 14 L102 32 Z" fill="currentColor"/><rect x="20" y="34" width="80" height="3" rx="1.5" fill="currentColor"/><circle cx="60" cy="5" r="3" fill="currentColor"/><circle cx="10" cy="12" r="2.2" fill="currentColor"/><circle cx="110" cy="12" r="2.2" fill="currentColor"/></svg>`,
  menorah_divider: `<svg viewBox="0 0 120 32" width="100%" height="100%" aria-hidden="true"><line x1="6" y1="16" x2="42" y2="16" stroke="currentColor" stroke-width="1.4"/><line x1="78" y1="16" x2="114" y2="16" stroke="currentColor" stroke-width="1.4"/><path d="M60 4 L60 26 M48 26 L72 26 M52 8 Q52 17 60 17 Q68 17 68 8 M45 8 Q45 21 60 21 Q75 21 75 8" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>`,
  floral_parasha: `<svg viewBox="0 0 100 32" width="100%" height="100%" aria-hidden="true"><path d="M50 4 C56 10 56 22 50 28 C44 22 44 10 50 4 Z M38 16 C44 10 56 10 62 16 C56 22 44 22 38 16 Z" fill="currentColor"/><path d="M12 16 Q24 9 34 16 Q24 23 12 16 Z M88 16 Q76 9 66 16 Q76 23 88 16 Z" fill="none" stroke="currentColor" stroke-width="1.4"/><circle cx="50" cy="16" r="2.5" fill="currentColor"/></svg>`,
  star_tailpiece: `<svg viewBox="0 0 60 60" width="100%" height="100%" aria-hidden="true"><polygon points="30,6 51,42 9,42" fill="none" stroke="currentColor" stroke-width="2"/><polygon points="30,54 9,18 51,18" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="30" cy="30" r="3.5" fill="currentColor"/></svg>`,
  folio_cartouche: `<svg viewBox="0 0 120 36" width="100%" height="100%" aria-hidden="true"><path d="M12 18 Q20 4 36 6 L84 6 Q100 4 108 18 Q100 32 84 30 L36 30 Q20 32 12 18 Z" fill="none" stroke="currentColor" stroke-width="1.3"/><circle cx="8" cy="18" r="2.2" fill="currentColor"/><circle cx="112" cy="18" r="2.2" fill="currentColor"/></svg>`
};

    const columnRulesHtml = (pageBox.column_rules || [])
      .map(([x1, y1, _x2, y2]) => {
        const heightPt = Math.max(0, y2 - y1);
        return `<div class="tok-column-rule" style="position: absolute; left: ${x1.toFixed(2)}pt; top: ${y1.toFixed(2)}pt; width: 0.4pt; height: ${heightPt.toFixed(2)}pt; background-color: rgba(40, 40, 40, 0.45); pointer-events: none;"></div>`;
      })
      .join('\n');

    const imagesHtml = (pageBox.images || [])
      .map((img) => {
        const dpi = Math.round(Math.min(img.effective_dpi_x, img.effective_dpi_y));
        const lowDpiClass = dpi > 0 && dpi < 300 ? ' tok-low-dpi' : '';
        return `<div class="tok-placed-image${lowDpiClass}" data-asset-path="${escapeHtml(img.asset_path)}" data-dpi="${dpi}" style="position: absolute; left: ${img.rect.x.toFixed(2)}pt; top: ${img.rect.y.toFixed(2)}pt; width: ${img.rect.width.toFixed(2)}pt; height: ${img.rect.height.toFixed(2)}pt; border: 0.5pt dashed rgba(100,116,139,0.55); background: rgba(148,163,184,0.08); display: flex; align-items: center; justify-content: center; overflow: hidden; box-sizing: border-box; pointer-events: none;"><span style="font-size: 7pt; color: #475569; direction: ltr; text-align: center; padding: 2pt;">${escapeHtml(img.asset_path)} (${dpi} DPI)</span></div>`;
      })
      .join('\n');

    const ornamentsHtml = (pageBox.ornaments || [])
      .map((orn) => {
        const cssColor = safeCssColor(colorJsonToCss(orn.color)) || '#1C1B18';
        const svgInner = BUILTIN_ORNAMENT_SVGS[orn.ornament_id] || `<svg viewBox="0 0 80 20" width="100%" height="100%" aria-hidden="true"><line x1="4" y1="10" x2="76" y2="10" stroke="currentColor" stroke-width="1.2"/></svg>`;
        return `<div class="tok-placed-ornament" data-ornament-id="${escapeHtml(orn.ornament_id)}" style="position: absolute; left: ${orn.rect.x.toFixed(2)}pt; top: ${orn.rect.y.toFixed(2)}pt; width: ${orn.rect.width.toFixed(2)}pt; height: ${orn.rect.height.toFixed(2)}pt; color: ${cssColor}; display: flex; align-items: center; justify-content: center; pointer-events: none;">${svgInner}</div>`;
      })
      .join('\n');

    const htmlContent = `
      <div class="tok-page-layout-rust" data-page-index="${pageIndex}" style="position: relative; width: 100%; height: 100%; overflow: hidden;">
        ${columnRulesHtml}
        ${imagesHtml}
        ${ornamentsHtml}
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

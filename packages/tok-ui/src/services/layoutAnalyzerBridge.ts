/**
 * TypeScript bridge for the Rust `tok-layout-analyzer` Tauri command (`analyze_page_layout`).
 */

import { TemplateType } from '../engine/FlowPaginator';

export type PageForm =
  | 'standard_prose'
  | 'prose_with_footnotes'
  | 'tzurat_hadaf'
  | 'multi_commentary_grid'
  | 'multi_column_modern'
  | 'title_or_outlier';

export interface RecommendedFlowRole {
  flowId: string;
  roleName: string;
  widthRatio?: number;
}

export interface SuggestedSectionConfig {
  layoutKind?: string;
  columnProportions?: number[];
  columnCount?: number;
  columnGutterPt?: number;
  drawColumnRule: boolean;
  flowRoles: RecommendedFlowRole[];
}

export interface LayoutAlternative {
  pageForm: PageForm;
  templateType?: TemplateType;
  confidence: number;
}

export interface LayoutRecommendation {
  pageForm: PageForm;
  recommendedTemplateType?: TemplateType;
  displayNameHe: string;
  confidence: number;
  alternatives: LayoutAlternative[];
  suggestedSectionConfig: SuggestedSectionConfig;
  analyzedPageCount: number;
  outlierPageCount: number;
  rectoVersoVerified: boolean;
  usedMlFallback: boolean;
}

/**
 * Invokes the Tauri backend `analyze_page_layout` command on a file path.
 * Returns `null` when running in a pure browser environment without Tauri IPC.
 */
export async function analyzePageLayout(
  filePath: string,
  startPage = 1,
  endPage = 10,
): Promise<LayoutRecommendation | null> {
  const win = window as unknown as {
    __TAURI_INTERNALS__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
    };
  };
  if (!win.__TAURI_INTERNALS__?.invoke) {
    return null;
  }
  return win.__TAURI_INTERNALS__.invoke<LayoutRecommendation>('analyze_page_layout', {
    filePath,
    startPage,
    endPage,
  });
}

/**
 * Formats confidence as a rounded percentage string (e.g. `"94%"`).
 */
export function formatConfidencePercent(confidence: number): string {
  const pct = Math.round(Math.max(0, Math.min(1, confidence)) * 100);
  return `${pct}%`;
}

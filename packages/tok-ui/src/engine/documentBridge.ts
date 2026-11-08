/**
 * TypesetOK Document Model Bridge (UI <-> Rust Contract).
 *
 * Provides bidirectional lossless conversion between:
 * - UI State: `MultiFlowDocumentState` (flows of `StoryParagraph`s, template type, title)
 * - Rust Model: `DocumentRoot` JSON (sections, flows, paragraphs with ULID NodeIds,
 *   FractionalIndex order keys, typographic ParagraphStyles, and DocumentMetadata).
 *
 * Implements strict compliance with schemas/document/tok_document_schema.json
 * and tok-core model specifications.
 */

import type { MultiFlowDocumentState, TemplateType } from './FlowPaginator.js';
import {
  CharacterFormat,
  ParagraphStylePatch,
  StoryParagraph,
  TextSpan
} from 'tok-story-editor';
import type {
  DocumentSettings,
  FootnoteApparatusConfigData,
  GrepRuleData,
  MultiFlowAnchorLink
} from '../types.js';
import type { ImportResult } from './textImporter.js';

export const CROCKFORD_ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';


/**
 * Validates whether a string is a 26-character Crockford Base32 ULID
 * matching tok-core Ulid::from_string (first character <= '7').
 */
export function isValidUlid(s: unknown): boolean {
  if (typeof s !== 'string' || s.length !== 26) return false;
  return /^[0-7][0123456789ABCDEFGHJKMNPQRSTVWXYZabcdefghjkmnpqrstvwxyz]{25}$/.test(s);
}

/**
 * Generates a valid 26-character Crockford Base32 ULID.
 * Formatted as: 10 chars timestamp (48 bits) + 16 chars entropy (80 bits).
 */
export function generateCrockfordUlid(seedTime: number = Date.now()): string {
  let timeStr = '';
  let t = Math.max(0, Math.floor(seedTime));
  for (let i = 0; i < 10; i++) {
    const mod = t % 32;
    timeStr = CROCKFORD_ALPHABET[mod] + timeStr;
    t = Math.floor(t / 32);
  }

  let randStr = '';
  for (let i = 0; i < 16; i++) {
    const rand = Math.floor(Math.random() * 32);
    randStr += CROCKFORD_ALPHABET[rand];
  }

  return timeStr + randStr;
}

/**
 * Strips HTML formatting tags (like <b>, </i>, etc.) while preserving
 * unicode Hebrew letters, niqqud, and punctuation.
 */
export function stripHtmlTags(text: string): string {
  if (!text) return '';
  return text.replace(/<\/?[^>]+(>|$)/g, '');
}

/**
 * Generates a lexicographically sorted fractional index key for order preservation.
 */
export function fractionalIndexFor(position: number): string {
  return 'a' + position.toString(36).padStart(5, '0');
}

// ---------------------------------------------------------------------------
// Rust DocumentRoot JSON Schema Types
// ---------------------------------------------------------------------------

export interface ColorJson {
  DeviceCmyk?: {
    c: number;
    m: number;
    y: number;
    k: number;
  };
  Rgb?: {
    r: number;
    g: number;
    b: number;
  };
}

export interface ParagraphStyleJson {
  id: string;
  name: string;
  font_family: string;
  font_weight: number;
  font_size_pt: number;
  line_height_pt: number;
  space_before_pt: number;
  space_after_pt: number;
  first_line_indent_pt: number;
  alignment: 'Justified' | 'Right' | 'Left' | 'Center';
  ahalterm_stretch: 'AutoVariable' | 'Off' | 'StaticSwash';
  color: ColorJson;
  keep_with_next: boolean;
  orphan_control: number;
  widow_control: number;
}

export interface StylePatchJson {
  font_family?: string;
  font_size_pt?: number;
  line_height_pt?: number;
  alignment?: 'Justified' | 'Right' | 'Left' | 'Center';
  tracking_em?: number;
  color?: ColorJson;
  font_weight?: number;
  space_before_pt?: number;
  space_after_pt?: number;
  first_line_indent_pt?: number;
  indent_start_pt?: number;
  indent_end_pt?: number;
  direction?: 'Rtl' | 'Ltr';
}

export interface CharacterFormatJson {
  character_style_id?: string;
  font_family?: string;
  font_size_pt?: number;
  font_scale?: number;
  font_weight?: number;
  italic?: boolean;
  underline?: boolean;
  tracking_em?: number;
  color?: ColorJson;
  link_url?: string;
}

export type FootnoteApparatusConfigJson = FootnoteApparatusConfigData;
export type GrepRuleJson = GrepRuleData;

/**
 * Built-in Torah & scholarly DTP Live GREP presets matching `GrepRule::builtin_presets()` in `tok-core`.
 */
export function getBuiltinGrepRules(): GrepRuleJson[] {
  return [
    {
      id: 'grep-parens-citations',
      name: 'הקטנת מראי מקומות בסוגריים (80% + אפור עדין)',
      pattern: '\\([^()]+\\)',
      target_style_id: 'char-citation-parens',
      format: {
        character_style_id: 'char-citation-parens',
        font_scale: 0.80,
        color: { DeviceCmyk: { c: 0.0, m: 0.0, y: 0.0, k: 0.65 } }
      },
      enabled: false
    },
    {
      id: 'grep-acronyms-rashi',
      name: 'ראשי תיבות בגרשיים (גופן רש״י / הדגשה)',
      pattern: '[\\u0590-\\u05FF]+["״][\\u0590-\\u05FF]+',
      target_style_id: 'char-acronym',
      format: {
        character_style_id: 'char-acronym',
        font_family: 'Noto Rashi Hebrew',
        font_weight: 700
      },
      enabled: false
    },
    {
      id: 'grep-talmud-opening-words',
      name: 'הדגשת מילות פתיחה תלמודיות (אמר רבא, תנו רבנן)',
      pattern: '^(?:אמר רבא|אמר אביי|תנו רבנן|תנא|איתמר|בעא מיניה|גמרא|משנה)\\b',
      target_style_id: 'char-opening-talmud',
      format: {
        character_style_id: 'char-opening-talmud',
        font_weight: 700
      },
      enabled: false
    },
    {
      id: 'grep-section-markers',
      name: 'הדגשת מספרי סעיפים ואותיות (אות א׳, סעיף ב׳)',
      pattern: '(?:אות|סעיף|סימן|הלכה|ענף)\\s+[\\u05D0-\\u05EA]{1,3}[\'׳"]?',
      target_style_id: 'char-section-marker',
      format: {
        character_style_id: 'char-section-marker',
        font_weight: 700
      },
      enabled: false
    },
    {
      id: 'grep-double-quotes-emphasis',
      name: 'עיצוב מובאות במירכאות כפולות',
      pattern: '״[^״]+״|"[^"\\s][^"]*"',
      target_style_id: 'char-quoted',
      format: {
        character_style_id: 'char-quoted',
        italic: true
      },
      enabled: false
    }
  ];
}

/**
 * Default dual-apparatus footnotes configuration for Torah & Responsa books:
 * - Apparatus 1 (`notes`): "מקורות וציונים" (Hebrew letters א, ב, ג in 2 columns)
 * - Apparatus 2 (`biurim`): "ביאורים והרחבות" (Arabic numerals 1, 2, 3 in 1 wide column)
 */
export function getDefaultMultiApparatusConfigs(): FootnoteApparatusConfigJson[] {
  return [
    {
      id: 'notes',
      name: 'מדור א׳: מקורות וציונים',
      numbering_style: 'HebrewGematria',
      columns: 2,
      column_gap_pt: 12.0,
      max_height_ratio: 0.25,
      draw_separator_rule: true,
      separator_length_ratio: 0.33,
      font_family: 'Frank Ruhl Libre',
      font_size_pt: 8.5,
      line_height_pt: 11.5
    },
    {
      id: 'biurim',
      name: 'מדור ב׳: ביאורים והרחבות',
      numbering_style: 'ArabicNumerals',
      columns: 1,
      column_gap_pt: 12.0,
      max_height_ratio: 0.45,
      draw_separator_rule: true,
      separator_length_ratio: 0.18,
      font_family: 'Frank Ruhl Libre',
      font_size_pt: 9.5,
      line_height_pt: 13.0
    }
  ];
}

export interface TextSpanJson {
  start: number;
  end: number;
  format: CharacterFormatJson;
}

export type HierarchyLevelJson =
  | 'Volume'
  | 'Part'
  | 'Chapter'
  | 'Section'
  | 'Subsection'
  | 'Clause'
  | { Custom: string };

export type GematriaPunctuationJson = 'StandardPunctuation' | 'PlainLetters';

export type NumberingScopeJson = 'ContinuousBook' | 'PerSection' | 'PerPage';

export type NumberingSchemeJson =
  | 'Arabic'
  | { HebrewGematria: GematriaPunctuationJson }
  | { TorahSiman: GematriaPunctuationJson }
  | { TorahSeif: GematriaPunctuationJson }
  | { TorahOt: GematriaPunctuationJson }
  | { TorahDafAmud: GematriaPunctuationJson }
  | 'RomanLower'
  | 'RomanUpper'
  | { Custom: { prefix: string; base: NumberingSchemeJson; suffix: string } };

export interface DocumentHierarchyNodeJson {
  id: string;
  level: HierarchyLevelJson;
  title: string;
  number_label?: string;
  numbering_scheme?: NumberingSchemeJson;
  numbering_scope?: NumberingScopeJson;
  section_id?: string;
  start_anchor?: { node_id: string; offset_key: string; bias: 'Before' | 'After' };
  target_paragraph_id?: string;
  children: DocumentHierarchyNodeJson[];
}

export interface DocumentOutlineJson {
  root_nodes: DocumentHierarchyNodeJson[];
}

export interface LevelNumberingRuleJson {
  level: HierarchyLevelJson;
  scheme: NumberingSchemeJson;
  scope?: NumberingScopeJson;
  prefix?: string;
  suffix?: string;
}

export interface OutlineNumberingConfigJson {
  level_rules: LevelNumberingRuleJson[];
  punctuation: GematriaPunctuationJson;
}

/**
 * Converts a positive integer into traditional Hebrew gematria.
 * Supports:
 * - 1-9: א-ט
 * - 10-90: י-צ
 * - 100-900: ק-ת, תק, תר, תש, תת, תתק
 * - Traditional substitutions for 15 (ט״ו) and 16 (ט״ז)
 * - StandardPunctuation (single letter gets ׳, multi-letter gets ״ before last letter)
 * - PlainLetters (pure letters without quote marks)
 */
export function formatHebrewGematria(
  num: number,
  punct: GematriaPunctuationJson = 'StandardPunctuation'
): string {
  if (num <= 0 || !Number.isFinite(num)) return '';

  const thousands = Math.floor(num / 1000);
  const remainder = num % 1000;

  function toGematriaSub1000(n: number): string {
    let rem = n;
    let res = '';

    while (rem >= 400) {
      res += 'ת';
      rem -= 400;
    }
    if (rem >= 300) {
      res += 'ש';
      rem -= 300;
    } else if (rem >= 200) {
      res += 'ר';
      rem -= 200;
    } else if (rem >= 100) {
      res += 'ק';
      rem -= 100;
    }

    if (rem === 15) {
      res += 'טו';
      return res;
    }
    if (rem === 16) {
      res += 'טז';
      return res;
    }

    const tens = [
      [90, 'צ'],
      [80, 'פ'],
      [70, 'ע'],
      [60, 'ס'],
      [50, 'נ'],
      [40, 'מ'],
      [30, 'ל'],
      [20, 'כ'],
      [10, 'י']
    ] as const;
    for (const [val, ch] of tens) {
      if (rem >= val) {
        res += ch;
        rem -= val;
        break;
      }
    }

    const units = [
      [9, 'ט'],
      [8, 'ח'],
      [7, 'ז'],
      [6, 'ו'],
      [5, 'ה'],
      [4, 'ד'],
      [3, 'ג'],
      [2, 'ב'],
      [1, 'א']
    ] as const;
    for (const [val, ch] of units) {
      if (rem >= val) {
        res += ch;
        break;
      }
    }

    return res;
  }

  if (punct === 'PlainLetters') {
    let s = '';
    if (thousands > 0) s += toGematriaSub1000(thousands);
    if (remainder > 0) s += toGematriaSub1000(remainder);
    return s;
  }

  let s = '';
  if (thousands > 0) {
    s += toGematriaSub1000(thousands) + '׳';
  }
  if (remainder > 0) {
    const chars = Array.from(toGematriaSub1000(remainder));
    if (chars.length === 1) {
      s += chars[0] + '׳';
    } else {
      for (let i = 0; i < chars.length; i++) {
        if (i === chars.length - 1) {
          s += '״';
        }
        s += chars[i];
      }
    }
  }
  return s;
}

/**
 * Formats an ordinal value (1-based) according to a numbering scheme.
 */
export function formatTorahNumbering(
  scheme: NumberingSchemeJson,
  value: number,
  defaultPunct: GematriaPunctuationJson = 'StandardPunctuation'
): string {
  if (value === 0) return '0';

  if (scheme === 'Arabic') {
    return String(value);
  }
  if (typeof scheme === 'object') {
    if ('HebrewGematria' in scheme) {
      return formatHebrewGematria(value, scheme.HebrewGematria || defaultPunct);
    }
    if ('TorahSiman' in scheme) {
      return `סימן ${formatHebrewGematria(value, scheme.TorahSiman || defaultPunct)}`;
    }
    if ('TorahSeif' in scheme) {
      return `סעיף ${formatHebrewGematria(value, scheme.TorahSeif || defaultPunct)}`;
    }
    if ('TorahOt' in scheme) {
      return `אות ${formatHebrewGematria(value, scheme.TorahOt || defaultPunct)}`;
    }
    if ('TorahDafAmud' in scheme) {
      const p = scheme.TorahDafAmud || defaultPunct;
      const dafNumber = Math.floor((value - 1) / 2) + 2;
      const isAmudB = (value - 1) % 2 === 1;
      const dafStr = formatHebrewGematria(dafNumber, p);
      const amudStr = isAmudB
        ? p === 'StandardPunctuation'
          ? 'ע״ב'
          : 'עב'
        : p === 'StandardPunctuation'
        ? 'ע״א'
        : 'עא';
      return `דף ${dafStr} ${amudStr}`;
    }
    if ('Custom' in scheme) {
      const c = scheme.Custom;
      return `${c.prefix}${formatTorahNumbering(c.base, value, defaultPunct)}${c.suffix}`;
    }
  }
  if (scheme === 'RomanUpper' || scheme === 'RomanLower') {
    const lookup: [number, string][] = [
      [1000, 'M'],
      [900, 'CM'],
      [500, 'D'],
      [400, 'CD'],
      [100, 'C'],
      [90, 'XC'],
      [50, 'L'],
      [40, 'XL'],
      [10, 'X'],
      [9, 'IX'],
      [5, 'V'],
      [4, 'IV'],
      [1, 'I']
    ];
    let rem = value;
    let roman = '';
    for (const [val, sym] of lookup) {
      while (rem >= val) {
        roman += sym;
        rem -= val;
      }
    }
    return scheme === 'RomanLower' ? roman.toLowerCase() : roman;
  }
  return String(value);
}

/**
 * Creates canonical Torah book numbering defaults matching tok-core OutlineNumberingConfig.
 */
export function getDefaultTorahOutlineNumberingConfig(
  punctuation: GematriaPunctuationJson = 'StandardPunctuation'
): OutlineNumberingConfigJson {
  return {
    level_rules: [
      {
        level: 'Part',
        scheme: {
          Custom: {
            prefix: 'חלק ',
            base: { HebrewGematria: punctuation },
            suffix: ''
          }
        },
        scope: 'ContinuousBook'
      },
      {
        level: 'Chapter',
        scheme: { TorahSiman: punctuation },
        scope: 'ContinuousBook'
      },
      {
        level: 'Section',
        scheme: { TorahSeif: punctuation },
        scope: 'PerSection'
      },
      {
        level: 'Subsection',
        scheme: { TorahOt: punctuation },
        scope: 'PerSection'
      }
    ],
    punctuation
  };
}

/**
 * Dynamically recalculates number labels for all nodes in the document outline tree.
 */
export function recalculateOutlineNumbers(
  outline: DocumentOutlineJson,
  config: OutlineNumberingConfigJson = getDefaultTorahOutlineNumberingConfig()
): DocumentOutlineJson {
  const globalCounters = new Map<string, number>();

  function levelKey(lvl: HierarchyLevelJson): string {
    return typeof lvl === 'string' ? lvl : `Custom:${lvl.Custom}`;
  }

  function getRule(lvl: HierarchyLevelJson): LevelNumberingRuleJson | undefined {
    const key = levelKey(lvl);
    return config.level_rules.find((r) => levelKey(r.level) === key);
  }

  function processSlice(nodes: DocumentHierarchyNodeJson[]): void {
    const localCounters = new Map<string, number>();

    for (const node of nodes) {
      const rule = getRule(node.level);
      const scheme = node.numbering_scheme || rule?.scheme;
      const scope = node.numbering_scope || rule?.scope || 'PerSection';

      if (scheme) {
        const key = levelKey(node.level);
        let ordinal = 1;
        if (scope === 'ContinuousBook') {
          const current = (globalCounters.get(key) || 0) + 1;
          globalCounters.set(key, current);
          ordinal = current;
        } else {
          const current = (localCounters.get(key) || 0) + 1;
          localCounters.set(key, current);
          ordinal = current;
        }

        node.number_label = formatTorahNumbering(scheme, ordinal, config.punctuation);
      }

      if (node.children && node.children.length > 0) {
        processSlice(node.children);
      }
    }
  }

  if (outline?.root_nodes) {
    processSlice(outline.root_nodes);
  }

  return outline;
}

export function removeOutlineNode(
  outline: DocumentOutlineJson,
  nodeId: string,
  config?: OutlineNumberingConfigJson
): DocumentHierarchyNodeJson | null {
  function searchAndRemove(nodes: DocumentHierarchyNodeJson[]): DocumentHierarchyNodeJson | null {
    const idx = nodes.findIndex((n) => n.id === nodeId);
    if (idx !== -1) {
      return nodes.splice(idx, 1)[0];
    }
    for (const n of nodes) {
      if (n.children) {
        const found = searchAndRemove(n.children);
        if (found) return found;
      }
    }
    return null;
  }

  const removed = searchAndRemove(outline.root_nodes);
  if (removed && config) {
    recalculateOutlineNumbers(outline, config);
  }
  return removed;
}

export function insertOutlineNode(
  outline: DocumentOutlineJson,
  parentId: string | null | undefined,
  targetIndex: number,
  node: DocumentHierarchyNodeJson,
  config?: OutlineNumberingConfigJson
): void {
  function findNode(
    nodes: DocumentHierarchyNodeJson[],
    id: string
  ): DocumentHierarchyNodeJson | null {
    for (const n of nodes) {
      if (n.id === id) return n;
      if (n.children) {
        const found = findNode(n.children, id);
        if (found) return found;
      }
    }
    return null;
  }

  if (parentId) {
    const parent = findNode(outline.root_nodes, parentId);
    if (parent) {
      if (!parent.children) parent.children = [];
      const idx = Math.min(Math.max(0, targetIndex), parent.children.length);
      parent.children.splice(idx, 0, node);
    } else {
      const idx = Math.min(Math.max(0, targetIndex), outline.root_nodes.length);
      outline.root_nodes.splice(idx, 0, node);
    }
  } else {
    const idx = Math.min(Math.max(0, targetIndex), outline.root_nodes.length);
    outline.root_nodes.splice(idx, 0, node);
  }

  if (config) {
    recalculateOutlineNumbers(outline, config);
  }
}

export function isOutlineDescendant(
  outline: DocumentOutlineJson,
  descendantId: string,
  ancestorId: string
): boolean {
  if (descendantId === ancestorId) return false;
  function search(nodes: DocumentHierarchyNodeJson[], targetId: string, path: string[]): boolean {
    for (const n of nodes) {
      path.push(n.id);
      if (n.id === targetId) return true;
      if (n.children && search(n.children, targetId, path)) return true;
      path.pop();
    }
    return false;
  }
  const path: string[] = [];
  if (search(outline.root_nodes, descendantId, path)) {
    path.pop();
    return path.includes(ancestorId);
  }
  return false;
}

export function reorderOutlineNode(
  outline: DocumentOutlineJson,
  nodeId: string,
  newParentId: string | null | undefined,
  targetIndex: number,
  config?: OutlineNumberingConfigJson
): boolean {
  if (newParentId) {
    if (newParentId === nodeId || isOutlineDescendant(outline, newParentId, nodeId)) {
      return false;
    }
  }

  const removed = removeOutlineNode(outline, nodeId);
  if (!removed) return false;

  insertOutlineNode(outline, newParentId, targetIndex, removed, config);
  return true;
}

export function syncOutlineNumberingToStoryParagraphs(
  outline: DocumentOutlineJson,
  paragraphs: StoryParagraph[],
  config?: OutlineNumberingConfigJson
): Array<{ paragraphId: string; numberLabel: string }> {
  if (config) {
    recalculateOutlineNumbers(outline, config);
  }

  const labelMap = new Map<string, string>();
  function collect(nodes: DocumentHierarchyNodeJson[]): void {
    for (const n of nodes) {
      const pid = n.target_paragraph_id || n.start_anchor?.node_id;
      if (pid && n.number_label) {
        labelMap.set(pid, n.number_label);
      }
      if (n.children) collect(n.children);
    }
  }
  collect(outline.root_nodes);

  const affected: Array<{ paragraphId: string; numberLabel: string }> = [];
  for (const p of paragraphs) {
    const label = labelMap.get(p.id);
    if (label) {
      (p as any).numberLabel = label;
      affected.push({ paragraphId: p.id, numberLabel: label });
    }
  }
  return affected;
}

export interface ParagraphNodeJson {
  id: string;
  index: string;
  style_id: string;
  text: string;
  style_patches: StylePatchJson[];
  spans?: TextSpanJson[];
  heading_level?: HierarchyLevelJson;
  numbering_scheme?: NumberingSchemeJson;
  number_label?: string;
}

export function patchToStylePatchJson(patch: ParagraphStylePatch): StylePatchJson {
  const json: StylePatchJson = {};
  if (patch.fontFamily !== undefined) json.font_family = patch.fontFamily;
  if (patch.fontSizePt !== undefined) json.font_size_pt = patch.fontSizePt;
  if (patch.fontWeight !== undefined) json.font_weight = patch.fontWeight;
  if (patch.lineHeightPt !== undefined) json.line_height_pt = patch.lineHeightPt;
  if (patch.spaceBeforePt !== undefined) json.space_before_pt = patch.spaceBeforePt;
  if (patch.spaceAfterPt !== undefined) json.space_after_pt = patch.spaceAfterPt;
  if (patch.firstLineIndentPt !== undefined) json.first_line_indent_pt = patch.firstLineIndentPt;
  if (patch.indentStartPt !== undefined) json.indent_start_pt = patch.indentStartPt;
  if (patch.indentEndPt !== undefined) json.indent_end_pt = patch.indentEndPt;
  if (patch.alignment !== undefined) json.alignment = patch.alignment;
  if (patch.direction !== undefined) json.direction = patch.direction;
  return json;
}

export function stylePatchJsonToPatch(json: StylePatchJson): ParagraphStylePatch {
  const patch: ParagraphStylePatch = {};
  if (json.font_family != null) patch.fontFamily = json.font_family;
  if (json.font_size_pt != null) patch.fontSizePt = json.font_size_pt;
  if (json.font_weight != null) patch.fontWeight = json.font_weight;
  if (json.line_height_pt != null) patch.lineHeightPt = json.line_height_pt;
  if (json.space_before_pt != null) patch.spaceBeforePt = json.space_before_pt;
  if (json.space_after_pt != null) patch.spaceAfterPt = json.space_after_pt;
  if (json.first_line_indent_pt != null) patch.firstLineIndentPt = json.first_line_indent_pt;
  if (json.indent_start_pt != null) patch.indentStartPt = json.indent_start_pt;
  if (json.indent_end_pt != null) patch.indentEndPt = json.indent_end_pt;
  if (json.alignment != null) patch.alignment = json.alignment;
  if (json.direction != null) patch.direction = json.direction;
  return patch;
}

export function spanToSpanJson(span: TextSpan): TextSpanJson {
  const fmt = span.format || {};
  const fmtJson: CharacterFormatJson = {};
  if (fmt.characterStyleId != null) fmtJson.character_style_id = fmt.characterStyleId;
  if (fmt.fontFamily != null) fmtJson.font_family = fmt.fontFamily;
  if (fmt.fontSizePt != null) fmtJson.font_size_pt = fmt.fontSizePt;
  if (fmt.fontScale != null) fmtJson.font_scale = fmt.fontScale;
  if (fmt.fontWeight != null) fmtJson.font_weight = fmt.fontWeight;
  if (fmt.italic != null) fmtJson.italic = fmt.italic;
  if (fmt.underline != null) fmtJson.underline = fmt.underline;
  if (fmt.trackingEm != null) fmtJson.tracking_em = fmt.trackingEm;
  if (fmt.linkUrl != null) fmtJson.link_url = fmt.linkUrl;
  return {
    start: span.start,
    end: span.end,
    format: fmtJson
  };
}

export function spanJsonToSpan(json: TextSpanJson): TextSpan {
  const fmt = json.format || {};
  const format: CharacterFormat = {};
  if (fmt.character_style_id != null) format.characterStyleId = fmt.character_style_id;
  if (fmt.font_family != null) format.fontFamily = fmt.font_family;
  if (fmt.font_size_pt != null) format.fontSizePt = fmt.font_size_pt;
  if (fmt.font_scale != null) format.fontScale = fmt.font_scale;
  if (fmt.font_weight != null) format.fontWeight = fmt.font_weight;
  if (fmt.italic != null) format.italic = fmt.italic;
  if (fmt.underline != null) format.underline = fmt.underline;
  if (fmt.tracking_em != null) format.trackingEm = fmt.tracking_em;
  if (fmt.link_url != null) format.linkUrl = fmt.link_url;
  if (fmt.color?.DeviceCmyk) {
    const k = Math.min(1, Math.max(0, fmt.color.DeviceCmyk.k));
    const gray = Math.round((1 - k) * 255);
    format.color = `rgb(${gray}, ${gray}, ${gray})`;
  } else if (fmt.color?.Rgb) {
    const r = Math.round(fmt.color.Rgb.r * 255);
    const g = Math.round(fmt.color.Rgb.g * 255);
    const b = Math.round(fmt.color.Rgb.b * 255);
    format.color = `rgb(${r}, ${g}, ${b})`;
  }
  return {
    start: json.start,
    end: json.end,
    format
  };
}

/**
 * Evaluates active Live GREP rules on a paragraph's text and returns merged TextSpans
 * non-destructively (without modifying `paragraph.text`).
 */
export function evaluateLiveGrepSpansInUi(
  text: string,
  baseSpans: TextSpan[] | undefined,
  grepRules: GrepRuleJson[] | undefined,
  flowId?: string
): TextSpan[] {
  const result: TextSpan[] = baseSpans ? [...baseSpans] : [];
  if (!text || !grepRules || grepRules.length === 0) {
    return result;
  }

  for (const rule of grepRules) {
    if (!rule.enabled || !rule.pattern.trim()) continue;
    if (rule.scope_flow_id && flowId && rule.scope_flow_id !== flowId) continue;

    try {
      const regex = new RegExp(rule.pattern, 'gu');
      let match: RegExpExecArray | null;
      while ((match = regex.exec(text)) !== null) {
        if (match[0].length === 0) {
          regex.lastIndex++;
          continue;
        }
        // Convert UTF-16 code-unit index to Unicode code-point index
        const startChar = Array.from(text.slice(0, match.index)).length;
        const matchCharLen = Array.from(match[0]).length;
        const endChar = startChar + matchCharLen;
        if (endChar > startChar) {
          const spanFmt = spanJsonToSpan({
            start: startChar,
            end: endChar,
            format: rule.format
          }).format;
          result.push({
            start: startChar,
            end: endChar,
            format: spanFmt
          });
        }
      }
    } catch {
      // Ignore invalid regex during live typing
    }
  }

  return result.sort((a, b) => a.start - b.start);
}


export type FlowTypeJson = 'Main' | 'CommentA' | 'CommentB' | 'Footnote';

export interface FlowJson {
  id: string;
  flow_type: FlowTypeJson;
  placement_role?: string;
  paragraphs: ParagraphNodeJson[];
}

export interface SectionNodeJson {
  id: string;
  name: string;
  page_style: string;
  flows: FlowJson[];
}

export interface DocumentMetadataJson {
  title: string;
  author: string;
  progression: 'Rtl' | 'Ltr';
  primary_language: string;
  schema_version: string;
  document_settings?: DocumentSettings;
}

export interface MultiFlowAnchorMapJson {
  anchors: Record<string, {
    id: string;
    primary_flow_id: string;
    primary_node_id: string;
    primary_anchor: {
      node_id: string;
      offset: string;
      bias: 'Before' | 'After';
    };
    label?: string;
  }>;
  primary_node_to_anchors: Record<string, string[]>;
  anchor_to_commentaries: Record<string, Array<{
    commentary_flow_id: string;
    commentary_node_id: string;
    anchor_id: string;
    priority: number;
  }>>;
  commentary_to_anchor: Record<string, string>;
}

export interface DocumentRootJson {
  id: string;
  metadata: DocumentMetadataJson;
  paragraph_styles: ParagraphStyleJson[];
  character_styles: unknown[];
  sections: SectionNodeJson[];
  outline?: DocumentOutlineJson;
  multi_flow_anchors?: MultiFlowAnchorMapJson;
  footnote_apparatuses?: FootnoteApparatusConfigJson[];
  grep_rules?: GrepRuleJson[];
}

/**
 * Creates canonical paragraph styles matching tok-core styles and UI fonts.
 */
export function createDefaultParagraphStyles(): ParagraphStyleJson[] {
  const cmykBlack: ColorJson = { DeviceCmyk: { c: 0, m: 0, y: 0, k: 1 } };
  return [
    {
      id: 'style-gemara-main',
      name: 'גמרא ראשי',
      font_family: 'Frank Ruhl Libre',
      font_weight: 700,
      font_size_pt: 13.5,
      line_height_pt: 20.9,
      space_before_pt: 0.0,
      space_after_pt: 2.0,
      first_line_indent_pt: 0.0,
      alignment: 'Justified',
      ahalterm_stretch: 'AutoVariable',
      color: cmykBlack,
      keep_with_next: false,
      orphan_control: 2,
      widow_control: 2
    },
    {
      id: 'style-rashi-body',
      name: 'רש״י',
      font_family: 'Noto Rashi Hebrew',
      font_weight: 400,
      font_size_pt: 10.5,
      line_height_pt: 15.5,
      space_before_pt: 0.0,
      space_after_pt: 2.0,
      first_line_indent_pt: 0.0,
      alignment: 'Justified',
      ahalterm_stretch: 'AutoVariable',
      color: cmykBlack,
      keep_with_next: false,
      orphan_control: 2,
      widow_control: 2
    },
    {
      id: 'style-tosafot-body',
      name: 'תוספות',
      font_family: 'Noto Rashi Hebrew',
      font_weight: 400,
      font_size_pt: 10.5,
      line_height_pt: 15.5,
      space_before_pt: 0.0,
      space_after_pt: 2.0,
      first_line_indent_pt: 0.0,
      alignment: 'Justified',
      ahalterm_stretch: 'AutoVariable',
      color: cmykBlack,
      keep_with_next: false,
      orphan_control: 2,
      widow_control: 2
    },
    {
      id: 'style-footnotes',
      name: 'הערות ומסורת',
      font_family: 'Frank Ruhl Libre',
      font_weight: 400,
      font_size_pt: 9.0,
      line_height_pt: 12.6,
      space_before_pt: 0.0,
      space_after_pt: 1.5,
      first_line_indent_pt: 0.0,
      alignment: 'Justified',
      ahalterm_stretch: 'AutoVariable',
      color: cmykBlack,
      keep_with_next: false,
      orphan_control: 2,
      widow_control: 2
    },
    {
      id: 'normal',
      name: 'גוף הטקסט רגיל',
      font_family: 'Frank Ruhl Libre',
      font_weight: 400,
      font_size_pt: 11.0,
      line_height_pt: 14.5,
      space_before_pt: 0.0,
      space_after_pt: 2.0,
      first_line_indent_pt: 0.0,
      alignment: 'Justified',
      ahalterm_stretch: 'AutoVariable',
      color: cmykBlack,
      keep_with_next: false,
      orphan_control: 2,
      widow_control: 2
    },
    {
      id: 'default-body',
      name: 'גוף הטקסט',
      font_family: 'Frank Ruhl Libre',
      font_weight: 400,
      font_size_pt: 11.0,
      line_height_pt: 14.5,
      space_before_pt: 0.0,
      space_after_pt: 2.0,
      first_line_indent_pt: 0.0,
      alignment: 'Justified',
      ahalterm_stretch: 'AutoVariable',
      color: cmykBlack,
      keep_with_next: false,
      orphan_control: 2,
      widow_control: 2
    }
  ];
}

/**
 * Converts UI `MultiFlowDocumentState` to Rust `DocumentRoot` JSON format.
 */
export function documentStateToDocumentRoot(
  state: MultiFlowDocumentState,
  options?: { author?: string }
): DocumentRootJson {
  const isProse = state.templateType === 'prose';
  const defaultStyles = createDefaultParagraphStyles();
  const definedStyleIds = new Set(defaultStyles.map((s) => s.id));

  const apparatusIds = new Set(
    (state.footnoteApparatuses || []).map((a) => a.id)
  );
  apparatusIds.add('notes');
  apparatusIds.add('footnotes');

  // Determine flow types
  const flowKeys = Object.keys(state.flows || {});
  const flows: FlowJson[] = [];

  for (const flowKey of flowKeys) {
    const paras = state.flows[flowKey] || [];
    const isApparatusFlow = apparatusIds.has(flowKey) || state.customFlowMetadata?.[flowKey]?.placement === 'bottom';
    if (isProse && flowKey !== 'gemara' && flowKey !== 'main' && !(isApparatusFlow && paras.length > 0)) {
      continue;
    }
    let flowId = flowKey;
    let flowType: FlowTypeJson = 'Main';
    let placementRole: string | undefined;

    if (flowKey === 'gemara') {
      flowId = isProse ? 'main' : 'gemara';
      flowType = 'Main';
    } else if (flowKey === 'rashi') {
      flowId = 'rashi';
      flowType = 'CommentA';
    } else if (flowKey === 'tosafot') {
      flowId = 'tosafot';
      flowType = 'CommentB';
    } else if (flowKey === 'notes' || isApparatusFlow) {
      flowId = flowKey;
      flowType = 'Footnote';
      placementRole = 'bottom_band';
    } else if (flowKey === 'main') {
      flowId = 'main';
      flowType = 'Main';
    }

    const convertedParas: ParagraphNodeJson[] = paras.map((p, idx) => {
      const pid = isValidUlid(p.id) ? p.id : generateCrockfordUlid();
      const styleId = p.styleId || (
        flowKey === 'gemara' ? (isProse ? 'normal' : 'style-gemara-main') :
        flowKey === 'rashi' ? 'style-rashi-body' :
        flowKey === 'tosafot' ? 'style-tosafot-body' :
        (flowKey === 'notes' || isApparatusFlow) ? 'style-footnotes' :
        'default-body'
      );

      // Auto-register unknown styles
      if (!definedStyleIds.has(styleId)) {
        defaultStyles.push({
          id: styleId,
          name: styleId,
          font_family: 'Frank Ruhl Libre',
          font_weight: 400,
          font_size_pt: 11.0,
          line_height_pt: 14.5,
          space_before_pt: 0.0,
          space_after_pt: 2.0,
          first_line_indent_pt: 0.0,
          alignment: 'Justified',
          ahalterm_stretch: 'AutoVariable',
          color: { DeviceCmyk: { c: 0, m: 0, y: 0, k: 1 } },
          keep_with_next: false,
          orphan_control: 2,
          widow_control: 2
        });
        definedStyleIds.add(styleId);
      }

      const stylePatches: StylePatchJson[] = [];
      if (p.patch) {
        const pj = patchToStylePatchJson(p.patch);
        if (Object.keys(pj).length > 0) {
          stylePatches.push(pj);
        }
      }

      const spansJson: TextSpanJson[] = (p.spans || []).map(spanToSpanJson);

      return {
        id: pid,
        index: fractionalIndexFor(idx),
        style_id: styleId,
        text: stripHtmlTags(p.text || ''),
        style_patches: stylePatches,
        spans: spansJson.length > 0 ? spansJson : undefined
      };
    });

    flows.push({
      id: flowId,
      flow_type: flowType,
      placement_role: placementRole,
      paragraphs: convertedParas
    });
  }

  // Ensure at least one flow exists even for empty documents
  if (flows.length === 0) {
    flows.push({
      id: isProse ? 'main' : 'gemara',
      flow_type: 'Main',
      paragraphs: []
    });
  }

  // Merge user custom styles from state if present
  if (state.styles) {
    for (const [sid, sty] of Object.entries(state.styles)) {
      const normStyle: ParagraphStyleJson = {
        id: sty.id || sid,
        name: sty.name || sid,
        font_family: (sty as any).fontFamily || sty.font_family || 'Frank Ruhl Libre',
        font_weight: (sty as any).fontWeight || sty.font_weight || 400,
        font_size_pt: (sty as any).fontSizePt || sty.font_size_pt || 12,
        line_height_pt: (sty as any).lineHeightPt || sty.line_height_pt || 14,
        space_before_pt: (sty as any).spaceBeforePt || sty.space_before_pt || 0,
        space_after_pt: (sty as any).spaceAfterPt || sty.space_after_pt || 0,
        first_line_indent_pt: (sty as any).firstLineIndentPt || sty.first_line_indent_pt || 0,
        alignment: sty.alignment || 'Justified',
        ahalterm_stretch: (sty as any).ahalterm_stretch || 'AutoVariable',
        color: (sty as any).color || { DeviceCmyk: { c: 0, m: 0, y: 0, k: 1 } },
        keep_with_next: (sty as any).keep_with_next ?? false,
        orphan_control: (sty as any).orphan_control ?? 2,
        widow_control: (sty as any).widow_control ?? 2
      };
      const idx = defaultStyles.findIndex((s) => s.id === sid);
      if (idx >= 0) {
        defaultStyles[idx] = normStyle;
      } else {
        defaultStyles.push(normStyle);
        definedStyleIds.add(sid);
      }
    }
  }

  const docTitle = (state.title || 'מסמך ללא שם').replace(/\.tok$/i, '');

  let multiFlowAnchorsJson: MultiFlowAnchorMapJson | undefined = undefined;
  if (state.multiFlowAnchors && state.multiFlowAnchors.length > 0) {
    const map: MultiFlowAnchorMapJson = {
      anchors: {},
      primary_node_to_anchors: {},
      anchor_to_commentaries: {},
      commentary_to_anchor: {}
    };
    for (const link of state.multiFlowAnchors) {
      const aid = link.anchorId;
      if (!map.anchors[aid]) {
        map.anchors[aid] = {
          id: aid,
          primary_flow_id: link.primaryFlowId,
          primary_node_id: link.primaryNodeId,
          primary_anchor: {
            node_id: link.primaryNodeId,
            offset: 'a00000',
            bias: 'Before'
          },
          label: link.label
        };
        if (!map.primary_node_to_anchors[link.primaryNodeId]) {
          map.primary_node_to_anchors[link.primaryNodeId] = [];
        }
        map.primary_node_to_anchors[link.primaryNodeId].push(aid);
      }
      if (!map.anchor_to_commentaries[aid]) {
        map.anchor_to_commentaries[aid] = [];
      }
      map.anchor_to_commentaries[aid].push({
        commentary_flow_id: link.commentaryFlowId,
        commentary_node_id: link.commentaryNodeId,
        anchor_id: aid,
        priority: map.anchor_to_commentaries[aid].length
      });
      map.commentary_to_anchor[link.commentaryNodeId] = aid;
    }
    multiFlowAnchorsJson = map;
  }

  return {
    id: generateCrockfordUlid(),
    metadata: {
      title: docTitle,
      author: options?.author || 'TypesetOK',
      progression: 'Rtl',
      primary_language: 'he',
      schema_version: '1.0',
      document_settings: state.settings
    },
    paragraph_styles: defaultStyles,
    character_styles: [],
    sections: [
      {
        id: generateCrockfordUlid(),
        name: docTitle,
        page_style: 'chapter-first',
        flows
      }
    ],
    multi_flow_anchors: multiFlowAnchorsJson,
    footnote_apparatuses: state.footnoteApparatuses && state.footnoteApparatuses.length > 0
      ? state.footnoteApparatuses
      : undefined,
    grep_rules: state.grepRules && state.grepRules.length > 0
      ? state.grepRules
      : undefined
  };
}

/**
 * Converts Rust `DocumentRoot` JSON format back to UI `MultiFlowDocumentState`.
 */
export function documentRootToDocumentState(root: DocumentRootJson): MultiFlowDocumentState {
  const title = root.metadata?.title || 'מסמך ללא שם';
  const sec = root.sections?.[0];
  const flows = sec?.flows || [];

  const flowMap: MultiFlowDocumentState['flows'] = {
    gemara: [],
    rashi: [],
    tosafot: [],
    notes: []
  };

  let hasCommentary = false;
  for (const flow of flows) {
    const key = flow.id === 'main' ? 'gemara' : flow.id;
    if (key === 'rashi' || key === 'tosafot') {
      hasCommentary = true;
    }

    const storyParas: StoryParagraph[] = (flow.paragraphs || []).map((p) => {
      let patch: ParagraphStylePatch | undefined;
      if (p.style_patches && p.style_patches.length > 0) {
        patch = {};
        for (const sp of p.style_patches) {
          Object.assign(patch, stylePatchJsonToPatch(sp));
        }
      }

      const spans = p.spans && p.spans.length > 0 ? p.spans.map(spanJsonToSpan) : undefined;

      return {
        id: p.id,
        styleId: p.style_id,
        text: p.text,
        spans,
        patch
      };
    });

    flowMap[key] = storyParas;
  }

  let templateType: TemplateType = 'prose';
  if (hasCommentary) {
    templateType = 'gemara';
  } else if ((flowMap.notes && flowMap.notes.length > 0)) {
    templateType = 'notes';
  }

  const stylesMap: Record<string, any> = {};
  if (Array.isArray(root.paragraph_styles)) {
    for (const ps of root.paragraph_styles) {
      if (ps && ps.id) {
        stylesMap[ps.id] = {
          ...ps,
          fontFamily: ps.font_family,
          fontWeight: ps.font_weight,
          fontSizePt: ps.font_size_pt,
          lineHeightPt: ps.line_height_pt,
          spaceBeforePt: ps.space_before_pt,
          spaceAfterPt: ps.space_after_pt,
          firstLineIndentPt: ps.first_line_indent_pt
        };
      }
    }
  }

  const multiFlowAnchors: MultiFlowAnchorLink[] = [];
  if (root.multi_flow_anchors) {
    for (const [commNodeId, anchorId] of Object.entries(root.multi_flow_anchors.commentary_to_anchor || {})) {
      const anchorDef = root.multi_flow_anchors.anchors[anchorId];
      if (anchorDef) {
        const commLink = (root.multi_flow_anchors.anchor_to_commentaries[anchorId] || []).find(
          (c) => c.commentary_node_id === commNodeId
        );
        multiFlowAnchors.push({
          anchorId,
          primaryFlowId: anchorDef.primary_flow_id,
          primaryNodeId: anchorDef.primary_node_id,
          commentaryFlowId: commLink?.commentary_flow_id || 'rashi',
          commentaryNodeId: commNodeId,
          label: anchorDef.label
        });
      }
    }
  }

  return {
    title,
    templateType,
    flows: flowMap,
    settings: root.metadata?.document_settings,
    styles: Object.keys(stylesMap).length > 0 ? stylesMap : undefined,
    multiFlowAnchors: multiFlowAnchors.length > 0 ? multiFlowAnchors : undefined,
    footnoteApparatuses: root.footnote_apparatuses && root.footnote_apparatuses.length > 0
      ? root.footnote_apparatuses
      : undefined,
    grepRules: root.grep_rules && root.grep_rules.length > 0
      ? root.grep_rules
      : undefined
  };
}

/**
 * Converts an imported document result into a MultiFlowDocumentState ready for typesetting and editing.
 */
export function importResultToDocumentState(
  importResult: ImportResult,
  templateType: TemplateType = 'prose',
  options?: { targetFlow?: string; settings?: DocumentSettings }
): MultiFlowDocumentState {
  const targetFlow = options?.targetFlow || 'gemara';
  const flowMap: MultiFlowDocumentState['flows'] = {
    gemara: [],
    rashi: [],
    tosafot: [],
    notes: []
  };

  flowMap[targetFlow] = [...importResult.paragraphs];
  if (importResult.footnotes && importResult.footnotes.length > 0) {
    flowMap.notes = [...importResult.footnotes];
  }

  const effectiveTemplate: TemplateType =
    templateType === 'prose' && importResult.footnotes && importResult.footnotes.length > 0
      ? 'notes'
      : templateType;

  return {
    title: importResult.title || 'מסמך מיובא',
    templateType: effectiveTemplate,
    flows: flowMap,
    settings: options?.settings,
    multiFlowAnchors: importResult.multiFlowAnchors && importResult.multiFlowAnchors.length > 0
      ? [...importResult.multiFlowAnchors]
      : undefined
  };
}

/**
 * Converts an imported document result directly into a full tok-core DocumentRootJson.
 */
export function importResultToDocumentRoot(
  importResult: ImportResult,
  templateType: TemplateType = 'prose',
  settings?: DocumentSettings
): DocumentRootJson {
  const state = importResultToDocumentState(importResult, templateType, { settings });
  const root = documentStateToDocumentRoot(state, settings ? { author: 'TypesetOK' } : undefined);
  if (root.metadata) {
    if (importResult.detectedLanguage) {
      root.metadata.primary_language = importResult.detectedLanguage;
    }
    if (importResult.detectedDirection) {
      root.metadata.progression = importResult.detectedDirection === 'rtl' ? 'Rtl' : 'Ltr';
    }
  }
  return root;
}


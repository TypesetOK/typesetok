/**
 * TypesetOK Document Import Engine (TextImporter).
 *
 * Defines contracts, error classes, and result interfaces for importing
 * real documents (TXT, Markdown, DOCX) into the TypesetOK document pipeline.
 */

import { ParagraphStylePatch, StoryParagraph, TextSpan, CharacterFormat, isFormatEmpty } from 'tok-story-editor';
import { MultiFlowAnchorLink } from '../types';

const CROCKFORD_ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

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

export type SupportedImportFormat = 'txt' | 'md' | 'markdown' | 'docx';

export type ImportErrorCode =
  | 'UNSUPPORTED_FORMAT'
  | 'EMPTY_DOCUMENT'
  | 'CORRUPTED_FILE'
  | 'ENCODING_ERROR'
  | 'IO_ERROR'
  | 'INVALID_STRUCTURE';

export class DocumentImportError extends Error {
  public readonly code: ImportErrorCode;
  public readonly fileName?: string;
  public readonly userMessage: string;

  constructor(code: ImportErrorCode, message: string, userMessage?: string, fileName?: string) {
    super(message);
    this.name = 'DocumentImportError';
    this.code = code;
    this.fileName = fileName;
    this.userMessage = userMessage || message;
  }
}

export interface ImportedTableCell {
  text: string;
  spans?: TextSpan[];
  widthPt?: number;
  colSpan?: number;
  isHeader?: boolean;
  nestedTables?: ImportedTableData[];
}

export interface ImportedTableRow {
  cells: ImportedTableCell[];
  isHeader?: boolean;
}

export interface ImportedTableData {
  id: string;
  rows: ImportedTableRow[];
  columnWidthsPt: number[];
  columnRatios: number[];
  hasBorders: boolean;
  borderColor?: string;
  borderWidthPt?: number;
  isRtl: boolean;
  nestingLevel: number;
}

export interface DocxStyleDefinition {
  id: string;
  name: string;
  type: 'paragraph' | 'character' | 'table' | 'numbering';
  isDefault?: boolean;
  basedOn?: string;
  nextStyleId?: string;
  paragraphPatch?: ParagraphStylePatch;
  characterFormat?: CharacterFormat;
}

export interface ImportedFieldLink {
  paragraphId?: string;
  text: string;
  url: string;
  fieldType: 'HYPERLINK' | 'REF' | 'PAGEREF' | 'OTHER';
  start?: number;
  end?: number;
  isExternal?: boolean;
}

export interface DocxFieldInfo {
  type: 'HYPERLINK' | 'REF' | 'PAGEREF' | 'PAGE' | 'NUMPAGES' | 'DATE' | 'OTHER';
  targetUrl?: string;
  isExternal?: boolean;
  targetAnchor?: string;
  rawInstruction: string;
}

export interface DocxArchiveParts {
  documentXml: string;
  footnotesXml?: string;
  endnotesXml?: string;
  numberingXml?: string;
  stylesXml?: string;
}

export interface ImportOptions {
  fileName?: string;
  format?: SupportedImportFormat;
  defaultStyleId?: string;
  headingStylePrefix?: string;
  targetFlowId?: string;
  docxXml?: string;
  footnotesXml?: string;
  endnotesXml?: string;
  numberingXml?: string;
  stylesXml?: string;
  footnoteNumbering?: 'hebrew' | 'numeric';
}

export interface ImportResult {
  title: string;
  paragraphs: StoryParagraph[];
  footnotes?: StoryParagraph[];
  tables?: ImportedTableData[];
  multiFlowAnchors?: MultiFlowAnchorLink[];
  styles?: Record<string, DocxStyleDefinition>;
  links?: ImportedFieldLink[];
  format: SupportedImportFormat;
  detectedLanguage: 'he' | 'en' | 'mixed';
  detectedDirection: 'rtl' | 'ltr';
  wordCount: number;
  paragraphCount: number;
  metadata?: Record<string, unknown>;
}

/**
 * Detects whether text is primarily Hebrew (RTL) or Latin (LTR).
 */
export function detectTextDirection(text: string): 'rtl' | 'ltr' {
  if (!text) return 'rtl';
  const hebrewMatches = (text.match(/[\u0590-\u05FF]/g) || []).length;
  const latinMatches = (text.match(/[a-zA-Z]/g) || []).length;
  return hebrewMatches >= latinMatches ? 'rtl' : 'ltr';
}

/**
 * Detects whether text is Hebrew, English, or Mixed.
 */
export function detectTextLanguage(text: string): 'he' | 'en' | 'mixed' {
  if (!text) return 'he';
  const hebrewMatches = (text.match(/[\u0590-\u05FF]/g) || []).length;
  const latinMatches = (text.match(/[a-zA-Z]/g) || []).length;
  if (hebrewMatches > 0 && latinMatches > 0) return 'mixed';
  if (hebrewMatches > 0) return 'he';
  return 'en';
}

/**
 * Extracts a clean document title from a filename by stripping extensions and replacing underscores.
 */
export function cleanDocumentTitle(fileName?: string): string {
  if (!fileName) return 'מסמך ללא שם';
  const base = fileName.split(/[\\/]/).pop() || fileName;
  const stripped = base.replace(/\.(txt|md|markdown|docx|tok|json)$/i, '').replace(/_+/g, ' ').trim();
  return stripped || 'מסמך ללא שם';
}

/**
 * Helper to create a new StoryParagraph with a valid Crockford ULID.
 */
export function createStoryParagraph(
  text: string,
  styleId = 'default-body',
  spans?: TextSpan[],
  patch?: ParagraphStylePatch
): StoryParagraph {
  return {
    id: generateCrockfordUlid(),
    styleId,
    text,
    spans: spans && spans.length > 0 ? spans : undefined,
    patch: patch && Object.keys(patch).length > 0 ? patch : undefined
  };
}

/**
 * Decodes raw buffer bytes into string with BOM and encoding detection.
 */
export function decodeTextBuffer(raw: Uint8Array | ArrayBuffer | string): string {
  if (typeof raw === 'string') return raw;
  const bytes = raw instanceof Uint8Array ? raw : new Uint8Array(raw);
  if (bytes.length === 0) return '';

  // Check for UTF-8 BOM: 0xEF, 0xBB, 0xBF
  if (bytes.length >= 3 && bytes[0] === 0xEF && bytes[1] === 0xBB && bytes[2] === 0xBF) {
    const decoder = new TextDecoder('utf-8');
    return decoder.decode(bytes.subarray(3));
  }

  // Check for UTF-16LE BOM: 0xFF, 0xFE
  if (bytes.length >= 2 && bytes[0] === 0xFF && bytes[1] === 0xFE) {
    const decoder = new TextDecoder('utf-16le');
    return decoder.decode(bytes.subarray(2));
  }

  // Check for UTF-16BE BOM: 0xFE, 0xFF
  if (bytes.length >= 2 && bytes[0] === 0xFE && bytes[1] === 0xFF) {
    const decoder = new TextDecoder('utf-16be');
    return decoder.decode(bytes.subarray(2));
  }

  // Check for null bytes indicating binary file
  let nullCount = 0;
  const sampleLen = Math.min(bytes.length, 512);
  for (let i = 0; i < sampleLen; i++) {
    if (bytes[i] === 0) nullCount++;
  }
  if (nullCount > sampleLen * 0.1) {
    throw new DocumentImportError(
      'CORRUPTED_FILE',
      'The file appears to be a binary format or unsupported encoding',
      'הקובץ שנבחר הוא קובץ בינארי או בעל קידוד לא נתמך'
    );
  }

  // Standard UTF-8 decode
  const decoder = new TextDecoder('utf-8', { fatal: false });
  return decoder.decode(bytes);
}

/**
 * Imports plain text (.txt) into structured StoryParagraph array.
 */
export function importPlainText(
  content: string | Uint8Array | ArrayBuffer,
  options?: ImportOptions
): ImportResult {
  const text = decodeTextBuffer(content).replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const clean = text.trim();

  if (!clean) {
    throw new DocumentImportError(
      'EMPTY_DOCUMENT',
      'The text document is empty',
      'קובץ הטקסט שנבחר ריק מתוכן',
      options?.fileName
    );
  }

  // Split on double newlines for paragraph preservation
  const rawParagraphs = clean.split(/\n\s*\n+/);
  const paragraphs: StoryParagraph[] = [];

  for (const rawP of rawParagraphs) {
    const trimmed = rawP.replace(/[ \t]+/g, ' ').trim();
    if (!trimmed) continue;
    paragraphs.push(createStoryParagraph(trimmed, options?.defaultStyleId || 'default-body'));
  }

  if (paragraphs.length === 0) {
    throw new DocumentImportError(
      'EMPTY_DOCUMENT',
      'No valid paragraphs found in text document',
      'לא נמצאו פסקאות תקינות במסמך',
      options?.fileName
    );
  }

  const title = cleanDocumentTitle(options?.fileName);
  const detectedDirection = detectTextDirection(clean);
  const detectedLanguage = detectTextLanguage(clean);
  const wordCount = (clean.match(/\S+/g) || []).length;

  return {
    title,
    paragraphs,
    format: 'txt',
    detectedLanguage,
    detectedDirection,
    wordCount,
    paragraphCount: paragraphs.length
  };
}

/**
 * Parses inline markdown tokens (bold, italic, code) into plain text and rich TextSpans.
 */
export function parseMarkdownInlineSpans(rawText: string): { text: string; cleanText: string; spans: TextSpan[] } {
  let cleanText = '';
  const spans: TextSpan[] = [];
  let i = 0;

  while (i < rawText.length) {
    // Check for bold-italic ***text*** or ___text___
    if (
      (rawText.startsWith('***', i) || rawText.startsWith('___', i)) &&
      rawText.indexOf(rawText.slice(i, i + 3), i + 3) !== -1
    ) {
      const marker = rawText.slice(i, i + 3);
      const endIdx = rawText.indexOf(marker, i + 3);
      const inner = rawText.slice(i + 3, endIdx);
      const start = cleanText.length;
      cleanText += inner;
      const end = cleanText.length;
      spans.push({ start, end, format: { fontWeight: 700, italic: true } });
      i = endIdx + 3;
      continue;
    }

    // Check for bold **text** or __text__
    if (
      (rawText.startsWith('**', i) || rawText.startsWith('__', i)) &&
      rawText.indexOf(rawText.slice(i, i + 2), i + 2) !== -1
    ) {
      const marker = rawText.slice(i, i + 2);
      const endIdx = rawText.indexOf(marker, i + 2);
      const inner = rawText.slice(i + 2, endIdx);
      const start = cleanText.length;
      cleanText += inner;
      const end = cleanText.length;
      spans.push({ start, end, format: { fontWeight: 700 } });
      i = endIdx + 2;
      continue;
    }

    // Check for italic *text* or _text_
    if (
      (rawText[i] === '*' || rawText[i] === '_') &&
      i + 1 < rawText.length &&
      rawText[i + 1] !== ' ' &&
      rawText.indexOf(rawText[i], i + 1) !== -1
    ) {
      const marker = rawText[i];
      const endIdx = rawText.indexOf(marker, i + 1);
      if (rawText[endIdx + 1] !== marker && rawText[endIdx - 1] !== ' ') {
        const inner = rawText.slice(i + 1, endIdx);
        const start = cleanText.length;
        cleanText += inner;
        const end = cleanText.length;
        spans.push({ start, end, format: { italic: true } });
        i = endIdx + 1;
        continue;
      }
    }

    // Check for inline code `text`
    if (rawText[i] === '`' && rawText.indexOf('`', i + 1) !== -1) {
      const endIdx = rawText.indexOf('`', i + 1);
      const inner = rawText.slice(i + 1, endIdx);
      const start = cleanText.length;
      cleanText += inner;
      const end = cleanText.length;
      spans.push({ start, end, format: { fontFamily: 'monospace' } });
      i = endIdx + 1;
      continue;
    }

    cleanText += rawText[i];
    i++;
  }

  return { text: cleanText, cleanText, spans };
}

/**
 * Imports Markdown document (.md) preserving headings, lists, quotes, and inline styles.
 */
export function importMarkdown(
  content: string | Uint8Array | ArrayBuffer,
  options?: ImportOptions
): ImportResult {
  const text = decodeTextBuffer(content).replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const clean = text.trim();

  if (!clean) {
    throw new DocumentImportError(
      'EMPTY_DOCUMENT',
      'The Markdown document is empty',
      'קובץ ה-Markdown שנבחר ריק מתוכן',
      options?.fileName
    );
  }

  const lines = text.split('\n');
  const paragraphs: StoryParagraph[] = [];
  let currentParaLines: string[] = [];
  let extractedTitle: string | null = null;

  function flushParagraph() {
    if (currentParaLines.length === 0) return;
    const joined = currentParaLines.join(' ').trim();
    currentParaLines = [];
    if (!joined) return;
    const { cleanText, spans } = parseMarkdownInlineSpans(joined);
    paragraphs.push(createStoryParagraph(cleanText, options?.defaultStyleId || 'default-body', spans));
  }

  for (let idx = 0; idx < lines.length; idx++) {
    const rawLine = lines[idx];
    const trimmed = rawLine.trim();

    if (!trimmed) {
      flushParagraph();
      continue;
    }

    // Heading 1
    if (/^#\s+/.test(trimmed)) {
      flushParagraph();
      const headingText = trimmed.replace(/^#\s+/, '').trim();
      if (!extractedTitle) extractedTitle = headingText;
      const { cleanText, spans } = parseMarkdownInlineSpans(headingText);
      paragraphs.push(createStoryParagraph(cleanText, 'heading-1', spans));
      continue;
    }

    // Heading 2
    if (/^##\s+/.test(trimmed)) {
      flushParagraph();
      const headingText = trimmed.replace(/^##\s+/, '').trim();
      const { cleanText, spans } = parseMarkdownInlineSpans(headingText);
      paragraphs.push(createStoryParagraph(cleanText, 'heading-2', spans));
      continue;
    }

    // Heading 3
    if (/^###\s+/.test(trimmed)) {
      flushParagraph();
      const headingText = trimmed.replace(/^###\s+/, '').trim();
      const { cleanText, spans } = parseMarkdownInlineSpans(headingText);
      paragraphs.push(createStoryParagraph(cleanText, 'heading-3', spans));
      continue;
    }

    // Blockquote
    if (/^>\s*/.test(trimmed)) {
      flushParagraph();
      const quoteText = trimmed.replace(/^>\s*/, '').trim();
      const { cleanText, spans } = parseMarkdownInlineSpans(quoteText);
      paragraphs.push(createStoryParagraph(cleanText, 'blockquote', spans));
      continue;
    }

    // List item
    if (/^([-*+]|\d+\.)\s+/.test(trimmed)) {
      flushParagraph();
      const itemText = trimmed.replace(/^([-*+]|\d+\.)\s+/, '').trim();
      const { cleanText, spans } = parseMarkdownInlineSpans(itemText);
      paragraphs.push(createStoryParagraph(cleanText, 'list-item', spans));
      continue;
    }

    currentParaLines.push(trimmed);
  }
  flushParagraph();

  if (paragraphs.length === 0) {
    throw new DocumentImportError(
      'EMPTY_DOCUMENT',
      'No readable paragraphs found in Markdown document',
      'לא נמצאו פסקאות תקינות במסמך ה-Markdown',
      options?.fileName
    );
  }

  const title = extractedTitle || cleanDocumentTitle(options?.fileName);
  const detectedDirection = detectTextDirection(clean);
  const detectedLanguage = detectTextLanguage(clean);
  const wordCount = (clean.match(/\S+/g) || []).length;

  return {
    title,
    paragraphs,
    format: 'md',
    detectedLanguage,
    detectedDirection,
    wordCount,
    paragraphCount: paragraphs.length
  };
}

/**
 * Decodes standard XML entities.
 */
function decodeXmlEntities(str: string): string {
  return str
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) => String.fromCharCode(parseInt(hex, 16)))
    .replace(/&#([0-9]+);/g, (_, dec) => String.fromCharCode(parseInt(dec, 10)));
}

export async function decompressDeflateRawAsync(bytes: Uint8Array): Promise<Uint8Array> {
  if (typeof DecompressionStream !== 'undefined') {
    const ds = new DecompressionStream('deflate-raw');
    const writer = ds.writable.getWriter();
    writer.write(bytes as any);
    writer.close();
    const buf = await new Response(ds.readable).arrayBuffer();
    return new Uint8Array(buf);
  }
  let zlib: any = null;
  if (typeof (globalThis as any).require === 'function') {
    try { zlib = (globalThis as any).require('zlib'); } catch {}
  }
  const proc = (globalThis as any).process;
  if (!zlib && typeof proc !== 'undefined' && typeof proc.getBuiltinModule === 'function') {
    try { zlib = proc.getBuiltinModule('node:zlib') || proc.getBuiltinModule('zlib'); } catch {}
  }
  if (zlib && typeof zlib.inflateRawSync === 'function') {
    return zlib.inflateRawSync(bytes);
  }
  throw new Error('DecompressionStream or zlib is not available in current environment');
}

function decompressZipEntryBytes(entryBytes: Uint8Array, compressionMethod: number, entryName: string): string {
  if (compressionMethod === 0) {
    return new TextDecoder('utf-8').decode(entryBytes);
  }
  if (compressionMethod === 8) {
    try {
      let zlib: any = null;
      if (typeof (globalThis as any).require === 'function') {
        try { zlib = (globalThis as any).require('zlib'); } catch {}
      }
      const proc = (globalThis as any).process;
      if (!zlib && proc && typeof proc.getBuiltinModule === 'function') {
        try { zlib = proc.getBuiltinModule('node:zlib') || proc.getBuiltinModule('zlib'); } catch {}
      }
      if (zlib && typeof zlib.inflateRawSync === 'function') {
        const inflated = zlib.inflateRawSync(entryBytes);
        return new TextDecoder('utf-8').decode(inflated);
      }
      throw new Error('zlib decompression is not available in current environment');
    } catch (e: any) {
      throw new DocumentImportError(
        'CORRUPTED_FILE',
        `Failed to decompress ${entryName}: ${e?.message}`,
        'שגיאה בפריסת תוכן ה-DOCX'
      );
    }
  }
  throw new DocumentImportError(
    'UNSUPPORTED_FORMAT',
    `Unsupported ZIP compression method: ${compressionMethod}`,
    'שיטת דחיסה לא נתמכת בקובץ DOCX'
  );
}

/**
 * Extracts all relevant XML parts (`word/document.xml`, `word/footnotes.xml`,
 * `word/endnotes.xml`, `word/numbering.xml`, `word/styles.xml`) from a DOCX zip archive buffer.
 */
export function extractDocxArchive(raw: Uint8Array | ArrayBuffer, options?: ImportOptions): DocxArchiveParts {
  if (options?.docxXml) {
    return {
      documentXml: options.docxXml,
      footnotesXml: options.footnotesXml,
      endnotesXml: options.endnotesXml,
      numberingXml: options.numberingXml,
      stylesXml: options.stylesXml
    };
  }

  const bytes = raw instanceof Uint8Array ? raw : new Uint8Array(raw);
  if (bytes.length < 30) {
    throw new DocumentImportError(
      'CORRUPTED_FILE',
      'The file is too small to be a valid DOCX package',
      'קובץ ה-DOCX קטן מדי או פגום'
    );
  }

  if (bytes[0] !== 0x50 || bytes[1] !== 0x4B || bytes[2] !== 0x03 || bytes[3] !== 0x04) {
    throw new DocumentImportError(
      'CORRUPTED_FILE',
      'Invalid ZIP signature in DOCX file',
      'מבנה קובץ ה-DOCX פגום (חתימת ZIP שגויה)'
    );
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const wantedEntries = new Set([
    'word/document.xml',
    'word/footnotes.xml',
    'word/endnotes.xml',
    'word/numbering.xml',
    'word/styles.xml'
  ]);
  const extracted: Record<string, string> = {};

  // 1. Try Central Directory first (handles data descriptors where local header compressedSize == 0)
  let eocdOffset = -1;
  const minEocdSearch = Math.max(0, bytes.length - 65557);
  for (let i = bytes.length - 22; i >= minEocdSearch; i--) {
    if (view.getUint32(i, true) === 0x06054b50) {
      eocdOffset = i;
      break;
    }
  }

  if (eocdOffset !== -1) {
    const cdCount = view.getUint16(eocdOffset + 10, true);
    let cdOffset = view.getUint32(eocdOffset + 16, true);
    for (let idx = 0; idx < cdCount && cdOffset + 46 <= bytes.length; idx++) {
      if (view.getUint32(cdOffset, true) !== 0x02014b50) break;
      const compressionMethod = view.getUint16(cdOffset + 10, true);
      const compressedSize = view.getUint32(cdOffset + 20, true);
      const fileNameLen = view.getUint16(cdOffset + 28, true);
      const extraFieldLen = view.getUint16(cdOffset + 30, true);
      const commentLen = view.getUint16(cdOffset + 32, true);
      const localHeaderOffset = view.getUint32(cdOffset + 42, true);

      const entryName = new TextDecoder('utf-8').decode(
        bytes.subarray(cdOffset + 46, cdOffset + 46 + fileNameLen)
      );

      if (wantedEntries.has(entryName) && localHeaderOffset + 30 <= bytes.length) {
        const localNameLen = view.getUint16(localHeaderOffset + 26, true);
        const localExtraLen = view.getUint16(localHeaderOffset + 28, true);
        const dataStart = localHeaderOffset + 30 + localNameLen + localExtraLen;
        if (dataStart + compressedSize <= bytes.length) {
          const entryBytes = bytes.subarray(dataStart, dataStart + compressedSize);
          extracted[entryName] = decompressZipEntryBytes(entryBytes, compressionMethod, entryName);
        }
      }

      cdOffset += 46 + fileNameLen + extraFieldLen + commentLen;
    }
  }

  // 2. Fallback / supplement via Local File Headers scan
  let offset = 0;
  while (offset + 30 <= bytes.length) {
    const sig = view.getUint32(offset, true);
    if (sig !== 0x04034b50) {
      break;
    }

    const compressionMethod = view.getUint16(offset + 8, true);
    const compressedSize = view.getUint32(offset + 18, true);
    const fileNameLen = view.getUint16(offset + 26, true);
    const extraFieldLen = view.getUint16(offset + 28, true);

    const fileNameBytes = bytes.subarray(offset + 30, offset + 30 + fileNameLen);
    const entryName = new TextDecoder('utf-8').decode(fileNameBytes);
    const dataStart = offset + 30 + fileNameLen + extraFieldLen;

    if (wantedEntries.has(entryName) && !extracted[entryName] && compressedSize > 0 && dataStart + compressedSize <= bytes.length) {
      const entryBytes = bytes.subarray(dataStart, dataStart + compressedSize);
      extracted[entryName] = decompressZipEntryBytes(entryBytes, compressionMethod, entryName);
    }

    if (compressedSize === 0 && (view.getUint16(offset + 6, true) & 0x08) !== 0) {
      // Data descriptor with unknown local size — Central Directory already handled it
      break;
    }
    offset = dataStart + compressedSize;
  }

  const documentXml = extracted['word/document.xml'];
  if (!documentXml) {
    throw new DocumentImportError(
      'INVALID_STRUCTURE',
      'word/document.xml not found in DOCX package',
      'לא נמצא תוכן מסמך תקין (word/document.xml) בתוך קובץ ה-DOCX'
    );
  }

  return {
    documentXml,
    footnotesXml: options?.footnotesXml || extracted['word/footnotes.xml'],
    endnotesXml: options?.endnotesXml || extracted['word/endnotes.xml'],
    numberingXml: options?.numberingXml || extracted['word/numbering.xml'],
    stylesXml: options?.stylesXml || extracted['word/styles.xml']
  };
}

/**
 * Extracts raw uncompressed word/document.xml from a DOCX zip archive buffer.
 */
export function extractDocxXml(raw: Uint8Array | ArrayBuffer, options?: ImportOptions): string {
  return extractDocxArchive(raw, options).documentXml;
}

const HEBREW_ONES = ['', 'א', 'ב', 'ג', 'ד', 'ה', 'ו', 'ז', 'ח', 'ט'];
const HEBREW_TENS = ['', 'י', 'כ', 'ל', 'מ', 'נ', 'ס', 'ע', 'פ', 'צ'];
const HEBREW_HUNDREDS = ['', 'ק', 'ר', 'ש', 'ת'];

/**
 * Formats a positive integer (1..999) into traditional Hebrew Gematria letters (א, ב, ... טו, טז ...).
 */
export function toHebrewListMarker(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return 'א';
  let rem = Math.floor(n);
  let out = '';
  while (rem >= 400) {
    out += 'ת';
    rem -= 400;
  }
  if (rem >= 100) {
    const h = Math.floor(rem / 100);
    out += HEBREW_HUNDREDS[h] || '';
    rem %= 100;
  }
  if (rem === 15) {
    out += 'טו';
  } else if (rem === 16) {
    out += 'טז';
  } else {
    if (rem >= 10) {
      const t = Math.floor(rem / 10);
      out += HEBREW_TENS[t] || '';
      rem %= 10;
    }
    if (rem > 0) {
      out += HEBREW_ONES[rem] || '';
    }
  }
  return out || 'א';
}

function toRomanMarker(n: number, upper = false): string {
  const vals = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
  const syms = ['m', 'cm', 'd', 'cd', 'c', 'xc', 'l', 'xl', 'x', 'ix', 'v', 'iv', 'i'];
  let num = Math.max(1, Math.min(3999, Math.floor(n)));
  let res = '';
  for (let i = 0; i < vals.length; i++) {
    while (num >= vals[i]) {
      res += syms[i];
      num -= vals[i];
    }
  }
  return upper ? res.toUpperCase() : res;
}

function toAlphaMarker(n: number, upper = false): string {
  const idx = ((Math.max(1, Math.floor(n)) - 1) % 26);
  const ch = String.fromCharCode(97 + idx);
  return upper ? ch.toUpperCase() : ch;
}

export interface DocxFootnoteEntry {
  id: string;
  text: string;
  spans: TextSpan[];
}

/**
 * Parses `word/footnotes.xml` (or `word/endnotes.xml`) into a lookup map by `w:id`.
 * Skips separator/continuation footnotes (ids <= 0 or w:type="separator"/"continuationSeparator").
 */
export function parseDocxFootnotesXml(
  footnotesXml?: string,
  styleRegistry?: Map<string, DocxStyleDefinition>
): Map<string, DocxFootnoteEntry> {
  const map = new Map<string, DocxFootnoteEntry>();
  if (!footnotesXml) return map;

  const fnRegex = /<w:(?:footnote|endnote)\b([^>]*)>([\s\S]*?)<\/w:(?:footnote|endnote)>/g;
  let fnMatch: RegExpExecArray | null;

  while ((fnMatch = fnRegex.exec(footnotesXml)) !== null) {
    const attrs = fnMatch[1] || '';
    const innerXml = fnMatch[2] || '';

    const typeMatch = /w:type="([^"]+)"/i.exec(attrs);
    if (typeMatch) {
      const tVal = typeMatch[1].toLowerCase();
      if (tVal.includes('separator') || tVal.includes('notice')) {
        continue;
      }
    }

    const idMatch = /w:id="([^"]+)"/i.exec(attrs);
    if (!idMatch) continue;
    const idStr = idMatch[1].trim();
    const idNum = parseInt(idStr, 10);
    if (!Number.isNaN(idNum) && idNum <= 0) {
      continue;
    }

    // Parse all paragraphs inside the footnote
    const pRegex = /<w:p(?:\s[^>]*)?>([\s\S]*?)<\/w:p>/g;
    let pMatch: RegExpExecArray | null;
    let combinedText = '';
    const combinedSpans: TextSpan[] = [];

    while ((pMatch = pRegex.exec(innerXml)) !== null) {
      const pBody = pMatch[1];
      const rRegex = /<w:r(?:\s[^>]*)?>([\s\S]*?)<\/w:r>/g;
      let rMatch: RegExpExecArray | null;
      let paraText = '';
      const paraSpans: TextSpan[] = [];

      while ((rMatch = rRegex.exec(pBody)) !== null) {
        const rBody = rMatch[1];
        // Skip <w:footnoteRef/> marker run itself because we format our own canonical prefix
        if (/<w:(?:footnoteRef|endnoteRef)\b/.test(rBody)) {
          continue;
        }

        let baseFormat: CharacterFormat = {};
        const rStyleMatch = /<w:rStyle\b[^>]*?w:val="([^"]+)"/i.exec(rBody);
        let charStyleId: string | undefined;
        if (rStyleMatch) {
          charStyleId = rStyleMatch[1].trim();
          const def = styleRegistry?.get(charStyleId);
          if (def?.characterFormat) {
            baseFormat = { ...def.characterFormat };
          }
        }

        const isBold = /<w:b(?:\s|\/|>)/.test(rBody) && !/<w:b\s+w:val="(?:0|false|none)"/i.test(rBody);
        const isItalic = /<w:i(?:\s|\/|>)/.test(rBody) && !/<w:i\s+w:val="(?:0|false|none)"/i.test(rBody);
        const isUnderline = /<w:u(?:\s|\/|>)/.test(rBody) && !/<w:u\s+w:val="(?:0|false|none)"/i.test(rBody);
        const szMatch = /<w:(?:sz|szCs)\b[^>]*?w:val="(\d+)"/i.exec(rBody);
        const colorMatch = /<w:color\b[^>]*?w:val="([0-9a-fA-F]{6}|auto)"/i.exec(rBody);

        const format: CharacterFormat = { ...baseFormat };
        if (charStyleId) format.characterStyleId = charStyleId;
        if (isBold) format.fontWeight = 700;
        if (isItalic) format.italic = true;
        if (isUnderline) format.underline = true;
        if (szMatch) format.fontSizePt = Math.round((parseInt(szMatch[1], 10) / 2) * 10) / 10;
        if (colorMatch && colorMatch[1].toLowerCase() !== 'auto') format.color = `#${colorMatch[1]}`;

        const tRegex = /<w:t(?:[^>]*)>([\s\S]*?)<\/w:t>/g;
        let tMatch: RegExpExecArray | null;
        let runText = '';
        while ((tMatch = tRegex.exec(rBody)) !== null) {
          runText += decodeXmlEntities(tMatch[1]);
        }

        if (runText) {
          const start = paraText.length;
          paraText += runText;
          const end = paraText.length;
          if (!isFormatEmpty(format)) {
            paraSpans.push({
              start,
              end,
              format
            });
          }
        }
      }

      const cleanPara = paraText.replace(/[ \t]+/g, ' ').trim();
      if (cleanPara) {
        const baseOffset = combinedText ? combinedText.length + 1 : 0;
        if (combinedText) combinedText += ' ';
        const leadTrim = paraText.length - paraText.trimStart().length;
        combinedText += cleanPara;
        for (const sp of paraSpans) {
          const s = Math.max(0, sp.start - leadTrim) + baseOffset;
          const e = Math.min(combinedText.length, Math.max(s, sp.end - leadTrim + baseOffset));
          if (e > s) {
            combinedSpans.push({ start: s, end: e, format: sp.format });
          }
        }
      }
    }

    if (combinedText) {
      map.set(idStr, {
        id: idStr,
        text: combinedText,
        spans: combinedSpans
      });
    }
  }

  return map;
}

interface NumberingLevelDef {
  ilvl: number;
  start: number;
  numFmt: string;
  lvlText: string;
  indentPt?: number;
}

interface NumberingRegistry {
  abstractLevels: Map<string, Map<number, NumberingLevelDef>>;
  numToAbstract: Map<string, string>;
}

/**
 * Parses `word/numbering.xml` into abstract numbering definitions and numId mappings.
 */
export function parseDocxNumberingXml(numberingXml?: string): NumberingRegistry {
  const abstractLevels = new Map<string, Map<number, NumberingLevelDef>>();
  const numToAbstract = new Map<string, string>();
  if (!numberingXml) return { abstractLevels, numToAbstract };

  const absRegex = /<w:abstractNum\b[^>]*?w:abstractNumId="([^"]+)"[^>]*>([\s\S]*?)<\/w:abstractNum>/g;
  let absMatch: RegExpExecArray | null;
  while ((absMatch = absRegex.exec(numberingXml)) !== null) {
    const absId = absMatch[1];
    const absBody = absMatch[2];
    const levels = new Map<number, NumberingLevelDef>();

    const lvlRegex = /<w:lvl\b[^>]*?w:ilvl="(\d+)"[^>]*>([\s\S]*?)<\/w:lvl>/g;
    let lvlMatch: RegExpExecArray | null;
    while ((lvlMatch = lvlRegex.exec(absBody)) !== null) {
      const ilvl = parseInt(lvlMatch[1], 10) || 0;
      const lvlBody = lvlMatch[2];
      const startMatch = /<w:start\b[^>]*?w:val="(\d+)"/i.exec(lvlBody);
      const fmtMatch = /<w:numFmt\b[^>]*?w:val="([^"]+)"/i.exec(lvlBody);
      const txtMatch = /<w:lvlText\b[^>]*?w:val="([^"]*)"/i.exec(lvlBody);
      const indMatch = /<w:ind\b[^>]*?(?:w:left|w:start|w:right)="(\d+)"/i.exec(lvlBody);

      levels.set(ilvl, {
        ilvl,
        start: startMatch ? parseInt(startMatch[1], 10) || 1 : 1,
        numFmt: fmtMatch ? fmtMatch[1] : 'decimal',
        lvlText: txtMatch ? txtMatch[1] : `%${ilvl + 1}.`,
        indentPt: indMatch ? Math.round((parseInt(indMatch[1], 10) / 20) * 10) / 10 : undefined
      });
    }
    abstractLevels.set(absId, levels);
  }

  const numRegex = /<w:num\b[^>]*?w:numId="([^"]+)"[^>]*>([\s\S]*?)<\/w:num>/g;
  let numMatch: RegExpExecArray | null;
  while ((numMatch = numRegex.exec(numberingXml)) !== null) {
    const numId = numMatch[1];
    const numBody = numMatch[2];
    const absRef = /<w:abstractNumId\b[^>]*?w:val="([^"]+)"/i.exec(numBody);
    if (absRef) {
      numToAbstract.set(numId, absRef[1]);
    }
  }

  return { abstractLevels, numToAbstract };
}

function formatSingleLevelCounter(count: number, numFmt: string): string {
  const fmt = (numFmt || 'decimal').toLowerCase();
  if (fmt === 'bullet') return '•';
  if (fmt.includes('hebrew')) return toHebrewListMarker(count);
  if (fmt === 'lowerletter') return toAlphaMarker(count, false);
  if (fmt === 'upperletter') return toAlphaMarker(count, true);
  if (fmt === 'lowerroman') return toRomanMarker(count, false);
  if (fmt === 'upperroman') return toRomanMarker(count, true);
  return String(count);
}

/**
 * Formats a hierarchical list item marker and returns `{ marker, indentPt }`.
 */
function advanceAndFormatListMarker(
  numId: string,
  ilvl: number,
  registry: NumberingRegistry,
  counters: Map<string, number[]>
): { marker: string; indentPt: number } {
  const absId = registry.numToAbstract.get(numId);
  const absMap = absId ? registry.abstractLevels.get(absId) : undefined;
  const lvlDef = absMap?.get(ilvl);

  const counterKey = absId || numId;
  let state = counters.get(counterKey);
  if (!state) {
    state = new Array(10).fill(0);
    counters.set(counterKey, state);
  }

  // Ensure parent levels have at least their start value if skipped
  for (let l = 0; l < ilvl; l++) {
    if (state[l] === 0) {
      state[l] = absMap?.get(l)?.start ?? 1;
    }
  }

  // Increment current level
  if (state[ilvl] === 0) {
    state[ilvl] = lvlDef?.start ?? 1;
  } else {
    state[ilvl] += 1;
  }

  // Reset deeper levels
  for (let l = ilvl + 1; l < state.length; l++) {
    state[l] = 0;
  }

  const numFmt = lvlDef?.numFmt || 'decimal';
  const indentPt = lvlDef?.indentPt ?? (ilvl + 1) * 18;

  if (numFmt.toLowerCase() === 'bullet') {
    return { marker: '•', indentPt };
  }

  let pattern = lvlDef?.lvlText || (ilvl > 0 ? Array.from({ length: ilvl + 1 }, (_, i) => `%${i + 1}`).join('.') + '.' : `%1.`);
  pattern = pattern.replace(/%([1-9])/g, (_, dStr) => {
    const lvlIdx = parseInt(dStr, 10) - 1;
    const cVal = state![lvlIdx] || 1;
    const lFmt = absMap?.get(lvlIdx)?.numFmt || numFmt;
    return formatSingleLevelCounter(cVal, lFmt);
  });

  if (!pattern.trim()) {
    pattern = `${formatSingleLevelCounter(state[ilvl], numFmt)}.`;
  }

  return { marker: pattern.trim(), indentPt };
}

/**
 * Parses `word/styles.xml` into a registry of resolved paragraph and character style definitions.
 * Handles style inheritance (`<w:basedOn>`), paragraph properties, and character formatting.
 */
export function parseDocxStylesXml(stylesXml?: string): Map<string, DocxStyleDefinition> {
  const map = new Map<string, DocxStyleDefinition>();
  if (!stylesXml) return map;

  const styleRegex = /<w:style\b([^>]*)>([\s\S]*?)<\/w:style>/g;
  let sMatch: RegExpExecArray | null;

  while ((sMatch = styleRegex.exec(stylesXml)) !== null) {
    const attrs = sMatch[1] || '';
    const body = sMatch[2] || '';

    const idMatch = /w:styleId="([^"]+)"/i.exec(attrs);
    if (!idMatch) continue;
    const id = idMatch[1];

    const typeMatch = /w:type="([^"]+)"/i.exec(attrs);
    const typeStr = typeMatch ? typeMatch[1].toLowerCase() : 'paragraph';
    const type: 'paragraph' | 'character' | 'table' | 'numbering' =
      typeStr === 'character' ? 'character' :
      typeStr === 'table' ? 'table' :
      typeStr === 'numbering' ? 'numbering' : 'paragraph';

    const isDefault = /w:default="(?:1|true)"/i.test(attrs);

    const nameMatch = /<w:name\b[^>]*?w:val="([^"]+)"/i.exec(body);
    const name = nameMatch ? decodeXmlEntities(nameMatch[1]) : id;

    const basedOnMatch = /<w:basedOn\b[^>]*?w:val="([^"]+)"/i.exec(body);
    const basedOn = basedOnMatch ? basedOnMatch[1] : undefined;

    const nextMatch = /<w:next\b[^>]*?w:val="([^"]+)"/i.exec(body);
    const nextStyleId = nextMatch ? nextMatch[1] : undefined;

    // Parse paragraph properties <w:pPr>
    const pPrMatch = /<w:pPr\b[^>]*>([\s\S]*?)<\/w:pPr>/i.exec(body);
    const pPrXml = pPrMatch ? pPrMatch[1] : '';
    const paragraphPatch: ParagraphStylePatch = {};

    if (pPrXml) {
      // Alignment <w:jc w:val="..."/>
      const jcMatch = /<w:jc\b[^>]*?w:val="([^"]+)"/i.exec(pPrXml);
      if (jcMatch) {
        const jc = jcMatch[1].toLowerCase();
        if (jc === 'center') paragraphPatch.alignment = 'Center';
        else if (jc === 'both' || jc === 'distribute' || jc === 'thaidistribute') paragraphPatch.alignment = 'Justified';
        else if (jc === 'left' || jc === 'end') paragraphPatch.alignment = 'Left';
        else if (jc === 'right' || jc === 'start') paragraphPatch.alignment = 'Right';
      }

      // Direction <w:bidi/>
      if (/<w:bidi(?:\s|\/|>)/.test(pPrXml) && !/<w:bidi\s+w:val="(?:0|false)"/i.test(pPrXml)) {
        paragraphPatch.direction = 'Rtl';
      }

      // Indentation <w:ind .../>
      const indMatch = /<w:ind\b([^>]*)\/?>/i.exec(pPrXml);
      if (indMatch) {
        const indAttrs = indMatch[1];
        const startDxa = /(?:w:start|w:right|w:left)="(\d+)"/i.exec(indAttrs);
        const firstLineDxa = /w:firstLine="(\d+)"/i.exec(indAttrs);
        const endDxa = /(?:w:end|w:right)="(\d+)"/i.exec(indAttrs);
        if (startDxa) {
          paragraphPatch.indentStartPt = Math.round((parseInt(startDxa[1], 10) / 20) * 10) / 10;
        }
        if (firstLineDxa) {
          paragraphPatch.firstLineIndentPt = Math.round((parseInt(firstLineDxa[1], 10) / 20) * 10) / 10;
        }
        if (endDxa) {
          paragraphPatch.indentEndPt = Math.round((parseInt(endDxa[1], 10) / 20) * 10) / 10;
        }
      }

      // Spacing <w:spacing .../>
      const spMatch = /<w:spacing\b([^>]*)\/?>/i.exec(pPrXml);
      if (spMatch) {
        const spAttrs = spMatch[1];
        const beforeMatch = /w:before="(\d+)"/i.exec(spAttrs);
        const afterMatch = /w:after="(\d+)"/i.exec(spAttrs);
        const lineMatch = /w:line="(\d+)"/i.exec(spAttrs);
        if (beforeMatch) {
          paragraphPatch.spaceBeforePt = Math.round((parseInt(beforeMatch[1], 10) / 20) * 10) / 10;
        }
        if (afterMatch) {
          paragraphPatch.spaceAfterPt = Math.round((parseInt(afterMatch[1], 10) / 20) * 10) / 10;
        }
        if (lineMatch) {
          paragraphPatch.lineHeightPt = Math.round((parseInt(lineMatch[1], 10) / 20) * 10) / 10;
        }
      }
    }

    // Parse run/character properties <w:rPr>
    const rPrMatch = /<w:rPr\b[^>]*>([\s\S]*?)<\/w:rPr>/i.exec(body);
    const rPrXml = rPrMatch ? rPrMatch[1] : '';
    const characterFormat: CharacterFormat = {};

    if (rPrXml) {
      // Font family
      const fontsMatch = /<w:rFonts\b([^>]*)\/?>/i.exec(rPrXml);
      if (fontsMatch) {
        const fontAttrs = fontsMatch[1];
        const csFont = /w:cs="([^"]+)"/i.exec(fontAttrs);
        const asciiFont = /w:ascii="([^"]+)"/i.exec(fontAttrs);
        const hAnsiFont = /w:hAnsi="([^"]+)"/i.exec(fontAttrs);
        const family = csFont ? csFont[1] : (asciiFont ? asciiFont[1] : (hAnsiFont ? hAnsiFont[1] : undefined));
        if (family) {
          characterFormat.fontFamily = decodeXmlEntities(family);
          paragraphPatch.fontFamily = characterFormat.fontFamily;
        }
      }

      // Bold
      if (/<w:b(?:\s|\/|>)/.test(rPrXml) && !/<w:b\s+w:val="(?:0|false|none)"/i.test(rPrXml)) {
        characterFormat.fontWeight = 700;
        paragraphPatch.fontWeight = 700;
      }

      // Italic
      if (/<w:i(?:\s|\/|>)/.test(rPrXml) && !/<w:i\s+w:val="(?:0|false|none)"/i.test(rPrXml)) {
        characterFormat.italic = true;
      }

      // Underline
      if (/<w:u(?:\s|\/|>)/.test(rPrXml) && !/<w:u\s+w:val="(?:0|false|none)"/i.test(rPrXml)) {
        characterFormat.underline = true;
      }

      // Font size (sz in half-points)
      const szMatch = /<w:(?:sz|szCs)\b[^>]*?w:val="(\d+)"/i.exec(rPrXml);
      if (szMatch) {
        const pt = Math.round((parseInt(szMatch[1], 10) / 2) * 10) / 10;
        characterFormat.fontSizePt = pt;
        paragraphPatch.fontSizePt = pt;
      }

      // Color
      const colorMatch = /<w:color\b[^>]*?w:val="([0-9a-fA-F]{6}|auto)"/i.exec(rPrXml);
      if (colorMatch && colorMatch[1].toLowerCase() !== 'auto') {
        characterFormat.color = `#${colorMatch[1]}`;
        paragraphPatch.color = characterFormat.color;
      }
    }

    map.set(id, {
      id,
      name,
      type,
      isDefault,
      basedOn,
      nextStyleId,
      paragraphPatch,
      characterFormat
    });
  }

  // Resolve inheritance: merge basedOn recursively
  const resolved = new Map<string, DocxStyleDefinition>();
  function resolveDef(styleId: string, visited: Set<string>): DocxStyleDefinition | undefined {
    if (visited.has(styleId)) return map.get(styleId);
    visited.add(styleId);
    const def = map.get(styleId);
    if (!def) return undefined;
    if (resolved.has(styleId)) return resolved.get(styleId);

    let mergedParagraphPatch = { ...def.paragraphPatch };
    let mergedCharacterFormat = { ...def.characterFormat };

    if (def.basedOn && map.has(def.basedOn)) {
      const base = resolveDef(def.basedOn, visited);
      if (base) {
        mergedParagraphPatch = { ...base.paragraphPatch, ...def.paragraphPatch };
        mergedCharacterFormat = { ...base.characterFormat, ...def.characterFormat };
      }
    }

    const mergedDef: DocxStyleDefinition = {
      ...def,
      paragraphPatch: mergedParagraphPatch,
      characterFormat: mergedCharacterFormat
    };
    resolved.set(styleId, mergedDef);
    return mergedDef;
  }

  for (const styleId of map.keys()) {
    resolveDef(styleId, new Set());
  }

  return resolved;
}

/**
 * Maps a DOCX style identifier and name to a canonical TypesetOK style ID or preserves custom styles.
 */
export function mapDocxStyleId(styleVal: string, styleName?: string): string {
  const normVal = (styleVal || '').toLowerCase().trim();
  const normName = (styleName || '').toLowerCase().trim();
  const matches = (target: string) => normVal.includes(target) || normName.includes(target);

  if (
    matches('heading 1') ||
    matches('heading1') ||
    normVal === '1' ||
    matches('כותרת 1') ||
    matches('כותרת1') ||
    matches('title') ||
    matches('כותרת ראשית')
  ) {
    return 'heading-1';
  }
  if (
    matches('heading 2') ||
    matches('heading2') ||
    normVal === '2' ||
    matches('כותרת 2') ||
    matches('כותרת2') ||
    matches('subtitle') ||
    matches('כותרת משנה')
  ) {
    return 'heading-2';
  }
  if (
    matches('heading 3') ||
    matches('heading3') ||
    normVal === '3' ||
    matches('כותרת 3') ||
    matches('כותרת3')
  ) {
    return 'heading-3';
  }
  if (matches('quote') || matches('ציטוט') || matches('blockquote')) {
    return 'blockquote';
  }
  if (matches('footnote') || matches('הערת שוליים') || matches('endnote') || matches('הערת סיום')) {
    return 'style-footnotes';
  }
  if (matches('list') || matches('רשימה')) {
    return 'list-item';
  }
  if (matches('normal') || matches('body') || matches('רגיל') || matches('גוף הטקסט')) {
    return 'default-body';
  }
  return styleVal || 'default-body';
}

/**
 * Parses a DOCX field instruction string (from `w:instr` or `w:instrText`),
 * extracting HYPERLINK URLs, internal bookmark targets (`\l`), REF, and PAGEREF targets.
 */
export function parseDocxFieldInstruction(rawInstr: string): DocxFieldInfo {
  const decoded = decodeXmlEntities(rawInstr).trim();
  const match = /^([A-Za-z]+)\b([\s\S]*)$/i.exec(decoded);
  if (!match) {
    return { type: 'OTHER', isExternal: false, rawInstruction: decoded };
  }
  const typeUpper = match[1].toUpperCase();
  const rest = match[2].trim();

  if (typeUpper === 'HYPERLINK') {
    let targetUrl: string | undefined;
    let targetAnchor: string | undefined;
    let remaining = rest;

    // Sub-location bookmark flag: \l "bookmark" or \l bookmark
    const subLocMatch = /\\l\s+["']?([^"'\s]+)["']?/i.exec(remaining);
    if (subLocMatch) {
      targetAnchor = subLocMatch[1];
      remaining = remaining.replace(subLocMatch[0], ' ').trim();
    }

    // Quoted URL: "https://..." or 'https://...'
    const quotedMatch = /(?:^|\s)["'](https?:\/\/[^"']+|mailto:[^"']+|ftp:\/\/[^"']+|www\.[^"']+|[^"'\s]+)["']/i.exec(remaining);
    if (quotedMatch) {
      targetUrl = quotedMatch[1];
    } else if (remaining) {
      // Unquoted URL
      const unquotedMatch = /(?:^|\s)(https?:\/\/\S+|mailto:\S+|ftp:\/\/\S+|www\.\S+)/i.exec(remaining);
      if (unquotedMatch) {
        targetUrl = unquotedMatch[1];
      }
    }

    if (targetAnchor && !targetUrl) {
      targetUrl = `#${targetAnchor}`;
    } else if (targetAnchor && targetUrl) {
      targetUrl = `${targetUrl}#${targetAnchor}`;
    }

    const isExternal = Boolean(targetUrl && !targetUrl.startsWith('#'));
    return {
      type: 'HYPERLINK',
      targetUrl: targetUrl || '#',
      isExternal,
      targetAnchor,
      rawInstruction: decoded
    };
  }

  if (typeUpper === 'REF' || typeUpper === 'PAGEREF') {
    const bookmarkMatch = /["']?([A-Za-z0-9_#\u0590-\u05FF-]+)["']?/.exec(rest);
    const targetAnchor = bookmarkMatch ? bookmarkMatch[1] : undefined;
    return {
      type: typeUpper as 'REF' | 'PAGEREF',
      targetUrl: targetAnchor ? `#${targetAnchor}` : undefined,
      isExternal: false,
      targetAnchor,
      rawInstruction: decoded
    };
  }

  const standardTypes: Array<DocxFieldInfo['type']> = ['PAGE', 'NUMPAGES', 'DATE'];
  const matchedType = standardTypes.find((t) => t === typeUpper) || 'OTHER';

  return {
    type: matchedType,
    isExternal: false,
    rawInstruction: decoded
  };
}

interface DocxRunChunk {
  rXml: string;
  fieldInfo?: DocxFieldInfo;
}

/**
 * Extracts run chunks from a `<w:p>` paragraph body preserving field elements like
 * `<w:fldSimple>` and `<w:hyperlink>`.
 */
function extractParagraphRunChunks(pXml: string): DocxRunChunk[] {
  const bodyXml = pXml.replace(/<w:pPr\b[\s\S]*?<\/w:pPr>/i, '');
  const chunks: DocxRunChunk[] = [];

  const elemRegex = /<w:(fldSimple|hyperlink|r)\b([^>]*)>([\s\S]*?)<\/w:\1>|<w:r\b([^>]*)\/>/g;
  let match: RegExpExecArray | null;

  while ((match = elemRegex.exec(bodyXml)) !== null) {
    if (!match[1]) continue;
    const tag = match[1];
    const attrs = match[2] || '';
    const inner = match[3] || '';

    if (tag === 'r') {
      chunks.push({ rXml: inner });
    } else if (tag === 'fldSimple') {
      const instrMatch = /w:instr="([^"]*)"/i.exec(attrs);
      const rawInstr = instrMatch ? instrMatch[1] : '';
      const fieldInfo = parseDocxFieldInstruction(rawInstr);

      const rInnerRegex = /<w:r\b[^>]*>([\s\S]*?)<\/w:r>/g;
      let rMatch: RegExpExecArray | null;
      let matchedAny = false;
      while ((rMatch = rInnerRegex.exec(inner)) !== null) {
        matchedAny = true;
        chunks.push({ rXml: rMatch[1], fieldInfo });
      }
      if (!matchedAny && inner.includes('<w:t')) {
        chunks.push({ rXml: inner, fieldInfo });
      }
    } else if (tag === 'hyperlink') {
      const anchorMatch = /w:anchor="([^"]*)"/i.exec(attrs);
      const ridMatch = /r:id="([^"]*)"/i.exec(attrs);
      const targetAnchor = anchorMatch ? anchorMatch[1] : undefined;
      const targetUrl = targetAnchor ? `#${targetAnchor}` : (ridMatch ? `#${ridMatch[1]}` : '#');
      const fieldInfo: DocxFieldInfo = {
        type: 'HYPERLINK',
        targetUrl,
        isExternal: !targetUrl.startsWith('#'),
        targetAnchor,
        rawInstruction: `HYPERLINK "${targetUrl}"`
      };

      const rInnerRegex = /<w:r\b[^>]*>([\s\S]*?)<\/w:r>/g;
      let rMatch: RegExpExecArray | null;
      let matchedAny = false;
      while ((rMatch = rInnerRegex.exec(inner)) !== null) {
        matchedAny = true;
        chunks.push({ rXml: rMatch[1], fieldInfo });
      }
      if (!matchedAny && inner.includes('<w:t')) {
        chunks.push({ rXml: inner, fieldInfo });
      }
    }
  }

  if (chunks.length === 0) {
    const rInnerRegex = /<w:r\b[^>]*>([\s\S]*?)<\/w:r>/g;
    let rMatch: RegExpExecArray | null;
    while ((rMatch = rInnerRegex.exec(bodyXml)) !== null) {
      chunks.push({ rXml: rMatch[1] });
    }
  }

  return chunks;
}

/**
 * Finds top-level `<w:p>` and `<w:tbl>` blocks in document order without matching `<w:p>` inside `<w:tbl>` twice.
 */
function extractTopLevelBodyBlocks(xml: string): Array<{ type: 'p' | 'tbl'; xml: string }> {
  const bodyMatch = /<w:body\b[^>]*>([\s\S]*?)<\/w:body>/i.exec(xml);
  const source = bodyMatch ? bodyMatch[1] : xml;
  const blocks: Array<{ type: 'p' | 'tbl'; xml: string }> = [];

  const tokenRegex = /<(\/?)w:(p|tbl)\b[^>]*?(\/?)>/g;
  let tok: RegExpExecArray | null;
  let currentType: 'p' | 'tbl' | null = null;
  let depth = 0;
  let startIdx = 0;

  while ((tok = tokenRegex.exec(source)) !== null) {
    const isClose = tok[1] === '/';
    const tag = tok[2] as 'p' | 'tbl';
    const isSelfClose = tok[3] === '/';

    if (currentType === null) {
      if (!isClose && !isSelfClose) {
        currentType = tag;
        depth = 1;
        startIdx = tok.index;
      }
    } else if (tag === currentType) {
      if (!isClose && !isSelfClose) {
        depth++;
      } else if (isClose) {
        depth--;
        if (depth === 0) {
          const endIdx = tok.index + tok[0].length;
          blocks.push({ type: currentType, xml: source.slice(startIdx, endIdx) });
          currentType = null;
        }
      }
    }
  }

  return blocks;
}

/**
 * Extracts direct `<w:tr>` rows of a `<w:tbl>` without matching `<w:tr>` of nested tables.
 */
function extractDirectSubElements(xml: string, tagName: 'tr' | 'tc'): string[] {
  const results: string[] = [];
  const inner = xml.replace(/^<w:tbl\b[^>]*>|^<w:tr\b[^>]*>/i, '').replace(/<\/w:tbl>\s*$|<\/w:tr>\s*$/i, '');
  const tokenRegex = new RegExp(`<(\\/?)w:(${tagName}|tbl)\\b[^>]*?(\\/?)>`, 'g');
  let tok: RegExpExecArray | null;
  let tblDepth = 0;
  let targetDepth = 0;
  let startIdx = 0;

  while ((tok = tokenRegex.exec(inner)) !== null) {
    const isClose = tok[1] === '/';
    const tag = tok[2];
    const isSelfClose = tok[3] === '/';

    if (tag === 'tbl') {
      if (!isClose && !isSelfClose) tblDepth++;
      else if (isClose) tblDepth = Math.max(0, tblDepth - 1);
      continue;
    }

    if (tblDepth === 0 && tag === tagName) {
      if (!isClose && !isSelfClose) {
        if (targetDepth === 0) startIdx = tok.index;
        targetDepth++;
      } else if (isClose) {
        targetDepth--;
        if (targetDepth === 0) {
          results.push(inner.slice(startIdx, tok.index + tok[0].length));
        }
      }
    }
  }
  return results;
}

/**
 * Parses a single `<w:p>` element, applying custom styles from `word/styles.xml`,
 * resolving dynamic field links (`w:fldSimple`, `w:hyperlink`, complex fields),
 * and linking inline footnote references.
 */
function parseDocxParagraphElement(
  pXml: string,
  options: ImportOptions | undefined,
  footnoteLookup: Map<string, DocxFootnoteEntry>,
  numberingRegistry: NumberingRegistry,
  numberingCounters: Map<string, number[]>,
  styleRegistry: Map<string, DocxStyleDefinition>,
  emittedFootnotes: StoryParagraph[],
  anchorLinks: MultiFlowAnchorLink[],
  emittedLinks: ImportedFieldLink[],
  nextFootnoteSeq: () => number
): StoryParagraph | null {
  let styleId = options?.defaultStyleId || 'default-body';
  let matchedStyleDef: DocxStyleDefinition | undefined;

  const styleMatch = /<w:pStyle[^>]*?w:val="([^"]+)"/i.exec(pXml);
  if (styleMatch) {
    const rawVal = styleMatch[1].trim();
    matchedStyleDef = styleRegistry.get(rawVal);
    styleId = mapDocxStyleId(rawVal, matchedStyleDef?.name);
  }

  // Base paragraph patch inherited from the resolved style definition
  const patch: ParagraphStylePatch = {
    ...(matchedStyleDef?.paragraphPatch || {})
  };

  // Local paragraph alignment <w:jc w:val="..."/> override
  const jcMatch = /<w:jc\b[^>]*?w:val="([^"]+)"/i.exec(pXml);
  if (jcMatch) {
    const jc = jcMatch[1].toLowerCase();
    if (jc === 'center') patch.alignment = 'Center';
    else if (jc === 'both' || jc === 'distribute' || jc === 'ThaiDistribute') patch.alignment = 'Justified';
    else if (jc === 'left' || jc === 'end') patch.alignment = 'Left';
    else if (jc === 'right' || jc === 'start') patch.alignment = 'Right';
  }

  // Local paragraph direction <w:bidi/> override
  if (/<w:bidi(?:\s|\/|>)/.test(pXml) && !/<w:bidi\s+w:val="(?:0|false)"/i.test(pXml)) {
    patch.direction = 'Rtl';
  }

  // Local paragraph indentation <w:ind .../> override
  const indTagMatch = /<w:ind\b([^>]*)\/?>/i.exec(pXml);
  if (indTagMatch) {
    const indAttrs = indTagMatch[1];
    const startDxa = /(?:w:start|w:right|w:left)="(\d+)"/i.exec(indAttrs);
    const firstLineDxa = /w:firstLine="(\d+)"/i.exec(indAttrs);
    const endDxa = /(?:w:end|w:right)="(\d+)"/i.exec(indAttrs);
    if (startDxa) {
      patch.indentStartPt = Math.round((parseInt(startDxa[1], 10) / 20) * 10) / 10;
    }
    if (firstLineDxa) {
      patch.firstLineIndentPt = Math.round((parseInt(firstLineDxa[1], 10) / 20) * 10) / 10;
    }
    if (endDxa) {
      patch.indentEndPt = Math.round((parseInt(endDxa[1], 10) / 20) * 10) / 10;
    }
  }

  // Local paragraph spacing <w:spacing .../> override
  const spTagMatch = /<w:spacing\b([^>]*)\/?>/i.exec(pXml);
  if (spTagMatch) {
    const spAttrs = spTagMatch[1];
    const beforeDxa = /w:before="(\d+)"/i.exec(spAttrs);
    const afterDxa = /w:after="(\d+)"/i.exec(spAttrs);
    const lineDxa = /w:line="(\d+)"/i.exec(spAttrs);
    if (beforeDxa) {
      patch.spaceBeforePt = Math.round((parseInt(beforeDxa[1], 10) / 20) * 10) / 10;
    }
    if (afterDxa) {
      patch.spaceAfterPt = Math.round((parseInt(afterDxa[1], 10) / 20) * 10) / 10;
    }
    if (lineDxa) {
      patch.lineHeightPt = Math.round((parseInt(lineDxa[1], 10) / 20) * 10) / 10;
    }
  }

  // Check for numbered/bulleted list item <w:numPr>
  let listPrefix = '';
  const numPrMatch = /<w:numPr\b[^>]*>([\s\S]*?)<\/w:numPr>/i.exec(pXml);
  if (numPrMatch) {
    styleId = 'list-item';
    const numPrBody = numPrMatch[1];
    const ilvlMatch = /<w:ilvl\b[^>]*?w:val="(\d+)"/i.exec(numPrBody);
    const numIdMatch = /<w:numId\b[^>]*?w:val="([^"]+)"/i.exec(numPrBody);
    const ilvl = ilvlMatch ? parseInt(ilvlMatch[1], 10) || 0 : 0;
    const numId = numIdMatch ? numIdMatch[1] : '1';
    if (numId !== '0') {
      const formatted = advanceAndFormatListMarker(numId, ilvl, numberingRegistry, numberingCounters);
      listPrefix = `${formatted.marker} `;
      if (patch.indentStartPt === undefined) {
        patch.indentStartPt = formatted.indentPt;
      }
    }
  }

  let fullParaText = listPrefix;
  const spans: TextSpan[] = [];
  const pendingAnchorsForPara: Array<{ fnParaId: string; label: string }> = [];
  const pendingLinksForPara: ImportedFieldLink[] = [];

  if (listPrefix) {
    spans.push({
      start: 0,
      end: listPrefix.trimEnd().length,
      format: { fontWeight: 700 }
    });
  }

  const useHebrewMarkers = (options?.footnoteNumbering ?? 'hebrew') === 'hebrew';
  const chunks = extractParagraphRunChunks(pXml);

  let activeComplexField: DocxFieldInfo | undefined;
  let collectingComplexInstr = false;
  let complexInstrBuffer = '';

  for (const chunk of chunks) {
    const rXml = chunk.rXml;

    // Complex field codes state machine (<w:fldChar>)
    if (/<w:fldChar\b[^>]*?w:fldCharType="begin"/i.test(rXml)) {
      collectingComplexInstr = true;
      complexInstrBuffer = '';
      activeComplexField = undefined;
      continue;
    }
    if (collectingComplexInstr) {
      const instrMatch = /<w:instrText\b[^>]*>([\s\S]*?)<\/w:instrText>/gi;
      let im: RegExpExecArray | null;
      while ((im = instrMatch.exec(rXml)) !== null) {
        complexInstrBuffer += im[1];
      }
      if (/<w:fldChar\b[^>]*?w:fldCharType="separate"/i.test(rXml)) {
        collectingComplexInstr = false;
        activeComplexField = parseDocxFieldInstruction(complexInstrBuffer);
      }
      continue;
    }
    if (/<w:fldChar\b[^>]*?w:fldCharType="end"/i.test(rXml)) {
      activeComplexField = undefined;
      continue;
    }

    const effectiveField = chunk.fieldInfo || activeComplexField;

    // Character style resolution from styleRegistry
    let baseCharFormat: CharacterFormat = {};
    let characterStyleId: string | undefined;
    const rStyleMatch = /<w:rStyle\b[^>]*?w:val="([^"]+)"/i.exec(rXml);
    if (rStyleMatch) {
      const rStyleVal = rStyleMatch[1].trim();
      characterStyleId = rStyleVal;
      const def = styleRegistry.get(rStyleVal);
      if (def?.characterFormat) {
        baseCharFormat = { ...def.characterFormat };
      }
      if (rStyleVal.toLowerCase().includes('hyperlink') || rStyleVal.includes('קישור')) {
        baseCharFormat.underline = true;
        if (!baseCharFormat.color) baseCharFormat.color = '#0563C1';
        characterStyleId = 'hyperlink';
      }
    }

    // Direct run properties overrides
    const isBold = /<w:b(?:\s|\/|>)/.test(rXml) && !/<w:b\s+w:val="(?:0|false|none)"/i.test(rXml);
    const isItalic = /<w:i(?:\s|\/|>)/.test(rXml) && !/<w:i\s+w:val="(?:0|false|none)"/i.test(rXml);
    const isUnderline = /<w:u(?:\s|\/|>)/.test(rXml) && !/<w:u\s+w:val="(?:0|false|none)"/i.test(rXml);
    const szMatch = /<w:(?:sz|szCs)\b[^>]*?w:val="(\d+)"/i.exec(rXml);
    const colorMatch = /<w:color\b[^>]*?w:val="([0-9a-fA-F]{6}|auto)"/i.exec(rXml);

    const fontsMatch = /<w:rFonts\b([^>]*)\/?>/i.exec(rXml);
    let runFontFamily: string | undefined;
    if (fontsMatch) {
      const fontAttrs = fontsMatch[1];
      const csFont = /w:cs="([^"]+)"/i.exec(fontAttrs);
      const asciiFont = /w:ascii="([^"]+)"/i.exec(fontAttrs);
      const hAnsiFont = /w:hAnsi="([^"]+)"/i.exec(fontAttrs);
      runFontFamily = csFont ? csFont[1] : (asciiFont ? asciiFont[1] : (hAnsiFont ? hAnsiFont[1] : undefined));
    }

    const format: CharacterFormat = { ...baseCharFormat };
    if (characterStyleId) format.characterStyleId = characterStyleId;
    if (isBold) format.fontWeight = 700;
    if (isItalic) format.italic = true;
    if (isUnderline) format.underline = true;
    if (szMatch) format.fontSizePt = Math.round((parseInt(szMatch[1], 10) / 2) * 10) / 10;
    if (colorMatch && colorMatch[1].toLowerCase() !== 'auto') format.color = `#${colorMatch[1]}`;
    if (runFontFamily) format.fontFamily = decodeXmlEntities(runFontFamily);

    // If field instruction applies
    if (effectiveField) {
      if (effectiveField.type === 'HYPERLINK' && effectiveField.targetUrl) {
        format.linkUrl = effectiveField.targetUrl;
        if (!format.characterStyleId) format.characterStyleId = 'hyperlink';
        if (format.underline === undefined) format.underline = true;
        if (!format.color) format.color = '#0563C1';
      } else if ((effectiveField.type === 'REF' || effectiveField.type === 'PAGEREF') && effectiveField.targetUrl) {
        format.linkUrl = effectiveField.targetUrl;
        if (!format.characterStyleId) format.characterStyleId = 'cross-reference';
        if (format.underline === undefined) format.underline = true;
        if (!format.color) format.color = '#0563C1';
      } else if (['PAGE', 'NUMPAGES', 'DATE'].includes(effectiveField.type)) {
        if (!format.characterStyleId) format.characterStyleId = `field-${effectiveField.type.toLowerCase()}`;
      }
    }

    // Extract text from <w:t>, <w:tab/>, <w:br/>
    const tRegex = /<w:t(?:[^>]*)>([\s\S]*?)<\/w:t>/g;
    let tMatch: RegExpExecArray | null;
    let runText = '';
    while ((tMatch = tRegex.exec(rXml)) !== null) {
      runText += decodeXmlEntities(tMatch[1]);
    }
    if (/<w:tab(?:\s|\/|>)/.test(rXml)) {
      runText += '\t';
    }
    if (/<w:br(?:\s|\/|>)/.test(rXml)) {
      runText += '\n';
    }

    if (runText) {
      const start = fullParaText.length;
      fullParaText += runText;
      const end = fullParaText.length;

      if (!isFormatEmpty(format)) {
        spans.push({ start, end, format });
      }

      if (effectiveField?.targetUrl) {
        pendingLinksForPara.push({
          text: runText,
          url: effectiveField.targetUrl,
          fieldType:
            effectiveField.type === 'HYPERLINK'
              ? 'HYPERLINK'
              : effectiveField.type === 'REF'
              ? 'REF'
              : effectiveField.type === 'PAGEREF'
              ? 'PAGEREF'
              : 'OTHER',
          start,
          end,
          isExternal: effectiveField.isExternal
        });
      }
    }

    // Check for <w:footnoteReference w:id="..."/> or <w:endnoteReference w:id="..."/>
    const fnRefRegex = /<w:(?:footnoteReference|endnoteReference)\b[^>]*?w:id="([^"]+)"[^>]*\/?>/g;
    let fnRefMatch: RegExpExecArray | null;
    while ((fnRefMatch = fnRefRegex.exec(rXml)) !== null) {
      const refId = fnRefMatch[1].trim();
      const fnEntry = footnoteLookup.get(refId);
      const seq = nextFootnoteSeq();
      const markerLabel = useHebrewMarkers ? toHebrewListMarker(seq) : String(seq);
      const inlineMarker = `[${markerLabel}]`;

      const mStart = fullParaText.length;
      fullParaText += inlineMarker;
      const mEnd = fullParaText.length;
      spans.push({
        start: mStart,
        end: mEnd,
        format: {
          characterStyleId: 'footnote-ref',
          fontSizePt: 8.5,
          fontWeight: 700
        }
      });

      if (fnEntry) {
        const fnPrefix = `${markerLabel}. `;
        const fnFullText = fnPrefix + fnEntry.text;
        const fnSpans: TextSpan[] = [
          {
            start: 0,
            end: fnPrefix.trimEnd().length,
            format: { fontWeight: 700 }
          },
          ...fnEntry.spans.map((sp) => ({
            start: sp.start + fnPrefix.length,
            end: sp.end + fnPrefix.length,
            format: { ...sp.format }
          }))
        ];
        const fnPara = createStoryParagraph(fnFullText, 'style-footnotes', fnSpans);
        emittedFootnotes.push(fnPara);
        pendingAnchorsForPara.push({ fnParaId: fnPara.id, label: markerLabel });
      }
    }
  }

  const leadSpaces = fullParaText.length - fullParaText.trimStart().length;
  const trimmed = fullParaText.trim();
  if (!trimmed) return null;

  const adjustedSpans: TextSpan[] = [];
  for (const sp of spans) {
    const s = Math.max(0, sp.start - leadSpaces);
    const e = Math.min(trimmed.length, sp.end - leadSpaces);
    if (e > s) {
      adjustedSpans.push({ start: s, end: e, format: sp.format });
    }
  }

  const para = createStoryParagraph(trimmed, styleId, adjustedSpans, patch);

  for (const anc of pendingAnchorsForPara) {
    anchorLinks.push({
      anchorId: `anc_${para.id}_${anc.fnParaId}`,
      primaryFlowId: options?.targetFlowId || 'gemara',
      primaryNodeId: para.id,
      commentaryFlowId: 'notes',
      commentaryNodeId: anc.fnParaId,
      label: anc.label
    });
  }

  // Consolidate links for this paragraph and emit
  if (pendingLinksForPara.length > 0) {
    let currentLink: ImportedFieldLink | null = null;
    for (const pl of pendingLinksForPara) {
      const s = Math.max(0, (pl.start ?? 0) - leadSpaces);
      const e = Math.min(trimmed.length, (pl.end ?? 0) - leadSpaces);
      if (e <= s) continue;

      if (currentLink && currentLink.url === pl.url && currentLink.end === s) {
        currentLink.end = e;
        currentLink.text = trimmed.slice(currentLink.start ?? 0, currentLink.end);
      } else {
        if (currentLink) {
          emittedLinks.push(currentLink);
        }
        currentLink = {
          paragraphId: para.id,
          text: trimmed.slice(s, e),
          url: pl.url,
          fieldType: pl.fieldType,
          start: s,
          end: e,
          isExternal: pl.isExternal
        };
      }
    }
    if (currentLink) {
      emittedLinks.push(currentLink);
    }
  }

  return para;
}

/**
 * Parses a `<w:tbl>` XML block (including nested `<w:tbl>` inside cells) into
 * structured `ImportedTableData` and RTL tabular `StoryParagraph` rows.
 */
function parseDocxTableElement(
  tblXml: string,
  nestingLevel: number,
  options: ImportOptions | undefined,
  footnoteLookup: Map<string, DocxFootnoteEntry>,
  numberingRegistry: NumberingRegistry,
  numberingCounters: Map<string, number[]>,
  styleRegistry: Map<string, DocxStyleDefinition>,
  emittedFootnotes: StoryParagraph[],
  anchorLinks: MultiFlowAnchorLink[],
  emittedTables: ImportedTableData[],
  emittedLinks: ImportedFieldLink[],
  nextFootnoteSeq: () => number
): { tableData: ImportedTableData; rowParagraphs: StoryParagraph[] } {
  const tblPrMatch = /<w:tblPr\b[^>]*>([\s\S]*?)<\/w:tblPr>/i.exec(tblXml);
  const tblPr = tblPrMatch ? tblPrMatch[1] : '';
  const isRtl = !/<w:bidiVisual\s+w:val="(?:0|false)"/i.test(tblPr);
  const hasBorders = /<w:tblBorders\b|<w:tcBorders\b/i.test(tblXml) && !/<w:top\s+w:val="none"/i.test(tblPr);

  const gridCols: number[] = [];
  const tblGridMatch = /<w:tblGrid\b[^>]*>([\s\S]*?)<\/w:tblGrid>/i.exec(tblXml);
  if (tblGridMatch) {
    const colRegex = /<w:gridCol\b[^>]*?w:w="(\d+)"/g;
    let cMatch: RegExpExecArray | null;
    while ((cMatch = colRegex.exec(tblGridMatch[1])) !== null) {
      gridCols.push(Math.max(18, Math.round((parseInt(cMatch[1], 10) / 20) * 10) / 10));
    }
  }

  const trList = extractDirectSubElements(tblXml, 'tr');
  const rows: ImportedTableRow[] = [];
  const rowParagraphs: StoryParagraph[] = [];
  let maxCols = gridCols.length;

  for (let rIdx = 0; rIdx < trList.length; rIdx++) {
    const trXml = trList[rIdx];
    const isHeaderRow = /<w:tblHeader(?:\s|\/|>)/.test(trXml) || rIdx === 0;
    const tcList = extractDirectSubElements(trXml, 'tc');
    maxCols = Math.max(maxCols, tcList.length);

    const cells: ImportedTableCell[] = [];
    const nestedRowParas: StoryParagraph[] = [];

    for (let cIdx = 0; cIdx < tcList.length; cIdx++) {
      const tcXml = tcList[cIdx];
      const tcWMatch = /<w:tcW\b[^>]*?w:w="(\d+)"/i.exec(tcXml);
      const widthPt = tcWMatch
        ? Math.max(18, Math.round((parseInt(tcWMatch[1], 10) / 20) * 10) / 10)
        : gridCols[cIdx];
      const spanMatch = /<w:gridSpan\b[^>]*?w:val="(\d+)"/i.exec(tcXml);
      const colSpan = spanMatch ? Math.max(1, parseInt(spanMatch[1], 10) || 1) : 1;

      const cellBlocks = extractTopLevelBodyBlocks(tcXml);
      const cellTexts: string[] = [];
      const cellSpans: TextSpan[] = [];
      const nestedTables: ImportedTableData[] = [];

      for (const blk of cellBlocks) {
        if (blk.type === 'p') {
          const p = parseDocxParagraphElement(
            blk.xml,
            options,
            footnoteLookup,
            numberingRegistry,
            numberingCounters,
            styleRegistry,
            emittedFootnotes,
            anchorLinks,
            emittedLinks,
            nextFootnoteSeq
          );
          if (p && p.text) {
            const offset = cellTexts.length > 0 ? cellTexts.join(' ').length + 1 : 0;
            cellTexts.push(p.text);
            if (p.spans) {
              for (const sp of p.spans) {
                cellSpans.push({
                  start: sp.start + offset,
                  end: sp.end + offset,
                  format: sp.format
                });
              }
            }
          }
        } else if (blk.type === 'tbl') {
          const nested = parseDocxTableElement(
            blk.xml,
            nestingLevel + 1,
            options,
            footnoteLookup,
            numberingRegistry,
            numberingCounters,
            styleRegistry,
            emittedFootnotes,
            anchorLinks,
            emittedTables,
            emittedLinks,
            nextFootnoteSeq
          );
          nestedTables.push(nested.tableData);
          nestedRowParas.push(...nested.rowParagraphs);
        }
      }

      cells.push({
        text: cellTexts.join(' ').trim(),
        spans: cellSpans.length > 0 ? cellSpans : undefined,
        widthPt,
        colSpan,
        isHeader: isHeaderRow,
        nestedTables: nestedTables.length > 0 ? nestedTables : undefined
      });
    }

    rows.push({ cells, isHeader: isHeaderRow });

    // Build a structured RTL tabular paragraph representing this table row
    const delimiter = '  │  ';
    let rowText = '';
    const rowSpans: TextSpan[] = [];

    for (let cIdx = 0; cIdx < cells.length; cIdx++) {
      if (cIdx > 0) {
        rowText += delimiter;
      }
      const cell = cells[cIdx];
      const cellStart = rowText.length;
      const cellContent = cell.text || '—';
      rowText += cellContent;
      const cellEnd = rowText.length;

      if (isHeaderRow) {
        rowSpans.push({
          start: cellStart,
          end: cellEnd,
          format: { fontWeight: 700 }
        });
      } else if (cell.spans) {
        for (const sp of cell.spans) {
          rowSpans.push({
            start: cellStart + sp.start,
            end: Math.min(cellEnd, cellStart + sp.end),
            format: sp.format
          });
        }
      }
    }

    if (rowText.trim()) {
      const rowPatch: ParagraphStylePatch = {
        direction: isRtl ? 'Rtl' : 'Ltr',
        alignment: 'Right',
        indentStartPt: nestingLevel > 0 ? nestingLevel * 16 : 0,
        spaceBeforePt: rIdx === 0 ? 4 : 1,
        spaceAfterPt: rIdx === trList.length - 1 ? 4 : 1
      };
      const rowPara = createStoryParagraph(
        rowText,
        isHeaderRow ? 'table-header' : 'table-row',
        rowSpans,
        rowPatch
      );
      (rowPara as any).tableRowMeta = {
        rowIndex: rIdx,
        isHeader: isHeaderRow,
        nestingLevel,
        hasBorders,
        cellCount: cells.length,
        columnWidthsPt: cells.map((c) => c.widthPt || 100)
      };
      rowParagraphs.push(rowPara);
    }

    if (nestedRowParas.length > 0) {
      rowParagraphs.push(...nestedRowParas);
    }
  }

  const colCount = Math.max(1, maxCols);
  const columnWidthsPt =
    gridCols.length >= colCount
      ? gridCols.slice(0, colCount)
      : Array.from({ length: colCount }, (_, i) => rows[0]?.cells[i]?.widthPt || Math.round(450 / colCount));
  const totalW = columnWidthsPt.reduce((a, b) => a + b, 0) || 1;
  const columnRatios = columnWidthsPt.map((w) => Math.round((w / totalW) * 1000) / 1000);

  const tableData: ImportedTableData = {
    id: generateCrockfordUlid(),
    rows,
    columnWidthsPt,
    columnRatios,
    hasBorders,
    isRtl,
    nestingLevel
  };
  emittedTables.push(tableData);

  return { tableData, rowParagraphs };
}

/**
 * Parses word/document.xml (along with optional `footnotesXml`, `numberingXml`, and `stylesXml`)
 * into structured StoryParagraphs, linked footnotes, custom styled paragraphs, field links,
 * tables, and hierarchical numbered lists.
 */
export function parseDocxXml(xml: string, options?: ImportOptions): ImportResult {
  const styleRegistry = parseDocxStylesXml(options?.stylesXml);
  const footnoteLookup = parseDocxFootnotesXml(options?.footnotesXml, styleRegistry);
  if (options?.endnotesXml) {
    const endnotes = parseDocxFootnotesXml(options.endnotesXml, styleRegistry);
    for (const [k, v] of endnotes.entries()) {
      if (!footnoteLookup.has(k)) footnoteLookup.set(k, v);
    }
  }
  const numberingRegistry = parseDocxNumberingXml(options?.numberingXml);
  const numberingCounters = new Map<string, number[]>();

  const paragraphs: StoryParagraph[] = [];
  const emittedFootnotes: StoryParagraph[] = [];
  const emittedTables: ImportedTableData[] = [];
  const emittedLinks: ImportedFieldLink[] = [];
  const anchorLinks: MultiFlowAnchorLink[] = [];
  let footnoteSeqCounter = 0;
  const nextFootnoteSeq = () => ++footnoteSeqCounter;

  const blocks = extractTopLevelBodyBlocks(xml);
  for (const blk of blocks) {
    if (blk.type === 'p') {
      const p = parseDocxParagraphElement(
        blk.xml,
        options,
        footnoteLookup,
        numberingRegistry,
        numberingCounters,
        styleRegistry,
        emittedFootnotes,
        anchorLinks,
        emittedLinks,
        nextFootnoteSeq
      );
      if (p) paragraphs.push(p);
    } else if (blk.type === 'tbl') {
      const { rowParagraphs } = parseDocxTableElement(
        blk.xml,
        0,
        options,
        footnoteLookup,
        numberingRegistry,
        numberingCounters,
        styleRegistry,
        emittedFootnotes,
        anchorLinks,
        emittedTables,
        emittedLinks,
        nextFootnoteSeq
      );
      paragraphs.push(...rowParagraphs);
    }
  }

  // Any remaining footnotes in word/footnotes.xml that were not explicitly referenced by w:footnoteReference
  if (footnoteLookup.size > 0 && emittedFootnotes.length === 0) {
    const useHebrewMarkers = (options?.footnoteNumbering ?? 'hebrew') === 'hebrew';
    for (const [, entry] of footnoteLookup.entries()) {
      const seq = nextFootnoteSeq();
      const markerLabel = useHebrewMarkers ? toHebrewListMarker(seq) : String(seq);
      const fnPrefix = `${markerLabel}. `;
      const fnSpans: TextSpan[] = [
        { start: 0, end: fnPrefix.trimEnd().length, format: { fontWeight: 700 } },
        ...entry.spans.map((sp) => ({
          start: sp.start + fnPrefix.length,
          end: sp.end + fnPrefix.length,
          format: { ...sp.format }
        }))
      ];
      emittedFootnotes.push(createStoryParagraph(fnPrefix + entry.text, 'style-footnotes', fnSpans));
    }
  }

  if (paragraphs.length === 0) {
    throw new DocumentImportError(
      'EMPTY_DOCUMENT',
      'The DOCX document contains no text paragraphs',
      'קובץ ה-DOCX אינו מכיל פסקאות טקסט',
      options?.fileName
    );
  }

  const allText = [
    ...paragraphs.map((p) => p.text),
    ...emittedFootnotes.map((f) => f.text)
  ].join('\n');
  const title = cleanDocumentTitle(options?.fileName);
  const detectedDirection = detectTextDirection(allText);
  const detectedLanguage = detectTextLanguage(allText);
  const wordCount = (allText.match(/\S+/g) || []).length;
  const stylesRecord = styleRegistry.size > 0 ? Object.fromEntries(styleRegistry.entries()) : undefined;

  return {
    title,
    paragraphs,
    footnotes: emittedFootnotes.length > 0 ? emittedFootnotes : undefined,
    tables: emittedTables.length > 0 ? emittedTables : undefined,
    multiFlowAnchors: anchorLinks.length > 0 ? anchorLinks : undefined,
    styles: stylesRecord,
    links: emittedLinks.length > 0 ? emittedLinks : undefined,
    format: 'docx',
    detectedLanguage,
    detectedDirection,
    wordCount,
    paragraphCount: paragraphs.length,
    metadata: {
      footnoteCount: emittedFootnotes.length,
      tableCount: emittedTables.length,
      styleCount: styleRegistry.size,
      linkCount: emittedLinks.length,
      styles: stylesRecord,
      customStyles: stylesRecord,
      links: emittedLinks.length > 0 ? emittedLinks : undefined
    }
  };
}

/**
 * Imports DOCX document buffer (extracting document.xml, footnotes.xml, numbering.xml, and styles.xml).
 */
export function importDocx(
  content: Uint8Array | ArrayBuffer,
  options?: ImportOptions
): ImportResult {
  const archive = extractDocxArchive(content, options);
  return parseDocxXml(archive.documentXml, {
    ...options,
    docxXml: archive.documentXml,
    footnotesXml: archive.footnotesXml,
    endnotesXml: archive.endnotesXml,
    numberingXml: archive.numberingXml,
    stylesXml: archive.stylesXml
  });
}

/**
 * Unified entry point for importing any supported document file (TXT, Markdown, DOCX).
 */
export function importDocument(
  content: string | Uint8Array | ArrayBuffer,
  options?: ImportOptions
): ImportResult {
  const fileName = options?.fileName || '';
  const ext = fileName.split('.').pop()?.toLowerCase();

  // 1. Explicit format or filename match
  if (options?.format === 'docx' || ext === 'docx') {
    if (typeof content === 'string') {
      if (content.includes('<w:document') || content.includes('<w:body')) {
        return parseDocxXml(content, options);
      }
      throw new DocumentImportError(
        'INVALID_STRUCTURE',
        'DOCX format requires binary buffer input',
        'קובץ DOCX דורש נתונים בינאריים'
      );
    }
    return importDocx(content, options);
  }

  if (options?.format === 'md' || options?.format === 'markdown' || ext === 'md' || ext === 'markdown') {
    return importMarkdown(content, options);
  }

  if (options?.format === 'txt' || ext === 'txt') {
    return importPlainText(content, options);
  }

  // 2. Auto-detect from binary signature
  if (typeof content !== 'string') {
    const bytes = content instanceof Uint8Array ? content : new Uint8Array(content);
    if (bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4B && bytes[2] === 0x03 && bytes[3] === 0x04) {
      return importDocx(bytes, options);
    }
  }

  // 3. Auto-detect Markdown vs Plain Text
  const decoded = typeof content === 'string' ? content : decodeTextBuffer(content);
  const hasMarkdownTokens = /^#\s+|\n#\s+|\*\*|__|\n[-*+]\s+/m.test(decoded);
  if (hasMarkdownTokens) {
    return importMarkdown(content, options);
  }

  return importPlainText(content, options);
}



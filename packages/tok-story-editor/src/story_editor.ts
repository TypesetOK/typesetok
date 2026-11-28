/**
 * Continuous Unpaginated Story Editor (Section 6.1).
 *
 * Dedicated editor panel decoupled from page geometry, similar to InDesign's Story Editor.
 * Allows authors to write and edit long texts with Niqqud without the overhead of page pagination.
 */

import {
  CharacterFormat,
  ParagraphStylePatch,
  StoryParagraph,
  StorySelection,
  clearRangeInSpans,
  formatRangeInSpans,
  insertSpanRange,
  mergeParagraphs,
  removeSpanRange,
  splitParagraph
} from './types.js';
import { renderParagraphDom, parseParagraphDom } from './dom_converter.js';
import { StoryHistory, cloneParagraph, cloneParagraphs } from './history.js';
import {
  FindOptions,
  FindResult,
  findAllInParagraphs,
  findNextInParagraphs,
  findPrevInParagraphs
} from './find_replace.js';

export { StoryParagraph, StorySelection, FindOptions, FindResult };


export class StoryEditor {
  private container: HTMLElement;
  private editorEl: HTMLDivElement;
  private history: StoryHistory = new StoryHistory();
  private lastSavedParagraphs: StoryParagraph[] = [];
  private lastSelection: StorySelection | null = null;
  private onTextChangeCallback?: (paraId: string, newText: string) => void;
  private onStoryChangeCallback?: (paragraphs: StoryParagraph[]) => void;

  private historyChangeCallbacks: Array<(canUndo: boolean, canRedo: boolean) => void> = [];
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;
  private debounceMs = 200;
  private onSelectionChange = () => this.handleSelectionChange();


  constructor(container: HTMLElement) {
    this.container = container;
    this.editorEl = document.createElement('div');
    this.editorEl.className = 'tok-story-editor-content';
    this.editorEl.dir = 'rtl';
    this.editorEl.contentEditable = 'true';
    this.editorEl.lang = 'he';
    // Pointed (niqqud) Hebrew is flagged almost word-by-word by the spellchecker.
    this.editorEl.spellcheck = false;
    this.editorEl.setAttribute('role', 'textbox');
    this.editorEl.setAttribute('aria-multiline', 'true');
    this.editorEl.setAttribute('aria-label', 'Story editor / עורך סיפור');
    // Typography and colors come from the host stylesheet (.tok-story-editor-content),
    // so the editor follows the light/dark theme.
    this.editorEl.style.outline = 'none';
    // The workbench sets `user-select: none` on <body>; without overriding it here the
    // editor's text cannot be selected (no drag-select, no Shift+Arrow, no copy).
    this.editorEl.style.userSelect = 'text';
    this.editorEl.style.setProperty('-webkit-user-select', 'text');

    this.container.appendChild(this.editorEl);

    this.editorEl.addEventListener('input', () => {
      this.handleInput();
    });

    this.editorEl.addEventListener('keydown', (e: KeyboardEvent) => {
      const isCtrlOrCmd = e.ctrlKey || e.metaKey;

      if (isCtrlOrCmd && !e.altKey) {
        const code = e.code;
        const key = e.key.toLowerCase();
        if (code === 'KeyZ' || key === 'z' || key === 'ז') {
          e.preventDefault();
          if (e.shiftKey) {
            this.redo();
          } else {
            this.undo();
          }
          return;
        } else if (code === 'KeyY' || key === 'y' || key === 'ט') {
          e.preventDefault();
          this.redo();
          return;
        } else if (code === 'KeyB' || key === 'b' || key === 'נ') {
          e.preventDefault();
          this.toggleFormatSelection('bold');
          return;
        } else if (code === 'KeyI' || key === 'i' || key === 'ן') {
          e.preventDefault();
          this.toggleFormatSelection('italic');
          return;
        } else if (code === 'KeyU' || key === 'u' || key === 'ו') {
          e.preventDefault();
          this.toggleFormatSelection('underline');
          return;
        } else if (code === 'Backslash' || key === '\\') {
          e.preventDefault();
          this.clearFormattingSelection();
          return;
        }
      }

      if (e.key === 'Tab') {
        e.preventDefault();
        const sel = this.getStorySelection();
        if (sel) {
          if (e.shiftKey) {
            this.outdentParagraph(sel.paraId);
          } else {
            this.indentParagraph(sel.paraId);
          }
        }
        return;
      }

      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        this.splitParagraphAtCaret();
        return;
      }

      if (e.key === 'Backspace') {
        const sel = this.getStorySelection();
        if (sel && sel.isCollapsed && sel.start === 0) {
          const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(sel.paraId)}"]`);
          const prevEl = pEl?.previousElementSibling as HTMLElement | null;
          if (prevEl && prevEl.dataset.paraId) {
            e.preventDefault();
            this.mergeParagraphWithPrevious(sel.paraId);
            return;
          }
        }
      }
    });

    this.editorEl.addEventListener('beforeinput', (e: InputEvent) => {
      if (e.inputType === 'historyUndo') {
        e.preventDefault();
        this.undo();
      } else if (e.inputType === 'historyRedo') {
        e.preventDefault();
        this.redo();
      }
    });

    // Highlight the paragraph that holds the caret.
    document.addEventListener('selectionchange', this.onSelectionChange);
  }

  public destroy(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
    }
    document.removeEventListener('selectionchange', this.onSelectionChange);
    this.editorEl.remove();
  }

  public setDebounceMs(ms: number): void {
    this.debounceMs = ms;
  }

  public flush(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
    }
    const domParas = this.parseAllParagraphsDom();
    if (domParas.length > 0) {
      this.lastSavedParagraphs = domParas;
    }
    if (this.onStoryChangeCallback) {
      this.onStoryChangeCallback(this.cloneParagraphs(this.lastSavedParagraphs));
    }
  }


  /** The editable element (for focusing and scrolling from the host). */
  public getElement(): HTMLElement {
    return this.editorEl;
  }

  /** Editor text size in px (display only; the document's typography is unchanged). */
  public setFontSize(px: number): void {
    this.editorEl.style.fontSize = `${px}px`;
  }

  private handleSelectionChange(): void {
    this.markCurrentParagraph();
    const sel = this.getStorySelection();
    if (sel) {
      this.lastSelection = sel;
    }
  }

  private markCurrentParagraph(): void {
    const sel = window.getSelection();
    let node: Node | null = sel && sel.anchorNode && this.editorEl.contains(sel.anchorNode) ? sel.anchorNode : null;
    while (node && node !== this.editorEl && node.nodeName !== 'P') node = node.parentNode;
    const current = node && node.nodeName === 'P' ? (node as HTMLElement) : null;
    this.editorEl.querySelectorAll('p.tok-current').forEach((p) => {
      if (p !== current) p.classList.remove('tok-current');
    });
    current?.classList.add('tok-current');
  }

  private renderParagraphs(paragraphs: StoryParagraph[]): void {
    const frag = document.createDocumentFragment();
    for (const p of paragraphs) {
      const pEl = renderParagraphDom(p);
      frag.appendChild(pEl);
    }
    this.editorEl.replaceChildren(frag);
  }

  private cloneParagraphs(paragraphs: StoryParagraph[]): StoryParagraph[] {
    return paragraphs.map((p) => ({
      id: p.id,
      styleId: p.styleId,
      text: p.text,
      spans: p.spans ? p.spans.map((s) => ({ start: s.start, end: s.end, format: { ...s.format } })) : undefined,
      patch: p.patch ? { ...p.patch } : undefined
    }));
  }

  public loadStory(paragraphs: StoryParagraph[], resetHistory = true): void {
    this.renderParagraphs(paragraphs);
    this.lastSavedParagraphs = this.cloneParagraphs(paragraphs);
    if (resetHistory) {
      this.history.clear();
    }
    this.notifyHistoryChange();
  }

  /** Current paragraphs as edited with semantic spans and style patches. */
  public getStory(freshFromDom = false): StoryParagraph[] {
    if (freshFromDom) {
      if (this.debounceTimer) {
        clearTimeout(this.debounceTimer);
        this.debounceTimer = null;
      }
      const domParas = this.parseAllParagraphsDom();
      if (domParas.length > 0 || this.lastSavedParagraphs.length === 0) {
        this.lastSavedParagraphs = domParas;
      }
    } else if (this.lastSavedParagraphs.length === 0) {
      this.lastSavedParagraphs = this.parseAllParagraphsDom();
    }
    return this.cloneParagraphs(this.lastSavedParagraphs);
  }

  public parseAllParagraphsDom(): StoryParagraph[] {
    const queried = Array.from(this.editorEl.querySelectorAll<HTMLElement>('p[data-para-id]'));
    if (queried.length > 0) {
      return queried.map((p) => parseParagraphDom(p));
    }
    const directChildren = Array.from(this.editorEl.children || []).filter(
      (c): c is HTMLElement =>
        (c as HTMLElement).tagName?.toUpperCase() === 'P' && Boolean((c as HTMLElement).dataset?.paraId)
    );
    return directChildren.map((p) => parseParagraphDom(p));
  }

  /** Retrieve a single paragraph by ID */
  public getParagraph(paraId: string): StoryParagraph | null {
    const cached = this.lastSavedParagraphs.find((p) => p.id === paraId);
    if (cached) return cloneParagraph(cached);
    const el = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
    return el ? parseParagraphDom(el) : null;
  }


  public onTextChange(cb: (paraId: string, newText: string) => void): void {
    this.onTextChangeCallback = cb;
  }

  public onStoryChange(cb: (paragraphs: StoryParagraph[]) => void): void {
    this.onStoryChangeCallback = cb;
  }

  public onHistoryChange(cb: (canUndo: boolean, canRedo: boolean) => void): () => void {
    this.historyChangeCallbacks.push(cb);
    cb(this.canUndo(), this.canRedo());
    return () => {
      this.historyChangeCallbacks = this.historyChangeCallbacks.filter((c) => c !== cb);
    };
  }

  public canUndo(): boolean {
    return this.history.canUndo();
  }

  public canRedo(): boolean {
    return this.history.canRedo();
  }

  public getUndoCount(): number {
    return this.history.getUndoCount();
  }

  public getRedoCount(): number {
    return this.history.getRedoCount();
  }

  public undo(): boolean {
    const state = this.history.undo();
    if (!state) return false;
    this.renderParagraphs(state.paragraphs);
    this.lastSavedParagraphs = cloneParagraphs(state.paragraphs);
    if (state.selection) {
      this.setStorySelection(state.selection);
    }
    this.notifyHistoryChange();
    if (this.onTextChangeCallback) {
      this.onTextChangeCallback('', '');
    }
    if (this.onStoryChangeCallback) {
      this.onStoryChangeCallback(state.paragraphs);
    }
    return true;
  }

  public redo(): boolean {
    const state = this.history.redo();
    if (!state) return false;
    this.renderParagraphs(state.paragraphs);
    this.lastSavedParagraphs = cloneParagraphs(state.paragraphs);
    if (state.selection) {
      this.setStorySelection(state.selection);
    }
    this.notifyHistoryChange();
    if (this.onTextChangeCallback) {
      this.onTextChangeCallback('', '');
    }
    if (this.onStoryChangeCallback) {
      this.onStoryChangeCallback(state.paragraphs);
    }
    return true;
  }

  public executeTransaction(
    description: string,
    fn: () => void,
    hooks?: { onUndo?: () => void; onRedo?: () => void }
  ): void {
    const before = this.getStory();
    const selBefore = this.getStorySelection();

    fn();

    this.dedupeParagraphIds();
    const after = this.getStory();
    const selAfter = this.getStorySelection();

    this.history.record(before, after, {
      description,
      isTyping: false,
      selectionBefore: selBefore || undefined,
      selectionAfter: selAfter || undefined,
      onUndo: hooks?.onUndo,
      onRedo: hooks?.onRedo
    });

    this.lastSavedParagraphs = cloneParagraphs(after);
    this.lastSelection = selAfter;
    this.notifyHistoryChange();

    if (this.onTextChangeCallback) {
      this.onTextChangeCallback('', '');
    }
    if (this.onStoryChangeCallback) {
      this.onStoryChangeCallback(after);
    }
  }

  public getSelectionFormat(): CharacterFormat {
    const sel = this.getStorySelection();
    if (!sel) return {};
    const p = this.getParagraph(sel.paraId);
    if (!p || !p.spans || p.spans.length === 0) return {};

    const offset = sel.start;
    const match =
      p.spans.find((s) => s.start <= offset && s.end > offset) ||
      (offset > 0 ? p.spans.find((s) => s.end === offset) : undefined);
    return match ? { ...match.format } : {};
  }

  public toggleFormatSelection(style: 'bold' | 'italic' | 'underline'): void {
    const sel = this.getStorySelection();
    const currentFmt = this.getSelectionFormat();
    const nextFmt: Partial<CharacterFormat> = {};

    if (style === 'bold') {
      const p = sel ? this.getParagraph(sel.paraId) : null;
      const isBold = ((currentFmt.fontWeight ?? p?.patch?.fontWeight ?? 400) >= 600);
      nextFmt.fontWeight = isBold ? 400 : 700;
      if (sel && sel.isCollapsed) {
        this.setParagraphPatch(sel.paraId, { fontWeight: nextFmt.fontWeight });
        return;
      }
    } else if (style === 'italic') {
      nextFmt.italic = !currentFmt.italic;
    } else if (style === 'underline') {
      nextFmt.underline = !currentFmt.underline;
    }

    this.formatSelection(nextFmt);
  }

  public formatSelection(format: Partial<CharacterFormat>): void {
    const sel = this.getStorySelection();
    if (!sel || sel.isCollapsed) return;

    this.executeTransaction('החלת עיצוב תווים', () => {
      const allParas = Array.from(this.editorEl.querySelectorAll<HTMLElement>('p[data-para-id]'));
      const aId = sel.anchorParaId || sel.paraId;
      const fId = sel.focusParaId || sel.paraId;

      let inRange = false;
      for (const pEl of allParas) {
        const pid = pEl.dataset.paraId!;
        const isStart = pid === aId;
        const isEnd = pid === fId;

        if (aId === fId && isStart) {
          const p = parseParagraphDom(pEl);
          const charsLen = Array.from(p.text).length;
          const start = Math.min(sel.start, sel.end);
          const end = Math.max(sel.start, sel.end);
          p.spans = formatRangeInSpans(p.spans || [], start, end, format as CharacterFormat, charsLen);
          const newEl = renderParagraphDom(p);
          pEl.replaceWith(newEl);
          break;
        }

        if (isStart || isEnd) {
          const p = parseParagraphDom(pEl);
          const charsLen = Array.from(p.text).length;
          if (!inRange) {
            inRange = true;
            const start = isStart ? sel.anchorOffset ?? sel.start : sel.focusOffset ?? sel.start;
            p.spans = formatRangeInSpans(p.spans || [], start, charsLen, format as CharacterFormat, charsLen);
          } else {
            const end = isEnd ? sel.focusOffset ?? sel.end : sel.anchorOffset ?? sel.end;
            p.spans = formatRangeInSpans(p.spans || [], 0, end, format as CharacterFormat, charsLen);
            inRange = false;
          }
          const newEl = renderParagraphDom(p);
          pEl.replaceWith(newEl);
        } else if (inRange) {
          const p = parseParagraphDom(pEl);
          const charsLen = Array.from(p.text).length;
          p.spans = formatRangeInSpans(p.spans || [], 0, charsLen, format as CharacterFormat, charsLen);
          const newEl = renderParagraphDom(p);
          pEl.replaceWith(newEl);
        }
      }
    });

    this.setStorySelection(sel);
  }

  public clearFormattingSelection(): void {
    const sel = this.getStorySelection();
    if (!sel || sel.isCollapsed) return;

    this.executeTransaction('ניקוי עיצוב', () => {
      const allParas = Array.from(this.editorEl.querySelectorAll<HTMLElement>('p[data-para-id]'));
      const aId = sel.anchorParaId || sel.paraId;
      const fId = sel.focusParaId || sel.paraId;

      let inRange = false;
      for (const pEl of allParas) {
        const pid = pEl.dataset.paraId!;
        const isStart = pid === aId;
        const isEnd = pid === fId;

        if (aId === fId && isStart) {
          const p = parseParagraphDom(pEl);
          const charsLen = Array.from(p.text).length;
          const start = Math.min(sel.start, sel.end);
          const end = Math.max(sel.start, sel.end);
          p.spans = clearRangeInSpans(p.spans || [], start, end, charsLen);
          const newEl = renderParagraphDom(p);
          pEl.replaceWith(newEl);
          break;
        }

        if (isStart || isEnd) {
          const p = parseParagraphDom(pEl);
          const charsLen = Array.from(p.text).length;
          if (!inRange) {
            inRange = true;
            const start = isStart ? sel.anchorOffset ?? sel.start : sel.focusOffset ?? sel.start;
            p.spans = clearRangeInSpans(p.spans || [], start, charsLen, charsLen);
          } else {
            const end = isEnd ? sel.focusOffset ?? sel.end : sel.anchorOffset ?? sel.end;
            p.spans = clearRangeInSpans(p.spans || [], 0, end, charsLen);
            inRange = false;
          }
          const newEl = renderParagraphDom(p);
          pEl.replaceWith(newEl);
        } else if (inRange) {
          const p = parseParagraphDom(pEl);
          p.spans = [];
          const newEl = renderParagraphDom(p);
          pEl.replaceWith(newEl);
        }
      }
    });

    this.setStorySelection(sel);
  }

  public setParagraphPatch(paraId: string, patch: Partial<ParagraphStylePatch>): void {
    const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
    if (!pEl) return;

    this.executeTransaction('עיצוב פסקה', () => {
      const p = parseParagraphDom(pEl);
      p.patch = { ...(p.patch || {}), ...patch };
      const newEl = renderParagraphDom(p);
      pEl.replaceWith(newEl);
    });
  }

  public setParagraphStyle(paraId: string, styleId: string): void {
    const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
    if (!pEl) return;

    this.executeTransaction('סגנון פסקה', () => {
      const p = parseParagraphDom(pEl);
      p.styleId = styleId;
      const newEl = renderParagraphDom(p);
      pEl.replaceWith(newEl);
    });
  }

  public indentParagraph(paraId: string, deltaPt = 14.17): void {
    const p = this.getParagraph(paraId);
    if (!p) return;
    const currentIndent = p.patch?.firstLineIndentPt ?? 0;
    this.setParagraphPatch(paraId, { firstLineIndentPt: Math.max(0, currentIndent + deltaPt) });
  }

  public outdentParagraph(paraId: string, deltaPt = 14.17): void {
    const p = this.getParagraph(paraId);
    if (!p) return;
    const currentIndent = p.patch?.firstLineIndentPt ?? 0;
    this.setParagraphPatch(paraId, { firstLineIndentPt: Math.max(0, currentIndent - deltaPt) });
  }

  public splitParagraphAtCaret(): void {
    const sel = this.getStorySelection();
    if (!sel) return;

    const p = this.getParagraph(sel.paraId);
    if (!p) return;

    const currentOffset = Math.min(sel.start, Array.from(p.text).length);
    const newId = `${p.id}_${Math.random().toString(36).substring(2, 7)}`;

    this.executeTransaction('פיצול פסקה', () => {
      const [p1, p2] = splitParagraph(p, currentOffset, newId);
      const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(p.id)}"]`);
      if (pEl) {
        const newP1 = renderParagraphDom(p1);
        const newP2 = renderParagraphDom(p2);
        pEl.replaceWith(newP1, newP2);
      }
    });

    this.setStorySelection({
      paraId: newId,
      start: 0,
      end: 0,
      isCollapsed: true,
      anchorParaId: newId,
      anchorOffset: 0,
      focusParaId: newId,
      focusOffset: 0
    });
  }

  public mergeParagraphWithPrevious(paraId: string): void {
    const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
    if (!pEl) return;
    const prevEl = pEl.previousElementSibling as HTMLElement | null;
    if (!prevEl || !prevEl.dataset.paraId) return;

    const prevId = prevEl.dataset.paraId;
    const prevPara = parseParagraphDom(prevEl);
    const currPara = parseParagraphDom(pEl);

    const prevLen = Array.from(prevPara.text).length;

    this.executeTransaction('מיזוג פסקאות', () => {
      const merged = mergeParagraphs(prevPara, currPara);
      const newPrevEl = renderParagraphDom(merged);
      prevEl.replaceWith(newPrevEl);
      pEl.remove();
    });

    this.setStorySelection({
      paraId: prevId,
      start: prevLen,
      end: prevLen,
      isCollapsed: true,
      anchorParaId: prevId,
      anchorOffset: prevLen,
      focusParaId: prevId,
      focusOffset: prevLen
    });
  }

  public findAll(options: FindOptions): FindResult[] {
    return findAllInParagraphs(this.getStory(), options);
  }

  public findNext(options: FindOptions): FindResult | null {
    const sel = this.getStorySelection();
    const fromParaId = sel ? sel.paraId : '';
    const fromOffset = sel ? sel.end : 0;
    const match = findNextInParagraphs(this.getStory(), fromParaId, fromOffset, options);
    if (match) {
      this.setStorySelection({
        paraId: match.paraId,
        start: match.start,
        end: match.end,
        isCollapsed: false,
        anchorParaId: match.paraId,
        anchorOffset: match.start,
        focusParaId: match.paraId,
        focusOffset: match.end
      });
    }
    return match;
  }

  public findPrev(options: FindOptions): FindResult | null {
    const sel = this.getStorySelection();
    const fromParaId = sel ? sel.paraId : '';
    const fromOffset = sel ? sel.start : 0;
    const match = findPrevInParagraphs(this.getStory(), fromParaId, fromOffset, options);
    if (match) {
      this.setStorySelection({
        paraId: match.paraId,
        start: match.start,
        end: match.end,
        isCollapsed: false,
        anchorParaId: match.paraId,
        anchorOffset: match.start,
        focusParaId: match.paraId,
        focusOffset: match.end
      });
    }
    return match;
  }

  public replaceNext(replacement: string, options: FindOptions): boolean {
    const sel = this.getStorySelection();
    let match: FindResult | null = null;

    if (sel && !sel.isCollapsed) {
      const p = this.getParagraph(sel.paraId);
      if (p) {
        const textSlice = Array.from(p.text).slice(sel.start, sel.end).join('');
        const matches = findAllInParagraphs(
          [{ id: p.id, styleId: p.styleId, text: textSlice }],
          options
        );
        if (matches.length > 0 && matches[0].start === 0 && matches[0].end === (sel.end - sel.start)) {
          match = {
            paraId: sel.paraId,
            paraIndex: 0,
            start: sel.start,
            end: sel.end,
            matchedText: textSlice
          };
        }
      }
    }

    if (!match) {
      match = this.findNext(options);
    }

    if (!match) return false;

    const targetMatch = match;
    const repLen = Array.from(replacement).length;

    this.executeTransaction('החלפה', () => {
      const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(targetMatch.paraId)}"]`);
      if (pEl) {
        const p = parseParagraphDom(pEl);
        const updated = this.replaceRangeInParagraph(p, targetMatch.start, targetMatch.end, replacement);
        const newEl = renderParagraphDom(updated);
        pEl.replaceWith(newEl);
      }
    });

    this.setStorySelection({
      paraId: targetMatch.paraId,
      start: targetMatch.start,
      end: targetMatch.start + repLen,
      isCollapsed: false,
      anchorParaId: targetMatch.paraId,
      anchorOffset: targetMatch.start,
      focusParaId: targetMatch.paraId,
      focusOffset: targetMatch.start + repLen
    });

    return true;
  }

  public replaceAll(replacement: string, options: FindOptions): number {
    const allMatches = findAllInParagraphs(this.getStory(), options);
    if (allMatches.length === 0) return 0;

    const byPara = new Map<string, FindResult[]>();
    for (const m of allMatches) {
      const list = byPara.get(m.paraId) || [];
      list.push(m);
      byPara.set(m.paraId, list);
    }

    this.executeTransaction('החלף הכל', () => {
      for (const [paraId, matches] of byPara.entries()) {
        const pEl = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(paraId)}"]`);
        if (!pEl) continue;

        let p = parseParagraphDom(pEl);
        matches.sort((a, b) => b.start - a.start);

        for (const m of matches) {
          p = this.replaceRangeInParagraph(p, m.start, m.end, replacement);
        }

        const newEl = renderParagraphDom(p);
        pEl.replaceWith(newEl);
      }
    });

    return allMatches.length;
  }

  private replaceRangeInParagraph(
    p: StoryParagraph,
    start: number,
    end: number,
    replacement: string
  ): StoryParagraph {
    const chars = Array.from(p.text);
    const repChars = Array.from(replacement);
    const newChars = [...chars.slice(0, start), ...repChars, ...chars.slice(end)];
    const newText = newChars.join('');

    const afterRemove = removeSpanRange(p.spans || [], start, end, chars.length);
    const newSpans = insertSpanRange(afterRemove, start, repChars.length, [], chars.length - (end - start));

    return {
      ...p,
      text: newText,
      spans: newSpans
    };
  }



  private notifyHistoryChange(): void {
    const u = this.canUndo();
    const r = this.canRedo();
    for (const cb of this.historyChangeCallbacks) {
      try {
        cb(u, r);
      } catch (e) {
        console.error(e);
      }
    }
  }

  private handleInput(): void {
    const sel = window.getSelection();
    let node: Node | null = sel && sel.anchorNode;
    while (node && node !== this.editorEl && node.nodeName !== 'P') {
      node = node.parentNode;
    }

    const pEl = node && node.nodeName === 'P' ? (node as HTMLParagraphElement) : null;
    const currentParagraphEls = this.editorEl.querySelectorAll('p[data-para-id]');
    let currentParagraphs: StoryParagraph[];
    let targetParaId: string | undefined;
    let targetText = '';

    if (pEl && pEl.dataset.paraId && currentParagraphEls.length === this.lastSavedParagraphs.length) {
      // Selective O(1) single-paragraph update: no need to re-parse entire document!
      targetParaId = pEl.dataset.paraId;
      targetText = pEl.textContent || '';
      const updatedP = parseParagraphDom(pEl);
      const idx = this.lastSavedParagraphs.findIndex((p) => p.id === targetParaId);
      if (idx >= 0) {
        currentParagraphs = [...this.lastSavedParagraphs];
        currentParagraphs[idx] = updatedP;
      } else {
        currentParagraphs = this.parseAllParagraphsDom();
      }
    } else {
      this.dedupeParagraphIds();
      currentParagraphs = this.parseAllParagraphsDom();
      targetParaId = pEl ? pEl.dataset.paraId : undefined;
      targetText = pEl ? pEl.textContent || '' : '';
    }

    const currentSel = this.getStorySelection();

    this.history.record(this.lastSavedParagraphs, currentParagraphs, {
      isTyping: true,
      selectionBefore: this.lastSelection || undefined,
      selectionAfter: currentSel || undefined
    });

    this.lastSavedParagraphs = currentParagraphs;
    this.lastSelection = currentSel;
    this.notifyHistoryChange();

    if (this.onTextChangeCallback) {
      this.onTextChangeCallback(targetParaId || '', targetText);
    }

    this.scheduleStoryChange(currentParagraphs);
  }

  private scheduleStoryChange(paragraphs: StoryParagraph[]): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    this.debounceTimer = setTimeout(() => {
      this.debounceTimer = null;
      if (this.onStoryChangeCallback) {
        this.onStoryChangeCallback(paragraphs);
      }
    }, this.debounceMs);
  }


  public getStorySelection(): StorySelection | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !sel.anchorNode || !sel.focusNode) return null;
    if (!this.editorEl.contains(sel.anchorNode) || !this.editorEl.contains(sel.focusNode)) return null;

    let anchorP: HTMLElement | null = null;
    let curr: Node | null = sel.anchorNode;
    while (curr && curr !== this.editorEl) {
      if (curr.nodeName === 'P' && (curr as HTMLElement).dataset.paraId) {
        anchorP = curr as HTMLElement;
        break;
      }
      curr = curr.parentNode;
    }

    let focusP: HTMLElement | null = null;
    curr = sel.focusNode;
    while (curr && curr !== this.editorEl) {
      if (curr.nodeName === 'P' && (curr as HTMLElement).dataset.paraId) {
        focusP = curr as HTMLElement;
        break;
      }
      curr = curr.parentNode;
    }

    if (!anchorP || !focusP) return null;

    const anchorParaId = anchorP.dataset.paraId!;
    const focusParaId = focusP.dataset.paraId!;
    const anchorOffset = this.getOffsetInParagraph(anchorP, sel.anchorNode, sel.anchorOffset);
    const focusOffset = this.getOffsetInParagraph(focusP, sel.focusNode, sel.focusOffset);

    const isCollapsed = sel.isCollapsed;
    const isSinglePara = anchorParaId === focusParaId;
    const start = isSinglePara ? Math.min(anchorOffset, focusOffset) : anchorOffset;
    const end = isSinglePara ? Math.max(anchorOffset, focusOffset) : anchorOffset;

    return {
      paraId: anchorParaId,
      start,
      end,
      isCollapsed,
      anchorParaId,
      anchorOffset,
      focusParaId,
      focusOffset
    };
  }

  public setStorySelection(selInfo: StorySelection): void {
    const aId = selInfo.anchorParaId || selInfo.paraId;
    const fId = selInfo.focusParaId || selInfo.paraId;
    const aOff = selInfo.anchorOffset !== undefined ? selInfo.anchorOffset : selInfo.start;
    const fOff = selInfo.focusOffset !== undefined ? selInfo.focusOffset : selInfo.end;

    const anchorP = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(aId)}"]`);
    const focusP = this.editorEl.querySelector<HTMLElement>(`p[data-para-id="${CSS.escape(fId)}"]`);
    if (!anchorP || !focusP) return;

    const sel = window.getSelection();
    if (!sel) return;

    const anchorPoint = this.findNodeAndOffset(anchorP, aOff);
    const focusPoint = this.findNodeAndOffset(focusP, fOff);

    try {
      if (typeof sel.setBaseAndExtent === 'function') {
        sel.setBaseAndExtent(anchorPoint.node, anchorPoint.offset, focusPoint.node, focusPoint.offset);
      } else {
        const range = document.createRange();
        range.setStart(anchorPoint.node, anchorPoint.offset);
        range.setEnd(focusPoint.node, focusPoint.offset);
        sel.removeAllRanges();
        sel.addRange(range);
      }
    } catch {
      // Fallback if range fails in disconnected or synthetic tree
    }
  }

  private getOffsetInParagraph(paraEl: HTMLElement, targetNode: Node, targetOffset: number): number {
    let offset = 0;
    const walker = document.createTreeWalker(paraEl, NodeFilter.SHOW_TEXT);
    let node: Node | null = walker.nextNode();
    while (node) {
      if (node === targetNode) {
        return offset + targetOffset;
      }
      offset += (node.textContent || '').length;
      node = walker.nextNode();
    }
    if (targetNode === paraEl || targetNode.nodeType === Node.ELEMENT_NODE) {
      let count = 0;
      for (let i = 0; i < targetOffset && i < targetNode.childNodes.length; i++) {
        count += targetNode.childNodes[i].textContent?.length || 0;
      }
      return count;
    }
    return offset;
  }

  private findNodeAndOffset(paraEl: HTMLElement, targetOffset: number): { node: Node; offset: number } {
    const walker = document.createTreeWalker(paraEl, NodeFilter.SHOW_TEXT);
    let node: Node | null = walker.nextNode();
    let remaining = targetOffset;
    let lastNode: Node | null = null;

    while (node) {
      lastNode = node;
      const len = (node.textContent || '').length;
      if (remaining <= len) {
        return { node, offset: remaining };
      }
      remaining -= len;
      node = walker.nextNode();
    }

    if (lastNode) {
      return { node: lastNode, offset: (lastNode.textContent || '').length };
    }
    return { node: paraEl, offset: 0 };
  }

  /**
   * Pressing Enter inside a paragraph makes Chromium split the <p> by cloning it,
   * data-para-id included. Both halves would then report the same id and each edit
   * would overwrite the other paragraph's text. The first occurrence keeps the id;
   * later copies get a fresh one.
   */
  private dedupeParagraphIds(): void {
    const seen = new Set<string>();
    const paragraphs = this.editorEl.querySelectorAll<HTMLElement>('p[data-para-id]');
    paragraphs.forEach((p) => {
      const id = p.dataset.paraId as string;
      if (seen.has(id)) {
        let n = 1;
        while (seen.has(`${id}.${n}`) || this.editorEl.querySelector(`p[data-para-id="${CSS.escape(`${id}.${n}`)}"]`)) n++;
        p.dataset.paraId = `${id}.${n}`;
      }
      seen.add(p.dataset.paraId as string);
    });
  }
}


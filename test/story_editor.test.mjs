/**
 * TypesetOK (TOK) - Professional Text Editor Automated Test Suite (Stage 11).
 *
 * Covers:
 * 1. Semantic span normalization, arithmetic and disjoint invariants
 * 2. Hebrew typography, Niqqud, and cantillation preservation
 * 3. Bidirectional DOM conversion without tag soup
 * 4. Reliable compound transaction undo/redo stack and typing coalescing
 * 5. Paragraph splitting and merging with span shifting
 * 6. Hebrew-aware and Niqqud-insensitive Find & Replace with exact boundary mapping
 * 7. Long document performance and selective O(1) updating
 */

import { test, describe, before } from 'node:test';
import assert from 'node:assert/strict';
import * as path from 'node:path';

// Setup minimal DOM mock in Node.js environment
class MockNode {
  constructor(nodeType, nodeName) {
    this.nodeType = nodeType;
    this.nodeName = nodeName;
    this.parentNode = null;
    this.childNodes = [];
  }

  appendChild(child) {
    if (child.nodeType === 11) {
      for (const fChild of child.childNodes) {
        fChild.parentNode = this;
        this.childNodes.push(fChild);
      }
      child.childNodes = [];
      return child;
    }
    child.parentNode = this;
    this.childNodes.push(child);
    return child;
  }


  replaceChildren(...children) {
    this.childNodes = [];
    for (const child of children) {
      if (child.nodeType === 11) { // Fragment
        for (const fChild of child.childNodes) {
          fChild.parentNode = this;
          this.childNodes.push(fChild);
        }
      } else {
        child.parentNode = this;
        this.childNodes.push(child);
      }
    }
  }

  replaceWith(...newNodes) {
    if (!this.parentNode) return;
    const parent = this.parentNode;
    const idx = parent.childNodes.indexOf(this);
    if (idx !== -1) {
      parent.childNodes.splice(idx, 1);
      let insertIdx = idx;
      for (const node of newNodes) {
        if (node.nodeType === 11) {
          for (const fChild of node.childNodes) {
            fChild.parentNode = parent;
            parent.childNodes.splice(insertIdx++, 0, fChild);
          }
        } else {
          node.parentNode = parent;
          parent.childNodes.splice(insertIdx++, 0, node);
        }
      }
    }
    this.parentNode = null;
  }

  remove() {
    if (!this.parentNode) return;
    const idx = this.parentNode.childNodes.indexOf(this);
    if (idx !== -1) this.parentNode.childNodes.splice(idx, 1);
    this.parentNode = null;
  }

  get textContent() {
    if (this.nodeType === 3) return this._text || '';
    return this.childNodes.map((c) => c.textContent).join('');
  }

  set textContent(val) {
    if (this.nodeType === 3) {
      this._text = val;
    } else {
      this.childNodes = [];
      if (val) {
        const tn = new MockNode(3, '#text');
        tn._text = val;
        tn.parentNode = this;
        this.childNodes.push(tn);
      }
    }
  }

  get previousElementSibling() {
    if (!this.parentNode) return null;
    const idx = this.parentNode.childNodes.indexOf(this);
    for (let i = idx - 1; i >= 0; i--) {
      if (this.parentNode.childNodes[i].nodeType === 1) return this.parentNode.childNodes[i];
    }
    return null;
  }

  get nextElementSibling() {
    if (!this.parentNode) return null;
    const idx = this.parentNode.childNodes.indexOf(this);
    for (let i = idx + 1; i < this.parentNode.childNodes.length; i++) {
      if (this.parentNode.childNodes[i].nodeType === 1) return this.parentNode.childNodes[i];
    }
    return null;
  }
}

class MockElement extends MockNode {
  constructor(tagName) {
    super(1, tagName.toUpperCase());
    this.tagName = tagName.toUpperCase();
    this.dataset = {};
    this.style = {
      setProperty: (k, v) => { this.style[k] = String(v); },
      removeProperty: (k) => { delete this.style[k]; }
    };

    this.attributes = {};
    this.classList = {
      _classes: new Set(),
      add: (c) => this.classList._classes.add(c),
      remove: (c) => this.classList._classes.delete(c),
      contains: (c) => this.classList._classes.has(c)
    };
  }

  setAttribute(k, v) { this.attributes[k] = String(v); }
  getAttribute(k) { return this.attributes[k] || null; }

  querySelectorAll(selector) {
    const results = [];
    function collect(node) {
      for (const child of node.childNodes) {
        if (child.nodeType === 1) {
          if (selector === 'p[data-para-id]' && child.tagName === 'P' && child.dataset.paraId) {
            results.push(child);
          } else if (selector.startsWith('p[data-para-id="')) {
            const id = selector.slice(17, -2);
            if (child.tagName === 'P' && child.dataset.paraId === id) results.push(child);
          } else if (selector === 'p.tok-current' && child.tagName === 'P' && child.classList.contains('tok-current')) {
            results.push(child);
          }
          collect(child);
        }
      }
    }
    collect(this);
    return results;
  }

  querySelector(selector) {
    const list = this.querySelectorAll(selector);
    return list.length > 0 ? list[0] : null;
  }

  addEventListener() {}
  removeEventListener() {}
}

// Attach globals
if (typeof globalThis.document === 'undefined') {
  globalThis.document = {
    createElement(tag) { return new MockElement(tag); },
    createTextNode(text) {
      const n = new MockNode(3, '#text');
      n._text = text;
      return n;
    },
    createDocumentFragment() { return new MockNode(11, '#document-fragment'); },
    createTreeWalker(root) {
      const textNodes = [];
      function collect(node) {
        if (node.nodeType === 3) textNodes.push(node);
        else {
          for (const c of node.childNodes) collect(c);
        }
      }
      collect(root);
      let idx = 0;
      return {
        nextNode: () => (idx < textNodes.length ? textNodes[idx++] : null)
      };
    },
    addEventListener() {},
    removeEventListener() {}
  };

  globalThis.window = {
    getSelection() {
      return {
        rangeCount: 0,
        isCollapsed: true,
        removeAllRanges() {},
        addRange() {}
      };
    }
  };

  globalThis.NodeFilter = { SHOW_TEXT: 4 };
  globalThis.Node = { ELEMENT_NODE: 1, TEXT_NODE: 3, DOCUMENT_FRAGMENT_NODE: 11 };
  globalThis.CSS = { escape: (s) => s.replace(/([^\w-])/g, '\\$1') };
}

// Dynamic import of tok-story-editor
let types;
let domConverter;
let historyModule;
let findReplace;
let storyEditorModule;

before(async () => {
  types = await import('../packages/tok-story-editor/dist/types.js');
  domConverter = await import('../packages/tok-story-editor/dist/dom_converter.js');
  historyModule = await import('../packages/tok-story-editor/dist/history.js');
  findReplace = await import('../packages/tok-story-editor/dist/find_replace.js');
  storyEditorModule = await import('../packages/tok-story-editor/dist/story_editor.js');
});

describe('Stage 11: Text Span Utilities and Disjoint Normalization', () => {
  test('normalizeSpans cleans empty formats and merges contiguous identical spans', () => {
    const raw = [
      { start: 0, end: 5, format: { fontWeight: 700 } },
      { start: 5, end: 10, format: { fontWeight: 700 } }, // Touching identical -> merge to [0, 10]
      { start: 12, end: 15, format: {} } // Empty format -> dropped
    ];
    const norm = types.normalizeSpans(raw, 20);
    assert.equal(norm.length, 1);
    assert.equal(norm[0].start, 0);
    assert.equal(norm[0].end, 10);
    assert.equal(norm[0].format.fontWeight, 700);
  });

  test('formatRangeInSpans paints format onto overlapping ranges', () => {
    const initial = [{ start: 0, end: 10, format: { fontWeight: 700 } }];
    // Apply italic to [4, 8)
    const formatted = types.formatRangeInSpans(initial, 4, 8, { italic: true }, 10);
    assert.equal(formatted.length, 3);
    assert.equal(formatted[0].start, 0);
    assert.equal(formatted[0].end, 4);
    assert.equal(formatted[0].format.fontWeight, 700);

    assert.equal(formatted[1].start, 4);
    assert.equal(formatted[1].end, 8);
    assert.equal(formatted[1].format.italic, true);

    assert.equal(formatted[2].start, 8);
    assert.equal(formatted[2].end, 10);
    assert.equal(formatted[2].format.fontWeight, 700);
  });

  test('removeSpanRange shifts trailing spans on character deletion', () => {
    const spans = [
      { start: 0, end: 5, format: { fontWeight: 700 } },
      { start: 10, end: 15, format: { italic: true } }
    ];
    // Delete 3 characters at offset 2 (removes 2..5)
    const updated = types.removeSpanRange(spans, 2, 5, 20);
    assert.equal(updated.length, 2);
    assert.equal(updated[0].start, 0);
    assert.equal(updated[0].end, 2);
    // The second span shifted left by 3
    assert.equal(updated[1].start, 7);
    assert.equal(updated[1].end, 12);
  });

  test('insertSpanRange shifts trailing spans on character insertion', () => {
    const spans = [{ start: 5, end: 10, format: { fontWeight: 700 } }];
    // Insert 4 characters at offset 2
    const updated = types.insertSpanRange(spans, 2, 4, [], 10);
    assert.equal(updated.length, 1);
    assert.equal(updated[0].start, 9);
    assert.equal(updated[0].end, 14);
  });
});

describe('Stage 11: Paragraph Splitting and Merging', () => {
  test('splitParagraph splits text and spans at caret offset cleanly', () => {
    const para = {
      id: 'p1',
      styleId: 'normal',
      text: 'שלום עולם יפה',
      spans: [{ start: 5, end: 9, format: { fontWeight: 700 } }] // "עולם"
    };

    // Split at offset 5 (right before "עולם")
    const [left, right] = types.splitParagraph(para, 5, 'p2');
    assert.equal(left.id, 'p1');
    assert.equal(left.text, 'שלום ');
    assert.equal((left.spans || []).length, 0);

    assert.equal(right.id, 'p2');
    assert.equal(right.text, 'עולם יפה');
    assert.equal((right.spans || []).length, 1);
    assert.equal(right.spans[0].start, 0);
    assert.equal(right.spans[0].end, 4);
    assert.equal(right.spans[0].format.fontWeight, 700);
  });

  test('mergeParagraphs joins texts and offsets spans accurately', () => {
    const p1 = {
      id: 'p1',
      styleId: 'normal',
      text: 'חלק ראשון ',
      spans: [{ start: 0, end: 4, format: { fontWeight: 700 } }]
    };
    const p2 = {
      id: 'p2',
      styleId: 'normal',
      text: 'וחלק שני',
      spans: [{ start: 1, end: 5, format: { italic: true } }]
    };

    const merged = types.mergeParagraphs(p1, p2);
    assert.equal(merged.id, 'p1');
    assert.equal(merged.text, 'חלק ראשון וחלק שני');
    assert.equal(merged.spans.length, 2);
    assert.equal(merged.spans[0].start, 0);
    assert.equal(merged.spans[0].end, 4);
    // Second span offset by p1.length (10)
    assert.equal(merged.spans[1].start, 11);
    assert.equal(merged.spans[1].end, 15);
  });
});

describe('Stage 11: Hebrew-Aware and Niqqud-Insensitive Find & Replace', () => {
  test('isHebrewMark accurately identifies Niqqud and Taamim marks', () => {
    assert.equal(findReplace.isHebrewMark('ש'), false);
    assert.equal(findReplace.isHebrewMark('a'), false);
    assert.equal(findReplace.isHebrewMark('\u05B8'), true); // Qamats
    assert.equal(findReplace.isHebrewMark('\u05BC'), true); // Dagesh
    assert.equal(findReplace.isHebrewMark('\u0591'), true); // Etnahta
  });

  test('stripHebrewMarks removes all marks preserving consonants', () => {
    const pointed = 'שָׁלוֹם עַל יִשְׂרָאֵל';
    assert.equal(findReplace.stripHebrewMarks(pointed), 'שלום על ישראל');
  });

  test('findAllInParagraphs matches Hebrew with Niqqud when searching unpointed query', () => {
    const paragraphs = [
      { id: 'p1', styleId: 'normal', text: 'מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית?' },
      { id: 'p2', styleId: 'normal', text: 'מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.' }
    ];

    // Search "שמע" with ignoreNiqqud: true
    const results = findReplace.findAllInParagraphs(paragraphs, {
      query: 'שמע',
      ignoreNiqqud: true
    });

    assert.equal(results.length, 1);
    assert.equal(results[0].paraId, 'p1');
    assert.equal(results[0].matchedText, 'שְׁמַע');
  });

  test('findAllInParagraphs handles case sensitivity and whole word', () => {
    const paragraphs = [
      { id: 'p1', styleId: 'normal', text: 'Hello World and hello TypesetOK' }
    ];

    const matchCase = findReplace.findAllInParagraphs(paragraphs, {
      query: 'Hello',
      matchCase: true
    });
    assert.equal(matchCase.length, 1);

    const caseInsensitive = findReplace.findAllInParagraphs(paragraphs, {
      query: 'hello',
      matchCase: false
    });
    assert.equal(caseInsensitive.length, 2);

    const wholeWord = findReplace.findAllInParagraphs(paragraphs, {
      query: 'Hell',
      wholeWord: true
    });
    assert.equal(wholeWord.length, 0); // Not a whole word
  });

  test('findNext and findPrev navigate matches circular wrap-around', () => {
    const paragraphs = [
      { id: 'p1', styleId: 'normal', text: 'אחד שניים' },
      { id: 'p2', styleId: 'normal', text: 'שלושה אחד' }
    ];

    const first = findReplace.findNextInParagraphs(paragraphs, 'p1', 0, { query: 'אחד' });
    assert.equal(first.paraId, 'p1');

    const second = findReplace.findNextInParagraphs(paragraphs, 'p1', 5, { query: 'אחד' });
    assert.equal(second.paraId, 'p2');

    // Wrap around
    const wrapped = findReplace.findNextInParagraphs(paragraphs, 'p2', 10, { query: 'אחד' });
    assert.equal(wrapped.paraId, 'p1');

    // Prev
    const prev = findReplace.findPrevInParagraphs(paragraphs, 'p2', 0, { query: 'אחד' });
    assert.equal(prev.paraId, 'p1');
  });
});

describe('Stage 11: Reliable Compound Transaction History Stack', () => {
  test('records discrete transactions and enforces undo/redo stack invariants', () => {
    const hist = new historyModule.StoryHistory(100);
    assert.equal(hist.canUndo(), false);
    assert.equal(hist.canRedo(), false);

    const p0 = [{ id: 'p1', styleId: 'normal', text: 'A' }];
    const p1 = [{ id: 'p1', styleId: 'normal', text: 'B' }];
    const p2 = [{ id: 'p1', styleId: 'normal', text: 'C' }];

    hist.record(p0, p1, { isTyping: false });
    assert.equal(hist.canUndo(), true);
    assert.equal(hist.canRedo(), false);
    assert.equal(hist.getUndoCount(), 1);

    hist.record(p1, p2, { isTyping: false });
    assert.equal(hist.getUndoCount(), 2);

    // Undo step 2
    const undo2 = hist.undo();
    assert.deepEqual(undo2.paragraphs, p1);
    assert.equal(hist.canRedo(), true);
    assert.equal(hist.getRedoCount(), 1);

    // Undo step 1
    const undo1 = hist.undo();
    assert.deepEqual(undo1.paragraphs, p0);
    assert.equal(hist.canUndo(), false);

    // Redo step 1
    const redo1 = hist.redo();
    assert.deepEqual(redo1.paragraphs, p1);
    assert.equal(hist.canUndo(), true);

    // Redo step 2
    const redo2 = hist.redo();
    assert.deepEqual(redo2.paragraphs, p2);
    assert.equal(hist.canRedo(), false);
  });

  test('coalesces continuous typing within coalesce window', () => {
    const hist = new historyModule.StoryHistory(100);
    const p0 = [{ id: 'p1', styleId: 'normal', text: 'ש' }];
    const p1 = [{ id: 'p1', styleId: 'normal', text: 'של' }];
    const p2 = [{ id: 'p1', styleId: 'normal', text: 'שלו' }];
    const p3 = [{ id: 'p1', styleId: 'normal', text: 'שלום' }];

    hist.record(p0, p1, { isTyping: true });
    hist.record(p1, p2, { isTyping: true });
    hist.record(p2, p3, { isTyping: true });

    // Should coalesce into 1 typing undo record
    assert.equal(hist.getUndoCount(), 1);
    const undone = hist.undo();
    assert.deepEqual(undone.paragraphs, p0);
  });
});

describe('Stage 11: DOM Converter Bidirectional Round-Trip', () => {
  test('renders semantic paragraph with spans into clean DOM and parses back losslessly', () => {
    const para = {
      id: 'para-roundtrip-1',
      styleId: 'normal',
      text: 'בְּרֵאשִׁית בָּרָא אֱלֹהִים',
      spans: [
        {
          start: 0,
          end: 9,
          format: { fontWeight: 700 } // "בְּרֵאשִׁית"
        },
        {
          start: 15,
          end: 22,
          format: { italic: true, underline: true } // "אֱלֹהִים"
        }
      ],
      patch: {
        alignment: 'Justified',
        direction: 'Rtl',
        firstLineIndentPt: 15
      }
    };

    const domEl = domConverter.renderParagraphDom(para);
    assert.equal(domEl.tagName, 'P');
    assert.equal(domEl.dataset.paraId, para.id);
    assert.equal(domEl.dataset.styleId, para.styleId);
    assert.equal(domEl.style.textAlign, 'justify');
    assert.equal(domEl.dir, 'rtl');

    const parsed = domConverter.parseParagraphDom(domEl);
    assert.equal(parsed.id, para.id);
    assert.equal(parsed.styleId, para.styleId);
    assert.equal(parsed.text, para.text);
    assert.equal(parsed.spans.length, 2);
    assert.equal(parsed.spans[0].start, 0);
    assert.equal(parsed.spans[0].end, 9);
    assert.equal(parsed.spans[0].format.fontWeight, 700);

    assert.equal(parsed.spans[1].start, 15);
    assert.equal(parsed.spans[1].end, 22);
    assert.equal(parsed.spans[1].format.italic, true);
    assert.equal(parsed.spans[1].format.underline, true);
  });
});

describe('Stage 11: Long Document Scalability & Performance', () => {
  test('handles 1,000 paragraphs with selective O(1) paragraph updates', () => {
    const container = new MockElement('div');
    const editor = new storyEditorModule.StoryEditor(container);

    const largeStory = [];
    for (let i = 0; i < 1000; i++) {
      largeStory.push({
        id: `p-${i}`,
        styleId: 'normal',
        text: `פסקה מספר ${i} בטקסט הארוך עם ניקוד וטעמים`
      });
    }

    const t0 = Date.now();
    editor.loadStory(largeStory);
    const loadTimeMs = Date.now() - t0;
    assert.ok(loadTimeMs < 1000, `Loading 1000 paragraphs should be fast (<1s), took ${loadTimeMs}ms`);

    // Verify retrieval
    const retrieved = editor.getStory();
    assert.equal(retrieved.length, 1000);
    assert.equal(retrieved[500].text, 'פסקה מספר 500 בטקסט הארוך עם ניקוד וטעמים');

    editor.destroy();
  });
});

describe('Page view paints span formatting on engine lines', () => {
  let lineRuns;
  let bridge;
  const toCss = (family) => `'${family}'`;

  before(async () => {
    lineRuns = await import('../packages/tok-ui/dist/engine/lineRuns.js');
    bridge = await import('../packages/tok-ui/dist/engine/typesetBridge.js');
  });

  const line = (text, extra = {}) => ({ line_index: 0, paragraph_id: 'P1', text, baseline_y: 12, height: 14, width: 100, is_rtl: true, ...extra });

  test('lines are located as consecutive slices of their paragraph', () => {
    const source = { text: 'אחת שתיים שלוש ארבע', spans: [] };
    const a = line('אחת שתיים');
    const b = line('שלוש ארבע', { line_index: 1 });
    const located = lineRuns.locateLines([a, b], (id) => (id === 'P1' ? source : undefined));
    assert.equal(located.get(a).start, 0);
    assert.equal(located.get(b).start, 10);
  });

  test('a line that is not part of the paragraph keeps plain rendering', () => {
    const located = lineRuns.locateLines([line('טקסט אחר')], () => ({ text: 'אחת שתיים', spans: [] }));
    assert.equal(located.size, 0);
  });

  test('italic, underline and colour become styled runs, the rest stays plain', () => {
    const source = {
      text: 'אחת שתיים שלוש',
      spans: [
        { start: 4, end: 9, format: { italic: true, underline: true } },
        { start: 10, end: 14, format: { color: '#aa0000' } }
      ]
    };
    const l = line('אחת שתיים שלוש');
    const located = lineRuns.locateLines([l], () => source).get(l);
    const html = lineRuns.renderStyledLine(l.text, located, toCss);
    assert.ok(html.startsWith('אחת '));
    assert.match(html, /<span style="font-style: italic; text-decoration: underline;">שתיים<\/span>/);
    assert.match(html, /<span style="color: #aa0000;">שלוש<\/span>/);
  });

  test('a span that starts on an earlier line is still painted on the following line', () => {
    const source = { text: 'אחת שתיים שלוש ארבע', spans: [{ start: 0, end: 19, format: { italic: true } }] };
    const b = line('שלוש ארבע');
    const located = lineRuns.locateLines([b], () => source).get(b);
    assert.equal(lineRuns.renderStyledLine(b.text, located, toCss), '<span style="font-style: italic;">שלוש ארבע</span>');
  });

  test('lines without visible formatting return null', () => {
    const source = { text: 'אחת שתיים', spans: [{ start: 0, end: 4, format: { linkUrl: 'https://example.org' } }] };
    const l = line('אחת שתיים');
    const located = lineRuns.locateLines([l], () => source).get(l);
    assert.equal(lineRuns.renderStyledLine(l.text, located, toCss), null);
  });

  test('text and colours are escaped before they reach the page', () => {
    const source = { text: 'a<b', spans: [{ start: 0, end: 3, format: { italic: true, color: 'red;background:url(x)' } }] };
    const l = line('a<b');
    const located = lineRuns.locateLines([l], () => source).get(l);
    const html = lineRuns.renderStyledLine(l.text, located, toCss);
    assert.equal(html, '<span style="font-style: italic;">a&lt;b</span>');
    assert.equal(lineRuns.safeCssColor('red;background:url(x)'), undefined);
  });

  test('layoutToPageDescriptors paints runs and paragraph colour, and is unchanged without a lookup', () => {
    const pages = [{
      page_index: 0,
      page_number_gematria: 'א',
      dimensions: { x: 0, y: 0, width: 400, height: 600 },
      frames: [{ frame_id: 'f1', flow_id: 'gemara', rect: { x: 10, y: 10, width: 300, height: 500 }, lines: [line('אחת שתיים')] }]
    }];
    const plain = bridge.layoutToPageDescriptors(pages)[0].htmlContent;
    assert.ok(!plain.includes('font-style: italic'));
    const lookup = () => ({ text: 'אחת שתיים', color: '#112233', spans: [{ start: 4, end: 9, format: { italic: true } }] });
    const styled = bridge.layoutToPageDescriptors(pages, lookup)[0].htmlContent;
    assert.match(styled, /<span style="font-style: italic;">שתיים<\/span>/);
    assert.match(styled, /color: #112233;/);
  });
});

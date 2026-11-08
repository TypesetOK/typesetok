import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import zlib from 'node:zlib';

import {
  decodeTextBuffer,
  importPlainText,
  importMarkdown,
  parseMarkdownInlineSpans,
  extractDocxXml,
  parseDocxXml,
  importDocx,
  importDocument,
  parseDocxStylesXml,
  mapDocxStyleId,
  parseDocxFieldInstruction,
  detectTextDirection,
  detectTextLanguage,
  cleanDocumentTitle,
  DocumentImportError
} from '../packages/tok-ui/dist/engine/textImporter.js';

import {
  importResultToDocumentState,
  importResultToDocumentRoot,
  documentStateToDocumentRoot,
  documentRootToDocumentState,
  evaluateLiveGrepSpansInUi,
  getBuiltinGrepRules,
  getDefaultMultiApparatusConfigs,
  isValidUlid
} from '../packages/tok-ui/dist/engine/documentBridge.js';

import {
  layoutToPageDescriptors
} from '../packages/tok-ui/dist/engine/typesetBridge.js';

describe('Phase 3 Import: Text, Markdown and DOCX Importers', () => {

  describe('1. Text Direction & Language Detection', () => {
    it('detects Hebrew direction and language correctly', () => {
      const hebrewText = 'בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ';
      assert.equal(detectTextDirection(hebrewText), 'rtl');
      assert.equal(detectTextLanguage(hebrewText), 'he');
    });

    it('detects English direction and language correctly', () => {
      const englishText = 'In the beginning God created the heaven and the earth.';
      assert.equal(detectTextDirection(englishText), 'ltr');
      assert.equal(detectTextLanguage(englishText), 'en');
    });

    it('detects mixed language with dominant Hebrew', () => {
      const mixedText = 'ספר התניא מאת רבי שניאור זלמן מלאדי (Rabbi Shneur Zalman of Liadi).';
      assert.equal(detectTextDirection(mixedText), 'rtl');
      assert.equal(detectTextLanguage(mixedText), 'mixed');
    });

    it('cleans document title from file extensions and symbols', () => {
      assert.equal(cleanDocumentTitle('ספר_התניא_מהדורה_חדשה.tok'), 'ספר התניא מהדורה חדשה');
      assert.equal(cleanDocumentTitle('my_hebrew_book.docx'), 'my hebrew book');
      assert.equal(cleanDocumentTitle(''), 'מסמך ללא שם');
    });
  });

  describe('2. Plain Text (TXT) Importer', () => {
    it('imports UTF-8 plain text with paragraphs and preserves Hebrew niqqud', () => {
      const txt = `בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ:

וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ וְחֹשֶׁךְ עַל־פְּנֵי תְהוֹם:

וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר וַיְהִי־אוֹר:`;

      const result = importPlainText(txt, { fileName: 'בראשית.txt' });
      assert.equal(result.format, 'txt');
      assert.equal(result.title, 'בראשית');
      assert.equal(result.paragraphCount, 3);
      assert.equal(result.detectedDirection, 'rtl');
      assert.equal(result.detectedLanguage, 'he');
      assert(result.paragraphs[0].text.includes('בְּרֵאשִׁית'));
      assert(result.paragraphs[2].text.includes('וַיְהִי־אוֹר'));
    });

    it('decodes UTF-16LE text buffer with BOM', () => {
      const original = 'שלום עליכם ומזל טוב לכל המסובין';
      const buf = Buffer.alloc(2 + original.length * 2);
      // UTF-16LE BOM: 0xFF, 0xFE
      buf[0] = 0xFF;
      buf[1] = 0xFE;
      for (let i = 0; i < original.length; i++) {
        buf.writeUInt16LE(original.charCodeAt(i), 2 + i * 2);
      }

      const decoded = decodeTextBuffer(new Uint8Array(buf));
      assert.equal(decoded, original);

      const result = importPlainText(buf, { fileName: 'utf16_test.txt' });
      assert.equal(result.paragraphs.length, 1);
      assert.equal(result.paragraphs[0].text, original);
    });

    it('throws EMPTY_DOCUMENT for empty text or whitespace-only files', () => {
      assert.throws(() => {
        importPlainText('   \n\n\t  ', { fileName: 'empty.txt' });
      }, (err) => {
        return err instanceof DocumentImportError && err.code === 'EMPTY_DOCUMENT';
      });
    });

    it('throws CORRUPTED_FILE for binary files containing null bytes', () => {
      const corruptBytes = new Uint8Array([0x48, 0x65, 0x6C, 0x6C, 0x00, 0x00, 0x00, 0x6F]);
      assert.throws(() => {
        importPlainText(corruptBytes, { fileName: 'corrupt.bin' });
      }, (err) => {
        return err instanceof DocumentImportError && err.code === 'CORRUPTED_FILE';
      });
    });
  });

  describe('3. Markdown (MD) Importer', () => {
    it('parses inline markdown spans: bold, italic, bold-italic and code', () => {
      const raw = 'טקסט רגיל **טקסט מודגש** עוד רגיל *טקסט נטוי* וגם ***מודגש ונטוי*** ו־`קוד`.';
      const { text, spans } = parseMarkdownInlineSpans(raw);

      assert.equal(text, 'טקסט רגיל טקסט מודגש עוד רגיל טקסט נטוי וגם מודגש ונטוי ו־קוד.');
      assert.equal(spans.length, 4);

      // Bold
      assert.equal(spans[0].format.fontWeight, 700);
      assert.equal(text.slice(spans[0].start, spans[0].end), 'טקסט מודגש');

      // Italic
      assert.equal(spans[1].format.italic, true);
      assert.equal(text.slice(spans[1].start, spans[1].end), 'טקסט נטוי');

      // Bold + Italic
      assert.equal(spans[2].format.fontWeight, 700);
      assert.equal(spans[2].format.italic, true);
      assert.equal(text.slice(spans[2].start, spans[2].end), 'מודגש ונטוי');

      // Monospace code
      assert.equal(spans[3].format.fontFamily, 'monospace');
      assert.equal(text.slice(spans[3].start, spans[3].end), 'קוד');
    });

    it('parses structured markdown with headings, blockquotes, lists and paragraphs', () => {
      const md = `# פרק ראשון: הלכות תלמוד תורה

## הלכה א׳
כל איש מישראל **חייב** בתלמוד תורה, בין עני ובין עשיר.

> וְהָיוּ הַדְּבָרִים הָאֵלֶּה אֲשֶׁר אָנֹכִי מְצַוְּךָ הַיּוֹם עַל־לְבָבֶךָ.

### פירוט הזמנים
- שחרית
- ערבית
- חצות הלילה`;

      const result = importMarkdown(md, { fileName: 'הלכות_תלמוד_תורה.md' });
      assert.equal(result.format, 'md');
      assert.equal(result.title, 'פרק ראשון: הלכות תלמוד תורה');
      assert.equal(result.paragraphs.length, 8);

      assert.equal(result.paragraphs[0].styleId, 'heading-1');
      assert.equal(result.paragraphs[0].text, 'פרק ראשון: הלכות תלמוד תורה');

      assert.equal(result.paragraphs[1].styleId, 'heading-2');
      assert.equal(result.paragraphs[1].text, 'הלכה א׳');

      assert.equal(result.paragraphs[2].styleId, 'default-body');
      assert(result.paragraphs[2].spans && result.paragraphs[2].spans.length > 0);
      assert.equal(result.paragraphs[2].spans[0].format.fontWeight, 700);

      assert.equal(result.paragraphs[3].styleId, 'blockquote');
      assert(result.paragraphs[3].text.includes('וְהָיוּ הַדְּבָרִים'));

      assert.equal(result.paragraphs[4].styleId, 'heading-3');
      assert.equal(result.paragraphs[5].styleId, 'list-item');
      assert.equal(result.paragraphs[6].styleId, 'list-item');
    });

    it('throws EMPTY_DOCUMENT on empty markdown', () => {
      assert.throws(() => {
        importMarkdown('', { fileName: 'blank.md' });
      }, (err) => {
        return err instanceof DocumentImportError && err.code === 'EMPTY_DOCUMENT';
      });
    });
  });

  describe('4. DOCX Document Importer', () => {
    // Helper to generate a minimal in-memory DOCX zip buffer
    function buildMockDocxZip(documentXml) {
      const xmlBuffer = Buffer.from(documentXml, 'utf-8');
      const deflated = zlib.deflateRawSync(xmlBuffer);
      const entryName = 'word/document.xml';
      const nameBuf = Buffer.from(entryName, 'utf-8');

      // Local file header (30 bytes + filename + extra + compressed data)
      const header = Buffer.alloc(30);
      header.writeUInt32LE(0x04034b50, 0); // PK\x03\x04
      header.writeUInt16LE(20, 4); // version needed
      header.writeUInt16LE(0, 6); // general purpose bit flag
      header.writeUInt16LE(8, 8); // compression method (deflate)
      header.writeUInt16LE(0, 10); // file time
      header.writeUInt16LE(0, 12); // file date
      header.writeUInt32LE(0x12345678, 14); // crc-32 (dummy)
      header.writeUInt32LE(deflated.length, 18); // compressed size
      header.writeUInt32LE(xmlBuffer.length, 22); // uncompressed size
      header.writeUInt16LE(nameBuf.length, 26); // file name length
      header.writeUInt16LE(0, 28); // extra field length

      return Buffer.concat([header, nameBuf, deflated]);
    }

    it('extracts word/document.xml from DOCX zip archive and parses paragraphs and spans', () => {
      const docXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr>
        <w:pStyle w:val="Heading1"/>
      </w:pPr>
      <w:r>
        <w:t>שער ראשון: יסודות העימוד העברי</w:t>
      </w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:t>זהו משפט פתיחה עם </w:t>
      </w:r>
      <w:r>
        <w:rPr>
          <w:b/>
        </w:rPr>
        <w:t>מילה מודגשת</w:t>
      </w:r>
      <w:r>
        <w:t> וגם </w:t>
      </w:r>
      <w:r>
        <w:rPr>
          <w:i/>
        </w:rPr>
        <w:t>מילה נטויה</w:t>
      </w:r>
      <w:r>
        <w:t>.</w:t>
      </w:r>
    </w:p>
  </w:body>
</w:document>`;

      const docxBytes = buildMockDocxZip(docXml);
      const extractedXml = extractDocxXml(docxBytes);
      assert(extractedXml.includes('שער ראשון: יסודות העימוד העברי'));

      const result = importDocx(docxBytes, { fileName: 'עימוד_עברי.docx' });
      assert.equal(result.format, 'docx');
      assert.equal(result.title, 'עימוד עברי');
      assert.equal(result.paragraphCount, 2);
      assert.equal(result.paragraphs[0].styleId, 'heading-1');
      assert.equal(result.paragraphs[0].text, 'שער ראשון: יסודות העימוד העברי');

      const p2 = result.paragraphs[1];
      assert.equal(p2.text, 'זהו משפט פתיחה עם מילה מודגשת וגם מילה נטויה.');
      assert.equal(p2.spans?.length, 2);

      // Verify bold span
      assert.equal(p2.spans[0].format.fontWeight, 700);
      assert.equal(p2.text.slice(p2.spans[0].start, p2.spans[0].end), 'מילה מודגשת');

      // Verify italic span
      assert.equal(p2.spans[1].format.italic, true);
      assert.equal(p2.text.slice(p2.spans[1].start, p2.spans[1].end), 'מילה נטויה');
    });

    it('rejects corrupt or non-zip files when importing DOCX', () => {
      const fakeDocx = new Uint8Array([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
      assert.throws(() => {
        importDocx(fakeDocx, { fileName: 'corrupt.docx' });
      }, (err) => {
        return err instanceof DocumentImportError && err.code === 'CORRUPTED_FILE';
      });
    });

    it('rejects zip archive missing word/document.xml', () => {
      // ZIP with only a dummy entry
      const nameBuf = Buffer.from('other_file.txt', 'utf-8');
      const dataBuf = Buffer.from('dummy data', 'utf-8');
      const header = Buffer.alloc(30);
      header.writeUInt32LE(0x04034b50, 0);
      header.writeUInt16LE(20, 4);
      header.writeUInt16LE(0, 6);
      header.writeUInt16LE(0, 8); // uncompressed
      header.writeUInt32LE(dataBuf.length, 18);
      header.writeUInt32LE(dataBuf.length, 22);
      header.writeUInt16LE(nameBuf.length, 26);
      header.writeUInt16LE(0, 28);
      const invalidZip = Buffer.concat([header, nameBuf, dataBuf]);

      assert.throws(() => {
        importDocx(invalidZip, { fileName: 'invalid_structure.docx' });
      }, (err) => {
        return err instanceof DocumentImportError && err.code === 'INVALID_STRUCTURE';
      });
    });
  });

  describe('5. Unified importDocument Routing', () => {
    it('routes TXT files correctly via filename and content', () => {
      const res = importDocument('שלום עולם', { fileName: 'test.txt' });
      assert.equal(res.format, 'txt');
      assert.equal(res.paragraphCount, 1);
    });

    it('routes Markdown files correctly via filename and content', () => {
      const res = importDocument('# כותרת\nפסקה עם **הדגשה**', { fileName: 'test.md' });
      assert.equal(res.format, 'md');
      assert.equal(res.paragraphCount, 2);
    });

    it('auto-detects Markdown from header markup even without extension', () => {
      const res = importDocument('# כותרת ללא סיומת\nתוכן הפסקה');
      assert.equal(res.format, 'md');
    });
  });

  describe('6. Integration with Document Bridge & tok-core Model', () => {
    it('converts ImportResult into valid MultiFlowDocumentState and DocumentRootJson', () => {
      const md = `# ספר המוסר

איזהו חכם? **הרואה את הנולד**.

איזהו גיבור? **הכובש את יצרו**.`;

      const importResult = importMarkdown(md, { fileName: 'ספר_המוסר.md' });
      const docState = importResultToDocumentState(importResult, 'prose');

      assert.equal(docState.title, 'ספר המוסר');
      assert.equal(docState.templateType, 'prose');
      assert.equal(docState.flows.gemara.length, 3);
      assert.equal(docState.flows.rashi.length, 0);
      assert.equal(docState.flows.tosafot.length, 0);

      // Verify DocumentRootJson conversion
      const docRoot = importResultToDocumentRoot(importResult, 'prose');
      assert.equal(docRoot.metadata.title, 'ספר המוסר');
      assert.equal(docRoot.metadata.progression, 'Rtl');
      assert.equal(docRoot.metadata.primary_language, 'he');
      assert(isValidUlid(docRoot.id));

      const sec = docRoot.sections[0];
      assert(sec && sec.flows.length >= 1);
      const flow = sec.flows[0];
      assert.equal(flow.paragraphs.length, 3);
      assert(isValidUlid(flow.paragraphs[0].id));
      assert(flow.paragraphs[1].spans && flow.paragraphs[1].spans.length > 0);
      assert.equal(flow.paragraphs[1].spans[0].format.font_weight, 700);
    });

    it('[Section 5.1] extracts DOCX footnotes (word/footnotes.xml), nested tables (w:tbl), and numbered lists (word/numbering.xml)', () => {
      function buildMultiPartDocxZip(entries) {
        const parts = [];
        for (const [entryName, xmlContent] of Object.entries(entries)) {
          const xmlBuffer = Buffer.from(xmlContent, 'utf-8');
          const deflated = zlib.deflateRawSync(xmlBuffer);
          const nameBuf = Buffer.from(entryName, 'utf-8');
          const header = Buffer.alloc(30);
          header.writeUInt32LE(0x04034b50, 0);
          header.writeUInt16LE(20, 4);
          header.writeUInt16LE(0, 6);
          header.writeUInt16LE(8, 8);
          header.writeUInt16LE(0, 10);
          header.writeUInt16LE(0, 12);
          header.writeUInt32LE(0x12345678, 14);
          header.writeUInt32LE(deflated.length, 18);
          header.writeUInt32LE(xmlBuffer.length, 22);
          header.writeUInt16LE(nameBuf.length, 26);
          header.writeUInt16LE(0, 28);
          parts.push(header, nameBuf, deflated);
        }
        return Buffer.concat(parts);
      }

      const documentXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr>
        <w:numPr>
          <w:ilvl w:val="0"/>
          <w:numId w:val="1"/>
        </w:numPr>
      </w:pPr>
      <w:r><w:t>דין קריאת שמע של ערבית</w:t></w:r>
      <w:r><w:footnoteReference w:id="2"/></w:r>
    </w:p>
    <w:tbl>
      <w:tblGrid>
        <w:gridCol w:w="2400"/>
        <w:gridCol w:w="2400"/>
      </w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>שיטת רש״י</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>שיטת ר״ת</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>`;

      const footnotesXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:type="separator" w:id="0"><w:p/></w:footnote>
  <w:footnote w:id="2">
    <w:p><w:r><w:t>ראה ברכות דף ב עמוד א במשנה.</w:t></w:r></w:p>
  </w:footnote>
</w:footnotes>`;

      const numberingXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="10">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="hebrew1"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="10"/>
  </w:num>
</w:numbering>`;

      const zipBuf = buildMultiPartDocxZip({
        'word/document.xml': documentXml,
        'word/footnotes.xml': footnotesXml,
        'word/numbering.xml': numberingXml
      });

      const imported = importDocx(zipBuf, { fileName: 'שו_ת_הלכה.docx' });
      assert(imported.footnotes && imported.footnotes.length === 1, 'Must extract footnote from word/footnotes.xml');
      assert(imported.footnotes[0].text.includes('ראה ברכות דף ב עמוד א'), 'Footnote body must match');
      assert(imported.multiFlowAnchors && imported.multiFlowAnchors.length === 1, 'Must link primary paragraph to footnote via multiFlowAnchors');
      assert(imported.paragraphs[0].text.startsWith('א.'), 'Must prefix Hebrew list item marker א. from word/numbering.xml');
      assert(imported.paragraphs[0].text.includes('[א]'), 'Must insert Hebrew footnote reference marker [א]');
      assert(imported.tables && imported.tables.length === 1, 'Must extract structured table metadata');
      assert(imported.paragraphs.some((p) => p.text.includes('שיטת רש״י') && p.text.includes('שיטת ר״ת')), 'Must convert table cells into RTL tabular paragraph');

      const state = importResultToDocumentState(imported, 'prose');
      assert.equal(state.flows.notes.length, 1);
      assert.equal(state.multiFlowAnchors?.length, 1);
    });

    it('[Section 2.1 & 5.3] supports Multi-Apparatus Footnotes and non-destructive Live GREP rules in UI and DocumentRoot', () => {
      const apparatuses = getDefaultMultiApparatusConfigs();
      assert.equal(apparatuses.length, 2);
      assert.equal(apparatuses[0].columns, 2);
      assert.equal(apparatuses[0].numbering_style, 'HebrewGematria');
      assert.equal(apparatuses[1].columns, 1);
      assert.equal(apparatuses[1].numbering_style, 'ArabicNumerals');

      const rules = getBuiltinGrepRules().map((r) => ({ ...r, enabled: true }));
      const sampleText = 'אמר רבא כתב הרמב״ם (שבת כג ע״א) בהלכות חנוכה.';
      const grepSpans = evaluateLiveGrepSpansInUi(sampleText, undefined, rules, 'gemara');
      assert(grepSpans.some((s) => s.format.fontScale === 0.80), 'Parenthetical citation must get 80% fontScale');
      assert(grepSpans.some((s) => s.format.fontWeight === 700), 'Opening words and acronyms must get bold 700');

      const state = {
        title: 'ספר שו״ת רב-מדורי',
        templateType: 'notes',
        flows: {
          gemara: [{ id: 'p-main-1', styleId: 'normal', text: sampleText }],
          rashi: [],
          tosafot: [],
          notes: [{ id: 'p-app1-1', styleId: 'style-footnotes', text: 'א. שבת כג ע״א.' }],
          biurim: [{ id: 'p-app2-1', styleId: 'style-footnotes', text: '1. ביאור ארוך ומנומק בדברי הרמב״ם.' }]
        },
        footnoteApparatuses: apparatuses,
        grepRules: rules
      };

      const mockPageBoxes = [
        {
          page_index: 0,
          page_number_gematria: 'א',
          dimensions: { x: 0, y: 0, width: 481.89, height: 680.31 },
          frames: [
            {
              frame_id: 'frame_0_main',
              flow_id: 'main',
              rect: { x: 42, y: 50, width: 382, height: 420 },
              lines: [{ line_index: 0, baseline_y: 16, height: 14, width: 300, text: sampleText, is_rtl: true }]
            },
            {
              frame_id: 'frame_0_footnote_notes',
              flow_id: 'notes',
              rect: { x: 42, y: 480, width: 382, height: 55 },
              lines: [{ line_index: 0, baseline_y: 12, height: 11.5, width: 185, text: 'א. שבת כג ע״א.', is_rtl: true }]
            },
            {
              frame_id: 'frame_0_footnote_biurim',
              flow_id: 'biurim',
              rect: { x: 42, y: 545, width: 382, height: 70 },
              lines: [{ line_index: 0, baseline_y: 13, height: 13, width: 382, text: '1. ביאור ארוך ומנומק בדברי הרמב״ם.', is_rtl: true }]
            }
          ]
        }
      ];

      const pages = layoutToPageDescriptors(mockPageBoxes);
      assert(pages.length === 1);
      assert(pages[0].htmlContent.includes('data-rule-tier="primary"'), 'Must render primary separator rule');
      assert(pages[0].htmlContent.includes('data-rule-tier="inter-apparatus"'), 'Must render inter-apparatus separator rule');

      const rootJson = documentStateToDocumentRoot(state);
      assert.equal(rootJson.footnote_apparatuses?.length, 2);
      assert.equal(rootJson.grep_rules?.length, rules.length);
      const restored = documentRootToDocumentState(rootJson);
      assert.equal(restored.footnoteApparatuses?.length, 2);
      assert.equal(restored.grepRules?.length, rules.length);
    });

    it('[Section 5.1] parses word/styles.xml, resolving custom paragraph and character styles with basedOn inheritance', () => {
      const stylesXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="BaseTorah">
    <w:name w:val="סגנון בסיס תורני"/>
    <w:pPr>
      <w:jc w:val="both"/>
      <w:bidi/>
      <w:ind w:start="720" w:firstLine="360"/>
      <w:spacing w:before="120" w:after="60" w:line="280"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Frank Ruhl Libre" w:cs="Frank Ruhl Libre"/>
      <w:sz w:val="24"/>
      <w:color w:val="1A1A1A"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="CommentaryDerived">
    <w:name w:val="פירוש משני נגזר"/>
    <w:basedOn w:val="BaseTorah"/>
    <w:pPr>
      <w:ind w:start="1080"/>
      <w:spacing w:before="80"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Noto Rashi Hebrew" w:cs="Noto Rashi Hebrew"/>
      <w:sz w:val="20"/>
    </w:rPr>
  </w:style>
  <w:style w:type="character" w:styleId="CustomCharHighlight">
    <w:name w:val="הדגשה תורנית"/>
    <w:rPr>
      <w:b/>
      <w:sz w:val="22"/>
      <w:color w:val="880000"/>
    </w:rPr>
  </w:style>
</w:styles>`;

      const registry = parseDocxStylesXml(stylesXml);
      assert.equal(registry.size, 3);

      const base = registry.get('BaseTorah');
      assert(base);
      assert.equal(base.name, 'סגנון בסיס תורני');
      assert.equal(base.paragraphPatch?.alignment, 'Justified');
      assert.equal(base.paragraphPatch?.direction, 'Rtl');
      assert.equal(base.paragraphPatch?.indentStartPt, 36);
      assert.equal(base.paragraphPatch?.firstLineIndentPt, 18);
      assert.equal(base.paragraphPatch?.spaceBeforePt, 6);
      assert.equal(base.paragraphPatch?.spaceAfterPt, 3);
      assert.equal(base.paragraphPatch?.fontFamily, 'Frank Ruhl Libre');
      assert.equal(base.paragraphPatch?.fontSizePt, 12);
      assert.equal(base.paragraphPatch?.color, '#1A1A1A');

      const derived = registry.get('CommentaryDerived');
      assert(derived);
      assert.equal(derived.name, 'פירוש משני נגזר');
      assert.equal(derived.paragraphPatch?.alignment, 'Justified', 'Derived style must inherit base alignment');
      assert.equal(derived.paragraphPatch?.direction, 'Rtl', 'Derived style must inherit base direction');
      assert.equal(derived.paragraphPatch?.indentStartPt, 54, 'Derived style must override indentStartPt (1080/20)');
      assert.equal(derived.paragraphPatch?.firstLineIndentPt, 18, 'Derived style must inherit base firstLineIndentPt');
      assert.equal(derived.paragraphPatch?.spaceBeforePt, 4, 'Derived style must override spaceBeforePt (80/20)');
      assert.equal(derived.paragraphPatch?.spaceAfterPt, 3, 'Derived style must inherit base spaceAfterPt');
      assert.equal(derived.paragraphPatch?.fontFamily, 'Noto Rashi Hebrew', 'Derived style must override fontFamily');
      assert.equal(derived.paragraphPatch?.fontSizePt, 10, 'Derived style must override fontSizePt (20/2)');
      assert.equal(derived.paragraphPatch?.color, '#1A1A1A', 'Derived style must inherit base color');

      const charStyle = registry.get('CustomCharHighlight');
      assert(charStyle);
      assert.equal(charStyle.name, 'הדגשה תורנית');
      assert.equal(charStyle.characterFormat?.fontWeight, 700);
      assert.equal(charStyle.characterFormat?.fontSizePt, 11);
      assert.equal(charStyle.characterFormat?.color, '#880000');
    });

    it('[Section 5.1] parses field instructions from w:fldSimple (HYPERLINK, REF, PAGEREF)', () => {
      // 1. Quoted external URL
      const link1 = parseDocxFieldInstruction('HYPERLINK "https://www.hebrewbooks.org/1234"');
      assert.equal(link1.type, 'HYPERLINK');
      assert.equal(link1.targetUrl, 'https://www.hebrewbooks.org/1234');
      assert.equal(link1.isExternal, true);

      // 2. Encoded entity URL with target window
      const link2 = parseDocxFieldInstruction('HYPERLINK &quot;https://sefaria.org/Berakhot.2a&quot; \\t &quot;_blank&quot;');
      assert.equal(link2.type, 'HYPERLINK');
      assert.equal(link2.targetUrl, 'https://sefaria.org/Berakhot.2a');
      assert.equal(link2.isExternal, true);

      // 3. Internal bookmark anchor via \\l
      const link3 = parseDocxFieldInstruction('HYPERLINK \\l "siman_aleph"');
      assert.equal(link3.type, 'HYPERLINK');
      assert.equal(link3.targetUrl, '#siman_aleph');
      assert.equal(link3.targetAnchor, 'siman_aleph');
      assert.equal(link3.isExternal, false);

      // 4. External URL combined with internal bookmark anchor
      const link4 = parseDocxFieldInstruction('HYPERLINK "http://example.com/page" \\l "anchor_here"');
      assert.equal(link4.type, 'HYPERLINK');
      assert.equal(link4.targetUrl, 'http://example.com/page#anchor_here');
      assert.equal(link4.isExternal, true);

      // 5. REF cross reference
      const ref1 = parseDocxFieldInstruction('REF chapter_one \\h');
      assert.equal(ref1.type, 'REF');
      assert.equal(ref1.targetUrl, '#chapter_one');
      assert.equal(ref1.isExternal, false);

      // 6. PAGEREF
      const pageRef = parseDocxFieldInstruction('PAGEREF page_target \\h');
      assert.equal(pageRef.type, 'PAGEREF');
      assert.equal(pageRef.targetUrl, '#page_target');
      assert.equal(pageRef.isExternal, false);
    });

    it('[Section 5.1] imports DOCX with custom styles from word/styles.xml and field hyperlinks from w:fldSimple', () => {
      function buildDocxPackage(entries) {
        const parts = [];
        for (const [entryName, xmlContent] of Object.entries(entries)) {
          const xmlBuffer = Buffer.from(xmlContent, 'utf-8');
          const deflated = zlib.deflateRawSync(xmlBuffer);
          const nameBuf = Buffer.from(entryName, 'utf-8');
          const header = Buffer.alloc(30);
          header.writeUInt32LE(0x04034b50, 0);
          header.writeUInt16LE(20, 4);
          header.writeUInt16LE(0, 6);
          header.writeUInt16LE(8, 8);
          header.writeUInt16LE(0, 10);
          header.writeUInt16LE(0, 12);
          header.writeUInt32LE(0x12345678, 14);
          header.writeUInt32LE(deflated.length, 18);
          header.writeUInt32LE(xmlBuffer.length, 22);
          header.writeUInt16LE(nameBuf.length, 26);
          header.writeUInt16LE(0, 28);
          parts.push(header, nameBuf, deflated);
        }
        return Buffer.concat(parts);
      }

      const stylesXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="TorahHeading">
    <w:name w:val="כותרת תורנית מותאמת"/>
    <w:pPr>
      <w:jc w:val="center"/>
      <w:bidi/>
      <w:spacing w:before="240" w:after="120"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Taamey Frank" w:cs="Taamey Frank"/>
      <w:b/>
      <w:sz w:val="32"/>
      <w:color w:val="331100"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="CustomCommentary">
    <w:name w:val="פירוש עמוק"/>
    <w:pPr>
      <w:jc w:val="both"/>
      <w:bidi/>
      <w:ind w:start="360" w:firstLine="240"/>
      <w:spacing w:before="100" w:after="50"/>
    </w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Frank Ruhl Libre" w:cs="Frank Ruhl Libre"/>
      <w:sz w:val="22"/>
      <w:color w:val="222222"/>
    </w:rPr>
  </w:style>
  <w:style w:type="character" w:styleId="TorahHighlight">
    <w:name w:val="ציטוט מקור"/>
    <w:rPr>
      <w:b/>
      <w:color w:val="990000"/>
    </w:rPr>
  </w:style>
</w:styles>`;

      const documentXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr>
        <w:pStyle w:val="TorahHeading"/>
      </w:pPr>
      <w:r><w:t>שער הביאורים וההערות</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr>
        <w:pStyle w:val="CustomCommentary"/>
        <!-- Local override for indentation -->
        <w:ind w:start="500"/>
      </w:pPr>
      <w:r><w:t>זהו ביאור מקיף, המסתמך על </w:t></w:r>
      <w:fldSimple w:instr="HYPERLINK &quot;https://www.hebrewbooks.org/54321&quot;">
        <w:r>
          <w:rPr>
            <w:rStyle w:val="TorahHighlight"/>
          </w:rPr>
          <w:t>ספר התניא קדישא</w:t>
        </w:r>
      </w:fldSimple>
      <w:r><w:t> וכן </w:t></w:r>
      <w:fldSimple w:instr="HYPERLINK \\l &quot;perek_2&quot;">
        <w:r><w:t>בפרק ב׳ להלן</w:t></w:r>
      </w:fldSimple>
      <w:r><w:t>.</w:t></w:r>
      <w:r><w:footnoteReference w:id="3"/></w:r>
    </w:p>
  </w:body>
</w:document>`;

      const footnotesXml = `<?xml version="1.0" encoding="UTF-8"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:type="separator" w:id="0"><w:p/></w:footnote>
  <w:footnote w:id="3">
    <w:p>
      <w:r><w:t>ראה מקור נוסף באתר </w:t></w:r>
      <w:fldSimple w:instr="HYPERLINK &quot;https://sefaria.org&quot;">
        <w:r><w:t>ספריא</w:t></w:r>
      </w:fldSimple>
      <w:r><w:t> בביאור סוגיא זו.</w:t></w:r>
    </w:p>
  </w:footnote>
</w:footnotes>`;

      const zipBytes = buildDocxPackage({
        'word/document.xml': documentXml,
        'word/styles.xml': stylesXml,
        'word/footnotes.xml': footnotesXml
      });

      const imported = importDocx(zipBytes, { fileName: 'ספר_ביאורים.docx' });
      assert.equal(imported.format, 'docx');
      assert.equal(imported.paragraphCount, 2);

      // Verify custom styles parsed in imported result
      assert(imported.styles, 'imported.styles must exist');
      assert.equal(Object.keys(imported.styles).length, 3);
      assert(imported.styles.TorahHeading);
      assert(imported.styles.CustomCommentary);
      assert(imported.styles.TorahHighlight);

      // Verify Paragraph 1: TorahHeading
      const p1 = imported.paragraphs[0];
      assert.equal(p1.styleId, 'TorahHeading');
      assert.equal(p1.text, 'שער הביאורים וההערות');
      assert.equal(p1.patch?.alignment, 'Center');
      assert.equal(p1.patch?.direction, 'Rtl');
      assert.equal(p1.patch?.spaceBeforePt, 12);
      assert.equal(p1.patch?.spaceAfterPt, 6);
      assert.equal(p1.patch?.fontFamily, 'Taamey Frank');
      assert.equal(p1.patch?.fontSizePt, 16);
      assert.equal(p1.patch?.color, '#331100');

      // Verify Paragraph 2: CustomCommentary with local override
      const p2 = imported.paragraphs[1];
      assert.equal(p2.styleId, 'CustomCommentary');
      assert(p2.text.includes('ספר התניא קדישא'));
      assert(p2.text.includes('בפרק ב׳ להלן'));
      assert(p2.text.includes('[א]'), 'Must contain footnote marker');
      assert.equal(p2.patch?.alignment, 'Justified');
      assert.equal(p2.patch?.direction, 'Rtl');
      assert.equal(p2.patch?.indentStartPt, 25, 'Local override 500dxa / 20 = 25pt must override style 360dxa');
      assert.equal(p2.patch?.firstLineIndentPt, 12, 'Inherited firstLine 240dxa / 20 = 12pt');
      assert.equal(p2.patch?.fontFamily, 'Frank Ruhl Libre');
      assert.equal(p2.patch?.fontSizePt, 11);
      assert.equal(p2.patch?.color, '#222222');

      // Verify extracted links in paragraph 2
      assert(imported.links && imported.links.length >= 2, 'Must extract at least 2 links from document');
      const externalLink = imported.links.find((l) => l.url.includes('hebrewbooks.org'));
      assert(externalLink, 'Must find hebrewbooks external link');
      assert.equal(externalLink.text, 'ספר התניא קדישא');
      assert.equal(externalLink.isExternal, true);
      assert.equal(externalLink.paragraphId, p2.id);

      const internalLink = imported.links.find((l) => l.url === '#perek_2');
      assert(internalLink, 'Must find internal #perek_2 bookmark anchor');
      assert.equal(internalLink.text, 'בפרק ב׳ להלן');
      assert.equal(internalLink.isExternal, false);

      // Verify TextSpans formatting on the links
      const linkSpan1 = p2.spans?.find((s) => s.format.linkUrl === 'https://www.hebrewbooks.org/54321');
      assert(linkSpan1, 'Must have span with linkUrl for hebrewbooks');
      assert.equal(p2.text.slice(linkSpan1.start, linkSpan1.end), 'ספר התניא קדישא');
      assert.equal(linkSpan1.format.fontWeight, 700, 'Must inherit bold from TorahHighlight style');
      assert.equal(linkSpan1.format.color, '#990000', 'Must inherit color from TorahHighlight style');
      assert.equal(linkSpan1.format.underline, true, 'Link must be underlined');

      const linkSpan2 = p2.spans?.find((s) => s.format.linkUrl === '#perek_2');
      assert(linkSpan2, 'Must have span with linkUrl for internal anchor');
      assert.equal(p2.text.slice(linkSpan2.start, linkSpan2.end), 'בפרק ב׳ להלן');
      assert.equal(linkSpan2.format.characterStyleId, 'hyperlink');

      // Verify Footnote was extracted and linked
      assert(imported.footnotes && imported.footnotes.length === 1);
      assert(imported.footnotes[0].text.includes('ראה מקור נוסף באתר'));
      assert(imported.multiFlowAnchors && imported.multiFlowAnchors.length === 1);
      assert.equal(imported.multiFlowAnchors[0].primaryNodeId, p2.id);
      assert.equal(imported.multiFlowAnchors[0].commentaryNodeId, imported.footnotes[0].id);

      // Verify DocumentRoot roundtrip preserves linkUrl and styles
      const root = importResultToDocumentRoot(imported, 'prose');
      const gemaraFlow = root.sections[0].flows[0];
      const p2Json = gemaraFlow.paragraphs[1];
      const p2LinkSpanJson = p2Json.spans?.find((s) => s.format.link_url === 'https://www.hebrewbooks.org/54321');
      assert(p2LinkSpanJson, 'DocumentRoot JSON must preserve link_url in span format');

      const restoredState = documentRootToDocumentState(root);
      const restoredP2 = restoredState.flows.gemara[1];
      const restoredSpan = restoredP2.spans?.find((s) => s.format.linkUrl === 'https://www.hebrewbooks.org/54321');
      assert(restoredSpan, 'Restored DocumentState must preserve linkUrl');
    });
  });

});

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  importDocument,
  importBatchDocx,
  parseLegacyTagDocument,
  isLegacyTagFormat
} from '../packages/tok-ui/dist/engine/textImporter.js';
import { layoutToPageDescriptors } from '../packages/tok-ui/dist/engine/typesetBridge.js';

describe('Architectural Completion Suite (Tag Parser, Batch DOCX, Heatmap)', () => {
  it('parses legacy Tag (@1, @2, <<B>>, <<I>>, <<S:9>>, {{FN:...}}) into semantic paragraphs and spans', () => {
    const raw = [
      '@1 פרק א: הלכות קריאת שמע',
      '@00 אמר <<B>>רב יהודה<</B>> אמר <<I>>שמואל<</I>> <<S:9>>(עיין רש"י)<</S>>{{FN:ברכות דף ב ע"א}}'
    ].join('\n');

    assert.equal(isLegacyTagFormat(raw), true);
    const res = parseLegacyTagDocument(raw, { fileName: 'shiur.tag' });
    assert.equal(res.format, 'tag');
    assert.equal(res.paragraphs.length, 2);
    assert.equal(res.paragraphs[0].styleId, 'default-heading');
    assert.equal(res.paragraphs[0].text, 'פרק א: הלכות קריאת שמע');
    assert.equal(res.paragraphs[1].styleId, 'default-body');
    assert.equal(res.paragraphs[1].spans?.length, 3);
    assert.equal(res.paragraphs[1].spans?.[0].format.fontWeight, 700);
    assert.equal(res.paragraphs[1].spans?.[1].format.italic, true);
    assert.equal(res.paragraphs[1].spans?.[2].format.fontSizePt, 9);
    assert.equal(res.footnotes?.length, 1);
    assert.ok(res.footnotes?.[0].text.includes('ברכות דף ב ע"א'));

    // Auto-detection via importDocument
    const auto = importDocument(raw);
    assert.equal(auto.format, 'tag');
  });

  it('batch-imports and stitches multiple DOCX documents without ID collisions', () => {
    const makeDocxXml = (bodyText, fnText) => ({
      documentXml: `<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>${bodyText}</w:t></w:r><w:r><w:footnoteReference w:id="1"/></w:r></w:p></w:body></w:document>`,
      footnotesXml: `<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:footnote w:id="1"><w:p><w:r><w:t>${fnText}</w:t></w:r></w:p></w:footnote></w:footnotes>`
    });

    const f1 = makeDocxXml('פרק ראשון', 'הערה ראשונה');
    const f2 = makeDocxXml('פרק שני', 'הערה שניה');

    const res = importBatchDocx(
      [
        { fileName: 'chapter1.docx', content: new Uint8Array(), options: { docxXml: f1.documentXml, footnotesXml: f1.footnotesXml } },
        { fileName: 'chapter2.docx', content: new Uint8Array(), options: { docxXml: f2.documentXml, footnotesXml: f2.footnotesXml } }
      ],
      { mode: 'split_sections' }
    );

    assert.equal(res.importedFileCount, 2);
    assert.equal(res.sections.length, 2);
    assert.equal(res.sections[0].name, 'chapter1');
    assert.equal(res.sections[1].name, 'chapter2');
    assert.equal(res.paragraphs.length, 2);
    assert.notEqual(res.paragraphs[0].id, res.paragraphs[1].id);
    assert.equal(res.footnotes?.length, 2);
  });

  it('emits justification heatmap attributes (data-adj-ratio & data-hj-tier) in layoutToPageDescriptors', () => {
    const pages = [
      {
        page_index: 0,
        page_number_gematria: 'א׳',
        dimensions: { x: 0, y: 0, width: 595, height: 842 },
        frames: [
          {
            frame_id: 'main',
            flow_id: 'main',
            rect: { x: 50, y: 50, width: 495, height: 700 },
            lines: [
              {
                line_index: 0,
                baseline_y: 16,
                height: 16,
                width: 495,
                text: 'שורה מאוזנת היטב',
                is_rtl: true,
                adjustment_ratio: 0.25
              },
              {
                line_index: 1,
                baseline_y: 32,
                height: 16,
                width: 495,
                text: 'שורה מתוחה מאוד',
                is_rtl: true,
                adjustment_ratio: 1.85
              }
            ]
          }
        ]
      }
    ];

    const descriptors = layoutToPageDescriptors(pages);
    assert.equal(descriptors.length, 1);
    assert.ok(descriptors[0].htmlContent.includes('data-adj-ratio="0.250" data-hj-tier="good"'));
    assert.ok(descriptors[0].htmlContent.includes('data-adj-ratio="1.850" data-hj-tier="bad"'));
  });
});

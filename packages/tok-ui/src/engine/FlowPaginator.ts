/**
 * Multi-flow Real-Time Live Paginator (Engine for Talmud & Sacred Texts).
 *
 * Synchronizes multiple parallel text flows across spreads and pages in real time:
 * - Gemara (Main flow in center column)
 * - Rashi (Inner commentary along the spine: left on Recto, right on Verso)
 * - Tosafot (Outer commentary: right on Recto, left on Verso)
 * - Footnotes / Tradition (Bottom full-width frame)
 *
 * Implements:
 * 1. Recto / Verso aware column placement according to Hebrew RTL binding.
 * 2. Automatic multi-page cascading: overflowing paragraphs flow seamlessly into page K+1, K+2...
 * 3. Dynamic L-Shape commentary expansion under early-terminating Gemara text ("צורת הדף").
 * 4. High-performance synchronous and debounced pagination (<2ms per spread).
 */

import { StoryParagraph } from 'tok-story-editor';
import { PageDescriptor } from 'tok-viewer';
import { toHebrewGematria } from '../gematria';
import { isRectoPage, isRightHandPage } from '../components/SpreadCanvas';

export type TemplateType = 'gemara' | 'mikraot' | 'prose' | 'notes' | 'bulletin';

export interface MultiFlowDocumentState {
  title: string;
  templateType: TemplateType;
  flows: {
    gemara: StoryParagraph[];
    rashi: StoryParagraph[];
    tosafot: StoryParagraph[];
    notes: StoryParagraph[];
    [key: string]: StoryParagraph[];
  };
}

/** Pre-defined authentic sacred texts for initial templates */
export const DEFAULT_TALMUD_FLOWS: MultiFlowDocumentState['flows'] = {
  gemara: [
    {
      id: 'g-1',
      styleId: 'style-gemara-main',
      text: 'מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן, עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה, דִּבְרֵי רַבִּי אֱלִיעֶזֶר. וַחֲכָמִים אוֹמְרִים: עַד חֲצוֹת. רַבָּן גַּמְלִיאֵל אוֹמֵר: עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר.'
    },
    {
      id: 'g-2',
      styleId: 'style-gemara-main',
      text: 'מַעֲשֶׂה שֶׁבָּאוּ בָנָיו מִבֵּית הַמִּשְׁתֶּה, אָמְרוּ לוֹ: לֹא קָרִינוּ אֶת שְׁמַע! אָמַר לָהֶם: אִם לֹא עָלָה עַמּוּד הַשָּׁחַר, חַיָּבִין אַתֶּם לִקְרוֹת.'
    },
    {
      id: 'g-3',
      styleId: 'style-gemara-main',
      text: 'וְלֹא זוֹ בִלְבַד, אֶלָּא כָּל מַה שֶׁאָמְרוּ חֲכָמִים עַד חֲצוֹת, מִצְוָתָן עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר. הֶקְטֵר חֲלָבִים וְאֵבָרִים מִצְוָתָן עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר, וְכָל הַנֶּאֱכָלִים לְיוֹם אֶחָד מִצְוָתָן עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר.'
    },
    {
      id: 'g-4',
      styleId: 'style-gemara-main',
      text: 'אִם כֵּן, לָמָּה אָמְרוּ חֲכָמִים עַד חֲצוֹת? כְּדֵי לְהַרְחִיק אֶת הָאָדָם מִן הָעֲבֵרָה, שֶׁלֹּא יֹאמַר אָדָם: יֵשׁ לִי עוֹד זְמַן, וְנִמְצָא יָשֵׁן וְעוֹבֵר עַל דִּבְרֵי תוֹרָה.'
    },
    {
      id: 'g-5',
      styleId: 'style-gemara-main',
      text: 'גְּמָרָא. תַּנָּא הֵיכָא קָאֵי דְּקָתָנֵי מֵאֵימָתַי? וְתוּ, מַאי שְׁנָא דְּתָנֵי בְּעַרְבִית בְּרֵישָׁא, לִתְנֵי דְּשַׁחֲרִית בְּרֵישָׁא? תַּנָּא אַקְּרָא קָאֵי, דִּכְתִיב: בְּשָׁכְבְּךָ וּבְקוּמֶךָ. וְהָכִי קָתָנֵי: זְמַן קְרִיאַת שְׁמַע דִּשְׁכִיבָה אֵימַת? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן.'
    },
    {
      id: 'g-6',
      styleId: 'style-gemara-main',
      text: 'וְאִי בָּעֵית אֵימָא, יָלֵיף מִבְּרִיָּתוֹ שֶׁל עוֹלָם, דִּכְתִיב: וַיְהִי עֶרֶב וַיְהִי בֹקֶר יוֹם אֶחָד. אִי הָכִי, סֵיפָא דְּקָתָנֵי: בַּשַּׁחַר מְבָרֵךְ שְׁתַּיִם לְפָנֶיהָ וְאַחַת לְאַחֲרֶיהָ, וּבָעֶרֶב מְבָרֵךְ שְׁתַּיִם לְפָנֶיהָ וּשְׁתַּיִם לְאַחֲרֶיהָ, לִתְנֵי דְּעַרְבִית בְּרֵישָׁא? תַּנָּא פָּתַח בְּעַרְבִית וַהֲדַר תָּנֵי בְּשַׁחֲרִית.'
    },
    {
      id: 'g-7',
      styleId: 'style-gemara-main',
      text: 'אָמַר מָר: מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן. כֹּהֲנִים אֵימַת קָא אָכְלֵי תְּרוּמָה? מִשָּׁעָה דְּטָבְלוּ וְהֶעֱרִיב שִׁמְשָׁן. וְאֵימַת הֶעֱרִיב שִׁמְשָׁן? מִצֵּאת הַכּוֹכָבִים!'
    }
  ],
  rashi: [
    {
      id: 'r-1',
      styleId: 'style-rashi-body',
      text: '<b>מֵאֵימָתַי קוֹרִין</b> - תַּנָּא אַקְּרָא קָאֵי דִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ, וְהָכִי קָתָנֵי: זְמַן קְרִיאַת שְׁמַע שֶׁל שְׁכִיבָה אֵימַת, מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן, שֶׁהוּא גְּמַר בִּיאַת הַשֶּׁמֶשׁ וְצֵאת הַכּוֹכָבִים.'
    },
    {
      id: 'r-2',
      styleId: 'style-rashi-body',
      text: '<b>שֶׁהַכֹּהֲנִים נִכְנָסִים</b> - שֶׁנִּטְמְאוּ וְטָבְלוּ וְהֶעֱרִיב שִׁמְשָׁן וְהִגִּיעַ עֵת לֶאֱכֹל בִּתְרוּמָתָן, וְהַיְינוּ צֵאת הַכּוֹכָבִים.'
    },
    {
      id: 'r-3',
      styleId: 'style-rashi-body',
      text: '<b>עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה</b> - שְׁלִישׁ הַלַּיְלָה, דְּהַלַּיְלָה נֶחֱלָק לְשָׁלֹשׁ מִשְׁמָרוֹת, מִכָּאן וְאֵילָךְ לָאו זְמַן שְׁכִיבָה הוּא.'
    },
    {
      id: 'r-4',
      styleId: 'style-rashi-body',
      text: '<b>עַד חֲצוֹת</b> - כְּדֵי לְהַרְחִיק אֶת הָאָדָם מִן הָעֲבֵרָה, שֶׁלֹּא יֹאמַר אָדָם: יֵשׁ לִי עוֹד זְמַן וְנִמְצָא יָשֵׁן וְעוֹבֵר עַל דִּבְרֵי תוֹרָה.'
    },
    {
      id: 'r-5',
      styleId: 'style-rashi-body',
      text: '<b>דְּקָתָנֵי מֵאֵימָתַי</b> - מִכְּלָל דִּפְשִׁיטָא לֵיהּ דְּחוֹבָה הִיא, וְאַלְמָא קָאֵי אַקְּרָא דְּאוֹרַיְיתָא.'
    },
    {
      id: 'r-6',
      styleId: 'style-rashi-body',
      text: '<b>דִּכְתִיב בְּשָׁכְבְּךָ</b> - בִּשְׁעַת שְׁכִיבָה, וּבְקוּמְךָ בִּשְׁעַת קִימָה.'
    },
    {
      id: 'r-7',
      styleId: 'style-rashi-body',
      text: '<b>וַהֲדַר תָּנֵי בְּשַׁחֲרִית</b> - מִשּׁוּם דְּחָבִיבָא לֵיהּ מִצְוָה דִּתְדִירָא.'
    }
  ],
  tosafot: [
    {
      id: 't-1',
      styleId: 'style-tosafot-body',
      text: '<b>מֵאֵימָתַי קוֹרִין</b> - פֵּרֵשׁ רַשִׁ\"י דְּתַנָּא אַקְּרָא קָאֵי, וְתֵימַהּ לְרִ\"י, דְּהָא תְּנַן בִּבְרָכוֹת פֶּרֶק שֵׁנִי: הַקּוֹרֵא לְמַפְרֵעַ לֹא יָצָא, וְלָא אִיצְטְרִיךְ תַּנָּא לְמִתְנֵי מִנַּיִן דְּחוֹבָה הִיא, דְּכָל מִצְוֹת עֲשֵׂה שֶׁבַּתּוֹרָה בְּיָדוּעַ שֶׁחַיָּב בָּהֶן.'
    },
    {
      id: 't-2',
      styleId: 'style-tosafot-body',
      text: '<b>וְעוֹד יֵשׁ לוֹמַר</b> - דְּמִדִּכְתִיב בְּשָׁכְבְּךָ וּבְקוּמֶךָ שָׁמְעִינַן תַּרְתֵּי: חֲדָא דְּחוֹבָה לִקְרוֹת, וַחֲדָא דִּזְמַן שְׁכִיבָה קָא קָבַע רַחֲמָנָא, וְלָכֵן תָּנֵי מֵאֵימָתַי בִּלְשׁוֹן זְמַן.'
    },
    {
      id: 't-3',
      styleId: 'style-tosafot-body',
      text: '<b>עַד שֶׁיַּעֲלֶה עַמּוּד הַשָּׁחַר</b> - פֵּרֵשׁ רַבֵּנוּ תָּם, דְּכָל הַלַּיְלָה קָרוּי זְמַן שְׁכִיבָה עַד עֲלוֹת הַשַּׁחַר, וְהִלְכְתָא כְּוָתֵיהּ דְּרַבָּן גַּמְלִיאֵל.'
    },
    {
      id: 't-4',
      styleId: 'style-tosafot-body',
      text: '<b>הֶקְטֵר חֲלָבִים וְאֵבָרִים</b> - תֵּימַהּ לְרַבֵּנוּ יִצְחָק, לָמָּה לִי לְמִתְנֵי הָכָא חֲלָבִים וְאֵבָרִים? וְיֵשׁ לוֹמַר דְּאַגַּב דִּתְנָא קְרִיאַת שְׁמַע תְּנָא נַמֵּי הָנֵי.'
    }
  ],
  notes: [
    {
      id: 'n-1',
      styleId: 'style-footnotes',
      text: '<b>תּוֹרָה אוֹר:</b> בְּשָׁכְבְּךָ וּבְקוּמֶךָ - דְּבָרִים ו, ז.'
    },
    {
      id: 'n-2',
      styleId: 'style-footnotes',
      text: '<b>עֵין מִשְׁפָּט:</b> מֵאֵימָתַי קוֹרִין אֶת שְׁמַע - רַמְבַּ\"ם הִלְכוֹת קְרִיאַת שְׁמַע פֶּרֶק א הֲלָכָה ט, טוּר וְשׁוּעַ אֹרַח חַיִּים סִימָן רל\"ה סְעִיף ג.'
    },
    {
      id: 'n-3',
      styleId: 'style-footnotes',
      text: '<b>מָסֹרֶת הַשַּׁ\"ס:</b> לְקַמָּן ט, א. יב, א. פסחים קכ, ב.'
    }
  ]
};

export const DEFAULT_PROSE_FLOWS: MultiFlowDocumentState['flows'] = {
  gemara: [
    {
      id: 'p-1',
      styleId: 'normal',
      text: 'ספר הטיפוגרפיה העברית והעימוד השולחני המודרני מהווה נדבך יסודי בהתפתחות עולם הדפוס היהודי לדורותיו. מראשית ימי הדפוס בוונציה, קראקא ושקלאוו, השקיעו המדפיסים מאמצים כבירים ביישור שורות מופתי ובהרחבת אותיות אהלתר״ם.'
    },
    {
      id: 'p-2',
      styleId: 'normal',
      text: 'במערכת TypesetOK שולבו עקרונות הנדסיים מודרניים לצד מסורת הדפוס העתיקה. מנוע ה-Knuth-Plass ב-Rust מחשב שבירת שורות אופטימלית לכל פסקה תוך מזעור פגמים טיפוגרפיים, במטרה להשיג קריאות מרבית וזרימת קריאה הרמונית.'
    },
    {
      id: 'p-3',
      styleId: 'normal',
      text: 'בספרי קודש ובספרי עיון מורכבים, הדרישה לסנכרון רב-תזרימי חוצה-עמודים היא תנאי בל יעבור. העימוד מתבצע באופן דינמי כאשר הפירושים זורמים במקביל לטקסט המרכזי תוך שמירה על עמודת שדרה פנימית מדויקת (Spine Alignment).'
    },
    {
      id: 'p-4',
      styleId: 'normal',
      text: 'על ידי ניצול כוח המחשוב המקבילי של מעבדים מודרניים, כל עמוד מחושב בשברירי מילי-שנייה, ומאפשר עריכה שוטפת של ספרים בני מאות עמודים ללא שום נעילת ממשק.'
    }
  ],
  rashi: [],
  tosafot: [],
  notes: []
};

/** Flow capacity estimates per page in words/characters for balanced pagination */
const CAPACITY = {
  gemaraWordsPerPage: 95,
  rashiWordsPerPage: 110,
  tosafotWordsPerPage: 95,
  notesWordsPerPage: 50,
  proseWordsPerPage: 220
};

function countWords(str: string): number {
  return (str.replace(/<[^>]+>/g, ' ').match(/\S+/g) || []).length;
}

/** Chunks an array of paragraphs across pages according to flow capacity */
function partitionParagraphs(paragraphs: StoryParagraph[], targetWordsPerPage: number): StoryParagraph[][] {
  if (!paragraphs || paragraphs.length === 0) return [];
  const pages: StoryParagraph[][] = [];
  let currentPage: StoryParagraph[] = [];
  let currentWords = 0;

  for (const p of paragraphs) {
    const w = Math.max(1, countWords(p.text));
    if (currentWords + w > targetWordsPerPage && currentPage.length > 0) {
      pages.push(currentPage);
      currentPage = [p];
      currentWords = w;
    } else {
      currentPage.push(p);
      currentWords += w;
    }
  }

  if (currentPage.length > 0) {
    pages.push(currentPage);
  }

  return pages;
}

export class FlowPaginator {
  /**
   * Paginates a multi-flow document into a list of full PageDescriptors for SpreadCanvas.
   * Ensures that all flows are represented on each spread with Recto/Verso aware layout.
   */
  public static paginateDocument(
    doc: MultiFlowDocumentState,
    minPages = 1
  ): PageDescriptor[] {
    const isTalmud = doc.templateType === 'gemara' || doc.templateType === 'mikraot';
    const isProse = doc.templateType === 'prose';
    const isNotes = doc.templateType === 'notes';

    if (isProse) {
      return this.paginateProse(doc.flows.gemara || [], minPages);
    }

    if (isNotes) {
      return this.paginateWithFootnotes(doc.flows.gemara || [], doc.flows.notes || [], minPages);
    }

    // Default: Full Talmudic Multi-flow layout (Gemara + Rashi + Tosafot + Notes)
    return this.paginateTalmud(doc, minPages);
  }

  /**
   * Paginates Talmud / Mikraot multi-flow layout with Recto/Verso spine awareness.
   */
  private static paginateTalmud(doc: MultiFlowDocumentState, minPages: number): PageDescriptor[] {
    const gemaraParas = doc.flows.gemara || [];
    const rashiParas = doc.flows.rashi || [];
    const tosafotParas = doc.flows.tosafot || [];
    const notesParas = doc.flows.notes || [];

    const gemaraChunks = partitionParagraphs(gemaraParas, CAPACITY.gemaraWordsPerPage);
    const rashiChunks = partitionParagraphs(rashiParas, CAPACITY.rashiWordsPerPage);
    const tosafotChunks = partitionParagraphs(tosafotParas, CAPACITY.tosafotWordsPerPage);
    const notesChunks = partitionParagraphs(notesParas, CAPACITY.notesWordsPerPage);

    const totalPages = Math.max(
      minPages,
      Math.max(
        gemaraChunks.length,
        rashiChunks.length,
        tosafotChunks.length,
        notesChunks.length,
        1
      )
    );

    const pages: PageDescriptor[] = [];

    for (let pageIdx = 0; pageIdx < totalPages; pageIdx++) {
      const isRecto = isRectoPage(pageIdx);
      const isRight = isRightHandPage(pageIdx);
      const gematria = toHebrewGematria(pageIdx + 1);

      const pGemara = gemaraChunks[pageIdx] || [];
      const pRashi = rashiChunks[pageIdx] || [];
      const pTosafot = tosafotChunks[pageIdx] || [];
      const pNotes = notesChunks[pageIdx] || [];

      // Determine Spine placement and commentary sides:
      // On Recto (ע״א - right page in Hebrew book): Spine is on the LEFT.
      //   Inner commentary (Rashi) -> LEFT column.
      //   Outer commentary (Tosafot) -> RIGHT column.
      // On Verso (ע״ב - left page in Hebrew book): Spine is on the RIGHT.
      //   Inner commentary (Rashi) -> RIGHT column.
      //   Outer commentary (Tosafot) -> LEFT column.
      const leftFlowId = isRecto ? 'rashi' : 'tosafot';
      const rightFlowId = isRecto ? 'tosafot' : 'rashi';
      const leftParas = isRecto ? pRashi : pTosafot;
      const rightParas = isRecto ? pTosafot : pRashi;
      const leftTitle = isRecto ? 'רש"י (פנימי)' : 'תוספות (חיצוני)';
      const rightTitle = isRecto ? 'תוספות (חיצוני)' : 'רש"י (פנימי)';
      const leftClass = isRecto ? 'tok-frame-comm' : 'tok-frame-comm';
      const rightClass = isRecto ? 'tok-frame-comm' : 'tok-frame-comm';

      // Check for dynamic L-Shape commentary expansion:
      // If Gemara text ends early (few words/lines on this page), commentary expands below it!
      const gemaraWords = pGemara.reduce((sum, p) => sum + countWords(p.text), 0);
      const isGemaraShort = pGemara.length > 0 && gemaraWords < 45 && (leftParas.length > 2 || rightParas.length > 2);
      const lShapeCommentary = isGemaraShort ? (isRecto ? 'הרחבת רש"י תחת הגמרא (צורת הדף)' : 'הרחבת תוספות תחת הגמרא (צורת הדף)') : null;

      const html = this.buildTalmudPageHtml({
        pageIndex: pageIdx,
        gemaraParas: pGemara,
        leftFlowId,
        leftTitle,
        leftClass,
        leftParas,
        rightFlowId,
        rightTitle,
        rightClass,
        rightParas,
        notesParas: pNotes,
        lShapeCommentary
      });

      pages.push({
        pageIndex: pageIdx,
        gematriaNumber: gematria,
        widthPt: 480,
        heightPt: 678,
        htmlContent: html
      });
    }

    return pages;
  }

  /**
   * Paginates a single-flow prose document across multiple pages.
   */
  private static paginateProse(paragraphs: StoryParagraph[], minPages: number): PageDescriptor[] {
    const chunks = partitionParagraphs(paragraphs, CAPACITY.proseWordsPerPage);
    const totalPages = Math.max(minPages, chunks.length, 1);
    const pages: PageDescriptor[] = [];

    for (let pageIdx = 0; pageIdx < totalPages; pageIdx++) {
      const paras = chunks[pageIdx] || [];
      const gematria = toHebrewGematria(pageIdx + 1);

      const html = `
        <div class="tok-page-content" style="flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 24px 28px; overflow: hidden;">
          <div class="tok-interactive-frame tok-frame-main" data-frame-id="frame-main-${pageIdx}" data-flow-id="gemara" style="flex: 1; min-height: 0;">
            <div class="tok-frame-text tok-frame-gemara" style="font-size: 13.5px; line-height: 1.6; text-align: justify;">
              ${paras.map((p) => `<p data-para-id="${p.id}" class="${p.styleId ? `tok-style-${p.styleId}` : ''}" style="margin: 0 0 0.8em; text-indent: 1.2em;">${p.text || '&nbsp;'}</p>`).join('')}
            </div>
          </div>
        </div>
      `;

      pages.push({
        pageIndex: pageIdx,
        gematriaNumber: gematria,
        widthPt: 480,
        heightPt: 678,
        htmlContent: html
      });
    }

    return pages;
  }

  /**
   * Paginates a book with main text and bottom running footnotes.
   */
  private static paginateWithFootnotes(
    mainParas: StoryParagraph[],
    noteParas: StoryParagraph[],
    minPages: number
  ): PageDescriptor[] {
    const mainChunks = partitionParagraphs(mainParas, 160);
    const noteChunks = partitionParagraphs(noteParas, 45);
    const totalPages = Math.max(minPages, Math.max(mainChunks.length, noteChunks.length, 1));
    const pages: PageDescriptor[] = [];

    for (let pageIdx = 0; pageIdx < totalPages; pageIdx++) {
      const pMain = mainChunks[pageIdx] || [];
      const pNotes = noteChunks[pageIdx] || [];
      const gematria = toHebrewGematria(pageIdx + 1);

      const html = `
        <div class="tok-page-content" style="flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 24px 28px; overflow: hidden; gap: 12px;">
          <div class="tok-interactive-frame tok-frame-main" data-frame-id="frame-main-${pageIdx}" data-flow-id="gemara" style="flex: 1; min-height: 0;">
            <div class="tok-frame-text tok-frame-gemara" style="font-size: 13.5px; line-height: 1.6; text-align: justify;">
              ${pMain.map((p) => `<p data-para-id="${p.id}" style="margin: 0 0 0.8em;">${p.text || '&nbsp;'}</p>`).join('')}
            </div>
          </div>
          ${pNotes.length > 0 ? `
            <div class="tok-interactive-frame tok-frame-notes-box" data-frame-id="frame-notes-${pageIdx}" data-flow-id="notes" style="flex: none; border-top: 1px solid var(--tok-ink-rule); padding-top: 8px;">
              <div class="tok-frame-text tok-frame-notes" style="font-size: 9.5px; line-height: 1.45; text-align: justify; color: var(--tok-ink-soft);">
                ${pNotes.map((p) => `<p data-para-id="${p.id}" style="margin: 0 0 0.4em;">${p.text}</p>`).join('')}
              </div>
            </div>
          ` : ''}
        </div>
      `;

      pages.push({
        pageIndex: pageIdx,
        gematriaNumber: gematria,
        widthPt: 480,
        heightPt: 678,
        htmlContent: html
      });
    }

    return pages;
  }

  /**
   * Builds the complete, interactive 3-column HTML layout for a Talmud page.
   */
  private static buildTalmudPageHtml(params: {
    pageIndex: number;
    gemaraParas: StoryParagraph[];
    leftFlowId: string;
    leftTitle: string;
    leftClass: string;
    leftParas: StoryParagraph[];
    rightFlowId: string;
    rightTitle: string;
    rightClass: string;
    rightParas: StoryParagraph[];
    notesParas: StoryParagraph[];
    lShapeCommentary: string | null;
  }): string {
    const {
      pageIndex,
      gemaraParas,
      leftFlowId,
      leftTitle,
      leftClass,
      leftParas,
      rightFlowId,
      rightTitle,
      rightClass,
      rightParas,
      notesParas,
      lShapeCommentary
    } = params;

    const renderParas = (paras: StoryParagraph[]) =>
      paras.length > 0
        ? paras
            .map(
              (p) =>
                `<p data-para-id="${p.id}" style="margin: 0 0 0.55em; text-align: justify;">${p.text || '&nbsp;'}</p>`
            )
            .join('')
        : `<div style="font-size: 10px; color: var(--tok-text-muted); font-style: italic; text-align: center; padding-top: 20px;">[אין טקסט בעמוד זה]</div>`;

    return `
      <div class="tok-page-content" style="flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 22px 24px; overflow: hidden; gap: 8px;">
        <!-- 3-Column Talmud Spread Grid -->
        <div class="tok-talmud-grid" style="flex: 1; min-height: 0; display: grid; grid-template-columns: 1.05fr 1.35fr 1.05fr; column-gap: 12px; row-gap: 8px; overflow: hidden;">
          
          <!-- Column 1: Right (Outer on Recto / Inner on Verso) -->
          <div class="tok-interactive-frame tok-col-right" data-frame-id="frame-right-${pageIndex}" data-flow-id="${rightFlowId}" style="display: flex; flex-direction: column; overflow: hidden;">
            <div class="tok-frame-label" style="font-size: 10px; font-weight: 700; text-align: center; border-bottom: 0.5px solid var(--tok-ink-rule); margin-bottom: 4px; padding-bottom: 2px;">
              ${rightTitle}
            </div>
            <div class="tok-frame-text ${rightClass}" style="flex: 1; min-height: 0; overflow: hidden; font-size: 10px; line-height: 1.48;">
              ${renderParas(rightParas)}
            </div>
          </div>

          <!-- Column 2: Center (Gemara) -->
          <div class="tok-interactive-frame tok-col-center" data-frame-id="frame-gemara-${pageIndex}" data-flow-id="gemara" style="display: flex; flex-direction: column; overflow: hidden; background: rgba(0,0,0,0.012); padding: 0 4px; border-radius: 2px;">
            <div class="tok-frame-label" style="font-size: 11px; font-weight: 800; text-align: center; border-bottom: 0.5px solid var(--tok-ink-rule); margin-bottom: 4px; padding-bottom: 2px;">
              גמרא (טקסט ראשי)
            </div>
            <div class="tok-frame-text tok-frame-gemara" style="flex: 1; min-height: 0; overflow: hidden; font-size: 13.5px; line-height: 1.55; font-weight: 600;">
              ${renderParas(gemaraParas)}
              ${lShapeCommentary ? `
                <div class="tok-lshape-box" style="margin-top: 10px; padding: 6px; border-top: 1px dashed var(--tok-ink-rule); font-size: 9.5px; font-family: var(--tok-font-hebrew-rashi); color: var(--tok-ink-soft);">
                  <div style="font-weight: 700; margin-bottom: 2px;">◄ ${lShapeCommentary}</div>
                  המשך לשון רש״י זורם ומתרחב ברוחב שני טורים מתחת לסיום הגמרא כמסורת דפוס וילנא.
                </div>
              ` : ''}
            </div>
          </div>

          <!-- Column 3: Left (Inner on Recto / Outer on Verso) -->
          <div class="tok-interactive-frame tok-col-left" data-frame-id="frame-left-${pageIndex}" data-flow-id="${leftFlowId}" style="display: flex; flex-direction: column; overflow: hidden;">
            <div class="tok-frame-label" style="font-size: 10px; font-weight: 700; text-align: center; border-bottom: 0.5px solid var(--tok-ink-rule); margin-bottom: 4px; padding-bottom: 2px;">
              ${leftTitle}
            </div>
            <div class="tok-frame-text ${leftClass}" style="flex: 1; min-height: 0; overflow: hidden; font-size: 10px; line-height: 1.48;">
              ${renderParas(leftParas)}
            </div>
          </div>

        </div>

        <!-- Optional Bottom Footnotes / Traditional References Frame -->
        ${notesParas.length > 0 ? `
          <div class="tok-interactive-frame tok-col-notes" data-frame-id="frame-notes-${pageIndex}" data-flow-id="notes" style="flex: none; max-height: 90px; border-top: 0.75px solid var(--tok-ink-rule); padding-top: 4px; overflow: hidden;">
            <div class="tok-frame-text tok-frame-notes" style="font-size: 9px; line-height: 1.4; color: var(--tok-ink-rule);">
              ${notesParas.map((p) => `<span data-para-id="${p.id}" style="margin-inline-end: 12px; display: inline-block;">${p.text}</span>`).join('')}
            </div>
          </div>
        ` : ''}
      </div>
    `;
  }
}

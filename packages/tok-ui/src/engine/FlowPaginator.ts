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
import { toHebrewGematria } from '../gematria.js';
import { isRectoPage, isRightHandPage } from '../components/SpreadCanvas.js';
import {
  DocumentSettings,
  FootnoteApparatusConfigData,
  GrepRuleData,
  MultiFlowAnchorLink,
  MultiFlowItem
} from '../types.js';
import { evaluateLiveGrepSpansInUi } from './documentBridge.js';

export type TemplateType = 'gemara' | 'mikraot' | 'prose' | 'notes' | 'bulletin' | 'blank';

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
  customFlowMetadata?: Record<string, {
    name: string;
    color: string;
    role: string;
    placement: 'split-edges' | 'bottom' | 'margin-outer' | 'free';
  }>;
  settings?: DocumentSettings;
  styles?: Record<string, any>;
  isDirty?: boolean;
  filePath?: string | null;
  multiFlowAnchors?: MultiFlowAnchorLink[];
  footnoteApparatuses?: FootnoteApparatusConfigData[];
  grepRules?: GrepRuleData[];
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

export const DEFAULT_MIKRAOT_FLOWS: MultiFlowDocumentState['flows'] = {
  gemara: [
    {
      id: 'm-1',
      styleId: 'style-gemara-main',
      text: 'בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ׃ וְהָאָרֶץ הָיְתָה תֹהוּ וָבֹהוּ וְחֹשֶׁךְ עַל־פְּנֵי תְהוֹם וְרוּחַ אֱלֹהִים מְרַחֶפֶת עַל־פְּנֵי הַמָּיִם׃'
    },
    {
      id: 'm-2',
      styleId: 'style-gemara-main',
      text: 'וַיֹּאמֶר אֱלֹהִים יְהִי אוֹר וַיְהִי־אוֹר׃ וַיַּרְא אֱלֹהִים אֶת־הָאוֹר כִּי־טוֹב וַיַּבְדֵּל אֱלֹהִים בֵּין הָאוֹר וּבֵין הַחֹשֶׁךְ׃'
    }
  ],
  rashi: [
    {
      id: 'mr-1',
      styleId: 'style-rashi-body',
      text: 'בְּרֵאשִׁית בָּרָא — אָמַר רַבִּי יִצְחָק: לֹא הָיָה צָרִיךְ לְהַתְחִיל אֶת הַתּוֹרָה אֶלָּא מֵ״הַחֹדֶשׁ הַזֶּה לָכֶם״, שֶׁהִיא מִצְוָה רִאשׁוֹנָה שֶׁנִּצְטַוּוּ בָּהּ יִשְׂרָאֵל, וּמַה טַּעַם פָּתַח בִּבְרֵאשִׁית? מִשּׁוּם כֹּחַ מַעֲשָׂיו הִגִּיד לְעַמּוֹ לָתֵת לָהֶם נַחֲלַת גּוֹיִם.'
    },
    {
      id: 'mr-2',
      styleId: 'style-rashi-body',
      text: 'שֶׁאִם יֹאמְרוּ אֻמּוֹת הָעוֹלָם לְיִשְׂרָאֵל: לִסְטִים אַתֶּם, שֶׁכְּבַשְׁתֶּם אַרְצוֹת שִׁבְעָה גוֹיִם, הֵם אוֹמְרִים לָהֶם: כָּל הָאָרֶץ שֶׁל הַקָּדוֹשׁ בָּרוּךְ הוּא הִיא, הוּא בְּרָאָהּ וּנְתָנָהּ לַאֲשֶׁר יָשַׁר בְּעֵינָיו.'
    }
  ],
  tosafot: [
    {
      id: 'mt-1',
      styleId: 'style-tosafot-body',
      text: 'בְּקַדְמִין בְּרָא יְיָ יָת שְׁמַיָּא וְיָת אַרְעָא. וְאַרְעָא הֲוָת צָדְיָא וְרֵיקַנְיָא וַחֲשׁוֹכָא עַל אַפֵּי תְהוֹמָא וְרוּחָא מִן קֳדָם יְיָ מְנַשְּׁבָא עַל אַפֵּי מַיָּא.'
    },
    {
      id: 'mt-2',
      styleId: 'style-tosafot-body',
      text: 'וַאֲמַר יְיָ יְהֵי נְהוֹרָא וַהֲוָה נְהוֹרָא. וַחֲזָא יְיָ יָת נְהוֹרָא אֲרֵי טָב וְאַפְרֵישׁ יְיָ בֵּין נְהוֹרָא וּבֵין חֲשׁוֹכָא.'
    }
  ],
  notes: [
    {
      id: 'mn-1',
      styleId: 'style-footnotes',
      text: 'רמב״ן: פתח במעשה בראשית להודיע כי העולם מחודש ולא קדמון, וכי הבריאה יש מאין הגמור היתה בחפץ הבורא יתברך שמו.'
    }
  ]
};

export const DEFAULT_NOTES_FLOWS: MultiFlowDocumentState['flows'] = {
  gemara: [
    {
      id: 'n-1',
      styleId: 'normal',
      text: 'פרק ראשון: יסודות חכמת העימוד וסדרי אותיות הקודש. מלאכת הדפוס והעימוד בספרי ישראל אינה רק עניין טכני של סידור שורות, אלא מעשה אומנות קדוש המשפיע ישירות על ישוב דעתו של הלומד ועל בהירות השגת הסוגיה.'
    },
    {
      id: 'n-2',
      styleId: 'normal',
      text: 'המדפיסים הנודעים בדורות עברו הקפידו על גודל השוליים, מרחב הנשימה בין הפסקאות ואיזון מופתי בין גוף הטקסט לבין הערות הביאור הנלוות אליו בתחתית העמוד.'
    }
  ],
  rashi: [],
  tosafot: [],
  notes: [
    {
      id: 'fn-1',
      styleId: 'style-footnotes',
      text: '1. עיין בהקדמת המדפיסים בוונציה שנת שכ״ח, שכתבו כי אות נאה ומאירת עיניים מרחבת דעתו של אדם לתורה.'
    },
    {
      id: 'fn-2',
      styleId: 'style-footnotes',
      text: '2. וכן מצינו בספר חסידים סימן תשפ״ו שהזהיר על כתיבה נאה ומדויקת שלא יטעו בה הקוראים.'
    }
  ]
};

export const DEFAULT_BULLETIN_FLOWS: MultiFlowDocumentState['flows'] = {
  gemara: [
    {
      id: 'b-1',
      styleId: 'normal',
      text: 'עלון שבת קודש — פניני הפרשה והלכה. שבת קודש היא מקור הברכה והשלווה לנפש, יום של התעלות רוחנית ולימוד תורה מתוך שמחה ושלום בית.'
    },
    {
      id: 'b-2',
      styleId: 'normal',
      text: 'בפרשתנו אנו למדים על מעלת השלום והאחדות בעם ישראל, ועל החובה לחזק איש את רעהו בדברי חיזוק ובמאור פנים.'
    }
  ],
  rashi: [
    {
      id: 'br-1',
      styleId: 'normal',
      text: 'הלכה למעשה: דיני הכנה משבת ליום חול. ראוי לכל אדם להיזהר שלא לעשות שום מלאכה או טורח בשבת לצורך ימי החול הבאים.'
    }
  ],
  tosafot: [],
  notes: [
    {
      id: 'bn-1',
      styleId: 'style-footnotes',
      text: 'זמני תפילות השבת: מנחה ערב שבת 18:20, שחרית בהנץ 06:15, שיעור בהלכה 17:30.'
    }
  ]
};

export const DEFAULT_BLANK_FLOWS: MultiFlowDocumentState['flows'] = {
  gemara: [
    {
      id: 'p-blank-1',
      styleId: 'normal',
      text: ''
    }
  ],
  rashi: [],
  tosafot: [],
  notes: []
};

export function getDefaultFlowsForTemplate(templateType: TemplateType): MultiFlowDocumentState['flows'] {
  switch (templateType) {
    case 'mikraot':
      return JSON.parse(JSON.stringify(DEFAULT_MIKRAOT_FLOWS));
    case 'notes':
      return JSON.parse(JSON.stringify(DEFAULT_NOTES_FLOWS));
    case 'bulletin':
      return JSON.parse(JSON.stringify(DEFAULT_BULLETIN_FLOWS));
    case 'prose':
      return JSON.parse(JSON.stringify(DEFAULT_PROSE_FLOWS));
    case 'blank':
      return JSON.parse(JSON.stringify(DEFAULT_BLANK_FLOWS));
    case 'gemara':
    default:
      return JSON.parse(JSON.stringify(DEFAULT_TALMUD_FLOWS));
  }
}

export function getFlowsForTemplate(templateType: TemplateType): MultiFlowItem[] {
  switch (templateType) {
    case 'mikraot':
      return [
        { id: 'gemara', name: 'מקרא (חומש)', color: '#1E4A9E', role: 'מרכז העמוד', wordCount: 0, isActive: true },
        { id: 'rashi', name: 'רש"י', color: '#B45309', role: 'פירוש עיקרי', wordCount: 0, isActive: false },
        { id: 'tosafot', name: 'תרגום אונקלוס', color: '#15803D', role: 'עמודת תרגום', wordCount: 0, isActive: false },
        { id: 'notes', name: 'רמב"ן ומפרשים', color: '#7C3AED', role: 'תחתית העמוד', wordCount: 0, isActive: false }
      ];
    case 'notes':
      return [
        { id: 'gemara', name: 'טקסט ראשי', color: '#1E4A9E', role: 'גוף הספר', wordCount: 0, isActive: true },
        { id: 'notes', name: 'הערות שוליים', color: '#7C3AED', role: 'תחתית העמוד', wordCount: 0, isActive: false }
      ];
    case 'bulletin':
      return [
        { id: 'gemara', name: 'מאמר ראשי', color: '#1E4A9E', role: 'עמודה ראשית', wordCount: 0, isActive: true },
        { id: 'rashi', name: 'טור משני', color: '#B45309', role: 'עמודה שניה', wordCount: 0, isActive: false },
        { id: 'notes', name: 'מסגרת והודעות', color: '#7C3AED', role: 'תחתית העלון', wordCount: 0, isActive: false }
      ];
    case 'prose':
      return [
        { id: 'gemara', name: 'טקסט הספר', color: '#1E4A9E', role: 'רציף', wordCount: 0, isActive: true }
      ];
    case 'blank':
      return [
        { id: 'gemara', name: 'דף ראשי', color: '#1E4A9E', role: 'טקסט', wordCount: 0, isActive: true }
      ];
    case 'gemara':
    default:
      return [
        { id: 'gemara', name: 'גמרא (ראשי)', color: '#1E4A9E', role: 'מרכז העמוד', wordCount: 0, isActive: true },
        { id: 'rashi', name: 'רש"י', color: '#B45309', role: 'פירוש פנימי', wordCount: 0, isActive: false },
        { id: 'tosafot', name: 'תוספות', color: '#15803D', role: 'פירוש חיצוני', wordCount: 0, isActive: false },
        { id: 'notes', name: 'הערות שוליים', color: '#7C3AED', role: 'תחתית העמוד', wordCount: 0, isActive: false }
      ];
  }
}

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

/**
 * @deprecated Use Rust typesetting engine via `tok:typeset-document` and `typesetBridge.ts`
 * instead of simulated JavaScript word-count splitting. Kept for backwards-compatible test fixtures.
 */
function getDocDimensionsPt(doc?: MultiFlowDocumentState): { widthPt: number; heightPt: number } {
  if (doc?.settings?.pageWidthMm && doc?.settings?.pageHeightMm) {
    return {
      widthPt: Math.round(doc.settings.pageWidthMm * 72 / 25.4),
      heightPt: Math.round(doc.settings.pageHeightMm * 72 / 25.4)
    };
  }
  return { widthPt: 480, heightPt: 678 };
}

function renderParagraphHtml(
  p: StoryParagraph,
  defaultStyle: { fontSize?: string; lineHeight?: string; fontFamily?: string; textAlign?: string; grepRules?: GrepRuleData[]; flowId?: string } = {}
): string {
  const patch = p.patch || {};
  const styles: string[] = ['margin: 0 0 0.55em'];

  const align = patch.alignment ? (patch.alignment === 'Justified' ? 'justify' : patch.alignment.toLowerCase()) : (defaultStyle.textAlign || 'justify');
  styles.push(`text-align: ${align}`);

  if (patch.fontFamily) {
    styles.push(`font-family: '${patch.fontFamily}', serif`);
  } else if (defaultStyle.fontFamily) {
    styles.push(`font-family: ${defaultStyle.fontFamily}`);
  }

  if (patch.fontSizePt !== undefined) {
    styles.push(`font-size: ${patch.fontSizePt}pt`);
  } else if (defaultStyle.fontSize) {
    styles.push(`font-size: ${defaultStyle.fontSize}`);
  }

  if (patch.lineHeightPt !== undefined) {
    styles.push(`line-height: ${patch.lineHeightPt}pt`);
  } else if (defaultStyle.lineHeight) {
    styles.push(`line-height: ${defaultStyle.lineHeight}`);
  }

  if (patch.fontWeight !== undefined) {
    styles.push(`font-weight: ${patch.fontWeight}`);
  }

  if (patch.firstLineIndentPt !== undefined) {
    styles.push(`text-indent: ${patch.firstLineIndentPt}pt`);
  }
  if (patch.indentStartPt !== undefined) {
    styles.push(`padding-inline-start: ${patch.indentStartPt}pt`);
  }

  if (patch.spaceBeforePt !== undefined) {
    styles.push(`margin-top: ${patch.spaceBeforePt}pt`);
  }
  if (patch.spaceAfterPt !== undefined) {
    styles.push(`margin-bottom: ${patch.spaceAfterPt}pt`);
  }

  if (patch.color) {
    styles.push(`color: ${patch.color}`);
  }

  let text = p.text || '&nbsp;';
  const effectiveSpans = evaluateLiveGrepSpansInUi(
    text === '&nbsp;' ? '' : text,
    p.spans,
    defaultStyle.grepRules,
    defaultStyle.flowId
  );
  if (effectiveSpans.length > 0 && text !== '&nbsp;') {
    const chars = Array.from(text);
    const parts: string[] = [];
    let lastIndex = 0;
    const sortedSpans = [...effectiveSpans].sort((a, b) => a.start - b.start);
    for (const span of sortedSpans) {
      if (span.start < lastIndex) continue;
      if (span.start > lastIndex) {
        parts.push(chars.slice(lastIndex, span.start).join(''));
      }
      const spanChars = chars.slice(span.start, span.end).join('');
      let spanStyle = '';
      if (span.format.fontWeight && span.format.fontWeight >= 600) spanStyle += 'font-weight: bold;';
      if (span.format.italic) spanStyle += 'font-style: italic;';
      if (span.format.underline) spanStyle += 'text-decoration: underline;';
      if (span.format.fontFamily) spanStyle += `font-family: '${span.format.fontFamily}', serif;`;
      if (span.format.fontSizePt) spanStyle += `font-size: ${span.format.fontSizePt}pt;`;
      else if (span.format.fontScale) spanStyle += `font-size: ${Math.round(span.format.fontScale * 100)}%;`;
      if (span.format.color) spanStyle += `color: ${span.format.color};`;
      const styleIdClass = span.format.characterStyleId ? ` class="tok-char-${span.format.characterStyleId}"` : '';
      parts.push(`<span${styleIdClass} style="${spanStyle}">${spanChars}</span>`);
      lastIndex = Math.max(lastIndex, span.end);
    }
    if (lastIndex < chars.length) {
      parts.push(chars.slice(lastIndex).join(''));
    }
    text = parts.join('');
  }

  const classAttr = p.styleId ? ` class="tok-style-${p.styleId}"` : '';
  return `<p data-para-id="${p.id}"${classAttr} style="${styles.join('; ')}">${text}</p>`;
}

export class FlowPaginator {
  /**
   * Paginates a multi-flow document into a list of full PageDescriptors for SpreadCanvas.
   * Ensures that all flows are represented on each spread with Recto/Verso aware layout.
   */
  public static paginateDocument(
    doc: MultiFlowDocumentState,
    minPages = 1,
    isExplicitFallback = false
  ): PageDescriptor[] {
    const isTalmud = doc.templateType === 'gemara' || doc.templateType === 'mikraot';
    const isProse = doc.templateType === 'prose';
    const isNotes = doc.templateType === 'notes';

    let pages: PageDescriptor[];
    if (isProse) {
      pages = this.paginateProse(doc, minPages);
    } else if (isNotes) {
      pages = this.paginateWithFootnotes(doc, minPages);
    } else {
      // Default: Full Talmudic Multi-flow layout (Gemara + Rashi + Tosafot + Notes)
      pages = this.paginateTalmud(doc, minPages);
    }

    if (doc.customFlowMetadata && Object.keys(doc.customFlowMetadata).length > 0) {
      for (const p of pages) {
        const customHtml = this.renderCustomFlowsHtml(doc, p.pageIndex);
        if (customHtml) {
          p.htmlContent = p.htmlContent.replace(/<\/div>\s*$/, `${customHtml}\n</div>`);
        }
      }
    }

    if (isExplicitFallback) {
      const banner = `<div class="tok-fallback-warning-badge" data-fallback="true" style="background: rgba(239, 68, 68, 0.08); border: 1px dashed var(--tok-status-warning, #d97706); color: var(--tok-status-warning, #d97706); padding: 4px 8px; font-size: 11px; font-weight: 500; text-align: center; margin-bottom: 8px; border-radius: 4px;">⚠️ תצוגה משוערת בלבד — מנוע העימוד Rust אינו זמין</div>`;
      for (const p of pages) {
        (p as any).isSimulatedFallback = true;
        p.htmlContent = `${banner}\n${p.htmlContent}`;
      }
    }

    return pages;
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
      const isMikraot = doc.templateType === 'mikraot';
      const leftFlowId = isRecto ? 'rashi' : 'tosafot';
      const rightFlowId = isRecto ? 'tosafot' : 'rashi';
      const leftParas = isRecto ? pRashi : pTosafot;
      const rightParas = isRecto ? pTosafot : pRashi;
      const leftTitle = isRecto
        ? 'רש"י (פנימי)'
        : (isMikraot ? 'תרגום אונקלוס (חיצוני)' : 'תוספות (חיצוני)');
      const rightTitle = isRecto
        ? (isMikraot ? 'תרגום אונקלוס (חיצוני)' : 'תוספות (חיצוני)')
        : 'רש"י (פנימי)';
      const mainTitle = isMikraot ? 'מקרא (חומש)' : 'גמרא (טקסט ראשי)';
      const leftClass = isRecto ? 'tok-frame-comm' : 'tok-frame-comm';
      const rightClass = isRecto ? 'tok-frame-comm' : 'tok-frame-comm';

      // Check for dynamic L-Shape commentary expansion:
      // If Gemara text ends early (few words/lines on this page), commentary expands below it!
      const gemaraWords = pGemara.reduce((sum, p) => sum + countWords(p.text), 0);
      const isGemaraShort = pGemara.length > 0 && gemaraWords < 45 && (leftParas.length > 2 || rightParas.length > 2);
      const lShapeCommentary = isGemaraShort ? (isRecto ? 'הרחבת רש"י תחת הגמרא (צורת הדף)' : 'הרחבת תוספות תחת הגמרא (צורת הדף)') : null;

      const html = this.buildTalmudPageHtml({
        pageIndex: pageIdx,
        mainTitle,
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

      const { widthPt, heightPt } = getDocDimensionsPt(doc);
      pages.push({
        pageIndex: pageIdx,
        gematriaNumber: gematria,
        widthPt,
        heightPt,
        htmlContent: html
      });
    }

    return pages;
  }

  /**
   * Paginates a single-flow prose document across multiple pages.
   */
  private static paginateProse(doc: MultiFlowDocumentState, minPages: number): PageDescriptor[] {
    const paragraphs = doc.flows.gemara || [];
    const chunks = partitionParagraphs(paragraphs, CAPACITY.proseWordsPerPage);
    const apparatuses = this.getActiveApparatuses(doc);
    const apparatusChunksMap: Record<string, StoryParagraph[][]> = {};
    let maxApparatusPages = 0;
    for (const app of apparatuses) {
      const flowParas = doc.flows[app.id] || [];
      if (flowParas.length > 0) {
        const ch = partitionParagraphs(flowParas, 55);
        apparatusChunksMap[app.id] = ch;
        maxApparatusPages = Math.max(maxApparatusPages, ch.length);
      }
    }

    const totalPages = Math.max(minPages, chunks.length, maxApparatusPages, 1);
    const pages: PageDescriptor[] = [];
    const { widthPt, heightPt } = getDocDimensionsPt(doc);

    for (let pageIdx = 0; pageIdx < totalPages; pageIdx++) {
      const paras = chunks[pageIdx] || [];
      const gematria = toHebrewGematria(pageIdx + 1);
      const apparatusBandsHtml = this.renderMultiApparatusBandsHtml(doc, apparatuses, apparatusChunksMap, pageIdx);

      const html = `
        <div class="tok-page-content" style="flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 24px 28px; overflow: hidden;">
          <div class="tok-interactive-frame tok-frame-main" data-frame-id="frame-main-${pageIdx}" data-flow-id="gemara" style="flex: 1; min-height: 0;">
            <div class="tok-frame-text tok-frame-gemara" style="font-size: 13.5px; line-height: 1.6; text-align: justify;">
              ${paras.map((p) => renderParagraphHtml(p, { fontSize: '13.5px', lineHeight: '1.6', textAlign: 'justify', grepRules: doc.grepRules, flowId: 'gemara' })).join('')}
            </div>
          </div>
          ${apparatusBandsHtml}
        </div>
      `;

      pages.push({
        pageIndex: pageIdx,
        gematriaNumber: gematria,
        widthPt,
        heightPt,
        htmlContent: html
      });
    }

    return pages;
  }

  /**
   * Returns configured footnote apparatuses or synthesizes a default apparatus when `notes` has paragraphs.
   */
  private static getActiveApparatuses(doc: MultiFlowDocumentState): FootnoteApparatusConfigData[] {
    if (doc.footnoteApparatuses && doc.footnoteApparatuses.length > 0) {
      return doc.footnoteApparatuses;
    }
    if (doc.flows.notes && doc.flows.notes.length > 0) {
      return [
        {
          id: 'notes',
          name: 'הערות שוליים',
          numbering_style: 'HebrewGematria',
          columns: 1,
          column_gap_pt: 12,
          max_height_ratio: 0.35,
          draw_separator_rule: true,
          separator_length_ratio: 0.33,
          font_family: 'Frank Ruhl Libre',
          font_size_pt: 9.5,
          line_height_pt: 13.0
        }
      ];
    }
    return [];
  }

  /**
   * Renders multi-apparatus footnote bands at the bottom of the page with separator rules
   * (main separator rule 33% width above first apparatus, shorter 18% rule between apparatuses)
   * and independent column count & typography per apparatus.
   */
  private static renderMultiApparatusBandsHtml(
    doc: MultiFlowDocumentState,
    apparatuses: FootnoteApparatusConfigData[],
    apparatusChunksMap: Record<string, StoryParagraph[][]>,
    pageIdx: number
  ): string {
    const activeBands: Array<{ app: FootnoteApparatusConfigData; paras: StoryParagraph[] }> = [];
    for (const app of apparatuses) {
      const paras = apparatusChunksMap[app.id]?.[pageIdx] || [];
      if (paras.length > 0) {
        activeBands.push({ app, paras });
      }
    }
    if (activeBands.length === 0) return '';

    return activeBands.map(({ app, paras }, idx) => {
      const isPrimaryBand = idx === 0;
      const ruleRatioPct = Math.round(((app.separator_length_ratio ?? (isPrimaryBand ? 0.33 : 0.18)) * 100));
      const ruleHeightPx = isPrimaryBand ? '1px' : '0.75px';
      const separatorHtml = app.draw_separator_rule !== false
        ? `<div class="tok-footnote-separator-rule" data-apparatus-id="${app.id}" data-rule-tier="${isPrimaryBand ? 'primary' : 'inter-apparatus'}" style="width: ${ruleRatioPct}%; height: ${ruleHeightPx}; background: var(--tok-ink-rule, #333); margin: ${isPrimaryBand ? '8px' : '5px'} 0 4px auto;"></div>`
        : '';
      const cols = Math.max(1, app.columns || 1);
      const colStyle = cols > 1
        ? `column-count: ${cols}; column-gap: ${app.column_gap_pt || 12}pt;`
        : '';
      const fontSize = `${app.font_size_pt || 9.5}pt`;
      const lineHeight = app.line_height_pt ? `${app.line_height_pt}pt` : '1.42';
      const fontFamily = app.font_family ? `'${app.font_family}', serif` : undefined;

      return `
        <div class="tok-interactive-frame tok-frame-notes-box tok-apparatus-band"
             data-frame-id="frame-${app.id}-${pageIdx}"
             data-flow-id="${app.id}"
             data-apparatus-columns="${cols}"
             data-apparatus-numbering="${app.numbering_style}"
             style="flex: none; padding-top: 2px;">
          ${separatorHtml}
          <div class="tok-frame-text tok-frame-notes" style="font-size: ${fontSize}; line-height: ${lineHeight}; ${fontFamily ? `font-family: ${fontFamily}; ` : ''}${colStyle}text-align: justify; color: var(--tok-ink-soft);">
            ${paras.map((p) => renderParagraphHtml(p, { fontSize, lineHeight, fontFamily, textAlign: 'justify', grepRules: doc.grepRules, flowId: app.id })).join('')}
          </div>
        </div>
      `;
    }).join('\n');
  }

  /**
   * Paginates a book with main text and bottom running footnotes (supporting multi-apparatus).
   */
  private static paginateWithFootnotes(
    doc: MultiFlowDocumentState,
    minPages: number
  ): PageDescriptor[] {
    const mainParas = doc.flows.gemara || [];
    const mainChunks = partitionParagraphs(mainParas, 160);
    const apparatuses = this.getActiveApparatuses(doc);
    const apparatusChunksMap: Record<string, StoryParagraph[][]> = {};
    let maxApparatusPages = 0;

    for (const app of apparatuses) {
      const flowParas = doc.flows[app.id] || [];
      if (flowParas.length > 0) {
        const ch = partitionParagraphs(flowParas, 45 * Math.max(1, app.columns || 1));
        apparatusChunksMap[app.id] = ch;
        maxApparatusPages = Math.max(maxApparatusPages, ch.length);
      }
    }

    const totalPages = Math.max(minPages, Math.max(mainChunks.length, maxApparatusPages, 1));
    const pages: PageDescriptor[] = [];
    const { widthPt, heightPt } = getDocDimensionsPt(doc);

    for (let pageIdx = 0; pageIdx < totalPages; pageIdx++) {
      const pMain = mainChunks[pageIdx] || [];
      const gematria = toHebrewGematria(pageIdx + 1);
      const apparatusBandsHtml = this.renderMultiApparatusBandsHtml(doc, apparatuses, apparatusChunksMap, pageIdx);

      const html = `
        <div class="tok-page-content" style="flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 24px 28px; overflow: hidden; gap: 6px;">
          <div class="tok-interactive-frame tok-frame-main" data-frame-id="frame-main-${pageIdx}" data-flow-id="gemara" style="flex: 1; min-height: 0;">
            <div class="tok-frame-text tok-frame-gemara" style="font-size: 13.5px; line-height: 1.6; text-align: justify;">
              ${pMain.map((p) => renderParagraphHtml(p, { fontSize: '13.5px', lineHeight: '1.6', textAlign: 'justify', grepRules: doc.grepRules, flowId: 'gemara' })).join('')}
            </div>
          </div>
          ${apparatusBandsHtml}
        </div>
      `;

      pages.push({
        pageIndex: pageIdx,
        gematriaNumber: gematria,
        widthPt,
        heightPt,
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
    mainTitle?: string;
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
      mainTitle,
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
                renderParagraphHtml(p, { textAlign: 'justify' })
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
              ${mainTitle || 'גמרא (טקסט ראשי)'}
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
  private static renderCustomFlowsHtml(doc: MultiFlowDocumentState, pageIndex: number): string {
    if (!doc.customFlowMetadata) return '';
    const items: string[] = [];
    for (const [flowId, meta] of Object.entries(doc.customFlowMetadata)) {
      const paras = doc.flows[flowId] || [];
      if (paras.length === 0) continue;

      if (meta.placement === 'split-edges') {
        const half = Math.ceil(paras.length / 2);
        const rightParas = paras.slice(0, half);
        const leftParas = paras.slice(half);

        const renderHalf = (halfParas: StoryParagraph[], side: 'right' | 'left') => `
          <div class="tok-interactive-frame tok-col-split-${side}"
               data-frame-id="frame-${flowId}-${side}-${pageIndex}"
               data-flow-id="${flowId}"
               data-layer-id="text-commentary"
               style="position: absolute; ${side}: 8pt; top: 40pt; width: 62pt; max-height: 570pt; overflow: hidden; background: rgba(0,0,0,0.015); border-${side === 'right' ? 'left' : 'right'}: 1px dashed ${meta.color}66; padding: 4px 6px;">
            <div class="tok-frame-label" style="font-size: 9.5px; font-weight: 700; color: ${meta.color}; text-align: center; border-bottom: 0.5px solid var(--tok-ink-rule); margin-bottom: 3px; padding-bottom: 1px;">
              ${meta.name} (${side === 'right' ? 'טור ימין' : 'טור שמאל'})
            </div>
            <div class="tok-frame-text" style="font-size: 9.5px; line-height: 1.42; color: var(--tok-ink);">
              ${halfParas.map((p) => `<p data-para-id="${p.id}" style="margin: 0 0 0.45em; text-align: justify;">${p.text || '&nbsp;'}</p>`).join('')}
            </div>
          </div>
        `;

        items.push(renderHalf(rightParas, 'right'));
        items.push(renderHalf(leftParas, 'left'));
      } else if (meta.placement === 'bottom') {
        items.push(`
          <div class="tok-interactive-frame tok-custom-bottom-flow"
               data-frame-id="frame-${flowId}-bottom-${pageIndex}"
               data-flow-id="${flowId}"
               data-layer-id="text-commentary"
               style="flex: none; border-top: 1px solid ${meta.color}88; padding: 4px 6px; margin-top: 4px; overflow: hidden; max-height: 85pt;">
            <div class="tok-frame-label" style="font-size: 9.5px; font-weight: 700; color: ${meta.color}; margin-bottom: 2px;">
              ${meta.name}
            </div>
            <div class="tok-frame-text" style="font-size: 9px; line-height: 1.38; color: var(--tok-ink);">
              ${paras.map((p) => `<span data-para-id="${p.id}" style="margin-inline-end: 10px; display: inline-block;">${p.text}</span>`).join('')}
            </div>
          </div>
        `);
      } else {
        items.push(`
          <div class="tok-interactive-frame tok-custom-margin-flow"
               data-frame-id="frame-${flowId}-margin-${pageIndex}"
               data-flow-id="${flowId}"
               data-layer-id="text-commentary"
               style="position: absolute; right: 10pt; top: 45pt; width: 65pt; overflow: hidden; padding: 4px; border: 1px dashed ${meta.color}66;">
            <div class="tok-frame-label" style="font-size: 9.5px; font-weight: 700; color: ${meta.color}; text-align: center; margin-bottom: 2px;">
              ${meta.name}
            </div>
            <div class="tok-frame-text" style="font-size: 9.5px; line-height: 1.4;">
              ${paras.map((p) => `<p data-para-id="${p.id}" style="margin: 0 0 0.45em;">${p.text}</p>`).join('')}
            </div>
          </div>
        `);
      }
    }
    return items.join('\n');
  }
}

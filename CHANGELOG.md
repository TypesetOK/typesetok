# TypesetOK Changelog

מקור האמת המרכזי לכל עדכוני התוכנה ויומן השינויים של TypesetOK.
המידע בקובץ זה נשאב באופן אוטומטי לחלוניות "מה חדש" ולמודאל הכרזות הגרסה בממשק התוכנה.

---

## [0.35.2] - 2026-11-09
- **Importance**: patch
- **Badge**: עברית: תיקון ושדרוג | en: Fixes & Update Improvements
- **Title**: עברית: שדרוג מנגנון העדכון ותיקון אתחול מסמכים | en: Upgraded In-App Updater & Document Initialization Fix
- **Summary**: עברית: שדרוג מקיף של מנגנון העדכון עם הורדה ישירה מתוך התוכנה ללא תלות בדפדפן, אימות תקינות וקובץ SHA-256 בזמן ההורדה, איתור מתקינים מקומיים בכוננים חיצוניים ובתיקיות נפוצות והשוואת גרסאות אוטומטית. בנוסף, תיקון סדר אתחול ContextualInspector כדי לאפשר יצירה וייבוא תקינים של מסמכים ולמנוע סביבת עבודה ריקה. | en: Comprehensive in-app updater improvements with browserless direct downloads, streaming SHA-256 and executable validation, local installer discovery across external drives and common folders, and automatic version comparison. Also fixes ContextualInspector initialization order so document creation and DOCX import work correctly instead of opening a blank workspace.

### Highlights
- [updater-streaming] icon: download | badge: עברית: עדכון מובנה, en: In-App Updates
  - Title: עברית: הורדה ישירה ובטוחה מתוך התוכנה | en: Direct & Resilient In-App Downloads
  - Description: עברית: הורדה בזרימה לקובץ זמני עם מדדי מהירות וזמן משוער, אימות SHA-256 בזמן ההעברה ובדיקת כותרת PE וגודל הקובץ לפני הפעלה. | en: Stream downloads to a temporary file with live speed and ETA, streaming SHA-256 verification, and PE header and file-size checks before execution.
- [local-installer-discovery] icon: hard-drive | badge: עברית: איתור מתקינים, en: Installer Discovery
  - Title: עברית: איתור מתקינים בכוננים ותיקיות מקומיות | en: Local & External Installer Discovery
  - Description: עברית: סריקה היוריסטית של כוננים חיצוניים ותיקיות נפוצות, השוואת SemVer והמלצה על המתקין העדכני ביותר. | en: Heuristic scanning of external drives and common folders, SemVer comparison, and recommendation of the newest installer.
- [document-init-fix] icon: file-check | badge: עברית: תיקון אתחול, en: Initialization Fix
  - Title: עברית: תיקון יצירת מסמכים וייבוא Word | en: Fixed Document Creation & Word Import
  - Description: עברית: תיקון סדר יצירת ContextualInspector והפעלת setFlows לאחר האתחול, למניעת מסמכים ריקים. | en: Corrected ContextualInspector initialization order and moved setFlows after initialization to prevent blank documents.

### Changes
- [improvements] עברית: מנגנון עדכון מובנה עם הורדה ישירה ללא פתיחת דפדפן חיצוני, כתיבה אטומית לקובץ ‎.part, מדדי מהירות ו-ETA ואימות SHA-256 בזמן הורדה | en: Browserless in-app updater with direct streaming download, atomic .part file handling, live speed/ETA, and streaming SHA-256 verification
- [security] עברית: אימות כותרת PE ‏(MZ), גודל מינימלי ותקינות קובץ לפני הפעלת המתקין | en: Validate PE (MZ) header, minimum file size, and installer integrity before execution
- [features] עברית: סריקה עמוקה של כוננים חיצוניים ותיקיות מתקינים נפוצות, השוואת SemVer והמלצה על גרסה חדשה | en: Deep external-drive and common installer-folder discovery with SemVer comparison and newest-candidate recommendation
- [improvements] עברית: הפעלה מנותקת של המתקין וסגירה מסודרת של התוכנה לשחרור נעילות קבצים | en: Detached installer execution and clean application shutdown to release binary locks
- [tests] עברית: הוספת 11 בדיקות יחידה ורגרסיה למנוע העדכון, אימות PE, SHA-256 וגישור IPC | en: Added 11 unit and regression tests for updater engine, PE validation, SHA-256, and IPC bridge contracts
- [fixes] עברית: תיקון אתחול ContextualInspector והעברת setFlows לאחר יצירת inspector, לתיקון יצירת מסמכים וייבוא DOCX ב-Issue #28 | en: Fix ContextualInspector initialization order and call setFlows after inspector creation, resolving document creation and DOCX import failures in Issue #28

---

## [0.35.1] - 2026-11-08
- **Importance**: minor
- **Badge**: עברית: גרסה נוכחית | en: Current Version
- **Title**: עברית: מנוע מספור אוטומטי של כותרות תורניות, סימנים וסעיפים וחיבור IPC מלא | en: Automatic Torah Heading & Simanim Numbering Engine with Full IPC Integration
- **Summary**: עברית: השלמת מנוע המספור האוטומטי לכותרות תורניות: חיבור ישיר בין סכמות המספור של הליבה (סימן, סעיף, אות, דף ועמוד) לעץ הכותרות (DocumentHierarchyNode ו-ParagraphNode), עדכון דינמי בזמן אמת של מספור הסימנים והסעיפים בעת שינוי סדר כותרות, הוספה או מחיקה, תמיכה מלאה בגימטריה מסורתית מול אותיות נקיות, וחשיפת שליטה מלאה ב-IPC לממשק המשתמש. | en: Completion of automatic Torah heading numbering engine: direct integration between core numbering schemes (Siman, Seif, Ot, Daf/Amud) and document hierarchy tree (DocumentHierarchyNode and ParagraphNode), real-time dynamic renumbering upon reordering, inserting, or deleting headings, full support for standard punctuation vs plain gematria letters, and complete IPC bridge exposure for desktop UI.

### Highlights
- [torah-auto-numbering] icon: layers | badge: עברית: מספור תורני, en: Torah Numbering
  - Title: עברית: מספור אוטומטי דינמי של סימנים, סעיפים ואותיות | en: Dynamic Automatic Numbering for Simanim, Se'ifim & Otot
  - Description: עברית: שילוב ישיר בין סכמות המספור בליבת Rust לבין עץ הכותרות והפסקאות (DocumentHierarchyNode ו-ParagraphNode) עם עדכון מיידי של כל הסימנים והסעיפים. | en: Direct integration between Rust core numbering schemes and document hierarchy/paragraph tree with instant renumbering across all subdivisions.
- [heading-reordering] icon: check | badge: עברית: שינוי סדר חי, en: Live Reordering
  - Title: עברית: עדכון דינמי בזמן אמת בשינוי סדר, הוספה ומחיקה | en: Real-Time Dynamic Renumbering on Reorder, Insert & Delete
  - Description: עברית: שינוי סדר כותרות, הוספת סעיף ביניים או מחיקה מעדכנים מיידית את רצף המספור ללא צורך בעריכה ידנית, כולל מניעת לולאות היררכיות. | en: Moving headings, inserting intermediate clauses, or deleting nodes updates numbering sequence immediately without manual edits, with cycle prevention.
- [ipc-full-control] icon: code | badge: עברית: שליטת IPC, en: Full IPC Control
  - Title: עברית: חשיפת שליטה מלאה ב-IPC ובממשק המשתמש | en: Comprehensive IPC Control & Bridge for Desktop UI
  - Description: עברית: פקודות ואירועי IPC ייעודיים (ConfigureHeadingNumbering, ReorderHeadingNode, InsertHeadingNode, DeleteHeadingNode, RecalculateHeadingNumbers) מחוברים ל-Electron ול-UI. | en: Dedicated IPC commands and events exposed in Rust core, framed binary IPC, Electron main/preload bridge, and UI client bridge.
- [docx-styles-and-links] icon: file-text | badge: עברית: סגנונות וקישורים ב-DOCX, en: DOCX Styles & Links
  - Title: עברית: מיפוי סגנונות מותאמים אישית מ-word/styles.xml ושדות קישור w:fldSimple | en: Custom Styles Mapping from word/styles.xml & Field Links (w:fldSimple)
  - Description: עברית: חילוץ מלא של סגנונות פסקאות ותווים כולל הורשה מרובת רמות (basedOn), הזחות, יישור, גופנים וצבעים, לצד פענוח היפר-קישורים, עוגנים והפניות צולבות. | en: Full extraction of custom paragraph and character styles with basedOn inheritance, indents, alignments, and fonts, plus hyperlink, anchor and cross-reference parsing.
- [forced-odd-recto-blank] icon: book-open | badge: עברית: פתיחת ימין כפויה, en: Forced Recto & Blank Verso
  - Title: עברית: פתיחת פרק בעמוד ימין (Recto) והזרקת עמוד חלק אוטומטית | en: Forced Odd-Page (Recto) Section Opening & Automatic Blank Verso Page Injection
  - Description: עברית: כפיית פתיחת סימנים בעמוד אי-זוגי ימני עם הזרקה אוטומטית של עמוד שמאל חלק מסונכרן (is_blank) ללא עמודות תוכן או כותרות עליונות. | en: Enforced odd-page chapter starts with synchronized blank verso page injection suppressing frames and running headers.

### Changes
- [features] עברית: מיפוי סגנונות מותאמים אישית מתוך word/styles.xml כולל מודל הורשת סגנונות (basedOn), מאפייני פסקאות ותווים וייצוא מלא ב-ImportResult | en: DOCX custom styles extraction and inheritance from word/styles.xml with paragraph and character formatting
- [features] עברית: פענוח שדות קישור w:fldSimple, אלמנטי w:hyperlink ושדות מורכבים (HYPERLINK, REF, PAGEREF, PAGE, NUMPAGES) והחלת סגנון קישור | en: Parse w:fldSimple, w:hyperlink, and complex fields with hyperlink styling and URL targeting
- [features] עברית: כפיית פתיחת פרקים וסימנים בעמוד אי-זוגי (Recto) והזרקת עמוד שמאל חלק (Blank Verso Page) אוטומטית במנוע העימוד ובתצוגת ה-UI | en: Enforced odd-page section openings with automatic blank verso page injection in typeset engine and UI
- [features] עברית: טיפוגרפיה מסורתית, ניקוד וטעמים: תמיכה מלאה באותיות זעירות, רבתי ותלויות בשני מסלולים (סגנון תו CharacterStyle עם יחס גודל גופן והזחת קו בסיס, לצד הפעלת תכונות OpenType ייעודיות cvXX/ssXX/titl/smcp/sups בגופנים תורניים) | en: Traditional typography: Literae Majusculae, Minusculae, and Suspensae via CharacterStyle scaling & baseline shift and OpenType features (titl/smcp/sups/cvXX/ssXX)
- [features] עברית: תמיכה ורינדור תקני של נו"ן הפוכה (U+05C6 Hebrew Punctuation Nun Hafukha) במנוע העימוד rustybuzz, מניעת קריסת רוחב ושימור בטבלת /ToUnicode ב-PDF | en: Compliant rendering and shaping of Inverted Nun (U+05C6) via rustybuzz, non-zero advance preservation, and PDF /ToUnicode mapping
- [features] עברית: העברת תגי OpenType ייעודיים (calt, ss01, ss04, ss02) ל-rustybuzz עבור טעמי מקרא נדירים (שלשלת, קרני פרה, ירח בן יומו, מירכא כפולה, תלישא גדולה) וטעמי אמ"ת בספרי השירה | en: Automatic routing of OpenType features (calt, ss01, ss04, ss02) to rustybuzz for rare cantillation marks and poetic Sifrei Emet accents
- [features] עברית: חיבור מלא בין סכמות מספור תורניות (TorahSiman, TorahSeif, TorahOt) לעץ הכותרות DocumentHierarchyNode ולפסקאות ParagraphNode | en: Full connection between Torah numbering schemes and DocumentHierarchyNode/ParagraphNode tree
- [features] עברית: עדכון דינמי בזמן אמת של מספור הסימנים והסעיפים בעת שינוי סדר כותרות (Heading Reordering), הוספה או מחיקה | en: Real-time dynamic renumbering of Simanim and Se'ifim upon heading reordering, insertion, or deletion
- [features] עברית: חשיפת פקודות ואירועי IPC ייעודיים ב-tok-ipc ובגשר Electron לשליטה מלאה בממשק המשתמש | en: Dedicated IPC commands and events in tok-ipc and Electron bridge for full UI control
- [improvements] עברית: תמיכה בסנכרון מספור אוטומטי ישירות דרך כלי ה-CLI tok-cli sync-heading-numbers | en: CLI support for heading numbering synchronization via tok-cli sync-heading-numbers

---

## [0.35.0] - 2026-11-07
- **Importance**: major
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: ייבוא DOCX מתקדם (הערות שוליים, טבלאות ורשימות), ריבוי מדורי הערות במקביל, ניהול צבע ICC/PDF-X ומנוע GREP חי | en: Advanced DOCX Import, Multi-Apparatus Footnotes, ICC Color Management & Live GREP Engine
- **Summary**: עברית: שחרור גרסה 0.35.0 המשלים ארבע יכולות ליבה לעימוד ספרי קודש וספרות תורנית מקצועית: ייבוא מלא מקובצי Word כולל חילוץ הערות שוליים וקישורן לטקסט הראשי, טבלאות RTL ורשימות ממוספרות מקוננות; מערכת הערות שוליים רב-מדורית במקביל (כגון מדור מקורות ב-2 טורים ומדור ביאורים בטור רחב) עם איזון גובה דינמי וקווי הפרדה; מנוע ניהול צבע קדם-דפוס ופרופילי ICC (FOGRA39, FOGRA51, GRACoL, SWOP) עם הגבלת עומס דיו (TAC <= 300%) והטמעת OutputIntent תקני ב-PDF/X-1a ו-PDF/X-4; ומנוע חוקי GREP חי לעיצוב אוטומטי בזמן אמת ללא שינוי טקסט המקור. | en: Release 0.35.0 delivering four core capabilities: full DOCX import extracting footnotes, nested RTL tables, and numbered lists; parallel multi-apparatus footnotes with independent columns, numbering, proportional height budgeting, and separator rules; pre-press ICC color management (FOGRA39, FOGRA51, GRACoL, SWOP) with GCR/UCR TAC <= 300% and embedded PDF/X OutputIntent profiles; and a non-destructive Live GREP regex styling engine.

### Highlights
- [docx-advanced-import] icon: file | badge: עברית: ייבוא וורד מלא, en: Full DOCX Import
  - Title: עברית: חילוץ הערות שוליים, טבלאות ורשימות ממוספרות מ-Word | en: DOCX Footnotes, Nested Tables & Numbered Lists Import
  - Description: עברית: קריאה ישירה של word/footnotes.xml, word/numbering.xml וטבלאות מקוננות מקובץ ה-DOCX ושיבוצם האוטומטי בתזרימי המסמך עם עוגנים מקושרים. | en: Direct extraction of word/footnotes.xml, word/numbering.xml, and tables into linked document flows.
- [multi-apparatus-footnotes] icon: layers | badge: עברית: רב-מדורי, en: Multi-Apparatus
  - Title: עברית: ריבוי מערכות הערות שוליים עצמאיות במקביל | en: Parallel Multi-Apparatus Footnote Channels
  - Description: עברית: תמיכה במדורי הערות מקבילים בתחתית העמוד (מדור מקורות באותיות ובשני טורים לצד מדור ביאורים במספרים ובטור אחד) עם חלוקת תקציב גובה וקווי הפרדה. | en: Independent bottom footnote apparatuses with custom columns, numbering schemes, dynamic height budgeting, and separator rules.
- [icc-color-pdfx] icon: check | badge: עברית: קדם-דפוס ICC, en: ICC & PDF/X
  - Title: עברית: ניהול צבע מקצועי ופרופילי ICC מוטמעים ב-PDF/X | en: Professional ICC Color Management & PDF/X OutputIntent
  - Description: עברית: המרת RGB ל-CMYK בשיטת GCR/UCR עם הגבלת דיו ל-300%, שמירת טקסט שחור 100% K והטמעת פרופיל DestOutputProfile תקני (FOGRA39/51, GRACoL, SWOP). | en: GCR/UCR RGB-to-CMYK separation with 300% total ink limit, pure 100% K black text, and embedded ICC OutputIntent streams.
- [live-grep-engine] icon: sparkle | badge: עברית: מנוע GREP חי, en: Live GREP Engine
  - Title: עברית: מנוע ביטויים רגולריים (GREP) חי לעיצוב אוטומטי | en: Non-Destructive Live GREP Styling Engine
  - Description: עברית: החלת סגנונות תו אוטומטית בזמן אמת (הקטנת מראי מקומות בסוגריים ל-80%, הדגשת ראשי תיבות ומילות פתיחה בגמרא) מבלי לשנות את טקסט המקור. | en: Real-time non-destructive regex styling for parenthetical citations, acronyms, and opening words.

### Changes
- [features] עברית: חילוץ אוטומטי של הערות שוליים (word/footnotes.xml), טבלאות מקוננות ורשימות ממוספרות (word/numbering.xml) בייבוא קובצי DOCX | en: Extract footnotes, nested tables, and hierarchical numbered lists from DOCX archives
- [features] עברית: תמיכה בריבוי מדורי הערות שוליים עצמאיים במקביל עם מספור, כמות טורים, גופנים וקווי הפרדה ייחודיים לכל מדור | en: Support multiple parallel footnote apparatuses with per-apparatus columns, numbering, typography, and separator rules
- [features] עברית: מנוע ניהול צבע קדם-דפוס בליבת ה-Rust התומך בפרופילי FOGRA39, FOGRA51, ISO Coated v2, GRACoL 2013 ו-SWOP עם הטמעת DestOutputProfile ב-PDF/X-1a ו-PDF/X-4 | en: Rust pre-press color management engine with GCR/UCR separation, 300% TAC enforcement, and embedded ICC profiles in PDF/X
- [features] עברית: מנוע חוקי GREP חי בליבת ה-Rust ובממשק המשתמש עם תבניות מוכנות לעימוד תורני ויכולת הוספת כללי Regex מותאמים אישית | en: Live non-destructive GREP regex rule engine in Rust and UI inspector with built-in Torah typesetting presets

---

## [0.34.11] - 2026-11-07
- **Importance**: patch
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: יישור קנבס בתצוגה משולבת, פס פקודות מורחב, שמירה בתיקיית פרויקטים, שימור שינוי שם והסבר שכבות | en: Split View Canvas Centering, Comfortable Top Bar, Projects Folder Autosave, Persistent Renaming & Layer Guides
- **Summary**: עברית: שחרור גרסה 0.34.11 המשפר את נוחות העבודה ואמינות הנתונים: יישור ממורכז ואוטומטי של עמודי הקנבס במעבר לתצוגה משולבת (Split View) במקום בריחה שמאלה, הרחבת פס הפקודות העליון ל-50 פיקסלים בסטנדרט שולחני מרווח ונוח, מנגנון שמירה אוטומטי מלא של מסמכים בתיקיית הפרויקטים של התוכנה, שימור וסנכרון מלא של שינוי שם מסמך מול הקובץ הפיזי בדיסק, הוספת הסבר ברור ומצב חי בטאב השכבות, ותיקון כפילות כותרת השמירה האוטומטית בהגדרות. | en: Release 0.34.11 improving workflow comfort and data persistence: automatic horizontal centering of canvas pages when entering split view preventing off-screen drift, comfortable 50px top command bar, continuous document auto-saving directly into the application Projects directory, persistent renaming synced to disk packages, clear layer explanations with live statuses, and Settings modal duplicate title fix.

### Highlights
- [split-view-center] icon: split | badge: עברית: תצוגה משולבת, en: Split View
  - Title: עברית: מרכוז עמוד אוטומטי בתצוגה משולבת | en: Automatic Page Centering in Split View
  - Description: עברית: במעבר לתצוגה משולבת או שינוי גודל המחיצה, העמוד מתיישר אוטומטית למרכז שטח הקנבס שנותר לו ללא בריחה שמאלה מחוץ לחלון. | en: When switching to split view or adjusting divider, pages automatically center in remaining viewport instead of drifting off-screen.
- [topbar-comfort] icon: canvas | badge: עברית: פס עליון, en: Top Command Bar
  - Title: עברית: פס פקודות עליון מורחב, נוח ונגיש | en: Spacious & Comfortable Top Command Bar
  - Description: עברית: שורת הפקודות העליונה הורחבה לגובה 50 פיקסלים בסטנדרט שולחני מודרני עם כפתורים, שדות חיפוש וכותרת נוחים ללחיצה. | en: Top command bar heightened to 50px conforming to spacious desktop UI with comfortable hit targets.
- [project-autosave] icon: folder | badge: עברית: תיקיית פרויקטים, en: Projects Directory
  - Title: עברית: שמירה אוטומטית רציפה בתיקיית הפרויקטים | en: Continuous Autosave to Dedicated Projects Directory
  - Description: עברית: כל מסמך חדש נשמר אוטומטית ברקע בתיקיית הפרויקטים של התוכנה (גם ללא צורך ב-Ctrl+Shift+S) ומעודכן ברשימת הפרויקטים האחרונים. | en: Continuous autosave automatically writes new documents into the dedicated Projects folder and updates recent projects list.
- [document-rename] icon: file | badge: עברית: שינוי שם, en: Rename Document
  - Title: עברית: שימור מלא של שינוי שם מסמך | en: Persistent Document Renaming
  - Description: עברית: שינוי שם מסמך (בלחיצה כפולה או F2) נשמר לצמיתות, מעדכן את שם הקובץ בדיסק, ואינו חוזר לשם הישן בעת ביצוע פעולה או שמירה. | en: Renaming document titles updates the underlying physical file and remains permanently without reverting on subsequent actions.

### Changes
- [fixes] עברית: מרכוז אוטומטי מלא של עמודי הקנבס בעת מעבר לתצוגה משולבת (Split View) ובגרירת מחיצת החלוקה | en: Automatic horizontal canvas centering when switching to split view and resizing layout divider
- [improvements] עברית: הרחבת גובה פס הפקודות העליון ל-50px ועיצוב מרווח ונוח לכל הפקדים | en: Expand top system bar height to 50px providing comfortable and spacious desktop controls
- [features] עברית: שמירה אוטומטית מלאה של מסמכים בתיקיית הפרויקטים של התוכנה וזיהוי נתיב פרויקטים מובנה | en: Integrated projects directory detection and continuous automatic saving of active documents
- [fixes] עברית: שימור קבוע של שם המסמך בעת עריכה וסנכרון שינוי השם עם הקובץ בדיסק | en: Fix document rename reversion bug and sync rename with disk storage
- [improvements] עברית: הוספת כרטיס הסבר מפורט ומצב חי (מוצג/נעול) לכל פריט במסך השכבות בסרגל הימני | en: Add descriptive guide card and live state indicators in the right sidebar Layers panel
- [fixes] עברית: תיקון כפילות כותרת "שמירה אוטומטית של מסמך" במסך ההגדרות במראה | en: Remove duplicate autosave section header in Settings modal appearance tab

---

## [0.34.10] - 2026-11-06
- **Importance**: patch
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: פתיחה ישירה למסך הבית, פס עליון מעודכן, עיצוב סרגל שמאלי משודרג ומתקין חכם | en: Direct Home Screen Launch, Standard System Titlebar, Refined Left Sidebar & Smart Installer
- **Summary**: עברית: שחרור גרסה 0.34.10 המשפר את חוויית השימוש: פתיחה ישירה ומיידית למסך הבית ללא הבהוב או פתיחת פרויקט מקדים, הרחבת הפס העליון וכפתורי החלון לסטנדרט תוכנות שולחניות (38px), העברת "מה חדש" בלעדית לסרגל השמאלי עם מנגנון הרחבה וכיווץ מעוצב ואלגנטי, ושדרוג מלא של המתקין: רישיון לקריאה בלבד ללא אפשרות הקלדה, שיוך בלעדי לקובצי tok והסרת tokbook, וזיהוי אוטומטי של התקנה קיימת לשדרוג מהיר ללא שלבים מיותרים. | en: Release 0.34.10 improving core user experience: instant launch directly to the Home screen without flickering or opening prior projects, widened system titlebar and window buttons (38px), exclusive relocation of What's New into the left sidebar with a sleek expand/collapse rail, and comprehensive installer upgrades including read-only license display, unconditional .tok association, and smart update detection.

### Highlights
- [home-screen-direct] icon: brand | badge: עברית: פתיחה ישירה, en: Direct Launch
  - Title: עברית: פתיחה ישירה וחלקה למסך הבית | en: Direct & Instant Home Screen Launch
  - Description: עברית: התוכנה נפתחת ישירות למסך הבית ללא הבהוב מקדים של פרויקט ריק, עם טעינה מיידית וללא פתיחה שגויה של פרויקטים ישנים. | en: The application opens directly to the Home screen without project flashing, pre-rendering dummy documents, or unexpected document loads.
- [standard-titlebar] icon: maximize | badge: עברית: פס עליון מורחב, en: Standard Titlebar
  - Title: עברית: פס עליון וכפתורי חלון בסטנדרט שולחני | en: Desktop Standard Titlebar & Window Controls
  - Description: עברית: הגבהת הפס העליון ל-38 פיקסלים וכפתורי איקס ומיזעור רחבים ונוחים (48 פיקסלים) התואמים את הסטנדרט המקובל בתוכנות שולחניות. | en: Heightened system titlebar to 38px and broadened window buttons to 48px matching native desktop application conventions.
- [left-sidebar-whatsnew] icon: sparkle | badge: עברית: סרגל שמאלי, en: Left Sidebar
  - Title: עברית: עיצוב מלוטש לסרגל השמאלי והעברת "מה חדש" | en: Refined Left Sidebar & Dedicated What's New Integration
  - Description: עברית: מנגנון הרחבה וכיווץ מעוצב, נקי ואלגנטי ללא טקסט אנכי מעוות, עם שילוב כפתור "מה חדש" ונקודת התראה חיה בלעדית בסרגל השמאלי. | en: Elegant and sleek sidebar expand/collapse rail with live unread update indicator badge, exclusive to the left properties sidebar.
- [installer-upgrades] icon: layers | badge: עברית: שידרוג מתקין, en: Modern Installer
  - Title: עברית: מתקין שולחני משודרג וזיהוי גרסה קיימת | en: Upgraded Desktop Installer with Update Detection
  - Description: עברית: רישיון מוגן מפני הקלדה, שיוך אוטומטי מוחלט לקובצי tok (ללא tokbook), וזיהוי אוטומטי של גרסה ישנה לקיצור תהליך השדרוג. | en: Read-only license memo, unconditional native .tok file association, and smart previous version detection for accelerated installation flow.

### Changes
- [fixes] עברית: ביטול טעינת פרויקט זמני בעת הפעלת התוכנה ופתיחה ישירה של מסך הבית ללא הבהוב | en: Eliminate pre-rendered dummy document on startup and launch directly to Home screen
- [improvements] עברית: הרחבת הפס העליון ל-38px וכפתורי מזער/הגדל/סגור ל-48px בסטנדרט שולחני מלא | en: Widen titlebar to 38px and caption buttons to 48px conforming to standard desktop OS conventions
- [improvements] עברית: העברת כפתור "מה חדש" עם נקודת התראה חיה מהפס העליון לסרגל השמאלי בלבד | en: Move What's New trigger with live unread dot from top system bar to left sidebar exclusively
- [improvements] עברית: עיצוב מחדש של מצב כיווץ/הרחבה בסרגל השמאלי עם פס כלים קומפקטי ונקי | en: Redesign left sidebar collapse/expand rail with compact icon controls and clean layout
- [fixes] עברית: נעילת תיבת הרישיון במתקין לקריאה בלבד ללא אפשרות הקלדה או שינוי טקסט | en: Lock license agreement memo in installer to read-only preventing keyboard input
- [improvements] עברית: שיוך אוטומטי מלא לקובצי .tok במתקין והסרה מלאה של .tokbook | en: Unconditional automatic association for .tok files and complete removal of .tokbook
- [features] עברית: זיהוי אוטומטי של התקנות קודמות במתקין ודילוג על שלבי קיצור הדרך לשדרוג מהיר | en: Automatic previous installation detection skipping shortcut prompts for streamlined upgrades
- [fixes] עברית: הסרת כפתורי ייבוא ופתיחה כפולים מראש סרגל הפרויקטים האחרונים במסך הבית | en: Remove redundant import and open buttons from recent projects sidebar header
- [fixes] עברית: הסרת כפתור בדיקת עדכונים מתחתית מגירת מה חדש בהתאם לסטנדרט התוכנה | en: Remove check for updates button from What's New drawer footer
- [improvements] עברית: אינדיקטור עדכונים כחול עם מונה מספרי של שינויים שלא נקראו על כפתור העדכונים בסרגל הצד | en: Blue update badge displaying unread changelog count on sidebar What's New button
- [features] עברית: חיבור חי של קווי עזר לשולי העמוד (Top, Bottom, Inside, Outside) לקנבס וחסימת תווים שאינם ספרות בשדות מספריים | en: Real-time dynamic page margin guides reflecting inspector settings and numeric-only input restriction

---

## [0.34.9] - 2026-11-06
- **Importance**: minor
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: טיפוגרפיה מסורתית מקראית, מערך הערות רב-שכבתי, יחסי תזרימים דינמיים וסנכרון עיצוב חי | en: Biblical Masoretic Typography, Multi-Apparatus Footnotes, Dynamic Rabbinic Flow Ratios & Live Styling Pipeline
- **Summary**: עברית: שחרור גרסה 0.34.9 המשלב תמיכה טיפוגרפית מלאה בטקסט מקראי ומסורתי (נון הפוכה, פסיק, מתג, טעמי המקרא וגליפים מקראיים ב-OpenType), פריסת הערות רב-שכבתית מקבילה (מספר מנגנוני הערות ופירושים נפרדים בתחתית העמוד), יחסי תזרימים גמישים לעימוד תלמודי ורבני רב-טורי, שימור עיצובים וטלאי טיפוגרפיה בעורך הרציף, וסנכרון מלא של צינור הסגנונות בין סרגל המאפיינים, סרגל הכלים והקנבס. | en: Release 0.34.9 delivering complete Masoretic biblical typography (Nun Hafukha, Paseq, Meteg, cantillation marks and biblical OpenType shaping variants), parallel multi-apparatus footnote layouts, flexible multi-flow column width proportions for Talmudic and rabbinic spreads, persistent typography patches in story DOM, and unified live styling pipeline across inspector, toolbar, and canvas.

### Highlights
- [masoretic-typography] icon: typography | badge: עברית: טיפוגרפיה מקראית, en: Masoretic Typography
  - Title: עברית: טיפוגרפיה מקראית ומסורתית מתקדמת | en: Advanced Biblical Masoretic Typography
  - Description: עברית: עיצוב מדויק של תווי מסורה עבריים כולל נון הפוכה, פסיק, מתג עליון ותחתון, כללי הצמדת טעמים והחלת תכונות OpenType ייעודיות לטקסט מקראי. | en: Native biblical Hebrew shaping support including Nun Hafukha, Paseq, upper/lower Meteg, cantillation anchoring, and biblical OpenType font features.
- [multi-apparatus] icon: layers | badge: עברית: מערך הערות מקבילי, en: Multi-Apparatus
  - Title: עברית: מערך הערות שוליים רב-שכבתי ומקבילי | en: Parallel Multi-Apparatus Footnote Bands
  - Description: עברית: תמיכה במגוון מנגנוני הערות מקבילים על אותו עמוד (הערות שוליים, ביאורים ומראי מקומות) עם סימונים עצמאיים והקצאת גובה מאוזנת. | en: Multiple simultaneous parallel footnote bands on the same page (notes, glosses, critical apparatus) with distinct numbering schemes and balanced height allocation.
- [rabbinic-flow-ratios] icon: flows | badge: עברית: יחסי תזרימים, en: Flow Ratios
  - Title: עברית: יחסי רוחב דינמיים לתזרימים רבניים | en: Dynamic Flow Width Ratios for Rabbinic Layouts
  - Description: עברית: שליטה מדויקת ביחסי רוחב עמודות בעימוד ש"ס ומקראות גדולות (2, 3 ו-4 תזרימים) תוך שמירה מלאה על עקרון שדרה-שוליים recto/verso. | en: Precise column width ratio distribution for 2, 3, and 4 stream Talmudic and Mikraot Gedolot spreads with full recto/verso spine-margin symmetry.
- [live-styling-pipeline] icon: sliders | badge: עברית: סנכרון סגנונות חי, en: Live Styling
  - Title: עברית: סנכרון סגנונות וטיפוגרפיה חי בין הרכיבים | en: Real-Time Styling Pipeline Synchronization
  - Description: עברית: חיבור ישיר ורציף בין עריכת סגנונות בסרגל המאפיינים, פקדי הטיפוגרפיה בסרגל העליון, והקרנת ה-DOM בקנבס עם שימור טלאי עיצוב ב-Story DOM. | en: Continuous bidirectional synchronization between inspector properties, toolbar typography controls, and canvas DOM projection with lossless patch preservation.

### Changes
- [features] עברית: תמיכה טיפוגרפית מקיפה בטעמי המקרא, פסיק, מתג ונון הפוכה במנועי העיצוב והקדם-דפוס | en: Comprehensive biblical typography with cantillation marks, paseq, meteg, and nun hafukha in shaping and pre-press pipelines
- [features] עברית: פריסת הערות רב-שכבתית מקבילה (Multi-Apparatus) עם הגדרות גובה, קווי הפרדה ומספור עצמאי | en: Parallel multi-apparatus footnote layout supporting distinct apparatus configurations, custom separators, and independent numbering
- [features] עברית: חלוקת רוחב עמודות פרופורציונלית מותאמת אישית בתבניות 2, 3 ו-4 תזרימים (ש"ס ומקראות גדולות) | en: Proportional column width distribution in 2, 3, and 4 stream layouts with custom width ratios and recto/verso spine tracking
- [improvements] עברית: שימור טלאי טיפוגרפיה ועיצוב מקומי בעת המרת DOM בעורך הסיפור הרציף (Story DOM) | en: Preservation of typography patches and inline span styling during bidirectional DOM conversion in story editor
- [improvements] עברית: סנכרון חי של שינויי פונט, גודל, ריווח ויישור ישירות מקנבס העריכה ומסרגל המאפיינים | en: Real-time synchronization of font family, size, leading, tracking, and alignment directly from editor canvas and inspector
- [fixes] עברית: תיקון דרישות עיצוב קוד ב-CI ב-Rustfmt ושמירה על יציבות מלאה של צינור הבדיקות האוטומטי | en: Fix Rustfmt formatting compliance across crates/tok-typeset for CI stability

---

## [0.34.4] - 2026-11-06
- **Importance**: minor
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: הזזה חופשית ונעילת תיבות בעורך, תזרימים מותאמים אישית (טורים מפוצלים), סינון סגנונות ובחירת תבנית מודרכת | en: Free Frame Movement & Locking, Custom Flows with Split Edge Layout, Active Flow Style Filtering & Guided Template Creation
- **Summary**: עברית: שחרור גרסה 0.34.4 המוסיף שליטה ישירה ואינטואיטיבית בתיבות טקסט על גבי הקנבס כולל גרירה חופשית ונעילת מיקום, אפשרות הוספת תזרימים מותאמים אישית עם תמיכה בבלוקים מפוצלים בקצוות הזורמים לפי גובה, סינון סגנונות ממוקד לפי התבנית הפעילה, ומסלול פתיחת מסמך חדש מודרך עם בחירת תבנית מיידית והמלצה על ספר רציף. | en: Release 0.34.4 introducing direct canvas text frame dragging and location locking, custom text flow creation with split edge dual-margin block streaming by height, template-specific style filtering in structure bar, and guided new document creation with immediate template selection defaulting to continuous prose.

### Highlights
- [frame-drag-lock] icon: layers | badge: עברית: הזזה ונעילה, en: Drag & Lock
  - Title: עברית: הזזה חופשית ונעילה ישירה על הקנבס | en: Direct Canvas Frame Dragging & Locking
  - Description: עברית: אפשרות גרירה ושינוי גודל ישירים של תיבות טקסט במרחב העבודה עם מנגנון נעילה להגנה מפני תזוזה מקרית. | en: Freehand dragging and resizing of text frames directly on the editor canvas with instant locking guards.
- [custom-flows] icon: flows | badge: עברית: תזרימים מותאמים, en: Custom Flows
  - Title: עברית: הוספת תזרימים ובלוקים מפוצלים בקצוות | en: Custom Text Flows & Split Edge Layout
  - Description: עברית: כפתור ייעודי קומפקטי וחלונית עיצוב להוספת תזרימי טקסט חדשים, כולל חצי בלוק בקצה אחד וחצי בקצה שני ללא חיבור ביניהם הזורמים לפי גובה. | en: Dedicated compact button and configuration modal for custom flows, featuring dual split edge margin blocks streaming text by height.
- [template-onboarding] icon: templateBook | badge: עברית: בחירת תבנית, en: Guided Templates
  - Title: עברית: בחירת תבנית מודרכת וברירת מחדל ספר רציף | en: Guided Template Picker & Prose Default
  - Description: עברית: פתיחת מסמך חדש מציגה ישירות את חלון בחירת התבנית המעוצב עם המלצה ברורה על תבנית ספר רציף. | en: New document workflow prompts layout template chooser immediately, with continuous prose set as default recommendation.

### Changes
- [features] עברית: גרירה ושינוי גודל אינטראקטיביים של תיבות טקסט בעורך עם עדכון קואורדינטות חי בסרגל המאפיינים | en: Interactive text frame dragging and resizing directly on canvas with live coordinate synchronization in inspector
- [features] עברית: מנגנון נעילת תיבות טקסט (Lock/Unlock) מתגית התיבה ובסרגל המאפיינים למניעת שינויים לא רצויים | en: Text frame position and size locking toggle on frame tag and contextual inspector
- [features] עברית: הוספת תזרימי טקסט חדשים בכל תבנית ספר דרך כפתור קומפקטי וחלונית הגדרה ייעודית | en: Custom text flow creation across any layout template via a compact button and dedicated modal dialog
- [features] עברית: תמיכה במבנה בלוקים מפוצלים בקצוות (שמאל וימין ללא חיבור ביניהם עם זרימה לפי גובה) לתזרימים מותאמים | en: Split edge dual margin layout blocks flowing text by height without intermediate connecting elements
- [improvements] עברית: סינון סגנונות בסרגל הימני להצגת סגנונות רלוונטיים בלבד לתבנית ולתזרימים הפעילים במסמך | en: Dynamic style token filtering in structure bar displaying exclusively styles relevant to active template flows
- [improvements] עברית: יצירת מסמך חדש פותחת מיד את מסך בחירת התבנית, וברירת המחדל מוגדרת ל'ספר רציף' | en: New document creation presents template chooser immediately, defaulting to continuous prose layout

---

## [0.34.3] - 2026-11-06
- **Importance**: major
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: מרחב עבודה מודרני, שמירה אוטומטית מתמשכת, סרגלים מודולריים ומנוע טיפוגרפיה מורחב | en: Modern Workspace UI, Continuous Autosave, Modular Sidebars & Enhanced Typography
- **Summary**: עברית: שחרור רשמי של גרסה 0.34.3 הכולל עיצוב מחדש של מרחב העבודה עם פס כותרת דק (32px) ובקרות חלון מובנות, סרגלים מודולריים מתקפלים, מערכת שמירה אוטומטית מתמשכת ברקע ושחזור טיוטות עמיד, התאמת ניגודיות, סילוק פרויקטי דמו ואריזת שולחן עבודה מוקשחת. | en: Official release 0.34.3 delivering modern workspace redesign with slim 32px titlebar and native window controls, modular collapsible sidebars, continuous background autosave and resilient draft recovery, contrast harmonization, demo project cleanup, and hardened desktop packaging.

### Highlights
- [modern-workspace] icon: sparkle | badge: עברית: מרחב עבודה, en: Workspace
  - Title: עברית: מרחב עבודה נקי ורחב | en: Uncluttered Expansive Workspace
  - Description: עברית: הארכת סרגלי הצד עד לפס הכותרת הדק (32px) במקום הסרגל העליון הישן, ליצירת משטח עבודה רציף ללא הסחות דעת. | en: Full-height sidebars reaching the slim 32px titlebar for a continuous, distraction-free typesetting canvas.
- [collapsible-sidebars] icon: layers | badge: עברית: סרגלים מתקפלים, en: Collapsible Sidebars
  - Title: עברית: שליטה מודולרית בסרגלי המבנה והמאפיינים | en: Modular Structure and Inspector Controls
  - Description: עברית: כפתורי הרחבה וקיפול ייעודיים, כרטיסיות מופרדות ויזואלית בשינוי בהירות עדין, וכפתורי הגדרות ואודות נגישים בתחתית. | en: Dedicated expand/collapse toggles, visual brightness contrast segmentation, and accessible bottom settings/about buttons.
- [autosave-resilience] icon: check | badge: עברית: שמירה רציפה, en: Continuous Autosave
  - Title: עברית: שמירה אוטומטית מתמשכת ושחזור טיוטות | en: Continuous Autosave and Draft Recovery
  - Description: עברית: שמירה ברקע בכל עריכה, שחזור טיוטה מוגן לאחר סגירה פתאומית, וחסימת סגירת חלון עד לסיום כתיבת המסמך לדיסק. | en: Continuous background document saving, crash draft recovery, and window-close guards awaiting disk write completion.

### Changes
- [features] עברית: פס כותרת דק (32px) במרכז החלון עם בקרות מובנות (מזעור, הגדלה, סגירה) ועריכת שם פרויקט בלחיצה כפולה | en: Centered slim 32px titlebar with window controls and double-click rename
- [features] עברית: חלוקת סרגל המאפיינים השמאלי לכרטיסיות מופרדות בקו ובגוון בהיר יותר עם כפתור צמצום והרחבה | en: Left inspector partitioned into distinct cards with dividing lines, subtle brightness contrast, and collapse/expand toggle
- [features] עברית: כפתור מה חדש עם אייקון כוכב עדכונים ייעודי בצד שמאל למעלה ותצוגה תמציתית של יומן הגרסאות | en: What's New button with star icon at top-left and concise changelog summary presentation
- [features] עברית: סרגל ימני מתקפל בעיצוב מסילה ופאנל עם כפתור קיפול כיווני וכפתור אודות תחתון | en: Right structure bar with rail/panel layout, directional collapse button, and bottom about button
- [improvements] עברית: התאמת ניגודיות צבעים והפרדות קו בכל ערכות הנושא של התוכנה (כהה, בהיר וניגודיות גבוהה) | en: Harmonized contrast and line separators across all themes (dark, light, and high contrast)
- [fixes] עברית: הסרת פרויקטי דמו ממסך הבית, מניעת רישום פרויקטי דמה בהיסטוריה וטיהור רשומות ישנות מזיכרון התוכנה | en: Purged demo projects from home screen and storage, preventing demo entries in recent projects history
- [features] עברית: מנגנון שמירה אוטומטית מתמשכת ברקע (Autosave) ואינדיקטור שינויים שטרם נשמרו | en: Continuous background autosave mechanism with unsaved change indicators
- [features] עברית: שחזור טיוטות אוטומטי (Draft Recovery) במקרה של יציאה או סגירה בלתי צפויה | en: Automated draft recovery prompt on application startup after abnormal termination
- [improvements] עברית: הגנת סגירת חלון הממתינה להשלמת שמירה בדיסק למניעת איבוד מידע | en: Window close guard strictly awaiting pending save operations before closing
- [fixes] עברית: הבטחת בניית חבילת UI רעננה בתהליך אריזת ה-Desktop ותיעוד שגיאות קונסול מרנדרר | en: Ensured fresh UI bundle compilation during desktop packaging and added renderer console error logging

---

## [0.33.0] - 2026-10-31
- **Importance**: major
- **Badge**: עברית: גרסה קודמת | en: Previous Version
- **Title**: עברית: מהפכת "צורת הדף" חיה, עגינה רב-תזרימית והקשחת דטרמיניזם | en: Living Tzurat HaDaf, Dynamic Multi-Flow Anchoring & Pre-Press Hardening
- **Summary**: עברית: שחרור רשמי של גרסה 0.33.0 הכוללת עימוד רב-תזרימי חי לש"ס ומקראות גדולות עם עגינה דו-כיוונית, גלישת עמודים מסונכרנת בזמן אמת, איזון גבהי טורים משותף, הקשחת מנוע הדטרמיניזם ואוטומציית שחרור גרסאות מלאה. | en: Official release 0.33.0 introducing Living Tzurat HaDaf with bi-directional multi-flow anchoring, real-time synchronized pagination cascading, shared column height balancing, determinism engine hardening, and automated CI/CD release workflow.

### Highlights
- [living-tzurat-hadaf] icon: layers | badge: עברית: צורת הדף חיה, en: Living Tzurat HaDaf
  - Title: עברית: מנוע עגינה רב-תזרימי דו-כיווני | en: Bi-Directional Multi-Flow Anchor Cascade Engine
  - Description: עברית: קישור לוגי אוטומטי בין פסקאות הטקסט הראשי למפרשים (רש"י ותוספות), המאפשר גלישה והתרחבות דינמית בזמן אמת. | en: Strict logical binding between primary text segments and commentaries with dynamic cascading and absorption.
- [determinism-hardening] icon: sparkle | badge: עברית: דטרמיניזם מלא, en: Full Determinism
  - Title: עברית: הקשחת דטרמיניזם מוחלט בכל שלבי העימוד והייצוא | en: Absolute Pre-Press Output Determinism
  - Description: עברית: הבטחת שחזור ביט-אחר-ביט בייצוא PDF/X-1a ו-HTML בכל פלטפורמה, ללא תלות בסידורי גיבוב או מצבי זיכרון. | en: Guarantees bit-for-bit reproducible PDF/X-1a and HTML output across all platforms and execution environments.

### Changes
- [features] עברית: עגינה דו-כיוונית קנונית עם זיהוי דיבור המתחיל אוטומטי (ד"ה) עבור דפי גמרא ומקראות גדולות | en: Canonical bi-directional anchoring with automated Dibur HaMatchil (DH) cue recognition
- [features] עברית: גלישה מסונכרנת במעבר עמודים המונעת הופעת פירוש לפני העוגן המרכזי שלו (No Commentary Lead Invariant) | en: Synchronized multi-stream pagination strictly enforcing the no-commentary-lead invariant
- [typography] עברית: איזון גובה תחתון פעיל בעמודי תלמוד ומקראות גדולות (Shared Column Height Balancing) | en: Active shared column height balancing aligning column bottom baselines
- [typography] עברית: בקרת יתומים ואלמנות (Orphan/Widow Control) בחלוקת פירושים ארוכים במיוחד על פני עמודים | en: Strict orphan and widow control for oversized commentary paragraphs spanning page boundaries
- [typography] עברית: יישור אופטי של שוליים (Optical Margin Alignment) לסימני פיסוק וגליפיים בולטים בעברית ובכיווניות מעורבת | en: Professional Optical Margin Alignment for punctuation and overhanging glyphs with full RTL and Hebrew support
- [typography] עברית: איזון שורה אחרונה מאולץ באלגוריתם Knuth-Plass כולל אילוצי רוחב מינימלי ומיקום אופטימלי | en: Knuth-Plass constrained last-line balancing with minimum width constraints and optimal final-line placement
- [typography] עברית: מנוע פוליגונים כללי וטורים ברוחב משתנה (L-Shape & Variable Columns) עם חיתוך רצועות שמרני | en: Arbitrary polygonal text regions, variable column geometry, and conservative band measure-profile generation
- [typography] עברית: חלון מילת פתיחה עברי (Hebrew Drop Caps) עם מדידת תיבה תוחמת מדויקת של גליפים, ניקוד וטעמים | en: Hebrew Drop Caps with exact glyph outline bounding boxes and multi-line text-exclusion geometry
- [improvements] עברית: שיפור מנגנון ה-CI ואיחוד בדיקות הדטרמיניזם להבטחת שחרור אוטומטי חלק | en: Streamlined CI determinism verification and automated release pipeline trigger

---

## [0.31.0] - 2026-10-31
- **Importance**: major
- **Badge**: עברית: עדכון גדול | en: Major Release
- **Title**: עברית: עימוד רב-תזרימי חי ("צורת הדף" דינמית) וסנכרון עוגנים דו-כיווני | en: Dynamic Multi-Flow Anchoring ("Living Tzurat HaDaf") & Synchronized Pagination
- **Summary**: עברית: מימוש מלא של עימוד רב-תזרימי חי לתלמוד ומקראות גדולות: עגינה דו-כיוונית בין טקסט מרכזי לפירושים, גלישת עמודים מסונכרנת בזמן אמת, ואיזון גובה תחתון דינמי בכל הטורים. | en: Full end-to-end implementation of Living Tzurat HaDaf: bi-directional anchor linking between primary text and commentaries, real-time synchronized pagination across page boundaries, and dynamic shared column height balancing.

### Highlights
- [multi-flow-anchoring] icon: layers | badge: עברית: צורת הדף חיה, en: Living Tzurat HaDaf
  - Title: עברית: מנגנון עגינה דו-כיווני וסנכרון עמודים חי | en: Bi-Directional Multi-Flow Anchoring
  - Description: עברית: קישור לוגי הדוק בין פסקאות הגמרא/המקרא לפירושי רש"י ותוספות, המבטיח גלישה אוטומטית של פירושים לעמוד הבא כאשר הטקסט המרכזי גולש. | en: Strict logical binding between primary text segments and commentaries ensuring automatic cascading of commentaries when primary text reflows.
- [height-balancing] icon: align | badge: עברית: איזון טיפוגרפי, en: Typographic Balance
  - Title: עברית: איזון גובה תחתון משותף בעמודים | en: Shared Multi-Column Height Balancing
  - Description: עברית: יישור קו בסיס תחתון משותף לכל הטורים בעמוד באמצעות קפיצים אנכיים אלסטיים המפוזרים במרווחי שורות ופסקאות. | en: Harmonious bottom baseline alignment across parallel streams via distributed vertical springs across inter-line and inter-paragraph gaps.

### Changes
- [features] עברית: עגינה דו-כיוונית קנונית עם זיהוי דיבור המתחיל אוטומטי (ד"ה) עבור דפי גמרא ומקראות גדולות | en: Canonical bi-directional anchoring with automated Dibur HaMatchil (DH) cue recognition
- [features] עברית: גלישה מסונכרנת במעבר עמודים המונעת הופעת פירוש לפני העוגן המרכזי שלו (No Commentary Lead Invariant) | en: Synchronized multi-stream pagination strictly enforcing the no-commentary-lead invariant
- [typography] עברית: איזון גובה תחתון פעיל בעמודי תלמוד ומקראות גדולות (Shared Column Height Balancing) | en: Active shared column height balancing aligning column bottom baselines
- [typography] עברית: בקרת יתומים ואלמנות (Orphan/Widow Control) בחלוקת פירושים ארוכים במיוחד על פני עמודים | en: Strict orphan and widow control for oversized commentary paragraphs spanning page boundaries

---

## [0.30.4] - 2026-10-30
- **Importance**: minor
- **Title**: עברית: עדכון שיפורים, טיפוגרפיה ותאימות מקלדת | en: Enhancements, Hebrew Typography & Keyboard Usability
- **Summary**: עברית: עדכון זה מביא תמיכה מלאה במשקלי גופן (Regular/Bold) בסרגל המבנה ובקנבס, שיפור במקשי קיצור במקלדת עברית ותיקוני יציבות. | en: This update brings font weight support (Regular/Bold) across inspector and canvas, Hebrew keyboard shortcut compatibility, and stability fixes.

### Changes
- [typography] עברית: תמיכה מובנית במשקלי גופן (Bold 700 / Regular 400) בגופנים עבריים ושימורם בייצוא | en: Native font weight support (Bold 700 / Regular 400) for Hebrew fonts with faithful export retention
- [improvements] עברית: תאימות פלטת הפקודות (Ctrl+K) למקלדת עברית עם המקש "ל" ללא צורך בהחלפת שפה | en: Hebrew keyboard layout compatibility for Command Palette (Ctrl+K / Ctrl+ל)
- [fixes] עברית: מניעת השתלטות של מקש הסלאש ("/") בזמן הקלדה חופשית בעורך הטקסט | en: Prevented slash key ("/") from hijacking story editor typing
- [performance] עברית: ייעול מנגנון הרינדור של מסגרות הטקסט בקנבס בצפיפויות מסך גבוהות | en: Optimized canvas text frame rendering on high DPI displays

---

## [0.30.0] - 2026-10-15
- **Importance**: major
- **Badge**: עברית: עדכון גדול | en: Major Release
- **Title**: עברית: שחרור גרסה ראשית: מנוע עימוד רב-תזרימי ותקני דפוס מקצועיים | en: Major Release: Multi-Flow Layout Engine & Professional Pre-Press
- **Summary**: עברית: קפיצת מדרגה משמעותית לתוכנת TypesetOK עם מנוע Rust Native מהיר, יישור עברי תלת-שכבתי, ייצוא ISO PDF/X-1a ומגן שמות קדושים. | en: A massive milestone for TypesetOK featuring a Rust Native engine, 3-tier Hebrew justification, ISO PDF/X-1a pre-press export, and holy name protection.

### Highlights
- [rust-engine] icon: sparkle | badge: עברית: מהיר פי 10, en: 10x Faster
  - Title: עברית: מנוע עימוד Rust מקורי | en: Native Rust Layout Engine
  - Description: עברית: ליבת עימוד דטרמיניסטית וסופר-מהירה המבצעת שבירת שורות אופטימלית בשיטת Knuth-Plass על פני פסקאות עבריות שלמות. | en: Blazing fast deterministic layout engine running Knuth-Plass paragraph breaking across full Hebrew texts.
- [hebrew-justifier] icon: typography | badge: עברית: דיוק טיפוגרפי, en: Typographic Precision
  - Title: עברית: יישור עברי מסורתי תלת-שכבתי | en: 3-Tier Traditional Hebrew Justifier
  - Description: עברית: שילוב עדין של רווחי מילים גמישים, מתיחת אותיות רפויות (אהלתר"ם) וריווח בין-אותי מיקרומטרי ללא צורך במקפים מכוערים. | en: Harmonious blending of word spacing, wide Hebrew letter dilation, and micro letter-spacing.
- [pdf-x-export] icon: export | badge: עברית: תקן דפוס, en: Print Standard
  - Title: עברית: ייצוא תקני ISO PDF/X-1a ו-PDF/X-4 | en: ISO PDF/X-1a & PDF/X-4 Pre-Press
  - Description: עברית: הפקת קבצים מוכנים ישירות להדפסת אופסט ודפוס דיגיטלי מסחרי, עם הפרדת צבעים מלאה של 100% K לטקסט חד ונקי. | en: Production-ready PDF export for offset & digital printing, strict 100% pure black channel separation.
- [sacred-text-shield] icon: check | badge: עברית: מסורת ישראל, en: Traditional Rules
  - Title: עברית: קדושת הטקסט ונרמול קנוני | en: Sacred Text Shield & Canonical Normalization
  - Description: עברית: מניעת שבירת שמות קדושים בכללי No-Break קפדניים, סנכרון גימטריה עברית לעמודים ונרמול סדר ניקוד וטעמים. | en: Strict no-break protection for divine names, Hebrew gematria page numbering, and canonical niqqud normalization.

### Changes
- [features] עברית: מבנה עמוד רב-תזרימי מלא עבור דפי גמרא (מרכז, רש"י, תוספות, הערות) ומקראות גדולות | en: Full multi-flow page architecture for Talmud pages and rabbinic commentaries
- [features] עברית: ייצוא PDF/X-1a תקני עם פרופילי צבע Fogra 39 ו-ISO Coated v2 | en: Compliant ISO PDF/X-1a export with Fogra 39 and ISO Coated v2 profiles
- [typography] עברית: אלגוריתם מתיחת אותיות עבריות רפויות וריווח מילים גמיש | en: Hebrew dilated letter expansion and elastic word space distribution
- [improvements] עברית: ניהול פרויקטים ומסך פתיחה חדש עם תבניות מהירות לספרים, עלונים וקונטרסים | en: Project manager and fresh welcome dashboard with quick templates for books and booklets
- [fixes] עברית: תיקון חישוב גבולות עמודים בצמדי עמודים (Spreads) בספרים מימין לשמאל | en: Fixed page spread layout calculation for right-to-left Hebrew books

---

## [0.29.0] - 2026-09-20
- **Importance**: minor
- **Badge**: עברית: תוספים ועריכה | en: Plugins & Editing
- **Title**: עברית: מערכת תוספים מודולרית ועורך טקסט משולב | en: Modular Plugin System & Integrated Story Editor
- **Summary**: עברית: הוספת סביבת הרחבות מבודדת ומאובטחת לתוספים ב-TypeScript/JavaScript, לצד עורך טקסט רציף (Story Editor) עם חיפוש והחלפה. | en: Introduced an isolated plugin architecture for TypeScript/JavaScript extensions alongside a split Story Editor.

### Changes
- [features] עברית: מנוע פלאגינים מאובטח (PluginEngine) עם הרשאות מוגדרות והידור בזמן ריצה | en: Hardened plugin sandbox engine with strict permission boundaries and runtime compilation
- [features] עברית: עורך טקסט משולב בתצוגת Split View עם ספירת מילים חיה לכל תזרים | en: Integrated Story Editor with split view and live word counts per flow
- [improvements] עברית: ייבוא ישיר של קובצי Word (.docx), Markdown (.md) וטקסט רציף | en: Direct import pipeline for Word (.docx), Markdown (.md), and plain text
- [fixes] עברית: שיפור מנגנון שמירה אוטומטית ומניעת איבוד שינויים בעת יציאה | en: Enhanced auto-save safety checks and unsaved change prevention on quit

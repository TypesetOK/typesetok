# מפת קוד ויעדי פיתוח — TypesetOK (TOK) Code Map & Roadmap
**גרסה נוכחית:** v0.34.3  
**תאריך עדכון:** נובמבר 2026

---

## 🗺️ מפת הקוד המלאה (Monorepo Code Map)

### 1. ליבת המערכת ב-Rust (`crates/`)

```
crates/
├── tok-core/                    # מודל מסמך סמנטי (TDM) ותשתית בסיס
│   ├── src/anchor.rs            # עוגני טקסט יציבים במרחב הפסקאות
│   ├── src/id.rs                # מזהי ULID ואינדוקס שברירי ב-O(1) (FractionalIndex)
│   ├── src/model.rs             # עץ המסמך: DocumentRoot, SectionNode, Flow, ParagraphNode
│   ├── src/normalizer.rs        # נרמול דטרמיניסטי תקן ישראלי ת״י 6100 (SI 6100)
│   ├── src/styles.rs            # מודל סגנונות פסקה, תו והתקדמות כיוונית
│   └── src/transaction.rs       # טרנזקציות אטומיות ו-Undo/Redo בלתי מוגבל
│
├── tok-typeset/                 # מנוע העימוד, הטיפוגרפיה והחישוב המרחבי
│   ├── src/bidi.rs              # חלוקת ריצות BiDi וכיווניות ימין-לשמאל
│   ├── src/engine.rs            # מתאם העימוד המרכזי, מרווחי פסקאות עם Margin Collapsing וחלוקה לעמודים
│   ├── src/font.rs              # ניהול גופני TrueType/OpenType ועיצוב rustybuzz/HarfBuzz
│   ├── src/gematria.rs          # גימטריה דטרמיניסטית עם תווי גרש/גרשיים וטבלת טאבו
│   ├── src/geometry.rs          # תיבות פריסה: PageLayoutBox, TextFrameBox, LineBox, GlyphBox
│   ├── src/hebrew_justify.rs    # יישור תלת-שלבי: מילים, מתיחת אהלתר״ם ומיקרו-טרקינג
│   ├── src/hit_test.rs          # מנוע Hit-Testing מרחבי תת-פיקסלי וסימון טווח שורות
│   ├── src/knuth_plass.rs       # אלגוריתם שבירת שורות אופטימלי גלובלי (Knuth-Plass)
│   ├── src/multi_flow.rs        # פותר צורת הדף, פריסות צמודות (Recto/Verso) והערות שוליים
│   └── src/shaper.rs            # עיצוב גליפים והתאמת רווחים
│
├── tok-pdf/                     # מנוע קדם-דפוס נייטיב וייצוא PDF/X
│   ├── src/boxes.rs             # חישוב תיבות דפוס (MediaBox, BleedBox, TrimBox, SlugBox)
│   ├── src/color.rs             # תמיכה ב-DeviceCMYK וצבעי ספוט
│   ├── src/font_subsetter.rs    # גזירת תת-קבוצות גופנים TrueType/OpenType Subsetting
│   ├── src/html_projection.rs   # מחולל Pre-paginated HTML לתצוגת דפדפן ו-Vivliostyle
│   ├── src/pdf_engine.rs        # פולט PDF/X תקני: דחיסת FlateDecode, תאריכי Manifest דטרמיניסטיים וסימני חיתוך
│   └── src/tounicode.rs         # מפות CMap /ToUnicode להעתקה וחיפוש של טקסט מנוקד
│
├── tok-storage/                 # מנוע אחסון עמיד קריסות ופורמט .tok
│   ├── src/book.rs              # מנהל רב-מסמכים (.tokbook) לספרים מרובי כרכים
│   ├── src/migration.rs         # מיגרציות סכמה עם תאימות לאחור
│   ├── src/package.rs           # ארכיב ZIP אטומי מוגן (.tok) עם Manifest
│   └── src/workspace.rs         # סביבת עבודה טרנזקציונית ACID מבוססת WAL
│
├── tok-ipc/                     # פרוטוקול תקשורת מהיר בין Rust ל-TypeScript
│   ├── src/framing.rs           # מסגור הודעות בינאריות (Length-Prefixed Framing)
│   └── src/protocol.rs          # סכמת פקודות ואירועי IPC (HitTest, InsertText וכו')
│
├── tok-plugin-host/             # מארח הרחבות מבודד
│   └── src/host.rs              # הרצת תוספים עם הגנת קריסות (catch_unwind)
│
└── tok-cli/                     # ממשק שורת פקודה לבדיקות, בנצ'מרק ורינדור
    └── src/main.rs              # הרצת מבדקי דטרמיניזם, בנצ'מרק 1,000 עמודים וייצוא
```

---

### 2. מעטפת שולחן העבודה ב-TypeScript (`packages/`)

```
packages/
├── tok-electron/                # תהליך ראשי של Electron
│   ├── src/main.ts              # ניהול חלונות, חיבור ל-tok-cli והפעלת המעטפת
│   ├── src/menu.ts              # תפריטי מערכת שולחניים מקומיים (Native Menus)
│   └── src/preload.ts           # גשר IPC מבודד (Context Isolation & contextBridge)
│
├── tok-viewer/                  # מנוע תצוגה מבוסס וירטואליזציה
│   └── src/virtualizer.ts       # אכיפת חלון פעיל של 3 עמודים בלבד [K-1, K, K+1]
│
├── tok-canvas/                  # שכבת כיסוי אינטראקטיבית שקופה (Canvas Overlay)
│   └── src/overlay.ts           # סמן עריכה אופטימי ב-RTL (<16ms), סימון גרירה והקלדה
│
├── tok-story-editor/            # עורך סיפור רציף (Story Editor)
│   └── src/editor.ts            # עריכת פסקאות טקסט רציף המסונכרן מול הדפים
│
└── tok-ui/                      # אפליקציית סביבת העבודה (Desktop Workbench)
    ├── src/app.ts               # מחלקת האפליקציה הראשית TypesetOkApp
    ├── src/renderer.ts          # נקודת כניסה לתצוגה וחיבור אירועים
    └── src/components/          # רכיבי ממשק: סרגל כלים (Toolbar), חלונית עמודים וכו'
```

---

## 🚀 מפת יעדים ואבני דרך (Milestones & Roadmap)

| גרסה | אבן דרך | תיאור והישגים מרכזיים | סטטוס |
| :---: | :--- | :--- | :---: |
| **v0.1** | **ליבת המסמך והעימוד הבסיסי** | מודל AST סמנטי (TDM), אינדוקס שברירי, נרמול ת״י 6100, ואלגוריתם Knuth-Plass. | **הושלם** ✅ |
| **v0.2** | **מנוע קדם-דפוס ו-PDF/X** | ייצוא PDF/X-1a, צבע שחור K=100%, סימני חיתוך, ותיבות Bleed/Trim. | **הושלם** ✅ |
| **v0.3** | **מעטפת Electron ווירטואליזציה** | שולחן עבודה Electron, וירטואליזציית DOM של 3 עמודים פעילים (120 FPS). | **הושלם** ✅ |
| **v0.4** | **גופני עברית מקוריים ו-Subsetting** | שילוב Noto Serif Hebrew ו-David Libre, עיצוב OpenType וגזירת גופנים ב-PDF. | **הושלם** ✅ |
| **v0.6** | **צורת הדף לש״ס והערות שוליים** | פריסות צמודות (Recto/Verso), שבירת צורה ב-L-Shape מתחת לגמרא, והערות שוליים צפות. | **הושלם** ✅ |
| **v0.7** | **שילוש ארכיטקטוני וממשק DTP מודרני** | מימוש Action HUD, מפקח הקשרי (Inspector) עם יישור עברי תלת-שלבי (אהלתר"ם), Command Palette (Ctrl+K), סרגל סטטוס ורשת עזרים. | **הושלם** ✅ |
| **v0.8** | **חוויית שולחן עבודה מלאה, מערכת תוספים והתקנה** | מנגנון עדכונים מתוך התוכנה, לוגים עם מחיקה אוטומטית לפי ימים, מתקין דו-לשוני (עברית/אנגלית), מערכת תוספים (JS/TS), ספלאש מהיר אנטי-ניגודיות, מסך בחירת פרויקטים, כותרת מודרנית ללא סרגל אפרפר, RTL/LTR מלא, והגדרות ואודות. | **הושלם** ✅ *(גרסה נוכחית)* |
| **v1.0** | **אריזה רשמית ומוכנות לייצור** | קובץ התקנה חתום דיגיטלית ל-Windows, חנות תוספים מורחבת ומוכנות מלאה למכוני הוצאה לאור. | **היעד הבא** 🎯 |

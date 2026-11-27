<p align="center">
  <img src="assets/icon.png" alt="TypesetOK Icon" width="128" height="128" />
</p>

<h1 align="center">TypesetOK (TOK)</h1>

<p align="center">
  <strong>תוכנת עימוד שולחנית מקצועית לעברית, רב-טקסט (מקראות גדולות וש"ס), צורת הדף חיה וקדם-דפוס</strong><br>
  <em>גרסה 0.38.1 • ליבת Native Rust • מעטפת Tauri v2 • דטרמיניזם מלא</em>
</p>

<p align="center">
  <a href="./README.md">English</a> • <strong>עברית</strong>
</p>

<p align="center">
  <img src="assets/logo.jpg" alt="TypesetOK Official Logo" width="640" />
</p>

---

**TypesetOK (TOK)** היא תוכנת עימוד שולחנית (Desktop Publishing — DTP) שנבנתה מן היסוד עבור טיפוגרפיה עברית מתקדמת, טקסט דו-כיווני (`Bidi` / `RTL`), עימוד רב-תזרימי מסונכרן ("צורת הדף" חיה, מקראות גדולות, ש"ס תלמודי, ספרי שו"ת ומחקר), והפקת קובצי דפוס תקניים (`ISO 15930-1:2001 PDF/X-1a:2001`) באופן דטרמיניסטי מקצה לקצה.

---

## 1. סקירה ארכיטקטונית (Architecture Overview)

הפרויקט מאורגן כ-Monorepo המפריד באופן מוחלט בין ליבת העימוד והמסמך ב-Rust לבין שכבות התצוגה והאינטראקציה ב-TypeScript. לאינדקס מלא המפנה לכל קובץ בפרויקט ומסביר את תפקידו ראו: [`docs/PROJECT_FILES.md`](./docs/PROJECT_FILES.md).

### תרשים זרימת נתונים דטרמיניסטי (End-to-End Deterministic Data Flow)

```text
[UI / Story Editor / SpreadCanvas (TypeScript)]
       │
       ▼  (פקודות IPC של Tauri v2)
[tok-desktop (Tauri v2)]
       │
       ├──► [tok-engine-service]  שכבת תיאום מאוחדת לעימוד, קדם-דפוס, אחסון ותוספים
       ├──► [tok-core]            Canonical Document Model (TDM), Multi-Apparatus, B-Tree Rope, ULID, Hebrew Normalizer
       ├──► [tok-typeset]         Unicode Bidi (UAX #9) + Masoretic Shaper + Knuth-Plass + Multi-Apparatus + Cassowary + Co-Pagination
       │
       ▼
[PageLayoutBox Geometry Tree (in DTP Points / pt)]
       │
       ├──────────────────────────────────────────────────────────┐
       ▼                                                          ▼
[Screen & Interaction Pipeline]                         [Native Pre-Press Pipeline]
- tok-ui: SpreadCanvas (Deterministic DOM Projection)   - tok-pdf: Direct PDF/X-1a:2001 & PDF/X-4 Generator
- tok-viewer: PageDomVirtualizer ([K-1, K, K+1] Window)   (DeviceCMYK, Spot Colors, TrimBox/BleedBox,
- tok-canvas: CanvasInteractionOverlay (120fps Caret)      Embedded Font Subsets & /ToUnicode CMap)
- tok-story-editor: Unpaginated Lossless Story Editor
```

> **עיקרון הנדסי מחייב:** הדפדפן/WebView **אינו** מחליט היכן נשברות שורות או היכן מתפצלים עמודים, והמערכת **אינה** משתמשת בספריות Polyfill חיצוניות בדפדפן (כגון `@vivliostyle/core` או `Paged.js`). כל הגיאומטריה מחושבת ב-Rust (`tok-typeset`), מיוצאת כמבני `PageLayoutBox` ביחידות `pt` מדויקות, ומוקרנת להיטל DOM קשיח (`SpreadCanvas`) או ישירות לקובץ `PDF/X-1a` / `PDF/X-4`.

---

### 9 חבילות הליבה ב-Rust (`crates/`)

| חבילה (Crate) | תפקיד ואחריות מרכזית |
| :--- | :--- |
| **`tok-core`** | מודל המסמך הקנוני (TDM): עץ מסמך אי-מוטבילי, `ropey` B-Tree Rope לטקסטים ארוכים, מזהי `ULID`, אינדוקס שברירי (`FractionalIndex`), מחסנית טרנזקציות ו-Undo/Redo דלתא, תצורות מערך הערות רב-שכבתי (`FootnoteApparatus`), טלאי עיצוב טיפוגרפיים, מספור כותרות וגימטריה בסיסית (`ט״ו`, `ט״ז`), ושער נרמול קלט טיפוגרפי עברי קנוני ($\text{Shin/Sin Dot} \rightarrow \text{Dagesh/Mapiq} \rightarrow \text{Niqqud} \rightarrow \text{Meteg} \rightarrow \text{Te'amim}$). |
| **`tok-typeset`** | מנוע העימוד והטיפוגרפיה: שייפר מסורה וטעמי מקרא ייעודי (`MasoreticShaper`, נון הפוכה, פסיק, מתג עליון ותחתון ותכונות OpenType מקראיות), פירוק כיווניות `unicode-bidi` (UAX #9), עיצוב גופנים ב-Rust טהור (`rustybuzz` כולל GPOS `mark`/`mkmk`), אלגוריתם שבירת שורות גלובלי **Knuth-Plass**, מדרג יישור עברי תלת-שלבי (רווחי מילים מבוקרים $\rightarrow$ מתיחת אותיות אהלתר"ם $\rightarrow$ מיקרו-טרקינג $\pm 2\%$), פותר אילוצים ליניארי (**Cassowary**), מנוע גימטריה עברית עם טבלת טאבו (`ט״ו`, `ט״ז`, `ע״ר`, `ער״ב`, `ער״ה`, `חר״צ`, `ד״ש`, `שי״ד`, `שד״מ`, `נט״ש`, `תרח״צ`, `תשד״מ` ותווי `U+05F3`/`U+05F4`), חלוקת יחסי רוחב דינמיים לתזרימים רבניים (2, 3 ו-4 טורים בש"ס ומקראות גדולות תוך מעקב Recto/Verso מדויק), מנוע פריסת הערות שוליים רב-שכבתי מקבילי (`FootnotePaginator`), אותיות פתיחה עבריות (Drop Caps), ומנוע מצולעים וחיתוך Scanline לספיגת פירושים בצורת L. |
| **`tok-pdf`** | מנוע קדם-דפוס נייטיבי (`pdf-writer`): הפקה ישירה של קובצי `PDF/X-1a:2001` (ISO 15930-1) ומצבי `PDF/X-4`, מרחב צבע `DeviceCMYK` וצבעי ספוט (`Separation`), תיבות דפוס תקניות (`MediaBox`, `CropBox`, `BleedBox`, `TrimBox`), סימני חיתוך ורישום (Crop & Registration Marks), קידוד תמונות PNG/JPEG ל-XObject (`image_xobject`), שיבוץ גופנים וטבלאות `/ToUnicode` לחיפוש והעתקת טקסט עברי, ומנוע בדיקות קדם-דפוס (`Preflight`). |
| **`tok-storage`** | מנוע אחסון כפול: סביבת עבודה חסינת-קריסות ב-**`redb`** (ACID) + שמירה אטומית לקובץ חבילה `.tok` (ארכיב ZIP בדחיסת `Deflate` הכולל `manifest.json`, `document.json`, `assets/` ו-`previews/`). |
| **`tok-plugin-host`** | מנוע הרחבות ותוספים דטרמיניסטי: הרצת תוספי ליבה מבוססי טרנזקציות אטומיות עם בדיקות הרשאות (`PluginCapability`) ובידוד קריסות (`catch_unwind`), כולל התוספים המובנים `tok.builtin.divine-shield` ו-`tok.builtin.hebrew-grep-acronym`. |
| **`tok-engine-service`** | שכבת שירות מאוחדת ב-Rust המרכזת טעינת מסמכים, עימוד, ייצוא PDF/HTML, בדיקות `Preflight`, שמירת חבילות והרצת תוספים עבור `tok-desktop` ו-`tok-cli`. |
| **`tok-ipc`** | ספריית פרוטוקול תקשורת (`TOKB` Binary Framing ו-`WorkerRequest` ב-JSON) להודעות פקודה ותגובה בין תהליכים. |
| **`tok-cli`** | בינארי ה-CLI הראשי של המערכת, המשמש להרצה באצווה (Headless Batch Processing & CI) ולמצב `worker` מתמיד. |
| **`tok-desktop`** | מעטפת שולחן העבודה הראשית ב-**Tauri v2**: מנהלת חלונות מקומיים, פקודות IPC של Tauri, גופני מערכת ב-Windows, שיוך קובצי `.tok`, טעינת תוספים ועדכונים מאומתי Ed25519, וקוראת ישירות ל-`tok-engine-service` בתוך אותו תהליך. |

---

### 4 חבילות הממשק ב-TypeScript (`packages/`)

| חבילה (Package) | תפקיד ואחריות מרכזית |
| :--- | :--- |
| **`tok-ui`** | סביבת העבודה השולחנית המלאה: רכיב `SpreadCanvas` להקרנת `PageLayoutBox` להיטל DOM דטרמיניסטי, צינור סנכרון סגנונות וטיפוגרפיה חי בין סרגל המאפיינים, סרגל הכלים והקנבס, גרירה ישירה ונעילת תיבות טקסט על גבי הקנבס, הוספת תזרימים מותאמים אישית (כולל פריסת בלוקים מפוצלים בקצוות ללא חיבור ביניהם הזורמים לפי גובה), סינון סגנונות מותאם לתבנית הפעילה, מסך בחירת תבנית מודרך בפתיחת מסמך עם המלצה על ספר רציף, מערכת עדכונים ויומן שינויים ("מה חדש"), מנוע תוספים מוקשח (`PluginEngine`), ובדיקות קדם-דפוס חיות (`Preflight`). |
| **`tok-viewer`** | מנהל הווירטואליזציה של עמודי התצוגה (`PageDomVirtualizer`): שומר ב-DOM חלון פעיל של 3 עמודים בלבד (`[K-1, K, K+1]`) ומייצג את שאר עמודי הספר כאלמנטי Spacer קלי-משקל. |
| **`tok-canvas`** | שכבת הכיסוי הגרפית השקופה (`CanvasInteractionOverlay`): ציור סמן דו-כיווני (`Dual Caret` / `RTL Caret`), תנועת סמן אופטימית מיידית (`<16ms`), בחירה ויזואלית לא-רציפה בטקסט מעורב, ובדיקת פגיעה מרחבית (`Hit-Testing`). |
| **`tok-story-editor`** | עורך סיפור רציף (`Unpaginated Story Editor`) לעריכת תוכן מהירה ללא תקורת עימוד רב-עמודי, עם המרת DOM דו-כיוונית ללא אובדן נתונים, ושימור מלא של טלאי עיצוב וסגנונות מקומיים. |

---

## 2. דרישות קדם והוראות התקנה ובנייה (Prerequisites & Setup)

### דרישות מערכת
1. **מערכת הפעלה:** Windows 10/11 (64-bit).
2. **קומפיילר Rust:** גרסה יציבה עדכנית (`rustc` / `cargo`) עם יעד `x86_64-pc-windows-gnu`.
   - הקובץ [`.cargo/config.toml`](./.cargo/config.toml) במאגר מוגדר מראש לשימוש בלינקר המהיר `rust-lld` ולתיקיית הידור מקומית (`C:/Users/USER/AppData/Local/tok_target`) למניעת בעיות נתיבים בעברית ב-Windows. אם שם המשתמש שלך ב-Windows שונה מ-`USER`, עדכן את `target-dir` בקובץ זה בהתאם.
3. **Node.js:** גרסה `20.0.0` ומעלה + `npm`.

### שלבי התקנה ובנייה מלאים

```powershell
# 1. התקנת תלויות ה-Node/TypeScript
npm install

# 2. בניית כל חבילות ה-Rust במאגר (כולל tok-desktop ו-tok-cli)
cargo build --workspace

# 3. בדיקת תקינות הטיפוסים, הידור ואריזת ממשק המשתמש
npm run lint
npm run build

# 4. הפעלת תוכנת השולחן TypesetOK (Tauri v2)
npm start

# 5. (אופציונלי) אריזת גרסה שולחנית ובניית קובץ התקנה (Installer)
npm run package:desktop
npm run package:installer
```

---

## 3. שימוש בבינארי שורת הפקודה (`tok-cli`)

הבינארי `tok-cli` מאפשר הרצה של כל פעולות הליבה ישירות משורת הפקודה או מצינורות CI/CD:

| תת-פקודה (Subcommand) | תחביר ופרמטרים | תיאור הפעולה |
| :--- | :--- | :--- |
| **`typeset-document`** | `tok-cli typeset-document <INPUT.json\|.tok\|--demo> <OUTPUT.json> [--fonts-dir <DIR>] [--assets-from <TOK>]` | עימוד מלא של מסמך (כולל Knuth-Plass, Cassowary ו-Co-Pagination) והחזרת עץ `PageLayoutBox` מפורט ביחידות `pt`. |
| **`incremental-typeset`** | `tok-cli incremental-typeset <INPUT.json\|--demo> <OUTPUT.json> [--node-id <ID>]` | עימוד אינקרמנטלי מבוסס התכנסות (`Convergence Paginator`) עם אבחון חישוב מחדש. |
| **`diagnose-recompute`** | `tok-cli diagnose-recompute <INPUT.json\|.tok\|--demo>` | הרצת דוח אבחון לביטול תקפות ומטמון בשלוש נקודות במסמך. |
| **`render-pdf`** | `tok-cli render-pdf <INPUT.json\|.tok\|--demo> <OUTPUT.pdf> [--standard pdfx4] [--profile <ID>] [--no-bleed] [--no-crop-marks] [--registration-marks]` | הפקת קובץ דפוס תקני `PDF/X-1a:2001` או `PDF/X-4` ישירות מהמסמך (כולל CMYK, תיבות דפוס ושיבוץ גופנים). |
| **`render-html`** | `tok-cli render-html <INPUT.json\|.tok\|--demo> <OUTPUT.html>` | הפקת קובץ HTML עצמאי בעל עמודים קבועים מראש (Pre-paginated HTML/CSS) לתצוגה ובדיקה. |
| **`save-package`** | `tok-cli save-package <INPUT.json> <OUTPUT.tok> [--fonts-dir <DIR>] [--assets-from <TOK>]` | אריזה ושמירה אטומית של מסמך JSON לפורמט החבילה `.tok` (ZIP + Deflate). |
| **`open-package`** | `tok-cli open-package <INPUT.tok> <OUTPUT.json>` | פתיחת קובץ `.tok` ושחזור ה-JSON הקנוני של המסמך. |
| **`inspect-package`** | `tok-cli inspect-package <INPUT.tok>` | בדיקת תקינות ארכיב `.tok` והדפסת סיכום מטא-דאטה (כותרת, גרסת סכמה ונכסים מוטמעים). |
| **`inspect-font`** | `tok-cli inspect-font <PATH.ttf>` | בדיקת גופן חיצוני (`glyf`/OS2 `fsType`/כיסוי עברי) ופליטת דוח JSON. |
| **`preflight`** | `tok-cli preflight <INPUT.json\|.tok\|--demo> [OUTPUT.json] [--fonts-dir <DIR>] [--assets-from <TOK>]` | הרצת בדיקות קדם-דפוס (`Preflight`) ופליטת דוח `PreflightReport` ב-JSON. |
| **`run-plugins`** | `tok-cli run-plugins <INPUT.json\|.tok\|--demo> <OUTPUT.json>` | הרצת תוספי הליבה הדטרמיניסטיים המובנים (`tok.builtin.divine-shield`, `tok.builtin.hebrew-grep-acronym`) על המסמך. |
| **`sync-heading-numbers`** | `tok-cli sync-heading-numbers <INPUT.json\|.tok\|--demo> <OUTPUT.json> [--plain]` | סנכרון מספור כותרות וסימנים לפי ראשי הפרקים והגימטריה העברית. |
| **`verify-determinism`** | `tok-cli verify-determinism` | הרצת עימוד והפקת PDF/HTML עבור תרחיש פרוזה ותלמוד רב-תזרימי ואימות זהות מוחלטת ברמת הבייט (SHA-256). |
| **`benchmark-typeset`** | `tok-cli benchmark-typeset [--pages N]` | בדיקת עומס וביצועים (ברירת מחדל `1000` עמודים) והדפסת מדדי זמן עימוד ומהירות לעמוד. |
| **`worker`** | `tok-cli worker` | הפעלת `tok-cli` במצב תהליך מתמיד דרך stdin/stdout (שורות JSON) עם שמירת מטמון עימוד. |

---

## 4. בדיקות ואימות מערכת (Testing & Verification)

הפרויקט כולל מערך בדיקות רב-שכבתי המאמת את ליבת ה-Rust, את חבילות ה-TypeScript ואת האינטגרציה מקצה לקצה:

```powershell
# 1. הרצת כל בדיקות היחידה והאינטגרציה של ליבת ה-Rust
cargo test --workspace

# 2. בדיקת קומפילציה וטיפוסים קפדנית בכל 4 חבילות ה-TypeScript
npm run lint

# 3. הרצת חבילת הבדיקות של ה-Frontend, הגימטריה, הווירטואליזציה, ה-Canvas, ה-Story Editor, מערכת העדכונים והאבטחה
npm test

# 4. אימות דטרמיניזם ביט-לביט (Bit-for-Bit Determinism)
cargo run -p tok-cli -- verify-determinism
```

---

## 5. מגבלות ידועות בהיקף הנוכחי (Known Limitations)

בהתאם למשמעת ההנדסית של הפרויקט (אפס טענות שווא ואפס מצגי דמה), להלן המגבלות הידועות בגרסה הנוכחית:

1. **תמיכה בתמונות TIFF (BigTIFF / JPEG-in-TIFF):** תמונות TIFF תקניות (1/8/16-bit ב־RGB, ‏CMYK, גווני אפור, פלטה, דחיסת LZW, ‏ZIP/Deflate, ‏PackBits, קבצים מרובי עמודים באמצעות `#page=N`, מטא־דאטה של רזולוציית DPI ופרופילי ICC מוטמעים) נתמכות במלואן ב־`tok-pdf` (`tiff`, `image_xobject`); פורמט BigTIFF (היסטים של 64 ביט, magic `43`) ו־TIFF בדחיסת JPEG פנימית (`Compression=6/7`) אינם נתמכים.
2. **מנוע בתהליך יחיד (`tok-engine-service`):** מעטפת ה-Tauri v2 (`tok-desktop`) קוראת ישירות ל-`tok-engine-service` בתוך אותו תהליך, בעוד `crates/tok-ipc` מספקת את טיפוסי המסגור ופרוטוקול ה-Worker עבור `tok-cli`.
3. **ייצוא PDF/X-4 וניהול פרופילי ICC חיצוניים:** מחולל ה-PDF מפיק קובצי `PDF/X-1a:2001` ו-`PDF/X-4` עם פקודות צבע `DeviceCMYK` וצבעי `Separation` (Spot Colors) אמיתיים ומילון `OutputIntent` מובנה (ISO Coated v2 / Fogra39), ללא קישור דינמי לספריית C חיצונית דוגמת `LittleCMS`.

---

## 6. הנחיות הנדסיות ותרומה לקוד (Engineering Guidelines)

לכללי התרומה והתנהלות הקהילה ראו גם: [`CONTRIBUTING.md`](./CONTRIBUTING.md) ו-[`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md).

1. **דטרמיניזם מוחלט:** כל שינוי ב-`tok-core`, `tok-typeset` או `tok-pdf` חייב לעבור בהצלחה את `tok-cli verify-determinism`. אסור להשתמש ב-`HashMap` בעל סדר אקראי בפלטים משפיעי-פריסה (יש להשתמש ב-`BTreeMap` או במיון מפורש).
2. **אפס `unsafe` ב-Rust:** חל איסור על הוספת בלוקים של `unsafe` בחבילות הליבה.
3. **אפס כפתורי דמה ב-UI:** אין להוסיף כפתורים או תפריטים שמציגים רק הודעת `Toast` ללא פעולה אמיתית המשנה את מודל המסמך, את התצוגה או את הקובץ המיוצא.
4. **בדיקות אמת (No Mock Re-implementations):** על כל בדיקה ב-`test/frontend.test.mjs` לייבא ולהריץ את הקוד האמיתי מתוך `packages/**` או `tok-cli`, ולא לממש פונקציות חיקוי מקומיות בתוך קובץ הבדיקה.

---

## 7. רישוי וזכויות יוצרים (License & Third-Party Notices)

- קוד המקור של **TypesetOK (TOK)** מופץ תחת רישיון **TypesetOK Non-Commercial Copyleft License v1.2 (`TOK-NCCL v1.2`)**.
- זהו רישיון **Source-Available לשימוש לא-מסחרי בלבד** (אינו רישיון Open Source מאושר OSI). מותרים בו, בכפוף לתנאי הרישיון (ייחוס, Copyleft מלא על נגזרות, והיתר שימוש אוטומטי ובלתי חוזר לפרויקט הראשי בקוד של פורקים):
  - שימוש אישי, אקדמי, מחקרי, חינוכי וקהילתי ללא מטרות רווח;
  - עימוד והפצה **במחיר העלות בלבד (`At-Cost`)** — כיסוי הוצאות ישירות בפועל בלבד ללא רווח, מרווח שיווקי או שכר טרחת עימוד;
  - **מחבר המעמד את ספרו האישי (`Self-Authored Book`)** — אדם פרטי המעמד בעצמו ספר שהוא מחברו/בעליו המקורי לצורך מכירתו (מעמדים מקצועיים, בתי דפוס, מכוני עימוד, הוצאות לאור מסחריות וכל ניסיון עקיפה אסורים ללא רישיון מסחרי).
- כל שימוש מסחרי אחר (לרבות בחינה מסחרית פנימית) מחייב הסכם רישיון מסחרי נפרד בכתב דרך הערוץ הרשמי של הפרויקט.
- לנוסח הרישיון המלא והמחייב משפטית (הנוסח באנגלית הוא הקובע בכל סתירה) ראו: [`LICENSE.md`](./LICENSE.md).
- לפירוט מלא של כלל ספריות הצד-השלישי (Rust ו-Node.js), הגופנים המצורפים (`SIL Open Font License 1.1`), וניתוח התאימות המשפטית ראו: [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md).

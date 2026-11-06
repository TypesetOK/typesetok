<p align="center">
  <img src="assets/icon.png" alt="TypesetOK Icon" width="128" height="128" />
</p>

<h1 align="center">TypesetOK (TOK)</h1>

<p align="center">
  <strong>תוכנת עימוד שולחנית מקצועית לעברית, רב-טקסט (מקראות גדולות וש"ס), צורת הדף חיה וקדם-דפוס</strong><br>
  <em>גרסה 0.34.9 • ליבת Native Rust • מעטפת Electron מוקשחת • דטרמיניזם מלא</em>
</p>

<p align="center">
  <img src="assets/logo.jpg" alt="TypesetOK Official Logo" width="640" />
</p>

---

**TypesetOK (TOK)** היא תוכנת עימוד שולחנית (Desktop Publishing — DTP) שנבנתה מן היסוד עבור טיפוגרפיה עברית מתקדמת, טקסט דו-כיווני (`Bidi` / `RTL`), עימוד רב-תזרימי מסונכרן ("צורת הדף" חיה, מקראות גדולות, ש"ס תלמודי, ספרי שו"ת ומחקר), והפקת קובצי דפוס תקניים (`ISO 15930-1:2001 PDF/X-1a:2001`) באופן דטרמיניסטי מקצה לקצה.

---

## 1. סקירה ארכיטקטונית (Architecture Overview)

הפרויקט מאורגן כ-Monorepo המפריד באופן מוחלט בין ליבת העימוד והמסמך ב-Rust לבין שכבות התצוגה והאינטראקציה ב-TypeScript:

### תרשים זרימת נתונים דטרמיניסטי (End-to-End Deterministic Data Flow)

```text
[UI / Story Editor / SpreadCanvas (TypeScript)]
       │
       ▼  (JSON Document Snapshot / Incremental Edit)
[tok-cli (Rust Binary Bridge)]
       │
       ├──► [tok-core]      Canonical Document Model (TDM), Multi-Apparatus, B-Tree Rope, ULID, Hebrew Normalizer
       ├──► [tok-typeset]   Unicode Bidi (UAX #9) + Masoretic Shaper + Knuth-Plass + Multi-Apparatus + Cassowary + Co-Pagination
       │
       ▼
[PageLayoutBox Geometry Tree (in DTP Points / pt)]
       │
       ├──────────────────────────────────────────────────────────┐
       ▼                                                          ▼
[Screen & Interaction Pipeline]                         [Native Pre-Press Pipeline]
- tok-ui: SpreadCanvas (Deterministic DOM Projection)   - tok-pdf: Direct PDF/X-1a:2001 Generator
- tok-viewer: PageDomVirtualizer ([K-1, K, K+1] Window)   (DeviceCMYK, Spot Colors, TrimBox/BleedBox,
- tok-canvas: CanvasInteractionOverlay (120fps Caret)      Embedded Font Subsets & /ToUnicode CMap)
- tok-story-editor: Unpaginated Lossless Story Editor
```

> **עיקרון הנדסי מחייב:** הדפדפן (Chromium) **אינו** מחליט היכן נשברות שורות או היכן מתפצלים עמודים, והמערכת **אינה** משתמשת בספריות Polyfill חיצוניות בדפדפן (כגון `@vivliostyle/core` או `Paged.js`). כל הגיאומטריה מחושבת ב-Rust (`tok-typeset`), מיוצאת כמבני `PageLayoutBox` ביחידות `pt` מדויקות, ומוקרנת להיטל DOM קשיח (`SpreadCanvas`) או ישירות לקובץ `PDF/X-1a`.

---

### 7 חבילות הליבה ב-Rust (`crates/`)

| חבילה (Crate) | תפקיד ואחריות מרכזית |
| :--- | :--- |
| **`tok-core`** | מודל המסמך הקנוני (TDM): עץ מסמך אי-מוטבילי, `ropey` B-Tree Rope לטקסטים ארוכים, מזהי `ULID`, אינדוקס שברירי (`FractionalIndex`), מחסנית טרנזקציות ו-Undo/Redo דלתא, תצורות מערך הערות רב-שכבתי (`ApparatusConfig`), טלאי עיצוב טיפוגרפיים, מנוע גימטריה עברית (כולל טבלת טאבו: ט״ו, ט״ז, ע״ר, ער״ב, ער״ה, רח״צ, שי״ד, נט״ש, תשי״ד ותווי `U+05F3`/`U+05F4`), ושער נרמול קלט טיפוגרפי עברי קנוני ($\text{Base} \rightarrow \text{Shin/Sin Dot} \rightarrow \text{Dagesh/Mapiq} \rightarrow \text{Niqqud} \rightarrow \text{Meteg} \rightarrow \text{Te'amim}$). |
| **`tok-typeset`** | מנוע העימוד והטיפוגרפיה: שייפר מסורה וטעמי מקרא ייעודי (`MasoreticShaper`, נון הפוכה, פסיק, מתג עליון ותחתון ותכונות OpenType מקראיות), פירוק כיווניות `unicode-bidi` (UAX #9), עיצוב גופנים ב-Rust טהור (`rustybuzz` כולל GPOS `mark`/`mkmk`), אלגוריתם שבירת שורות גלובלי **Knuth-Plass**, מדרג יישור עברי תלת-שלבי (רווחי מילים מבוקרים $\leftarrow$ מתיחת אותיות אהלתר"ם $\leftarrow$ מיקרו-טרקינג $\pm 2\%$), פותר אילוצים ליניארי (**Cassowary**), חלוקת יחסי רוחב דינמיים לתזרימים רבניים (2, 3 ו-4 טורים בש"ס ומקראות גדולות תוך מעקב Recto/Verso מדויק), מנוע פריסת הערות שוליים רב-שכבתי מקבילי (`FootnotePaginator`), אותיות פתיחה עבריות (Drop Caps), ומנוע מצולעים וחיתוך Scanline לספיגת פירושים בצורת L. |
| **`tok-pdf`** | מנוע קדם-דפוס נייטיבי (`pdf-writer`): הפקה ישירה של קובצי `PDF/X-1a:2001` (ISO 15930-1) ומצבי PDF/X-4, מרחב צבע `DeviceCMYK` וצבעי ספוט (`Separation`), תיבות דפוס תקניות (`MediaBox`, `CropBox`, `BleedBox`, `TrimBox`), סימני חיתוך ורישום (Crop & Registration Marks), שיבוץ גופנים וטבלאות `/ToUnicode` לחיפוש והעתקת טקסט עברי, ומנוע בדיקות קדם-דפוס (`Preflight`). |
| **`tok-storage`** | מנוע אחסון כפול: סביבת עבודה חסינת-קריסות ב-**SQLite WAL** (`rusqlite`) + שמירה אטומית לקובץ חבילה `.tok` (ארכיב ZIP בדחיסת **Zstandard** הכולל `manifest.json`, `document.json`, `styles.json` ונכסים מאומתים בחתימת SHA-256). |
| **`tok-plugin-host`** | מנוע הרחבות ותוספים דטרמיניסטי: הרצת תוספי ליבה מבוססי טרנזקציות אטומיות תחת מגבלות משאבים (`Fuel` / `Memory` / `Capabilities`), כולל תוספים מובנים לניקוי רווחים כפולים, החלת סגנון דיבור המתחיל (ד"ה) ברש"י ותוספות, ואכיפת מקף עברי (`U+05BE`). |
| **`tok-ipc`** | ספריית פרוטוקול תקשורת בינארי (`TOKB` Binary Framing) להודעות פקודה ותגובה בין תהליכים. |
| **`tok-cli`** | בינארי ה-CLI הראשי של המערכת, המשמש הן להרצה באצווה (Headless Batch Processing & CI) והן כגשר העימוד והאחסון של מעטפת ה-Electron. |

---

### 5 חבילות הממשק ושולחן העבודה ב-TypeScript (`packages/`)

| חבילה (Package) | תפקיד ואחריות מרכזית |
| :--- | :--- |
| **`tok-electron`** | תהליך ה-Main של Electron וגשר `preload` מוקשח (`contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`), אימות סכמת IPC קפדני, חסימת ניווט חיצוני, וניהול קריאות תהליכי משנה אל `tok-cli`. |
| **`tok-ui`** | סביבת העבודה השולחנית המלאה: רכיב `SpreadCanvas` להקרנת `PageLayoutBox` להיטל DOM דטרמיניסטי, צינור סנכרון סגנונות וטיפוגרפיה חי בין סרגל המאפיינים, סרגל הכלים והקנבס, גרירה ישירה ונעילת תיבות טקסט על גבי הקנבס, הוספת תזרימים מותאמים אישית (כולל פריסת בלוקים מפוצלים בקצוות ללא חיבור ביניהם הזורמים לפי גובה), סינון סגנונות מותאם לתבנית הפעילה, מסך בחירת תבנית מודרך בפתיחת מסמך עם המלצה על ספר רציף, מערכת עדכונים ויומן שינויים ("מה חדש"), מנוע תוספים מוקשח (`PluginEngine`), ובדיקות קדם-דפוס חיות (`Preflight`). |
| **`tok-viewer`** | מנהל הווירטואליזציה של עמודי התצוגה (`PageDomVirtualizer`): שומר ב-DOM חלון פעיל של 3 עמודים בלבד (`[K-1, K, K+1]`) ומייצג את שאר עמודי הספר כאלמנטי Spacer קלי-משקל. |
| **`tok-canvas`** | שכבת הכיסוי הגרפית השקופה (`CanvasInteractionOverlay`): ציור סמן דו-כיווני (`Dual Caret` / `RTL Caret`), תנועת סמן אופטימית מיידית (`<16ms`), בחירה ויזואלית לא-רציפה בטקסט מעורב, ובדיקת פגיעה מרחבית (`Hit-Testing`). |
| **`tok-story-editor`** | עורך סיפור רציף (`Unpaginated Story Editor`) לעריכת תוכן מהירה ללא תקורת עימוד רב-עמודי, עם המרת DOM דו-כיוונית ללא אובדן נתונים, ושימור מלא של טלאי עיצוב וסגנונות מקומיים. |

---

## 2. דרישות קדם והוראות התקנה ובנייה (Prerequisites & Setup)

### דרישות מערכת
1. **מערכת הפעלה:** Windows 10/11 (64-bit).
2. **קומפיילר Rust:** גרסה יציבה עדכנית (`rustc` / `cargo`) עם יעד `x86_64-pc-windows-gnu`.
   - הקובץ `.cargo/config.toml` במאגר מוגדר מראש לשימוש בלינקר המהיר `rust-lld` ולתיקיית הידור מקומית (`C:/Users/USER/AppData/Local/tok_target`) למניעת בעיות נתיבים בעברית ב-Windows.
3. **Node.js:** גרסה `20.0.0` ומעלה + `npm`.

### שלבי התקנה ובנייה מלאים

```powershell
# 1. התקנת תלויות ה-Node/TypeScript
npm install

# 2. בניית כל חבילות ה-Rust במאגר (כולל בינארי ה-CLI החיוני למעטפת השולחנית)
cargo build --workspace
cargo build --bin tok-cli

# 3. בדיקת תקינות הטיפוסים והידור חבילות ה-UI וה-Electron
npm run lint
npm run build

# 4. הפעלת תוכנת השולחן TypesetOK ב-Electron
npm start
```

> **הערה חשובה לגבי איתור `tok-cli.exe`:** מעטפת ה-Electron (`packages/tok-electron/src/main.ts`) מחפשת את הקובץ `tok-cli.exe` לפי הסדר הבא:
> 1. משתנה הסביבה `TOK_CLI_PATH` (אם הוגדר).
> 2. תיקיית היעד המוגדרת ב-`.cargo/config.toml` (`%LOCALAPPDATA%\tok_target\debug\tok-cli.exe` או `release\tok-cli.exe`).
> 3. תיקיית `target/debug/tok-cli.exe` או `target/release/tok-cli.exe` בשורש המאגר.
> לכן חובה להריץ `cargo build --bin tok-cli` לפני הפעלת `npm start`.

---

## 3. שימוש בבינארי שורת הפקודה (`tok-cli`)

הבינארי `tok-cli` מאפשר הרצת כל פעולות הליבה ישירות משורת הפקודה או מצינורות CI/CD:

| תת-פקודה (Subcommand) | תחביר ופרמטרים | תיאור הפעולה |
| :--- | :--- | :--- |
| **`typeset-document`** | `tok-cli typeset-document <input.json> [output.layout.json]` | עימוד מלא של מסמך JSON (כולל Knuth-Plass, Cassowary ו-Co-Pagination) והחזרת עץ `PageLayoutBox` מפורט ביחידות `pt` (כולל גליפים, שורות, מסגרות, הערות שוליים ודוח Preflight). |
| **`incremental-typeset`** | `tok-cli incremental-typeset <input.json> <dirty_paragraph_index> [output.json]` | עימוד אינקרמנטלי מבוסס התכנסות (`Convergence Paginator`) החל מהפסקה שהשתנתה ועד לנקודת ההתכנסות. |
| **`diagnose-recompute`** | `tok-cli diagnose-recompute <base.json> <edited.json> <dirty_idx>` | השוואה ואבחון של מספר העמודים שחושבו מחדש לעומת עמודים שנשלפו מהמטמון בעריכה אינקרמנטלית. |
| **`render-pdf`** | `tok-cli render-pdf <input.json\|.tok> <output.pdf>` | הפקת קובץ דפוס תקני `PDF/X-1a:2001` ישירות מהמסמך (כולל CMYK, תיבות דפוס ושיבוץ גופנים). |
| **`render-html`** | `tok-cli render-html <input.json\|.tok> <output.html>` | הפקת קובץ HTML עצמאי בעל עמודים קבועים מראש (Pre-paginated HTML/CSS) לתצוגה ובדיקה. |
| **`save-package`** | `tok-cli save-package <input.json> <output.tok>` | אריזה ושמירה אטומית של מסמך JSON לפורמט החבילה `.tok` (ZIP + Zstandard + SHA-256 manifest). |
| **`open-package`** | `tok-cli open-package <input.tok> [output.json]` | פתיחת קובץ `.tok`, אימות שלמות החבילה והנכסים, ושחזור ה-JSON הקנוני של המסמך. |
| **`inspect-package`** | `tok-cli inspect-package <input.tok>` | בדיקת תקינות ארכיב `.tok` והדפסת סיכום מטא-דאטה (כותרת, מספר מדורים, פסקאות וגרסת סכמה). |
| **`run-plugins`** | `tok-cli run-plugins <input.json> <plugin_id[,plugin_id...]> [output.json]` | הרצת תוספי ליבה דטרמיניסטיים ב-Sandbox (`builtin.strip-double-spaces`, `builtin.apply-dibbur-hamatchil`, `builtin.enforce-hebrew-maqaf`) על המסמך. |
| **`verify-determinism`** | `tok-cli verify-determinism <input.json> [iterations]` | הרצת עימוד והפקת PDF/HTML מספר רב של פעמים ואימות זהות מוחלטת ברמת הבייט (SHA-256 Bit-for-Bit Determinism). |
| **`benchmark-typeset`** | `tok-cli benchmark-typeset <paragraphs_count>` | בדיקת עומס וביצועים על מסמך סינתטי גדול והדפסת מדדי זמן עימוד ומהירות לעמוד. |

### דוגמה להרצה מהירה ב-PowerShell

```powershell
# עימוד קובץ דוגמה של מקראות גדולות והפקת PDF/X-1a
cargo run --bin tok-cli -- typeset-document examples/mikraot_gedolot_sample.json out_layout.json
cargo run --bin tok-cli -- render-pdf examples/mikraot_gedolot_sample.json out_print.pdf
cargo run --bin tok-cli -- verify-determinism examples/mikraot_gedolot_sample.json 5
```

---

## 4. בדיקות ואימות מערכת (Testing & Verification)

הפרויקט כולל מערך בדיקות רב-שכבתי המאמת את ליבת ה-Rust, את חבילות ה-TypeScript ואת האינטגרציה מקצה לקצה:

```powershell
# 1. הרצת כל בדיקות היחידה והאינטגרציה של ליבת ה-Rust (7 Crates)
cargo test --workspace

# 2. בדיקת קומפילציה וטיפוסים קפדנית בכל 5 חבילות ה-TypeScript
npm run lint

# 3. הרצת חבילת הבדיקות של ה-Frontend, הגימטריה, הווירטואליזציה, ה-Canvas, ה-Story Editor, מערכת העדכונים והאבטחה (240 בדיקות מקיפות)
npm test

# 4. אימות דטרמיניזם ביט-לביט (Bit-for-Bit Determinism)
cargo run -p tok-cli -- verify-determinism
```

---

## 5. מגבלות ידועות בהיקף הנוכחי (Known Limitations)

בהתאם למשמעת ההנדסית של הפרויקט (אפס טענות שווא ואפס מצגי דמה), להלן המגבלות הידועות בגרסה הנוכחית:

1. **שיבוץ תמונות רסטר (Raster Image Placement):** מנוע הפריסה (`tok-typeset`) ומחולל ה-PDF (`tok-pdf`) תומכים כיום במסגרות טקסט, עיטורים וקטוריים, קווי הפרדה וניהול נכסים בחבילת `.tok`, אך טרם מממשים פריסת תמונות ביטמאפ (TIFF/JPEG/PNG) משובצות בתוך זרימת הטקסט.
2. **תקשורת תהליכים (`tok-ipc` לעומת `tok-cli`):** החבילה `crates/tok-ipc` מממשת את פרוטוקול המסגרות הבינארי (`TOKB`) המיועד למצב שרת רקע מתמיד (Persistent Daemon Mode). בגרסה הנוכחית, תהליך ה-Main של Electron מתקשר עם ליבת ה-Rust באמצעות הפעלה דטרמיניסטית ומבוקרת של תהליך המשנה `tok-cli`.
3. **ייצוא PDF/X-4 וניהול פרופילי ICC חיצוניים:** מחולל ה-PDF מפיק קובצי `PDF/X-1a:2001` ו-`PDF/X-4` עם פקודות צבע `DeviceCMYK` וצבעי `Separation` (Spot Colors) אמיתיים ומילון `OutputIntent` מובנה (ISO Coated v2 / Fogra39), ללא קישור דינמי לספריית C חיצונית דוגמת `LittleCMS`.
4. **גרסת Electron במאגר (`electron@29.4.6`):** המעטפת השולחנית מוקשחת במלואה (`contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`, CSP קפדני וחסימת ניווט), אך שדרוג לגרסת Electron ראשית חדשה יותר (v32+) דורש הורדת בינארי Electron בסביבה מקוונת.

---

## 6. הנחיות הנדסיות ותרומה לקוד (Engineering Guidelines)

1. **דטרמיניזם מוחלט:** כל שינוי ב-`tok-core`, `tok-typeset` או `tok-pdf` חייב לעבור בהצלחה את `tok-cli verify-determinism`. אסור להשתמש ב-`HashMap` בעל סדר אקראי בפלטים משפיעי-פריסה (יש להשתמש ב-`BTreeMap` או במיון מפורש).
2. **אפס `unsafe` ב-Rust:** חל איסור על הוספת בלוקים של `unsafe` בחבילות הליבה.
3. **אפס כפתורי דמה ב-UI:** אין להוסיף כפתורים או תפריטים שמציגים רק הודעת `Toast` ללא פעולה אמיתית המשנה את מודל המסמך, את התצוגה או את הקובץ המיוצא.
4. **בדיקות אמת (No Mock Re-implementations):** על כל בדיקה ב-`test/frontend.test.mjs` לייבא ולהריץ את הקוד האמיתי מתוך `packages/**` או `tok-cli`, ולא לממש פונקציות חיקוי מקומיות בתוך קובץ הבדיקה.

---

## 7. רישוי וזכויות יוצרים (License & Third-Party Notices)

- קוד המקור של **TypesetOK (TOK)** מופץ תחת רישיון **TypesetOK Non-Commercial Copyleft License v1.1 (`TOK-NCCL v1.1`)**.
- זהו רישיון **Source-Available לשימוש לא-מסחרי בלבד** (אינו רישיון Open Source מאושר OSI). שימוש אישי, אקדמי, מחקרי, חינוכי וקהילתי ללא מטרות רווח מותר בכפוף לתנאי הרישיון (ייחוס ו-Copyleft על נגזרות). כל שימוש מסחרי מחייב הסכם רישוי מסחרי נפרד בכתב.
- לנוסח הרישיון המלא ראו: [`LICENSE.md`](./LICENSE.md).
- לפירוט מלא של כלל ספריות הצד-השלישי (Rust ו-Node.js), הגופנים המצורפים (`SIL Open Font License 1.1`), וניתוח התאימות המשפטית ראו: [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md).

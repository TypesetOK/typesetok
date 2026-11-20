<p align="center">
  <img src="assets/icon.png" alt="TypesetOK Icon" width="128" height="128" />
</p>

<h1 align="center">TypesetOK (TOK)</h1>

<p align="center">
  <strong>Professional Desktop Publishing Software for Hebrew, Multi-Text (Mikraot Gedolot &amp; Talmud / Shas), Live Tzurat HaDaf &amp; Pre-Press</strong><br>
  <strong>תוכנת עימוד שולחנית מקצועית לעברית, רב-טקסט (מקראות גדולות וש"ס), צורת הדף חיה וקדם-דפוס</strong><br>
  <em>Version 0.37.0 • Native Rust Core • Tauri v2 Desktop Shell • Full Determinism</em><br>
  <em>גרסה 0.37.0 • ליבת Native Rust • מעטפת Tauri v2 • דטרמיניזם מלא</em>
</p>

<p align="center">
  <a href="#english">English</a> • <a href="#hebrew">עברית</a>
</p>

<p align="center">
  <img src="assets/logo.jpg" alt="TypesetOK Official Logo" width="640" />
</p>

---

<a id="english"></a>

# English

**TypesetOK (TOK)** is a Desktop Publishing (DTP) application built from the ground up for advanced Hebrew typography, bidirectional text (`Bidi` / `RTL`), synchronized multi-stream typesetting (live *Tzurat HaDaf*, *Mikraot Gedolot*, Talmud / *Shas*, *Responsa* / *Shu"t*, and academic editions), and end-to-end deterministic generation of print-ready files (`ISO 15930-1:2001 PDF/X-1a:2001`).

---

## 1. Architecture Overview

The project is organized as a Monorepo that strictly separates the Rust typesetting and document core from the TypeScript presentation and interaction layers:

### End-to-End Deterministic Data Flow

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

> **Mandatory Engineering Principle:** The browser (Chromium) **does not** decide where lines break or where pages split, and the system **does not** use external browser polyfill libraries (such as `@vivliostyle/core` or `Paged.js`). All geometry is computed in Rust (`tok-typeset`), exported as `PageLayoutBox` structures in exact `pt` units, and projected onto a rigid DOM projection (`SpreadCanvas`) or directly into a `PDF/X-1a` file.

---

### 7 Core Rust Crates (`crates/`)

| Crate | Primary Role & Responsibilities |
| :--- | :--- |
| **`tok-core`** | Canonical Document Model (TDM): immutable document tree, `ropey` B-Tree Rope for long texts, `ULID` identifiers, `FractionalIndex`, transaction stack and delta Undo/Redo, multi-layered apparatus configurations (`ApparatusConfig`), typographic style patches, Hebrew gematria engine (including taboo table: ט״ו, ט״ז, ע״ר, ער״ב, ער״ה, רח״צ, שי״ד, נט״ש, תשי״ד and `U+05F3`/`U+05F4` characters), and canonical Hebrew typographic input normalizer ($\text{Base} \rightarrow \text{Shin/Sin Dot} \rightarrow \text{Dagesh/Mapiq} \rightarrow \text{Niqqud} \rightarrow \text{Meteg} \rightarrow \text{Te'amim}$). |
| **`tok-typeset`** | Typesetting & typography engine: dedicated Masoretic and biblical cantillation (*Te'amim*) shaper (`MasoreticShaper`, inverted Nun, Paseq, upper/lower Meteg, and biblical OpenType features), `unicode-bidi` (UAX #9) directionality resolution, pure-Rust font shaping (`rustybuzz` including GPOS `mark`/`mkmk`), global **Knuth-Plass** line-breaking algorithm, three-tier Hebrew justification hierarchy (controlled word spacing $\rightarrow$ letter elongation / *Ahaltaram* $\rightarrow$ micro-tracking $\pm 2\%$), linear constraint solver (**Cassowary**), dynamic width-ratio distribution for rabbinic streams (2, 3, and 4 columns in *Shas* and *Mikraot Gedolot* with exact Recto/Verso tracking), parallel multi-layered footnote layout engine (`FootnotePaginator`), Hebrew drop caps, and polygon/scanline clipping engine for L-shaped commentary wrapping. |
| **`tok-pdf`** | Native pre-press engine (`pdf-writer`): direct generation of `PDF/X-1a:2001` (ISO 15930-1) and PDF/X-4 modes, `DeviceCMYK` color space and spot colors (`Separation`), standard print boxes (`MediaBox`, `CropBox`, `BleedBox`, `TrimBox`), crop & registration marks, font subsetting and `/ToUnicode` CMap tables for Hebrew text search and copy, and a `Preflight` engine. |
| **`tok-storage`** | Dual storage engine: crash-resilient workspace in **SQLite WAL** (`rusqlite`) + atomic saving to `.tok` package files (ZIP archive with **Zstandard** compression containing `manifest.json`, `document.json`, `styles.json`, and SHA-256 verified assets). |
| **`tok-plugin-host`** | Deterministic extension & plugin engine: executes core plugins based on atomic transactions under resource limits (`Fuel` / `Memory` / `Capabilities`), including built-in plugins for stripping double spaces, applying *Dibbur HaMatchil* (opening words) styling in Rashi and Tosafot, and enforcing the Hebrew maqaf (`U+05BE`). |
| **`tok-ipc`** | Binary framing protocol library (`TOKB` Binary Framing) for inter-process command and response messages. |
| **`tok-cli`** | Main system CLI binary, serving both for headless batch processing & CI and as the typesetting and storage bridge for the Electron shell. |

---

### 5 TypeScript UI & Desktop Packages (`packages/`)

| Package | Primary Role & Responsibilities |
| :--- | :--- |
| **`tok-electron`** | Electron Main process and hardened `preload` bridge (`contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`), strict IPC schema validation, external navigation blocking, and `tok-cli` subprocess management. |
| **`tok-ui`** | Full desktop workspace: `SpreadCanvas` component for deterministic `PageLayoutBox` DOM projection, live style and typography sync pipeline across the inspector panel, toolbar, and canvas, direct dragging and locking of text frames on the canvas, adding custom streams (including edge-split unconnected blocks flowing by height), template-aware style filtering, guided template selection screen on document creation with continuous-book recommendation, update & changelog ("What's New") system, hardened `PluginEngine`, and live `Preflight` checks. |
| **`tok-viewer`** | Viewport page virtualization manager (`PageDomVirtualizer`): keeps an active window of only 3 pages in the DOM (`[K-1, K, K+1]`) and represents the rest of the book as lightweight Spacer elements. |
| **`tok-canvas`** | Transparent graphic overlay (`CanvasInteractionOverlay`): bidirectional caret rendering (`Dual Caret` / `RTL Caret`), optimistic instant caret movement (`<16ms`), non-contiguous visual selection in mixed text, and spatial hit-testing. |
| **`tok-story-editor`** | Continuous `Unpaginated Story Editor` for fast content editing without multi-page layout overhead, with lossless bidirectional DOM conversion and full preservation of local style patches and formatting. |

---

## 2. Prerequisites & Setup

### System Requirements
1. **Operating System:** Windows 10/11 (64-bit).
2. **Rust Compiler:** Recent stable version (`rustc` / `cargo`) targeting `x86_64-pc-windows-gnu`.
   - [`.cargo/config.toml`](./.cargo/config.toml) in the repository is pre-configured to use the fast `rust-lld` linker and a local build directory (`C:/Users/USER/AppData/Local/tok_target`) to avoid non-ASCII path issues on Windows. If your Windows username differs from `USER`, update `target-dir` in that file accordingly.
3. **Node.js:** Version `20.0.0` or higher + `npm`.

### Full Installation & Build Steps

```powershell
# 1. Install Node/TypeScript dependencies
npm install

# 2. Build all Rust crates in the workspace (including the CLI binary required by the desktop shell)
cargo build --workspace
cargo build --bin tok-cli

# 3. Type-check and compile the UI and Electron packages
npm run lint
npm run build

# 4. Launch the TypesetOK desktop application in Electron
npm start

# 5. (Optional) Package the desktop bundle and build an installer
npm run package:desktop
npm run package:installer
```

> **Important Note on Locating `tok-cli.exe`:** The Electron shell (`packages/tok-electron/src/main.ts`) searches for `tok-cli.exe` in the following order:
> 1. The `TOK_CLI_PATH` environment variable (if set).
> 2. The target directory configured in `.cargo/config.toml` (`%LOCALAPPDATA%\tok_target\debug\tok-cli.exe` or `release\tok-cli.exe`).
> 3. `target/debug/tok-cli.exe` or `target/release/tok-cli.exe` at the repository root.
> Therefore, you must run `cargo build --bin tok-cli` before running `npm start`.

---

## 3. CLI Binary Usage (`tok-cli`)

The `tok-cli` binary allows running all core operations directly from the command line or CI/CD pipelines:

| Subcommand | Syntax & Parameters | Description |
| :--- | :--- | :--- |
| **`typeset-document`** | `tok-cli typeset-document <input.json> [output.layout.json]` | Full typesetting of a JSON document (including Knuth-Plass, Cassowary, and Co-Pagination), returning a detailed `PageLayoutBox` tree in `pt` units (including glyphs, lines, frames, footnotes, and Preflight report). |
| **`incremental-typeset`** | `tok-cli incremental-typeset <input.json> <dirty_paragraph_index> [output.json]` | Convergence-based incremental typesetting (`Convergence Paginator`) starting from the modified paragraph until the convergence point. |
| **`diagnose-recompute`** | `tok-cli diagnose-recompute <base.json> <edited.json> <dirty_idx>` | Compares and diagnoses how many pages were recomputed versus fetched from cache during an incremental edit. |
| **`render-pdf`** | `tok-cli render-pdf <input.json\|.tok> <output.pdf>` | Generates a print-standard `PDF/X-1a:2001` file directly from the document (including CMYK, print boxes, and font subsetting). |
| **`render-html`** | `tok-cli render-html <input.json\|.tok> <output.html>` | Generates a standalone pre-paginated HTML/CSS file for preview and inspection. |
| **`save-package`** | `tok-cli save-package <input.json> <output.tok>` | Atomically packs and saves a JSON document into the `.tok` package format (ZIP + Zstandard + SHA-256 manifest). |
| **`open-package`** | `tok-cli open-package <input.tok> [output.json]` | Opens a `.tok` file, verifies package and asset integrity, and restores the canonical document JSON. |
| **`inspect-package`** | `tok-cli inspect-package <input.tok>` | Verifies `.tok` archive integrity and prints a metadata summary (title, section count, paragraphs, and schema version). |
| **`run-plugins`** | `tok-cli run-plugins <input.json> <plugin_id[,plugin_id...]> [output.json]` | Runs sandboxed deterministic core plugins (`builtin.strip-double-spaces`, `builtin.apply-dibbur-hamatchil`, `builtin.enforce-hebrew-maqaf`) on the document. |
| **`verify-determinism`** | `tok-cli verify-determinism <input.json> [iterations]` | Runs typesetting and PDF/HTML generation multiple times and verifies bit-for-bit identity (SHA-256 Bit-for-Bit Determinism). |
| **`benchmark-typeset`** | `tok-cli benchmark-typeset <paragraphs_count>` | Runs a stress and performance benchmark on a large synthetic document and prints layout timing and per-page speed metrics. |

---

## 4. Testing & Verification

The project includes a multi-layered test suite verifying the Rust core, the TypeScript packages, and end-to-end integration:

```powershell
# 1. Run all unit and integration tests for the Rust core (all workspace crates)
cargo test --workspace

# 2. Run strict TypeScript compilation and type checking across all 5 packages
npm run lint

# 3. Run the frontend, gematria, virtualization, Canvas, Story Editor, updater, and security test suite
npm test

# 4. Verify Bit-for-Bit Determinism
cargo run -p tok-cli -- verify-determinism
```

---

## 5. Known Limitations

In accordance with the project's engineering discipline (zero false claims and zero mock implementations), the following are the known limitations in the current release:

1. **Raster Image Placement:** The layout engine (`tok-typeset`) and PDF generator (`tok-pdf`) currently support text frames, vector ornaments, divider rules, and asset management in `.tok` packages, but do not yet lay out bitmap images (TIFF/JPEG/PNG) embedded inline within the text flow.
2. **Inter-Process Communication (`tok-ipc` vs. `tok-cli`):** The `crates/tok-ipc` crate implements the binary framing protocol (`TOKB`) intended for Persistent Daemon Mode. In the current version, the Electron Main process communicates with the Rust core via deterministic, controlled invocations of the `tok-cli` subprocess.
3. **PDF/X-4 Export & External ICC Profile Management:** The PDF generator produces `PDF/X-1a:2001` and `PDF/X-4` files with real `DeviceCMYK` and `Separation` (Spot Colors) operators and a built-in `OutputIntent` dictionary (ISO Coated v2 / Fogra39), without dynamically linking an external C library such as `LittleCMS`.
4. **Repository Electron Version (`electron@29.4.6`):** The desktop shell is fully hardened (`contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`, strict CSP, and navigation blocking), though upgrading to a newer major Electron version (v32+) requires downloading the Electron binary in an online environment.

---

## 6. Engineering Guidelines & Contributing

For contribution guidelines and community standards, see [`CONTRIBUTING.md`](./CONTRIBUTING.md) and [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md).

1. **Absolute Determinism:** Every change in `tok-core`, `tok-typeset`, or `tok-pdf` must pass `tok-cli verify-determinism`. Never use a randomized-order `HashMap` in layout-affecting outputs (use `BTreeMap` or explicit sorting).
2. **Zero `unsafe` in Rust:** Adding `unsafe` blocks in the core crates is prohibited.
3. **Zero Mock Buttons in the UI:** Do not add buttons or menus that only show a `Toast` notification without performing a real action that modifies the document model, the view, or the exported file.
4. **Real Tests Only (No Mock Re-implementations):** Every test in `test/frontend.test.mjs` must import and execute the actual code from `packages/**` or `tok-cli`, rather than implementing local mock functions inside the test file.

---

## 7. License & Third-Party Notices

- The source code of **TypesetOK (TOK)** is distributed under the **TypesetOK Non-Commercial Copyleft License v1.2 (`TOK-NCCL v1.2`)**.
- This is a **Source-Available Non-Commercial** license (not an OSI-approved Open Source license). Subject to the license terms (attribution, full Copyleft on derivatives, and an automatic, irrevocable grant to the upstream project for code in forks), it permits:
  - Personal, academic, research, educational, and community non-profit use;
  - Typesetting and distribution **At-Cost only** — covering actual direct expenses only, with no profit, marketing margin, or typesetting fee;
  - **Self-Authored Book** — an individual personally typesetting a book of which they are the original author/owner for sale (professional typesetters, print shops, typesetting institutes, commercial publishers, and any circumvention attempts are prohibited without a commercial license).
- Any other commercial use (including internal commercial evaluation) requires a separate written commercial license agreement through the project's official channel.
- For the full, legally binding license text (the English text governs in the event of any conflict), see [`LICENSE.md`](./LICENSE.md).
- For a complete breakdown of all third-party libraries (Rust and Node.js), bundled fonts (`SIL Open Font License 1.1`), and legal compatibility analysis, see [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md).

---

<a id="hebrew"></a>

# עברית

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
| **`tok-typeset`** | מנוע העימוד והטיפוגרפיה: שייפר מסורה וטעמי מקרא ייעודי (`MasoreticShaper`, נון הפוכה, פסיק, מתג עליון ותחתון ותכונות OpenType מקראיות), פירוק כיווניות `unicode-bidi` (UAX #9), עיצוב גופנים ב-Rust טהור (`rustybuzz` כולל GPOS `mark`/`mkmk`), אלגוריתם שבירת שורות גלובלי **Knuth-Plass**, מדרג יישור עברי תלת-שלבי (רווחי מילים מבוקרים $\rightarrow$ מתיחת אותיות אהלתר"ם $\rightarrow$ מיקרו-טרקינג $\pm 2\%$), פותר אילוצים ליניארי (**Cassowary**), חלוקת יחסי רוחב דינמיים לתזרימים רבניים (2, 3 ו-4 טורים בש"ס ומקראות גדולות תוך מעקב Recto/Verso מדויק), מנוע פריסת הערות שוליים רב-שכבתי מקבילי (`FootnotePaginator`), אותיות פתיחה עבריות (Drop Caps), ומנוע מצולעים וחיתוך Scanline לספיגת פירושים בצורת L. |
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
   - הקובץ [`.cargo/config.toml`](./.cargo/config.toml) במאגר מוגדר מראש לשימוש בלינקר המהיר `rust-lld` ולתיקיית הידור מקומית (`C:/Users/USER/AppData/Local/tok_target`) למניעת בעיות נתיבים בעברית ב-Windows. אם שם המשתמש שלך ב-Windows שונה מ-`USER`, עדכן את `target-dir` בקובץ זה בהתאם.
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

# 5. (אופציונלי) אריזת גרסה שולחנית ובניית קובץ התקנה (Installer)
npm run package:desktop
npm run package:installer
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

---

## 4. בדיקות ואימות מערכת (Testing & Verification)

הפרויקט כולל מערך בדיקות רב-שכבתי המאמת את ליבת ה-Rust, את חבילות ה-TypeScript ואת האינטגרציה מקצה לקצה:

```powershell
# 1. הרצת כל בדיקות היחידה והאינטגרציה של ליבת ה-Rust
cargo test --workspace

# 2. בדיקת קומפילציה וטיפוסים קפדנית בכל 5 חבילות ה-TypeScript
npm run lint

# 3. הרצת חבילת הבדיקות של ה-Frontend, הגימטריה, הווירטואליזציה, ה-Canvas, ה-Story Editor, מערכת העדכונים והאבטחה
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

<div align="center">

<img src="assets/logo.jpg" alt="TypesetOK Logo" width="160" style="border-radius: 16px; margin-bottom: 12px;" />

# TypesetOK (TOK)
### תוכנת עימוד שולחנית מקצועית בקוד פתוח | Open-Source Professional Desktop Publishing (DTP) System

[![CI Build](https://github.com/TypesetOK/typesetok/actions/workflows/ci.yml/badge.svg)](https://github.com/TypesetOK/typesetok/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/Version-0.9.1-blue.svg)]()
[![Tests](https://img.shields.io/badge/Tests-239%2F239%20Passing-brightgreen.svg)]()
[![Electron](https://img.shields.io/badge/Electron-29.4%2B-blue.svg?logo=electron)](https://www.electronjs.org)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Standard](https://img.shields.io/badge/Standard-ת"י%206100%20(SI%206100)-blue.svg)]()
[![Pre-Press](https://img.shields.io/badge/PDF%2FX--1a%20%7C%20PDF%2FX--4-ISO%2015930-purple.svg)]()
[![License](https://img.shields.io/badge/License-TOK--NCCL%20v1.0-blue.svg)](LICENSE.md)

<p align="center">
  <b>[ <a href="#-עברית">עברית</a> | <a href="#-english">English</a> ]</b>
</p>

</div>

---

## 🇮🇱 עברית

### 📖 סקירה כללית
**TypesetOK (TOK)** היא מערכת עימוד ופרסום שולחני (Desktop Publishing - DTP) מודרנית בקוד פתוח, שנבנתה מן המסד עבור טיפוגרפיה עברית מתקדמת, ספרי קודש (ש"ס, מקראות גדולות, שו"ת), תמיכה רב-תזרימית וצינור קדם-דפוס נייטיב מלא.

עולם העימוד המקצועי בעברית נשען מזה עשרות שנים על תוכנות מונוליתיות קנייניות ישנות (כדוגמת "תג" ו-Adobe InDesign). מערכות אלו מתקשות להתמודד עם טקסטים ענקיים בני אלפי עמודים, סובלות מנעילת ממשק (UI Freezing), ואינן מספקות פתרון מודרני לתזרימי טקסט מרובים וסנכרון עמודים דינמי. TypesetOK מפרידה קפדנית בין ליבת העימוד העצמאית ב-Rust המשיגה ביצועים גבוהים לבין מעטפת המשתמש.

---

### 🏛️ עמודי התווך הארכיטקטוניים

```mermaid
graph TD
    subgraph UI_Shell [מעטפת שולחן עבודה - TypeScript & Electron]
        Toolbar[סרגל כלים טיפוגרפי ופאנלים]
        Virtualizer[וירטואליזציית 3 עמודים פעילים - DOM Virtualizer]
        CanvasOverlay[שכבת כיסוי שקופה - Canvas Overlay & Caret]
        StoryEditor[עורך סיפור רציף - Unpaginated Story Editor]
    end

    subgraph Binary_Bridge [גשר תקשורת בינארי - FlatBuffers / Framed IPC]
        SharedMem[Framed Length-Prefixed Binary Protocol]
    end

    subgraph Rust_Core [ליבת העימוד והדפוס - Rust Core Engine]
        TDM[מודל מסמך סמנטי & טרנזקציות אטומיות - TDM AST]
        Normalizer[נרמול תקן ישראלי ת"י 6100 & מנוע גימטריה]
        Typesetter[מעמד Knuth-Plass + HarfBuzz + יישור אהלתר"ם]
        MultiFlow[פותר אילוצים רב-תזרימי - Talmud Solver]
        Storage[סביבת עבודה ACID WAL + ארכיב ZIP אטומי .tok]
        Prepress[מנוע PDF/X-1a נייטיב & מחולל Pre-paginated HTML]
        PluginHost[מארח הרחבות מבודד - GREP & מגן שמות קדושים]
    end

    UI_Shell <--> Binary_Bridge
    Binary_Bridge <--> Rust_Core
```

#### 1. מודל מסמך סמנטי וטרנזקציות (`tok-core`)
- **AST סמנטי וזיהוי ב-ULID:** מודל מסמך מבוסס צמתים סמנטיים (`DocumentRoot`, `SectionNode`, `Flow`, `ParagraphNode`) עם מזהים ייחודיים בני 128 סיביות.
- **אינדוקס שברירי ב-$O(1)$ (`FractionalIndex`):** הכנסת פסקאות ושורות חדשות בין כל שתי נקודות קיימות ללא מספור מחדש של המסמך.
- **טרנזקציות אטומיות ו-Rollback אוטומטי:** כל פעולה מפיקה דלתא נגדית מדויקת לתמיכה ב-Undo/Redo בלתי מוגבל, עם מנגנון ביטול וגלגול לאחור (Rollback) אוטומטי במקרה של שגיאה.

#### 2. טיפוגרפיה, ניקוד ועימוד עברי (`tok-typeset`)
- **נרמול קפדני לפי ת"י 6100 (SI 6100):** אכיפת סדר יוניקוד דטרמיניסטי: `אות בסיס ← נקודת שין/שין ← דגש/מפיק ← ניקוד ← מתג ← טעמי מקרא`.
- **מנוע שבירת שורות Knuth-Plass:** אופטימיזציה דינמית למזעור פגמים (Demerits) לאורך הפסקה, מניעת שורות רפויות ויתומות.
- **מרווחי פסקאות ו-Margin Collapsing:** תמיכה מלאה ב-`space_before` ו-`space_after` ברמת הפסקה עם קריסת מרווחים (Margin Collapsing) תקנית בין פסקאות, איפוס מרווח בראש עמוד, מניעת גלישת רווח במעבר עמודים, וקריסה חכמה של פסקאות ריקות.
- **יישור עברי תלת-שלבי (3-Tier Hebrew Justification):**
  1. *רווחי מילים (Tier 1):* מתיחה מבוקרת (80% עד 130%).
  2. *אותיות התפשטות אהלתר"ם (Tier 2):* זיהוי אותיות מתרחבות (א, ה, ל, ת, ר, ם) והרחבתן הטיפוגרפית.
  3. *מיקרו-טרקינג (Tier 3):* התאמת מרווח גליפים עדינה ($\pm 2\%$ em).
- **גימטריה עברית דטרמיניסטית:** אכיפת גרש תקני `U+05F3` וגרשיים `U+05F4`, ומנגנון המרות טאבו ושמות קודש (15 ← ט״ו, 16 ← ט״ז, 270 ← ע״ר, 272 ← ער״ב, 275 ← ער״ה, 298 ← חר״צ, 304 ← ד״ש, 344 ← שד״מ, 359 ← נט״ש, 698 ← תרח״צ, 744 ← תשד״מ).
- **פותר אילוצים רב-תזרימי (Multi-Flow Solver):** עימוד עמוד ש"ס ומקראות גדולות תוך עמידה בחוקי גלישה וסנכרון פסקאות חוצה-עמודים.

#### 3. מנוע קדם-דפוס נייטיב (`tok-pdf`)
- **תאימות ISO 15930 (PDF/X-1a:2001 ו-PDF/X-4):** שחור `100% K` (DeviceCMYK), תמיכה בצבעי ספוט (Spot/Pantone), והזרקת פרופילי Fogra 39 / Fogra 51.
- **דחיסת זרמים FlateDecode:** דחיסת zlib FlateDecode מובנית לזרמי תוכן עמודים (`/Contents`), גופני TrueType (`/FontFile2`) וטבלאות מיפוי `/ToUnicode` לפלט PDF קומפקטי.
- **מטא-נתונים ותאריכים דטרמיניסטיים:** חילוץ `CreationDate` ו-`ModDate` מתוך ה-Manifest של המסמך בהתאם לתקן, תוך שמירה מוחלטת על שחזור ביט-אחר-ביט דטרמיניסטי.
- **תיבות דפוס מקצועיות וסימני חיתוך:** יצירת MediaBox, BleedBox (3 מ"מ), TrimBox, CropBox וציור וקטורי של צלבי רישום וסימני חיתוך.
- **טבלאות `/ToUnicode`:** שיבוץ טבלאות מיפוי PostScript המבטיחות חיפוש, הדגשה והעתקת טקסט מנוקד ללא שיבושים.

#### 4. אחסון היברידי ועמידות קריסות (`tok-storage`)
- **סביבת עבודה שוטפת (Workspace):** מסד נתונים פנימי ACID עם Write-Ahead Logging (WAL) לשמירה רציפה ברקע ועמידות בפני נפילות מתח.
- **פורמט חבילה רשמי (`.tok`):** ארכיב ZIP תקני מוגן מפני Zip Slip ונגועים, המכיל מניפסט, עץ מסמך ונכסים מוטמעים.
- **שמירה אטומית מוגנת (Atomic Safe-Save):** כתיבה לקובץ זמני, סנכרון חומרה מלא (`fsync`), והחלפה אטומית.
- **מנהל רב-מסמכים (`.tokbook`):** סנכרון סגנונות מסטר, רציפות מספור עמודים עברי ומפתח עניינים (TOC) מאוחד.

#### 5. מערכת הרחבות ותוספים מאובטחת (`tok-plugin-host` & Plugin Engine)
- **API רשמי עשיר:** עריכת פסקאות אטומית (`tok.document`), כלי עזר לטיפוגרפיה עברית וגימטריה ב-0 השהיה (`tok.hebrew`), סרגל פקודות וטוסטים (`tok.ui`), ניווט דפים וזום (`tok.canvas`).
- **ארגז חול ומודל הרשאות מפורש:** הצללת IPC, חסימת תקשורת רשת לא מורשית, גבול בידוד שגיאות (Error Boundary).
- **תאימות מלאה לעמדות סייפר ומצב בטוח:** זיהוי אוטומטי של עמדות סייפר תורניות, פעולה מקומית מלאה (Offline), שמירה במחשבים מוקפאים, ומתג השבתת חירום / Safe Mode (`--safe-mode`).
- 📘 **מדריך מקיף ודוגמאות קוד:** ראו [מדריך ומפרט ה-API המלא לפיתוח תוספים (docs/PLUGINS_API.md)](docs/PLUGINS_API.md).

---

## 🇺🇸 English

### 📖 Overview
**TypesetOK (TOK)** is a modern, open-source Desktop Publishing (DTP) system written from the ground up in Rust for advanced Hebrew typography, sacred text typesetting (Talmud, Mikraot Gedolot, Responsa), multi-flow page rendering, and a native pre-press PDF engine.

Traditional Hebrew typesetting relies on legacy monolithic systems. TypesetOK separates the high-performance Rust typesetting core from the desktop UI shell, ensuring fluid 120 FPS frame rates, instant typing response (<16ms caret latency), and zero-lockup handling of 1,000+ page manuscripts.

---

### 🗺️ Monorepo Architecture Map

```
typesetok/
├── .github/
│   └── workflows/                  # GitHub Actions CI (fmt, clippy, matrix tests, audit)
├── Cargo.toml                       # Rust Workspace definition (7 crates)
├── package.json                     # TypeScript / Electron monorepo definition
│
├── crates/                          # Rust Engine Crates (100% Tested)
│   ├── tok-core/                    # Semantic Document AST, ULID, Fractional Index, SI 6100, Transactions
│   ├── tok-typeset/                 # Knuth-Plass Line Breaking, Paragraph Margins, 3-Tier Hebrew Justification, Bidi
│   ├── tok-pdf/                     # ISO PDF/X Pre-Press Engine, FlateDecode Compression, Deterministic Dates, HTML Export
│   ├── tok-storage/                 # ACID WAL Storage (redb), Atomic Safe-Save (.tok), Multi-Doc (.tokbook)
│   ├── tok-ipc/                     # Binary Framed IPC Schema & Geometry Hit-Testing
│   ├── tok-plugin-host/             # Sandboxed Extension Host (catch_unwind), Holy Name Guardian, GREP
│   └── tok-cli/                     # Headless Preflight, Rendering & Determinism Verification Binary
│
└── packages/                        # TypeScript / Electron Desktop Shell
    ├── tok-electron/                # Main Process, Native Menus, Window Management
    ├── tok-viewer/                  # 3-Active Page DOM Virtualizer (120 FPS)
    ├── tok-canvas/                  # Transparent Canvas Overlay & Caret (<16ms)
    ├── tok-story-editor/            # Continuous Unpaginated Story Editor
    └── tok-ui/                      # Workbench Application (Toolbar, Panels, Pages)
```

---

## 🚀 Quickstart & Usage

### Prerequisites
- [Rust 1.85+](https://www.rust-lang.org) (with Cargo)
- [Node.js 20+](https://nodejs.org) (for the Electron UI shell)

### Build Rust Engine Workspace
```bash
# Clone the repository
git clone https://github.com/TypesetOK/typesetok.git
cd typesetok

# Build all workspace crates
cargo build --workspace

# Run full test suite (175 passing tests)
cargo test --workspace
```

### 📥 הרצה ישירה ללא התקנה מוקדמת (Portable Desktop Binary)
למשתמשי Windows המעוניינים להריץ את התוכנה ישירות ללא צורך בהתקנת Node.js או Rust:
1. הורידו את `TypesetOK-v0.9.1-windows-desktop-app.zip` (או את אשף ההתקנה הרשמי `TypesetOK-v0.9.1-Setup.exe`) מתוך דף ה-[GitHub Releases](https://github.com/TypesetOK/typesetok/releases/tag/v0.9.1).
2. חלצו את קובץ ה-ZIP.
3. הפעילו ישירות בלחיצה כפולה את `TypesetOK.exe`.
(התוכנה מגיעה כחבילת Standalone עצמאית הכוללת את מעטפת ה-UI, מנוע ה-Electron, ובינארי ה-CLI המובנה).

### Launch Desktop Workbench (Development Environment)
```bash
# Install frontend dependencies
npm install

# Build all TypeScript packages and bundle UI
npm run build

# Run frontend test suite (64 passing tests across 28 suites)
npm test

# Build Standalone Desktop App bundle
npm run package:desktop

# Launch TypesetOK Desktop Application
npm start
```

### CLI Headless Operations (`tok-cli`)

```bash
# Export ISO PDF/X-1a print file with crop marks and DeviceCMYK:
cargo run -p tok-cli -- render-pdf --demo output.pdf

# Export pre-paginated HTML projection:
cargo run -p tok-cli -- render-html --demo output.html

# Run 1,000-page stress benchmark & cascade measurement:
cargo run -p tok-cli -- benchmark-typeset --pages 1000

# Verify bit-for-bit output determinism (Pass 1 SHA-256 == Pass 2 SHA-256):
cargo run -p tok-cli -- verify-determinism

# Inspect .tok package archive contents:
cargo run -p tok-cli -- inspect-package document.tok
```

---

## 📊 Verification & Benchmark Status (v0.9.1)

| Metric | Architectural Target | Actual Result | Status |
| :--- | :--- | :--- | :--- |
| **Rust Engine Tests** | 100% Pass Across All 7 Crates | **175 / 175 Tests Passing** | **PASSED** |
| **Frontend Shell Tests** | Gematria, Virtualizer, Tokens & Interaction Triad | **60 / 60 Tests Passing** | **PASSED** |
| **Total Automated Tests** | Rust + TypeScript CI Matrix | **235 / 235 Tests Passing** | **PASSED** |
| **Font Weight Support** | Bold (700) and Regular (400) Shaping & Styles | **Embedded OFL Bold Fonts & UI Wired** | **PASSED** |
| **Stream Compression** | zlib FlateDecode for Contents, Fonts & CMaps | **Enabled (Smaller PDF Output)** | **PASSED** |
| **Deterministic Dates** | Manifest Timestamps in PDF/X Metadata | **Pass 1 Hash == Pass 2 Hash** | **PASSED** |
| **Margin Collapsing** | Paragraph Space Before / After Collapsing | **Full Top/Bottom/Split Collapsing** | **PASSED** |
| **TrueType Font Subsetting** | TrueType OpenType Subsetting in PDF | **Identity-H & /ToUnicode** | **PASSED** |
| **Spatial Hit-Testing** | Sub-pixel glyph snap & selection range | **100% RTL & Bidi Coordinated** | **PASSED** |
| **Talmud Tzurat HaDaf** | L-Shape expansion & Recto/Verso spreads | **Tested & Validated** | **PASSED** |
| **Clippy Linter** | 0 Warnings with `-D warnings` | **0 Warnings (Clean)** | **PASSED** |
| **Formatting** | `cargo fmt --check` Compliant | **100% Formatted** | **PASSED** |
| **Bit-for-Bit Determinism** | Identical SHA-256 across runs | `Pass 1 SHA == Pass 2 SHA` | **PASSED** |
| **Massive Doc Benchmark** | 1,000 Pages Full Vocalization | **4,000 Paragraphs / 6,400 Lines** | **PASSED** |
| **Pre-press Standard** | ISO PDF/X-1a with Fogra39 | **100% DeviceCMYK Black & Marks** | **PASSED** |
| **DOM Virtualizer** | 3-Page Active Window `[K-1,K,K+1]` | **Active Window Verified** | **PASSED** |
| **Electron Shell** | Native Menus, Preload IPC Bridge | **Verified & Running** | **PASSED** |

---

## 🤝 Code of Conduct & Contributing

We warmly welcome contributions from developers, typographers, and Hebrew DTP experts!
- Please read our [**Contributing Guide (מדריך לתורמים)**](CONTRIBUTING.md) for full instructions on setup, coding standards, and pull request workflows.
- View the project architecture and planned milestones in our [**Roadmap & Code Map (מפת קוד ויעדים)**](ROADMAP.md).
- Review our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

---

## 📄 License

Project **TypesetOK (TOK)** is licensed under the:
**[TypesetOK Source-Available Non-Commercial Copyleft License (TOK-NCCL v1.0)](LICENSE.md)**

* 🚫 **Non-Commercial Use Only:** Commercial use, commercial DTP services, or paid book publishing require a separate commercial license from the author.
* 🔄 **Copyleft Requirement:** All forks, modifications, or derivative plugins must remain fully open-source under the exact same license.
* 🛡️ **No Warranty:** Software is provided "AS IS" without warranty of any kind.

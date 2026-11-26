# Project: TypesetOK (תוכנת עימוד שולחנית מקצועית)

## Architecture
TypesetOK הינה תוכנת DTP (עימוד וסדר דפוס) שולחנית אמיתית המיועדת לספרות עברית, ספרי קודש וטקסטים רב-זרימתיים מורכבים (כדוגמת דפי תלמוד ומפרשים).
המערכת מבוססת על הפרדה מוחלטת בין מנוע הליבה (Core Engine) למעטפת הממשק (Desktop UI):
1. **ליבת ה-Rust (Core Engine — 9 חבילות):**
   - `tok-core`: מודל מסמך היררכי קפדני (`DocumentRoot`, `Flow`, `ParagraphNode`), אלגוריתמי עברית קנוניים, גימטריה, מספור דפי קודש ומחסנית Undo/Redo טרנזקציונית.
   - `tok-typeset`: מנוע טיפוגרפי ועימוד מתקדם מבוסס Knuth-Plass Dynamic Programming, שלושת עמודי התווך של היישור העברי (רווחי מילים, אותיות אהלתר"ם מתרחבות ו-micro-tracking), פותר אילוצים גאומטריים Cassowary Simplex, ועימוד רב-זרימות קו-פגינציה (Co-Pagination) עם קפיצים אנכיים ועיגון פירושים.
   - `tok-pdf`: הפקת דפוס מקצועית לפי תקני ISO PDF/X-1a ו-PDF/X-4, הגדרת תיבות עמוד (MediaBox, BleedBox, TrimBox), גזירת גופנים (Subsetting), קידוד תמונות PNG/JPEG (`image_xobject`) וטבלאות יוניקוד CMap.
   - `tok-storage`: חבילות `.tok` אטומיות מוגנות Zip-Bomb, וניהול נתונים ACID באמצעות `redb`.
   - `tok-plugin-host`: מארח תוספי ליבה מבודד עם בקרת הרשאות (`PluginCapability`).
   - `tok-engine-service`: שכבת תיאום מאוחדת המשותפת ל-`tok-desktop` ול-`tok-cli`.
   - `tok-ipc`: פרוטוקול מסגור ו-Worker לתקשורת בין-תהליכית.
   - `tok-cli`: ממשק שורת הפקודה (`typeset-document`, `render-pdf`, `render-html`, `save-package`, `open-package`, `verify-determinism`, `benchmark-typeset`, `worker`).
   - `tok-desktop`: מעטפת שולחן העבודה הראשית ב-Tauri v2 הקוראת ישירות ל-`tok-engine-service` בתוך אותו תהליך.
2. **מעטפת הממשק וה-Renderer (Desktop UI — 4 חבילות TypeScript):**
   - `packages/tok-ui`: ממשק שולחני מהיר (Vanilla TypeScript נקי מ-React/Vue), רינדור נאמן ב-DOM של תוצאות הפריסה מ-Rust במיקום אבסולוטי מדויק בנקודות (pt), ומתאם `tauriHostAdapter.ts` מול Tauri v2.
   - `packages/tok-story-editor`: עורך טקסט עשיר בסגנון Story Editor של InDesign, היסטוריית שינויים, וחיפוש/החלפה עמידי ניקוד.
   - `packages/tok-viewer`: וירטואליזציית עמודים (`PageDomVirtualizer`).
   - `packages/tok-canvas`: שכבת כיסוי גרפית (`CanvasInteractionOverlay`).
3. **ערוצי התקשורת (IPC & Data Flow):**
   - מעבר נתונים חד-כיווני ודטרמיניסטי: UI Event -> State Update -> Tauri v2 IPC (`tok-desktop` -> `tok-engine-service`) או `tok-cli` (ב-Electron) -> `PageLayoutBox` (pt) -> Render to Screen / Output PDF.

---

## Feature Inventory
כל התכונות והממצאים שאותרו בשלב הסקר ממופים להלן עם מילסטון ייעודי:

| # | Feature / Finding | Description | Milestone | Source |
|---|-------------------|-------------|-----------|--------|
| 1 | Linker Config for Windows GNU | Add `.cargo/config.toml` configuring `-C linker=rust-lld` for Windows GNU toolchain | M1 | Survey (Rust) |
| 2 | License field in package.json | Add `"license": "SEE LICENSE IN LICENSE.md"` to all root and package `package.json` files | M1 | Survey (Security) |
| 3 | Version Synchronization | Align version numbers across `ROADMAP.md` (v0.8.0 -> v0.30.4) and docs | M1 | Survey (Security) |
| 4 | XSS Vulnerability in SpreadCanvas | Escape `p.text` using `escapeHtml` in `SpreadCanvas.ts` (lines 270, 573) before inserting into `innerHTML` | M2 | Survey (Security) |
| 5 | PluginEngine Sandbox Escape | Harden plugin execution in `PluginEngine.ts` to prevent access to `window.tokIpc` | M2 | Survey (Security) |
| 6 | Electron Dependency Vulnerabilities | Remediate High CVEs in Electron and extract-zip via package update/configuration | M2 | Survey (Security) |
| 7 | Keystroke Loss Race Condition | Flush `StoryEditor` DOM state before `runExport` and `saveDocumentToFile` | M3 | Survey (Frontend) |
| 8 | Multi-Select Frame Alignment | Implement genuine frame alignment calculations or disable unhandled buttons | M3 | Survey (Frontend) |
| 9 | Layer Visibility and Lock Controls | Connect StructureBar layer eye/lock controls to actual DOM/frame state or disable | M3 | Survey (Frontend) |
| 10| Removal of Fake "SI 6100" Claims | Replace fake "ת״י 6100" claims in UI strings and normalizer with accurate Unicode normalization descriptions | M3 | Survey (Frontend/Sec) |
| 11| Image Inspector Controls Cleanup | Remove or disable image frame controls in ContextualInspector since images are not supported | M3 | Survey (Frontend) |
| 12| Sacred Names Protection Integration | Wire `DivineNameShield` / sacred names protection into core text handling/normalizer | M3 | Survey (Rust) |
| 13| Dead Code: tok-canvas cleanup | Safely retire/archive unused `packages/tok-canvas` (`CanvasInteractionOverlay`) | M4 | Survey (Frontend/Sec) |
| 14| Dead Code: tok-ipc & tok-plugin-host | Safely document/retire or consolidate unused crates (`crates/tok-ipc`, `crates/tok-plugin-host`) | M4 | Survey (Rust/Sec) |
| 15| Long Document Virtualization | Connect `PageDomVirtualizer` to `SpreadCanvas` or optimize DOM allocation for long documents | M4 | Survey (Frontend) |
| 16| Cleanup Fake Tests in frontend.test.mjs | Replace in-test algorithm re-implementations with imports from actual modules | M4 | Survey (Frontend) |
| 17| Remove Phantom @vivliostyle/core refs | Clean up phantom references to @vivliostyle/core in architecture documents | M4 | Survey (Security) |
| 18| README.md Comprehensive Rewrite | Complete rewrite with accurate setup, architecture, usage, testing, limitations, and Hebrew guidelines | M5 | Survey / R5 |
| 19| License & Dependency Notice Audit | Complete license audit, dependency notices, and copyright headers | M5 | Survey / R5 |
| 20| E2E Testing Suite (Tiers 1-4) | Comprehensive opaque-box test suite across 4 tiers derived from requirements | E2E-Track | Dual Track |
| 21| 100% E2E Pass & Adversarial Hardening | Pass 100% E2E test suite + Tier 5 adversarial testing & verification | M6 (Final) | Dual Track |
| 22| Bilingual What's New System | Side drawer for minor updates, popup dialog for major updates, unread bell indicator, and full Hebrew/English changelog | M6 | Feature Track |

---

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Toolchain, Linker & Metadata Baseline | `.cargo/config.toml`, `package.json` license fields, version alignment | none | DONE (`.cargo/config.toml`, package licenses, 71/71 E2E pass) |
| M2 | Security Hardening & Vulnerability Remediation | XSS prevention in `SpreadCanvas.ts`, `PluginEngine` sandbox hardening, Electron security review | M1 | DONE (`escapeHtml` XSS prevention, recursive `sandboxedGlobal` Proxy, Electron `^29.4.6`) |
| M3 | Functional Bug Fixes & Integrity Alignment | Keystroke loss fix, frame alignment, layer controls, remove fake standard claims, sacred names | M1 | DONE (`getStory(true)` flush, `frameAlignment.ts`, layer visibility/lock, canonical Hebrew normalization, `DivineNameShieldPlugin` & `HebrewGrepAcronymPlugin`) |
| M4 | Dead Code Removal & Architectural Refactoring | `tok-canvas` cleanup, orphaned crates resolution, DOM virtualization, test suite cleanup | M2, M3 | DONE (`PageDomVirtualizer` wired to `SpreadCanvas`, `tok-plugin-host` wired to `tok-cli`, `frontend.test.mjs` mock cleanup) |
| M5 | Documentation, License Audit & Hebrew Report | README.md rewrite, license audit, copyright notices, Hebrew summary report | M4 | DONE (`README.md` rewritten, `THIRD_PARTY_NOTICES.md` created, `@vivliostyle/core` phantom refs removed) |
| E2E | E2E Testing Track | Requirement-driven opaque-box test harness & Tiers 1-4 test suite | M1 | DONE (`TEST_READY.md` published, 71/71 tests passing) |
| M6 | Final Verification & Adversarial Coverage Hardening | 100% E2E test pass + full Rust/TS/Integration verification | E2E, M5 | DONE (430 Rust tests + 198 npm tests + 71 E2E tests = 699/699 passing) |

---

## Interface Contracts

### 1. `tok-cli` ↔ `tok-electron`
- **Invocation**: `child_process.spawn(cliPath, args, { windowsHide: true })`
- **Commands**:
  - `typeset-document <input.json> <output.json>`:
    - Input: JSON serialization of `DocumentRoot`
    - Output: JSON serialization of `Vec<PageLayoutBox>` (pt coordinates, lines, styles)
    - Exit code: 0 on success, non-zero on error with stderr description
  - `render-pdf <input.json> <output.pdf>`:
    - Input: JSON serialization of `DocumentRoot`
    - Output: ISO PDF/X compliant binary file
  - `save-package <input.json> <output.tok>`:
    - Input: JSON `DocumentRoot`
    - Output: Atomic ZIP package `.tok` with CRC32 and SHA-256
  - `open-package <input.tok> <output.json>`:
    - Input: `.tok` package
    - Output: JSON `DocumentRoot`
- **Error Handling**: Non-zero exit code triggers an explicit UI error notification; fallback layout displays an explicit warning banner: `⚠️ תצוגה משוערת בלבד — מנוע העימוד Rust אינו זמין`.

### 2. `tok-ui` ↔ `tok-story-editor`
- **Method**: `storyEditor.flush()` / `storyEditor.getStory(true)`
- **Contract**: Immediate synchronous extraction and parse of DOM paragraphs, bypassing the 200ms debounce timer, ensuring zero keystroke loss before save/export.

### 3. `SpreadCanvas` ↔ DOM Rendering
- **Contract**: All text content rendered into DOM elements MUST be escaped via `escapeHtml()` or set via `textContent`, never raw string interpolation into `innerHTML`.

---

## Code Layout
- `crates/`
  - `tok-core/`: Document model, Hebrew text normalization, numbering, Undo/Redo
  - `tok-typeset/`: Knuth-Plass, Hebrew justification, Cassowary solver, Co-pagination, Gematria, HTML projection
  - `tok-pdf/`: PDF/X-1a & PDF/X-4, PageBoxes, Font subsetter, Image XObject, ToUnicode CMap, Preflight
  - `tok-storage/`: TokPackage ZIP, atomic file operations, `redb` workspace
  - `tok-plugin-host/`: Sandboxed core plugins (`divine-shield`, `hebrew-grep-acronym`)
  - `tok-engine-service/`: Unified typesetting, export, storage, and plugin service layer
  - `tok-ipc/`: IPC framing and worker protocol
  - `tok-cli/`: CLI binary entry point, worker mode, and benchmarks
  - `tok-desktop/`: Primary Tauri v2 desktop shell
- `packages/`
  - `tok-ui/`: Desktop UI renderer, `SpreadCanvas`, `tauriHostAdapter`, and components
  - `tok-story-editor/`: Text editor component and DOM converter
  - `tok-viewer/`: Viewport virtualization (`PageDomVirtualizer`)
  - `tok-canvas/`: Interactive canvas overlay (`CanvasInteractionOverlay`)
- `test/`: Integration and E2E test suites
- `docs/`: Architectural documentation, file index (`PROJECT_FILES.md`), and specifications

<p align="center">
  <img src="assets/icon.png" alt="TypesetOK Icon" width="128" height="128" />
</p>

<h1 align="center">TypesetOK (TOK)</h1>

<p align="center">
  <strong>Professional Desktop Publishing Software for Hebrew, Multi-Text (Mikraot Gedolot &amp; Talmud / Shas), Live Tzurat HaDaf &amp; Pre-Press</strong><br>
  <em>Version 0.40.3 • Native Rust Core • Tauri v2 Desktop Shell • Full Determinism</em>
</p>

<p align="center">
  <strong>English</strong> • <a href="./README.he.md">עברית (Hebrew)</a>
</p>

<p align="center">
  <img src="assets/logo.jpg" alt="TypesetOK Official Logo" width="640" />
</p>

---

**TypesetOK (TOK)** is a Desktop Publishing (DTP) application built from the ground up for advanced Hebrew typography, bidirectional text (`Bidi` / `RTL`), synchronized multi-stream typesetting (live *Tzurat HaDaf*, *Mikraot Gedolot*, Talmud / *Shas*, *Responsa* / *Shu"t*, and academic editions), and end-to-end deterministic generation of print-ready files (`ISO 15930-1:2001 PDF/X-1a:2001`).

---

## 1. Architecture Overview

The project is organized as a Monorepo that strictly separates the Rust typesetting and document core from the TypeScript presentation and interaction layers. For a complete file-by-file directory index and explanation of every file in the repository, see [`docs/PROJECT_FILES.md`](./docs/PROJECT_FILES.md).

### End-to-End Deterministic Data Flow

```text
[UI / Story Editor / SpreadCanvas (TypeScript)]
       │
       ▼  (Tauri v2 IPC Commands)
[tok-desktop (Tauri v2)]
       │
       ├──► [tok-engine-service]  Unified Typesetting, Pre-Press, Storage & Plugin Orchestration
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

> **Mandatory Engineering Principle:** The browser/webview **does not** decide where lines break or where pages split, and the system **does not** use external browser polyfill libraries (such as `@vivliostyle/core` or `Paged.js`). All geometry is computed in Rust (`tok-typeset`), exported as `PageLayoutBox` structures in exact `pt` units, and projected onto a rigid DOM projection (`SpreadCanvas`) or directly into a `PDF/X-1a` / `PDF/X-4` file.

---

### 9 Core Rust Crates (`crates/`)

| Crate | Primary Role & Responsibilities |
| :--- | :--- |
| **`tok-core`** | Canonical Document Model (TDM): immutable document tree, `ropey` B-Tree Rope for long texts, `ULID` identifiers, `FractionalIndex`, transaction stack and delta Undo/Redo, multi-layered apparatus configurations (`FootnoteApparatus`), typographic style patches, outline/gematria numbering (`ט״ו`, `ט״ז`), and canonical Hebrew typographic input normalizer ($\text{Shin/Sin Dot} \rightarrow \text{Dagesh/Mapiq} \rightarrow \text{Niqqud} \rightarrow \text{Meteg} \rightarrow \text{Te'amim}$). |
| **`tok-typeset`** | Typesetting & typography engine: dedicated Masoretic and biblical cantillation (*Te'amim*) shaper (`MasoreticShaper`, inverted Nun, Paseq, upper/lower Meteg, and biblical OpenType features), `unicode-bidi` (UAX #9) directionality resolution, pure-Rust font shaping (`rustybuzz` including GPOS `mark`/`mkmk`), global **Knuth-Plass** line-breaking algorithm, three-tier Hebrew justification hierarchy (controlled word spacing $\rightarrow$ letter elongation / *Ahaltaram* $\rightarrow$ micro-tracking $\pm 2\%$), linear constraint solver (**Cassowary**), Hebrew gematria engine with taboo table (`ט״ו`, `ט״ז`, `ע״ר`, `ער״ב`, `ער״ה`, `חר״צ`, `ד״ש`, `שי״ד`, `שד״מ`, `נט״ש`, `תרח״צ`, `תשד״מ` and `U+05F3`/`U+05F4`), dynamic width-ratio distribution for rabbinic streams (2, 3, and 4 columns in *Shas* and *Mikraot Gedolot* with exact Recto/Verso tracking), parallel multi-layered footnote layout engine (`FootnotePaginator`), Hebrew drop caps, and polygon/scanline clipping engine for L-shaped commentary wrapping. |
| **`tok-pdf`** | Native pre-press engine (`pdf-writer`): direct generation of `PDF/X-1a:2001` (ISO 15930-1) and `PDF/X-4` modes, `DeviceCMYK` color space and spot colors (`Separation`), standard print boxes (`MediaBox`, `CropBox`, `BleedBox`, `TrimBox`), crop & registration marks, PNG/JPEG image XObject encoding (`image_xobject`), font subsetting and `/ToUnicode` CMap tables for Hebrew text search and copy, and a `Preflight` engine. |
| **`tok-storage`** | Dual storage engine: crash-resilient ACID workspace in **`redb`** + atomic saving to `.tok` package files (ZIP archive with `Deflate` compression containing `manifest.json`, `document.json`, `assets/`, and `previews/`). |
| **`tok-plugin-host`** | Deterministic extension & plugin engine: executes core plugins based on atomic transactions under capability checks (`PluginCapability`) and panic isolation (`catch_unwind`), including built-in plugins `tok.builtin.divine-shield` and `tok.builtin.hebrew-grep-acronym`. |
| **`tok-engine-service`** | Unified Rust service layer orchestrating document loading, typesetting, PDF/HTML export, preflight evaluation, package persistence, and built-in plugins for both `tok-desktop` and `tok-cli`. |
| **`tok-ipc`** | Binary framing & worker protocol library (`TOKB` Binary Framing / JSON `WorkerRequest`) for inter-process command and response messages. |
| **`tok-cli`** | Main system CLI binary, serving for headless batch processing, CI, and persistent `worker` mode. |
| **`tok-desktop`** | Native **Tauri v2** desktop shell (`src-tauri` equivalent): manages native windows, Tauri IPC commands, Windows system fonts, `.tok` file associations, plugin loading, and Ed25519-verified updates, calling `tok-engine-service` directly in-process. |

---

### 4 TypeScript UI Packages (`packages/`)

| Package | Primary Role & Responsibilities |
| :--- | :--- |
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

# 2. Build all Rust crates in the workspace (including tok-desktop and tok-cli)
cargo build --workspace

# 3. Type-check, compile, and bundle the UI packages
npm run lint
npm run build

# 4. Launch the TypesetOK desktop application (Tauri v2)
npm start

# 5. (Optional) Package the desktop bundle and build an installer
npm run package:desktop
npm run package:installer
```

---

## 3. CLI Binary Usage (`tok-cli`)

The `tok-cli` binary allows running all core operations directly from the command line or CI/CD pipelines:

| Subcommand | Syntax & Parameters | Description |
| :--- | :--- | :--- |
| **`typeset-document`** | `tok-cli typeset-document <INPUT.json\|.tok\|--demo> <OUTPUT.json> [--fonts-dir <DIR>] [--assets-from <TOK>]` | Full typesetting of a document (including Knuth-Plass, Cassowary, and Co-Pagination), returning a detailed `PageLayoutBox` tree in `pt` units. |
| **`incremental-typeset`** | `tok-cli incremental-typeset <INPUT.json\|--demo> <OUTPUT.json> [--node-id <ID>]` | Convergence-based incremental typesetting (`Convergence Paginator`) with recompute diagnostics. |
| **`diagnose-recompute`** | `tok-cli diagnose-recompute <INPUT.json\|.tok\|--demo>` | Runs a 3-position invalidation & cache diagnostic report comparing recomputed vs. cached pages. |
| **`render-pdf`** | `tok-cli render-pdf <INPUT.json\|.tok\|--demo> <OUTPUT.pdf> [--standard pdfx4] [--profile <ID>] [--no-bleed] [--no-crop-marks] [--registration-marks]` | Generates a print-standard `PDF/X-1a:2001` or `PDF/X-4` file directly from the document (including CMYK, print boxes, and font subsetting). |
| **`render-html`** | `tok-cli render-html <INPUT.json\|.tok\|--demo> <OUTPUT.html>` | Generates a standalone pre-paginated HTML/CSS file for preview and inspection. |
| **`save-package`** | `tok-cli save-package <INPUT.json> <OUTPUT.tok> [--fonts-dir <DIR>] [--assets-from <TOK>]` | Atomically packs and saves a JSON document into the `.tok` package format (ZIP + Deflate). |
| **`open-package`** | `tok-cli open-package <INPUT.tok> <OUTPUT.json>` | Opens a `.tok` file and restores the canonical document JSON. |
| **`inspect-package`** | `tok-cli inspect-package <INPUT.tok>` | Verifies `.tok` archive structure and prints a metadata summary (title, schema version, and embedded assets). |
| **`inspect-font`** | `tok-cli inspect-font <PATH.ttf>` | Validates an external font (`glyf`/OS2 `fsType`/Hebrew coverage) and emits a JSON report. |
| **`preflight`** | `tok-cli preflight <INPUT.json\|.tok\|--demo> [OUTPUT.json] [--fonts-dir <DIR>] [--assets-from <TOK>]` | Runs real pre-press preflight evaluation and emits a `PreflightReport` JSON. |
| **`run-plugins`** | `tok-cli run-plugins <INPUT.json\|.tok\|--demo> <OUTPUT.json>` | Runs the built-in deterministic pre-typeset plugins (`tok.builtin.divine-shield`, `tok.builtin.hebrew-grep-acronym`) on the document. |
| **`sync-heading-numbers`** | `tok-cli sync-heading-numbers <INPUT.json\|.tok\|--demo> <OUTPUT.json> [--plain]` | Synchronizes outline/chapter heading numbers using Hebrew gematria formatting. |
| **`verify-determinism`** | `tok-cli verify-determinism` | Runs typesetting and PDF/HTML generation across single-flow and multi-flow pipelines and verifies bit-for-bit SHA-256 identity. |
| **`benchmark-typeset`** | `tok-cli benchmark-typeset [--pages N]` | Runs a stress and performance benchmark (default `1000` pages) and prints layout timing and cascade latency metrics. |
| **`worker`** | `tok-cli worker` | Runs `tok-cli` in persistent stdin/stdout JSON line worker mode with layout cache reuse. |

---

## 4. Testing & Verification

The project includes a multi-layered test suite verifying the Rust core, the TypeScript packages, and end-to-end integration:

```powershell
# 1. Run all unit and integration tests for the Rust core (all workspace crates)
cargo test --workspace

# 2. Run strict TypeScript compilation and type checking across all 4 packages
npm run lint

# 3. Run the frontend, gematria, virtualization, Canvas, Story Editor, updater, and security test suite
npm test

# 4. Verify Bit-for-Bit Determinism
cargo run -p tok-cli -- verify-determinism
```

---

## 5. Known Limitations

In accordance with the project's engineering discipline (zero false claims and zero mock implementations), the following are the known limitations in the current release:

1. **TIFF Image Support (BigTIFF / JPEG-in-TIFF):** Standard 1/8/16-bit TIFF images (RGB, CMYK, Grayscale, Palette, LZW, ZIP/Deflate, PackBits, multi-page IFDs via `#page=N`, DPI metadata, and embedded ICC profiles) are supported in `tok-pdf` (`tiff`, `image_xobject`); BigTIFF (64-bit offsets, magic `43`) and old-style JPEG-compressed TIFFs (`Compression=6/7`) are not supported.
2. **In-Process Engine (`tok-engine-service`):** `tok-desktop` (Tauri v2) invokes `tok-engine-service` directly in-process, while `crates/tok-ipc` provides the framing/worker protocol types for `tok-cli`.
3. **PDF/X-4 Export & External ICC Profile Management:** The PDF generator produces `PDF/X-1a:2001` and `PDF/X-4` files with real `DeviceCMYK` and `Separation` (Spot Colors) operators and a built-in `OutputIntent` dictionary (ISO Coated v2 / Fogra39), without dynamically linking an external C library such as `LittleCMS`.

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


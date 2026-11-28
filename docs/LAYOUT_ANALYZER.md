# TypesetOK Page Layout & Form Recognition (`tok-layout-analyzer`)

## Overview

`tok-layout-analyzer` bridges external document inputs (vector PDFs and scanned pages) with TypesetOK's deterministic typesetting engine (`tok-typeset`), service layer (`tok-engine-service`), desktop host (`tok-desktop`), and UI (`tok-ui`).

## Components

- **Crate:** [`crates/tok-layout-analyzer`](../crates/tok-layout-analyzer/Cargo.toml)
  - **Stage 1 (Deterministic Fast-Path):** PDF vector stream extraction (`pdf_vector`), Sauvola binarization, deskewing, gutter shadow removal (`preprocess`), two-page spread splitting (`spread_split`), recursive XY-Cut (`xycut`), connected-component line height & Hebrew Square-vs-Rashi script heuristics (`components`, `script_classifier`), horizontal rule detection (`rules`), and L-shape Tzurat HaDaf signature detection (`lshape`).
  - **Stage 2 (Optional ONNX ML Fallback):** Local `ort` inference for Apache-2.0 `PP-DocLayout-S` behind `--features ml-onnx` (`ml_onnx`, `ml_labels`, `hybrid`).
  - **Document-Level Context & Synthesis:** Outlier filtering, confidence-weighted majority voting, Recto/Verso mirror-symmetry verification (`context`), and `LayoutFamily` / `SectionNode` config synthesis (`synthesizer`).
  - **Synthetic Data Bootstrapping:** Zero-cost ground-truth extraction from `PageLayoutBox` (`synthetic`) and scan degradation augmentation (`augment`).
- **Service & CLI:** `recommend_layout_from_file` in `tok-engine-service`; `tok-cli analyze-layout` and `tok-cli generate-layout-dataset`.
- **Desktop & UI:** Async Tauri command `analyze_page_layout` (`tok-desktop`), `layoutAnalyzerBridge.ts`, `ImportTemplateModal.ts`, and `BatchImportModal.ts`.

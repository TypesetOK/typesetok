# `tok-layout-analyzer`

Page Layout & Form Recognition Engine for **TypesetOK**.

## Architecture

Implements a two-stage cascaded hybrid pipeline:

1. **Vector Fast-Path (`pdf_vector`)**: Extracts text boxes, font names, font sizes, and horizontal rules directly from PDF text streams.
2. **Raster Scan Path (`preprocess` + `xycut` + `rules`)**:
   - Automatic two-page spread splitting (`spread_split`) for open-book scans (`W > H`).
   - Gutter shadow normalization, projection-variance deskewing (`±10°`), and Sauvola adaptive binarization.
   - Recursive XY-Cut segmentation, connected-component line-height estimation, L-shape detection (`lshape`), and Hebrew Square-vs-Rashi script heuristics (`script_classifier`).
3. **Optional ML Fallback (`ml-onnx` feature)**: Local `ort` (ONNX Runtime) inference with Apache-2.0 `PP-DocLayout-S` when Stage 1 confidence falls below `0.85`.
4. **Document-Level Context (`context` + `synthesizer`)**: Filters title/opening outliers, performs confidence-weighted majority voting, verifies Recto/Verso mirror symmetry, and synthesizes `LayoutRecommendation` mapped to `tok_typeset::LayoutFamily` and `tok_core::SectionNode`.

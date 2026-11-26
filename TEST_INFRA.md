# E2E Test Infra: TypesetOK

## Test Philosophy
- Opaque-box, requirement-driven. No dependency on implementation internals.
- Verification methodology: Category-Partition + Boundary Value Analysis (BVA) + Pairwise Combinatorial + Real-World Workload Testing.
- All tests execute actual CLI binaries and library entry points with reproducible assertions.

## Feature Inventory & Test Mapping
| # | Feature | Source (Requirement) | Tier 1 (Coverage) | Tier 2 (Boundary) | Tier 3 (Pairwise) | Tier 4 (Workload) |
|---|---------|---------------------|:-----------------:|:-----------------:|:-----------------:|:-----------------:|
| 1 | Hebrew text normalization & Niqqud | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 2 | Knuth-Plass line breaking & Justification | ORIGINAL_REQUEST §R2, §R3 | 5 | 5 | ✓ | ✓ |
| 3 | Multi-stream Talmud layout (Gemara/Rashi/Tosafot) | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ | ✓ |
| 4 | ISO PDF/X-1a export & PageBoxes | ORIGINAL_REQUEST §R2, §R4 | 5 | 5 | ✓ | ✓ |
| 5 | Atomic .tok package save/open & Zip defense | ORIGINAL_REQUEST §R2, §R3 | 5 | 5 | ✓ | ✓ |
| 6 | Desktop Shell (Tauri v2) & Story Editor synchronization | ORIGINAL_REQUEST §R2, §R3 | 5 | 5 | ✓ | ✓ |

## Test Architecture
- Test runner: `cmd /c node test/run_e2e_suite.mjs`
- Test case format: Node.js test harness executing real `tok-cli.exe` and verifying exit codes, JSON outputs, and generated artifacts
- Pass/Fail semantics: Exit code 0, 100% assertions pass, 0 timeouts

## Real-World Application Scenarios (Tier 4)
| # | Scenario | Features Exercised | Expected Outcome |
|---|----------|--------------------|------------------|
| 1 | Full Talmud Tractate Page (Gemara + Rashi + Tosafot) | F1, F2, F3, F4, F5 | Deterministic multi-flow layout with exact column balance and valid PDF/X output |
| 2 | 100-page Hebrew Prose Book with Niqqud & Teamim | F1, F2, F4, F5 | 100% justified text, no orphan headers, valid ToUnicode CMap in PDF |
| 3 | Liturgical Text with Sacred Names Protection | F1, F2, F4 | Divine names preserved unbroken across line wraps, no accidental deletion |
| 4 | Multi-column Text with Variable Margins & Anchors | F1, F2, F3, F4 | Cassowary solver satisfies all geometric constraints without overlap |
| 5 | Full Roundtrip: Edit in StoryEditor -> Atomic .tok Save -> Reopen -> Export PDF | F1, F2, F4, F5, F6 | Exact document fidelity, 0 keystroke loss, identical PDF rendering |

## Coverage Goals
- Tier 1: ≥30 test cases (≥5 per feature)
- Tier 2: ≥30 boundary cases (empty, maximum size, invalid zip, extreme margins)
- Tier 3: ≥6 pairwise interaction tests
- Tier 4: ≥5 full application workloads
- Total: ≥71 comprehensive E2E test cases

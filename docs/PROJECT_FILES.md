# אינדקס קבצי הפרויקט — TypesetOK (TOK) File Index

מסמך זה מרכז את כל קבצי המקור, התצורה, הנכסים, הבדיקות והתיעוד בפרויקט (ללא תיקיות תלויות ותוצרי בנייה כגון `node_modules`, `target`, `dist`), עם הפניה ישירה והסבר על תפקידו של כל קובץ.

> חזרה ל-[README.md](../README.md)

---

## 1. קובצי שורש ותצורה (Root & Configuration)

| קובץ | תפקיד |
| :--- | :--- |
| [`.cargo/config.toml`](../.cargo/config.toml) | תצורת Cargo המקומית: הגדרת `rust-lld` ל-`x86_64-pc-windows-gnu` ותיקיית היעד `target-dir` |
| [`.gitignore`](../.gitignore) | רשימת קבצים ותיקיות להתעלמות ב-Git (`node_modules`, `target`, `dist`, וכו') |
| [`audit.toml`](../audit.toml) | תצורת `cargo-audit` לסריקת פגיעויות אבטחה בתלויות Rust |
| [`Cargo.toml`](../Cargo.toml) | מניפסט סביבת העבודה של Rust (9 חבילות workspace ותלויות משותפות) |
| [`Cargo.lock`](../Cargo.lock) | נעילת גרסאות מדויקת של תלויות Rust |
| [`package.json`](../package.json) | מניפסט Monorepo של Node/TypeScript, סקריפטי בנייה, בדיקות, אריזה והרצת Tauri/Electron |
| [`package-lock.json`](../package-lock.json) | נעילת גרסאות מדויקת של תלויות npm |
| [`tsconfig.json`](../tsconfig.json) | תצורת הידור TypeScript ראשית המקשרת בין 5 חבילות `packages/` |
| [`README.md`](../README.md) | התיעוד הראשי של הפרויקט (אנגלית ועברית) |
| [`CHANGELOG.md`](../CHANGELOG.md) | יומן השינויים הרשמי של הגרסאות (מקור האמת שממנו נוצר `changelogData.ts`) |
| [`PROJECT.md`](../PROJECT.md) | סקירת ארכיטקטורה, מפת תכונות, אבני דרך וחוזי ממשק |
| [`ROADMAP.md`](../ROADMAP.md) | מפת קוד, יעדי פיתוח ורעיונות עתידיים |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | מדריך תרומה לקוד, חוקי טיפוגרפיה ובדיקות חובה |
| [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md) | קוד ההתנהגות של קהילת הפרויקט |
| [`LICENSE.md`](../LICENSE.md) | נוסח הרישיון המשפטי `TOK-NCCL v1.2` |
| [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md) | פירוט רישיונות וזכויות יוצרים של ספריות צד-שלישי וגופנים |
| [`TEST_INFRA.md`](../TEST_INFRA.md) | אפיון תשתית בדיקות ה-E2E (4 שכבות כיסוי) |

---

## 2. צינורות CI/CD (`.github/workflows/`)

| קובץ | תפקיד |
| :--- | :--- |
| [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) | בדיקות CI ראשיות: הידור Rust, בדיקות ליבה, `lint`, ובדיקות Frontend/E2E |
| [`.github/workflows/desktop-build.yml`](../.github/workflows/desktop-build.yml) | בניית מעטפת שולחן העבודה והרצת בדיקות ייעודיות |
| [`.github/workflows/release.yml`](../.github/workflows/release.yml) | בנייה, אריזה והפצת גרסת שחרור (`Release`) עם הערות שחרור אוטומטיות |
| [`.github/workflows/benchmark.yml`](../.github/workflows/benchmark.yml) | הרצת בנצ'מרק ביצועים ועימוד תקופתי |
| [`.github/workflows/security-audit.yml`](../.github/workflows/security-audit.yml) | סריקת אבטחה אוטומטית של תלויות Rust ו-npm |

---

## 3. נכסים, גופנים, סכמות וסקריפטים (`assets/`, `schemas/`, `scripts/`)

### `assets/`
- **גופני דפוס וטקסט** (`assets/fonts/*.ttf` + רישיונות `OFL-*.txt`): `DavidLibre-Regular.ttf`, `FrankRuhlLibre-Regular.ttf`, `FrankRuhlLibre-Bold.ttf`, `NotoRashiHebrew-Regular.ttf`, `NotoRashiHebrew-Bold.ttf`, `NotoSerifHebrew-Regular.ttf`.
- **גופני ממשק המשתמש** (`assets/fonts/ui/*.woff2` + רישיונות `OFL-*.txt`): משפחות `frank-ruhl-libre`, `ibm-plex-mono`, `ibm-plex-sans-hebrew`, `noto-rashi-hebrew`.
- **אייקונים וגרפיקה**: `assets/icon.ico`, `assets/icon.png`, `assets/logo.jpg`, `assets/installer-header.{bmp,png}`, `assets/installer-sidebar.{bmp,png}`, ואייקוני SVG תחת `assets/icons/` (`book.svg`, `download.svg`, `globe.svg`, `help.svg`, `keyboard.svg`, `layers.svg`, `license.svg`, `sparkle.svg`, `terminal.svg`).

### `schemas/`
| קובץ | תפקיד |
| :--- | :--- |
| [`schemas/document/tok_document_schema.json`](../schemas/document/tok_document_schema.json) | סכמת JSON Schema הרשמית של מודל המסמך `DocumentRoot` |

### `scripts/`
| קובץ | תפקיד |
| :--- | :--- |
| [`scripts/bundle-ui.mjs`](../scripts/bundle-ui.mjs) | אריזת ממשק ה-UI (`tok-ui`) באמצעות `esbuild` עבור Electron ו-Tauri (`--tauri`) |
| [`scripts/build-desktop-bundle.mjs`](../scripts/build-desktop-bundle.mjs) | בניית חבילת ההפצה השולחנית (Tauri או Electron) ב-`dist/` |
| [`scripts/build-installer.mjs`](../scripts/build-installer.mjs) | בניית קובץ ההתקנה (`Setup.exe`) באמצעות NSIS/Inno Setup |
| [`scripts/installer.iss`](../scripts/installer.iss) | סקריפט מתקין Inno Setup ל-Windows |
| [`scripts/installer.nsi`](../scripts/installer.nsi) | סקריפט מתקין NSIS דו-לשוני ל-Windows |
| [`scripts/generate-installer-assets.ps1`](../scripts/generate-installer-assets.ps1) | יצירת תמונות הרקע והכותרת למתקין Windows |
| [`scripts/build-engine-fonts.py`](../scripts/build-engine-fonts.py) | עיבוד והטמעת נכסי גופנים עבור מנוע ה-Rust |
| [`scripts/sync-changelog.mjs`](../scripts/sync-changelog.mjs) | המרת `CHANGELOG.md` אוטומטית לקובץ TypeScript `changelogData.ts` |
| [`scripts/generate-release-notes.mjs`](../scripts/generate-release-notes.mjs) | הפקת הערות שחרור מעוצבות מ-`CHANGELOG.md` עבור GitHub Releases |
| [`scripts/build-commits-history.mjs`](../scripts/build-commits-history.mjs) | הפקת היסטוריית קומיטים מובנית לממשק/תיעוד |
| [`scripts/update-contributors.mjs`](../scripts/update-contributors.mjs) | יצירת `contributors.generated.ts` מרשימת התורמים ב-Git |
| [`scripts/measure-startup.mjs`](../scripts/measure-startup.mjs) | מדידת זמני עלייה (Startup Tracing) של האפליקציה |
| [`scripts/benchmark-multi-flow-pagination.mjs`](../scripts/benchmark-multi-flow-pagination.mjs) | הרצת בנצ'מרק לעימוד רב-תזרימי |
| [`scripts/verify-anchor-fixtures.mjs`](../scripts/verify-anchor-fixtures.mjs) | אימות תקינות קובצי ה-fixtures של עוגנים רב-תזרימיים |

---

## 4. חבילות הליבה ב-Rust (`crates/`)

### `crates/tok-core/` — מודל המסמך הקנוני (TDM)
| קובץ | תפקיד |
| :--- | :--- |
| [`crates/tok-core/Cargo.toml`](../crates/tok-core/Cargo.toml) | תצורת החבילה והתלויות (`ropey`, `serde`, `unicode-normalization`) |
| [`crates/tok-core/src/lib.rs`](../crates/tok-core/src/lib.rs) | נקודת הכניסה וייצוא המודולים של `tok-core` |
| [`crates/tok-core/src/model.rs`](../crates/tok-core/src/model.rs) | מבני המסמך: `DocumentModel`, `DocumentRoot`, `SectionNode`, `Flow`, `ParagraphNode` |
| [`crates/tok-core/src/id.rs`](../crates/tok-core/src/id.rs) | מזהי `Ulid` ואינדוקס שברירי `FractionalIndex` |
| [`crates/tok-core/src/anchor.rs`](../crates/tok-core/src/anchor.rs) | עוגני טקסט יציבים במרחב הפסקאות |
| [`crates/tok-core/src/multi_flow_anchor.rs`](../crates/tok-core/src/multi_flow_anchor.rs) | סריאליזציה ועיגון בין תזרימים (גמרא ↔ רש"י/תוספות) |
| [`crates/tok-core/src/normalizer.rs`](../crates/tok-core/src/normalizer.rs) | נרמול עברי קנוני (`HebrewNormalizer`: סדר נקודת ש/ס, דגש, ניקוד, מתג, טעמים) |
| [`crates/tok-core/src/hebrew/mod.rs`](../crates/tok-core/src/hebrew/mod.rs) | מודול עזר לעברית תחת `tok-core` |
| [`crates/tok-core/src/hebrew/normalization.rs`](../crates/tok-core/src/hebrew/normalization.rs) | שגרות נרמול עברי משלימות |
| [`crates/tok-core/src/numbering.rs`](../crates/tok-core/src/numbering.rs) | סכמות מספור (`Arabic`, `HebrewGematria`, `TorahSiman`, `TorahDafAmud`, `Roman`) |
| [`crates/tok-core/src/outline.rs`](../crates/tok-core/src/outline.rs) | עץ ראשי הפרקים (`DocumentOutline`) ומספור היררכי |
| [`crates/tok-core/src/header.rs`](../crates/tok-core/src/header.rs) | כותרות רצות (`RunningHeaderSpec`) והגדרות מערכי הערות (`FootnoteApparatus`) |
| [`crates/tok-core/src/reference.rs`](../crates/tok-core/src/reference.rs) | הפניות צולבות (`TargetAnchor`, `CrossReference`, `ReferenceRegistry`) |
| [`crates/tok-core/src/spans.rs`](../crates/tok-core/src/spans.rs) | טווחי עיצוב תווים (`CharacterSpan`) בתוך פסקה |
| [`crates/tok-core/src/styles.rs`](../crates/tok-core/src/styles.rs) | סגנונות פסקה ותו, צבעים, יישור ומרווחים |
| [`crates/tok-core/src/overrides.rs`](../crates/tok-core/src/overrides.rs) | נעילות פריסה ידניות (`ManualLayoutLock`) ואילוצי שבירת עמוד |
| [`crates/tok-core/src/dependency.rs`](../crates/tok-core/src/dependency.rs) | מעקב תלויות דו-כיווני (`ContentDependencyTracker`) בין פסקאות לעמודים |
| [`crates/tok-core/src/transaction.rs`](../crates/tok-core/src/transaction.rs) | פעולות אטומיות (`AtomicOperation`, `CompoundTransaction`) ומחסנית Undo/Redo |
| [`crates/tok-core/src/ui_state.rs`](../crates/tok-core/src/ui_state.rs) | מצב UI המתועד במודל המסמך |
| [`crates/tok-core/src/error.rs`](../crates/tok-core/src/error.rs) | טיפוסי השגיאות של `tok-core` |

### `crates/tok-typeset/` — מנוע העימוד, הטיפוגרפיה והחישוב המרחבי
| קובץ | תפקיד |
| :--- | :--- |
| [`crates/tok-typeset/Cargo.toml`](../crates/tok-typeset/Cargo.toml) | תצורת החבילה (`rustybuzz`, `unicode-bidi`, `cassowary`, `ttf-parser`, `rayon`) |
| [`crates/tok-typeset/src/lib.rs`](../crates/tok-typeset/src/lib.rs) | נקודת הכניסה וייצוא הממשקים של `tok-typeset` |
| [`crates/tok-typeset/src/engine.rs`](../crates/tok-typeset/src/engine.rs) | מתאם העימוד המרכזי (`TypesettingEngine`), פריסה חד-טורית, רב-טורית ושיבוץ מדיה |
| [`crates/tok-typeset/src/geometry.rs`](../crates/tok-typeset/src/geometry.rs) | מבני גיאומטריה ב-`pt` (`PageLayoutBox`, `TextFrameBox`, `LineBox`, `GlyphBox`, `PlacedImageBox`) |
| [`crates/tok-typeset/src/bidi.rs`](../crates/tok-typeset/src/bidi.rs) | פירוק כיווניות דו-כיוונית לפי Unicode UAX #9 |
| [`crates/tok-typeset/src/font.rs`](../crates/tok-typeset/src/font.rs) | טעינת גופנים מוטמעים וחיצוניים (`FontManager`) ובדיקת כיסוי גליפים |
| [`crates/tok-typeset/src/shaper.rs`](../crates/tok-typeset/src/shaper.rs) | עיצוב גליפים (`rustybuzz`) כולל GPOS `mark`/`mkmk` |
| [`crates/tok-typeset/src/masoretic.rs`](../crates/tok-typeset/src/masoretic.rs) | שייפר מסורה וטעמי מקרא (`MasoreticShaper`, נון הפוכה, פסיק, מתג) |
| [`crates/tok-typeset/src/knuth_plass.rs`](../crates/tok-typeset/src/knuth_plass.rs) | אלגוריתם שבירת שורות גלובלי אופטימלי (Knuth-Plass) |
| [`crates/tok-typeset/src/hebrew_justify.rs`](../crates/tok-typeset/src/hebrew_justify.rs) | יישור עברי תלת-שלבי (רווחי מילים, מתיחת אותיות אהלתר"ם, מיקרו-טרקינג) |
| [`crates/tok-typeset/src/optical_margin.rs`](../crates/tok-typeset/src/optical_margin.rs) | איזון שוליים אופטי (Optical Margin Alignment) |
| [`crates/tok-typeset/src/drop_cap.rs`](../crates/tok-typeset/src/drop_cap.rs) | אותיות פתיחה ומילות פתיחה עבריות (Drop Caps) |
| [`crates/tok-typeset/src/gematria.rs`](../crates/tok-typeset/src/gematria.rs) | מנוע גימטריה עברית דטרמיניסטי כולל טבלת הטאבו המלאה (`ט״ו` עד `תשד״מ`) |
| [`crates/tok-typeset/src/column_flow.rs`](../crates/tok-typeset/src/column_flow.rs) | זרימת טורים מרובים ומספורם |
| [`crates/tok-typeset/src/footnote_paginator.rs`](../crates/tok-typeset/src/footnote_paginator.rs) | פריסת הערות שוליים מקבילית רב-שכבתית |
| [`crates/tok-typeset/src/multi_flow.rs`](../crates/tok-typeset/src/multi_flow.rs) | פריסת רב-תזרימים, תבניות Recto/Verso ומפרשים |
| [`crates/tok-typeset/src/polygon.rs`](../crates/tok-typeset/src/polygon.rs) | מבני מצולעים לגיאומטריית צורת הדף |
| [`crates/tok-typeset/src/polygon_engine.rs`](../crates/tok-typeset/src/polygon_engine.rs) | מנוע חישוב וחיתוך מצולעים |
| [`crates/tok-typeset/src/polygon_flow.rs`](../crates/tok-typeset/src/polygon_flow.rs) | זרימת טקסט סביב מצולעים בצורת L (L-Shape wrapping) |
| [`crates/tok-typeset/src/co_pagination/`](../crates/tok-typeset/src/co_pagination/mod.rs) | תת-מערכת Co-Pagination: `mod.rs`, `engine.rs`, `sync.rs`, `cascade.rs`, `height_balance.rs`, `template.rs`, `worker.rs` |
| [`crates/tok-typeset/src/solver/`](../crates/tok-typeset/src/solver/mod.rs) | פותר אילוצים ליניארי Cassowary: `mod.rs`, `solver_core.rs`, `model.rs`, `page_model.rs`, `frame_model.rs`, `footnote_model.rs`, `l_shape_model.rs`, `proportions.rs` |
| [`crates/tok-typeset/src/layout_cache.rs`](../crates/tok-typeset/src/layout_cache.rs) | מטמון פריסת פסקאות (`ParagraphLayoutCache`) |
| [`crates/tok-typeset/src/incremental.rs`](../crates/tok-typeset/src/incremental.rs) | שובר עמודים אינקרמנטלי (`IncrementalPageBreaker`) וזיהוי התכנסות |
| [`crates/tok-typeset/src/page_break.rs`](../crates/tok-typeset/src/page_break.rs) | כללי שבירת עמוד, מניעת יתומים ואלמנות |
| [`crates/tok-typeset/src/hit_test.rs`](../crates/tok-typeset/src/hit_test.rs) | בדיקת פגיעה מרחבית (`Hit-Testing`) בין קואורדינטות לתווים |
| [`crates/tok-typeset/src/html_projection.rs`](../crates/tok-typeset/src/html_projection.rs) | קומפילציה של `PageLayoutBox` ל-HTML/CSS עם עימוד קבוע מראש |
| [`crates/tok-typeset/src/layout_classifier.rs`](../crates/tok-typeset/src/layout_classifier.rs) | סיווג תבניות ומאפייני פריסה |
| [`crates/tok-typeset/src/layout_family.rs`](../crates/tok-typeset/src/layout_family.rs) | הגדרת משפחות פריסה (ספר רציף, מקראות גדולות, ש"ס וכו') |
| [`crates/tok-typeset/src/layout_features.rs`](../crates/tok-typeset/src/layout_features.rs) | דגלים ותכונות פריסה ברמת מסמך |
| [`crates/tok-typeset/src/layout_template.rs`](../crates/tok-typeset/src/layout_template.rs) | מודל תבניות פריסה מובנות |
| [`crates/tok-typeset/src/template_synthesizer.rs`](../crates/tok-typeset/src/template_synthesizer.rs) | יצירה אוטומטית של תבניות מתוך מאפיינים |
| [`crates/tok-typeset/src/mpt.rs`](../crates/tok-typeset/src/mpt.rs) | מבנה Multi-Page Tree / כלי עזר לעימוד |
| [`crates/tok-typeset/src/diagnostics.rs`](../crates/tok-typeset/src/diagnostics.rs) | דוחות אבחון (`RecomputeReport`) |
| [`crates/tok-typeset/src/diagnostics.rs`](../crates/tok-typeset/src/fixtures.rs) / `fixtures.rs` | גנרטור מסמכי דמה לבדיקות ובנצ'מרק (תחת feature `fixtures`) |
| `crates/tok-typeset/tests/*.rs` | 13 קובצי בדיקות אינטגרציה ורגרסיה לטיפוגרפיה, Co-Pagination, דטרמיניזם ו-RTL |
| [`crates/tok-typeset/benches/incremental_benchmark.rs`](../crates/tok-typeset/benches/incremental_benchmark.rs) | בנצ'מרק ביצועים לעימוד אינקרמנטלי |

### `crates/tok-pdf/` — מנוע קדם-דפוס והפקת PDF/X
| קובץ | תפקיד |
| :--- | :--- |
| [`crates/tok-pdf/Cargo.toml`](../crates/tok-pdf/Cargo.toml) | תצורת החבילה (`pdf-writer`, `subsetter`, `ttf-parser`, `miniz_oxide`) |
| [`crates/tok-pdf/src/lib.rs`](../crates/tok-pdf/src/lib.rs) | נקודת הכניסה של `tok-pdf` |
| [`crates/tok-pdf/src/pdf_engine.rs`](../crates/tok-pdf/src/pdf_engine.rs) | מחולל ה-PDF הראשי (`PdfPrePressEngine`) לתקני `PDF/X-1a:2001` ו-`PDF/X-4` |
| [`crates/tok-pdf/src/boxes.rs`](../crates/tok-pdf/src/boxes.rs) | חישוב `MediaBox`, `CropBox`, `BleedBox`, `TrimBox` וסימני חיתוך/רישום |
| [`crates/tok-pdf/src/color.rs`](../crates/tok-pdf/src/color.rs) | ניהול צבע `DeviceCMYK`, צבעי ספוט `Separation` ופרופילי `OutputIntent` |
| [`crates/tok-pdf/src/font_subsetter.rs`](../crates/tok-pdf/src/font_subsetter.rs) | גזירת תת-קבוצות גופנים (Font Subsetting) להקטנת קובץ ה-PDF |
| [`crates/tok-pdf/src/tounicode.rs`](../crates/tok-pdf/src/tounicode.rs) | יצירת טבלאות `/ToUnicode` CMap לחיפוש והעתקת טקסט עברי מנוקד |
| [`crates/tok-pdf/src/image_xobject.rs`](../crates/tok-pdf/src/image_xobject.rs) | פענוח PNG/JPEG (כולל `/SMask` ל-PDF/X-4) וקידוד ל-PDF Image XObject |
| [`crates/tok-pdf/src/ornament.rs`](../crates/tok-pdf/src/ornament.rs) | רינדור עיטורים וקטוריים וקווי הפרדה ב-PDF |
| [`crates/tok-pdf/src/preflight.rs`](../crates/tok-pdf/src/preflight.rs) | מנוע בדיקות קדם-דפוס (`PreflightReport`) לזיהוי שגיאות ואזהרות דפוס |
| `crates/tok-pdf/assets/icc/*` | טבלאות LUT בינאריות (`a2b.bin`, `b2a.bin`, `gamt.bin`) וסקריפט יצירה `gen_icc_luts.py` |
| [`crates/tok-pdf/tests/e2e_pipeline_trace_test.rs`](../crates/tok-pdf/tests/e2e_pipeline_trace_test.rs) | בדיקת E2E למעקב אחר צינור הפקת ה-PDF |

### `crates/tok-storage/`, `tok-plugin-host/`, `tok-engine-service/`, `tok-ipc/`, `tok-cli/`, `tok-desktop/`
| קובץ / תיקייה | תפקיד |
| :--- | :--- |
| [`crates/tok-storage/src/lib.rs`](../crates/tok-storage/src/lib.rs) | ייצוא מודולי האחסון |
| [`crates/tok-storage/src/package.rs`](../crates/tok-storage/src/package.rs) | קריאה וכתיבה אטומית של חבילות `.tok` (ZIP + Deflate + הגנת Zip-Bomb) |
| [`crates/tok-storage/src/workspace.rs`](../crates/tok-storage/src/workspace.rs) | סביבת עבודה ACID חסינת קריסות מעל `redb` |
| [`crates/tok-storage/src/book.rs`](../crates/tok-storage/src/book.rs) | ניהול ספר מרובה מסמכים (`.tokbook`) |
| [`crates/tok-storage/src/migration.rs`](../crates/tok-storage/src/migration.rs) | מיגרציות סכמה בין גרסאות מסמך |
| [`crates/tok-storage/src/atomic.rs`](../crates/tok-storage/src/atomic.rs) | כתיבת קבצים אטומית וחותמות זמן UTC |
| [`crates/tok-storage/src/error.rs`](../crates/tok-storage/src/error.rs) | טיפוסי השגיאות של `tok-storage` |
| [`crates/tok-plugin-host/src/lib.rs`](../crates/tok-plugin-host/src/lib.rs) | מנוע הרצת תוספי ליבה עם הרשאות, בידוד panic, ותוספי `divine-shield` ו-`hebrew-grep-acronym` |
| [`crates/tok-engine-service/src/lib.rs`](../crates/tok-engine-service/src/lib.rs) | נקודת הכניסה של שכבת השירות המאוחדת |
| [`crates/tok-engine-service/src/document.rs`](../crates/tok-engine-service/src/document.rs) | טעינה, יצירה ובדיקת גופנים ומסמכים |
| [`crates/tok-engine-service/src/export.rs`](../crates/tok-engine-service/src/export.rs) | ייצוא PDF/HTML, הרצת Preflight ותוספים מובנים |
| [`crates/tok-engine-service/src/session.rs`](../crates/tok-engine-service/src/session.rs) | ניהול סשן עימוד מתמיד (`WorkspaceSession`) עם מטמון |
| [`crates/tok-engine-service/src/io.rs`](../crates/tok-engine-service/src/io.rs) ו-[`error.rs`](../crates/tok-engine-service/src/error.rs) | פעולות קלט/פלט אטומיות וטיפוסי שגיאות שירות |
| [`crates/tok-ipc/src/lib.rs`](../crates/tok-ipc/src/lib.rs) ו-[`protocol.rs`](../crates/tok-ipc/src/protocol.rs) | הגדרות פרוטוקול `TOKB` ו-`WorkerRequest`/`WorkerResponse` |
| [`crates/tok-cli/src/main.rs`](../crates/tok-cli/src/main.rs) | בינארי ה-CLI הראשי ומצב `worker` מתמיד |
| [`crates/tok-desktop/src/main.rs`](../crates/tok-desktop/src/main.rs) | אתחול אפליקציית Tauri v2, חלונות ורישום פקודות |
| [`crates/tok-desktop/src/commands.rs`](../crates/tok-desktop/src/commands.rs) | פקודות IPC של Tauri לעימוד, ייצוא, דיאלוגים וקבצים |
| [`crates/tok-desktop/src/assets.rs`](../crates/tok-desktop/src/assets.rs) | שרת הנכסים המקומי `tok-asset://` |
| [`crates/tok-desktop/src/fonts.rs`](../crates/tok-desktop/src/fonts.rs) | זיהוי וטעינת גופני מערכת ב-Windows |
| [`crates/tok-desktop/src/plugins.rs`](../crates/tok-desktop/src/plugins.rs) | טעינת תוספי משתמש מתיקיית התוספים וניהול הרשאות |
| [`crates/tok-desktop/src/updater.rs`](../crates/tok-desktop/src/updater.rs) | מנגנון עדכונים אונליין/אופליין מאומת Ed25519 |
| [`crates/tok-desktop/src/state.rs`](../crates/tok-desktop/src/state.rs) ו-[`logger.rs`](../crates/tok-desktop/src/logger.rs) | ניהול מצב האפליקציה, הגדרות ויומני לוג |
| [`crates/tok-desktop/tauri.conf.json`](../crates/tok-desktop/tauri.conf.json), [`build.rs`](../crates/tok-desktop/build.rs), `capabilities/`, `gen/schemas/` | תצורה, הרשאות ובנייה של Tauri v2 |

---

## 5. חבילות ה-TypeScript והממשק (`packages/`)

### `packages/tok-ui/` — ממשק שולחן העבודה
| קובץ | תפקיד |
| :--- | :--- |
| [`packages/tok-ui/src/index.html`](../packages/tok-ui/src/index.html) ו-[`renderer.ts`](../packages/tok-ui/src/renderer.ts) | נקודת הכניסה ל-WebView/Renderer |
| [`packages/tok-ui/src/app.ts`](../packages/tok-ui/src/app.ts) | מחלקת היישום הראשית `TypesetOkApp` וניהול זרימת העבודה |
| [`packages/tok-ui/src/ui.ts`](../packages/tok-ui/src/ui.ts), [`theme.ts`](../packages/tok-ui/src/theme.ts), [`i18n.ts`](../packages/tok-ui/src/i18n.ts), [`icons.ts`](../packages/tok-ui/src/icons.ts), [`fonts.ts`](../packages/tok-ui/src/fonts.ts), [`gematria.ts`](../packages/tok-ui/src/gematria.ts), [`types.ts`](../packages/tok-ui/src/types.ts), [`appInfo.ts`](../packages/tok-ui/src/appInfo.ts), [`textStats.ts`](../packages/tok-ui/src/textStats.ts), [`contributors.generated.ts`](../packages/tok-ui/src/contributors.generated.ts) | עיצוב, תרגומים דו-לשוניים (`he`/`en`), אייקונים, גימטריה, סטטיסטיקות וטיפוסים |
| `packages/tok-ui/src/components/*.ts` | רכיבי UI: `SpreadCanvas.ts`, `ContextualInspector.ts`, `ActionHud.ts`, `CommandPalette.ts`, `TopSystemBar.ts`, `StatusBar.ts`, `StructureBar.ts`, `SettingsModal.ts`, `ExportDialog.ts`, `PdfPreviewModal.ts`, `PreflightModal.ts`, `WelcomeModal.ts`, `ImportTemplateModal.ts`, `CustomFlowModal.ts`, `MissingFontsModal.ts`, `TextCleanupModal.ts`, `TextStatsModal.ts`, `UserGuideModal.ts`, `AboutModal.ts`, `ConfirmDialog.ts`, `ModalController.ts`, `UpdatesDrawer.ts`, `MajorUpdateModal.ts`, `baselineGuides.ts` |
| `packages/tok-ui/src/engine/*.ts` | גשרי עימוד ותוכן: `typesetBridge.ts`, `documentBridge.ts`, `FlowPaginator.ts`, `lineFit.ts`, `lineRuns.ts`, `textCleanup.ts`, `textImporter.ts` |
| `packages/tok-ui/src/platform/*.ts` | הפשטת פלטפורמה: `hostBridge.ts`, `tauriHostAdapter.ts`, `hostSettings.ts` |
| `packages/tok-ui/src/plugins/*.ts` | מערכת התוספים בצד ה-UI: `PluginEngine.ts`, `plugin-types.ts`, `hebrew-utils.ts` |
| `packages/tok-ui/src/updates/*.ts` | מערכת "מה חדש": `changelogData.ts`, `updatesManager.ts`, `ChangelogEntryCard.ts`, `UpdatesBellButton.ts`, `updatePills.ts`, `types.ts` |
| `packages/tok-ui/src/services/recentProjectsStore.ts` ו-`utils/frameAlignment.ts` | ניהול פרויקטים אחרונים ויישור מסגרות |

### `packages/tok-electron/`, `tok-story-editor/`, `tok-viewer/`, `tok-canvas/`
| קובץ | תפקיד |
| :--- | :--- |
| `packages/tok-electron/src/*.ts` | מעטפת Electron האופציונלית: `main.ts`, `preload.ts`, `menu.ts`, `plugin-manager.ts`, `updater.ts`, `logger.ts`, `install-language.ts`, `filter-ssl.ts` |
| `packages/tok-story-editor/src/*.ts` | עורך הסיפור הרציף: `story_editor.ts`, `dom_converter.ts`, `find_replace.ts`, `history.ts`, `types.ts`, `index.ts` |
| `packages/tok-viewer/src/*.ts` | מנהל וירטואליזציית עמודים (`[K-1, K, K+1]`): `virtualizer.ts`, `index.ts` |
| `packages/tok-canvas/src/*.ts` | שכבת כיסוי גרפית וסמן אינטראקטיבי: `overlay.ts`, `index.ts` |

---

## 6. בדיקות ותיעוד (`test/`, `docs/`)

### `test/`
- **בדיקות יחידה ואינטגרציה (`test/*.test.mjs`)**: `frontend.test.mjs`, `host_bridge.test.mjs`, `milestone_features.test.mjs`, `story_editor.test.mjs`, `outcome_validation.test.mjs`, `importer.test.mjs`, `lifecycle_flow.test.mjs`, `geometric_fidelity.test.mjs`, `phase3_integration.test.mjs`, `issue20_regression.test.mjs`, `offline_updater.test.mjs`, `whats_new_system.test.mjs`, `release_notes.test.mjs`.
- **בדיקות E2E (`test/e2e/` + [`test/run_e2e_suite.mjs`](../test/run_e2e_suite.mjs))**: `cli_helper.mjs`, `tier1_feature_coverage.test.mjs`, `tier2_boundary_corner.test.mjs`, `tier3_pairwise_combinations.test.mjs`, `tier4_real_world_scenarios.test.mjs`.
- **דוגמאות תזרימים (`test/fixtures/multi_flow_anchors/*.json`)**: 33 דפי תלמוד מייצגים לאימות עיגון רב-תזרימי.

### `docs/`
- [`docs/PROJECT_FILES.md`](./PROJECT_FILES.md): אינדקס זה.
- [`docs/PLUGINS_API.md`](./PLUGINS_API.md): מפרט פיתוח תוספים ומודל ההרשאות.
- [`docs/WHATS_NEW_SYSTEM.md`](./WHATS_NEW_SYSTEM.md): תיעוד מערכת העדכונים ויומן השינויים.
- [`docs/STAGE_9_CORE_ENGINE_VALIDATION.md`](./STAGE_9_CORE_ENGINE_VALIDATION.md) ו-[`docs/STAGE_10_DOCUMENT_INFRASTRUCTURE_AND_INCREMENTAL_TYPESETTING.md`](./STAGE_10_DOCUMENT_INFRASTRUCTURE_AND_INCREMENTAL_TYPESETTING.md): דוחות אימות שלבים 9 ו-10.
- `docs/architecture/`: מפרטי ארכיטקטורה ממוקדים (`ANCHOR_SERIALIZATION_FORMAT.md`, `INVARIANT_1_COMMENTARY_LEAD_CONTROL.md`, `LIVING_TZURAT_HADAF.md`, `SHARED_COLUMN_HEIGHT_BALANCING.md`, `SYNCHRONIZED_PAGINATION_CASCADING.md`).
- `docs/research/`: מסמכי מחקר (`DIBUR_HAMATCHIL_EXTRACTION_RULES.md`, `DTP_Modern_UX_UI_Research.md`, `MIKRAOT_GEDOLOT_LAYOUT_PATTERNS.md`, `TALMUDIC_PAGINATION_ALGORITHMS.md`, `tauri-v2-migration-report.md`).

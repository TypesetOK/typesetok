# TypesetOK Changelog

מקור האמת המרכזי לכל עדכוני התוכנה ויומן השינויים של TypesetOK.
המידע בקובץ זה נשאב באופן אוטומטי לחלוניות "מה חדש" ולמודאל הכרזות הגרסה בממשק התוכנה.

---

## [0.33.0] - 2026-10-31
- **Importance**: major
- **Badge**: עברית: גרסה נוכחית | en: Current Version
- **Title**: עברית: מהפכת "צורת הדף" חיה, עגינה רב-תזרימית והקשחת דטרמיניזם | en: Living Tzurat HaDaf, Dynamic Multi-Flow Anchoring & Pre-Press Hardening
- **Summary**: עברית: שחרור רשמי של גרסה 0.33.0 הכוללת עימוד רב-תזרימי חי לש"ס ומקראות גדולות עם עגינה דו-כיוונית, גלישת עמודים מסונכרנת בזמן אמת, איזון גבהי טורים משותף, הקשחת מנוע הדטרמיניזם ואוטומציית שחרור גרסאות מלאה. | en: Official release 0.33.0 introducing Living Tzurat HaDaf with bi-directional multi-flow anchoring, real-time synchronized pagination cascading, shared column height balancing, determinism engine hardening, and automated CI/CD release workflow.

### Highlights
- [living-tzurat-hadaf] icon: layers | badge: עברית: צורת הדף חיה, en: Living Tzurat HaDaf
  - Title: עברית: מנוע עגינה רב-תזרימי דו-כיווני | en: Bi-Directional Multi-Flow Anchor Cascade Engine
  - Description: עברית: קישור לוגי אוטומטי בין פסקאות הטקסט הראשי למפרשים (רש"י ותוספות), המאפשר גלישה והתרחבות דינמית בזמן אמת. | en: Strict logical binding between primary text segments and commentaries with dynamic cascading and absorption.
- [determinism-hardening] icon: sparkle | badge: עברית: דטרמיניזם מלא, en: Full Determinism
  - Title: עברית: הקשחת דטרמיניזם מוחלט בכל שלבי העימוד והייצוא | en: Absolute Pre-Press Output Determinism
  - Description: עברית: הבטחת שחזור ביט-אחר-ביט בייצוא PDF/X-1a ו-HTML בכל פלטפורמה, ללא תלות בסידורי גיבוב או מצבי זיכרון. | en: Guarantees bit-for-bit reproducible PDF/X-1a and HTML output across all platforms and execution environments.

### Changes
- [features] עברית: עגינה דו-כיוונית קנונית עם זיהוי דיבור המתחיל אוטומטי (ד"ה) עבור דפי גמרא ומקראות גדולות | en: Canonical bi-directional anchoring with automated Dibur HaMatchil (DH) cue recognition
- [features] עברית: גלישה מסונכרנת במעבר עמודים המונעת הופעת פירוש לפני העוגן המרכזי שלו (No Commentary Lead Invariant) | en: Synchronized multi-stream pagination strictly enforcing the no-commentary-lead invariant
- [typography] עברית: איזון גובה תחתון פעיל בעמודי תלמוד ומקראות גדולות (Shared Column Height Balancing) | en: Active shared column height balancing aligning column bottom baselines
- [typography] עברית: בקרת יתומים ואלמנות (Orphan/Widow Control) בחלוקת פירושים ארוכים במיוחד על פני עמודים | en: Strict orphan and widow control for oversized commentary paragraphs spanning page boundaries
- [typography] עברית: יישור אופטי של שוליים (Optical Margin Alignment) לסימני פיסוק וגליפיים בולטים בעברית ובכיווניות מעורבת | en: Professional Optical Margin Alignment for punctuation and overhanging glyphs with full RTL and Hebrew support
- [typography] עברית: איזון שורה אחרונה מאולץ באלגוריתם Knuth-Plass כולל אילוצי רוחב מינימלי ומיקום אופטימלי | en: Knuth-Plass constrained last-line balancing with minimum width constraints and optimal final-line placement
- [typography] עברית: מנוע פוליגונים כללי וטורים ברוחב משתנה (L-Shape & Variable Columns) עם חיתוך רצועות שמרני | en: Arbitrary polygonal text regions, variable column geometry, and conservative band measure-profile generation
- [typography] עברית: חלון מילת פתיחה עברי (Hebrew Drop Caps) עם מדידת תיבה תוחמת מדויקת של גליפים, ניקוד וטעמים | en: Hebrew Drop Caps with exact glyph outline bounding boxes and multi-line text-exclusion geometry
- [improvements] עברית: שיפור מנגנון ה-CI ואיחוד בדיקות הדטרמיניזם להבטחת שחרור אוטומטי חלק | en: Streamlined CI determinism verification and automated release pipeline trigger

---

## [0.31.0] - 2026-10-31
- **Importance**: major
- **Badge**: עברית: עדכון גדול | en: Major Release
- **Title**: עברית: עימוד רב-תזרימי חי ("צורת הדף" דינמית) וסנכרון עוגנים דו-כיווני | en: Dynamic Multi-Flow Anchoring ("Living Tzurat HaDaf") & Synchronized Pagination
- **Summary**: עברית: מימוש מלא של עימוד רב-תזרימי חי לתלמוד ומקראות גדולות: עגינה דו-כיוונית בין טקסט מרכזי לפירושים, גלישת עמודים מסונכרנת בזמן אמת, ואיזון גובה תחתון דינמי בכל הטורים. | en: Full end-to-end implementation of Living Tzurat HaDaf: bi-directional anchor linking between primary text and commentaries, real-time synchronized pagination across page boundaries, and dynamic shared column height balancing.

### Highlights
- [multi-flow-anchoring] icon: layers | badge: עברית: צורת הדף חיה, en: Living Tzurat HaDaf
  - Title: עברית: מנגנון עגינה דו-כיווני וסנכרון עמודים חי | en: Bi-Directional Multi-Flow Anchoring
  - Description: עברית: קישור לוגי הדוק בין פסקאות הגמרא/המקרא לפירושי רש"י ותוספות, המבטיח גלישה אוטומטית של פירושים לעמוד הבא כאשר הטקסט המרכזי גולש. | en: Strict logical binding between primary text segments and commentaries ensuring automatic cascading of commentaries when primary text reflows.
- [height-balancing] icon: align | badge: עברית: איזון טיפוגרפי, en: Typographic Balance
  - Title: עברית: איזון גובה תחתון משותף בעמודים | en: Shared Multi-Column Height Balancing
  - Description: עברית: יישור קו בסיס תחתון משותף לכל הטורים בעמוד באמצעות קפיצים אנכיים אלסטיים המפוזרים במרווחי שורות ופסקאות. | en: Harmonious bottom baseline alignment across parallel streams via distributed vertical springs across inter-line and inter-paragraph gaps.

### Changes
- [features] עברית: עגינה דו-כיוונית קנונית עם זיהוי דיבור המתחיל אוטומטי (ד"ה) עבור דפי גמרא ומקראות גדולות | en: Canonical bi-directional anchoring with automated Dibur HaMatchil (DH) cue recognition
- [features] עברית: גלישה מסונכרנת במעבר עמודים המונעת הופעת פירוש לפני העוגן המרכזי שלו (No Commentary Lead Invariant) | en: Synchronized multi-stream pagination strictly enforcing the no-commentary-lead invariant
- [typography] עברית: איזון גובה תחתון פעיל בעמודי תלמוד ומקראות גדולות (Shared Column Height Balancing) | en: Active shared column height balancing aligning column bottom baselines
- [typography] עברית: בקרת יתומים ואלמנות (Orphan/Widow Control) בחלוקת פירושים ארוכים במיוחד על פני עמודים | en: Strict orphan and widow control for oversized commentary paragraphs spanning page boundaries

---

## [0.30.4] - 2026-10-30
- **Importance**: minor
- **Badge**: עברית: גרסה נוכחית | en: Current Version
- **Title**: עברית: עדכון שיפורים, טיפוגרפיה ותאימות מקלדת | en: Enhancements, Hebrew Typography & Keyboard Usability
- **Summary**: עברית: עדכון זה מביא תמיכה מלאה במשקלי גופן (Regular/Bold) בסרגל המבנה ובקנבס, שיפור במקשי קיצור במקלדת עברית ותיקוני יציבות. | en: This update brings font weight support (Regular/Bold) across inspector and canvas, Hebrew keyboard shortcut compatibility, and stability fixes.

### Changes
- [typography] עברית: תמיכה מובנית במשקלי גופן (Bold 700 / Regular 400) בגופנים עבריים ושימורם בייצוא | en: Native font weight support (Bold 700 / Regular 400) for Hebrew fonts with faithful export retention
- [improvements] עברית: תאימות פלטת הפקודות (Ctrl+K) למקלדת עברית עם המקש "ל" ללא צורך בהחלפת שפה | en: Hebrew keyboard layout compatibility for Command Palette (Ctrl+K / Ctrl+ל)
- [fixes] עברית: מניעת השתלטות של מקש הסלאש ("/") בזמן הקלדה חופשית בעורך הטקסט | en: Prevented slash key ("/") from hijacking story editor typing
- [performance] עברית: ייעול מנגנון הרינדור של מסגרות הטקסט בקנבס בצפיפויות מסך גבוהות | en: Optimized canvas text frame rendering on high DPI displays

---

## [0.30.0] - 2026-10-15
- **Importance**: major
- **Badge**: עברית: עדכון גדול | en: Major Release
- **Title**: עברית: שחרור גרסה ראשית: מנוע עימוד רב-תזרימי ותקני דפוס מקצועיים | en: Major Release: Multi-Flow Layout Engine & Professional Pre-Press
- **Summary**: עברית: קפיצת מדרגה משמעותית לתוכנת TypesetOK עם מנוע Rust Native מהיר, יישור עברי תלת-שכבתי, ייצוא ISO PDF/X-1a ומגן שמות קדושים. | en: A massive milestone for TypesetOK featuring a Rust Native engine, 3-tier Hebrew justification, ISO PDF/X-1a pre-press export, and holy name protection.

### Highlights
- [rust-engine] icon: sparkle | badge: עברית: מהיר פי 10, en: 10x Faster
  - Title: עברית: מנוע עימוד Rust מקורי | en: Native Rust Layout Engine
  - Description: עברית: ליבת עימוד דטרמיניסטית וסופר-מהירה המבצעת שבירת שורות אופטימלית בשיטת Knuth-Plass על פני פסקאות עבריות שלמות. | en: Blazing fast deterministic layout engine running Knuth-Plass paragraph breaking across full Hebrew texts.
- [hebrew-justifier] icon: typography | badge: עברית: דיוק טיפוגרפי, en: Typographic Precision
  - Title: עברית: יישור עברי מסורתי תלת-שכבתי | en: 3-Tier Traditional Hebrew Justifier
  - Description: עברית: שילוב עדין של רווחי מילים גמישים, מתיחת אותיות רפויות (אהלתר"ם) וריווח בין-אותי מיקרומטרי ללא צורך במקפים מכוערים. | en: Harmonious blending of word spacing, wide Hebrew letter dilation, and micro letter-spacing.
- [pdf-x-export] icon: export | badge: עברית: תקן דפוס, en: Print Standard
  - Title: עברית: ייצוא תקני ISO PDF/X-1a ו-PDF/X-4 | en: ISO PDF/X-1a & PDF/X-4 Pre-Press
  - Description: עברית: הפקת קבצים מוכנים ישירות להדפסת אופסט ודפוס דיגיטלי מסחרי, עם הפרדת צבעים מלאה של 100% K לטקסט חד ונקי. | en: Production-ready PDF export for offset & digital printing, strict 100% pure black channel separation.
- [sacred-text-shield] icon: check | badge: עברית: מסורת ישראל, en: Traditional Rules
  - Title: עברית: קדושת הטקסט ונרמול קנוני | en: Sacred Text Shield & Canonical Normalization
  - Description: עברית: מניעת שבירת שמות קדושים בכללי No-Break קפדניים, סנכרון גימטריה עברית לעמודים ונרמול סדר ניקוד וטעמים. | en: Strict no-break protection for divine names, Hebrew gematria page numbering, and canonical niqqud normalization.

### Changes
- [features] עברית: מבנה עמוד רב-תזרימי מלא עבור דפי גמרא (מרכז, רש"י, תוספות, הערות) ומקראות גדולות | en: Full multi-flow page architecture for Talmud pages and rabbinic commentaries
- [features] עברית: ייצוא PDF/X-1a תקני עם פרופילי צבע Fogra 39 ו-ISO Coated v2 | en: Compliant ISO PDF/X-1a export with Fogra 39 and ISO Coated v2 profiles
- [typography] עברית: אלגוריתם מתיחת אותיות עבריות רפויות וריווח מילים גמיש | en: Hebrew dilated letter expansion and elastic word space distribution
- [improvements] עברית: ניהול פרויקטים ומסך פתיחה חדש עם תבניות מהירות לספרים, עלונים וקונטרסים | en: Project manager and fresh welcome dashboard with quick templates for books and booklets
- [fixes] עברית: תיקון חישוב גבולות עמודים בצמדי עמודים (Spreads) בספרים מימין לשמאל | en: Fixed page spread layout calculation for right-to-left Hebrew books

---

## [0.29.0] - 2026-09-20
- **Importance**: minor
- **Badge**: עברית: תוספים ועריכה | en: Plugins & Editing
- **Title**: עברית: מערכת תוספים מודולרית ועורך טקסט משולב | en: Modular Plugin System & Integrated Story Editor
- **Summary**: עברית: הוספת סביבת הרחבות מבודדת ומאובטחת לתוספים ב-TypeScript/JavaScript, לצד עורך טקסט רציף (Story Editor) עם חיפוש והחלפה. | en: Introduced an isolated plugin architecture for TypeScript/JavaScript extensions alongside a split Story Editor.

### Changes
- [features] עברית: מנוע פלאגינים מאובטח (PluginEngine) עם הרשאות מוגדרות והידור בזמן ריצה | en: Hardened plugin sandbox engine with strict permission boundaries and runtime compilation
- [features] עברית: עורך טקסט משולב בתצוגת Split View עם ספירת מילים חיה לכל תזרים | en: Integrated Story Editor with split view and live word counts per flow
- [improvements] עברית: ייבוא ישיר של קובצי Word (.docx), Markdown (.md) וטקסט רציף | en: Direct import pipeline for Word (.docx), Markdown (.md), and plain text
- [fixes] עברית: שיפור מנגנון שמירה אוטומטית ומניעת איבוד שינויים בעת יציאה | en: Enhanced auto-save safety checks and unsaved change prevention on quit

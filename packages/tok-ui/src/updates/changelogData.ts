/**
 * AUTO-GENERATED from CHANGELOG.md by scripts/sync-changelog.mjs
 * DO NOT EDIT DIRECTLY. Update CHANGELOG.md in the project root instead.
 */
import { ChangelogEntry } from './types';

export const CHANGELOG_ENTRIES: ChangelogEntry[] = [
  {
    "version": "0.34.9",
    "releaseDate": "2026-11-06",
    "importance": "minor",
    "title": {
      "he": "טיפוגרפיה מסורתית מקראית, מערך הערות רב-שכבתי, יחסי תזרימים דינמיים וסנכרון עיצוב חי",
      "en": "Biblical Masoretic Typography, Multi-Apparatus Footnotes, Dynamic Rabbinic Flow Ratios & Live Styling Pipeline"
    },
    "summary": {
      "he": "שחרור גרסה 0.34.9 המשלב תמיכה טיפוגרפית מלאה בטקסט מקראי ומסורתי (נון הפוכה, פסיק, מתג, טעמי המקרא וגליפים מקראיים ב-OpenType), פריסת הערות רב-שכבתית מקבילה (מספר מנגנוני הערות ופירושים נפרדים בתחתית העמוד), יחסי תזרימים גמישים לעימוד תלמודי ורבני רב-טורי, שימור עיצובים וטלאי טיפוגרפיה בעורך הרציף, וסנכרון מלא של צינור הסגנונות בין סרגל המאפיינים, סרגל הכלים והקנבס.",
      "en": "Release 0.34.9 delivering complete Masoretic biblical typography (Nun Hafukha, Paseq, Meteg, cantillation marks and biblical OpenType shaping variants), parallel multi-apparatus footnote layouts, flexible multi-flow column width proportions for Talmudic and rabbinic spreads, persistent typography patches in story DOM, and unified live styling pipeline across inspector, toolbar, and canvas."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "תמיכה טיפוגרפית מקיפה בטעמי המקרא, פסיק, מתג ונון הפוכה במנועי העיצוב והקדם-דפוס",
          "en": "Comprehensive biblical typography with cantillation marks, paseq, meteg, and nun hafukha in shaping and pre-press pipelines"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "פריסת הערות רב-שכבתית מקבילה (Multi-Apparatus) עם הגדרות גובה, קווי הפרדה ומספור עצמאי",
          "en": "Parallel multi-apparatus footnote layout supporting distinct apparatus configurations, custom separators, and independent numbering"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "חלוקת רוחב עמודות פרופורציונלית מותאמת אישית בתבניות 2, 3 ו-4 תזרימים (ש\"ס ומקראות גדולות)",
          "en": "Proportional column width distribution in 2, 3, and 4 stream layouts with custom width ratios and recto/verso spine tracking"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "שימור טלאי טיפוגרפיה ועיצוב מקומי בעת המרת DOM בעורך הסיפור הרציף (Story DOM)",
          "en": "Preservation of typography patches and inline span styling during bidirectional DOM conversion in story editor"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "סנכרון חי של שינויי פונט, גודל, ריווח ויישור ישירות מקנבס העריכה ומסרגל המאפיינים",
          "en": "Real-time synchronization of font family, size, leading, tracking, and alignment directly from editor canvas and inspector"
        }
      },
      {
        "category": "fixes",
        "text": {
          "he": "תיקון דרישות עיצוב קוד ב-CI ב-Rustfmt ושמירה על יציבות מלאה של צינור הבדיקות האוטומטי",
          "en": "Fix Rustfmt formatting compliance across crates/tok-typeset for CI stability"
        }
      }
    ],
    "highlights": [
      {
        "id": "masoretic-typography",
        "icon": "typography",
        "title": {
          "he": "טיפוגרפיה מקראית ומסורתית מתקדמת",
          "en": "Advanced Biblical Masoretic Typography"
        },
        "description": {
          "he": "עיצוב מדויק של תווי מסורה עבריים כולל נון הפוכה, פסיק, מתג עליון ותחתון, כללי הצמדת טעמים והחלת תכונות OpenType ייעודיות לטקסט מקראי.",
          "en": "Native biblical Hebrew shaping support including Nun Hafukha, Paseq, upper/lower Meteg, cantillation anchoring, and biblical OpenType font features."
        },
        "badge": {
          "he": "טיפוגרפיה מקראית",
          "en": "Masoretic Typography"
        }
      },
      {
        "id": "multi-apparatus",
        "icon": "layers",
        "title": {
          "he": "מערך הערות שוליים רב-שכבתי ומקבילי",
          "en": "Parallel Multi-Apparatus Footnote Bands"
        },
        "description": {
          "he": "תמיכה במגוון מנגנוני הערות מקבילים על אותו עמוד (הערות שוליים, ביאורים ומראי מקומות) עם סימונים עצמאיים והקצאת גובה מאוזנת.",
          "en": "Multiple simultaneous parallel footnote bands on the same page (notes, glosses, critical apparatus) with distinct numbering schemes and balanced height allocation."
        },
        "badge": {
          "he": "מערך הערות מקבילי",
          "en": "Multi-Apparatus"
        }
      },
      {
        "id": "rabbinic-flow-ratios",
        "icon": "flows",
        "title": {
          "he": "יחסי רוחב דינמיים לתזרימים רבניים",
          "en": "Dynamic Flow Width Ratios for Rabbinic Layouts"
        },
        "description": {
          "he": "שליטה מדויקת ביחסי רוחב עמודות בעימוד ש\"ס ומקראות גדולות (2, 3 ו-4 תזרימים) תוך שמירה מלאה על עקרון שדרה-שוליים recto/verso.",
          "en": "Precise column width ratio distribution for 2, 3, and 4 stream Talmudic and Mikraot Gedolot spreads with full recto/verso spine-margin symmetry."
        },
        "badge": {
          "he": "יחסי תזרימים",
          "en": "Flow Ratios"
        }
      },
      {
        "id": "live-styling-pipeline",
        "icon": "sliders",
        "title": {
          "he": "סנכרון סגנונות וטיפוגרפיה חי בין הרכיבים",
          "en": "Real-Time Styling Pipeline Synchronization"
        },
        "description": {
          "he": "חיבור ישיר ורציף בין עריכת סגנונות בסרגל המאפיינים, פקדי הטיפוגרפיה בסרגל העליון, והקרנת ה-DOM בקנבס עם שימור טלאי עיצוב ב-Story DOM.",
          "en": "Continuous bidirectional synchronization between inspector properties, toolbar typography controls, and canvas DOM projection with lossless patch preservation."
        },
        "badge": {
          "he": "סנכרון סגנונות חי",
          "en": "Live Styling"
        }
      }
    ],
    "badge": {
      "he": "גרסה נוכחית",
      "en": "Current Version"
    }
  },
  {
    "version": "0.34.4",
    "releaseDate": "2026-11-06",
    "importance": "minor",
    "title": {
      "he": "הזזה חופשית ונעילת תיבות בעורך, תזרימים מותאמים אישית (טורים מפוצלים), סינון סגנונות ובחירת תבנית מודרכת",
      "en": "Free Frame Movement & Locking, Custom Flows with Split Edge Layout, Active Flow Style Filtering & Guided Template Creation"
    },
    "summary": {
      "he": "שחרור גרסה 0.34.4 המוסיף שליטה ישירה ואינטואיטיבית בתיבות טקסט על גבי הקנבס כולל גרירה חופשית ונעילת מיקום, אפשרות הוספת תזרימים מותאמים אישית עם תמיכה בבלוקים מפוצלים בקצוות הזורמים לפי גובה, סינון סגנונות ממוקד לפי התבנית הפעילה, ומסלול פתיחת מסמך חדש מודרך עם בחירת תבנית מיידית והמלצה על ספר רציף.",
      "en": "Release 0.34.4 introducing direct canvas text frame dragging and location locking, custom text flow creation with split edge dual-margin block streaming by height, template-specific style filtering in structure bar, and guided new document creation with immediate template selection defaulting to continuous prose."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "גרירה ושינוי גודל אינטראקטיביים של תיבות טקסט בעורך עם עדכון קואורדינטות חי בסרגל המאפיינים",
          "en": "Interactive text frame dragging and resizing directly on canvas with live coordinate synchronization in inspector"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "מנגנון נעילת תיבות טקסט (Lock/Unlock) מתגית התיבה ובסרגל המאפיינים למניעת שינויים לא רצויים",
          "en": "Text frame position and size locking toggle on frame tag and contextual inspector"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "הוספת תזרימי טקסט חדשים בכל תבנית ספר דרך כפתור קומפקטי וחלונית הגדרה ייעודית",
          "en": "Custom text flow creation across any layout template via a compact button and dedicated modal dialog"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "תמיכה במבנה בלוקים מפוצלים בקצוות (שמאל וימין ללא חיבור ביניהם עם זרימה לפי גובה) לתזרימים מותאמים",
          "en": "Split edge dual margin layout blocks flowing text by height without intermediate connecting elements"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "סינון סגנונות בסרגל הימני להצגת סגנונות רלוונטיים בלבד לתבנית ולתזרימים הפעילים במסמך",
          "en": "Dynamic style token filtering in structure bar displaying exclusively styles relevant to active template flows"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "יצירת מסמך חדש פותחת מיד את מסך בחירת התבנית, וברירת המחדל מוגדרת ל'ספר רציף'",
          "en": "New document creation presents template chooser immediately, defaulting to continuous prose layout"
        }
      }
    ],
    "highlights": [
      {
        "id": "frame-drag-lock",
        "icon": "layers",
        "title": {
          "he": "הזזה חופשית ונעילה ישירה על הקנבס",
          "en": "Direct Canvas Frame Dragging & Locking"
        },
        "description": {
          "he": "אפשרות גרירה ושינוי גודל ישירים של תיבות טקסט במרחב העבודה עם מנגנון נעילה להגנה מפני תזוזה מקרית.",
          "en": "Freehand dragging and resizing of text frames directly on the editor canvas with instant locking guards."
        },
        "badge": {
          "he": "הזזה ונעילה",
          "en": "Drag & Lock"
        }
      },
      {
        "id": "custom-flows",
        "icon": "flows",
        "title": {
          "he": "הוספת תזרימים ובלוקים מפוצלים בקצוות",
          "en": "Custom Text Flows & Split Edge Layout"
        },
        "description": {
          "he": "כפתור ייעודי קומפקטי וחלונית עיצוב להוספת תזרימי טקסט חדשים, כולל חצי בלוק בקצה אחד וחצי בקצה שני ללא חיבור ביניהם הזורמים לפי גובה.",
          "en": "Dedicated compact button and configuration modal for custom flows, featuring dual split edge margin blocks streaming text by height."
        },
        "badge": {
          "he": "תזרימים מותאמים",
          "en": "Custom Flows"
        }
      },
      {
        "id": "template-onboarding",
        "icon": "templateBook",
        "title": {
          "he": "בחירת תבנית מודרכת וברירת מחדל ספר רציף",
          "en": "Guided Template Picker & Prose Default"
        },
        "description": {
          "he": "פתיחת מסמך חדש מציגה ישירות את חלון בחירת התבנית המעוצב עם המלצה ברורה על תבנית ספר רציף.",
          "en": "New document workflow prompts layout template chooser immediately, with continuous prose set as default recommendation."
        },
        "badge": {
          "he": "בחירת תבנית",
          "en": "Guided Templates"
        }
      }
    ],
    "badge": {
      "he": "גרסה קודמת",
      "en": "Previous Version"
    }
  },
  {
    "version": "0.34.3",
    "releaseDate": "2026-11-06",
    "importance": "major",
    "title": {
      "he": "מרחב עבודה מודרני, שמירה אוטומטית מתמשכת, סרגלים מודולריים ומנוע טיפוגרפיה מורחב",
      "en": "Modern Workspace UI, Continuous Autosave, Modular Sidebars & Enhanced Typography"
    },
    "summary": {
      "he": "שחרור רשמי של גרסה 0.34.3 הכולל עיצוב מחדש של מרחב העבודה עם פס כותרת דק (32px) ובקרות חלון מובנות, סרגלים מודולריים מתקפלים, מערכת שמירה אוטומטית מתמשכת ברקע ושחזור טיוטות עמיד, התאמת ניגודיות, סילוק פרויקטי דמו ואריזת שולחן עבודה מוקשחת.",
      "en": "Official release 0.34.3 delivering modern workspace redesign with slim 32px titlebar and native window controls, modular collapsible sidebars, continuous background autosave and resilient draft recovery, contrast harmonization, demo project cleanup, and hardened desktop packaging."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "פס כותרת דק (32px) במרכז החלון עם בקרות מובנות (מזעור, הגדלה, סגירה) ועריכת שם פרויקט בלחיצה כפולה",
          "en": "Centered slim 32px titlebar with window controls and double-click rename"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "חלוקת סרגל המאפיינים השמאלי לכרטיסיות מופרדות בקו ובגוון בהיר יותר עם כפתור צמצום והרחבה",
          "en": "Left inspector partitioned into distinct cards with dividing lines, subtle brightness contrast, and collapse/expand toggle"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "כפתור מה חדש עם אייקון כוכב עדכונים ייעודי בצד שמאל למעלה ותצוגה תמציתית של יומן הגרסאות",
          "en": "What's New button with star icon at top-left and concise changelog summary presentation"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "סרגל ימני מתקפל בעיצוב מסילה ופאנל עם כפתור קיפול כיווני וכפתור אודות תחתון",
          "en": "Right structure bar with rail/panel layout, directional collapse button, and bottom about button"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "התאמת ניגודיות צבעים והפרדות קו בכל ערכות הנושא של התוכנה (כהה, בהיר וניגודיות גבוהה)",
          "en": "Harmonized contrast and line separators across all themes (dark, light, and high contrast)"
        }
      },
      {
        "category": "fixes",
        "text": {
          "he": "הסרת פרויקטי דמו ממסך הבית, מניעת רישום פרויקטי דמה בהיסטוריה וטיהור רשומות ישנות מזיכרון התוכנה",
          "en": "Purged demo projects from home screen and storage, preventing demo entries in recent projects history"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "מנגנון שמירה אוטומטית מתמשכת ברקע (Autosave) ואינדיקטור שינויים שטרם נשמרו",
          "en": "Continuous background autosave mechanism with unsaved change indicators"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "שחזור טיוטות אוטומטי (Draft Recovery) במקרה של יציאה או סגירה בלתי צפויה",
          "en": "Automated draft recovery prompt on application startup after abnormal termination"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "הגנת סגירת חלון הממתינה להשלמת שמירה בדיסק למניעת איבוד מידע",
          "en": "Window close guard strictly awaiting pending save operations before closing"
        }
      },
      {
        "category": "fixes",
        "text": {
          "he": "הבטחת בניית חבילת UI רעננה בתהליך אריזת ה-Desktop ותיעוד שגיאות קונסול מרנדרר",
          "en": "Ensured fresh UI bundle compilation during desktop packaging and added renderer console error logging"
        }
      }
    ],
    "highlights": [
      {
        "id": "modern-workspace",
        "icon": "sparkle",
        "title": {
          "he": "מרחב עבודה נקי ורחב",
          "en": "Uncluttered Expansive Workspace"
        },
        "description": {
          "he": "הארכת סרגלי הצד עד לפס הכותרת הדק (32px) במקום הסרגל העליון הישן, ליצירת משטח עבודה רציף ללא הסחות דעת.",
          "en": "Full-height sidebars reaching the slim 32px titlebar for a continuous, distraction-free typesetting canvas."
        },
        "badge": {
          "he": "מרחב עבודה",
          "en": "Workspace"
        }
      },
      {
        "id": "collapsible-sidebars",
        "icon": "layers",
        "title": {
          "he": "שליטה מודולרית בסרגלי המבנה והמאפיינים",
          "en": "Modular Structure and Inspector Controls"
        },
        "description": {
          "he": "כפתורי הרחבה וקיפול ייעודיים, כרטיסיות מופרדות ויזואלית בשינוי בהירות עדין, וכפתורי הגדרות ואודות נגישים בתחתית.",
          "en": "Dedicated expand/collapse toggles, visual brightness contrast segmentation, and accessible bottom settings/about buttons."
        },
        "badge": {
          "he": "סרגלים מתקפלים",
          "en": "Collapsible Sidebars"
        }
      },
      {
        "id": "autosave-resilience",
        "icon": "check",
        "title": {
          "he": "שמירה אוטומטית מתמשכת ושחזור טיוטות",
          "en": "Continuous Autosave and Draft Recovery"
        },
        "description": {
          "he": "שמירה ברקע בכל עריכה, שחזור טיוטה מוגן לאחר סגירה פתאומית, וחסימת סגירת חלון עד לסיום כתיבת המסמך לדיסק.",
          "en": "Continuous background document saving, crash draft recovery, and window-close guards awaiting disk write completion."
        },
        "badge": {
          "he": "שמירה רציפה",
          "en": "Continuous Autosave"
        }
      }
    ],
    "badge": {
      "he": "גרסה קודמת",
      "en": "Previous Version"
    }
  },
  {
    "version": "0.33.0",
    "releaseDate": "2026-10-31",
    "importance": "major",
    "title": {
      "he": "מהפכת \"צורת הדף\" חיה, עגינה רב-תזרימית והקשחת דטרמיניזם",
      "en": "Living Tzurat HaDaf, Dynamic Multi-Flow Anchoring & Pre-Press Hardening"
    },
    "summary": {
      "he": "שחרור רשמי של גרסה 0.33.0 הכוללת עימוד רב-תזרימי חי לש\"ס ומקראות גדולות עם עגינה דו-כיוונית, גלישת עמודים מסונכרנת בזמן אמת, איזון גבהי טורים משותף, הקשחת מנוע הדטרמיניזם ואוטומציית שחרור גרסאות מלאה.",
      "en": "Official release 0.33.0 introducing Living Tzurat HaDaf with bi-directional multi-flow anchoring, real-time synchronized pagination cascading, shared column height balancing, determinism engine hardening, and automated CI/CD release workflow."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "עגינה דו-כיוונית קנונית עם זיהוי דיבור המתחיל אוטומטי (ד\"ה) עבור דפי גמרא ומקראות גדולות",
          "en": "Canonical bi-directional anchoring with automated Dibur HaMatchil (DH) cue recognition"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "גלישה מסונכרנת במעבר עמודים המונעת הופעת פירוש לפני העוגן המרכזי שלו (No Commentary Lead Invariant)",
          "en": "Synchronized multi-stream pagination strictly enforcing the no-commentary-lead invariant"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "איזון גובה תחתון פעיל בעמודי תלמוד ומקראות גדולות (Shared Column Height Balancing)",
          "en": "Active shared column height balancing aligning column bottom baselines"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "בקרת יתומים ואלמנות (Orphan/Widow Control) בחלוקת פירושים ארוכים במיוחד על פני עמודים",
          "en": "Strict orphan and widow control for oversized commentary paragraphs spanning page boundaries"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "יישור אופטי של שוליים (Optical Margin Alignment) לסימני פיסוק וגליפיים בולטים בעברית ובכיווניות מעורבת",
          "en": "Professional Optical Margin Alignment for punctuation and overhanging glyphs with full RTL and Hebrew support"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "איזון שורה אחרונה מאולץ באלגוריתם Knuth-Plass כולל אילוצי רוחב מינימלי ומיקום אופטימלי",
          "en": "Knuth-Plass constrained last-line balancing with minimum width constraints and optimal final-line placement"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "מנוע פוליגונים כללי וטורים ברוחב משתנה (L-Shape & Variable Columns) עם חיתוך רצועות שמרני",
          "en": "Arbitrary polygonal text regions, variable column geometry, and conservative band measure-profile generation"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "חלון מילת פתיחה עברי (Hebrew Drop Caps) עם מדידת תיבה תוחמת מדויקת של גליפים, ניקוד וטעמים",
          "en": "Hebrew Drop Caps with exact glyph outline bounding boxes and multi-line text-exclusion geometry"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "שיפור מנגנון ה-CI ואיחוד בדיקות הדטרמיניזם להבטחת שחרור אוטומטי חלק",
          "en": "Streamlined CI determinism verification and automated release pipeline trigger"
        }
      }
    ],
    "highlights": [
      {
        "id": "living-tzurat-hadaf",
        "icon": "layers",
        "title": {
          "he": "מנוע עגינה רב-תזרימי דו-כיווני",
          "en": "Bi-Directional Multi-Flow Anchor Cascade Engine"
        },
        "description": {
          "he": "קישור לוגי אוטומטי בין פסקאות הטקסט הראשי למפרשים (רש\"י ותוספות), המאפשר גלישה והתרחבות דינמית בזמן אמת.",
          "en": "Strict logical binding between primary text segments and commentaries with dynamic cascading and absorption."
        },
        "badge": {
          "he": "צורת הדף חיה",
          "en": "Living Tzurat HaDaf"
        }
      },
      {
        "id": "determinism-hardening",
        "icon": "sparkle",
        "title": {
          "he": "הקשחת דטרמיניזם מוחלט בכל שלבי העימוד והייצוא",
          "en": "Absolute Pre-Press Output Determinism"
        },
        "description": {
          "he": "הבטחת שחזור ביט-אחר-ביט בייצוא PDF/X-1a ו-HTML בכל פלטפורמה, ללא תלות בסידורי גיבוב או מצבי זיכרון.",
          "en": "Guarantees bit-for-bit reproducible PDF/X-1a and HTML output across all platforms and execution environments."
        },
        "badge": {
          "he": "דטרמיניזם מלא",
          "en": "Full Determinism"
        }
      }
    ],
    "badge": {
      "he": "גרסה קודמת",
      "en": "Previous Version"
    }
  },
  {
    "version": "0.31.0",
    "releaseDate": "2026-10-31",
    "importance": "major",
    "title": {
      "he": "עימוד רב-תזרימי חי (\"צורת הדף\" דינמית) וסנכרון עוגנים דו-כיווני",
      "en": "Dynamic Multi-Flow Anchoring (\"Living Tzurat HaDaf\") & Synchronized Pagination"
    },
    "summary": {
      "he": "מימוש מלא של עימוד רב-תזרימי חי לתלמוד ומקראות גדולות: עגינה דו-כיוונית בין טקסט מרכזי לפירושים, גלישת עמודים מסונכרנת בזמן אמת, ואיזון גובה תחתון דינמי בכל הטורים.",
      "en": "Full end-to-end implementation of Living Tzurat HaDaf: bi-directional anchor linking between primary text and commentaries, real-time synchronized pagination across page boundaries, and dynamic shared column height balancing."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "עגינה דו-כיוונית קנונית עם זיהוי דיבור המתחיל אוטומטי (ד\"ה) עבור דפי גמרא ומקראות גדולות",
          "en": "Canonical bi-directional anchoring with automated Dibur HaMatchil (DH) cue recognition"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "גלישה מסונכרנת במעבר עמודים המונעת הופעת פירוש לפני העוגן המרכזי שלו (No Commentary Lead Invariant)",
          "en": "Synchronized multi-stream pagination strictly enforcing the no-commentary-lead invariant"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "איזון גובה תחתון פעיל בעמודי תלמוד ומקראות גדולות (Shared Column Height Balancing)",
          "en": "Active shared column height balancing aligning column bottom baselines"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "בקרת יתומים ואלמנות (Orphan/Widow Control) בחלוקת פירושים ארוכים במיוחד על פני עמודים",
          "en": "Strict orphan and widow control for oversized commentary paragraphs spanning page boundaries"
        }
      }
    ],
    "highlights": [
      {
        "id": "multi-flow-anchoring",
        "icon": "layers",
        "title": {
          "he": "מנגנון עגינה דו-כיווני וסנכרון עמודים חי",
          "en": "Bi-Directional Multi-Flow Anchoring"
        },
        "description": {
          "he": "קישור לוגי הדוק בין פסקאות הגמרא/המקרא לפירושי רש\"י ותוספות, המבטיח גלישה אוטומטית של פירושים לעמוד הבא כאשר הטקסט המרכזי גולש.",
          "en": "Strict logical binding between primary text segments and commentaries ensuring automatic cascading of commentaries when primary text reflows."
        },
        "badge": {
          "he": "צורת הדף חיה",
          "en": "Living Tzurat HaDaf"
        }
      },
      {
        "id": "height-balancing",
        "icon": "align",
        "title": {
          "he": "איזון גובה תחתון משותף בעמודים",
          "en": "Shared Multi-Column Height Balancing"
        },
        "description": {
          "he": "יישור קו בסיס תחתון משותף לכל הטורים בעמוד באמצעות קפיצים אנכיים אלסטיים המפוזרים במרווחי שורות ופסקאות.",
          "en": "Harmonious bottom baseline alignment across parallel streams via distributed vertical springs across inter-line and inter-paragraph gaps."
        },
        "badge": {
          "he": "איזון טיפוגרפי",
          "en": "Typographic Balance"
        }
      }
    ],
    "badge": {
      "he": "עדכון גדול",
      "en": "Major Release"
    }
  },
  {
    "version": "0.30.4",
    "releaseDate": "2026-10-30",
    "importance": "minor",
    "title": {
      "he": "עדכון שיפורים, טיפוגרפיה ותאימות מקלדת",
      "en": "Enhancements, Hebrew Typography & Keyboard Usability"
    },
    "summary": {
      "he": "עדכון זה מביא תמיכה מלאה במשקלי גופן (Regular/Bold) בסרגל המבנה ובקנבס, שיפור במקשי קיצור במקלדת עברית ותיקוני יציבות.",
      "en": "This update brings font weight support (Regular/Bold) across inspector and canvas, Hebrew keyboard shortcut compatibility, and stability fixes."
    },
    "changes": [
      {
        "category": "typography",
        "text": {
          "he": "תמיכה מובנית במשקלי גופן (Bold 700 / Regular 400) בגופנים עבריים ושימורם בייצוא",
          "en": "Native font weight support (Bold 700 / Regular 400) for Hebrew fonts with faithful export retention"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "תאימות פלטת הפקודות (Ctrl+K) למקלדת עברית עם המקש \"ל\" ללא צורך בהחלפת שפה",
          "en": "Hebrew keyboard layout compatibility for Command Palette (Ctrl+K / Ctrl+ל)"
        }
      },
      {
        "category": "fixes",
        "text": {
          "he": "מניעת השתלטות של מקש הסלאש (\"/\") בזמן הקלדה חופשית בעורך הטקסט",
          "en": "Prevented slash key (\"/\") from hijacking story editor typing"
        }
      },
      {
        "category": "performance",
        "text": {
          "he": "ייעול מנגנון הרינדור של מסגרות הטקסט בקנבס בצפיפויות מסך גבוהות",
          "en": "Optimized canvas text frame rendering on high DPI displays"
        }
      }
    ],
    "highlights": []
  },
  {
    "version": "0.30.0",
    "releaseDate": "2026-10-15",
    "importance": "major",
    "title": {
      "he": "שחרור גרסה ראשית: מנוע עימוד רב-תזרימי ותקני דפוס מקצועיים",
      "en": "Major Release: Multi-Flow Layout Engine & Professional Pre-Press"
    },
    "summary": {
      "he": "קפיצת מדרגה משמעותית לתוכנת TypesetOK עם מנוע Rust Native מהיר, יישור עברי תלת-שכבתי, ייצוא ISO PDF/X-1a ומגן שמות קדושים.",
      "en": "A massive milestone for TypesetOK featuring a Rust Native engine, 3-tier Hebrew justification, ISO PDF/X-1a pre-press export, and holy name protection."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "מבנה עמוד רב-תזרימי מלא עבור דפי גמרא (מרכז, רש\"י, תוספות, הערות) ומקראות גדולות",
          "en": "Full multi-flow page architecture for Talmud pages and rabbinic commentaries"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "ייצוא PDF/X-1a תקני עם פרופילי צבע Fogra 39 ו-ISO Coated v2",
          "en": "Compliant ISO PDF/X-1a export with Fogra 39 and ISO Coated v2 profiles"
        }
      },
      {
        "category": "typography",
        "text": {
          "he": "אלגוריתם מתיחת אותיות עבריות רפויות וריווח מילים גמיש",
          "en": "Hebrew dilated letter expansion and elastic word space distribution"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "ניהול פרויקטים ומסך פתיחה חדש עם תבניות מהירות לספרים, עלונים וקונטרסים",
          "en": "Project manager and fresh welcome dashboard with quick templates for books and booklets"
        }
      },
      {
        "category": "fixes",
        "text": {
          "he": "תיקון חישוב גבולות עמודים בצמדי עמודים (Spreads) בספרים מימין לשמאל",
          "en": "Fixed page spread layout calculation for right-to-left Hebrew books"
        }
      }
    ],
    "highlights": [
      {
        "id": "rust-engine",
        "icon": "sparkle",
        "title": {
          "he": "מנוע עימוד Rust מקורי",
          "en": "Native Rust Layout Engine"
        },
        "description": {
          "he": "ליבת עימוד דטרמיניסטית וסופר-מהירה המבצעת שבירת שורות אופטימלית בשיטת Knuth-Plass על פני פסקאות עבריות שלמות.",
          "en": "Blazing fast deterministic layout engine running Knuth-Plass paragraph breaking across full Hebrew texts."
        },
        "badge": {
          "he": "מהיר פי 10",
          "en": "10x Faster"
        }
      },
      {
        "id": "hebrew-justifier",
        "icon": "typography",
        "title": {
          "he": "יישור עברי מסורתי תלת-שכבתי",
          "en": "3-Tier Traditional Hebrew Justifier"
        },
        "description": {
          "he": "שילוב עדין של רווחי מילים גמישים, מתיחת אותיות רפויות (אהלתר\"ם) וריווח בין-אותי מיקרומטרי ללא צורך במקפים מכוערים.",
          "en": "Harmonious blending of word spacing, wide Hebrew letter dilation, and micro letter-spacing."
        },
        "badge": {
          "he": "דיוק טיפוגרפי",
          "en": "Typographic Precision"
        }
      },
      {
        "id": "pdf-x-export",
        "icon": "export",
        "title": {
          "he": "ייצוא תקני ISO PDF/X-1a ו-PDF/X-4",
          "en": "ISO PDF/X-1a & PDF/X-4 Pre-Press"
        },
        "description": {
          "he": "הפקת קבצים מוכנים ישירות להדפסת אופסט ודפוס דיגיטלי מסחרי, עם הפרדת צבעים מלאה של 100% K לטקסט חד ונקי.",
          "en": "Production-ready PDF export for offset & digital printing, strict 100% pure black channel separation."
        },
        "badge": {
          "he": "תקן דפוס",
          "en": "Print Standard"
        }
      },
      {
        "id": "sacred-text-shield",
        "icon": "check",
        "title": {
          "he": "קדושת הטקסט ונרמול קנוני",
          "en": "Sacred Text Shield & Canonical Normalization"
        },
        "description": {
          "he": "מניעת שבירת שמות קדושים בכללי No-Break קפדניים, סנכרון גימטריה עברית לעמודים ונרמול סדר ניקוד וטעמים.",
          "en": "Strict no-break protection for divine names, Hebrew gematria page numbering, and canonical niqqud normalization."
        },
        "badge": {
          "he": "מסורת ישראל",
          "en": "Traditional Rules"
        }
      }
    ],
    "badge": {
      "he": "עדכון גדול",
      "en": "Major Release"
    }
  },
  {
    "version": "0.29.0",
    "releaseDate": "2026-09-20",
    "importance": "minor",
    "title": {
      "he": "מערכת תוספים מודולרית ועורך טקסט משולב",
      "en": "Modular Plugin System & Integrated Story Editor"
    },
    "summary": {
      "he": "הוספת סביבת הרחבות מבודדת ומאובטחת לתוספים ב-TypeScript/JavaScript, לצד עורך טקסט רציף (Story Editor) עם חיפוש והחלפה.",
      "en": "Introduced an isolated plugin architecture for TypeScript/JavaScript extensions alongside a split Story Editor."
    },
    "changes": [
      {
        "category": "features",
        "text": {
          "he": "מנוע פלאגינים מאובטח (PluginEngine) עם הרשאות מוגדרות והידור בזמן ריצה",
          "en": "Hardened plugin sandbox engine with strict permission boundaries and runtime compilation"
        }
      },
      {
        "category": "features",
        "text": {
          "he": "עורך טקסט משולב בתצוגת Split View עם ספירת מילים חיה לכל תזרים",
          "en": "Integrated Story Editor with split view and live word counts per flow"
        }
      },
      {
        "category": "improvements",
        "text": {
          "he": "ייבוא ישיר של קובצי Word (.docx), Markdown (.md) וטקסט רציף",
          "en": "Direct import pipeline for Word (.docx), Markdown (.md), and plain text"
        }
      },
      {
        "category": "fixes",
        "text": {
          "he": "שיפור מנגנון שמירה אוטומטית ומניעת איבוד שינויים בעת יציאה",
          "en": "Enhanced auto-save safety checks and unsaved change prevention on quit"
        }
      }
    ],
    "highlights": [],
    "badge": {
      "he": "תוספים ועריכה",
      "en": "Plugins & Editing"
    }
  }
];

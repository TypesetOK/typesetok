import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button, kbd } from '../ui';
import { ModalController } from './ModalController';
import { IconName } from '../icons';

export interface UserGuideModalCallbacks {
  onClose: () => void;
}

interface GuideBlock {
  title?: string;
  body: string;
  items?: string[];
  /** Friendly highlighted hint shown under the block. */
  tip?: string;
}

interface GuideSection {
  id: string;
  titleHe: string;
  titleEn: string;
  icon: IconName;
  contentHe: GuideBlock[];
  contentEn: GuideBlock[];
}

const GUIDE_SECTIONS: GuideSection[] = [
  {
    id: 'start',
    titleHe: 'התחלה מהירה',
    titleEn: 'Quick Start',
    icon: 'brand',
    contentHe: [
      {
        title: 'ברוכים הבאים ל-TypesetOK',
        body: 'TypesetOK (TOK) היא תוכנת עימוד לטקסט עברי, לספרי קודש ולספרי עיון. אתם כותבים או מייבאים את הטקסט, בוחרים פריסה, והתוכנה מסדרת את הדפים בעצמה: שורות מיושרות, ניקוד וטעמים במקומם, ומספור עמודים בגימטריה.'
      },
      {
        title: 'ארבעה צעדים ראשונים',
        body: 'כך מגיעים מטקסט גולמי לדף מוכן לדפוס:',
        items: [
          'בדף הבית בחרו "ייבוא מסמך" (קובץ Word‏ ‎.docx‎, ‏Markdown‏ ‎.md‎ או טקסט פשוט ‎.txt‎), או "מסמך ריק" אם אתם מתחילים מאפס.',
          'בחרו פריסה שמתאימה לספר שלכם: דף גמרא, מקראות גדולות, ספר רציף, ספר עם הערות או עלון.',
          'עברו על הטקסט בעורך, ובמידת הצורך הפעילו את אשף הניקוי (Ctrl+Shift+C) שמסדר רווחים, גרשיים ופתיחי פסקאות.',
          'שמרו (Ctrl+S) וייצאו ל-PDF לדפוס (Ctrl+E).'
        ],
        tip: 'לא צריך לזכור הכול. Ctrl+K פותח לוח פקודות שבו אפשר לחפש כל פעולה בשמה.'
      }
    ],
    contentEn: [
      {
        title: 'Welcome to TypesetOK',
        body: 'TypesetOK (TOK) is a typesetting app for Hebrew text, sacred texts and scholarly books. You write or import your text, pick a layout, and the app sets the pages for you: justified lines, vowel points and cantillation marks in place, and page numbers in Hebrew numerals.'
      },
      {
        title: 'Your first four steps',
        body: 'This is the path from raw text to a print-ready page:',
        items: [
          'On the home screen choose "Import document" (Word .docx, Markdown .md or plain text .txt), or "Blank document" to start from scratch.',
          'Pick the layout that fits your book: Talmud page, Mikraot Gedolot, continuous book, book with notes, or bulletin.',
          'Review the text in the editor and, if you like, run the Text Cleanup wizard (Ctrl+Shift+C) to tidy spaces, quotation marks and paragraph openings.',
          'Save (Ctrl+S) and export to a print PDF (Ctrl+E).'
        ],
        tip: 'You do not have to memorize anything. Ctrl+K opens a command palette where you can search any action by name.'
      }
    ]
  },
  {
    id: 'documents',
    titleHe: 'פתיחה, ייבוא ושמירה',
    titleEn: 'Opening, Importing & Saving',
    icon: 'folder',
    contentHe: [
      {
        title: 'דף הבית',
        body: 'דף הבית מציג את שתי נקודות ההתחלה, "מסמך ריק" ו"ייבוא מסמך", ומימין או משמאל רשימת "פרויקטים אחרונים". לחיצה על פרויקט ברשימה פותחת אותו מיד.',
        items: [
          'כדי לפתוח קובץ TOK שמור (‎.tok‎): גררו אותו אל האזור המתאים בדף הבית, או לחצו "פתיחת קובץ" (Ctrl+O).',
          'כדי להסיר פרויקט מהרשימה: לחצו על ה-X שלידו. ההסרה נוגעת רק ברשימה, והקובץ עצמו לא נמחק.',
          'את דף הבית אפשר לפתוח בכל רגע עם Ctrl+Shift+P.'
        ]
      },
      {
        title: 'ייבוא מסמך קיים',
        body: 'בחרו קובץ Word‏ (‎.docx‎), ‏Markdown‏ (‎.md‎) או טקסט פשוט (‎.txt‎). לאחר הבחירה תתבקשו לבחור פריסה. כשתפתחו שוב את אותו קובץ מרשימת הפרויקטים האחרונים, התוכנה תזכור את הפריסה ותדלג על השאלה.',
        tip: 'רוצים פריסה אחרת לאותו קובץ? ייבאו אותו שוב עם Ctrl+I והבחירה תוצג מחדש. הייבוא טוען את הקובץ המקורי מההתחלה, ולכן שמרו קודם את מה שערכתם.'
      },
      {
        title: 'שמירה ושם המסמך',
        body: 'Ctrl+S שומר את המסמך כקובץ TOK, ו-Ctrl+Shift+S שומר עותק בשם אחר. שם המסמך משתנה ב-F2. אם תנסו לסגור את התוכנה כשיש שינויים שלא נשמרו, היא תשאל אם לשמור לפני היציאה.'
      }
    ],
    contentEn: [
      {
        title: 'The home screen',
        body: 'The home screen offers the two starting points, "Blank document" and "Import document", next to a list of "Recent projects". Click a project in the list to open it right away.',
        items: [
          'To open a saved TOK file (.tok): drag it onto the matching area of the home screen, or click "Open file" (Ctrl+O).',
          'To remove a project from the list: click the X next to it. This only removes the list entry; the file itself is never deleted.',
          'You can bring the home screen back at any time with Ctrl+Shift+P.'
        ]
      },
      {
        title: 'Importing an existing document',
        body: 'Pick a Word (.docx), Markdown (.md) or plain text (.txt) file. You will then be asked to choose a layout. When you reopen the same file from Recent projects, the app remembers your layout and skips the question.',
        tip: 'Want a different layout for the same file? Import it again with Ctrl+I and the choice is shown again. Importing reloads the original file from scratch, so save what you edited first.'
      },
      {
        title: 'Saving and naming',
        body: 'Ctrl+S saves the document as a TOK file and Ctrl+Shift+S saves a copy under another name. Rename the document with F2. If you try to quit with unsaved changes, the app asks whether to save before exiting.'
      }
    ]
  },
  {
    id: 'layouts',
    titleHe: 'בחירת פריסת עימוד',
    titleEn: 'Choosing a Layout',
    icon: 'templateBook',
    contentHe: [
      {
        title: 'חמש פריסות לבחירה',
        body: 'הפריסה קובעת כמה טורים יש בדף ואילו טקסטים נכנסים אליהם. אלה האפשרויות:',
        items: [
          'דף גמרא: פריסה קלאסית עם טקסט הגמרא במרכז, רש״י בטור הפנימי ותוספות בטור החיצוני.',
          'מקראות גדולות: טקסט המקרא המנוקד עם טעמים בראש הדף, ותרגום ומפרשים תחתיו.',
          'ספר רציף: טור יחיד ומאוזן לספרי קריאה, הגות ועיון.',
          'ספר עם הערות: טקסט ראשי רציף עם אזור הערות שוליים בתחתית הדף.',
          'עלון וקונטרס: מבנה דו-טורי עם כותרת רוחבית, מתאים לעלוני שבת וחוברות.'
        ],
        tip: 'לא בטוחים? ספר רגיל עם טקסט אחד מתאים ל"ספר רציף". אם יש לכם הערות שוליים, בחרו "ספר עם הערות".'
      },
      {
        title: 'מה קורה לטקסט בכל פריסה',
        body: 'בפריסה עם כמה טקסטים (גמרא, מקראות, ספר עם הערות) מופיע כל טקסט כ"תזרים" נפרד בסרגל הצדדי. פירוט בפרק "טקסט ראשי ומפרשים".'
      }
    ],
    contentEn: [
      {
        title: 'Five layouts to choose from',
        body: 'The layout decides how many columns a page has and which texts go into them. The options are:',
        items: [
          'Talmud page: the classic layout with Gemara in the center, Rashi in the inner column and Tosafot in the outer column.',
          'Mikraot Gedolot: pointed biblical text with cantillation at the top, with translation and commentaries below.',
          'Continuous book: a single balanced column for reading books, essays and scholarship.',
          'Book with notes: continuous main text with a footnote area at the bottom of the page.',
          'Bulletin & booklet: a two-column structure with a spanning header, suited to weekly bulletins and booklets.'
        ],
        tip: 'Not sure? A regular book with one text fits "Continuous book". If you have footnotes, choose "Book with notes".'
      },
      {
        title: 'What happens to your text in each layout',
        body: 'In layouts with several texts (Talmud, Mikraot, book with notes) each text appears as its own "flow" in the side panel. More in the chapter "Main Text & Commentaries".'
      }
    ]
  },
  {
    id: 'workspace',
    titleHe: 'סביבת העבודה ומצבי תצוגה',
    titleEn: 'The Workspace & View Modes',
    icon: 'canvas',
    contentHe: [
      {
        title: 'ארבעת אזורי העבודה',
        body: 'חלון העבודה בנוי מכמה חלקים קבועים:',
        items: [
          'הסרגל העליון: תפריט הקבצים, כותרת המסמך, מעבר בין מצבי תצוגה, חיפוש פקודות וביטול/ביצוע חוזר.',
          'הסרגל הצדדי: עמודים, תזרימי טקסט (למשל גמרא, רש״י ותוספות), סגנונות ושכבות.',
          'לוח העבודה במרכז: תצוגת העמודים, עם שוליים וקווי עזר.',
          'סרגל המצב התחתון: ספירת מילים, התזרים הפעיל, מספר העמוד ובקרת התקריב.'
        ]
      },
      {
        title: 'שלושה מצבי תצוגה',
        body: 'עוברים ביניהם מהסרגל העליון או בקיצורים:',
        items: [
          'עמודים (Ctrl+1): מציג את הדפים כפי שיודפסו.',
          'משולב (Ctrl+2): העורך מצד אחד והדפים מהצד השני. כל מה שאתם מקלידים מתעדכן בדפים בזמן אמת.',
          'עורך טקסט (Ctrl+3): חלון כתיבה נקי, בלי הסחות דעת.'
        ],
        tip: 'לכתיבה ועריכה נוחות מומלץ התצוגה המשולבת. למראה הסופי של הספר עברו לתצוגת עמודים.'
      }
    ],
    contentEn: [
      {
        title: 'The four work areas',
        body: 'The workspace is built from a few fixed parts:',
        items: [
          'Top bar: the file menu, document title, view-mode switcher, command search and undo/redo.',
          'Side panel: pages, text flows (for example Gemara, Rashi and Tosafot), styles and layers.',
          'The central canvas: the page view, with margins and guide lines.',
          'Status bar: word count, the active flow, the page number and the zoom control.'
        ]
      },
      {
        title: 'Three view modes',
        body: 'Switch between them from the top bar or with shortcuts:',
        items: [
          'Pages (Ctrl+1): shows the pages as they will be printed.',
          'Split (Ctrl+2): the editor on one side and the pages on the other. Everything you type updates the pages in real time.',
          'Text editor (Ctrl+3): a clean writing window with no distractions.'
        ],
        tip: 'Split view is the most comfortable for writing and editing. Switch to Pages view for the final look of the book.'
      }
    ]
  },
  {
    id: 'editing',
    titleHe: 'כתיבה ועריכה',
    titleEn: 'Writing & Editing',
    icon: 'edit',
    contentHe: [
      {
        title: 'עריכת הטקסט',
        body: 'מקלידים בעורך כמו בכל מעבד תמלילים. אפשר להדגיש, לשנות את גודל האותיות ולהחיל סגנון על פסקה. כל פעולה נרשמת בהיסטוריה: Ctrl+Z מבטל ו-Ctrl+Y מבצע שוב. הוספת עמוד חדש: Ctrl+Enter.'
      },
      {
        title: 'חיפוש והחלפה שמבין עברית',
        body: 'Ctrl+F פותח חיפוש, ו-Ctrl+H חיפוש והחלפה. אפשר לחפש מילה בלי ניקוד ולמצוא אותה גם כשהיא מנוקדת בטקסט, ואפשר להגביל לאותיות גדולות/קטנות או למילה שלמה.'
      },
      {
        title: 'אשף ניקוי והכשרת טקסט (Ctrl+Shift+C)',
        body: 'כשמייבאים טקסט מקובץ Word או מהאינטרנט, לרוב נשארים בו רווחים כפולים, מירכאות לא נכונות ופסקאות ריקות. האשף מציג כמה תיקונים הוא מצא, ואתם בוחרים מה להחיל:',
        items: [
          'הסרת רווחים כפולים ומיותרים, והסרת רווחים לפני סימני פיסוק.',
          'הוספת רווח אחרי סימני פיסוק צמודים.',
          'המרת מירכאות לגרשיים וגרש עבריים (למשל רמב״ם, עמ׳).',
          'איחוד פסקאות ריקות כפולות.',
          'הדגשת "דיבור המתחיל": המילים הפותחות כל פסקה עד לסימן מפריד שתבחרו (נקודה, נקודתיים או מקף).'
        ],
        tip: 'אפשר להחיל את הניקוי על התזרים הנוכחי בלבד או על כל התזרימים במסמך. אם משהו לא יצא כמו שרציתם, Ctrl+Z מחזיר אחורה.'
      }
    ],
    contentEn: [
      {
        title: 'Editing text',
        body: 'Type in the editor as in any word processor. You can bold text, change letter size and apply a style to a paragraph. Every action is recorded in history: Ctrl+Z undoes and Ctrl+Y redoes. Add a new page with Ctrl+Enter.'
      },
      {
        title: 'Find & replace that understands Hebrew',
        body: 'Ctrl+F opens search and Ctrl+H opens find and replace. You can search for a word without vowel points and still find it where the text is pointed, and you can limit the search to matching case or whole words.'
      },
      {
        title: 'Text Cleanup wizard (Ctrl+Shift+C)',
        body: 'Text imported from Word or the web usually carries double spaces, wrong quotation marks and empty paragraphs. The wizard shows how many fixes it found and you choose which to apply:',
        items: [
          'Remove double and unnecessary spaces, and spaces before punctuation.',
          'Add a space after punctuation that sticks to the next word.',
          'Convert quotation marks to Hebrew geresh and gershayim (for example Rambam, p.).',
          'Merge repeated empty paragraphs.',
          'Highlight the "dibur hamatchil": the opening words of each paragraph up to a separator you choose (period, colon or dash).'
        ],
        tip: 'You can apply the cleanup to the current flow only or to every flow in the document. If something did not turn out as expected, Ctrl+Z takes it back.'
      }
    ]
  },
  {
    id: 'flows',
    titleHe: 'טקסט ראשי ומפרשים',
    titleEn: 'Main Text & Commentaries',
    icon: 'flows',
    contentHe: [
      {
        title: 'מהו תזרים?',
        body: 'תזרים הוא רצף טקסט אחד שזורם בין העמודים, למשל הגמרא, רש״י או תוספות. בפריסות עם כמה טקסטים כל תזרים מעוצב בטור משלו, והתוכנה דואגת לכך שהפירוש יופיע ליד המקום שהוא מתייחס אליו.'
      },
      {
        title: 'מעבר בין תזרימים',
        body: 'בוחרים את התזרים הפעיל מהסרגל הצדדי או מהעורך, או פשוט לוחצים על הטקסט שלו בדף. העורך והכלים בצד יעברו אליו.',
        tip: 'כדי להתחיל להקליד פירוש, בחרו קודם את התזרים שלו בסרגל הצדדי, ואז הקלידו. כך הטקסט נכנס לטור הנכון.'
      }
    ],
    contentEn: [
      {
        title: 'What is a flow?',
        body: 'A flow is one continuous text that runs across the pages, for example the Gemara, Rashi or Tosafot. In layouts with several texts each flow is set in its own column, and the app makes sure a commentary appears next to the place it refers to.'
      },
      {
        title: 'Switching between flows',
        body: 'Choose the active flow in the side panel or the editor, or simply click its text on the page. The editor and the side tools switch to it.',
        tip: 'To start typing a commentary, first select its flow in the side panel and then type. That way the text goes into the right column.'
      }
    ]
  },
  {
    id: 'gestures-zoom',
    titleHe: 'תקריב (זום) ומחוות מגע',
    titleEn: 'Zoom & Touchpad Gestures',
    icon: 'search',
    contentHe: [
      {
        title: 'שליטה בתקריב',
        body: 'מגדילים ומקטינים עם כפתורי הזום בתצוגת העמודים, בסרגל המצב התחתון, או במקלדת: Ctrl++ להגדלה, Ctrl+- להקטנה ו-Ctrl+0 להתאמת הדף לחלון.'
      },
      {
        title: 'משטח מגע במחשב נייד',
        body: 'משטחי מגע נתמכים:',
        items: [
          'צביטה בשתי אצבעות: הגדלה והקטנה חלקה של הדפים.',
          'החלקה בשתי אצבעות: גלילה בין עמודים כשהסמן נמצא מעל הדפים.'
        ]
      }
    ],
    contentEn: [
      {
        title: 'Controlling the zoom',
        body: 'Zoom in and out with the zoom buttons in Pages view, in the bottom status bar, or on the keyboard: Ctrl++ to zoom in, Ctrl+- to zoom out and Ctrl+0 to fit the page to the window.'
      },
      {
        title: 'Touchpad on a laptop',
        body: 'Touchpads are supported:',
        items: [
          'Two-finger pinch: smooth zooming of the pages.',
          'Two-finger swipe: scroll between pages while the pointer is over the pages.'
        ]
      }
    ]
  },
  {
    id: 'typography-niqqud',
    titleHe: 'טיפוגרפיה, ניקוד וגימטריה',
    titleEn: 'Typography, Niqqud & Gematria',
    icon: 'typography',
    contentHe: [
      {
        title: 'ניקוד וטעמים',
        body: 'ניקוד וטעמים מוצגים במקומם הנכון מעל ומתחת לאותיות. אם הקלדתם או הדבקתם טקסט שבו הסימנים בסדר לא תקין, הפקודה "נרמול ניקוד" (בתפריט הקבצים) מסדרת אותם ומבטיחה תצוגה אחידה.'
      },
      {
        title: 'הגנה על שמות קדושים',
        body: 'הפקודה "הגנה על שמות קדושים" מונעת מהשמות להיפרד בסוף שורה, כך שלא יישברו באמצע.'
      },
      {
        title: 'מספור עמודים בגימטריה',
        body: 'מספרי העמודים מופיעים באותיות עבריות (א׳, ב׳, ג׳...). אם שיניתם את מספר העמודים או את הסדר, "חישוב גימטריה מחדש" בתפריט הקבצים מעדכן את המספור.'
      }
    ],
    contentEn: [
      {
        title: 'Niqqud and cantillation',
        body: 'Vowel points and cantillation marks are shown in their correct place above and below the letters. If you typed or pasted text whose marks are in an invalid order, the "Normalize niqqud" command (in the file menu) puts them in order and keeps the display consistent.'
      },
      {
        title: 'Protecting divine names',
        body: 'The "Shield divine names" command keeps these names from being separated at the end of a line, so they are never broken in the middle.'
      },
      {
        title: 'Page numbers in gematria',
        body: 'Page numbers appear as Hebrew letters (א׳, ב׳, ג׳...). If you changed the number or order of pages, "Recalculate gematria" in the file menu updates the numbering.'
      }
    ]
  },
  {
    id: 'export',
    titleHe: 'ייצוא ל-PDF ולדפוס',
    titleEn: 'Exporting to PDF & Print',
    icon: 'export',
    contentHe: [
      {
        title: 'קובץ מוכן לדפוס',
        body: 'Ctrl+E מייצא את המסמך ל-PDF. הקובץ בנוי לפי תקן הדפוס PDF/X-1a, עם גופנים מוטמעים ואיכות וקטורית חדה, כך שבית דפוס יכול לקבל אותו כמו שהוא.',
        tip: 'לפני הייצוא עברו על הספר בתצוגת עמודים: לוודא שפירושים מופיעים ליד המקום הנכון ושמספור העמודים תקין.'
      }
    ],
    contentEn: [
      {
        title: 'A print-ready file',
        body: 'Ctrl+E exports the document to PDF. The file follows the PDF/X-1a print standard, with embedded fonts and sharp vector quality, so a print shop can accept it as is.',
        tip: 'Before exporting, look through the book in Pages view: make sure commentaries sit next to the right place and the page numbering is correct.'
      }
    ]
  },
  {
    id: 'settings',
    titleHe: 'הגדרות והתאמה אישית',
    titleEn: 'Settings & Personalization',
    icon: 'settings',
    contentHe: [
      {
        title: 'חלון ההגדרות (Ctrl+,)',
        body: 'ההגדרות מחולקות ללשוניות:',
        items: [
          'מראה: ערכת נושא (בהירה, כהה ועוד).',
          'נגישות: אפשרויות לשיפור הקריאות והנוחות.',
          'שפה: מעבר בין עברית לאנגלית. המדריך הזה מתחלף בהתאם.',
          'תוספים: ניהול תוספים.',
          'עדכונים, ויומן ותחזוקה.'
        ]
      },
      {
        title: 'עוד על התוכנה',
        body: 'בתפריט הקבצים תמצאו גם "מה חדש" שמציג את השינויים בכל גרסה, ו"אודות" עם פרטי המפתחים והתורמים.'
      }
    ],
    contentEn: [
      {
        title: 'The settings window (Ctrl+,)',
        body: 'Settings are split into tabs:',
        items: [
          'Appearance: color theme (light, dark and more).',
          'Accessibility: options for readability and comfort.',
          'Language: switch between Hebrew and English. This guide changes with it.',
          'Plugins: manage plugins.',
          'Updates, and logs & maintenance.'
        ]
      },
      {
        title: 'More about the app',
        body: 'The file menu also has "What\u2019s new", which lists the changes in each version, and "About", with the developers and contributors.'
      }
    ]
  },
  {
    id: 'shortcuts',
    titleHe: 'קיצורי מקשים',
    titleEn: 'Keyboard Shortcuts',
    icon: 'keyboard',
    contentHe: [
      {
        title: 'קבצים',
        body: 'הקיצורים עובדים גם כשהמקלדת בעברית:',
        items: [
          'Ctrl+N: מסמך חדש',
          'Ctrl+O: פתיחת קובץ',
          'Ctrl+I: ייבוא מסמך',
          'Ctrl+S: שמירה',
          'Ctrl+Shift+S: שמירה בשם',
          'F2: שינוי שם המסמך',
          'Ctrl+Shift+P: דף הבית ורשימת הפרויקטים',
          'Ctrl+E: ייצוא PDF',
          'Ctrl+Q: יציאה'
        ]
      },
      {
        title: 'עריכה ותצוגה',
        body: 'פעולות שחוזרות על עצמן:',
        items: [
          'Ctrl+Z / Ctrl+Y: ביטול וביצוע חוזר',
          'Ctrl+F: חיפוש. Ctrl+H: חיפוש והחלפה',
          'Ctrl+Shift+C: אשף ניקוי הטקסט',
          'Ctrl+Enter: הוספת עמוד',
          'Ctrl+1 / Ctrl+2 / Ctrl+3: עמודים, משולב, עורך טקסט',
          'Ctrl++ / Ctrl+- / Ctrl+0: הגדלה, הקטנה, התאמה לחלון',
          'Ctrl+,: הגדרות',
          'Ctrl+K: לוח פקודות'
        ]
      }
    ],
    contentEn: [
      {
        title: 'Files',
        body: 'Shortcuts also work with a Hebrew keyboard layout:',
        items: [
          'Ctrl+N: new document',
          'Ctrl+O: open file',
          'Ctrl+I: import document',
          'Ctrl+S: save',
          'Ctrl+Shift+S: save as',
          'F2: rename document',
          'Ctrl+Shift+P: home screen and recent projects',
          'Ctrl+E: export PDF',
          'Ctrl+Q: quit'
        ]
      },
      {
        title: 'Editing and view',
        body: 'The actions you repeat most:',
        items: [
          'Ctrl+Z / Ctrl+Y: undo and redo',
          'Ctrl+F: find. Ctrl+H: find and replace',
          'Ctrl+Shift+C: Text Cleanup wizard',
          'Ctrl+Enter: add a page',
          'Ctrl+1 / Ctrl+2 / Ctrl+3: pages, split, text editor',
          'Ctrl++ / Ctrl+- / Ctrl+0: zoom in, zoom out, fit to window',
          'Ctrl+,: settings',
          'Ctrl+K: command palette'
        ]
      }
    ]
  }
];

export class UserGuideModal {
  public element: HTMLElement;
  private callbacks: UserGuideModalCallbacks;
  private activeSectionId = 'intro';
  private isVisible = false;
  private modal: ModalController;
  private contentPanel!: HTMLElement;

  constructor(callbacks: UserGuideModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-guide-overlay');

    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });

    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(sectionId?: string): void {
    if (sectionId) this.activeSectionId = sectionId;
    this.isVisible = true;
    this.element.classList.add('tok-open');
    this.render();
    this.modal.opened();
  }

  public hide(): void {
    if (!this.isVisible) return;
    this.isVisible = false;
    this.element.classList.remove('tok-open');
    this.modal.closed();
    this.callbacks.onClose();
  }

  private render(): void {
    const focusKey = this.modal.captureFocus();
    this.renderContent();
    this.modal.afterRender(focusKey);
  }

  private renderContent(): void {
    this.element.innerHTML = '';
    const isEn = i18n.getLanguage() === 'en';
    this.element.dir = isEn ? 'ltr' : 'rtl';

    const card = el('div', 'tok-dialog tok-guide-card');
    card.style.width = '880px';
    card.style.height = '680px';
    card.style.maxHeight = '90vh';
    card.style.display = 'flex';
    card.style.flexDirection = 'column';

    // Header
    const head = el('div', 'tok-dialog-head');
    const headTitle = el('div', 'tok-dialog-head-text');
    headTitle.appendChild(el('h2', undefined, { 'data-modal-title': '' }, isEn ? 'User Guide' : 'מדריך למשתמש'));
    head.appendChild(headTitle);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', isEn ? 'Close' : 'סגירה', () => this.hide(), {
      attrs: { 'data-focus-key': 'close' }
    }));
    card.appendChild(head);

    // Body container (Sidebar + Content)
    const body = el('div', 'tok-dialog-body', {
      style: 'display:flex;flex:1;min-height:0;overflow:hidden'
    });

    // Navigation sidebar
    const nav = el('nav', 'tok-settings-nav', {
      role: 'tablist',
      'aria-orientation': 'vertical',
      style: 'width:240px;flex:none;overflow-y:auto;border-inline-end:1px solid var(--tok-border-subtle);padding:12px 8px;display:flex;flex-direction:column;gap:4px;'
    });

    for (const sec of GUIDE_SECTIONS) {
      const on = this.activeSectionId === sec.id;
      const b = el('button', 'tok-settings-nav-btn', {
        type: 'button',
        role: 'tab',
        'aria-selected': String(on),
        tabindex: on ? '0' : '-1',
        'data-focus-key': `sec-${sec.id}`
      });
      b.appendChild(icon(sec.icon, 16));
      b.appendChild(el('span', undefined, undefined, isEn ? sec.titleEn : sec.titleHe));
      b.addEventListener('click', () => {
        if (this.activeSectionId === sec.id) return;
        this.activeSectionId = sec.id;
        nav.querySelectorAll('.tok-settings-nav-btn').forEach((btn) => {
          btn.setAttribute('aria-selected', 'false');
          btn.setAttribute('tabindex', '-1');
        });
        b.setAttribute('aria-selected', 'true');
        b.setAttribute('tabindex', '0');
        this.renderSectionContent();
      });
      nav.appendChild(b);
    }
    body.appendChild(nav);

    // Content view
    this.contentPanel = el('div', 'tok-guide-content', {
      style: 'flex:1;overflow-y:auto;padding:24px 32px;display:flex;flex-direction:column;gap:20px;'
    });
    body.appendChild(this.contentPanel);
    card.appendChild(body);

    // Footer
    const foot = el('div', 'tok-dialog-foot');
    foot.appendChild(el('span', 'tok-dialog-note', undefined, isEn ? 'TypesetOK Documentation' : 'תיעוד רשמי · TypesetOK'));
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(isEn ? 'Close' : 'סגירה', {
      className: 'tok-btn tok-btn-primary',
      attrs: { 'data-focus-key': 'done' },
      onClick: () => this.hide()
    }));
    card.appendChild(foot);

    this.element.appendChild(card);
    this.renderSectionContent();
  }

  private renderSectionContent(): void {
    if (!this.contentPanel) return;
    this.contentPanel.replaceChildren();

    const isEn = i18n.getLanguage() === 'en';
    const sec = GUIDE_SECTIONS.find((s) => s.id === this.activeSectionId) || GUIDE_SECTIONS[0];

    const h = el('h3', undefined, {
      style: 'font-size:18px;font-weight:700;margin:0 0 4px;color:var(--tok-text-primary)'
    }, isEn ? sec.titleEn : sec.titleHe);
    this.contentPanel.appendChild(h);

    const divider = el('div', undefined, {
      style: 'height:1px;background:var(--tok-border-subtle);margin-bottom:12px;'
    });
    this.contentPanel.appendChild(divider);

    const blocks = isEn ? sec.contentEn : sec.contentHe;
    for (const blk of blocks) {
      const blockEl = el('div', undefined, { style: 'display:flex;flex-direction:column;gap:6px;' });
      if (blk.title) {
        blockEl.appendChild(el('h4', undefined, {
          style: 'font-size:14px;font-weight:600;margin:0;color:var(--tok-accent-text)'
        }, blk.title));
      }
      blockEl.appendChild(el('p', undefined, {
        style: 'font-size:13px;line-height:1.65;margin:0;color:var(--tok-text-primary)'
      }, blk.body));

      if (blk.items && blk.items.length > 0) {
        const ul = el('ul', undefined, {
          style: 'margin:4px 0 0;padding-inline-start:20px;font-size:13px;line-height:1.6;color:var(--tok-text-secondary);display:flex;flex-direction:column;gap:4px;'
        });
        for (const item of blk.items) {
          ul.appendChild(el('li', undefined, undefined, item));
        }
        blockEl.appendChild(ul);
      }
      if (blk.tip) {
        const tip = el('div', undefined, {
          style: 'margin-top:6px;padding:10px 14px;border-radius:10px;border-inline-start:3px solid var(--tok-accent-primary);background:var(--tok-accent-soft);display:flex;gap:10px;align-items:flex-start;'
        });
        tip.appendChild(icon('sparkle', 16));
        tip.appendChild(el('span', undefined, { style: 'font-size:13px;line-height:1.6;color:var(--tok-text-primary);' }, blk.tip));
        blockEl.appendChild(tip);
      }
      this.contentPanel.appendChild(blockEl);
    }
  }
}

import { t, tf, i18n } from '../i18n';
import { el, icon, iconButton, button, kbd } from '../ui';
import { ModalController } from './ModalController';
import { IconName } from '../icons';

export interface UserGuideModalCallbacks {
  onClose: () => void;
}

interface GuideSection {
  id: string;
  titleHe: string;
  titleEn: string;
  icon: IconName;
  contentHe: { title?: string; body: string; items?: string[] }[];
  contentEn: { title?: string; body: string; items?: string[] }[];
}

const GUIDE_SECTIONS: GuideSection[] = [
  {
    id: 'intro',
    titleHe: 'מבוא וסביבת העבודה',
    titleEn: 'Introduction & Workspace',
    icon: 'brand',
    contentHe: [
      {
        title: 'ברוכים הבאים ל-TypesetOK',
        body: 'TypesetOK (TOK) היא מערכת עימוד והוצאה לאור מקצועית המיועדת במיוחד לטקסט עברי ולספרי קודש ועיון. המערכת מבוססת על ליבת עימוד מדויקת, תמיכה מלאה בתקני טיפוגרפיה עברית מסורתיים, ניהול תזרימי טקסט מקבילים וייצוא מהיר לדפוס.'
      },
      {
        title: 'מבנה סביבת העבודה',
        body: 'חלון העבודה כולל ארבעה אזורים מרכזיים:',
        items: [
          'הסרגל העליון (Top Bar): ניהול מסמכים, מצבי תצוגה (עמודים, משולבת, עורך טקסט), כותרת המסמך וייצוא ל-PDF.',
          'סרגל המבנה הצדי (Structure Bar): ניהול עמודים, תזרימים (גמרא, רש״י, תוספות, הערות), סגנונות ושכבות.',
          'לוח העבודה המרכזי: תצוגת עמודים פרוסה עם פריסה דו-צדדית (לוח ספר), רשת שורות בסיס וקווי שוליים.',
          'סרגל המצב התחתון: סטטיסטיקת מילים, תזרים פעיל, אינדיקטור עמודים ובקרת תקריב (זום).'
        ]
      }
    ],
    contentEn: [
      {
        title: 'Welcome to TypesetOK',
        body: 'TypesetOK (TOK) is a professional desktop publishing and typesetting system designed specifically for Hebrew text, sacred manuscripts, and scholarly books. It combines an exact layout engine with traditional Hebrew typographic rules, multi-flow synchronization, and high-fidelity print output.'
      },
      {
        title: 'Workspace Architecture',
        body: 'The workspace consists of four primary regions:',
        items: [
          'Top System Bar: Document management, view modes (Pages, Split, Text Editor), document title, and PDF export.',
          'Structure Bar: Page thumbnails, text flows (Gemara, Rashi, Tosafot, Notes), styles, and layers.',
          'Central Spread Canvas: High-precision spread viewer with two-page book layout, baseline grids, and margin guides.',
          'Status Bar: Word counts, active flow indicator, page counter, and zoom controls.'
        ]
      }
    ]
  },
  {
    id: 'templates',
    titleHe: 'תבניות עימוד ופריסה',
    titleEn: 'Templates & Layouts',
    icon: 'pages',
    contentHe: [
      {
        title: 'תבניות מובנות',
        body: 'המערכת מציעה מגוון תבניות מותאמות לסגנונות ספרותיים שונים:',
        items: [
          'דף גמרא: פריסה קלאסית עם טקסט גמרא מרכזי, רש״י בטור פנימי ותוספות בטור חיצוני.',
          'מקראות גדולות: טקסט מקרא מנוקד עם טעמים בראש הדף, ותרגום ומפרשים תחתיו.',
          'ספר רציף: טור יחיד ומאוזן לספרי קריאה, הגות ועיון.',
          'ספר עם הערות: טקסט ראשי רציף עם אזור הערות שוליים דינמי בתחתית הדף.',
          'עלון וקונטרס: מבנה דו-טורי אלגנטי עם כותרת רוחבית, מתאים לעלוני שבת וחוברות.'
        ]
      }
    ],
    contentEn: [
      {
        title: 'Built-in Templates',
        body: 'TypesetOK includes pre-configured layouts for classic Hebrew publishing formats:',
        items: [
          'Talmud Page: Canonical layout with central Gemara, inner Rashi, and outer Tosafot columns.',
          'Mikraot Gedolot: Pointed biblical text with cantillation on top, accompanied by commentaries below.',
          'Continuous Book: Single-column balanced layout for scholarly books, essays, and literature.',
          'Book with Footnotes: Primary body text with dynamic footnote allocation at the foot of each page.',
          'Bulletin & Booklet: Two-column layout with spanning header, ideal for weekly bulletins and booklets.'
        ]
      }
    ]
  },
  {
    id: 'editor-flows',
    titleHe: 'עריכת טקסט ותזרימים',
    titleEn: 'Text Editing & Multi-Flows',
    icon: 'story',
    contentHe: [
      {
        title: 'עורך הטקסט והתצוגה המשולבת',
        body: 'עורך הטקסט מספק סביבת עבודה נוחה וממוקדת לכתיבה ולעריכה. במצב תצוגה משולבת (Split View), כל שינוי שמוקלד בעורך משתקף בזמן אמת בתצוגת העמודים שמשמאל.'
      },
      {
        title: 'עבודה עם תזרימי טקסט מקבילים',
        body: 'כל עמוד יכול להכיל מספר תזרימים נפרדים (למשל: טקסט ראשי ומפרשים). ניתן לבחור את התזרים הפעיל מסרגל העורך או מסרגל המבנה. שינוי גודל הטקסט בעורך מתבצע באמצעות כפתורי האותיות הייעודיים.'
      }
    ],
    contentEn: [
      {
        title: 'Text Editor & Split View',
        body: 'The Text Editor provides a continuous, focused writing environment. In Split View, text typed into the editor renders instantly and in real-time onto the page canvas on the opposite side.'
      },
      {
        title: 'Working with Multi-Flows',
        body: 'Pages can contain synchronized parallel flows (e.g. main text alongside commentaries). Switch the active flow via the editor toolbar or the Structure bar. Adjust editor preview size using the compact typography buttons.'
      }
    ]
  },
  {
    id: 'gestures-zoom',
    titleHe: 'תקריב (זום) ומחוות מגע',
    titleEn: 'Zoom & Trackpad Gestures',
    icon: 'search',
    contentHe: [
      {
        title: 'שליטה בתקריב (Zoom)',
        body: 'ניתן לשלוט ברמת התקריב באמצעות כפתורי הזום החדשים בחלון תצוגת העמודים, דרך סרגל המצב התחתון, או באמצעות קיצורי המקשים Ctrl++ ו-Ctrl+-. כפתור "התאם" מתאים את הדף במדויק לחלון.'
      },
      {
        title: 'מחוות משטח מגע במחשבים ניידים',
        body: 'משטחי מגע (Touchpad) נתמכים באופן מלא:',
        items: [
          'צביטה לפתיחה ולסגירה (Pinch to Zoom): הגדלה והקטנה חלקה ומדויקת של הדפים.',
          'החלקה בשתי אצבעות: גלילה ומעבר חלק בין עמודים ופרוסות כאשר סמן העכבר נמצא מעל הדפים.'
        ]
      }
    ],
    contentEn: [
      {
        title: 'Zoom Controls',
        body: 'Control magnification directly using the on-canvas zoom toolbar, the bottom status bar, or shortcuts Ctrl++ and Ctrl+-. The "Fit" control automatically fits the active spread to the window.'
      },
      {
        title: 'Laptop Trackpad Gestures',
        body: 'Touchpads are natively supported:',
        items: [
          'Pinch-to-zoom: Smooth, continuous scaling centered on your view.',
          'Two-finger swipe: Intuitive paging and smooth panning across sheets when positioned over the canvas.'
        ]
      }
    ]
  },
  {
    id: 'typography-niqqud',
    titleHe: 'טיפוגרפיה, ניקוד ודפוס',
    titleEn: 'Typography & Print Standards',
    icon: 'sparkle',
    contentHe: [
      {
        title: 'נרמול ניקוד וטעמים (ת״י 6100)',
        body: 'המערכת כוללת מנוע ייעודי לנרמול רצפי יוניקוד של אות, ניקוד וטעמים. הפעולה מתקנת שגיאות הקלדה נפוצות ומבטיחה יישור מושלם של סימני הניקוד מעל ומתחת לאותיות.'
      },
      {
        title: 'מגן שמות קדושים',
        body: 'כלל הלכתי וטיפוגרפי המונע פיצול או שבירה של שמות קדושים בסוף שורה.'
      },
      {
        title: 'ייצוא מוכן לדפוס (PDF/X)',
        body: 'ייצוא קובצי PDF מקצועיים עם גופנים מוטמעים, רזולוציה וקטורית חדה ותאימות מלאה לבתי דפוס.'
      }
    ],
    contentEn: [
      {
        title: 'Niqqud & Cantillation Normalization',
        body: 'Built-in engine conforming to SI 6100 standardizes Unicode sequences (base letter, niqqud, dagesh, cantillation) ensuring flawless glyph rendering.'
      },
      {
        title: 'Divine Names Shield',
        body: 'Halachic and typographic rule preventing divine names from being split across line breaks.'
      },
      {
        title: 'Print-Ready PDF Export',
        body: 'Produces vector PDFs with fully embedded fonts and high-fidelity output conforming to prepress standards.'
      }
    ]
  },
  {
    id: 'shortcuts',
    titleHe: 'קיצורי מקשים שימושיים',
    titleEn: 'Keyboard Shortcuts',
    icon: 'settings',
    contentHe: [
      {
        title: 'קיצורים נפוצים',
        body: 'המערכת מזהה קיצורי מקשים הן בפריסת מקלדת עברית והן באנגלית:',
        items: [
          'Ctrl+S / Ctrl+ד: שמירת מסמך',
          'Ctrl+Shift+S: שמירה בשם',
          'Ctrl+E / Ctrl+ק: ייצוא PDF לדפוס',
          'Ctrl+1: מעבר לתצוגת עמודים',
          'Ctrl+2: מעבר לתצוגה משולבת (עורך + עמודים)',
          'Ctrl+3: מעבר לעורך טקסט בלבד',
          'Ctrl++ / Ctrl+=: הגדלת זום',
          'Ctrl+-: הקטנת זום',
          'Ctrl+0: התאמת עמוד לחלון (Fit)',
          'Ctrl+Enter: הוספת עמוד חדש למסמך',
          'Ctrl+K: פתיחת לוח פקודות עימוד'
        ]
      }
    ],
    contentEn: [
      {
        title: 'Key Shortcuts',
        body: 'TypesetOK recognizes both English and Hebrew keyboard layouts seamlessly:',
        items: [
          'Ctrl+S: Save Document',
          'Ctrl+Shift+S: Save As',
          'Ctrl+E: Export to Print PDF',
          'Ctrl+1: Pages View',
          'Ctrl+2: Split View (Editor + Pages)',
          'Ctrl+3: Text Editor View',
          'Ctrl++: Zoom In',
          'Ctrl+-: Zoom Out',
          'Ctrl+0: Fit to Window',
          'Ctrl+Enter: Add New Page',
          'Ctrl+K: Typesetting Command Palette'
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
      this.contentPanel.appendChild(blockEl);
    }
  }
}

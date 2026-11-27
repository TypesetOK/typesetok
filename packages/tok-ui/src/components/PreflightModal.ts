import { i18n } from '../i18n';
import { el, iconButton, button, kbd } from '../ui';
import { ModalController } from './ModalController';
import type { PreflightReport, PreflightCheckItem, PreflightIssue, PreflightStatus } from '../engine/typesetBridge';

export interface PreflightModalCallbacks {
  onClose: () => void;
}

interface CheckCopy {
  title: [string, string];
  /** What the check verifies, in plain language: [he, en]. */
  about: [string, string];
  /** Explanation per non-passing status: [he, en]. */
  warning?: [string, string];
  fail?: [string, string];
  /** What the user can do about it: [he, en]. */
  fix?: [string, string];
}

/** Plain-language copy for each engine check id (see crates/tok-pdf/src/preflight.rs). */
const CHECK_COPY: Record<string, CheckCopy> = {
  fonts: {
    title: ['גופנים ואותיות עבריות', 'Fonts & Hebrew coverage'],
    about: [
      'בודק שכל הגופנים במסמך מותקנים, מותרים להטמעה בקובץ ה-PDF, ושיש בהם את כל האותיות, הניקוד והטעמים שבטקסט.',
      'Checks that every font in the document is installed, licensed for PDF embedding, and contains all the letters, vowel points and cantillation marks used in the text.'
    ],
    warning: [
      'בגופן חסרים סימני ניקוד או טעמים. הם עלולים להיעלם או להיראות שגוי בהדפסה.',
      'A font is missing vowel points or cantillation marks. They may disappear or look wrong in print.'
    ],
    fail: [
      'גופן חסר, אינו מכיל אותיות עבריות, או שהרישיון שלו אוסר הטמעה ב-PDF. בבית הדפוס יוחלף גופן והעימוד ישתנה.',
      'A font is missing, has no Hebrew letters, or its licence forbids PDF embedding. The printer would substitute a font and the layout would change.'
    ],
    fix: [
      'התקינו את הגופן החסר, או החליפו אותו בגופן אחר שתומך בעברית ובניקוד (למשל Frank Ruhl Libre).',
      'Install the missing font, or switch to another font that supports Hebrew and niqqud (for example Frank Ruhl Libre).'
    ]
  },
  color: {
    title: ['צבעים ושימוש בדיו', 'Colour & ink limit'],
    about: [
      'בודק את פרופיל הצבע ואת כמות הדיו המרבית (TAC) המותרת בהדפסה, ושלצבעי ספוט יש שם דיו.',
      'Checks the colour profile, the maximum ink coverage (TAC) allowed for print, and that spot colours have an ink name.'
    ],
    warning: [
      'חלק מהצבעים עוברים את מגבלת הדיו (300%). הם יוקטנו אוטומטית בייצוא, ולכן הגוון עשוי להשתנות מעט.',
      'Some colours exceed the ink limit (300%). They are reduced automatically on export, so the shade may change slightly.'
    ],
    fail: [
      'יש צבע ספוט בלי שם דיו, ולכן בית הדפוס לא יוכל להפיק ממנו לוח הדפסה.',
      'A spot colour has no ink name, so the printer cannot create a printing plate for it.'
    ],
    fix: [
      'תנו שם לצבע הספוט, או החליפו אותו בצבע CMYK רגיל.',
      'Name the spot colour, or replace it with a regular CMYK colour.'
    ]
  },
  geometry: {
    title: ['גודל עמוד וחיתוך (Bleed)', 'Page size & bleed'],
    about: [
      'בודק שמידות העמוד תקינות ושיש שוליים לחיתוך (Bleed) של 3 מ״מ לפחות, כנהוג בבתי דפוס.',
      'Checks that the page size is valid and that there is at least 3 mm of bleed, as print shops expect.'
    ],
    warning: [
      'שוליי החיתוך קטנים מ-3 מ״מ. אם יש רקע או תמונה עד קצה העמוד, ייתכן פס לבן אחרי החיתוך.',
      'The bleed is smaller than 3 mm. If a background or image reaches the page edge, a white strip may show after trimming.'
    ],
    fail: [
      'מידות העמוד אינן תקינות או שאין עמודים במסמך.',
      'The page size is invalid or the document has no pages.'
    ],
    fix: [
      'הגדילו את שוליי החיתוך בהגדרות הייצוא ל-3 מ״מ ומעלה, ובדקו את גודל העמוד.',
      'Set the bleed to 3 mm or more in the export settings, and check the page size.'
    ]
  },
  images: {
    title: ['רזולוציית תמונות', 'Image resolution'],
    about: [
      'בודק שלתמונות במסמך יש רזולוציה אפקטיבית מספקת להדפסה (מומלץ 300 DPI ומעלה).',
      'Checks that images have enough effective resolution for print (300 DPI or more is recommended).'
    ],
    warning: [
      'תמונה ברזולוציה של 150–299 DPI. היא תודפס, אך עלולה להיראות מעט מטושטשת.',
      'An image is between 150 and 299 DPI. It will print, but may look slightly soft.'
    ],
    fail: [
      'תמונה ברזולוציה נמוכה מ-150 DPI. היא תודפס מפוקסלת.',
      'An image is below 150 DPI. It will print pixelated.'
    ],
    fix: [
      'החליפו אותה בתמונה ברזולוציה גבוהה יותר, או הקטינו אותה בעמוד.',
      'Replace it with a higher-resolution image, or make it smaller on the page.'
    ]
  },
  columns: {
    title: ['איזון גובה העמודות', 'Column balance'],
    about: [
      'בודק שתחתיות העמודות באותו עמוד מיושרות זו לזו (סטייה של עד 2 נקודות).',
      'Checks that the bottoms of the columns on a page line up (within 2 points).'
    ],
    warning: [
      'בעמוד אחד או יותר העמודות אינן מיושרות בתחתיתן, ולכן העמוד נראה לא מאוזן.',
      'On one or more pages the column bottoms do not line up, so the page looks unbalanced.'
    ],
    fail: [
      'נמצאה שגיאת איזון בעמודות.',
      'A column balance error was found.'
    ],
    fix: [
      'שנו מעט את הטקסט, את מרווחי השורות או את מיקום מעברי העמוד, והעימוד יתאזן מחדש.',
      'Adjust the text, line spacing or page breaks a little and the layout will rebalance.'
    ]
  },
  pagination: {
    title: ['גלישת טקסט מחוץ לעמוד', 'Text overset'],
    about: [
      'בודק שאף מסגרת טקסט אינה חורגת מתחתית העמוד.',
      'Checks that no text frame runs past the bottom of the page.'
    ],
    warning: [
      'מסגרת טקסט חורגת מתחתית העמוד, ולכן חלק מהטקסט עלול להיחתך בהדפסה.',
      'A text frame runs past the bottom of the page, so part of the text may be cut off in print.'
    ],
    fail: [
      'מסגרת טקסט חורגת מהעמוד.',
      'A text frame goes beyond the page.'
    ],
    fix: [
      'קצרו את הטקסט, הקטינו את הגופן או הוסיפו עמוד.',
      'Shorten the text, use a smaller font, or add a page.'
    ]
  }
};

const FLAG_TEXT: Record<string, [string, string]> = {
  'font not installed/embedded': ['הגופן אינו מותקן', 'font is not installed'],
  'missing Hebrew letters': ['חסרות אותיות עבריות', 'missing Hebrew letters'],
  'missing Niqqud': ['חסר ניקוד', 'missing niqqud'],
  'missing Taamim': ['חסרים טעמי מקרא', 'missing cantillation marks']
};

/** Turns the engine's English diagnostics into the UI language. Unknown messages are shown as-is. */
function localizeDetail(raw: string, isEn: boolean): string {
  if (isEn) return raw;
  let m: RegExpMatchArray | null;

  if ((m = raw.match(/^(.*) \((\d+) paragraphs\): (.*)$/))) {
    const flags = m[3].split(', ').map((f) => FLAG_TEXT[f]?.[0] ?? f).join(', ');
    return `${m[1]} (${m[2]} פסקאות): ${flags}`;
  }
  if ((m = raw.match(/^(.*): restricted OS\/2 fsType=(0x[0-9A-Fa-f]+)$/))) {
    return `${m[1]}: רישיון הגופן אינו מאפשר הטמעה ב-PDF (fsType=${m[2]})`;
  }
  if (raw === 'Spot color with empty ink name') return 'צבע ספוט ללא שם דיו';
  if ((m = raw.match(/^(\d+) color specification\(s\) exceeded 300% TAC and are auto-clamped via GCR$/))) {
    return `${m[1]} צבעים חורגים מ-300% דיו ויוקטנו אוטומטית`;
  }
  if ((m = raw.match(/^Spot \/Separation plates: (.*)$/))) return `לוחות ספוט: ${m[1]}`;
  if (raw === 'Invalid or empty page TrimBox dimensions') return 'מידות העמוד אינן תקינות או שאין עמודים';
  if ((m = raw.match(/^Bleed is ([\d.]+)pt \(/))) {
    return `שולי החיתוך הם ${m[1]} נקודות (מומלץ 8.5 נקודות, כלומר 3 מ״מ, ומעלה)`;
  }
  if ((m = raw.match(/^Page (\d+): (.*) effective (\d+) DPI \((.*)\)$/))) {
    const level = m[4].startsWith('<') ? 'מתחת למינימום של 150' : 'בטווח 150–299, אזהרת הדפסה';
    return `עמוד ${m[1]}: ${m[2]} ברזולוציה אפקטיבית של ${m[3]} DPI (${level})`;
  }
  if ((m = raw.match(/^Page (\d+) frame '(.*)' exceeds page bottom by ([\d.]+)pt$/))) {
    return `עמוד ${m[1]}: המסגרת '${m[2]}' חורגת מתחתית העמוד ב-${m[3]} נקודות`;
  }
  return raw;
}

const STATUS_RANK: Record<PreflightStatus, number> = { Fail: 0, Warning: 1, Pass: 2 };

function statusLabel(s: PreflightStatus, isEn: boolean): string {
  if (s === 'Fail') return isEn ? 'Error' : 'שגיאה';
  if (s === 'Warning') return isEn ? 'Warning' : 'אזהרה';
  return isEn ? 'OK' : 'תקין';
}

function statusColor(s: PreflightStatus): { fg: string; bg: string } {
  if (s === 'Fail') return { fg: 'var(--tok-status-error)', bg: 'var(--tok-error-soft)' };
  if (s === 'Warning') return { fg: 'var(--tok-status-warning)', bg: 'var(--tok-warning-soft)' };
  return { fg: 'var(--tok-status-success)', bg: 'var(--tok-success-soft)' };
}

export class PreflightModal {
  public element: HTMLElement;
  private callbacks: PreflightModalCallbacks;
  private isVisible = false;
  private modal: ModalController;
  private report: PreflightReport | null = null;

  constructor(callbacks: PreflightModalCallbacks) {
    this.callbacks = callbacks;
    this.element = el('div', 'tok-overlay tok-preflight-overlay');
    this.modal = new ModalController(this.element, () => this.hide(), { closeOnBackdrop: true });
    i18n.onChange(() => {
      if (this.isVisible) this.render();
    });
  }

  public show(report: PreflightReport | null): void {
    this.report = report;
    this.isVisible = true;
    this.element.classList.add('tok-open');
    this.render();
    this.modal.opened();
  }

  /** Refreshes an open dialog when a newer report arrives. */
  public update(report: PreflightReport | null): void {
    this.report = report;
    if (this.isVisible) this.render();
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

    const card = el('div', 'tok-dialog tok-preflight-card', {
      style: 'width:640px;max-width:94vw;max-height:90vh;display:flex;flex-direction:column;'
    });

    const head = el('div', 'tok-dialog-head');
    const headText = el('div', 'tok-dialog-head-text');
    headText.appendChild(el('h2', undefined, { 'data-modal-title': '' }, isEn ? 'Print check' : 'בדיקת דפוס'));
    if (this.report) {
      headText.appendChild(
        el('p', undefined, undefined,
          `${this.report.profile_name} · ${this.report.pdfx_standard} · ${
            isEn ? `${this.report.total_pages} pages` : `${this.report.total_pages} עמודים`
          }`)
      );
    }
    head.appendChild(headText);
    head.appendChild(kbd('Esc'));
    head.appendChild(iconButton('close', isEn ? 'Close' : 'סגירה', () => this.hide(), {
      attrs: { 'data-focus-key': 'close' }
    }));
    card.appendChild(head);

    const body = el('div', 'tok-dialog-body', {
      style: 'padding:16px 24px;display:flex;flex-direction:column;gap:12px;overflow:auto;'
    });

    if (!this.report) {
      body.appendChild(
        el('p', undefined, { style: 'color:var(--tok-text-secondary)' },
          isEn
            ? 'The print check has not run yet. It runs automatically after the document is typeset.'
            : 'בדיקת הדפוס עדיין לא רצה. היא רצה אוטומטית אחרי עימוד המסמך.')
      );
    } else {
      body.appendChild(this.renderSummary(this.report, isEn));
      const checks = [...this.report.checks].sort((a, b) => STATUS_RANK[a.status] - STATUS_RANK[b.status]);
      const allIssues = this.report.issues ?? [];
      for (const ck of checks) {
        const checkIssues = allIssues.filter((issue) => issue.check_id === ck.id);
        body.appendChild(this.renderCheck(ck, checkIssues, isEn));
      }
    }
    card.appendChild(body);

    const foot = el('div', 'tok-dialog-foot');
    foot.appendChild(el('span', 'tok-grow'));
    foot.appendChild(button(isEn ? 'Close' : 'סגירה', {
      className: 'tok-btn tok-btn-primary',
      attrs: { 'data-focus-key': 'done' },
      onClick: () => this.hide()
    }));
    card.appendChild(foot);

    this.element.appendChild(card);
  }

  private renderSummary(report: PreflightReport, isEn: boolean): HTMLElement {
    const failCount = report.checks.filter((c) => c.status === 'Fail').length;
    const warnCount = report.checks.filter((c) => c.status === 'Warning').length;
    const { fg, bg } = statusColor(report.overall_status);
    const box = el('div', undefined, {
      style: `background:${bg};border:1px solid ${fg};border-radius:10px;padding:12px 14px;display:flex;flex-direction:column;gap:4px;`
    });
    let title: string;
    let sub: string;
    if (report.overall_status === 'Fail') {
      title = isEn ? 'Errors found — fix before sending to print' : 'נמצאו שגיאות — מומלץ לתקן לפני שליחה להדפסה';
      sub = isEn ? `${failCount} error(s), ${warnCount} warning(s)` : `${failCount} שגיאות, ${warnCount} אזהרות`;
    } else if (report.overall_status === 'Warning') {
      title = isEn ? 'Warnings found — the file can be exported' : 'נמצאו אזהרות — אפשר לייצא את הקובץ';
      sub = isEn ? `${warnCount} warning(s)` : `${warnCount} אזהרות`;
    } else {
      title = isEn ? 'All good — ready for print' : 'הכול תקין — מוכן להדפסה';
      sub = isEn ? 'All checks passed' : 'כל הבדיקות עברו';
    }
    box.appendChild(el('strong', undefined, { style: `color:${fg};font-size:15px` }, title));
    box.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-secondary)' }, sub));
    if (report.affected_pages && report.affected_pages.length > 0) {
      const pagesText = report.affected_pages.join(', ');
      box.appendChild(
        el(
          'span',
          undefined,
          { style: 'font-size:12px;color:var(--tok-text-secondary)' },
          isEn ? `Affected pages: ${pagesText}` : `עמודים עם ממצאים: ${pagesText}`
        )
      );
    }
    return box;
  }

  private renderCheck(ck: PreflightCheckItem, checkIssues: PreflightIssue[], isEn: boolean): HTMLElement {
    const copy = CHECK_COPY[ck.id];
    const idx = isEn ? 1 : 0;
    const { fg, bg } = statusColor(ck.status);
    const card = el('div', undefined, {
      style: 'background:var(--tok-bg-surface-1);border:1px solid var(--tok-border-subtle);border-radius:10px;padding:12px 14px;display:flex;flex-direction:column;gap:6px;'
    });

    const top = el('div', undefined, { style: 'display:flex;align-items:center;gap:10px;' });
    top.appendChild(el('strong', undefined, { style: 'flex:1;font-size:14px' }, copy ? copy.title[idx] : ck.title));
    top.appendChild(
      el('span', undefined, {
        style: `background:${bg};color:${fg};border-radius:12px;padding:2px 10px;font-size:11px;font-weight:600;`
      }, statusLabel(ck.status, isEn))
    );
    card.appendChild(top);

    if (copy) {
      card.appendChild(el('span', undefined, { style: 'font-size:12px;color:var(--tok-text-muted)' }, copy.about[idx]));
      const explain = ck.status === 'Fail' ? copy.fail : ck.status === 'Warning' ? copy.warning : undefined;
      if (explain) {
        card.appendChild(el('span', undefined, { style: 'font-size:13px;color:var(--tok-text-primary)' }, explain[idx]));
      }
      if (ck.status !== 'Pass' && copy.fix) {
        card.appendChild(
          el('span', undefined, { style: 'font-size:13px;color:var(--tok-accent-text)' },
            `${isEn ? 'What to do: ' : 'מה לעשות: '}${copy.fix[idx]}`)
        );
      }
    } else {
      // A check this UI has no copy for yet: show the engine's own summary.
      card.appendChild(el('span', undefined, { style: 'font-size:13px' }, ck.summary));
    }

    if (checkIssues.length > 0) {
      const sortedIssues = [...checkIssues].sort((a, b) => {
        const sevDiff = STATUS_RANK[a.severity] - STATUS_RANK[b.severity];
        if (sevDiff !== 0) return sevDiff;
        return (a.page_number ?? 0) - (b.page_number ?? 0);
      });
      const list = el('div', undefined, {
        style: 'margin:6px 0 0;display:flex;flex-direction:column;gap:6px;'
      });
      for (const issue of sortedIssues) {
        const ic = statusColor(issue.severity);
        const row = el('div', undefined, {
          style: 'background:var(--tok-bg-surface-2, rgba(0,0,0,0.03));border-inline-start:3px solid ' + ic.fg + ';border-radius:6px;padding:6px 10px;display:flex;flex-direction:column;gap:3px;font-size:12px;'
        });
        const meta = el('div', undefined, { style: 'display:flex;align-items:center;gap:6px;flex-wrap:wrap;' });
        const pageBadge = issue.page_number != null
          ? (isEn ? `Page ${issue.page_number}` : `עמוד ${issue.page_number}`)
          : (isEn ? 'Document' : 'כל המסמך');
        meta.appendChild(el('span', undefined, {
          style: `background:${ic.bg};color:${ic.fg};border-radius:10px;padding:1px 8px;font-size:11px;font-weight:600;`
        }, pageBadge));
        if (issue.location) {
          meta.appendChild(el('code', undefined, { style: 'font-size:11px;color:var(--tok-text-muted)' }, issue.location));
        }
        row.appendChild(meta);
        row.appendChild(el('span', undefined, { style: 'color:var(--tok-text-primary)' }, isEn ? issue.message : issue.message_he));
        const sug = isEn ? issue.suggestion : issue.suggestion_he;
        if (sug) {
          row.appendChild(el('span', undefined, { style: 'color:var(--tok-accent-text);font-size:11px' }, sug));
        }
        list.appendChild(row);
      }
      card.appendChild(list);
    } else if (ck.details && ck.details.length > 0) {
      const list = el('ul', undefined, {
        style: 'margin:4px 0 0;padding-inline-start:18px;font-size:12px;color:var(--tok-text-secondary);display:flex;flex-direction:column;gap:2px;'
      });
      for (const d of ck.details) {
        list.appendChild(el('li', undefined, { title: d }, localizeDetail(d, isEn)));
      }
      card.appendChild(list);
    }
    return card;
  }
}

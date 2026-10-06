# 🔌 TypesetOK (TOK) - Official Plugin API Reference & Developer Guide
### מדריך ומפרט ה-API הרשמי לפיתוח תוספים והרחבות למערכת TypesetOK

ברוכים הבאים למערכת ההרחבות והתוספים של **TypesetOK (TOK)**.
מערכת התוספים של TypesetOK תוכננה מן המסד כדי לספק למפתחים ממשק עוצמתי (TypeScript & JavaScript) לעבודה ישירה עם טקסט, טיפוגרפיה עברית, ספרי קודש, קנבס וסרגל הפקודות – **תוך שמירה מוחלטת על ביצועי שיא (Zero Performance Overhead), ארגז חול מאובטח (Sandboxing), ותמיכה מלאה בעמדות מחשב מנוהלות (מצב סייפר / Safe Mode)**.

---

## 📑 תוכן העניינים
1. [ארכיטקטורת התוספים וביצועים](#1-ארכיטקטורת-התוספים-וביצועים)
2. [מבנה תוסף וקובץ `plugin.json`](#2-מבנה-תוסף-וקובץ-pluginjson)
3. [מערכת ההרשאות ואבטחה (Permissions & Sandbox)](#3-מערכת-ההרשאות-ואבטחה-permissions--sandbox)
4. [תמיכה במצב סייפר (Safer Workstations & Safe Mode)](#4-תמיכה-במצב-סייפר-safer-workstations--safe-mode)
5. [מפרט ה-API המלא (`tok.*`)](#5-מפרט-ה-api-המלא-tok)
   - [`tok.document` (עריכת טקסט ומסמך)](#tokdocument---עריכת-טקסט-ומסמך)
   - [`tok.hebrew` (טיפוגרפיה עברית, שמות קודש וגימטריה)](#tokhebrew---טיפוגרפיה-עברית-שמות-קודש-וגימטריה)
   - [`tok.ui` (ממשק משתמש ופקודות)](#tokui---ממשק-משתמש-ופקודות)
   - [`tok.canvas` (קנבס, דפים וזום)](#tokcanvas---קנבס-דפים-וזום)
   - [`tok.events` (אירועי מערכת)](#tokevents---אירועי-מערכת)
   - [`tok.env` (סטטוס סביבה ומצב בטוח)](#tokenv---סטטוס-סביבה-ומצב-בטוח)
6. [מחזור חיים וניקוי משאבים (Lifecycle & Disposable)](#6-מחזור-חיים-וניקוי-משאבים-lifecycle--disposable)
7. [דוגמאות קוד מלאות לשימוש מעשי](#7-דוגמאות-קוד-מלאות-לשימוש-מעשי)
   - [דוגמה 1: מנקה ניקוד וטעמים לחיפוש והדפסה](#דוגמה-1-מנקה-ניקוד-וטעמים-לחיפוש-והדפסה)
   - [דוגמה 2: מגן שמות קדושים (בדיקת שמות שאינם נמחקים)](#דוגמה-2-מגן-שמות-קדושים-בדיקת-שמות-שאינם-נמחקים)
   - [דוגמה 3: תיקון ראשי תיבות והמרת גרשיים תקניים](#דוגמה-3-תיקון-ראשי-תיבות-והמרה-לגרשיים-תקניים)
   - [דוגמה 4: מחשבון גימטריה וסטטיסטיקת עמוד](#דוגמה-4-מחשבון-גימטריה-וסטטיסטיקת-עמוד)

---

## 1. ארכיטקטורת התוספים וביצועים

### עקרונות תכנון מרכזיים:
* **אפס פגיעה בביצועים (Zero Performance Overhead):** כל הכלים הטיפוגרפיים ב-`tok.hebrew` פועלים כפונקציות טהורות נטולות DOM או קריאות IPC, בזמן ביצוע של מיקרו-שניות ($O(n)$).
* **טרנזקציות אטומיות ומקבצים (`batch`):** שינויי טקסט של אלפי פסקאות מבוצעים במקבץ אטומי אחד (`tok.document.batch()`), תוך השהיית עדכוני DOM מיותרים וחישובי ספירת מילים עד לסיום הפעולה.
* **ארגז חול מבודד (Security Sandbox):** קוד התוסף מבודד לחלוטין מגישה ישירה לרכיבי Electron פנימיים (`tokIpc` מוצלל), ואין אפשרות לזליגת מידע או גישה לאחסון מקומי רגיש.
* **תאימות לשפות פיתוח:** תמיכה מלאה הן ב-TypeScript (`.ts`) והן ב-JavaScript מודרני (`.js`).

---

## 2. מבנה תוסף וקובץ `plugin.json`

כל תוסף ממוקם בתיקייה משלו תחת תיקיית התוספים של המערכת (ניתן לפתוח ישירות מתפריט ההגדרות: `הגדרות` ← `תוספים` ← `פתיחת תיקיית התוספים`).

### מבנה תיקיית התוסף:
```text
my-hebrew-plugin/
├── plugin.json       # מניפסט התוסף והרשאות
└── index.ts          # קובץ הכניסה (או index.js)
```

### דוגמה למניפסט `plugin.json`:
```json
{
  "id": "org.typesetok.divine-shield",
  "name": "מגן שמות קדושים",
  "version": "1.0.0",
  "description": "סורק את הטקסט, מזהה שמות שאינם נמחקים ומונע שיבושי מיקוף או מחיקה",
  "author": "TypesetOK Community",
  "main": "index.ts",
  "enabled": true,
  "permissions": [
    "document:read",
    "document:write",
    "ui:commands",
    "ui:notifications"
  ],
  "minTokVersion": "0.9.1"
}
```

---

## 3. מערכת ההרשאות ואבטחה (Permissions & Sandbox)

כדי למנוע נזק מתוספים זדוניים או שגויים, TypesetOK מיישמת מודל הרשאות מפורש (**Capability-based Security**). תוסף יכול לגשת אך ורק ליכולות שהצהיר עליהן במניפסט:

| הרשאה | תיאור | פעולות מותרות |
| :--- | :--- | :--- |
| `document:read` | קריאת תוכן המסמך | `getStory()`, `getParagraph()`, `getSelectedText()`, `getStats()` |
| `document:write` | שינוי ועריכת טקסט | `loadStory()`, `updateParagraph()`, `insertParagraph()`, `deleteParagraph()`, `replaceSelection()`, `batch()` |
| `ui:commands` | רישום פקודות במערכת | `registerCommand()` בסרגל הפקודות (Cmd/Ctrl+K) |
| `ui:notifications` | הצגת התראות | `showToast()` להודעות סטטוס והתראות |
| `ui:modals` | פתיחת חלונות מודאליים | `openModal()` להצגת דיאלוגים מותאמים אישית |
| `canvas:read` | קריאת תצוגת הקנבס | `getPageCount()`, `getActivePageIndex()`, `getZoom()`, `getViewMode()` |
| `canvas:navigate` | ניווט ושליטה בקנבס | `scrollToPage()`, `setZoom()`, `toggleMarginsGuide()`, `toggleBaselineGuide()` |
| `network:fetch` | גישה לרשת החיצונית | פניות HTTP/HTTPS (חסום לחלוטין כברירת מחדל ובמצב סייפר) |

### שכבות ההגנה בארגז החול:
1. **הצללת IPC (`window.tokIpc`):** לתוספים אין כל גישה לגשר ה-IPC של המעטפת או לתהליך הראשי של Electron.
2. **הסגר רשת (Network Quarantine):** ברירת המחדל היא עבודה מנותקת רשת (Offline-First). קריאות `fetch`, `XMLHttpRequest` ו-`WebSocket` חסומות אוטומטית.
3. **הגנת גלישת נתיבים (Path Traversal Guard):** התוכנה מוודאת שכל קובץ ריצה נמצא אך ורק בתוך תיקיית התוסף עצמה ומונעת קישורי `../` או Symlinks זדוניים.
4. **גבול בידוד קריסות (Error Boundary):** שגיאה או קריסה בתוסף נתפסת בארגז החול, מציגה הודעה ברורה למשתמש ואינה מפילה את תוכנת העימוד.

---

## 4. תמיכה במצב סייפר (Safer Workstations & Safe Mode)

TypesetOK כוללת התאמה מלאה למחשבים הפועלים תחת מערכות ניהול עמדות כגון **"סייפר" (Safer)** – הנפוצה בבתי כנסת, ישיבות, כוללים ומוסדות תורניים, וכן לתפעול במצב בטוח (**Safe Mode**):

### 1. זיהוי אוטומטי של סביבת סייפר:
* המערכת מזהה אוטומטית נוכחות של מערכת סייפר (`C:\Program Files\safer` או דגל `--safer`).
* כאשר מזוהה סייפר, התוכנה עוברת למצב עבודה מקומי מוחלט (**Strict Offline Mode**), מנטרלת בדיקות עדכונים וקריאות רשת שעלולות לעורר חסימות חומת אש או שגיאות תעודות SSL, ומציגה חיווי ייעודי בהגדרות.

### 2. תמיכה במחשבים מוקפאים (Deep Freeze / AppData חסום):
* בעמדות שבהן תיקיית המערכת נעולה לכתיבה, מנהל התוספים מבצע מעקף אוטומטי ושומר את הקבצים וההגדרות בתיקיית המסמכים של המשתמש (`Documents/TypesetOK/plugins`) ללא תקלות הרשאה (`EACCES`).

### 3. מצב בטוח (Safe Mode):
* ניתן להפעיל מצב בטוח בכל עת דרך לשונית התוספים בהגדרות, או באמצעות הפעלת התוכנה עם הדגל `--safe-mode`.
* **במצב בטוח:** כל התוספים החיצוניים מושבתים באופן מוחלט, ורק כלי הטיפוגרפיה המובנים והמאומתים פעילים.
* **כפתור השבתת חירום (Emergency Disable):** מאפשר למשתמש להשבית את כלל התוספים בלחיצה אחת במקרה של תוסף תקול או כבד מדי.

---

## 5. מפרט ה-API המלא (`tok.*`)

נקודת הכניסה של התוסף היא הפונקציה `activate(tok)`. האובייקט `tok` מספק גישה למרחבי השמות הבאים:

```typescript
export function activate(tok: TokPluginContext) {
  // קוד האתחול של התוסף
}
```

---

### `tok.document` - עריכת טקסט ומסמך

מאפשר קריאה ושינוי של טקסט המסמך בעורך הסיפור (Story Editor).

#### `tok.document.getStory(): StoryParagraph[]`
מחזיר מערך של כל הפסקאות במסמך הנוכחי.
```typescript
const paragraphs = tok.document.getStory();
console.log(`סך הכל פסקאות: ${paragraphs.length}`);
```

#### `tok.document.loadStory(paragraphs: StoryParagraph[]): void`
מחליף את כל תוכן המסמך במערך פסקאות חדש בפעולה אטומית אחת.
*(דורש הרשאת `document:write`)*

#### `tok.document.getParagraph(id: string): StoryParagraph | undefined`
מחזיר פסקה ספציפית לפי המזהה שלה.

#### `tok.document.updateParagraph(id: string, text: string, styleId?: string): boolean`
מעדכן טקסט או סגנון של פסקה בודדת.
*(דורש הרשאת `document:write`)*
```typescript
tok.document.updateParagraph('para-1', 'טקסט מעודכן');
```

#### `tok.document.insertParagraph(index: number, text: string, styleId?: string): string`
מוסיף פסקה חדשה במיקום המבוקש ומחזיר את המזהה החדש שנוצר.
*(דורש הרשאת `document:write`)*

#### `tok.document.deleteParagraph(id: string): boolean`
מוחק פסקה לפי מזהה.
*(דורש הרשאת `document:write`)*

#### `tok.document.getSelectedText(): string`
מחזיר את הטקסט שנבחר כעת על ידי המשתמש בעורך.

#### `tok.document.replaceSelection(text: string): void`
מחליף את הטקסט הנבחר בטקסט חדש.
*(דורש הרשאת `document:write`)*

#### `tok.document.getStats(): DocumentStats`
מחזיר סטטיסטיקה מיידית:
```typescript
const { wordCount, charCount, paragraphCount } = tok.document.getStats();
```

#### `tok.document.batch(fn: () => void): void`
מבצע מספר פעולות עריכה במקבץ אטומי אחד, תוך השהיית עדכוני תצוגה עד לסיום.
*(חיוני למניעת האטת התוכנה בעת עריכת מסמכים ארוכים)*
```typescript
tok.document.batch(() => {
  for (const p of tok.document.getStory()) {
    tok.document.updateParagraph(p.id, cleanText(p.text));
  }
});
```

#### `tok.document.transformText(fn: (text: string, para: StoryParagraph) => string): void`
עוזר ייעודי לעיבוד מהיר של כל פסקאות המסמך באמצעות פונקציה טהורה.
*(דורש הרשאת `document:write`)*

---

### `tok.hebrew` - טיפוגרפיה עברית, שמות קודש וגימטריה

כלי עזר עבריים מהירים במיוחד הפועלים ב-0 השהיה:

#### `tok.hebrew.toHebrewGematria(num: number): string`
ממיר מספר לערכו בגימטריה תקנית עם גרש/גרשיים והמרת שמות קודש וטאבו:
```typescript
tok.hebrew.toHebrewGematria(1);    // "א׳"
tok.hebrew.toHebrewGematria(15);   // "ט״ו"
tok.hebrew.toHebrewGematria(270);  // "ע״ר" (במקום ע"ע)
tok.hebrew.toHebrewGematria(5784); // "ה׳תשפ״ד"
```

#### `tok.hebrew.stripNiqqud(text: string): string`
מסיר את כל סימני הניקוד מהטקסט (שוא עד קמץ קטן, דגש, רפה, נקודות ש/ש) תוך שמירה על אותיות וטעמי מקרא.

#### `tok.hebrew.stripTaamim(text: string): string`
מסיר את כל טעמי המקרא תוך שמירה על הניקוד.

#### `tok.hebrew.stripAllMarks(text: string): string`
מסיר את כל הסימנים הדיאקריטיים (הן ניקוד והן טעמים) לקבלת אותיות בלבד.

#### `tok.hebrew.findHolyNames(text: string): HolyNameMatch[]`
סורק ומזהה שמות שאינם נמחקים (`יהוה`, `אלהים`, `אהיה`, `שדי`, `צבאות`, `אדני`) גם בתוך טקסט מנוקד וממוטעס, ומחזיר את המיקום המדויק ואורך התו המקורי:
```typescript
const matches = tok.hebrew.findHolyNames('בָּרוּךְ אַתָּה יהוה אֱלֹהִים');
// מחזיר:
// [
//   { name: 'יהוה', offset: 15, len: 4 },
//   { name: 'אלהים', offset: 20, len: 7 }
// ]
```

#### `tok.hebrew.normalizeQuotes(text: string): string`
ממיר גרשיים וגרש פשוטים (`"`, `'`) לגרשיים וגרש טיפוגרפיים עבריים תקניים (`״`, `׳`):
```typescript
tok.hebrew.normalizeQuotes('ר"ת ור\' משה'); // "ר״ת ור׳ משה"
```

#### `tok.hebrew.isHebrew(text: string): boolean`
בודק האם המחרוזת כוללת תווים באלפבית העברי.

---

### `tok.ui` - ממשק משתמש ופקודות

#### `tok.ui.registerCommand(cmd: PluginCommand): Disposable`
רושם פקודה בסרגל הפקודות (`Cmd/Ctrl+K`):
```typescript
const sub = tok.ui.registerCommand({
  id: 'my-plugin.clean-niqqud',
  title: 'הסרת ניקוד מהמסמך',
  category: 'טיפוגרפיה עברית',
  shortcut: 'Ctrl+Shift+K',
  action: () => {
    tok.document.transformText((text) => tok.hebrew.stripNiqqud(text));
    tok.ui.showToast('הניקוד הוסר בהצלחה');
  }
});
// הרישום מנוקה אוטומטית בהשבתת התוסף
tok.subscriptions.push(sub);
```

#### `tok.ui.showToast(msg: string, isError?: boolean): void`
מציג הודעת חיווי מהירה בתחתית המסך.

#### `tok.ui.getViewMode(): 'canvas' | 'story' | 'split'`
מחזיר את מצב התצוגה הפעיל.

#### `tok.ui.setViewMode(mode: 'canvas' | 'story' | 'split'): void`
משנה את מצב התצוגה של שולחן העבודה.

#### `tok.ui.getTheme(): string`
מחזיר את ערכת הצבעים הפעילה.

#### `tok.ui.getLanguage(): 'he' | 'en'`
מחזיר את שפת הממשק.

#### `tok.ui.openModal(title: string, render: (container: HTMLElement) => void | (() => void)): Disposable`
פותח חלון מודאלי מאובטח ומבודד להצגת הגדרות או ממשק מותאם אישית של התוסף.
*(דורש הרשאת `ui:modals`)*

---

### `tok.canvas` - קנבס, דפים וזום

#### `tok.canvas.getPageCount(): number`
מחזיר את מספר העמודים במסמך.

#### `tok.canvas.getActivePageIndex(): number`
מחזיר את האינדקס של העמוד הפעיל כעת (החל מ-0).

#### `tok.canvas.scrollToPage(index: number): void`
גולל את הקנבס ישירות לעמוד המבוקש.
*(דורש הרשאת `canvas:navigate`)*

#### `tok.canvas.getZoom(): number` / `setZoom(percent: number): void`
קריאה ושינוי של רמת התקריב באחוזים (25% עד 400%).
*(שינוי דורש הרשאת `canvas:navigate`)*

#### `tok.canvas.toggleMarginsGuide()` / `toggleBaselineGuide()`
מדליק או מכבה את קווי העזר לשוליים ולרשת השורות.

---

### `tok.events` - אירועי מערכת

מאפשר האזנה לאירועים בזמן אמת:
* `'text-change'` - מופעל בעת עריכת טקסט: `(paraId: string, text: string) => void`
* `'page-change'` - מופעל במעבר עמוד: `(pageIndex: number) => void`
* `'view-change'` - מופעל במעבר בין תצוגות: `(viewMode: string) => void`

כל האזנה מחזירה אובייקט `Disposable` עם מתודה `dispose()` לביטול מהיר.

---

### `tok.env` - סטטוס סביבה ומצב בטוח

מספק מידע חיוני על מצב האבטחה של התוכנה:
* `tok.env.appVersion`: גרסת התוכנה המותקנת (למשל `"0.9.1"`).
* `tok.env.isSafeMode`: האם פועל מצב בטוח (Safe Mode).
* `tok.env.isSaferActive`: האם זוהתה סביבת עמדות מחשב מנוהלת (סייפר).
* `tok.env.platform`: מערכת ההפעלה (`win32`, `darwin`, `linux`).

---

## 6. מחזור חיים וניקוי משאבים (Lifecycle & Disposable)

מערכת התוספים דואגת לניקוי יסודי של משאבים כאשר תוסף מושבת או נטען מחדש:

1. **מערך `tok.subscriptions`:**
   כל פקודה, האזנה לאירוע או חלון מודאלי שמתווספים למערך `tok.subscriptions` מנוקים אוטומטית בעת השבתת התוסף.
2. **מתודת `tok.onDeactivate(cleanupFn)`:**
   מאפשרת רישום פונקציית ניקוי עצמאית (למשל עצירת `setInterval` או שחרור זיכרון).
3. **ייצוא פונקציית `deactivate()`:**
   תוסף יכול לייצא פונקציה זו ישירות:
   ```typescript
   export function deactivate() {
     console.log('התוסף הושבת בהצלחה');
   }
   ```

---

## 7. דוגמאות קוד מלאות לשימוש מעשי

### דוגמה 1: מנקה ניקוד וטעמים לחיפוש והדפסה
תוסף המוסיף שתי פקודות: אחת להסרת ניקוד והשנייה להסרת טעמי מקרא.

```typescript
// index.ts
import type { TokPluginContext } from './plugin-types';

export function activate(tok: TokPluginContext) {
  // 1. פקודה להסרת ניקוד
  tok.registerCommand({
    id: 'org.tok.strip-niqqud',
    title: 'הסרת ניקוד מהטקסט הנבחר / מהמסמך',
    category: 'טיפוגרפיה עברית',
    action: () => {
      const selected = tok.document.getSelectedText();
      if (selected) {
        tok.document.replaceSelection(tok.hebrew.stripNiqqud(selected));
        tok.showToast('הניקוד הוסר מהטקסט הנבחר');
      } else {
        tok.document.batch(() => {
          tok.document.transformText((text) => tok.hebrew.stripNiqqud(text));
        });
        tok.showToast('כל הניקוד במסמך הוסר בהצלחה');
      }
    }
  });

  // 2. פקודה להסרת טעמי מקרא בלבד (השארת ניקוד)
  tok.registerCommand({
    id: 'org.tok.strip-taamim',
    title: 'הסרת טעמי מקרא בלבד (שמירת ניקוד)',
    category: 'טיפוגרפיה עברית',
    action: () => {
      tok.document.batch(() => {
        tok.document.transformText((text) => tok.hebrew.stripTaamim(text));
      });
      tok.showToast('טעמי המקרא הוסרו בהצלחה (הניקוד נשמר)');
    }
  });
}
```

---

### דוגמה 2: מגן שמות קדושים (בדיקת שמות שאינם נמחקים)
סורק את כל פסקאות המסמך, מאתר שמות שאינם נמחקים ומציג דוח מסכם.

```typescript
// index.ts
import type { TokPluginContext } from './plugin-types';

export function activate(tok: TokPluginContext) {
  tok.registerCommand({
    id: 'org.tok.scan-holy-names',
    title: 'סריקת שמות קדושים (שמות שאינם נמחקים)',
    category: 'בדיקות והגהה',
    action: () => {
      const story = tok.document.getStory();
      let totalFound = 0;
      const occurrences: string[] = [];

      for (const para of story) {
        const matches = tok.hebrew.findHolyNames(para.text);
        if (matches.length > 0) {
          totalFound += matches.length;
          matches.forEach((m) => occurrences.push(m.name));
        }
      }

      if (totalFound === 0) {
        tok.showToast('לא נמצאו שמות קדושים במסמך');
      } else {
        tok.showToast(
          `זוהו ${totalFound} מופעים של שמות קדושים במסמך (${[...new Set(occurrences)].join(', ')})`
        );
      }
    }
  });
}
```

---

### דוגמה 3: תיקון ראשי תיבות והמרה לגרשיים תקניים
מזהה ראשי תיבות שנכתבו עם מירכאות מקלדת פשוטות (`"`, `'`) וממיר אותן לגרשיים עבריים תקניים (`״`, `׳`).

```typescript
// index.ts
import type { TokPluginContext } from './plugin-types';

export function activate(tok: TokPluginContext) {
  tok.registerCommand({
    id: 'org.tok.fix-gershayim',
    title: 'תיקון ראשי תיבות לגרשיים וגרש תקניים',
    category: 'טיפוגרפיה עברית',
    shortcut: 'Ctrl+Shift+G',
    action: () => {
      let count = 0;
      tok.document.batch(() => {
        tok.document.transformText((text) => {
          const fixed = tok.hebrew.normalizeQuotes(text);
          if (fixed !== text) count++;
          return fixed;
        });
      });
      tok.showToast(`תוקנו ראשי תיבות וגרשיים ב-${count} פסקאות`);
    }
  });
}
```

---

### דוגמה 4: מחשבון גימטריה וסטטיסטיקת עמוד
מחשב את הגימטריה של הטקסט הנבחר ומציג חיווי מהיר.

```typescript
// index.ts
import type { TokPluginContext } from './plugin-types';

export function activate(tok: TokPluginContext) {
  tok.registerCommand({
    id: 'org.tok.calc-gematria',
    title: 'חישוב גימטריה של הטקסט הנבחר',
    category: 'עזרי מחקר',
    action: () => {
      const selected = tok.document.getSelectedText().trim();
      if (!selected) {
        tok.showToast('יש לבחור טקסט לחישוב גימטריה', true);
        return;
      }

      const stripped = tok.hebrew.stripAllMarks(selected);
      let sum = 0;
      const values: Record<string, number> = {
        'א': 1, 'ב': 2, 'ג': 3, 'ד': 4, 'ה': 5, 'ו': 6, 'ז': 7, 'ח': 8, 'ט': 9,
        'י': 10, 'כ': 20, 'ך': 20, 'ל': 30, 'מ': 40, 'ם': 40, 'נ': 50, 'ן': 50,
        'ס': 60, 'ע': 70, 'פ': 80, 'ף': 80, 'צ': 90, 'ץ': 90, 'ק': 100, 'ר': 200,
        'ש': 300, 'ת': 400
      };

      for (const ch of stripped) {
        if (values[ch]) sum += values[ch];
      }

      const hebrewNumeral = tok.hebrew.toHebrewGematria(sum);
      tok.showToast(`ערך גימטריה: ${sum.toLocaleString()} (${hebrewNumeral})`);
    }
  });
}
```

---

## 8. סיכום שאלות נפוצות (FAQ)

**ש: האם תוספים יכולים לגרום להאטה בהקלדה בעורך?**  
ת: לא. קוד התוספים אינו רץ על כל לחיצת מקש אלא מופעל אך ורק דרך פקודות יזומות או אירועים. שינויי טקסט מבוצעים במקבצים אטומיים (`batch()`) ללא עלות תצוגה מיותרת.

**ש: כיצד ניתן לוודא שהתוסף יעבוד היטב בעמדות סייפר של בתי כנסת?**  
ת: ודאו שהתוסף אינו דורש חיבור לאינטרנט ואינו מנסה לגשת לקבצים מחוץ לתיקייתו. כל הפונקציות ב-`tok.hebrew` ו-`tok.document` פועלות באופן מושלם ומנותק לחלוטין מכל רשת.

**ש: מה קורה אם תוסף מכיל לולאה אינסופית או שגיאת קוד?**  
ת: המערכת עוטפת כל הפעלת פקודה בגבול שגיאות (Error Boundary). השגיאה נלכדת מיד, מוצגת כהודעת טוסט ברורה למשתמש, והתוכנה ממשיכה לפעול כרגיל. במקרה חמור, ניתן להפעיל את התוכנה עם הדגל `--safe-mode` כדי לבטל את כל התוספים.

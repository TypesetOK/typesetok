import { i18n } from '../i18n';
import { hostSettings } from '../platform/hostSettings.js';

export interface RecentProject {
  name: string;
  path?: string;
  time?: string;
  lastSavedAt?: string;
  lastClosedAt?: string;
  pages: number;
  template: number;
  templateId?: string;
  templateName?: string;
}

const TEMPLATE_ID_TO_INDEX: Record<string, number> = {
  gemara: 0,
  mikraot: 1,
  prose: 2,
  notes: 3,
  bulletin: 4,
};

export function resolveTemplateIndex(templateId?: string): number {
  if (!templateId) return 0;
  return TEMPLATE_ID_TO_INDEX[templateId] ?? 2; // default to 'prose' (index 2) if unknown/blank
}

const DUMMY_PROJECT_NAMES = new Set([
  'מסכת ברכות — מהדורת מופת.tok',
  'מסכת ברכות — מהדורת מופת',
  'מסכת ברכות — צורת הדף.tok',
  'מסכת ברכות — צורת הדף',
  'מסכת ברכות.tok',
  'מסכת ברכות',
  'ספר תהילים עם פירוש המילות.tok',
  'ספר תהילים עם פירוש המילות',
  'ספר תהילים.tok',
  'ספר תהילים',
  'עלון שבת קודש — גיליון ק״מ.tok',
  'עלון שבת קודש — גיליון ק״מ',
  'עלון שבת קודש.tok',
  'עלון שבת קודש',
  'דמו — מסכת ברכות.tok',
  'דמו — מסכת ברכות',
  'דמו — מקראות גדולות.tok',
  'דמו — מקראות גדולות',
  'דמו — ספר קריאה וטקסט רציף.tok',
  'דמו — ספר קריאה וטקסט רציף',
  'דמו — ספר עם הערות שוליים.tok',
  'דמו — ספר עם הערות שוליים',
  'דמו — עלון וקונטרס.tok',
  'דמו — עלון וקונטרס',
  'פרויקט דף גמרא.tok',
  'פרויקט דף גמרא',
  'דף גמרא.tok',
  'דף גמרא',
  'פרויקט מקראות גדולות.tok',
  'פרויקט מקראות גדולות',
  'מקראות גדולות.tok',
  'מקראות גדולות',
  'מקראות גדולות — בראשית.tok',
  'מקראות גדולות — בראשית',
  'ספר קריאה וטקסט רציף.tok',
  'ספר קריאה וטקסט רציף',
  'ספר קריאה — מהדורה ראשונה.tok',
  'ספר קריאה — מהדורה ראשונה',
  'ספר עם הערות שוליים.tok',
  'ספר עם הערות שוליים',
  'ספר עם הערות — חלק א׳.tok',
  'ספר עם הערות — חלק א׳',
  'עלון וקונטרס.tok',
  'עלון וקונטרס',
  'מסמך ריק.tok',
  'מסמך ריק'
]);

/**
 * Checks whether a project name or path is a demo / sample / dummy project
 * which must never appear on the home screen or anywhere in the UI.
 */
export function isDemoProject(name?: string, path?: string): boolean {
  if (!name && !path) return true;
  const n = (name || '').trim();
  const lower = n.toLowerCase();
  const p = (path || '').toLowerCase();
  const cleanName = n.replace(/\.tok$/i, '').trim();

  // Any project without a real directory path is a sample / phantom project
  if (!path || path === name || (!path.includes('/') && !path.includes('\\'))) {
    return true;
  }

  if (DUMMY_PROJECT_NAMES.has(n) || DUMMY_PROJECT_NAMES.has(cleanName)) return true;

  if (
    lower.startsWith('דמו') ||
    lower.includes('דמו —') ||
    lower.includes('דמו -') ||
    lower.includes('דמו:') ||
    lower.includes('demo') ||
    lower.includes('דוגמה') ||
    lower.includes('דוגמא') ||
    lower.includes('sample') ||
    lower.includes('תבנית')
  ) {
    return true;
  }

  if (p.includes('tok_demo') || p.includes('demo') || p.includes('sample')) {
    return true;
  }

  return false;
}

/**
 * Actively purges all legacy sample, demo, or phantom projects from storage.
 */
export function purgeAllDemoProjects(): void {
  try {
    const raw = hostSettings.get('tok_recent_projects');
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    const cleaned = parsed.filter((p) => p && typeof p.name === 'string' && !isDemoProject(p.name, p.path));
    hostSettings.set('tok_recent_projects', JSON.stringify(cleaned));
  } catch (err) {
    console.warn('[WELCOME] Failed to prune demo recent projects:', err);
  }
}

/** Format accurate project timestamp into readable Hebrew/English date and time. */
export function formatProjectTimestamp(isoOrDate?: string): string {
  if (!isoOrDate || isoOrDate === 'זה עתה') {
    const d = new Date();
    const timeStr = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
    return `${d.getDate().toString().padStart(2, '0')}/${(d.getMonth() + 1).toString().padStart(2, '0')}/${d.getFullYear()} ${timeStr}`;
  }
  try {
    const d = new Date(isoOrDate);
    if (isNaN(d.getTime())) return isoOrDate;
    const now = new Date();
    const isToday = d.toDateString() === now.toDateString();
    const timeStr = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
    if (isToday) {
      return i18n.getLanguage() === 'en' ? `Today ${timeStr}` : `היום ${timeStr}`;
    }
    const dateStr = `${d.getDate().toString().padStart(2, '0')}/${(d.getMonth() + 1).toString().padStart(2, '0')}/${d.getFullYear()}`;
    return `${dateStr} ${timeStr}`;
  } catch {
    return isoOrDate;
  }
}

export function getRecentProjects(): RecentProject[] {
  try {
    const raw = hostSettings.get('tok_recent_projects');
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        const filtered = parsed
          .filter((p) => p && typeof p.name === 'string' && !isDemoProject(p.name, p.path));
        if (filtered.length !== parsed.length) {
          hostSettings.set('tok_recent_projects', JSON.stringify(filtered));
        }
        return filtered.map((p) => {
          const accurateTime = p.lastSavedAt || p.lastClosedAt;
          return {
            ...p,
            time: formatProjectTimestamp(accurateTime || p.time)
          };
        });
      }
    }
  } catch (err) {
    console.warn('[WELCOME] Failed to read recent projects:', err);
  }
  return [];
}

export function addRecentProject(entry: {
  name: string;
  path?: string;
  pages?: number;
  template?: number;
  templateId?: string;
  templateName?: string;
  lastSavedAt?: string;
  lastClosedAt?: string;
}): void {
  if (!entry || isDemoProject(entry.name, entry.path)) {
    return;
  }
  try {
    const nowIso = new Date().toISOString();
    const saved = entry.lastSavedAt || nowIso;
    const list = getRecentProjects().filter((p) => p.name !== entry.name && p.path !== entry.path);
    const resolvedIdx = entry.template ?? (entry.templateId ? resolveTemplateIndex(entry.templateId) : 0);
    list.unshift({
      name: entry.name,
      path: entry.path,
      lastSavedAt: saved,
      lastClosedAt: entry.lastClosedAt,
      time: formatProjectTimestamp(saved),
      pages: entry.pages ?? 1,
      template: resolvedIdx,
      ...(entry.templateId ? { templateId: entry.templateId } : {}),
      ...(entry.templateName ? { templateName: entry.templateName } : {})
    });
    hostSettings.set('tok_recent_projects', JSON.stringify(list.slice(0, 50)));
  } catch (err) {
    console.warn('[WELCOME] Failed to save recent project:', err);
  }
}

export function updateProjectClosedTime(nameOrPath: string): void {
  try {
    const list = getRecentProjects();
    const nowIso = new Date().toISOString();
    for (const p of list) {
      if (p.name === nameOrPath || p.path === nameOrPath) {
        p.lastClosedAt = nowIso;
        break;
      }
    }
    hostSettings.set('tok_recent_projects', JSON.stringify(list));
  } catch (err) {
    console.warn('[WELCOME] Failed to update recent projects:', err);
  }
}

export function removeRecentProject(nameOrPath: string): void {
  if (!nameOrPath) return;
  try {
    const list = getRecentProjects().filter((p) => p.name !== nameOrPath && p.path !== nameOrPath);
    hostSettings.set('tok_recent_projects', JSON.stringify(list));
  } catch (err) {
    console.warn('[WELCOME] Failed to update recent projects:', err);
  }
}

export function renameRecentProject(oldName: string, newName: string): void {
  try {
    const cleanNew = newName.trim();
    if (!cleanNew) return;
    const list = getRecentProjects().map((p) => {
      if (p.name === oldName) {
        return { ...p, name: cleanNew };
      }
      return p;
    });
    hostSettings.set('tok_recent_projects', JSON.stringify(list));
  } catch (err) {
    console.warn('[WELCOME] Failed to update recent projects:', err);
  }
}

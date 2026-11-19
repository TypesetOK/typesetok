import {
  ChangelogEntry,
  UpdatesFilterOptions,
  UserUpdatesState,
  UpdateImportance,
  ChangeCategory
} from './types';
import { CHANGELOG_ENTRIES } from './changelogData.js';
import { hostSettings } from '../platform/hostSettings.js';

const STORAGE_KEY = 'tok_updates_state_v1';

export function compareSemver(v1: string, v2: string): number {
  const clean1 = (v1 || '').replace(/^v/i, '').trim();
  const clean2 = (v2 || '').replace(/^v/i, '').trim();
  const p1 = clean1.split('.').map((x) => parseInt(x, 10) || 0);
  const p2 = clean2.split('.').map((x) => parseInt(x, 10) || 0);
  const len = Math.max(p1.length, p2.length);
  for (let i = 0; i < len; i++) {
    const num1 = p1[i] ?? 0;
    const num2 = p2[i] ?? 0;
    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }
  return 0;
}

export class UpdatesStateManager {
  private static instance: UpdatesStateManager;
  private state: UserUpdatesState;
  private entries: ChangelogEntry[];
  private listeners: Set<(state: UserUpdatesState) => void> = new Set();

  private constructor(entries: ChangelogEntry[] = CHANGELOG_ENTRIES) {
    this.entries = [...entries].sort((a, b) => compareSemver(b.version, a.version));
    this.state = this.loadState();
  }

  public static getInstance(customEntries?: ChangelogEntry[]): UpdatesStateManager {
    if (!UpdatesStateManager.instance || customEntries) {
      UpdatesStateManager.instance = new UpdatesStateManager(customEntries);
    }
    return UpdatesStateManager.instance;
  }

  /** Clears singleton (useful for isolated tests). */
  public static resetInstance(): void {
    (UpdatesStateManager as any).instance = undefined;
  }

  private loadState(): UserUpdatesState {
    const defaultState: UserUpdatesState = {
      lastSeenVersion: '0.0.0',
      lastSeenMajorVersion: '0.0.0',
      dismissedVersions: []
    };

    try {
      const raw = hostSettings.get(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        return {
          lastSeenVersion: typeof parsed.lastSeenVersion === 'string' ? parsed.lastSeenVersion : defaultState.lastSeenVersion,
          lastSeenMajorVersion: typeof parsed.lastSeenMajorVersion === 'string' ? parsed.lastSeenMajorVersion : defaultState.lastSeenMajorVersion,
          dismissedVersions: Array.isArray(parsed.dismissedVersions) ? parsed.dismissedVersions : defaultState.dismissedVersions,
          lastCheckedTime: typeof parsed.lastCheckedTime === 'number' ? parsed.lastCheckedTime : undefined
        };
      }
    } catch (e) {
      console.warn('[UPDATES] Failed to load updates state:', e);
    }

    return defaultState;
  }

  private persistState(): void {
    try {
      hostSettings.set(STORAGE_KEY, JSON.stringify(this.state));
    } catch (e) {
      console.warn('[UPDATES] Failed to persist updates state:', e);
    }
    this.notify();
  }

  private notify(): void {
    for (const listener of this.listeners) {
      try {
        listener({ ...this.state });
      } catch (e) {
        console.error('[UPDATES] Listener error:', e);
      }
    }
  }

  public subscribe(listener: (state: UserUpdatesState) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  public getState(): Readonly<UserUpdatesState> {
    return { ...this.state };
  }

  public getAllEntries(): ChangelogEntry[] {
    return [...this.entries];
  }

  public getLatestEntry(): ChangelogEntry | undefined {
    return this.entries[0];
  }

  public getLatestMajorEntry(): ChangelogEntry | undefined {
    return this.entries.find((e) => e.importance === 'major');
  }

  public getEntry(version: string): ChangelogEntry | undefined {
    return this.entries.find((e) => e.version === version.replace(/^v/i, ''));
  }

  /**
   * Checks whether the current version represents a major update that should prompt
   * a modal popup to the user on startup.
   */
  public shouldPromptMajorUpdate(targetVersion?: string): boolean {
    const versionToCheck = targetVersion || this.getLatestEntry()?.version;
    if (!versionToCheck) return false;

    const entry = this.getEntry(versionToCheck);
    if (!entry || entry.importance !== 'major') {
      return false;
    }

    // Don't show if user previously dismissed this version prompt
    if (this.state.dismissedVersions.includes(versionToCheck)) {
      return false;
    }

    // Compare with last seen major version
    return compareSemver(versionToCheck, this.state.lastSeenMajorVersion) > 0;
  }

  /**
   * Checks if there are unread minor or patch updates that should be highlighted
   * in the side drawer without an intrusive popup.
   */
  public hasUnreadMinorUpdates(targetVersion?: string): boolean {
    const versionToCheck = targetVersion || this.getLatestEntry()?.version;
    if (!versionToCheck) return false;

    const entry = this.getEntry(versionToCheck);
    if (!entry || entry.importance === 'major') {
      return false;
    }

    return compareSemver(versionToCheck, this.state.lastSeenVersion) > 0;
  }

  /**
   * Returns true if there is any update (major, minor, or patch) newer than lastSeenVersion.
   */
  public hasUnreadUpdates(targetVersion?: string): boolean {
    const versionToCheck = targetVersion || this.getLatestEntry()?.version;
    if (!versionToCheck) return false;

    return compareSemver(versionToCheck, this.state.lastSeenVersion) > 0;
  }

  /**
   * Count of entries newer than lastSeenVersion.
   */
  public getUnreadCount(): number {
    return this.entries.filter((e) => compareSemver(e.version, this.state.lastSeenVersion) > 0).length;
  }

  /**
   * Marks a version (and all older versions) as seen.
   */
  public markVersionAsSeen(version: string): void {
    const clean = version.replace(/^v/i, '');
    let changed = false;

    if (compareSemver(clean, this.state.lastSeenVersion) > 0) {
      this.state.lastSeenVersion = clean;
      changed = true;
    }

    const entry = this.getEntry(clean);
    if (entry && entry.importance === 'major' && compareSemver(clean, this.state.lastSeenMajorVersion) > 0) {
      this.state.lastSeenMajorVersion = clean;
      changed = true;
    }

    if (changed) {
      this.persistState();
    }
  }

  /**
   * Dismisses a major update prompt so it won't pop up again.
   */
  public dismissMajorPrompt(version: string): void {
    const clean = version.replace(/^v/i, '');
    if (!this.state.dismissedVersions.includes(clean)) {
      this.state.dismissedVersions.push(clean);
    }
    if (compareSemver(clean, this.state.lastSeenMajorVersion) > 0) {
      this.state.lastSeenMajorVersion = clean;
    }
    if (compareSemver(clean, this.state.lastSeenVersion) > 0) {
      this.state.lastSeenVersion = clean;
    }
    this.persistState();
  }

  /**
   * Marks all entries in the registry as seen.
   */
  public markAllAsSeen(): void {
    const latest = this.getLatestEntry();
    if (latest) {
      this.state.lastSeenVersion = latest.version;
      const latestMajor = this.getLatestMajorEntry();
      if (latestMajor) {
        this.state.lastSeenMajorVersion = latestMajor.version;
      }
      this.persistState();
    }
  }

  /**
   * Filters changelog entries based on category, importance, or textual search query.
   */
  public filterEntries(options: UpdatesFilterOptions = {}, lang: 'he' | 'en' = 'he'): ChangelogEntry[] {
    let result = [...this.entries];

    if (options.importance && options.importance !== 'all') {
      result = result.filter((e) => e.importance === options.importance);
    }

    if (options.category && options.category !== 'all') {
      const cat = options.category;
      result = result.filter((e) => e.changes.some((c) => c.category === cat));
    }

    if (options.searchQuery && options.searchQuery.trim()) {
      const q = options.searchQuery.trim().toLowerCase();
      result = result.filter((e) => {
        const titleMatch = (e.title[lang] || '').toLowerCase().includes(q) ||
                           (e.title.en || '').toLowerCase().includes(q) ||
                           (e.title.he || '').toLowerCase().includes(q);
        const summaryMatch = (e.summary[lang] || '').toLowerCase().includes(q);
        const versionMatch = e.version.includes(q);
        const changesMatch = e.changes.some((c) =>
          (c.text[lang] || '').toLowerCase().includes(q) ||
          (c.text.en || '').toLowerCase().includes(q) ||
          (c.text.he || '').toLowerCase().includes(q)
        );
        return titleMatch || summaryMatch || versionMatch || changesMatch;
      });
    }

    return result;
  }
}

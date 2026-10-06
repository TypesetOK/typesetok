import { app, shell } from 'electron';
import type { IncomingMessage } from 'http';
import { logger } from './logger';
import { fetchWithFilterSslFallback, diagnoseFilterSslError, isFilterSslError } from './filter-ssl';

export interface UpdateCheckResult {
  hasUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseName: string;
  releaseNotes: string;
  releaseUrl: string;
  downloadUrl?: string;
  publishedAt?: string;
  error?: string;
}

const MAX_RESPONSE_BYTES = 2 * 1024 * 1024;
const REQUEST_TIMEOUT_MS = 8000;

/**
 * The app version. Packaged builds carry it in resources/app/package.json
 * (app.getVersion()); when running from source Electron would report its own
 * version, so the monorepo root package.json is used instead.
 */
let cachedVersion: string | undefined;
export function getAppVersion(): string {
  if (cachedVersion) return cachedVersion;
  let v = '';
  if (app.isPackaged) {
    v = app.getVersion();
  } else {
    try {
      // packages/tok-electron/dist -> repo root
      v = require('../../../package.json').version;
    } catch {
      v = '';
    }
  }
  cachedVersion = v || '0.9.8';
  return cachedVersion;
}

/** Only http(s) URLs may be handed to the OS shell. */
export function isSafeExternalUrl(url: unknown): url is string {
  if (typeof url !== 'string') return false;
  try {
    const u = new URL(url);
    return u.protocol === 'https:' || u.protocol === 'http:';
  } catch {
    return false;
  }
}

export class TokUpdater {
  private repoOwner = 'TypesetOK';
  private repoName = 'typesetok';
  private currentVersion?: string;
  private inFlight: Promise<UpdateCheckResult> | null = null;

  constructor(currentVersion?: string) {
    this.currentVersion = currentVersion;
  }

  public getCurrentVersion(): string {
    return this.currentVersion || getAppVersion();
  }

  private get releasesPage(): string {
    return `https://github.com/${this.repoOwner}/${this.repoName}/releases`;
  }

  /**
   * Compares two semantic version strings (e.g. "0.7.0" vs "0.8.0" or "v0.8.0").
   * Returns: 1 if v1 > v2, -1 if v1 < v2, 0 if equal.
   */
  public compareVersions(v1: string, v2: string): number {
    const clean1 = v1.replace(/^v/i, '').trim();
    const clean2 = v2.replace(/^v/i, '').trim();
    const parts1 = clean1.split('.').map(n => parseInt(n, 10) || 0);
    const parts2 = clean2.split('.').map(n => parseInt(n, 10) || 0);

    const len = Math.max(parts1.length, parts2.length);
    for (let i = 0; i < len; i++) {
      const p1 = parts1[i] ?? 0;
      const p2 = parts2[i] ?? 0;
      if (p1 > p2) return 1;
      if (p1 < p2) return -1;
    }
    return 0;
  }

  private failure(error: string): UpdateCheckResult {
    const current = this.getCurrentVersion();
    return {
      hasUpdate: false,
      currentVersion: current,
      latestVersion: current,
      releaseName: '',
      releaseNotes: '',
      releaseUrl: this.releasesPage,
      error
    };
  }

  private parseRelease(data: string): UpdateCheckResult {
    const current = this.getCurrentVersion();
    const release = JSON.parse(data);
    if (!release || typeof release !== 'object') throw new Error('unexpected response shape');
    const tagName: string = String(release.tag_name || release.name || '');
    const latestVersion = tagName.replace(/^v/i, '');
    const hasUpdate = latestVersion !== '' && this.compareVersions(latestVersion, current) > 0;
    const releaseUrl = isSafeExternalUrl(release.html_url) ? release.html_url : this.releasesPage;

    // Prefer a Windows installer or zip asset if available
    let downloadUrl = releaseUrl;
    if (Array.isArray(release.assets)) {
      const asset = release.assets.find((a: any) =>
        typeof a?.name === 'string' && (a.name.endsWith('.exe') || a.name.endsWith('.zip'))
      );
      if (asset && isSafeExternalUrl(asset.browser_download_url)) {
        downloadUrl = asset.browser_download_url;
      }
    }

    return {
      hasUpdate,
      currentVersion: current,
      latestVersion: latestVersion || current,
      releaseName: String(release.name || tagName),
      releaseNotes: typeof release.body === 'string' && release.body ? release.body : 'אין הערות שחרור זמינות',
      releaseUrl,
      downloadUrl,
      publishedAt: typeof release.published_at === 'string' ? release.published_at : undefined
    };
  }

  /**
   * Checks GitHub Releases API for the latest published release.
   * Never rejects: network/parse problems are reported in `error`.
   * Concurrent calls share one request.
   */
  public checkForUpdates(): Promise<UpdateCheckResult> {
    if (!this.inFlight) {
      this.inFlight = this.doCheck().finally(() => {
        this.inFlight = null;
      });
    }
    return this.inFlight;
  }

  private async doCheck(): Promise<UpdateCheckResult> {
    logger.info('[UPDATER] Checking for updates against GitHub Releases (kosher-filter aware)...');
    const url = `https://api.github.com/repos/${this.repoOwner}/${this.repoName}/releases/latest`;

    try {
      const data = await fetchWithFilterSslFallback(url, {
        timeoutMs: REQUEST_TIMEOUT_MS,
        userAgent: `TypesetOK-Desktop/${this.getCurrentVersion()}`,
      });

      const result = this.parseRelease(data);
      logger.info('[UPDATER] Update check completed', {
        hasUpdate: result.hasUpdate,
        latestVersion: result.latestVersion,
        current: result.currentVersion,
      });
      return result;
    } catch (err: any) {
      logger.warn('[UPDATER] Update check failed:', { error: err?.message });
      const diagnosis = diagnoseFilterSslError(err);
      if (diagnosis.isFilterError) {
        return this.failure(
          `${diagnosis.errorMessageHebrew} ${diagnosis.recommendedActionHebrew}`
        );
      }
      return this.failure(`שגיאה בבדיקת עדכונים: ${err?.message || String(err)}`);
    }
  }

  public openReleaseUrl(url?: string): void {
    const target = isSafeExternalUrl(url) ? url : this.releasesPage;
    shell.openExternal(target).catch((err) => logger.warn('[UPDATER] Failed to open release URL', { error: err?.message }));
  }
}

export const updater = new TokUpdater();

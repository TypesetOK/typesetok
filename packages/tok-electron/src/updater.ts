import { app, shell } from 'electron';
import type { IncomingMessage } from 'http';
import * as http from 'http';
import * as https from 'https';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import * as crypto from 'crypto';
import { spawn } from 'child_process';
import { URL } from 'url';
import { logger } from './logger';
import { fetchWithFilterSslFallback, diagnoseFilterSslError, isFilterSslError } from './filter-ssl';

export interface UpdateDownloadProgress {
  percent: number;
  transferredBytes: number;
  totalBytes: number;
  speedBytesPerSec: number;
  etaSeconds: number;
}

export interface DownloadUpdateResult {
  success: boolean;
  filePath?: string;
  fileSize?: number;
  sha256?: string;
  error?: string;
}

export interface ApplyUpdateResult {
  success: boolean;
  error?: string;
}

export interface OfflineUpdateCandidate {
  filePath: string;
  fileName: string;
  fileSize: number;
  version?: string;
  isNewer?: boolean;
  isCurrent?: boolean;
  isRecommended?: boolean;
  driveLetter?: string;
  sha256?: string;
  modifiedAt?: string;
  statusDescriptionHebrew?: string;
}

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
  cachedVersion = v || '0.35.0';
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

  /**
   * Computes the SHA-256 hash of a local file using native Node crypto streaming.
   */
  public async computeFileSha256(filePath: string): Promise<string> {
    return new Promise((resolve, reject) => {
      if (!fs.existsSync(filePath)) return reject(new Error('קובץ לא נמצא'));
      const hash = crypto.createHash('sha256');
      const stream = fs.createReadStream(filePath);
      stream.on('data', (chunk) => hash.update(chunk));
      stream.on('end', () => resolve(hash.digest('hex')));
      stream.on('error', (err) => reject(err));
    });
  }

  /**
   * Validates executable binary integrity (minimum size & valid binary magic bytes).
   * Ensures corrupt or incomplete files are never launched.
   */
  public validateExecutableBinary(filePath: string): boolean {
    try {
      if (!fs.existsSync(filePath)) return false;
      const stat = fs.statSync(filePath);
      if (stat.size < 100 * 1024) return false; // Installers must be >= 100KB

      const fd = fs.openSync(filePath, 'r');
      const buf = Buffer.alloc(4);
      fs.readSync(fd, buf, 0, 4, 0);
      fs.closeSync(fd);

      const isWin = process.platform === 'win32' || filePath.toLowerCase().endsWith('.exe');
      if (isWin) {
        // Windows PE executable header: 'MZ' (0x4D, 0x5A)
        return buf[0] === 0x4d && buf[1] === 0x5a;
      }
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Downloads an update directly inside the app to temp storage with atomic write,
   * live speed/ETA estimation, and automatic SHA-256 hash calculation.
   */
  public async downloadUpdate(
    downloadUrl: string,
    onProgress?: (progress: UpdateDownloadProgress) => void
  ): Promise<DownloadUpdateResult> {
    if (!isSafeExternalUrl(downloadUrl)) {
      return { success: false, error: 'כתובת הורדה לא חוקית' };
    }

    try {
      const tempDir = app?.getPath ? app.getPath('temp') : path.join(process.cwd(), 'temp');
      if (!fs.existsSync(tempDir)) {
        fs.mkdirSync(tempDir, { recursive: true });
      }

      let fileName = path.basename(new URL(downloadUrl).pathname);
      if (!fileName || !fileName.includes('.')) {
        fileName = `TypesetOK-Update-${Date.now()}.exe`;
      }
      const targetPath = path.join(tempDir, fileName);
      const partPath = `${targetPath}.part`;

      logger.info('[UPDATER] Downloading update directly (in-app stream)...', {
        downloadUrl,
        targetPath,
        partPath
      });

      const sha256 = await this.downloadFileStreaming(downloadUrl, partPath, onProgress);

      if (!this.validateExecutableBinary(partPath)) {
        try { fs.unlinkSync(partPath); } catch {}
        return { success: false, error: 'קובץ ההתקנה שהורד אינו קובץ הפעלה תקין או שנפגם במהלך ההורדה' };
      }

      // Atomic rename from .part to final installer path
      if (fs.existsSync(targetPath)) {
        try { fs.unlinkSync(targetPath); } catch {}
      }
      fs.renameSync(partPath, targetPath);

      const stat = fs.statSync(targetPath);
      logger.info('[UPDATER] Direct download completed successfully', {
        targetPath,
        size: stat.size,
        sha256
      });

      return {
        success: true,
        filePath: targetPath,
        fileSize: stat.size,
        sha256
      };
    } catch (err: any) {
      logger.error('[UPDATER] Direct download failed:', { error: err?.message });
      const diagnosis = diagnoseFilterSslError(err);
      if (diagnosis.isFilterError) {
        return {
          success: false,
          error: `${diagnosis.errorMessageHebrew} ${diagnosis.recommendedActionHebrew}`
        };
      }
      return { success: false, error: `שגיאה בהורדת העדכון: ${err?.message || String(err)}` };
    }
  }

  /**
   * Streams remote file following redirects, computes SHA-256 on the fly,
   * and tracks moving-average transfer speed & ETA.
   */
  private downloadFileStreaming(
    sourceUrl: string,
    destinationPath: string,
    onProgress?: (p: UpdateDownloadProgress) => void,
    redirectCount = 0
  ): Promise<string> {
    if (redirectCount > 5) {
      return Promise.reject(new Error('יותר מדי הפניות (redirects) בהורדת הקובץ'));
    }

    return new Promise((resolve, reject) => {
      const parsed = new URL(sourceUrl);
      const isHttps = parsed.protocol === 'https:';
      const transport: any = isHttps ? https : http;

      const req = transport.get(
        sourceUrl,
        {
          headers: {
            'User-Agent': `TypesetOK-Desktop/${this.getCurrentVersion()}`,
            Accept: '*/*'
          },
          rejectUnauthorized: false
        },
        (res: IncomingMessage) => {
          if (res.statusCode && [301, 302, 303, 307, 308].includes(res.statusCode)) {
            const redirectLocation = res.headers.location;
            if (!redirectLocation) {
              reject(new Error(`שרת העדכונים הפנה לכתובת חסרה (HTTP ${res.statusCode})`));
              return;
            }
            const nextUrl = new URL(redirectLocation, sourceUrl).toString();
            this.downloadFileStreaming(nextUrl, destinationPath, onProgress, redirectCount + 1)
              .then(resolve)
              .catch(reject);
            return;
          }

          if (res.statusCode && (res.statusCode < 200 || res.statusCode >= 300)) {
            reject(new Error(`שרת ההורדות השיב בשגיאה: HTTP ${res.statusCode}`));
            return;
          }

          const totalBytes = parseInt(res.headers['content-length'] || '0', 10);
          let transferredBytes = 0;
          let lastSampleTime = Date.now();
          let lastTransferredBytes = 0;
          let currentSpeed = 0;

          const hash = crypto.createHash('sha256');
          const fileStream = fs.createWriteStream(destinationPath);

          res.on('data', (chunk: Buffer) => {
            transferredBytes += chunk.length;
            hash.update(chunk);

            const now = Date.now();
            const elapsed = (now - lastSampleTime) / 1000;
            if (elapsed >= 0.5) {
              const deltaBytes = transferredBytes - lastTransferredBytes;
              const instantSpeed = deltaBytes / elapsed;
              currentSpeed = currentSpeed === 0 ? instantSpeed : Math.round(currentSpeed * 0.7 + instantSpeed * 0.3);
              lastSampleTime = now;
              lastTransferredBytes = transferredBytes;
            }

            if (onProgress && totalBytes > 0) {
              const percent = Math.min(100, Math.round((transferredBytes / totalBytes) * 100));
              const remainingBytes = Math.max(0, totalBytes - transferredBytes);
              const etaSeconds = currentSpeed > 0 ? Math.round(remainingBytes / currentSpeed) : 0;
              onProgress({
                percent,
                transferredBytes,
                totalBytes,
                speedBytesPerSec: currentSpeed,
                etaSeconds
              });
            }
          });

          res.pipe(fileStream);

          fileStream.on('finish', () => {
            fileStream.close(() => resolve(hash.digest('hex')));
          });

          fileStream.on('error', (err: any) => {
            try { fs.unlinkSync(destinationPath); } catch {}
            reject(err);
          });

          res.on('error', (err: any) => {
            try { fs.unlinkSync(destinationPath); } catch {}
            reject(err);
          });
        }
      );

      req.on('error', (err: any) => {
        try { fs.unlinkSync(destinationPath); } catch {}
        reject(err);
      });

      req.setTimeout(60000, () => {
        req.destroy();
        try { fs.unlinkSync(destinationPath); } catch {}
        reject(new Error('פסק זמן בהורדת קובץ העדכון (60 שניות)'));
      });
    });
  }

  /**
   * Executes an installer in detached background mode and cleanly exits the app
   * so executables and resources are unlocked for upgrade.
   */
  public applyInstaller(filePath: string, silent = false): ApplyUpdateResult {
    if (!fs.existsSync(filePath)) {
      return { success: false, error: 'קובץ ההתקנה לא נמצא במערכת' };
    }

    if (!this.validateExecutableBinary(filePath)) {
      return { success: false, error: 'קובץ ההתקנה אינו תקין או פגום' };
    }

    try {
      logger.info('[UPDATER] Launching installer to apply update...', { filePath, silent });
      const isWin = process.platform === 'win32';
      let proc;

      if (isWin) {
        const args = silent ? ['/SILENT', '/NORESTART'] : [];
        proc = spawn(filePath, args, {
          detached: true,
          stdio: 'ignore',
          windowsHide: false
        });
      } else {
        // macOS or Linux AppImage
        try { fs.chmodSync(filePath, 0o755); } catch {}
        proc = spawn(filePath, [], {
          detached: true,
          stdio: 'ignore'
        });
      }

      proc.unref();

      setTimeout(() => {
        if (app?.quit) app.quit();
      }, 700);

      return { success: true };
    } catch (err: any) {
      logger.error('[UPDATER] Failed to launch installer:', { error: err?.message });
      return { success: false, error: `שגיאה בהפעלת קובץ ההתקנה: ${err?.message || String(err)}` };
    }
  }

  /**
   * Inspects a local file from disk/USB, extracts version, validates executable headers,
   * and generates descriptive status comparing against the current application version.
   */
  public inspectOfflineFile(filePath: string): OfflineUpdateCandidate | null {
    if (!fs.existsSync(filePath)) return null;
    try {
      const stat = fs.statSync(filePath);
      if (stat.size < 100 * 1024) return null;

      const fileName = path.basename(filePath);
      const lower = fileName.toLowerCase();

      // Check file extension: .exe, .appimage, or setup binary
      const isCandidateExt = lower.endsWith('.exe') || lower.endsWith('.appimage') || lower.endsWith('.dmg');
      const isRelated = lower.includes('typesetok') || lower.includes('tok') || lower.includes('setup');
      if (!isCandidateExt || !isRelated) return null;

      if (!this.validateExecutableBinary(filePath)) return null;

      const baseName = fileName
        .replace(/\.(exe|appimage|dmg|zip)$/i, '')
        .replace(/[-_](setup|installer|x64|ia32|arm64|win|mac|linux)/gi, '');
      const match = baseName.match(/v?(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)/i);
      const version = match ? match[1] : undefined;

      const curVer = this.getCurrentVersion();
      const comp = version ? this.compareVersions(version, curVer) : -1;
      const isNewer = comp > 0;
      const isCurrent = comp === 0;

      let statusDescriptionHebrew = 'קובץ התקנה תקין';
      if (version) {
        if (isNewer) statusDescriptionHebrew = `גרסה חדשה ${version} זמינה להתקנה (הנוכחית: ${curVer})`;
        else if (isCurrent) statusDescriptionHebrew = `גרסה ${version} זהה לגרסה המותקנת כעת`;
        else statusDescriptionHebrew = `גרסה ישנה ${version} (הנוכחית חדשה יותר: ${curVer})`;
      }

      return {
        filePath,
        fileName,
        fileSize: stat.size,
        version,
        isNewer,
        isCurrent,
        isRecommended: isNewer,
        driveLetter: filePath.charAt(1) === ':' ? filePath.charAt(0).toUpperCase() : undefined,
        modifiedAt: stat.mtime ? stat.mtime.toLocaleDateString('he-IL') : undefined,
        statusDescriptionHebrew
      };
    } catch {
      return null;
    }
  }

  /**
   * Deep-scans removable drives and external storage devices for TypesetOK installers.
   * Scans root directories and 1-level deep common installation/download folders.
   */
  public scanUsbDrives(): OfflineUpdateCandidate[] {
    const candidates: OfflineUpdateCandidate[] = [];
    const seenPaths = new Set<string>();

    const searchRoots: string[] = [];

    if (process.platform === 'win32') {
      const letters = ['D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'U', 'V', 'W', 'X', 'Y', 'Z'];
      for (const l of letters) {
        const root = `${l}:\\`;
        if (fs.existsSync(root)) searchRoots.push(root);
      }
    } else {
      // Unix / Linux / macOS mount points
      const unixMounts = ['/media', '/run/media', '/mnt', '/Volumes'];
      for (const m of unixMounts) {
        if (fs.existsSync(m)) {
          try {
            const sub = fs.readdirSync(m);
            for (const s of sub) {
              const full = path.join(m, s);
              if (fs.existsSync(full)) searchRoots.push(full);
            }
          } catch {}
        }
      }
    }

    const subfolderKeywords = [
      'typesetok',
      'תוכנות',
      'הורדות',
      'עדכונים',
      'עימוד',
      'ספרים',
      'ספרי',
      'קודש',
      'installers',
      'updates',
      'software'
    ];

    for (const root of searchRoots) {
      try {
        const entries = fs.readdirSync(root);
        for (const entry of entries) {
          const fullPath = path.join(root, entry);
          let stat;
          try { stat = fs.statSync(fullPath); } catch { continue; }

          if (stat.isFile()) {
            if (!seenPaths.has(fullPath)) {
              seenPaths.add(fullPath);
              const cand = this.inspectOfflineFile(fullPath);
              if (cand) candidates.push(cand);
            }
          } else if (stat.isDirectory()) {
            const lowerDir = entry.toLowerCase();
            const shouldDeepScan = subfolderKeywords.some((kw) => lowerDir.includes(kw));
            if (shouldDeepScan) {
              try {
                const subEntries = fs.readdirSync(fullPath);
                for (const sub of subEntries) {
                  const subFullPath = path.join(fullPath, sub);
                  if (!seenPaths.has(subFullPath)) {
                    seenPaths.add(subFullPath);
                    const cand = this.inspectOfflineFile(subFullPath);
                    if (cand) candidates.push(cand);
                  }
                }
              } catch {}
            }
          }
        }
      } catch {
        // Skip unreadable or locked drives
      }
    }

    // Sort candidates: newest version first
    candidates.sort((a, b) => {
      if (a.version && b.version) {
        return this.compareVersions(b.version, a.version);
      }
      return 0;
    });

    // Mark the highest newer candidate as the recommended choice
    let markedRecommended = false;
    for (const cand of candidates) {
      if (cand.isNewer && !markedRecommended) {
        cand.isRecommended = true;
        markedRecommended = true;
      } else {
        cand.isRecommended = false;
      }
    }

    return candidates;
  }
}

export const updater = new TokUpdater();

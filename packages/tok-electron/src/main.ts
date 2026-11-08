import { app, BrowserWindow, ipcMain, shell, dialog, IpcMainInvokeEvent } from 'electron';
import * as path from 'path';
import * as fs from 'fs';
import * as zlib from 'zlib';
import { spawn, execFileSync } from 'child_process';
import { fileURLToPath } from 'url';
import { buildApplicationMenu } from './menu';
import { logger } from './logger';
import { updater, getAppVersion, isSafeExternalUrl } from './updater';
import { pluginManager } from './plugin-manager';
import { parseInstallLanguage, parseInstallProjectsDir, InstallLanguage } from './install-language';

let mainWindow: BrowserWindow | null = null;
let splashWindow: BrowserWindow | null = null;
let startupFilePath: string | null = null;

const UI_INDEX_PATH = path.join(__dirname, '../../tok-ui/dist/index.html');
const APP_REPO_URL = 'https://github.com/TypesetOK/typesetok';

// ---------------------------------------------------------------------------
// Startup tracing (opt-in). TOK_STARTUP_TRACE=1 prints epoch-ms milestones to
// stdout; TOK_STARTUP_TRACE=exit additionally collects renderer timings and
// quits once the main window is visible. Used by scripts/measure-startup.mjs.
// ---------------------------------------------------------------------------
const STARTUP_TRACE = process.env.TOK_STARTUP_TRACE || '';
function trace(label: string, at: number = Date.now()): void {
  if (STARTUP_TRACE) process.stdout.write(`[TOK-TRACE] ${label} ${Math.round(at)}\n`);
}
trace('main-module-loaded');

function collectRendererTimingsAndMaybeQuit(win: BrowserWindow): void {
  if (!STARTUP_TRACE) return;
  const collect = () => {
    if (win.isDestroyed()) return;
    win.webContents
      .executeJavaScript(
        `new Promise((resolve) => requestAnimationFrame(() => setTimeout(() => {
           const o = performance.timeOrigin;
           const at = (n) => { const e = performance.getEntriesByName(n)[0]; return e ? o + e.startTime : null; };
           const nav = performance.getEntriesByType('navigation')[0] || {};
           resolve({ navStart: o, htmlLoaded: o + nav.responseEnd, scriptStart: at('tok-script-start'),
             dclStart: o + nav.domContentLoadedEventStart, uiBuilt: at('tok-ui-built'),
             domContentLoaded: o + nav.domContentLoadedEventEnd, fcp: at('first-contentful-paint') });
         }, 0)))`
      )
      .then((r: Record<string, number | null>) => {
        for (const [k, v] of Object.entries(r)) if (v) trace(`renderer-${k}`, v);
        trace('trace-done');
        if (STARTUP_TRACE === 'exit') setTimeout(() => app.quit(), 50);
      })
      .catch(() => {
        if (STARTUP_TRACE === 'exit') app.quit();
      });
  };
  if (win.isVisible()) collect();
  else win.once('show', collect);
}

// ---------------------------------------------------------------------------
// tok-cli (Rust engine) integration. The binary is only spawned on demand.
// ---------------------------------------------------------------------------
let cachedCliPath: string | null = null;

function getTokCliPath(): string {
  if (cachedCliPath && fs.existsSync(cachedCliPath)) return cachedCliPath;
  const isWin = process.platform === 'win32';
  const exe = isWin ? 'tok-cli.exe' : 'tok-cli';
  const bundled = path.join(process.resourcesPath, 'bin', exe);
  const repoRoot = path.join(__dirname, '../../..'); // packages/tok-electron/dist -> repo root
  const candidates = app.isPackaged
    ? [process.env.TOK_CLI_PATH, bundled]
    : [
        process.env.TOK_CLI_PATH,
        ...(process.env.CARGO_TARGET_DIR ? [
          path.join(process.env.CARGO_TARGET_DIR, 'release', exe),
          path.join(process.env.CARGO_TARGET_DIR, 'debug', exe),
        ] : []),
        ...(process.env.LOCALAPPDATA ? [
          path.join(process.env.LOCALAPPDATA, 'tok_target', 'release', exe),
          path.join(process.env.LOCALAPPDATA, 'tok_target', 'debug', exe),
        ] : []),
        path.join(repoRoot, 'target/release', exe),
        path.join(repoRoot, 'target/debug', exe),
        bundled,
      ];
  const found = candidates.find((p): p is string => !!p && fs.existsSync(p));
  cachedCliPath = found || null;
  return found || bundled;
}

interface CliResult {
  code: number | null;
  stdout: string;
  stderr: string;
}

/** Runs tok-cli without a shell. Rejects (instead of crashing the main process) when it cannot start. */
function runCli(args: string[]): Promise<CliResult> {
  const cli = getTokCliPath();
  return new Promise((resolve, reject) => {
    let proc: ReturnType<typeof spawn>;
    try {
      proc = spawn(cli, args, { windowsHide: true });
    } catch (err: any) {
      reject(new Error(`tok-cli could not be started (${cli}): ${err?.message}`));
      return;
    }
    let stdout = '';
    let stderr = '';
    proc.stdout?.on('data', (d) => (stdout += d.toString()));
    proc.stderr?.on('data', (d) => (stderr += d.toString()));
    proc.on('error', (err) => reject(new Error(`tok-cli could not be started (${cli}): ${err.message}`)));
    proc.on('close', (code) => resolve({ code, stdout, stderr }));
  });
}

function requireString(value: unknown, name: string): string {
  if (typeof value !== 'string' || !value) throw new Error(`Invalid ${name}`);
  return value;
}

/** Extracts and decompresses XML parts from a DOCX zip buffer */
function extractDocxPartsFromBuffer(buffer: Buffer): {
  documentXml: string | null;
  footnotesXml?: string;
  endnotesXml?: string;
  numberingXml?: string;
  stylesXml?: string;
} {
  if (buffer.length < 30) return { documentXml: null };
  if (buffer[0] !== 0x50 || buffer[1] !== 0x4B || buffer[2] !== 0x03 || buffer[3] !== 0x04) {
    return { documentXml: null };
  }
  const wanted = new Set([
    'word/document.xml',
    'word/footnotes.xml',
    'word/endnotes.xml',
    'word/numbering.xml',
    'word/styles.xml'
  ]);
  const extracted: Record<string, string> = {};

  // 1. Check Central Directory first
  let eocdOffset = -1;
  const minEocd = Math.max(0, buffer.length - 65557);
  for (let i = buffer.length - 22; i >= minEocd; i--) {
    if (buffer.readUInt32LE(i) === 0x06054b50) {
      eocdOffset = i;
      break;
    }
  }
  if (eocdOffset !== -1) {
    const cdCount = buffer.readUInt16LE(eocdOffset + 10);
    let cdOffset = buffer.readUInt32LE(eocdOffset + 16);
    for (let idx = 0; idx < cdCount && cdOffset + 46 <= buffer.length; idx++) {
      if (buffer.readUInt32LE(cdOffset) !== 0x02014b50) break;
      const compressionMethod = buffer.readUInt16LE(cdOffset + 10);
      const compressedSize = buffer.readUInt32LE(cdOffset + 20);
      const fileNameLen = buffer.readUInt16LE(cdOffset + 28);
      const extraFieldLen = buffer.readUInt16LE(cdOffset + 30);
      const commentLen = buffer.readUInt16LE(cdOffset + 32);
      const localHeaderOffset = buffer.readUInt32LE(cdOffset + 42);
      const fileName = buffer.toString('utf-8', cdOffset + 46, cdOffset + 46 + fileNameLen);

      if (wanted.has(fileName) && localHeaderOffset + 30 <= buffer.length) {
        const localNameLen = buffer.readUInt16LE(localHeaderOffset + 26);
        const localExtraLen = buffer.readUInt16LE(localHeaderOffset + 28);
        const dataStart = localHeaderOffset + 30 + localNameLen + localExtraLen;
        if (dataStart + compressedSize <= buffer.length) {
          const entryBytes = buffer.subarray(dataStart, dataStart + compressedSize);
          if (compressionMethod === 0) {
            extracted[fileName] = entryBytes.toString('utf-8');
          } else if (compressionMethod === 8) {
            extracted[fileName] = zlib.inflateRawSync(entryBytes).toString('utf-8');
          }
        }
      }
      cdOffset += 46 + fileNameLen + extraFieldLen + commentLen;
    }
  }

  // 2. Fallback scan of Local File Headers
  let offset = 0;
  while (offset + 30 <= buffer.length) {
    const sig = buffer.readUInt32LE(offset);
    if (sig !== 0x04034b50) break;
    const compressionMethod = buffer.readUInt16LE(offset + 8);
    const compressedSize = buffer.readUInt32LE(offset + 18);
    const fileNameLen = buffer.readUInt16LE(offset + 26);
    const extraFieldLen = buffer.readUInt16LE(offset + 28);
    const fileName = buffer.toString('utf-8', offset + 30, offset + 30 + fileNameLen);
    const dataStart = offset + 30 + fileNameLen + extraFieldLen;
    if (wanted.has(fileName) && !extracted[fileName] && compressedSize > 0 && dataStart + compressedSize <= buffer.length) {
      const entryBytes = buffer.subarray(dataStart, dataStart + compressedSize);
      if (compressionMethod === 0) {
        extracted[fileName] = entryBytes.toString('utf-8');
      } else if (compressionMethod === 8) {
        extracted[fileName] = zlib.inflateRawSync(entryBytes).toString('utf-8');
      }
    }
    if (compressedSize === 0 && (buffer.readUInt16LE(offset + 6) & 0x08) !== 0) break;
    offset = dataStart + compressedSize;
  }

  return {
    documentXml: extracted['word/document.xml'] || null,
    footnotesXml: extracted['word/footnotes.xml'],
    endnotesXml: extracted['word/endnotes.xml'],
    numberingXml: extracted['word/numbering.xml'],
    stylesXml: extracted['word/styles.xml']
  };
}

/** Extracts file path from launch arguments or second-instance commandLine */
function extractFilePathFromArgs(args: string[]): string | null {
  if (!Array.isArray(args) || args.length === 0) return null;
  const startIndex = app.isPackaged ? 1 : 1;
  for (let i = startIndex; i < args.length; i++) {
    const arg = args[i];
    if (!arg || arg.startsWith('--') || arg.startsWith('-')) continue;
    // Skip electron/node binary paths
    if (arg.endsWith('electron.exe') || arg.endsWith('electron') || arg.endsWith('main.js') || arg.endsWith('main.ts')) {
      continue;
    }
    try {
      if (fs.existsSync(arg) && fs.statSync(arg).isFile()) {
        return path.resolve(arg);
      }
    } catch {}
    if (/\.(tok|docx|md|txt|json)$/i.test(arg)) {
      try {
        const resolved = path.resolve(arg);
        if (fs.existsSync(resolved)) return resolved;
      } catch {}
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Security helpers
// ---------------------------------------------------------------------------
const normalizeFsPath = (p: string) => {
  const n = path.normalize(p);
  return process.platform === 'win32' ? n.toLowerCase() : n;
};
const UI_INDEX_KEY = normalizeFsPath(UI_INDEX_PATH);

/** True for the app's own UI page (any hash/query), false for anything else. */
function isAppUrl(url: string | undefined): boolean {
  if (!url || !url.startsWith('file:')) return false;
  try {
    return normalizeFsPath(fileURLToPath(url)) === UI_INDEX_KEY;
  } catch {
    return false;
  }
}

/** ipcMain.handle wrapper that only answers the app's own UI page and logs failures. */
function handle(channel: string, fn: (event: IpcMainInvokeEvent, ...args: any[]) => unknown): void {
  ipcMain.handle(channel, async (event, ...args) => {
    if (!isAppUrl(event.senderFrame?.url)) {
      logger.warn(`[IPC] Rejected ${channel} from untrusted frame`, { url: event.senderFrame?.url });
      throw new Error('Untrusted IPC sender');
    }
    try {
      return await fn(event, ...args);
    } catch (err: any) {
      logger.error(`[IPC] ${channel} failed: ${err?.message}`);
      throw err;
    }
  });
}

function openExternalSafely(url: unknown): void {
  if (isSafeExternalUrl(url)) {
    shell.openExternal(url).catch((err) => logger.warn('[MAIN] openExternal failed', { error: err?.message }));
  }
}

/**
 * Creates an instantaneous, dark-themed splash screen with a subtle pulsating logo.
 * Eliminates stark white contrast and provides instant launch feedback (<100ms).
 */
function createSplashWindow(): void {
  splashWindow = new BrowserWindow({
    width: 480,
    height: 340,
    frame: false,
    resizable: false,
    show: true,
    center: true,
    backgroundColor: '#0B132B',
    alwaysOnTop: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
    },
  });

  const splashHtml = `
  <!DOCTYPE html>
  <html dir="rtl" lang="he">
  <head>
    <meta charset="utf-8">
    <title>TypesetOK Starting...</title>
    <style>
      * { box-sizing: border-box; margin: 0; padding: 0; }
      body {
        background-color: #0B132B;
        color: #F8FAFC;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100vh;
        user-select: none;
        overflow: hidden;
        border: 1px solid #1E293B;
      }
      .brand-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
      }
      .logo-icon {
        width: 80px;
        height: 80px;
        margin-bottom: 18px;
        animation: tokGlowPulse 2s ease-in-out infinite;
      }
      @keyframes tokGlowPulse {
        0%, 100% {
          transform: scale(0.96);
          opacity: 0.85;
          filter: drop-shadow(0 0 14px rgba(59, 130, 246, 0.45));
        }
        50% {
          transform: scale(1.05);
          opacity: 1;
          filter: drop-shadow(0 0 28px rgba(59, 130, 246, 0.9));
        }
      }
      .title {
        font-size: 26px;
        font-weight: 800;
        letter-spacing: -0.5px;
        color: #60A5FA;
        margin-bottom: 6px;
      }
      .subtitle {
        font-size: 13px;
        color: #94A3B8;
        font-weight: 500;
        margin-bottom: 22px;
      }
      .loader-bar-wrap {
        width: 200px;
        height: 4px;
        background: #1E293B;
        border-radius: 4px;
        overflow: hidden;
        position: relative;
      }
      .loader-bar {
        position: absolute;
        top: 0;
        bottom: 0;
        background: linear-gradient(90deg, #2563EB, #60A5FA);
        border-radius: 4px;
        animation: loadSlide 1.5s infinite ease-in-out;
      }
      @keyframes loadSlide {
        0% { left: -40%; width: 40%; }
        50% { left: 30%; width: 60%; }
        100% { left: 100%; width: 40%; }
      }
      .status-text {
        font-size: 11px;
        color: #64748B;
        margin-top: 10px;
      }
    </style>
  </head>
  <body>
    <div class="brand-container">
      <svg class="logo-icon" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
        <rect width="64" height="64" rx="14" fill="#1E293B"/>
        <path d="M16 18C16 15.7909 17.7909 14 20 14H44C46.2091 14 48 15.7909 48 18V46C48 48.2091 46.2091 50 44 50H20C17.7909 50 16 48.2091 16 46V18Z" stroke="#3B82F6" stroke-width="2.5" fill="#0F172A"/>
        <line x1="22" y1="24" x2="42" y2="24" stroke="#60A5FA" stroke-width="2.5" stroke-linecap="round"/>
        <line x1="22" y1="32" x2="42" y2="32" stroke="#93C5FD" stroke-width="2" stroke-linecap="round"/>
        <line x1="22" y1="40" x2="36" y2="40" stroke="#93C5FD" stroke-width="2" stroke-linecap="round"/>
        <circle cx="42" cy="40" r="2.5" fill="#3B82F6"/>
      </svg>
      <div class="title">TypesetOK</div>
      <div class="subtitle">תוכנת עימוד מקצועית לטקסט עברי</div>
      <div class="loader-bar-wrap">
        <div class="loader-bar"></div>
      </div>
      <div class="status-text">טוען ליבת עימוד וסביבת עבודה...</div>
    </div>
  </body>
  </html>
  `;

  splashWindow.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(splashHtml)}`);
  if (STARTUP_TRACE) splashWindow.webContents.once('did-finish-load', () => trace('splash-loaded'));
}

function getAppIconPath(): string | undefined {
  const candidates = [
    path.join(__dirname, '../../../assets/icon.ico'),
    path.join(__dirname, '../../../assets/icon.png'),
    path.join(process.resourcesPath, 'app/assets/icon.ico'),
    path.join(process.resourcesPath, 'app/assets/icon.png'),
    path.join(process.resourcesPath, 'icon.ico'),
    path.join(process.resourcesPath, 'icon.png'),
  ];
  return candidates.find((p) => fs.existsSync(p));
}

function createWindow(): void {
  const appIcon = getAppIconPath();
  const win = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 1024,
    minHeight: 700,
    title: 'TypesetOK (TOK) - תוכנת עימוד מקצועית',
    backgroundColor: '#0B132B',
    icon: appIcon,
    frame: false,
    show: false, // Hidden until ready, preventing any white flicker
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      webviewTag: false,
    },
  });
  mainWindow = win;

  // Load the UI first so the renderer starts working while the menu is built.
  win.loadFile(UI_INDEX_PATH).catch((err) => {
    logger.warn(`Could not load ${UI_INDEX_PATH}: ${err.message}`);
  });

  // Remove the clunky greyish Windows native menu bar (accelerators stay active)
  win.setMenu(buildApplicationMenu(win));
  win.setMenuBarVisibility(false);
  win.setAutoHideMenuBar(true);

  // Never navigate the app window away from its UI or open new Electron windows;
  // http(s) links go to the system browser instead.
  win.webContents.setWindowOpenHandler(({ url }) => {
    openExternalSafely(url);
    return { action: 'deny' };
  });
  win.webContents.on('will-navigate', (event, url) => {
    if (!isAppUrl(url)) {
      event.preventDefault();
      openExternalSafely(url);
    }
  });

  // Once the UI has painted, swap the splash for the main window right away.
  // The main window is shown *before* the splash is destroyed so there is never
  // a frame with no TypesetOK window on screen.
  win.once('ready-to-show', () => {
    trace('main-ready-to-show');
    win.maximize();
    win.show();
    win.focus();
    trace('main-shown');
    closeSplash();
    logger.info('[MAIN] Main window ready to show (maximized).');
    scheduleDeferredStartupWork();
    if (startupFilePath) {
      win.webContents.send('tok:event', { action: 'open-file-at-path', data: startupFilePath });
    }
  });

  // If the UI cannot load at all, don't leave the user staring at the splash.
  win.webContents.on('console-message', (_event, level, message, line, sourceId) => {
    logger.info(`[RENDERER-CONSOLE] [lvl=${level}] ${message} (${sourceId}:${line})`);
  });

  win.webContents.on('did-fail-load', (_e, code, desc, url, isMainFrame) => {
    if (!isMainFrame || code === -3 /* ERR_ABORTED: superseded navigation */) return;
    logger.error(`[MAIN] UI failed to load (${code} ${desc}): ${url}`);
    closeSplash();
    if (!win.isDestroyed() && !win.isVisible()) win.show();
  });
  win.webContents.on('render-process-gone', (_e, details) => {
    logger.error('[MAIN] Renderer process gone', details);
  });

  collectRendererTimingsAndMaybeQuit(win);

  let isForceClosing = false;
  let isClosingPromptActive = false;

  win.on('close', async (e) => {
    if (isForceClosing) return;
    e.preventDefault();

    if (isClosingPromptActive) {
      logger.info('[MAIN] Close request ignored: close prompt already active');
      return;
    }

    isClosingPromptActive = true;

    try {
      if (win.isDestroyed() || win.webContents.isDestroyed()) {
        isForceClosing = true;
        win.close();
        return;
      }

      // Query renderer for dirty / unsaved changes with timeout and valid JavaScript
      const dirtyPromise = win.webContents.executeJavaScript(
        'Boolean(window.appInstance?.getDocumentState?.()?.isDirty || window.__tokIsDirty)'
      );
      const timeoutPromise = new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 1500));
      const isDirty = await Promise.race([dirtyPromise, timeoutPromise]).catch(() => false);

      if (!isDirty) {
        isForceClosing = true;
        win.destroy();
        return;
      }

      const choice = await dialog.showMessageBox(win, {
        type: 'warning',
        buttons: ['שמור וצא', 'צא ללא שמירה', 'ביטול'],
        defaultId: 0,
        cancelId: 2,
        noLink: true,
        title: 'TypesetOK',
        message: 'במסמך קיימים שינויים שלא נשמרו.',
        detail: 'האם ברצונך לשמור את השינויים לפני סגירת התוכנה?'
      });

      if (choice.response === 0) {
        // Save before exit
        const savePromise = win.webContents.executeJavaScript(
          'Promise.resolve(window.appInstance?.handleSystemAction?.("save-document")).then((res) => res !== false).catch(() => false)'
        );
        const saveTimeout = new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 30000));
        const saveResult = await Promise.race([savePromise, saveTimeout]).catch(() => false);

        if (saveResult === false) {
          logger.info('[MAIN] Save before exit was cancelled or failed; keeping window open');
          isClosingPromptActive = false;
          return;
        }

        // Allow save I/O to flush, then destroy window safely
        setTimeout(() => {
          isForceClosing = true;
          if (!win.isDestroyed()) win.destroy();
        }, 300);
      } else if (choice.response === 1) {
        // Discard without saving - unconditionally destroy window
        isForceClosing = true;
        win.destroy();
      } else {
        // Cancel: keep window open
        logger.info('[MAIN] Window close cancelled by user');
        isClosingPromptActive = false;
      }
    } catch (err: any) {
      logger.warn('[MAIN] Exception in window close guard; closing safely', { error: err?.message });
      isForceClosing = true;
      if (!win.isDestroyed()) win.destroy();
    }
  });

  win.on('closed', () => {
    if (mainWindow === win) mainWindow = null;
  });
}

function closeSplash(): void {
  if (splashWindow && !splashWindow.isDestroyed()) {
    splashWindow.destroy();
  }
  splashWindow = null;
}

// ---------------------------------------------------------------------------
// IPC Handlers
// ---------------------------------------------------------------------------
handle('tok:send-command', async (_, cmd: any) => {
  const action = typeof cmd === 'string' ? cmd : cmd?.action;
  logger.info(`[IPC] Received send-command: ${action}`);

  if (action === 'ping') {
    const cli = getTokCliPath();
    return { status: 'ok', version: getAppVersion(), cliPath: cli, cliExists: fs.existsSync(cli) };
  }

  if (action === 'benchmark-typeset') {
    const { code, stdout, stderr } = await runCli(['benchmark-typeset', '--pages', '100']);
    if (code === 0) return { ok: true, output: stdout };
    throw new Error(`Benchmark failed: ${stderr}`);
  }

  if (action === 'verify-determinism') {
    const { code, stdout, stderr } = await runCli(['verify-determinism']);
    if (code === 0) return { ok: true, output: stdout };
    throw new Error(`Determinism verification failed: ${stderr}`);
  }

  if (action === 'configure-heading-numbering' || cmd?.type === 'ConfigureHeadingNumbering') {
    const config = cmd?.config || cmd?.payload?.config;
    return { ok: true, action: 'configure-heading-numbering', config };
  }

  if (action === 'reorder-heading' || cmd?.type === 'ReorderHeadingNode') {
    const payload = cmd?.payload || cmd;
    return { ok: true, action: 'reorder-heading', payload };
  }

  if (action === 'insert-heading' || cmd?.type === 'InsertHeadingNode') {
    const payload = cmd?.payload || cmd;
    return { ok: true, action: 'insert-heading', payload };
  }

  if (action === 'delete-heading' || cmd?.type === 'DeleteHeadingNode') {
    const payload = cmd?.payload || cmd;
    return { ok: true, action: 'delete-heading', payload };
  }

  if (action === 'recalculate-heading-numbers' || cmd?.type === 'RecalculateHeadingNumbers') {
    return { ok: true, action: 'recalculate-heading-numbers', payload: cmd?.payload || cmd };
  }

  return { status: 'unhandled_command', cmd };
});

async function renderWithCli(kind: 'pdf' | 'html', payload: any): Promise<string> {
  const rawOutputPath = requireString(payload?.outputPath, 'outputPath');
  const outputPath = path.isAbsolute(rawOutputPath) ? rawOutputPath : path.resolve(rawOutputPath);

  let inputPath = typeof payload?.inputPath === 'string' && payload.inputPath ? payload.inputPath : null;
  let tempDocFile: string | null = null;

  try {
    if (payload?.document) {
      const tempDir = app.getPath('temp');
      const uniqueSuffix = `${Date.now()}_${process.pid}_${Math.random().toString(36).slice(2, 9)}`;
      tempDocFile = path.join(tempDir, `tok_doc_${uniqueSuffix}.json`);
      const content = typeof payload.document === 'string'
        ? payload.document
        : JSON.stringify(payload.document, null, 2);
      await fs.promises.writeFile(tempDocFile, content, 'utf-8');
      inputPath = tempDocFile;
    }

    if (!inputPath) {
      throw new Error('Either inputPath or document must be provided for rendering');
    }

    const cliArgs: string[] = [`render-${kind}`, inputPath, outputPath];
    if (kind === 'pdf') {
      const opts = payload?.options || payload;
      if (opts?.format) {
        cliArgs.push('--standard', String(opts.format));
      }
      if (opts?.colorProfile) {
        cliArgs.push('--profile', String(opts.colorProfile));
      }
      if (opts?.bleed === false) {
        cliArgs.push('--no-bleed');
      }
      if (opts?.cropMarks === false) {
        cliArgs.push('--no-crop-marks');
      }
      if (opts?.registrationMarks === true) {
        cliArgs.push('--registration-marks');
      }
    }

    const { code, stdout, stderr } = await runCli(cliArgs);
    if (code !== 0) {
      throw new Error(`tok-cli failed with code ${code}: ${stderr || stdout}`);
    }

    if (!fs.existsSync(outputPath)) {
      throw new Error(`Export failed: output file was not created at ${outputPath}`);
    }

    if (kind === 'pdf') {
      logger.info(`[IPC] PDF rendering succeeded to: ${outputPath}`);
    }
    return stdout;
  } finally {
    if (tempDocFile) {
      try {
        if (fs.existsSync(tempDocFile)) {
          await fs.promises.unlink(tempDocFile);
        }
      } catch (err: any) {
        logger.warn(`[IPC] Failed to remove temp document file: ${tempDocFile}`, { error: err?.message });
      }
    }
  }
}

handle('tok:render-pdf', (_, payload) => renderWithCli('pdf', payload));
handle('tok:render-html', (_, payload) => renderWithCli('html', payload));

handle('tok:save-document', async (_, payload: { document?: unknown; filePath?: string }) => {
  const filePath = requireString(payload?.filePath, 'filePath');
  const targetPath = path.isAbsolute(filePath) ? filePath : path.resolve(filePath);

  if (!payload?.document) {
    throw new Error('Document content must be provided for saving');
  }

  const tempDir = app.getPath('temp');
  const uniqueSuffix = `${Date.now()}_${process.pid}_${Math.random().toString(36).slice(2, 9)}`;
  const tempDocFile = path.join(tempDir, `tok_save_${uniqueSuffix}.json`);

  try {
    const content = typeof payload.document === 'string'
      ? payload.document
      : JSON.stringify(payload.document, null, 2);
    await fs.promises.writeFile(tempDocFile, content, 'utf-8');

    const { code, stdout, stderr } = await runCli(['save-package', tempDocFile, targetPath]);
    if (code !== 0) {
      throw new Error(`tok-cli save-package failed with code ${code}: ${stderr || stdout}`);
    }

    if (!fs.existsSync(targetPath)) {
      throw new Error(`Save failed: .tok package was not created at ${targetPath}`);
    }

    logger.info(`[IPC] Document saved successfully to: ${targetPath}`);
    return { success: true, filePath: targetPath };
  } finally {
    try {
      if (fs.existsSync(tempDocFile)) {
        await fs.promises.unlink(tempDocFile);
      }
    } catch (err: any) {
      logger.warn(`[IPC] Failed to remove temp save file: ${tempDocFile}`, { error: err?.message });
    }
  }
});

handle('tok:open-document', async (_, filePath: string) => {
  const sourcePath = requireString(filePath, 'filePath');
  const resolvedPath = path.isAbsolute(sourcePath) ? sourcePath : path.resolve(sourcePath);

  if (!fs.existsSync(resolvedPath)) {
    throw new Error(`File does not exist: ${resolvedPath}`);
  }

  const tempDir = app.getPath('temp');
  const uniqueSuffix = `${Date.now()}_${process.pid}_${Math.random().toString(36).slice(2, 9)}`;
  const tempOutFile = path.join(tempDir, `tok_open_${uniqueSuffix}.json`);

  try {
    const { code, stdout, stderr } = await runCli(['open-package', resolvedPath, tempOutFile]);
    if (code !== 0) {
      throw new Error(`tok-cli open-package failed with code ${code}: ${stderr || stdout}`);
    }

    if (!fs.existsSync(tempOutFile)) {
      throw new Error(`Open failed: extracted JSON was not produced at ${tempOutFile}`);
    }

    const rawContent = await fs.promises.readFile(tempOutFile, 'utf-8');
    const docRoot = JSON.parse(rawContent);
    logger.info(`[IPC] Document opened successfully from: ${resolvedPath}`);
    return docRoot;
  } finally {
    try {
      if (fs.existsSync(tempOutFile)) {
        await fs.promises.unlink(tempOutFile);
      }
    } catch (err: any) {
      logger.warn(`[IPC] Failed to remove temp open file: ${tempOutFile}`, { error: err?.message });
    }
  }
});

handle('tok:typeset-document', async (_, payload: { document?: unknown; inputPath?: string }) => {
  let inputPath = typeof payload?.inputPath === 'string' && payload.inputPath ? payload.inputPath : null;
  let tempDocFile: string | null = null;
  const tempDir = app.getPath('temp');
  const uniqueSuffix = `${Date.now()}_${process.pid}_${Math.random().toString(36).slice(2, 9)}`;
  const tempOutFile = path.join(tempDir, `tok_typeset_out_${uniqueSuffix}.json`);

  try {
    if (payload?.document) {
      tempDocFile = path.join(tempDir, `tok_typeset_in_${uniqueSuffix}.json`);
      const content = typeof payload.document === 'string'
        ? payload.document
        : JSON.stringify(payload.document, null, 2);
      await fs.promises.writeFile(tempDocFile, content, 'utf-8');
      inputPath = tempDocFile;
    }

    if (!inputPath) {
      throw new Error('Either document or inputPath must be provided for typesetting');
    }

    const { code, stdout, stderr } = await runCli(['typeset-document', inputPath, tempOutFile]);
    if (code !== 0) {
      throw new Error(`tok-cli typeset-document failed with code ${code}: ${stderr || stdout}`);
    }

    if (!fs.existsSync(tempOutFile)) {
      throw new Error(`Typeset failed: output file was not created at ${tempOutFile}`);
    }

    const rawContent = await fs.promises.readFile(tempOutFile, 'utf-8');
    const pages = JSON.parse(rawContent);
    return { success: true, pages };
  } finally {
    if (tempDocFile) {
      try {
        if (fs.existsSync(tempDocFile)) {
          await fs.promises.unlink(tempDocFile);
        }
      } catch (err: any) {
        logger.warn(`[IPC] Failed to remove temp typeset in file: ${tempDocFile}`, { error: err?.message });
      }
    }
    try {
      if (fs.existsSync(tempOutFile)) {
        await fs.promises.unlink(tempOutFile);
      }
    } catch (err: any) {
      logger.warn(`[IPC] Failed to remove temp typeset out file: ${tempOutFile}`, { error: err?.message });
    }
  }
});

// Logger Handlers
// What the user picked in the Windows installer (language and projects folder), read once from the registry.
// The language is requested synchronously by the preload on first run, because the renderer needs it before it starts.
let installRegistryCache: { language: InstallLanguage | null; projectsDir: string | null } | undefined;
function readInstallRegistry(): { language: InstallLanguage | null; projectsDir: string | null } {
  if (installRegistryCache) return installRegistryCache;
  const info: { language: InstallLanguage | null; projectsDir: string | null } = { language: null, projectsDir: null };
  if (process.platform === 'win32') {
    for (const root of ['HKCU', 'HKLM']) {
      try {
        const out = execFileSync('reg', ['query', `${root}\\Software\\TypesetOK`], { encoding: 'utf8', timeout: 3000, windowsHide: true });
        info.language ||= parseInstallLanguage(out);
        info.projectsDir ||= parseInstallProjectsDir(out);
      } catch {
        // key missing (portable run, dev build): keep the defaults
      }
    }
  }
  installRegistryCache = info;
  return info;
}
ipcMain.on('tok:get-install-language', (event) => {
  event.returnValue = isAppUrl(event.senderFrame?.url) ? readInstallRegistry().language : null;
});

/** The installer's projects folder, falling back to ~/Documents/TypesetOK. Always ensures directory exists. */
function getProjectsDir(): string {
  const regDir = readInstallRegistry().projectsDir;
  if (regDir) {
    try {
      if (!fs.existsSync(regDir)) fs.mkdirSync(regDir, { recursive: true });
      if (fs.statSync(regDir).isDirectory()) return regDir;
    } catch {}
  }
  const defaultDir = path.join(app.getPath('documents'), 'TypesetOK');
  try {
    if (!fs.existsSync(defaultDir)) fs.mkdirSync(defaultDir, { recursive: true });
  } catch {}
  return defaultDir;
}

/** A bare file name (or nothing) opens in the projects folder; a full path the caller chose is kept. */
function withProjectsDir(defaultPath?: string): string {
  const dir = getProjectsDir();
  if (!defaultPath) return dir;
  return path.basename(defaultPath) === defaultPath ? path.join(dir, defaultPath) : defaultPath;
}

handle('tok:get-projects-dir', () => getProjectsDir());
handle('tok:rename-document-file', async (_, payload: { oldPath?: string; newPath?: string }) => {
  const oldPath = requireString(payload?.oldPath, 'oldPath');
  const newPath = requireString(payload?.newPath, 'newPath');
  const resolvedOld = path.isAbsolute(oldPath) ? oldPath : path.resolve(oldPath);
  const resolvedNew = path.isAbsolute(newPath) ? newPath : path.resolve(newPath);
  if (fs.existsSync(resolvedOld)) {
    const targetDir = path.dirname(resolvedNew);
    if (!fs.existsSync(targetDir)) fs.mkdirSync(targetDir, { recursive: true });
    await fs.promises.rename(resolvedOld, resolvedNew);
    logger.info(`[IPC] Renamed document file from ${resolvedOld} to ${resolvedNew}`);
    return { success: true, filePath: resolvedNew };
  }
  return { success: false, error: 'Source file does not exist' };
});

// Heading & Outline Numbering Handlers
handle('tok:configure-heading-numbering', async (_, config: any) => {
  logger.info('[IPC] Received configure-heading-numbering', { config });
  return { success: true, config };
});

handle('tok:reorder-heading', async (_, payload: { nodeId: string; newParentId?: string; targetIndex: number; outline?: any; config?: any }) => {
  logger.info('[IPC] Received reorder-heading', { nodeId: payload?.nodeId, targetIndex: payload?.targetIndex });
  return { success: true, payload };
});

handle('tok:insert-heading', async (_, payload: { parentId?: string; targetIndex: number; node: any; config?: any }) => {
  logger.info('[IPC] Received insert-heading', { targetIndex: payload?.targetIndex });
  return { success: true, payload };
});

handle('tok:delete-heading', async (_, payload: { nodeId: string; config?: any }) => {
  logger.info('[IPC] Received delete-heading', { nodeId: payload?.nodeId });
  return { success: true, payload };
});

handle('tok:recalculate-heading-numbers', async (_, payload?: { outline?: any; config?: any; document?: any }) => {
  logger.info('[IPC] Received recalculate-heading-numbers');
  return { success: true, payload };
});

handle('tok:get-recent-logs', () => logger.getRecentLogs(100));
handle('tok:open-logs-folder', () => logger.openLogsFolder());
handle('tok:clean-old-logs', (_, days?: number) =>
  logger.cleanOldLogs(Number.isFinite(days) && (days as number) > 0 ? days : undefined)
);
handle('tok:set-log-retention', (_, days: number) => {
  if (Number.isFinite(days) && days > 0) logger.setRetentionDays(days);
});

// Updater Handlers
handle('tok:check-for-updates', () => updater.checkForUpdates());
handle('tok:open-release-url', (_, url?: string) => updater.openReleaseUrl(url));
handle('tok:download-update', async (event, url: string) => {
  return await updater.downloadUpdate(url, (progress) => {
    try {
      if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send('tok:update-download-progress', progress);
      }
    } catch {
      // Ignore destroyed window
    }
  });
});
handle('tok:apply-update', (_, filePath: string, silent?: boolean) => updater.applyInstaller(filePath, silent));
handle('tok:inspect-offline-update', (_, filePath: string) => updater.inspectOfflineFile(filePath));
handle('tok:scan-usb-updates', () => updater.scanUsbDrives());

// Plugin Handlers
handle('tok:get-plugins', async () => {
  trace('ipc-get-plugins-start');
  const plugins = await pluginManager.discoverPlugins();
  trace('ipc-get-plugins-end');
  return plugins;
});
handle('tok:toggle-plugin', (_, payload) => pluginManager.togglePlugin(payload?.pluginId, payload?.enabled));
handle('tok:open-plugins-folder', () => pluginManager.openPluginsFolder());
handle('tok:reload-plugins', () => pluginManager.discoverPlugins());
handle('tok:get-plugin-system-status', () => pluginManager.getSystemStatus());
handle('tok:set-plugin-safe-mode', (_, enabled: boolean) => pluginManager.setSafeMode(enabled));

// System Handlers
handle('tok:open-external', (_, url: string) => openExternalSafely(url));
handle('tok:get-app-info', () => ({
  version: getAppVersion(),
  name: 'TypesetOK (TOK)',
  repoUrl: APP_REPO_URL
}));

// File Dialogs
handle('tok:show-save-dialog', async (_, options?: { defaultPath?: string; filters?: { name: string; extensions: string[] }[] }) => {
  if (!mainWindow) return null;
  const res = await dialog.showSaveDialog(mainWindow, {
    defaultPath: withProjectsDir(options?.defaultPath),
    filters: options?.filters ?? [{ name: 'All Files', extensions: ['*'] }],
  });
  if (res.canceled) return null;
  return res.filePath;
});

handle('tok:show-open-dialog', async (_, options?: { filters?: { name: string; extensions: string[] }[] }) => {
  if (!mainWindow) return null;
  const res = await dialog.showOpenDialog(mainWindow, {
    defaultPath: getProjectsDir(),
    filters: options?.filters ?? [{ name: 'TypesetOK Document', extensions: ['tok'] }],
  });
  if (res.canceled || !res.filePaths.length) return null;
  return res.filePaths[0];
});

handle('tok:show-import-dialog', async (_, options?: { filters?: { name: string; extensions: string[] }[] }) => {
  if (!mainWindow) return null;
  const res = await dialog.showOpenDialog(mainWindow, {
    title: 'ייבוא מסמך ל-TypesetOK',
    filters: options?.filters ?? [
      { name: 'כל המסמכים הנתמכים (*.docx, *.md, *.txt)', extensions: ['docx', 'md', 'txt'] },
      { name: 'מסמכי Microsoft Word (*.docx)', extensions: ['docx'] },
      { name: 'מסמכי Markdown (*.md)', extensions: ['md'] },
      { name: 'קובצי טקסט פשוט (*.txt)', extensions: ['txt'] },
      { name: 'כל הקבצים (*.*)', extensions: ['*'] }
    ],
    properties: ['openFile']
  });
  if (res.canceled || !res.filePaths.length) return null;
  return res.filePaths[0];
});

ipcMain.on('tok:get-startup-file-sync', (event) => {
  event.returnValue = isAppUrl(event.senderFrame?.url) ? startupFilePath : null;
});

handle('tok:get-startup-file', () => startupFilePath);

handle('tok:read-import-file', async (_, filePath: string) => {
  const sourcePath = requireString(filePath, 'filePath');
  const resolvedPath = path.isAbsolute(sourcePath) ? sourcePath : path.resolve(sourcePath);
  if (!fs.existsSync(resolvedPath)) {
    throw new Error(`Import file does not exist: ${resolvedPath}`);
  }
  const stat = await fs.promises.stat(resolvedPath);
  const buffer = await fs.promises.readFile(resolvedPath);
  const ext = path.extname(resolvedPath).toLowerCase();
  const isBinary = ext === '.docx';
  let docxXml: string | undefined;
  let footnotesXml: string | undefined;
  let endnotesXml: string | undefined;
  let numberingXml: string | undefined;
  let stylesXml: string | undefined;
  if (ext === '.docx') {
    try {
      const parts = extractDocxPartsFromBuffer(buffer);
      if (parts.documentXml) docxXml = parts.documentXml;
      footnotesXml = parts.footnotesXml;
      endnotesXml = parts.endnotesXml;
      numberingXml = parts.numberingXml;
      stylesXml = parts.stylesXml;
    } catch (err: any) {
      logger.warn(`[IPC] Failed to pre-extract docx XML: ${err?.message}`);
    }
  }
  logger.info(`[IPC] Import file read successfully: ${resolvedPath} (${stat.size} bytes)`);
  return {
    fileName: path.basename(resolvedPath),
    filePath: resolvedPath,
    size: stat.size,
    isBinary,
    docxXml,
    footnotesXml,
    endnotesXml,
    numberingXml,
    stylesXml,
    dataBase64: buffer.toString('base64'),
    text: docxXml || (isBinary ? undefined : buffer.toString('utf-8'))
  };
});

handle('tok:decompress-raw', async (_, dataBase64: string) => {
  const buf = Buffer.from(requireString(dataBase64, 'dataBase64'), 'base64');
  const zlib = await import('node:zlib');
  const decompressed = zlib.inflateRawSync(buf);
  return decompressed.toString('base64');
});

// Window and Application Lifecycle Handlers
handle('tok:close-window', () => {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.close();
  }
  return { success: true };
});

handle('tok:minimize-window', () => {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.minimize();
  }
  return { success: true };
});

handle('tok:maximize-window', () => {
  if (mainWindow && !mainWindow.isDestroyed()) {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow.maximize();
    }
    return { success: true, isMaximized: mainWindow.isMaximized() };
  }
  return { success: false, isMaximized: false };
});

handle('tok:is-maximized', () => {
  return { isMaximized: mainWindow && !mainWindow.isDestroyed() ? mainWindow.isMaximized() : false };
});

handle('tok:quit-app', () => {
  app.quit();
  return { success: true };
});

// ---------------------------------------------------------------------------
// App lifecycle
// ---------------------------------------------------------------------------

/**
 * Housekeeping that must not delay the first frame (log retention sweep).
 * Runs a few seconds after the main window is visible.
 */
let deferredWorkScheduled = false;
function scheduleDeferredStartupWork(): void {
  if (deferredWorkScheduled) return;
  deferredWorkScheduled = true;
  setTimeout(() => {
    logger.cleanOldLogs().catch(() => {});
  }, 3000);
}

process.on('unhandledRejection', (reason: any) => {
  logger.error('[MAIN] Unhandled promise rejection', { reason: reason?.message ?? String(reason) });
});

// Defense in depth: no <webview> anywhere, no new windows from any web contents.
app.on('web-contents-created', (_e, contents) => {
  contents.on('will-attach-webview', (event) => event.preventDefault());
  contents.setWindowOpenHandler(({ url }) => {
    openExternalSafely(url);
    return { action: 'deny' };
  });
});

// Set App User Model ID so Windows properly groups and displays the custom taskbar icon
if (process.platform === 'win32') {
  app.setAppUserModelId('TypesetOK.App');
}

// A second launch focuses the running instance instead of starting a competing
// copy on the same profile.
if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  // Capture startup file argument if launched with a file path from OS file association or CLI
  startupFilePath = extractFilePathFromArgs(process.argv);

  app.on('second-instance', (_event, commandLine) => {
    const win = mainWindow;
    if (win && !win.isDestroyed()) {
      if (win.isMinimized()) win.restore();
      if (!win.isVisible()) win.show();
      win.focus();
      const filePath = extractFilePathFromArgs(commandLine);
      if (filePath) {
        win.webContents.send('tok:event', { action: 'open-file-at-path', data: filePath });
      }
    }
  });

  app.on('open-file', (event, filePath) => {
    event.preventDefault();
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('tok:event', { action: 'open-file-at-path', data: filePath });
    } else {
      startupFilePath = filePath;
    }
  });

  app.whenReady().then(() => {
    trace('app-ready');
    // Launch main window directly without competing splash process for fastest launch
    createWindow();
    trace('main-window-created');
    // Cheap: resolves the log dir; file writes are async and buffered.
    // The plugin manager initialises lazily on first use (tok:get-plugins).
    logger.init(14);
  });
}

let isAppQuitting = false;

app.on('before-quit', () => {
  isAppQuitting = true;
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (mainWindow === null && app.isReady()) {
    createWindow();
  }
});

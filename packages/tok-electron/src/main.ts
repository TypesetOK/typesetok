import { app, BrowserWindow, ipcMain, shell, dialog, IpcMainInvokeEvent } from 'electron';
import * as path from 'path';
import * as fs from 'fs';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { buildApplicationMenu } from './menu';
import { logger } from './logger';
import { updater, getAppVersion, isSafeExternalUrl } from './updater';
import { pluginManager } from './plugin-manager';

let mainWindow: BrowserWindow | null = null;
let splashWindow: BrowserWindow | null = null;

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
  });

  // If the UI cannot load at all, don't leave the user staring at the splash.
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

  if (action === 'get-demo-html') {
    const tempPath = path.join(app.getPath('temp'), `tok_demo_${process.pid}_${Date.now()}.html`);
    try {
      const { code, stderr } = await runCli(['render-html', '--demo', tempPath]);
      if (code !== 0) throw new Error(`Failed to generate demo HTML (code ${code}): ${stderr}`);
      const html = await fs.promises.readFile(tempPath, 'utf-8');
      return { ok: true, html };
    } finally {
      fs.promises.unlink(tempPath).catch(() => {});
    }
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

  return { status: 'unhandled_command', cmd };
});

async function renderWithCli(kind: 'pdf' | 'html', payload: any): Promise<string> {
  const inputPath = requireString(payload?.inputPath, 'inputPath');
  const outputPath = requireString(payload?.outputPath, 'outputPath');
  const { code, stdout, stderr } = await runCli([`render-${kind}`, inputPath, outputPath]);
  if (code !== 0) throw new Error(`tok-cli failed with code ${code}: ${stderr}`);
  if (kind === 'pdf') logger.info(`[IPC] PDF rendering succeeded to: ${outputPath}`);
  return stdout;
}

handle('tok:render-pdf', (_, payload) => renderWithCli('pdf', payload));
handle('tok:render-html', (_, payload) => renderWithCli('html', payload));

// Logger Handlers
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
    defaultPath: options?.defaultPath,
    filters: options?.filters ?? [{ name: 'All Files', extensions: ['*'] }],
  });
  if (res.canceled) return null;
  return res.filePath;
});

handle('tok:show-open-dialog', async (_, options?: { filters?: { name: string; extensions: string[] }[] }) => {
  if (!mainWindow) return null;
  const res = await dialog.showOpenDialog(mainWindow, {
    filters: options?.filters ?? [{ name: 'TypesetOK Document', extensions: ['tok'] }],
  });
  if (res.canceled || !res.filePaths.length) return null;
  return res.filePaths[0];
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
  app.on('second-instance', () => {
    const win = mainWindow;
    if (win && !win.isDestroyed()) {
      if (win.isMinimized()) win.restore();
      if (!win.isVisible()) win.show();
      win.focus();
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

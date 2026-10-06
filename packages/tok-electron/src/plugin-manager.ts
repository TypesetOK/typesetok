import { app, shell } from 'electron';
import * as fs from 'fs';
import * as path from 'path';
import { logger } from './logger';

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  main: string; // e.g. "index.ts" or "index.js"
  enabled?: boolean;
  permissions?: string[];
  minTokVersion?: string;
}

export interface LoadedPlugin {
  manifest: PluginManifest;
  dirPath: string;
  sourceType: 'ts' | 'js';
  compiledCode: string;
  enabled: boolean;
  error?: string;
  safeModeBlocked?: boolean;
}

export interface PluginSystemStatus {
  isSafeMode: boolean;
  isSaferActive: boolean;
  userPluginsDir: string;
  builtinPluginsDir: string;
}

/** Bump when compiler options change so stale compiled output is discarded. */
const COMPILE_CACHE_VERSION = 3;

interface CompileCacheEntry {
  mtimeMs: number;
  size: number;
  code: string;
}

const fsp = fs.promises;

async function pathExists(p: string): Promise<boolean> {
  try {
    await fsp.access(p);
    return true;
  } catch {
    return false;
  }
}

/** Write-then-rename so a crash or quit mid-write never leaves a truncated JSON file. */
async function writeFileAtomic(file: string, data: string): Promise<void> {
  const tmp = `${file}.${process.pid}.${Date.now()}.tmp`;
  await fsp.writeFile(tmp, data, 'utf-8');
  try {
    await fsp.rename(tmp, file);
  } catch (err) {
    await fsp.unlink(tmp).catch(() => {});
    throw err;
  }
}

/** Checks whether Safer (סייפר - מערכת עמדות מחשב תורניות / מוסדיות) is installed or active */
export function isSaferEnvironment(): boolean {
  try {
    const candidates = [
      'C:\\Program Files\\safer',
      'C:\\Program Files (x86)\\safer',
      'C:\\safer'
    ];
    for (const p of candidates) {
      if (fs.existsSync(p)) return true;
    }
  } catch {}
  return process.argv.includes('--safer') || process.env.SAFER_MODE === '1';
}

/** Checks whether Safe Mode (מצב בטוח) was requested via CLI flags or env */
export function isSafeModeRequested(): boolean {
  return (
    process.argv.includes('--safe-mode') ||
    process.argv.includes('--disable-plugins') ||
    process.env.TOK_SAFE_MODE === '1'
  );
}

/** esbuild is a dev dependency: present when running from source, absent in packaged builds. */
let esbuildModule: any | null | undefined;
function loadEsbuild(): any | null {
  if (esbuildModule === undefined) {
    try {
      esbuildModule = require('esbuild');
    } catch {
      esbuildModule = null;
    }
  }
  return esbuildModule;
}

export class TokPluginManager {
  private userPluginsDir: string;
  private builtinPluginsDir: string;
  private configFile: string;
  private cacheFile: string;
  private pluginStates: Record<string, boolean> = {};
  private compileCache: Record<string, CompileCacheEntry> = {};
  private initPromise: Promise<void> | null = null;
  private discoverPromise: Promise<LoadedPlugin[]> | null = null;
  private safeModeConfigured = false;
  private isSaferActiveCached = false;

  constructor() {
    let base: string;
    try {
      base = app.getPath('userData');
    } catch {
      base = process.cwd();
    }
    this.userPluginsDir = path.join(base, 'plugins');
    this.configFile = path.join(base, 'plugins_config.json');
    this.cacheFile = path.join(base, 'plugins_compile_cache.json');
    // <app root>/plugins (repo root in dev, resources/app in packaged builds)
    this.builtinPluginsDir = path.join(__dirname, '../../../plugins');
    this.isSaferActiveCached = isSaferEnvironment();
  }

  /** Returns true if Safe Mode is currently active (CLI flag or user config) */
  public isSafeMode(): boolean {
    return isSafeModeRequested() || this.safeModeConfigured;
  }

  /** Returns true if Safer managed workstation environment is detected */
  public isSaferActive(): boolean {
    return this.isSaferActiveCached;
  }

  public getSystemStatus(): PluginSystemStatus {
    return {
      isSafeMode: this.isSafeMode(),
      isSaferActive: this.isSaferActive(),
      userPluginsDir: this.userPluginsDir,
      builtinPluginsDir: this.builtinPluginsDir
    };
  }

  public async setSafeMode(enabled: boolean): Promise<boolean> {
    await this.init();
    this.safeModeConfigured = Boolean(enabled);
    const config = await this.readJson<Record<string, any>>(this.configFile, {});
    config.__safeMode = this.safeModeConfigured;
    try {
      await writeFileAtomic(this.configFile, JSON.stringify(config, null, 2));
      logger.info(`[PLUGINS] Safe Mode state persisted: ${this.safeModeConfigured}`);
      return true;
    } catch (err: any) {
      logger.error('[PLUGINS] Failed to persist Safe Mode state:', err?.message);
      return false;
    }
  }

  /**
   * Lazy, idempotent, non-blocking initialisation: creates the user plugin folder
   * on first run, loads enabled/disabled states and compile cache.
   * Resilient to restricted environments (e.g. Safer / Deep Freeze).
   */
  public init(): Promise<void> {
    if (!this.initPromise) {
      this.initPromise = this.doInit().catch((err: any) => {
        logger.error('[PLUGINS] Failed to initialize Plugin Manager:', err?.message);
      });
    }
    return this.initPromise;
  }

  private async doInit(): Promise<void> {
    try {
      if (!(await pathExists(this.userPluginsDir))) {
        await fsp.mkdir(this.userPluginsDir, { recursive: true });
      } else {
        // Clean up legacy starter examples if present so users don't see mock/dummy plugins
        for (const legacyDir of ['smart-quotes-ts', 'word-counter-js']) {
          const p = path.join(this.userPluginsDir, legacyDir);
          if (await pathExists(p)) {
            try {
              await fsp.rm(p, { recursive: true, force: true });
            } catch {}
          }
        }
      }
    } catch (err: any) {
      // In locked down or Deep Freeze environments, fallback to Documents folder
      logger.warn('[PLUGINS] Cannot write to default userData plugins dir. Attempting fallback:', err?.message);
      try {
        let fallbackBase: string;
        try {
          fallbackBase = app.getPath('documents');
        } catch {
          fallbackBase = process.cwd();
        }
        this.userPluginsDir = path.join(fallbackBase, 'TypesetOK', 'plugins');
        await fsp.mkdir(this.userPluginsDir, { recursive: true });
      } catch (fallbackErr: any) {
        logger.error('[PLUGINS] Plugin folder fallback also failed:', fallbackErr?.message);
      }
    }

    const config = await this.readJson<Record<string, any>>(this.configFile, {});
    if (typeof config.__safeMode === 'boolean') {
      this.safeModeConfigured = config.__safeMode;
    }

    // Load states, ignoring internal metadata keys starting with "__"
    this.pluginStates = {};
    for (const [key, val] of Object.entries(config)) {
      if (!key.startsWith('__') && typeof val === 'boolean') {
        this.pluginStates[key] = val;
      }
    }

    // Clean legacy dummy ids from config
    delete this.pluginStates['tok-smart-quotes'];
    delete this.pluginStates['tok-word-counter'];

    const cache = await this.readJson<{ version?: number; entries?: Record<string, CompileCacheEntry> }>(this.cacheFile, {});
    this.compileCache = cache.version === COMPILE_CACHE_VERSION && cache.entries ? cache.entries : {};
    logger.info('[PLUGINS] Plugin Manager initialized.', {
      dir: this.userPluginsDir,
      isSafer: this.isSaferActive(),
      isSafeMode: this.isSafeMode()
    });
  }

  private async readJson<T>(file: string, fallback: T): Promise<T> {
    try {
      const parsed = JSON.parse(await fsp.readFile(file, 'utf-8'));
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : fallback;
    } catch (err: any) {
      if (err?.code !== 'ENOENT') logger.warn(`[PLUGINS] Ignoring unreadable ${path.basename(file)}:`, err?.message);
      return fallback;
    }
  }

  private async saveConfig(): Promise<void> {
    try {
      const payload = {
        ...this.pluginStates,
        __safeMode: this.safeModeConfigured
      };
      await writeFileAtomic(this.configFile, JSON.stringify(payload, null, 2));
    } catch (err: any) {
      logger.error('[PLUGINS] Failed to save plugins config:', err?.message);
    }
  }

  /**
   * Compiles TypeScript code to runnable JavaScript using esbuild's async API.
   * If esbuild is absent (packaged builds without devDependencies), uses a robust
   * syntax transform that safely strips TypeScript types without corrupting
   * JavaScript object literals or colons.
   */
  private async compileTypeScript(code: string, filename: string): Promise<string> {
    const esbuild = loadEsbuild();
    if (esbuild) {
      try {
        const result = await esbuild.transform(code, {
          loader: 'ts',
          target: 'es2022',
          format: 'cjs',
          charset: 'utf8',
          sourcefile: filename
        });
        return result.code;
      } catch (err: any) {
        logger.warn(`[PLUGINS] esbuild transform failed for ${filename}, using fallback:`, err?.message);
      }
    }

    // Robust TypeScript stripper fallback (does not damage JSON or object literals)
    return code
      // Strip type-only imports: import type { ... } from '...';
      .replace(/import\s+type\s+[^;]+;/g, '')
      // Strip interfaces
      .replace(/(?:export\s+)?interface\s+\w+(?:<[^>]*>)?(?:\s+extends\s+[^{]+)?\s*\{[^}]*\}/gs, '')
      // Strip type aliases
      .replace(/(?:export\s+)?type\s+\w+(?:<[^>]*>)?\s*=\s*[^;]+;/g, '')
      // Transform export function activate(...) to module.exports.activate = function(...)
      .replace(/export\s+function\s+([A-Za-z0-9_$]+)/g, 'module.exports.$1 = function $1')
      .replace(/export\s+const\s+([A-Za-z0-9_$]+)/g, 'const $1 = module.exports.$1')
      .replace(/export\s+default\s+/g, 'module.exports = ')
      // Strip function parameter types: (tok: TokPluginContext) -> (tok)
      .replace(/\(([^):]+):\s*[A-Za-z0-9_<>\[\]|&\s]+\)/g, '($1)')
      // Strip function return types: ): void { or ): Promise<void> { -> ) {
      .replace(/\)\s*:\s*[A-Za-z0-9_<>\[\]|&\s]+(?=\s*\{)/g, ') ')
      // Strip 'as Type' casts
      .replace(/\s+as\s+[A-Za-z0-9_<>\[\]]+/g, '');
  }

  /** Returns compiled code for a TS entry, reusing the on-disk cache when the source is unchanged. */
  private async getCompiledTs(mainPath: string, code: string, filename: string): Promise<{ code: string; fresh: boolean }> {
    const stats = await fsp.stat(mainPath);
    const cached = this.compileCache[mainPath];
    if (cached && cached.mtimeMs === stats.mtimeMs && cached.size === stats.size) {
      return { code: cached.code, fresh: false };
    }
    const compiled = await this.compileTypeScript(code, filename);
    if (loadEsbuild()) {
      this.compileCache[mainPath] = { mtimeMs: stats.mtimeMs, size: stats.size, code: compiled };
    }
    return { code: compiled, fresh: true };
  }

  private async loadPlugin(pluginDir: string, isBuiltin = false): Promise<{ plugin: LoadedPlugin | null; compiled: boolean }> {
    const manifestPath = path.join(pluginDir, 'plugin.json');
    let manifest: PluginManifest;
    try {
      manifest = JSON.parse(await fsp.readFile(manifestPath, 'utf-8'));
    } catch (err: any) {
      if (err?.code !== 'ENOENT' && err?.code !== 'ENOTDIR') logger.warn(`[PLUGINS] Invalid plugin.json in ${pluginDir}:`, err?.message);
      return { plugin: null, compiled: false };
    }
    if (!manifest || typeof manifest.id !== 'string' || typeof manifest.main !== 'string') {
      logger.warn(`[PLUGINS] plugin.json in ${pluginDir} is missing "id" or "main"`);
      return { plugin: null, compiled: false };
    }

    // Security: normalize permissions
    if (manifest.permissions && !Array.isArray(manifest.permissions)) {
      manifest.permissions = [];
    }

    const safeMode = this.isSafeMode();
    // In Safe Mode, all non-builtin user plugins are strictly blocked from activating
    const blockedBySafeMode = safeMode && !isBuiltin;

    const isEnabled = blockedBySafeMode
      ? false
      : (this.pluginStates[manifest.id] !== undefined
          ? this.pluginStates[manifest.id]
          : (manifest.enabled ?? true));

    const mainPath = path.resolve(pluginDir, manifest.main);
    const isTs = mainPath.endsWith('.ts');
    const broken = (error: string): LoadedPlugin => ({
      manifest,
      dirPath: pluginDir,
      sourceType: isTs ? 'ts' : 'js',
      compiledCode: '',
      enabled: false,
      error,
      safeModeBlocked: blockedBySafeMode
    });

    // Security: Path Traversal Guard. The entry file must strictly reside inside the plugin's folder.
    const rel = path.relative(pluginDir, mainPath);
    if (!rel || rel.startsWith('..') || path.isAbsolute(rel)) {
      return { plugin: broken(`Main entry must be inside the plugin folder: ${manifest.main}`), compiled: false };
    }

    let codeRaw: string;
    try {
      codeRaw = await fsp.readFile(mainPath, 'utf-8');
    } catch {
      return { plugin: broken(`Main entry file not found: ${manifest.main}`), compiled: false };
    }

    let compiledCode = codeRaw;
    let compiled = false;
    if (isTs) {
      const r = await this.getCompiledTs(mainPath, codeRaw, manifest.main);
      compiledCode = r.code;
      compiled = r.fresh;
    }

    return {
      plugin: {
        manifest,
        dirPath: pluginDir,
        sourceType: isTs ? 'ts' : 'js',
        compiledCode,
        enabled: isEnabled,
        safeModeBlocked: blockedBySafeMode
      },
      compiled
    };
  }

  /**
   * Discovers and compiles all plugins from user and builtin directories.
   * Fully asynchronous; concurrent callers share one scan.
   */
  public discoverPlugins(): Promise<LoadedPlugin[]> {
    if (!this.discoverPromise) {
      this.discoverPromise = this.doDiscover().finally(() => {
        this.discoverPromise = null;
      });
    }
    return this.discoverPromise;
  }

  private async doDiscover(): Promise<LoadedPlugin[]> {
    await this.init();
    const plugins: LoadedPlugin[] = [];
    let cacheDirty = false;

    const dirsToScan = [
      { dir: this.builtinPluginsDir, isBuiltin: true },
      { dir: this.userPluginsDir, isBuiltin: false }
    ];

    for (const { dir: baseDir, isBuiltin } of dirsToScan) {
      let entries: fs.Dirent[];
      try {
        entries = await fsp.readdir(baseDir, { withFileTypes: true });
      } catch (err: any) {
        if (err?.code !== 'ENOENT') logger.error('[PLUGINS] Error reading plugins directory:', err?.message);
        continue;
      }

      const results = await Promise.all(
        entries
          .filter((e) => e.isDirectory() || e.isSymbolicLink())
          .map((e) =>
            this.loadPlugin(path.join(baseDir, e.name), isBuiltin).catch((err: any) => {
              logger.warn(`[PLUGINS] Failed to load plugin in ${path.join(baseDir, e.name)}:`, err?.message);
              return { plugin: null, compiled: false };
            })
          )
      );
      for (const r of results) {
        if (r.plugin) plugins.push(r.plugin);
        if (r.compiled) cacheDirty = true;
      }
    }

    // Drop cache entries of plugins that no longer exist.
    const live = new Set(plugins.map((p) => path.resolve(p.dirPath, p.manifest.main)));
    for (const key of Object.keys(this.compileCache)) {
      if (!live.has(key)) {
        delete this.compileCache[key];
        cacheDirty = true;
      }
    }

    if (cacheDirty) {
      Promise.resolve()
        .then(() => loadEsbuild()?.stop?.())
        .catch(() => {});
      writeFileAtomic(this.cacheFile, JSON.stringify({ version: COMPILE_CACHE_VERSION, entries: this.compileCache })).catch((err: any) => {
        logger.warn('[PLUGINS] Failed to write compile cache:', err?.message);
      });
    }
    return plugins;
  }

  public async togglePlugin(pluginId: string, enabled: boolean): Promise<boolean> {
    if (typeof pluginId !== 'string' || !pluginId) return false;
    await this.init();
    this.pluginStates[pluginId] = Boolean(enabled);
    await this.saveConfig();
    logger.info(`[PLUGINS] Plugin ${pluginId} state changed to: ${enabled ? 'enabled' : 'disabled'}`);
    return true;
  }

  public async openPluginsFolder(): Promise<void> {
    await this.init();
    try {
      await fsp.mkdir(this.userPluginsDir, { recursive: true });
    } catch (err: any) {
      logger.error('[PLUGINS] Cannot create plugins folder:', err?.message);
      return;
    }
    try {
      const err = await shell.openPath(this.userPluginsDir);
      if (err) logger.warn('[PLUGINS] Failed to open plugins folder:', err);
    } catch (openErr: any) {
      logger.warn('[PLUGINS] shell.openPath blocked or failed (Safer / restricted environment):', openErr?.message);
    }
  }
}

export const pluginManager = new TokPluginManager();

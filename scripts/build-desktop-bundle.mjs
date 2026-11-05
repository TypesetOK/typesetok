// Packages the built app (run `npm run build` first) into a portable Windows folder:
//   dist/TypesetOK-v<version>-windows-x64/TypesetOK.exe
//
// Env:
//   TOK_CLI_PATH=<path>        tok-cli binary to bundle (otherwise the usual cargo target dirs are searched)
//   TOK_KEEP_ALL_LOCALES=1     keep every Chromium locale (default: only Hebrew + English, ~35 MB smaller)
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const rootPkg = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf-8'));
const version = rootPkg.version || '0.0.0';

const electronDist = path.join(rootDir, 'node_modules/electron/dist');
const outDir = path.join(rootDir, `dist/TypesetOK-v${version}-windows-x64`);
const isWin = process.platform === 'win32';

function fail(msg) {
  console.error(`[BUNDLE] ${msg}`);
  process.exit(1);
}

console.log('[BUNDLE] Packaging TypesetOK Standalone Windows Desktop App...');

// Ensure UI bundle is always fresh and correctly packaged with esbuild
execSync('node scripts/bundle-ui.mjs', { cwd: rootDir, stdio: 'inherit' });

const electronDistSrc = path.join(rootDir, 'packages/tok-electron/dist');
const uiDistSrc = path.join(rootDir, 'packages/tok-ui/dist');
const requiredBuildOutputs = [
  path.join(electronDistSrc, 'main.js'),
  path.join(electronDistSrc, 'preload.js'),
  path.join(uiDistSrc, 'index.html'),
  path.join(uiDistSrc, 'renderer.js'),
];
for (const f of requiredBuildOutputs) {
  if (!fs.existsSync(f)) fail(`Missing build output ${path.relative(rootDir, f)} - run "npm run build" first.`);
}
if (!fs.existsSync(electronDist)) fail(`Electron dist directory not found at ${electronDist}`);

// 1. Prepare output dir
fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

// 2. Copy Electron runtime files (without the unused default app)
console.log('[BUNDLE] Copying Electron runtime...');
fs.cpSync(electronDist, outDir, { recursive: true });
fs.rmSync(path.join(outDir, 'resources/default_app.asar'), { force: true });

// 2b. Keep only the Chromium locales the app ships UI for (Chromium falls back to en-US).
if (!process.env.TOK_KEEP_ALL_LOCALES) {
  const localesDir = path.join(outDir, 'locales');
  const keep = new Set(['he.pak', 'en-US.pak', 'en-GB.pak']);
  if (fs.existsSync(localesDir)) {
    let removed = 0;
    for (const f of fs.readdirSync(localesDir)) {
      if (f.endsWith('.pak') && !keep.has(f)) {
        fs.rmSync(path.join(localesDir, f));
        removed++;
      }
    }
    console.log(`[BUNDLE] Pruned ${removed} unused Chromium locales (set TOK_KEEP_ALL_LOCALES=1 to keep them).`);
  }
}

// 3. Rename electron to TypesetOK
const defaultExe = path.join(outDir, isWin ? 'electron.exe' : 'electron');
const targetExe = path.join(outDir, isWin ? 'TypesetOK.exe' : 'TypesetOK');
if (fs.existsSync(defaultExe)) {
  fs.renameSync(defaultExe, targetExe);
  console.log(`[BUNDLE] Renamed ${path.basename(defaultExe)} -> ${path.basename(targetExe)}`);
}

// 4. Create resources structure
const resourcesDir = path.join(outDir, 'resources');
const appDir = path.join(resourcesDir, 'app');
const binDir = path.join(resourcesDir, 'bin');
fs.mkdirSync(appDir, { recursive: true });
fs.mkdirSync(binDir, { recursive: true });

// 5. Copy tok-cli into resources/bin/
const candidateCliPaths = [
  process.env.TOK_CLI_PATH,
  ...(process.env.CARGO_TARGET_DIR ? [
    path.join(process.env.CARGO_TARGET_DIR, 'release/tok-cli.exe'),
    path.join(process.env.CARGO_TARGET_DIR, 'release/tok-cli'),
    path.join(process.env.CARGO_TARGET_DIR, 'debug/tok-cli.exe'),
    path.join(process.env.CARGO_TARGET_DIR, 'debug/tok-cli')
  ] : []),
  ...(process.env.LOCALAPPDATA ? [
    path.join(process.env.LOCALAPPDATA, 'tok_target/release/tok-cli.exe'),
    path.join(process.env.LOCALAPPDATA, 'tok_target/debug/tok-cli.exe')
  ] : []),
  path.join(rootDir, 'target/release/tok-cli.exe'),
  path.join(rootDir, 'target/release/tok-cli'),
  path.join(rootDir, 'target/debug/tok-cli.exe'),
  path.join(rootDir, 'target/debug/tok-cli'),
].filter(Boolean);

const foundCli = candidateCliPaths.find((p) => fs.existsSync(p));
if (foundCli) {
  const destCli = path.join(binDir, path.basename(foundCli));
  fs.copyFileSync(foundCli, destCli);
  console.log('[BUNDLE] Bundled CLI binary from:', foundCli, '->', destCli);
} else {
  console.warn('[BUNDLE] Warning: tok-cli binary not found in candidate paths.');
}

// 6. Populate resources/app
fs.writeFileSync(
  path.join(appDir, 'package.json'),
  JSON.stringify(
    {
      name: 'typesetok',
      version: version,
      main: 'packages/tok-electron/dist/main.js'
    },
    null,
    2
  )
);

// Only runtime files: the main-process JS and the bundled renderer.
// (Type declarations and the per-file tsc output of tok-ui are not needed at runtime.)
const electronAppDir = path.join(appDir, 'packages/tok-electron/dist');
fs.mkdirSync(electronAppDir, { recursive: true });
for (const f of fs.readdirSync(electronDistSrc)) {
  if (f.endsWith('.js')) fs.copyFileSync(path.join(electronDistSrc, f), path.join(electronAppDir, f));
}
const uiAppDir = path.join(appDir, 'packages/tok-ui/dist');
fs.mkdirSync(uiAppDir, { recursive: true });
for (const f of ['index.html', 'renderer.js']) {
  fs.copyFileSync(path.join(uiDistSrc, f), path.join(uiAppDir, f));
}

// esbuild (JS API + the native binary for this platform, ~11 MB) so TypeScript plugins are
// compiled properly at runtime. Without it the plugin manager falls back to a regex
// type-stripper that cannot handle real-world TS (e.g. the bundled smart-quotes example).
const esbuildSrc = path.join(rootDir, 'node_modules/esbuild');
const esbuildBinSrc = path.join(rootDir, `node_modules/@esbuild/${process.platform}-${process.arch}`);
if (fs.existsSync(esbuildSrc) && fs.existsSync(esbuildBinSrc)) {
  const nm = path.join(appDir, 'node_modules');
  fs.mkdirSync(path.join(nm, 'esbuild/lib'), { recursive: true });
  for (const f of ['package.json', 'LICENSE.md', 'lib/main.js']) {
    fs.copyFileSync(path.join(esbuildSrc, f), path.join(nm, 'esbuild', f));
  }
  fs.cpSync(esbuildBinSrc, path.join(nm, `@esbuild/${process.platform}-${process.arch}`), { recursive: true });
  console.log('[BUNDLE] Bundled esbuild for runtime TypeScript plugin compilation.');
} else {
  console.warn('[BUNDLE] Warning: esbuild not found; TypeScript plugins will use the limited fallback compiler.');
}

const assetsSrc = path.join(rootDir, 'assets');
if (fs.existsSync(assetsSrc)) {
  fs.cpSync(assetsSrc, path.join(appDir, 'assets'), { recursive: true });
  if (fs.existsSync(path.join(assetsSrc, 'icon.ico'))) {
    fs.copyFileSync(path.join(assetsSrc, 'icon.ico'), path.join(resourcesDir, 'icon.ico'));
  }
  if (fs.existsSync(path.join(assetsSrc, 'icon.png'))) {
    fs.copyFileSync(path.join(assetsSrc, 'icon.png'), path.join(resourcesDir, 'icon.png'));
  }
}
const builtinPluginsSrc = path.join(rootDir, 'plugins');
if (fs.existsSync(builtinPluginsSrc)) {
  fs.cpSync(builtinPluginsSrc, path.join(appDir, 'plugins'), { recursive: true });
}

// Copy license and readme
for (const f of ['LICENSE.md', 'README.md']) {
  const src = path.join(rootDir, f);
  if (fs.existsSync(src)) fs.copyFileSync(src, path.join(outDir, f));
}

// 7. Embed application icon and version metadata into Windows executable (PE resources)
if (isWin && fs.existsSync(targetExe)) {
  const iconFile = path.join(assetsSrc, 'icon.ico');
  if (fs.existsSync(iconFile)) {
    try {
      const { rcedit } = await import('rcedit');
      console.log('[BUNDLE] Embedding icon and metadata into TypesetOK.exe...');
      await rcedit(targetExe, {
        icon: iconFile,
        'file-version': version,
        'product-version': version,
        'version-string': {
          CompanyName: 'TypesetOK Team',
          FileDescription: 'TypesetOK - מערכת עימוד מקצועית בעברית',
          ProductName: 'TypesetOK',
          LegalCopyright: 'Copyright © 2026 TypesetOK Contributors',
          OriginalFilename: 'TypesetOK.exe'
        }
      });
      console.log('[BUNDLE] Successfully embedded icon and metadata into TypesetOK.exe.');
    } catch (err) {
      console.warn('[BUNDLE] Warning: could not embed PE icon into TypesetOK.exe:', err.message);
    }
  }
}

console.log('[BUNDLE] Created portable app at:', outDir);
console.log('[BUNDLE] Executable binary:', targetExe);


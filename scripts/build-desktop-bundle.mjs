// Packages the built Tauri v2 app (run `npm run build` and `cargo build --release -p tok-desktop` first)
// into a portable Windows folder:
//   dist/TypesetOK-v<version>-windows-x64/TypesetOK.exe
//
// Env:
//   TOK_DESKTOP_PATH=<path>   tok-desktop binary to bundle
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const rootPkg = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf-8'));
const version = rootPkg.version || '0.0.0';

const outDir = path.join(rootDir, `dist/TypesetOK-v${version}-windows-x64`);
const isWin = process.platform === 'win32';

function fail(msg) {
  console.error(`[BUNDLE] ${msg}`);
  process.exit(1);
}

console.log('[BUNDLE] Packaging TypesetOK Tauri v2 Standalone Windows Desktop App...');
execSync('node scripts/bundle-ui.mjs --tauri', { cwd: rootDir, stdio: 'inherit' });

const candidateDesktopPaths = [
  process.env.TOK_DESKTOP_PATH,
  ...(process.env.CARGO_TARGET_DIR ? [
    path.join(process.env.CARGO_TARGET_DIR, 'release/tok-desktop.exe'),
    path.join(process.env.CARGO_TARGET_DIR, 'release/tok-desktop'),
    path.join(process.env.CARGO_TARGET_DIR, 'debug/tok-desktop.exe'),
    path.join(process.env.CARGO_TARGET_DIR, 'debug/tok-desktop')
  ] : []),
  ...(process.env.LOCALAPPDATA ? [
    path.join(process.env.LOCALAPPDATA, 'tok_target/release/tok-desktop.exe'),
    path.join(process.env.LOCALAPPDATA, 'tok_target/debug/tok-desktop.exe')
  ] : []),
  path.join(rootDir, 'target/release/tok-desktop.exe'),
  path.join(rootDir, 'target/release/tok-desktop'),
  path.join(rootDir, 'target/debug/tok-desktop.exe'),
  path.join(rootDir, 'target/debug/tok-desktop'),
].filter(Boolean);

const foundDesktop = candidateDesktopPaths.find((p) => fs.existsSync(p));
if (!foundDesktop) {
  fail('tok-desktop binary not found. Run "cargo build --release -p tok-desktop" first or set TOK_DESKTOP_PATH.');
}

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

const targetExe = path.join(outDir, isWin ? 'TypesetOK.exe' : 'TypesetOK');
fs.copyFileSync(foundDesktop, targetExe);
const exeSizeMb = (fs.statSync(targetExe).size / (1024 * 1024)).toFixed(2);
console.log(`[BUNDLE] Copied Tauri binary (${exeSizeMb} MB): ${foundDesktop} -> ${targetExe}`);

const resourcesDir = path.join(outDir, 'resources');
fs.mkdirSync(resourcesDir, { recursive: true });
const iconSrc = path.join(rootDir, 'assets/icon.ico');
if (fs.existsSync(iconSrc)) {
  fs.copyFileSync(iconSrc, path.join(resourcesDir, 'icon.ico'));
}

for (const f of ['LICENSE.md', 'README.md']) {
  const src = path.join(rootDir, f);
  if (fs.existsSync(src)) fs.copyFileSync(src, path.join(outDir, f));
}

console.log('[BUNDLE] Created Tauri portable app at:', outDir);
console.log('[BUNDLE] Executable binary:', targetExe);

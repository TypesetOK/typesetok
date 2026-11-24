// Bundles the renderer (packages/tok-ui/src/renderer.ts) into packages/tok-ui/dist/renderer.js
// and copies index.html next to it.
//
//   node scripts/bundle-ui.mjs          production bundle (minified)
//   node scripts/bundle-ui.mjs --dev    readable bundle with an inline source map
//   node scripts/bundle-ui.mjs --tauri  additionally builds packages/tok-ui/dist-tauri/
import * as esbuild from 'esbuild';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { syncChangelog } from './sync-changelog.mjs';

const defaultRootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const TAURI_ASSET_REF_RE = /\.\.\/\.\.\/\.\.\/assets\/([^'")\s]+)/g;
const CSP_META_RE = /(<meta\s+http-equiv=(["'])Content-Security-Policy\2\s+content=(["']))([\s\S]*?)(\3[^>]*>)/i;

function replaceCspDirective(csp, directiveName, nextValue) {
  const re = new RegExp(`(^|;\\s*)${directiveName}\\s+[^;]+`, 'i');
  if (!re.test(csp)) {
    throw new Error(`CSP directive "${directiveName}" not found in index.html`);
  }
  return csp.replace(re, `$1${directiveName} ${nextValue}`);
}

/**
 * Produces `packages/tok-ui/dist-tauri/` with rewritten `./assets/` paths,
 * copied runtime font/icon assets, and a Tauri-compatible CSP header without
 * mutating `packages/tok-ui/src/index.html` or the Electron `dist/` output.
 */
export function buildTauriFrontendDist(
  rootDir = defaultRootDir,
  uiDir = path.join(rootDir, 'packages/tok-ui')
) {
  const distDir = path.join(uiDir, 'dist');
  const rendererSrc = path.join(distDir, 'renderer.js');
  if (!fs.existsSync(rendererSrc)) {
    throw new Error(`Missing renderer bundle at ${rendererSrc}; run bundle-ui first`);
  }

  const srcHtmlPath = path.join(uiDir, 'src/index.html');
  let html = fs.readFileSync(srcHtmlPath, 'utf-8');

  const uniqueAssets = new Set();
  for (const match of html.matchAll(TAURI_ASSET_REF_RE)) {
    uniqueAssets.add(match[1]);
  }
  if (uniqueAssets.size === 0) {
    throw new Error(`Expected relative ../../../assets/ references in ${srcHtmlPath}`);
  }

  const cspMatch = html.match(CSP_META_RE);
  if (!cspMatch) {
    throw new Error(`Content-Security-Policy meta tag not found in ${srcHtmlPath}`);
  }

  let nextCsp = cspMatch[4];
  nextCsp = replaceCspDirective(
    nextCsp,
    'img-src',
    "'self' data: blob: tok-asset: http://tok-asset.localhost https://tok-asset.localhost"
  );
  nextCsp = replaceCspDirective(
    nextCsp,
    'font-src',
    "'self' data: tok-asset: http://tok-asset.localhost https://tok-asset.localhost"
  );
  nextCsp = replaceCspDirective(
    nextCsp,
    'connect-src',
    "'self' ipc: http://ipc.localhost https://ipc.localhost"
  );
  // The pre-export PDF preview shows a blob: URL inside an iframe.
  // The system PDF viewer is embedded as a plugin inside the frame, so object-src must allow blob: too.
  nextCsp = replaceCspDirective(nextCsp, 'frame-src', 'blob:');
  nextCsp = replaceCspDirective(nextCsp, 'object-src', 'blob:');
  html = html.replace(CSP_META_RE, `$1${nextCsp}$5`);
  html = html.replaceAll('../../../assets/', './assets/');

  const tauriDistDir = path.join(uiDir, 'dist-tauri');
  fs.rmSync(tauriDistDir, { recursive: true, force: true });
  fs.mkdirSync(tauriDistDir, { recursive: true });

  fs.copyFileSync(rendererSrc, path.join(tauriDistDir, 'renderer.js'));

  const copiedAssets = [];
  for (const rel of uniqueAssets) {
    const srcAsset = path.join(rootDir, 'assets', rel);
    if (!fs.existsSync(srcAsset)) {
      throw new Error(`Referenced UI asset missing: ${srcAsset}`);
    }
    const destAsset = path.join(tauriDistDir, 'assets', rel);
    fs.mkdirSync(path.dirname(destAsset), { recursive: true });
    fs.copyFileSync(srcAsset, destAsset);
    copiedAssets.push(rel);
  }

  fs.writeFileSync(path.join(tauriDistDir, 'index.html'), html, 'utf-8');
  return { tauriDistDir, copiedAssets };
}

export async function runBundleUi({
  dev = false,
  tauri = false,
  rootDir = defaultRootDir,
} = {}) {
  const uiDir = path.join(rootDir, 'packages/tok-ui');
  syncChangelog();
  await esbuild.build({
    entryPoints: [path.join(uiDir, 'src/renderer.ts')],
    outfile: path.join(uiDir, 'dist/renderer.js'),
    bundle: true,
    platform: 'browser',
    target: 'es2022',
    alias: {
      'tok-story-editor': path.join(rootDir, 'packages/tok-story-editor/src/index.ts'),
      'tok-viewer': path.join(rootDir, 'packages/tok-viewer/src/index.ts'),
      'tok-canvas': path.join(rootDir, 'packages/tok-canvas/src/index.ts'),
    },
    // The UI is mostly Hebrew strings; emitting them as UTF-8 instead of \uXXXX
    // escapes makes the bundle ~25% smaller. index.html declares <meta charset="utf-8">.
    charset: 'utf8',
    minify: !dev,
    // Keep class/function names: readable stack traces in logs and the
    // component names the test-suite looks for in the bundle.
    keepNames: true,
    sourcemap: dev ? 'inline' : false,
    legalComments: 'none',
    // Marks when the bundle starts evaluating (read by the opt-in startup trace).
    banner: { js: "performance.mark('tok-script-start');" },
    logLevel: 'info',
  });
  fs.copyFileSync(path.join(uiDir, 'src/index.html'), path.join(uiDir, 'dist/index.html'));
  if (tauri) {
    const res = buildTauriFrontendDist(rootDir, uiDir);
    console.log(`[BUNDLE] Built Tauri frontend dist at ${res.tauriDistDir} (${res.copiedAssets.length} assets)`);
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  runBundleUi({
    dev: process.argv.includes('--dev'),
    tauri: process.argv.includes('--tauri'),
  }).catch((err) => {
    if (err?.message && !err?.errors) {
      console.error(`[BUNDLE] ${err.message}`);
    }
    process.exit(1);
  });
}

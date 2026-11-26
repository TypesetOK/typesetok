import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const rootPkg = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf-8'));
const version = rootPkg.version || '0.35.0';
const distDir = path.join(rootDir, 'dist');
const bundleDir = path.join(distDir, `TypesetOK-v${version}-windows-x64`);
const buildOffline = process.argv.includes('--offline');
const buildDual = process.argv.includes('--dual');
const rebuildBundle = process.argv.includes('--rebuild');

console.log('====================================================');
console.log(' TypesetOK Modern Windows Installer Builder (Tauri v2)');
console.log('====================================================');

// 1. Build desktop bundle
if (rebuildBundle || !fs.existsSync(bundleDir)) {
  console.log('[BUILD-INSTALLER] Building desktop bundle...');
  execSync('node scripts/build-desktop-bundle.mjs --tauri', {
    cwd: rootDir,
    stdio: 'inherit'
  });
}

// 2. Ensure modern installer graphics exist
const sidebarBmp = path.join(rootDir, 'assets/installer-sidebar.bmp');
const headerBmp = path.join(rootDir, 'assets/installer-header.bmp');
if (!fs.existsSync(sidebarBmp) || !fs.existsSync(headerBmp)) {
  console.log('[BUILD-INSTALLER] Generating modern installer artwork and branding assets...');
  try {
    execSync('powershell -ExecutionPolicy Bypass -File scripts/generate-installer-assets.ps1', {
      cwd: rootDir,
      stdio: 'inherit'
    });
  } catch (err) {
    console.warn('[BUILD-INSTALLER] Notice: failed to auto-generate assets via powershell:', err.message);
  }
}

// 2b. Convert LICENSE.md -> scripts/LICENSE.rtf so Inno Setup & NSIS render formatted rich text
function markdownToRtf(mdText) {
  const escapeRtf = (str) => {
    let out = '';
    for (let i = 0; i < str.length; i++) {
      const code = str.charCodeAt(i);
      if (str[i] === '\\' || str[i] === '{' || str[i] === '}') {
        out += '\\' + str[i];
      } else if (code > 127) {
        const signed = code > 32767 ? code - 65536 : code;
        out += `\\u${signed}?`;
      } else {
        out += str[i];
      }
    }
    return out;
  };

  const formatInline = (text) => {
    let res = '';
    let i = 0;
    while (i < text.length) {
      if (text.startsWith('**', i)) {
        const end = text.indexOf('**', i + 2);
        if (end !== -1) {
          res += `{\\b ${escapeRtf(text.slice(i + 2, end))}}`;
          i = end + 2;
          continue;
        }
      }
      if (text[i] === '*' && text[i + 1] !== ' ') {
        const end = text.indexOf('*', i + 1);
        if (end !== -1 && text[end - 1] !== ' ') {
          res += `{\\i ${escapeRtf(text.slice(i + 1, end))}}`;
          i = end + 1;
          continue;
        }
      }
      if (text[i] === '`') {
        const end = text.indexOf('`', i + 1);
        if (end !== -1) {
          res += `{\\f1 ${escapeRtf(text.slice(i + 1, end))}}`;
          i = end + 1;
          continue;
        }
      }
      res += escapeRtf(text[i]);
      i++;
    }
    return res;
  };

  const lines = mdText.replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n');
  const body = [];
  for (const rawLine of lines) {
    const trimmed = rawLine.trim();
    if (!trimmed) {
      body.push('\\par');
      continue;
    }
    if (/^-{3,}$/.test(trimmed)) {
      body.push('\\pard\\sa100\\brdrb\\brdrs\\brdrw10\\brsp20 \\par\\pard\\sa80\\sl260\\slmult1');
      continue;
    }
    if (/^#\s+/.test(trimmed)) {
      body.push(`\\pard\\sb140\\sa80{\\b\\fs28 ${formatInline(trimmed.replace(/^#\s+/, ''))}}\\par`);
      continue;
    }
    if (/^###\s+/.test(trimmed)) {
      body.push(`\\pard\\sb120\\sa60{\\b\\fs22 ${formatInline(trimmed.replace(/^###\s+/, ''))}}\\par`);
      continue;
    }
    if (/^##\s+/.test(trimmed)) {
      body.push(`\\pard\\sb120\\sa60{\\b\\fs24 ${formatInline(trimmed.replace(/^##\s+/, ''))}}\\par`);
      continue;
    }
    const bulletMatch = trimmed.match(/^[-*]\s+(.*)$/);
    if (bulletMatch) {
      body.push(`\\pard\\li360\\sa60\\bullet\\tab ${formatInline(bulletMatch[1])}\\par`);
      continue;
    }
    body.push(`\\pard\\sa80\\sl260\\slmult1 ${formatInline(trimmed)}\\par`);
  }

  return [
    '{\\rtf1\\ansi\\deff0',
    '{\\fonttbl{\\f0\\fswiss Segoe UI;}{\\f1\\fmodern Consolas;}}',
    '\\fs19',
    ...body,
    '}'
  ].join('\n');
}

const licenseMdPath = path.join(rootDir, 'LICENSE.md');
const licenseRtfPath = path.join(rootDir, 'scripts', 'LICENSE.rtf');
if (fs.existsSync(licenseMdPath)) {
  const md = fs.readFileSync(licenseMdPath, 'utf-8');
  fs.writeFileSync(licenseRtfPath, markdownToRtf(md), 'utf-8');
  console.log('[BUILD-INSTALLER] Generated formatted scripts/LICENSE.rtf from LICENSE.md');
}

// 3. Find Inno Setup compiler (ISCC.exe)
const isccCandidates = [
  'iscc',
  'ISCC.exe',
  'C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe',
  'C:\\Program Files\\Inno Setup 6\\ISCC.exe',
  'C:\\Program Files (x86)\\Inno Setup 5\\ISCC.exe',
  ...(process.env.LOCALAPPDATA ? [
    path.join(process.env.LOCALAPPDATA, 'Programs', 'Inno Setup 6', 'ISCC.exe'),
    path.join(process.env.LOCALAPPDATA, 'Programs', 'Inno Setup 5', 'ISCC.exe')
  ] : [])
];

let foundIscc = null;
for (const p of isccCandidates) {
  if (path.isAbsolute(p)) {
    if (fs.existsSync(p)) {
      foundIscc = p;
      break;
    }
  } else {
    try {
      const check = execSync(`where ${p}`, { stdio: 'pipe' }).toString().trim();
      if (check) {
        foundIscc = check.split('\n')[0].trim();
        break;
      }
    } catch {
      // not in path
    }
  }
}

// 4. Candidate paths for NSIS (makensis.exe)
const nsisCandidates = [
  'makensis',
  'makensis.exe',
  'C:\\Program Files (x86)\\NSIS\\makensis.exe',
  'C:\\Program Files\\NSIS\\makensis.exe'
];

let foundNsis = null;
if (!foundIscc) {
  for (const p of nsisCandidates) {
    if (path.isAbsolute(p)) {
      if (fs.existsSync(p)) {
        foundNsis = p;
        break;
      }
    } else {
      try {
        const check = execSync(`where ${p}`, { stdio: 'pipe' }).toString().trim();
        if (check) {
          foundNsis = check.split('\n')[0].trim();
          break;
        }
      } catch {
        // not in path
      }
    }
  }
}

if (foundIscc) {
  const runtimeMode = 'tauri';
  console.log(`[BUILD-INSTALLER] Compiling with Inno Setup 6: ${foundIscc}`);
  console.log(`[BUILD-INSTALLER] Runtime Mode: ${runtimeMode}`);
  console.log(`[BUILD-INSTALLER] Visual Theme: Windows 11 Fluent Dynamic Mode (No retro bevels)`);
  console.log(`[BUILD-INSTALLER] Components: Core App, System Hebrew Fonts, Layout Templates`);
  console.log(`[BUILD-INSTALLER] File Associations: .tok (Document)`);

  const issFile = path.join(rootDir, 'scripts/installer.iss');
  const webview2Candidates = [
    process.env.WEBVIEW2_INSTALLER_PATH,
    path.join(rootDir, 'assets/redist/MicrosoftEdgeWebView2RuntimeInstallerX64.exe'),
    path.join(distDir, 'redist/MicrosoftEdgeWebView2RuntimeInstallerX64.exe'),
  ].filter(Boolean);
  const foundWebView2 = webview2Candidates.find((p) => fs.existsSync(p));

  const runStandard = !buildOffline || buildDual;
  const runOffline = buildOffline || buildDual;

  if (runStandard) {
    execSync(
      `"${foundIscc}" "/DMyAppVersion=${version}" "/DRuntimeMode=${runtimeMode}" "/DOfflineWebView2=0" "${issFile}"`,
      { cwd: rootDir, stdio: 'inherit' }
    );
    console.log(`[BUILD-INSTALLER] SUCCESS! Standard installer created in dist/TypesetOK-v${version}-Setup-x64.exe`);
  }

  if (runOffline) {
    if (!foundWebView2) {
      const msg = 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe not found (checked WEBVIEW2_INSTALLER_PATH, assets/redist/, dist/redist/).';
      if (buildOffline) {
        console.error(`[BUILD-INSTALLER] Error: ${msg}`);
        process.exit(1);
      } else {
        console.warn(`[BUILD-INSTALLER] Skipping offline installer: ${msg}`);
      }
    } else {
      execSync(
        `"${foundIscc}" "/DMyAppVersion=${version}" "/DRuntimeMode=${runtimeMode}" "/DOfflineWebView2=1" "/DWebView2InstallerPath=${foundWebView2}" "${issFile}"`,
        { cwd: rootDir, stdio: 'inherit' }
      );
      console.log(`[BUILD-INSTALLER] SUCCESS! Offline installer created in dist/TypesetOK-v${version}-Setup-Offline-x64.exe`);
    }
  }
} else if (foundNsis) {
  console.log(`[BUILD-INSTALLER] Compiling with NSIS: ${foundNsis}`);
  const nsiFile = path.join(rootDir, 'scripts/installer.nsi');
  execSync(`"${foundNsis}" "/DVERSION=${version}" "${nsiFile}"`, { cwd: rootDir, stdio: 'inherit' });
  console.log(`[BUILD-INSTALLER] SUCCESS! Installer created in dist/TypesetOK-v${version}-NSIS-Setup.exe`);
} else {
  console.log('[BUILD-INSTALLER] Note: Inno Setup (ISCC.exe) and NSIS (makensis.exe) were not found in standard paths.');
  console.log('[BUILD-INSTALLER] Bilingual Installer scripts generated successfully:');
  console.log('  -> scripts/installer.iss (Inno Setup 6 - Hebrew & English)');
  console.log('  -> scripts/installer.nsi (NSIS - Hebrew & English)');
}

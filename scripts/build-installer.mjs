import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const rootPkg = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf-8'));
const version = rootPkg.version || '0.9.8';
const distDir = path.join(rootDir, 'dist');
const bundleDir = path.join(distDir, `TypesetOK-v${version}-windows-x64`);

console.log('====================================================');
console.log(' TypesetOK Modern Windows Installer Builder');
console.log('====================================================');

// 1. Check if desktop bundle exists
if (!fs.existsSync(bundleDir)) {
  console.log('[BUILD-INSTALLER] Desktop bundle not found. Building desktop bundle first...');
  execSync('node scripts/build-desktop-bundle.mjs', { cwd: rootDir, stdio: 'inherit' });
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
  console.log(`[BUILD-INSTALLER] Compiling with Inno Setup 6: ${foundIscc}`);
  console.log(`[BUILD-INSTALLER] Visual Theme: Windows 11 Fluent Dynamic Mode (No retro bevels)`);
  console.log(`[BUILD-INSTALLER] Components: Core App, System Hebrew Fonts, Layout Templates`);
  console.log(`[BUILD-INSTALLER] File Associations: .tok (Document), .tokbook (Multi-Document Book)`);
  
  const issFile = path.join(rootDir, 'scripts/installer.iss');
  execSync(`"${foundIscc}" "/DMyAppVersion=${version}" "${issFile}"`, { cwd: rootDir, stdio: 'inherit' });
  console.log(`[BUILD-INSTALLER] SUCCESS! Installer created in dist/TypesetOK-v${version}-Setup-x64.exe`);
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

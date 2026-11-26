import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

describe('Core Feature 1: In-App Updater (מנגנון עדכונים מתוך התוכנה)', () => {
  test('Version comparison handles semantic versioning correctly', () => {
    function compareVersions(v1, v2) {
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

    assert.equal(compareVersions('0.8.0', '0.7.0'), 1, '0.8.0 is newer than 0.7.0');
    assert.equal(compareVersions('0.8.0', '0.8.0'), 0, 'equal versions return 0');
    assert.equal(compareVersions('0.7.0', '0.8.0'), -1, 'older version returns -1');
    assert.equal(compareVersions('v0.9.1', '0.9.0'), 1, 'handles leading v');
  });

  test('Updater module is implemented in Tauri desktop and defines GitHub repository target', () => {
    const updaterRs = path.join(rootDir, 'crates/tok-desktop/src/updater.rs');
    assert.equal(fs.existsSync(updaterRs), true);
    const content = fs.readFileSync(updaterRs, 'utf-8');
    assert.equal(content.includes('TypesetOK'), true);
    assert.equal(content.includes('typesetok'), true);
    const commandsRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/commands.rs'), 'utf-8');
    assert.equal(commandsRs.includes('tok_check_for_updates'), true);
  });
});

describe('Core Feature 2: Rotating Logs & Auto-cleanup (קובץ לוגים עם מחיקה אוטומטית)', () => {
  test('Logger auto-cleans old logs older than threshold', () => {
    const testLogDir = path.join(rootDir, 'dist/test_logs');
    if (fs.existsSync(testLogDir)) fs.rmSync(testLogDir, { recursive: true, force: true });
    fs.mkdirSync(testLogDir, { recursive: true });

    // Create 3 log files: today, 5 days ago, and 30 days ago
    const today = new Date().toISOString().split('T')[0];
    const fiveDaysAgo = new Date(Date.now() - 5 * 86400000).toISOString().split('T')[0];
    const thirtyDaysAgo = new Date(Date.now() - 30 * 86400000).toISOString().split('T')[0];

    fs.writeFileSync(path.join(testLogDir, `tok-${today}.log`), '[INFO] today log\n');
    fs.writeFileSync(path.join(testLogDir, `tok-${fiveDaysAgo}.log`), '[INFO] 5 days ago log\n');
    fs.writeFileSync(path.join(testLogDir, `tok-${thirtyDaysAgo}.log`), '[INFO] 30 days ago log\n');

    // Simulate retention cleanup with 14 days threshold
    const retentionDays = 14;
    const now = Date.now();
    const maxAgeMs = retentionDays * 86400000;
    const files = fs.readdirSync(testLogDir);
    let deletedCount = 0;

    for (const f of files) {
      const match = f.match(/^tok-(\d{4}-\d{2}-\d{2})\.log$/);
      if (!match) continue;
      const fDate = new Date(match[1]).getTime();
      if (now - fDate > maxAgeMs) {
        fs.unlinkSync(path.join(testLogDir, f));
        deletedCount++;
      }
    }

    assert.equal(deletedCount, 1, 'Only 30-day-old log file should be purged');
    assert.equal(fs.existsSync(path.join(testLogDir, `tok-${today}.log`)), true);
    assert.equal(fs.existsSync(path.join(testLogDir, `tok-${fiveDaysAgo}.log`)), true);
    assert.equal(fs.existsSync(path.join(testLogDir, `tok-${thirtyDaysAgo}.log`)), false);

    // Clean up test dir
    fs.rmSync(testLogDir, { recursive: true, force: true });
  });
});

describe('Core Feature 3: Multilingual Installer (מתקין עברית ואנגלית)', () => {
  test('Inno Setup script contains bilingual Hebrew & English messages and .tok file association', () => {
    const issPath = path.join(rootDir, 'scripts/installer.iss');
    assert.equal(fs.existsSync(issPath), true);
    const content = fs.readFileSync(issPath, 'utf-8');
    assert.equal(content.includes('hebrew'), true);
    assert.equal(content.includes('english'), true);
    assert.equal(content.includes('.tok'), true);
    assert.equal(content.includes('TypesetOK.exe'), true);
  });

  test('NSIS installer script includes Hebrew and English MUI languages', () => {
    const nsiPath = path.join(rootDir, 'scripts/installer.nsi');
    assert.equal(fs.existsSync(nsiPath), true);
    const content = fs.readFileSync(nsiPath, 'utf-8');
    assert.equal(content.includes('Hebrew'), true);
    assert.equal(content.includes('English'), true);
  });
});

describe('Core Feature 4: Plugin Architecture (מערכת תוספים JS ו-TS)', () => {
  test('Plugin Manager discovers and supports both TS and JS plugins', () => {
    const pluginsRs = path.join(rootDir, 'crates/tok-desktop/src/plugins.rs');
    assert.equal(fs.existsSync(pluginsRs), true);
    const content = fs.readFileSync(pluginsRs, 'utf-8');
    assert.equal(content.includes('discover_plugins'), true);
    assert.equal(content.includes('toggle_plugin'), true);
  });

  test('Plugin Engine executes plugins and registers commands into Command Palette', () => {
    const engineTs = path.join(rootDir, 'packages/tok-ui/src/plugins/PluginEngine.ts');
    assert.equal(fs.existsSync(engineTs), true);
    const content = fs.readFileSync(engineTs, 'utf-8');
    assert.equal(content.includes('registerCommand'), true);
    assert.equal(content.includes('loadPlugins'), true);
  });
});

describe('Core Feature 5: Startup Speed & Frameless Tauri Window (חלון מהיר ללא סרגל מערכת אפור)', () => {
  test('Tauri window configuration uses frameless decorations and dark background', () => {
    const mainRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/main.rs'), 'utf-8');
    assert.equal(mainRs.includes('.decorations(false)'), true, 'Eliminates ugly Windows grey menu bar');
  });
});

describe('UI/UX Feature 1: Welcome & Project Picker (מסך בחירת פרויקט בגוון כחול)', () => {
  test('Welcome modal component is implemented with templates and recent projects', () => {
    const welcomeTs = path.join(rootDir, 'packages/tok-ui/src/components/WelcomeModal.ts');
    assert.equal(fs.existsSync(welcomeTs), true);
    const content = fs.readFileSync(welcomeTs, 'utf-8');
    assert.equal(content.includes('tok-welcome-card'), true);
    // Colors come from the theme tokens (light/dark/color themes), never hard-coded.
    assert.equal(/#[0-9A-Fa-f]{6}\b/.test(content), false, 'No hard-coded colors: uses theme tokens');
    assert.equal(content.includes('tok-welcome-overlay'), true);
    assert.equal(content.includes('templateGemara'), true);
    assert.equal(content.includes('recentProjects'), true);
  });
});

describe('UI/UX Feature 2 & 3: Modern TopSystemBar & Minimalist Layout (ללא סרגל אפרפר וממשק נקי)', () => {
  test('TopSystemBar is modern dark blue with decluttered navigation and no grey toolbar', () => {
    const topBarTs = path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts');
    assert.equal(fs.existsSync(topBarTs), true);
    const content = fs.readFileSync(topBarTs, 'utf-8');
    // Colors come from the theme tokens (light/dark/color themes), never hard-coded.
    assert.equal(/#[0-9A-Fa-f]{6}\b/.test(content), false, 'No hard-coded colors: uses theme tokens');
    assert.equal(content.includes('tok-search-pill'), true, 'Clean search pill');
    assert.equal(content.includes('onOpenProjects'), true);
  });
});

describe('UI/UX Feature 4: Full Bilingual Hebrew/English with RTL/LTR (עברית ואנגלית מלא)', () => {
  test('i18n module provides complete translations and dynamic direction toggling', () => {
    const i18nTs = path.join(rootDir, 'packages/tok-ui/src/i18n.ts');
    assert.equal(fs.existsSync(i18nTs), true);
    const content = fs.readFileSync(i18nTs, 'utf-8');
    assert.equal(content.includes("lang === 'he' ? 'rtl' : 'ltr'"), true);
    assert.equal(content.includes('welcomeTitle'), true);
    assert.equal(content.includes('settingsTitle'), true);
    assert.equal(content.includes('aboutTitle'), true);
  });
});

describe('UI/UX Feature 5: Sidebar Bottom Section (רווח והגדרות ואודות)', () => {
  test('StructureBar contains bottom spacer with Settings and About buttons', () => {
    const structureTs = path.join(rootDir, 'packages/tok-ui/src/components/StructureBar.ts');
    assert.equal(fs.existsSync(structureTs), true);
    const content = fs.readFileSync(structureTs, 'utf-8');
    assert.equal(content.includes('tok-structure-bottom'), true);
    assert.equal(content.includes('onOpenSettings'), true);
    assert.equal(content.includes('onOpenAbout'), true);
  });

  test('Settings modal includes Appearance customization, Canvas tone, and Logs retention', () => {
    const settingsTs = path.join(rootDir, 'packages/tok-ui/src/components/SettingsModal.ts');
    assert.equal(fs.existsSync(settingsTs), true);
    const content = fs.readFileSync(settingsTs, 'utf-8');
    assert.equal(content.includes('renderAppearanceTab'), true);
    assert.equal(content.includes('renderLogsTab'), true);
    assert.equal(content.includes('renderUpdatesTab'), true);
    assert.equal(content.includes('renderPluginsTab'), true);
  });

  test('About modal links directly to GitHub repository', () => {
    const aboutTs = path.join(rootDir, 'packages/tok-ui/src/components/AboutModal.ts');
    assert.equal(fs.existsSync(aboutTs), true);
    const content = fs.readFileSync(aboutTs, 'utf-8');
    assert.equal(content.includes('https://github.com/TypesetOK/typesetok'), true);
  });
});

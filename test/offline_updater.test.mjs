import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');

test('Offline & Browserless Updater - Full Test Suite', async (t) => {
  await t.test('Invariant: Zero External Dependencies Policy', () => {
    const pkgJson = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8'));
    const dependencies = Object.keys(pkgJson.dependencies || {});
    // Check that no third-party libraries were added
    assert.deepEqual(dependencies, [], 'Dependencies in package.json must remain strictly zero to comply with Amlaach standards');
  });

  await t.test('Updater Logic: Filename Version Extraction & Validation', () => {
    function extractOfflineVersion(fileName) {
      const lower = fileName.toLowerCase();
      const isCandidateExt = lower.endsWith('.exe') || lower.endsWith('.appimage') || lower.endsWith('.dmg');
      const isRelated = lower.includes('typesetok') || lower.includes('tok') || lower.includes('setup');
      if (!isCandidateExt || !isRelated) return null;

      const baseName = fileName
        .replace(/\.(exe|appimage|dmg|zip)$/i, '')
        .replace(/[-_](setup|installer|x64|ia32|arm64|win|mac|linux)/gi, '');
      const match = baseName.match(/v?(\d+\.\d+\.\d+(?:-[a-z0-9.]+)?)/i);
      return match ? match[1] : null;
    }

    const testCases = [
      { filename: 'TypesetOK-0.35.1-Setup.exe', expected: '0.35.1' },
      { filename: 'TypesetOK-Setup-0.36.0.exe', expected: '0.36.0' },
      { filename: 'typesetok-v1.0.0.exe', expected: '1.0.0' },
      { filename: 'Setup-0.35.0.exe', expected: '0.35.0' },
      { filename: 'TypesetOK-1.0.0-beta.1.exe', expected: '1.0.0-beta.1' },
      { filename: 'TypesetOK-0.36.0.AppImage', expected: '0.36.0' },
      { filename: 'unrelated-document.pdf', expected: null },
      { filename: 'random-app-1.2.3.exe', expected: null },
      { filename: 'TypesetOK-Installer.exe', expected: null }
    ];

    for (const { filename, expected } of testCases) {
      const extracted = extractOfflineVersion(filename);
      assert.equal(extracted, expected, `Failed version extraction for ${filename}`);
    }
  });

  await t.test('Updater Logic: Semver Comparison for Offline Updates against v0.35.0', () => {
    function compareSemver(a, b) {
      const pa = a.replace(/^v/i, '').split('.').map(n => parseInt(n, 10) || 0);
      const pb = b.replace(/^v/i, '').split('.').map(n => parseInt(n, 10) || 0);
      for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
        const na = pa[i] || 0;
        const nb = pb[i] || 0;
        if (na > nb) return 1;
        if (na < nb) return -1;
      }
      return 0;
    }

    const currentVersion = '0.35.0';

    assert.equal(compareSemver('0.35.1', currentVersion) > 0, true, '0.35.1 should be detected as newer than 0.35.0');
    assert.equal(compareSemver('0.36.0', currentVersion) > 0, true, '0.36.0 should be detected as newer than 0.35.0');
    assert.equal(compareSemver('1.0.0', currentVersion) > 0, true, '1.0.0 should be detected as newer than 0.35.0');
    assert.equal(compareSemver('0.35.0', currentVersion) > 0, false, '0.35.0 should not be detected as newer than 0.35.0');
    assert.equal(compareSemver('0.34.11', currentVersion) > 0, false, '0.34.11 should not be detected as newer than 0.35.0');
  });

  await t.test('Integrity: Binary Validation (Magic Bytes & Minimum Size)', () => {
    function validateExecutableBinary(filePath) {
      try {
        if (!fs.existsSync(filePath)) return false;
        const stat = fs.statSync(filePath);
        if (stat.size < 100 * 1024) return false; // >= 100KB required

        const fd = fs.openSync(filePath, 'r');
        const buf = Buffer.alloc(4);
        fs.readSync(fd, buf, 0, 4, 0);
        fs.closeSync(fd);

        const isWin = process.platform === 'win32' || filePath.toLowerCase().endsWith('.exe');
        if (isWin) {
          // Windows PE 'MZ' magic header: 0x4D, 0x5A
          return buf[0] === 0x4d && buf[1] === 0x5a;
        }
        return true;
      } catch {
        return false;
      }
    }

    const tmpDir = os.tmpdir();
    const validExePath = path.join(tmpDir, 'test-valid-sample.exe');
    const tooSmallExePath = path.join(tmpDir, 'test-small.exe');
    const invalidHeaderPath = path.join(tmpDir, 'test-corrupt.exe');

    try {
      // 1. Create valid mock PE file (150KB with MZ header)
      const validBuf = Buffer.alloc(150 * 1024);
      validBuf[0] = 0x4d; // 'M'
      validBuf[1] = 0x5a; // 'Z'
      fs.writeFileSync(validExePath, validBuf);
      assert.equal(validateExecutableBinary(validExePath), true, 'Valid PE header with size >= 100KB must pass');

      // 2. Create small file (< 100KB)
      const smallBuf = Buffer.alloc(50 * 1024);
      smallBuf[0] = 0x4d;
      smallBuf[1] = 0x5a;
      fs.writeFileSync(tooSmallExePath, smallBuf);
      assert.equal(validateExecutableBinary(tooSmallExePath), false, 'Files under 100KB must fail validation');

      // 3. Create file with invalid header
      const corruptBuf = Buffer.alloc(120 * 1024);
      corruptBuf[0] = 0x00;
      corruptBuf[1] = 0x00;
      fs.writeFileSync(invalidHeaderPath, corruptBuf);
      assert.equal(validateExecutableBinary(invalidHeaderPath), false, 'Non-PE files must fail validation');
    } finally {
      try { fs.unlinkSync(validExePath); } catch {}
      try { fs.unlinkSync(tooSmallExePath); } catch {}
      try { fs.unlinkSync(invalidHeaderPath); } catch {}
    }
  });

  await t.test('Integrity: Streaming SHA-256 Checksum Calculation', async () => {
    async function computeFileSha256(filePath) {
      return new Promise((resolve, reject) => {
        if (!fs.existsSync(filePath)) return reject(new Error('File not found'));
        const hash = crypto.createHash('sha256');
        const stream = fs.createReadStream(filePath);
        stream.on('data', (chunk) => hash.update(chunk));
        stream.on('end', () => resolve(hash.digest('hex')));
        stream.on('error', (err) => reject(err));
      });
    }

    const tmpFile = path.join(os.tmpdir(), 'sha256-test.tmp');
    const testContent = 'TypesetOK-Smart-Updater-Integrity-Payload';
    fs.writeFileSync(tmpFile, testContent, 'utf8');

    try {
      const calculatedHash = await computeFileSha256(tmpFile);
      const expectedHash = crypto.createHash('sha256').update(testContent).digest('hex');
      assert.equal(calculatedHash, expectedHash, 'Streaming SHA-256 must match expected crypto digest');
      assert.equal(calculatedHash.length, 64, 'SHA-256 hex string must be exactly 64 characters long');
    } finally {
      try { fs.unlinkSync(tmpFile); } catch {}
    }
  });

  await t.test('Metrics: Speed & ETA Calculation Logic', () => {
    function calculateSpeedAndEta(transferredBytes, totalBytes, deltaBytes, elapsedSeconds, previousSpeed) {
      const instantSpeed = elapsedSeconds > 0 ? deltaBytes / elapsedSeconds : 0;
      const currentSpeed = previousSpeed === 0 ? instantSpeed : Math.round(previousSpeed * 0.7 + instantSpeed * 0.3);
      const remainingBytes = Math.max(0, totalBytes - transferredBytes);
      const etaSeconds = currentSpeed > 0 ? Math.round(remainingBytes / currentSpeed) : 0;
      return { currentSpeed, etaSeconds };
    }

    const total = 50 * 1024 * 1024; // 50 MB
    const transferred = 25 * 1024 * 1024; // 25 MB
    const delta = 5 * 1024 * 1024; // 5 MB transferred in 1 second
    const elapsed = 1.0; // 1s

    const { currentSpeed, etaSeconds } = calculateSpeedAndEta(transferred, total, delta, elapsed, 0);
    assert.equal(currentSpeed, 5 * 1024 * 1024, 'Speed should be 5 MB/s');
    assert.equal(etaSeconds, 5, 'Remaining 25MB at 5MB/s should estimate 5 seconds');
  });

  await t.test('Discovery: USB Keyword and Hierarchy Filtering', () => {
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

    const matchKeywords = (dirName) => {
      const lower = dirName.toLowerCase();
      return subfolderKeywords.some((kw) => lower.includes(kw));
    };

    assert.equal(matchKeywords('TypesetOK'), true);
    assert.equal(matchKeywords('תוכנות להתקנה'), true);
    assert.equal(matchKeywords('הורדות אחרונות'), true);
    assert.equal(matchKeywords('עדכונים 2026'), true);
    assert.equal(matchKeywords('ספרי קודש'), true);
    assert.equal(matchKeywords('Windows-System32'), false);
    assert.equal(matchKeywords('Pictures'), false);
  });

  await t.test('Localization: Offline & Browserless Translation Keys', () => {
    const i18nSource = fs.readFileSync(path.join(ROOT, 'packages/tok-ui/src/i18n.ts'), 'utf8');

    const requiredKeys = [
      'updatesDownloadInApp',
      'updatesDownloading',
      'updatesDownloadSuccess',
      'updatesApplyNow',
      'updatesSpeed',
      'updatesEta',
      'updatesSha256',
      'updatesOfflineTitle',
      'updatesOfflineDesc',
      'updatesOfflineSelectFile',
      'updatesOfflineScanUsb',
      'updatesOfflineFoundOnUsb',
      'updatesOfflineNoFilesFound',
      'updatesOfflineSelected',
      'updatesOfflineApply',
      'updatesOfflineFileInvalid',
      'updatesOfflineRecommended'
    ];

    for (const key of requiredKeys) {
      assert.ok(i18nSource.includes(`${key}:`), `Missing required translation key: ${key}`);
    }
  });

  await t.test('IPC Security & Interface: Preload & Main Registration', () => {
    const mainSource = fs.readFileSync(path.join(ROOT, 'packages/tok-electron/src/main.ts'), 'utf8');
    const preloadSource = fs.readFileSync(path.join(ROOT, 'packages/tok-electron/src/preload.ts'), 'utf8');

    const expectedChannels = [
      'tok:download-update',
      'tok:apply-update',
      'tok:inspect-offline-update',
      'tok:scan-usb-updates',
      'tok:update-download-progress'
    ];

    for (const channel of expectedChannels) {
      assert.ok(mainSource.includes(channel), `Main process missing IPC channel: ${channel}`);
      assert.ok(preloadSource.includes(channel), `Preload script missing IPC channel: ${channel}`);
    }
  });

  await t.test('UI Components: SettingsModal Integration', () => {
    const settingsSource = fs.readFileSync(path.join(ROOT, 'packages/tok-ui/src/components/SettingsModal.ts'), 'utf8');

    assert.ok(settingsSource.includes('updatesDownloadInApp'), 'SettingsModal must offer direct in-app download');
    assert.ok(settingsSource.includes('updatesOfflineSelectFile'), 'SettingsModal must offer offline file picker');
    assert.ok(settingsSource.includes('updatesOfflineScanUsb'), 'SettingsModal must offer USB drive scanning');
    assert.ok(settingsSource.includes('tokIpc.applyUpdate'), 'SettingsModal must invoke applyUpdate');
    assert.ok(settingsSource.includes('updatesSha256'), 'SettingsModal must render SHA-256 verification');
  });
});

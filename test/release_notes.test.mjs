import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import {
  normalizeTag,
  generateReleaseNotesForTag
} from '../scripts/generate-release-notes.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const changelogPath = path.join(rootDir, 'CHANGELOG.md');
const scriptPath = path.join(rootDir, 'scripts', 'generate-release-notes.mjs');
const emojiRegex = /[\u{1F300}-\u{1F5FF}\u{1F600}-\u{1F64F}\u{1F680}-\u{1F6FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F900}-\u{1F9FF}\u{1FA70}-\u{1FAFF}]/u;

describe('Release Notes Generator', () => {
  const realChangelog = fs.readFileSync(changelogPath, 'utf8');

  test('normalizeTag accepts with or without v prefix', () => {
    assert.deepEqual(normalizeTag('v0.36.6'), { tag: 'v0.36.6', version: '0.36.6' });
    assert.deepEqual(normalizeTag('0.36.6'), { tag: 'v0.36.6', version: '0.36.6' });
    assert.throws(() => normalizeTag('   '));
  });

  test('generates accurate notes for v0.36.6 without stale PR #30 / Issue #28 text or emojis', () => {
    const result = generateReleaseNotesForTag('v0.36.6', realChangelog);

    assert.equal(result.found, true);
    assert.equal(result.tag, 'v0.36.6');
    assert.equal(result.version, '0.36.6');
    assert.ok(result.body.includes('## TypesetOK (TOK) v0.36.6'));
    assert.ok(result.body.includes('האצת עימוד חי'));
    assert.ok(result.body.includes('TypesetOK-v0.36.6-Setup.exe'));
    assert.ok(result.body.includes('TypesetOK-v0.36.6-windows-desktop-app.zip'));
    assert.ok(result.body.includes('tok-cli-v0.36.6-windows-x64.zip'));
    assert.ok(result.body.includes('typesetok-desktop-workbench-v0.36.6.tar.gz'));
    assert.equal(result.body.includes('PR #30'), false);
    assert.equal(result.body.includes('Issue #28'), false);
    assert.equal(emojiRegex.test(result.body), false, 'Generated release notes must not contain emojis');
  });

  test('generates highlights and categorized changes for bare version 0.34.9', () => {
    const result = generateReleaseNotesForTag('0.34.9', realChangelog);

    assert.equal(result.found, true);
    assert.equal(result.tag, 'v0.34.9');
    assert.ok(result.body.includes('## TypesetOK (TOK) v0.34.9'));
    assert.ok(result.body.includes('### עיקרי הגרסה'));
    assert.ok(result.body.includes('### מה חדש בגרסה v0.34.9'));
    assert.ok(result.body.includes('#### תכונות חדשות'));
    assert.ok(result.body.includes('#### שיפורים'));
    assert.ok(result.body.includes('#### תיקוני באגים'));
    assert.ok(result.body.includes('TypesetOK-v0.34.9-Setup.exe'));
    assert.equal(emojiRegex.test(result.body), false);
  });

  test('falls back cleanly for missing version v9.9.9 without leaking other versions', () => {
    const result = generateReleaseNotesForTag('v9.9.9', realChangelog);

    assert.equal(result.found, false);
    assert.ok(result.body.includes('## TypesetOK (TOK) v9.9.9'));
    assert.ok(result.body.includes('TypesetOK-v9.9.9-Setup.exe'));
    assert.equal(result.body.includes('0.36.6'), false);
    assert.equal(result.body.includes('PR #30'), false);
    assert.equal(emojiRegex.test(result.body), false);
  });

  test('CLI writes release notes file, github output flag, and enforces --strict', () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tok-release-notes-test-'));
    try {
      const outFile = path.join(tmpDir, 'notes.md');
      const ghOutFile = path.join(tmpDir, 'github_output.txt');

      execFileSync(
        process.execPath,
        [scriptPath, 'v0.36.6', '--out', outFile, '--github-output', ghOutFile],
        { cwd: rootDir, encoding: 'utf8' }
      );

      const written = fs.readFileSync(outFile, 'utf8');
      const ghOut = fs.readFileSync(ghOutFile, 'utf8');
      assert.ok(written.includes('## TypesetOK (TOK) v0.36.6'));
      assert.ok(written.includes('TypesetOK-v0.36.6-Setup.exe'));
      assert.match(ghOut, /has_changelog_entry=true/);

      assert.throws(() => {
        execFileSync(
          process.execPath,
          [scriptPath, 'v9.9.9', '--strict'],
          { cwd: rootDir, stdio: 'pipe' }
        );
      });
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});

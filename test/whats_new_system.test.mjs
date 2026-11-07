import { test, describe, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

import { CHANGELOG_ENTRIES } from '../packages/tok-ui/dist/updates/changelogData.js';
import { UpdatesStateManager, compareSemver } from '../packages/tok-ui/dist/updates/updatesManager.js';
import { parseChangelogMarkdown } from '../scripts/sync-changelog.mjs';
import { strings } from '../packages/tok-ui/dist/i18n.js';

describe("What's New System - Full Test Suite", () => {
  beforeEach(() => {
    UpdatesStateManager.resetInstance();
  });

  describe('Changelog Registry & Data Integrity', () => {
    test('changelog entries exist and are ordered descending by semver', () => {
      assert.ok(CHANGELOG_ENTRIES.length >= 3, 'Should have at least 3 historical releases');

      for (let i = 0; i < CHANGELOG_ENTRIES.length - 1; i++) {
        const vCurr = CHANGELOG_ENTRIES[i].version;
        const vNext = CHANGELOG_ENTRIES[i + 1].version;
        assert.ok(
          compareSemver(vCurr, vNext) >= 0,
          `Version ${vCurr} should be >= ${vNext}`
        );
      }
    });

    test('every entry has complete Hebrew and English metadata', () => {
      for (const entry of CHANGELOG_ENTRIES) {
        assert.ok(entry.version, 'Entry must have a version');
        assert.match(entry.releaseDate, /^\d{4}-\d{2}-\d{2}$/, 'Release date must be YYYY-MM-DD');
        assert.ok(['major', 'minor', 'patch'].includes(entry.importance), 'Valid importance');

        // Bilingual title and summary
        assert.ok(entry.title?.he && entry.title.he.trim().length > 0, `Hebrew title missing in v${entry.version}`);
        assert.ok(entry.title?.en && entry.title.en.trim().length > 0, `English title missing in v${entry.version}`);
        assert.ok(entry.summary?.he && entry.summary.he.trim().length > 0, `Hebrew summary missing in v${entry.version}`);
        assert.ok(entry.summary?.en && entry.summary.en.trim().length > 0, `English summary missing in v${entry.version}`);

        // Changes
        assert.ok(Array.isArray(entry.changes) && entry.changes.length > 0, `Changes missing in v${entry.version}`);
        for (const change of entry.changes) {
          assert.ok(change.category, 'Category must be defined');
          assert.ok(change.text?.he, 'Hebrew text must be defined');
          assert.ok(change.text?.en, 'English text must be defined');
        }

        // Major releases must contain visual highlight cards
        if (entry.importance === 'major') {
          assert.ok(Array.isArray(entry.highlights) && entry.highlights.length > 0, `Major release v${entry.version} must have highlights`);
          for (const hl of entry.highlights) {
            assert.ok(hl.title?.he && hl.title?.en, 'Highlight must have bilingual title');
            assert.ok(hl.description?.he && hl.description?.en, 'Highlight must have bilingual description');
            assert.ok(hl.icon, 'Highlight must specify an icon');
          }
        }
      }
    });
  });

  describe('CHANGELOG.md Markdown Source-of-Truth & Parser', () => {
    test('CHANGELOG.md exists in project root and parses cleanly', () => {
      const changelogFile = path.join(rootDir, 'CHANGELOG.md');
      assert.ok(fs.existsSync(changelogFile), 'CHANGELOG.md must exist in root');

      const content = fs.readFileSync(changelogFile, 'utf8');
      const parsed = parseChangelogMarkdown(content);

      assert.ok(parsed.length >= 3, 'Must parse at least 3 releases');
      const v0304 = parsed.find(e => e.version === '0.30.4');
      assert.ok(v0304, '0.30.4 must be parsed');
      const v0300 = parsed.find(e => e.version === '0.30.0');
      assert.ok(v0300, '0.30.0 must be parsed');
      assert.equal(v0300.importance, 'major');
      assert.ok(v0300.highlights.length >= 4, 'Major release must parse highlights');
    });

    test('Strict No-Emoji & No-Vibe-Coding Policy in changelog and UI text', () => {
      const emojiRegex = /[\u{1F300}-\u{1F5FF}\u{1F600}-\u{1F64F}\u{1F680}-\u{1F6FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F900}-\u{1F9FF}\u{1FA70}-\u{1FAFF}]/u;

      const changelogContent = fs.readFileSync(path.join(rootDir, 'CHANGELOG.md'), 'utf8');
      assert.equal(emojiRegex.test(changelogContent), false, 'CHANGELOG.md must NOT contain any emojis (official vector icons only)');

      const dataContent = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/updates/changelogData.ts'), 'utf8');
      assert.equal(emojiRegex.test(dataContent), false, 'changelogData.ts must NOT contain any emojis');
    });
  });

  describe('Semver Comparison Logic', () => {
    test('compares standard versions correctly', () => {
      assert.equal(compareSemver('0.30.4', '0.30.0'), 1);
      assert.equal(compareSemver('0.30.0', '0.30.4'), -1);
      assert.equal(compareSemver('0.30.0', '0.30.0'), 0);
    });

    test('handles v prefix and varying component lengths', () => {
      assert.equal(compareSemver('v1.0.0', '0.9.9'), 1);
      assert.equal(compareSemver('0.30', '0.30.0'), 0);
      assert.equal(compareSemver('1.2.3', 'v1.2.3'), 0);
    });
  });

  describe('UpdatesStateManager Business Rules', () => {
    const mockEntries = [
      {
        version: '1.1.0',
        releaseDate: '2026-11-01',
        importance: 'minor',
        title: { he: 'עדכון קל', en: 'Minor Update' },
        summary: { he: 'תיקונים ושיפורים קטנים', en: 'Small fixes and tweaks' },
        changes: [
          { category: 'improvements', text: { he: 'שיפור ביצועים', en: 'Performance tweak' } },
          { category: 'fixes', text: { he: 'תיקון באג', en: 'Bug fix' } }
        ]
      },
      {
        version: '1.0.0',
        releaseDate: '2026-10-15',
        importance: 'major',
        title: { he: 'גרסה ראשית 1.0', en: 'Major Release 1.0' },
        summary: { he: 'מהפכה במערכת', en: 'Massive milestone' },
        highlights: [
          {
            id: 'engine',
            title: { he: 'מנוע חדש', en: 'New Engine' },
            description: { he: 'מהיר', en: 'Fast' },
            icon: 'sparkle'
          }
        ],
        changes: [
          { category: 'features', text: { he: 'פיצ׳ר ענק', en: 'Huge feature' } },
          { category: 'typography', text: { he: 'ניקוד מדויק', en: 'Precise niqqud' } }
        ]
      },
      {
        version: '0.9.0',
        releaseDate: '2026-09-01',
        importance: 'minor',
        title: { he: 'גרסה ישנה', en: 'Old version' },
        summary: { he: 'בסיס', en: 'Base' },
        changes: [
          { category: 'features', text: { he: 'בסיס', en: 'Base' } }
        ]
      }
    ];

    test('shouldPromptMajorUpdate triggers only for unread major releases', () => {
      const manager = UpdatesStateManager.getInstance(mockEntries);

      // 1.0.0 is major, hasn't been seen -> true
      assert.equal(manager.shouldPromptMajorUpdate('1.0.0'), true);

      // 1.1.0 is minor -> false (minor updates do NOT trigger popup!)
      assert.equal(manager.shouldPromptMajorUpdate('1.1.0'), false);

      // After user sees 1.0.0 -> should no longer prompt
      manager.markVersionAsSeen('1.0.0');
      assert.equal(manager.shouldPromptMajorUpdate('1.0.0'), false);

      // Dismissing major prompt prevents prompt
      UpdatesStateManager.resetInstance();
      const manager2 = UpdatesStateManager.getInstance(mockEntries);
      assert.equal(manager2.shouldPromptMajorUpdate('1.0.0'), true);
      manager2.dismissMajorPrompt('1.0.0');
      assert.equal(manager2.shouldPromptMajorUpdate('1.0.0'), false);
    });

    test('hasUnreadMinorUpdates identifies unread minor releases without prompting popup', () => {
      const manager = UpdatesStateManager.getInstance(mockEntries);

      // Initially user has seen 0.0.0, so 1.1.0 is unread
      assert.equal(manager.hasUnreadMinorUpdates('1.1.0'), true);

      // 1.0.0 is major, so hasUnreadMinorUpdates should return false for it
      assert.equal(manager.hasUnreadMinorUpdates('1.0.0'), false);

      // Once marked as seen, 1.1.0 is no longer unread
      manager.markVersionAsSeen('1.1.0');
      assert.equal(manager.hasUnreadMinorUpdates('1.1.0'), false);
    });

    test('filterEntries accurately filters by importance, category and query', () => {
      const manager = UpdatesStateManager.getInstance(mockEntries);

      // Filter by importance
      const majorEntries = manager.filterEntries({ importance: 'major' });
      assert.equal(majorEntries.length, 1);
      assert.equal(majorEntries[0].version, '1.0.0');

      // Filter by category
      const typoEntries = manager.filterEntries({ category: 'typography' });
      assert.equal(typoEntries.length, 1);
      assert.equal(typoEntries[0].version, '1.0.0');

      // Filter by textual query in Hebrew
      const searchHeb = manager.filterEntries({ searchQuery: 'ביצועים' });
      assert.equal(searchHeb.length, 1);
      assert.equal(searchHeb[0].version, '1.1.0');

      // Filter by textual query in English
      const searchEng = manager.filterEntries({ searchQuery: 'Huge feature' });
      assert.equal(searchEng.length, 1);
      assert.equal(searchEng[0].version, '1.0.0');
    });
  });

  describe('Localization & i18n Key Coverage', () => {
    test('all What\'s New translation keys exist and are non-empty in Hebrew and English', () => {
      const requiredKeys = [
        'whatsNewTitle',
        'whatsNewSubtitle',
        'whatsNewMajorTitle',
        'whatsNewMajorSubtitle',
        'whatsNewBadgeMajor',
        'whatsNewBadgeMinor',
        'whatsNewBadgePatch',
        'whatsNewFilterAll',
        'whatsNewFilterMajor',
        'whatsNewFilterMinor',
        'whatsNewFilterFixes',
        'whatsNewFilterTypography',
        'whatsNewSearchPlaceholder',
        'whatsNewExploreAll',
        'whatsNewGotIt',
        'whatsNewDontShowAgain',
        'whatsNewViewHistory',
        'whatsNewCheckUpdates',
        'whatsNewNoResults',
        'whatsNewCatFeatures',
        'whatsNewCatImprovements',
        'whatsNewCatFixes',
        'whatsNewCatTypography',
        'whatsNewCatPerformance',
        'whatsNewUnreadAria',
        'menuWhatsNew',
        'commandPaletteWhatsNew'
      ];

      for (const k of requiredKeys) {
        assert.ok(strings[k], `Missing i18n key: ${k}`);
        assert.ok(strings[k].he && strings[k].he.trim().length > 0, `Key ${k} must have Hebrew string`);
        assert.ok(strings[k].en && strings[k].en.trim().length > 0, `Key ${k} must have English string`);
      }
    });
  });

  describe('UI Component Integration & Wiring', () => {
    test('UpdatesDrawer source defines side panel with filter chips and search', () => {
      const drawerSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/UpdatesDrawer.ts'), 'utf8');
      assert.ok(drawerSrc.includes('tok-updates-drawer'), 'Must declare drawer class');
      assert.ok(drawerSrc.includes('tok-filter-chip'), 'Must support filter chips');
      assert.ok(drawerSrc.includes('tok-updates-search-input'), 'Must support search input');
      assert.ok(drawerSrc.includes('populateCards'), 'Must have card populating logic');
    });

    test('MajorUpdateModal source defines popup dialog with highlights grid and dismissal checkbox', () => {
      const modalSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/MajorUpdateModal.ts'), 'utf8');
      assert.ok(modalSrc.includes('tok-major-dialog'), 'Must declare major dialog class');
      assert.ok(modalSrc.includes('tok-major-showcase-grid'), 'Must include showcase grid');
      assert.ok(modalSrc.includes('dontShowAgainCheckbox'), 'Must include dont-show-again checkbox');
      assert.ok(modalSrc.includes('dismissMajorPrompt'), 'Must hook into dismissal logic');
    });

    test('Left sidebar includes unread bell button and TopSystemBar includes quick menu entry', () => {
      const inspSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/ContextualInspector.ts'), 'utf8');
      assert.ok(inspSrc.includes('UpdatesBellButton'), 'Left sidebar must use UpdatesBellButton');
      const barSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf8');
      assert.ok(barSrc.includes('menuWhatsNew'), 'Must declare menuWhatsNew quick action');
      assert.ok(barSrc.includes('open-whats-new'), 'Must dispatch open-whats-new action');
    });

    test('App shell exposes openWhatsNew, openMajorUpdate, and command palette action', () => {
      const appSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf8');
      assert.ok(appSrc.includes('openWhatsNew('), 'Must provide openWhatsNew method');
      assert.ok(appSrc.includes('openMajorUpdate('), 'Must provide openMajorUpdate method');
      assert.ok(appSrc.includes('cmd-whats-new'), 'Must register palette command');
      assert.ok(appSrc.includes('checkStartupUpdatesPrompt'), 'Must check updates prompt on startup');
    });

    test('AboutModal and SettingsModal offer direct links to What\'s New', () => {
      const aboutSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/AboutModal.ts'), 'utf8');
      assert.ok(aboutSrc.includes('whatsNewTitle'), 'AboutModal must link to What\'s New');

      const settingsSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/SettingsModal.ts'), 'utf8');
      assert.ok(settingsSrc.includes('whatsNewViewHistory'), 'SettingsModal updates tab must link to What\'s New history');
    });
  });
});

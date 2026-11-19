import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { parseChangelogMarkdown } from './sync-changelog.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

const CATEGORY_ORDER = [
  { key: 'features', heading: '#### תכונות חדשות' },
  { key: 'typography', heading: '#### טיפוגרפיה ועימוד' },
  { key: 'performance', heading: '#### ביצועים' },
  { key: 'improvements', heading: '#### שיפורים' },
  { key: 'fixes', heading: '#### תיקוני באגים' }
];
const OTHER_CATEGORY_HEADING = '#### שינויים נוספים';

/**
 * Normalizes a release tag or bare version string into `{ tag: 'vX.Y.Z', version: 'X.Y.Z' }`.
 */
export function normalizeTag(rawTag) {
  const trimmed = String(rawTag ?? '').trim();
  if (!trimmed) {
    throw new Error('Release tag is required.');
  }
  const version = trimmed.replace(/^v/i, '');
  if (!version) {
    throw new Error(`Invalid release tag: "${rawTag}"`);
  }
  return {
    tag: `v${version}`,
    version
  };
}

function pickLocalizedText(localized) {
  if (!localized) return '';
  return (localized.he || localized.en || '').trim();
}

function buildDownloadSection(tag) {
  return [
    '### קובצי הורדה',
    '',
    `- **מומלץ – מתקין ל-Windows:** \`TypesetOK-${tag}-Setup.exe\``,
    `- **גרסה ניידת:** \`TypesetOK-${tag}-windows-desktop-app.zip\``,
    `- **ממשק שורת הפקודה (CLI):** \`tok-cli-${tag}-windows-x64.zip\``,
    `- **חבילת קוד המקור:** \`typesetok-desktop-workbench-${tag}.tar.gz\``
  ].join('\n');
}

/**
 * Formats a parsed CHANGELOG entry into Hebrew GitHub release notes Markdown.
 */
export function formatReleaseNotes(entry, rawTag) {
  const { tag } = normalizeTag(rawTag ?? entry?.version);
  const titleText = pickLocalizedText(entry?.title);
  const heading = titleText ? `## TypesetOK (TOK) ${tag} — ${titleText}` : `## TypesetOK (TOK) ${tag}`;
  const sections = [heading];

  const summaryText = pickLocalizedText(entry?.summary);
  if (summaryText) {
    sections.push(summaryText);
  }

  if (Array.isArray(entry?.highlights) && entry.highlights.length > 0) {
    const hlLines = ['### עיקרי הגרסה', ''];
    for (const hl of entry.highlights) {
      const hlTitle = pickLocalizedText(hl.title);
      const hlDesc = pickLocalizedText(hl.description);
      if (hlTitle && hlDesc) {
        hlLines.push(`- **${hlTitle}**: ${hlDesc}`);
      } else if (hlTitle || hlDesc) {
        hlLines.push(`- ${hlTitle || hlDesc}`);
      }
    }
    if (hlLines.length > 2) {
      sections.push(hlLines.join('\n'));
    }
  }

  if (Array.isArray(entry?.changes) && entry.changes.length > 0) {
    const grouped = new Map();
    for (const change of entry.changes) {
      const text = pickLocalizedText(change.text);
      if (!text) continue;
      const cat = String(change.category || '').toLowerCase();
      if (!grouped.has(cat)) {
        grouped.set(cat, []);
      }
      grouped.get(cat).push(text);
    }

    const changeLines = [`### מה חדש בגרסה ${tag}`];
    const knownKeys = new Set(CATEGORY_ORDER.map((c) => c.key));

    for (const { key, heading: subHeading } of CATEGORY_ORDER) {
      const items = grouped.get(key);
      if (!items || items.length === 0) continue;
      changeLines.push('', subHeading);
      for (const item of items) {
        changeLines.push(`- ${item}`);
      }
    }

    const otherItems = [];
    for (const [cat, items] of grouped.entries()) {
      if (!knownKeys.has(cat)) {
        otherItems.push(...items);
      }
    }
    if (otherItems.length > 0) {
      changeLines.push('', OTHER_CATEGORY_HEADING);
      for (const item of otherItems) {
        changeLines.push(`- ${item}`);
      }
    }

    if (changeLines.length > 1) {
      sections.push(changeLines.join('\n'));
    }
  }

  sections.push(buildDownloadSection(tag));
  return sections.join('\n\n');
}

/**
 * Formats minimal fallback release notes when a tag is absent from CHANGELOG.md.
 */
export function formatFallbackReleaseNotes(rawTag) {
  const { tag } = normalizeTag(rawTag);
  return [`## TypesetOK (TOK) ${tag}`, buildDownloadSection(tag)].join('\n\n');
}

/**
 * Generates GitHub release notes for `rawTag` using parsed CHANGELOG.md entries.
 */
export function generateReleaseNotesForTag(rawTag, changelogMarkdown) {
  const { tag, version } = normalizeTag(rawTag);
  const entries = parseChangelogMarkdown(changelogMarkdown || '');
  const entry = entries.find((item) => item.version === version) || null;
  const body = entry ? formatReleaseNotes(entry, tag) : formatFallbackReleaseNotes(tag);
  return {
    found: Boolean(entry),
    tag,
    version,
    body,
    entry
  };
}

function parseCliArgs(argv) {
  const opts = {
    tag: '',
    changelogPath: path.join(rootDir, 'CHANGELOG.md'),
    outPath: '',
    githubOutputPath: '',
    strict: false
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--tag' && i + 1 < argv.length) {
      opts.tag = argv[++i];
    } else if (arg === '--changelog' && i + 1 < argv.length) {
      opts.changelogPath = path.resolve(argv[++i]);
    } else if (arg === '--out' && i + 1 < argv.length) {
      opts.outPath = path.resolve(argv[++i]);
    } else if (arg === '--github-output' && i + 1 < argv.length) {
      opts.githubOutputPath = path.resolve(argv[++i]);
    } else if (arg === '--strict') {
      opts.strict = true;
    } else if (!arg.startsWith('--') && !opts.tag) {
      opts.tag = arg;
    } else {
      throw new Error(`Unknown or incomplete argument: ${arg}`);
    }
  }

  return opts;
}

function runCli() {
  const opts = parseCliArgs(process.argv.slice(2));
  if (!opts.tag) {
    console.error('[RELEASE-NOTES] Usage: node scripts/generate-release-notes.mjs <tag> [--changelog <path>] [--out <path>] [--github-output <path>] [--strict]');
    process.exit(1);
  }

  if (!fs.existsSync(opts.changelogPath)) {
    console.error(`[RELEASE-NOTES] Error: changelog not found at ${opts.changelogPath}`);
    process.exit(1);
  }

  const markdown = fs.readFileSync(opts.changelogPath, 'utf8');
  const result = generateReleaseNotesForTag(opts.tag, markdown);

  if (opts.strict && !result.found) {
    console.error(`[RELEASE-NOTES] Error: version ${result.version} was not found in ${opts.changelogPath}`);
    process.exit(1);
  }

  if (opts.githubOutputPath) {
    fs.appendFileSync(opts.githubOutputPath, `has_changelog_entry=${result.found}\n`, 'utf8');
  }

  if (opts.outPath) {
    fs.mkdirSync(path.dirname(opts.outPath), { recursive: true });
    fs.writeFileSync(opts.outPath, `${result.body}\n`, 'utf8');
    console.log(`[RELEASE-NOTES] Wrote release notes for ${result.tag} (found=${result.found}) -> ${opts.outPath}`);
  } else {
    process.stdout.write(`${result.body}\n`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename)) {
  runCli();
}

// Measures desktop startup time of the built app (run `npm run build` first).
//
//   node scripts/measure-startup.mjs [runs=7] [appRoot ...]
//   node scripts/measure-startup.mjs --runs 7 --electron . --tauri path/to/tok-desktop.exe
//
// Launches Electron or Tauri with TOK_STARTUP_TRACE=exit, which makes the host print
// `[TOK-TRACE] <label> <epoch_ms>` milestones (and optional `[TOK-MEM] <label> <bytes>`)
// and quit as soon as the main window is visible and renderer timings are collected.
// All timings are ms since process spawn. One warm-up run per target is discarded;
// medians are reported, interleaved A/B/A/B across multiple targets.
import { spawn } from 'child_process';
import { createRequire } from 'module';
import * as os from 'os';
import * as path from 'path';
import { fileURLToPath } from 'url';

const require = createRequire(import.meta.url);
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
// Isolated profile so measurements never touch the real userData (logs, plugins, caches).
const defaultUserDataDir = process.env.TOK_MEASURE_USER_DATA || path.join(os.tmpdir(), 'typesetok-startup-profile');

const ORDER = [
  'main-module-loaded',
  'app-ready',
  'main-window-created',
  'splash-loaded',
  'renderer-navStart',
  'renderer-htmlLoaded',
  'renderer-scriptStart',
  'renderer-dclStart',
  'renderer-uiBuilt',
  'renderer-domContentLoaded',
  'ipc-get-plugins-start',
  'ipc-get-plugins-end',
  'renderer-fcp',
  'main-ready-to-show',
  'main-shown',
];

export function parseMeasureArgs(argv, fallbackRoot = rootDir) {
  let runs = 7;
  const targets = [];
  let sawNamedRun = false;

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--runs' && i + 1 < argv.length) {
      runs = Math.max(1, parseInt(argv[++i], 10) || 1);
      sawNamedRun = true;
    } else if (arg.startsWith('--runs=')) {
      runs = Math.max(1, parseInt(arg.slice('--runs='.length), 10) || 1);
      sawNamedRun = true;
    } else if (arg === '--tauri' && i + 1 < argv.length) {
      const targetPath = path.resolve(argv[++i]);
      targets.push({ kind: 'tauri', targetPath, label: `tauri:${targetPath}` });
    } else if (arg.startsWith('--tauri=')) {
      const targetPath = path.resolve(arg.slice('--tauri='.length));
      targets.push({ kind: 'tauri', targetPath, label: `tauri:${targetPath}` });
    } else if (arg === '--electron' && i + 1 < argv.length) {
      const targetPath = path.resolve(argv[++i]);
      targets.push({ kind: 'electron', targetPath, label: `electron:${targetPath}` });
    } else if (arg.startsWith('--electron=')) {
      const targetPath = path.resolve(arg.slice('--electron='.length));
      targets.push({ kind: 'electron', targetPath, label: `electron:${targetPath}` });
    } else if (i === 0 && !sawNamedRun && /^\d+$/.test(arg)) {
      runs = Math.max(1, parseInt(arg, 10));
    } else if (!arg.startsWith('--')) {
      const targetPath = path.resolve(arg);
      targets.push({ kind: 'electron', targetPath, label: targetPath });
    } else {
      throw new Error(`Unknown or incomplete argument: ${arg}`);
    }
  }

  if (targets.length === 0) {
    const targetPath = path.resolve(fallbackRoot);
    targets.push({ kind: 'electron', targetPath, label: targetPath });
  }
  return { runs, targets };
}

export function parseTraceLine(line, t0) {
  const trimmed = line.trim();
  const traceMatch = trimmed.match(/^\[TOK-TRACE\] (\S+) (\d+(?:\.\d+)?)$/);
  if (traceMatch) {
    return { kind: 'trace', label: traceMatch[1], value: Math.round(Number(traceMatch[2]) - t0) };
  }
  const memMatch = trimmed.match(/^\[TOK-MEM\] (\S+) (\d+(?:\.\d+)?)$/);
  if (memMatch) {
    return { kind: 'mem', label: memMatch[1], value: Number(memMatch[2]) };
  }
  return null;
}

function killTree(pid) {
  if (!pid) return;
  if (process.platform === 'win32') spawn('taskkill', ['/pid', String(pid), '/T', '/F'], { stdio: 'ignore' });
  else {
    try {
      process.kill(pid, 'SIGKILL');
    } catch {}
  }
}

function runOnce(target, electronExe, userDataDir) {
  return new Promise((resolve, reject) => {
    const exe = target.kind === 'tauri' ? target.targetPath : electronExe;
    const args = target.kind === 'tauri'
      ? []
      : [path.join(target.targetPath, 'packages/tok-electron/dist/main.js'), `--user-data-dir=${userDataDir}`];
    const t0 = Date.now();
    const child = spawn(exe, args, {
      cwd: rootDir,
      env: { ...process.env, TOK_STARTUP_TRACE: 'exit', TOK_USER_DATA_DIR: userDataDir },
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const marks = {};
    const mem = {};
    let buf = '';
    child.stdout.on('data', (d) => {
      buf += d.toString();
      let nl;
      while ((nl = buf.indexOf('\n')) >= 0) {
        const parsed = parseTraceLine(buf.slice(0, nl), t0);
        buf = buf.slice(nl + 1);
        if (!parsed) continue;
        if (parsed.kind === 'trace' && !(parsed.label in marks)) marks[parsed.label] = parsed.value;
        if (parsed.kind === 'mem' && !(parsed.label in mem)) mem[parsed.label] = parsed.value;
      }
    });
    const timer = setTimeout(() => {
      killTree(child.pid);
      if ('main-shown' in marks) return resolve({ marks, mem }); // shown, but renderer timings never arrived
      reject(new Error(`timeout waiting for startup trace of ${target.label} (got: ${Object.keys(marks).join(', ') || 'nothing'})`));
    }, 20000);
    child.on('exit', () => {
      clearTimeout(timer);
      if (!('main-shown' in marks)) reject(new Error(`run of ${target.label} ended without main-shown mark`));
      else resolve({ marks, mem });
    });
  });
}

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  const m = Math.floor(s.length / 2);
  return s.length % 2 ? s[m] : Math.round((s[m - 1] + s[m]) / 2);
};

async function main() {
  const electronExe = require('electron');
  const { runs, targets } = parseMeasureArgs(process.argv.slice(2), rootDir);
  console.log(`Electron: ${electronExe}\nProfile:  ${defaultUserDataDir}\nRuns: ${runs} per target (+1 warm-up)\n`);

  const results = targets.map(() => []);
  const tryRun = (t) =>
    runOnce(t, electronExe, defaultUserDataDir).catch((err) => {
      console.error(`  ! ${err.message}`);
      return null;
    });
  for (const t of targets) await tryRun(t); // warm-up (disk cache, first-run plugin folder creation)
  for (let i = 0; i < runs; i++) {
    for (let j = 0; j < targets.length; j++) {
      const r = await tryRun(targets[j]);
      if (!r) continue;
      results[j].push(r);
      console.log(`[${j}] run ${i + 1}: shown=${r.marks['main-shown']}ms ready-to-show=${r.marks['main-ready-to-show']}ms`);
    }
  }
  for (let j = 0; j < targets.length; j++) {
    console.log(`\n[${j}] ${targets[j].label}\nmedian ms since spawn   [min..max]`);
    for (const k of ORDER) {
      const vals = results[j].map((r) => r.marks[k]).filter((v) => typeof v === 'number');
      if (vals.length) console.log(`  ${k.padEnd(28)} ${String(median(vals)).padStart(6)}   [${Math.min(...vals)}..${Math.max(...vals)}]`);
    }
    const memKeys = [...new Set(results[j].flatMap((r) => Object.keys(r.mem)))];
    if (memKeys.length) {
      console.log('median memory MB        [min..max]');
      for (const k of memKeys) {
        const vals = results[j].map((r) => r.mem[k]).filter((v) => typeof v === 'number');
        const toMb = (b) => (b / (1024 * 1024)).toFixed(1);
        if (vals.length) console.log(`  ${k.padEnd(28)} ${toMb(median(vals)).padStart(6)}   [${toMb(Math.min(...vals))}..${toMb(Math.max(...vals))}]`);
      }
    }
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  await main();
}

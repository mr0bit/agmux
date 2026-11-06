#!/usr/bin/env node
// Diagnostic only: no providers, builders, original applications or paid tools run.
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname, resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(readFileSync(join(root, 'plugin.json'), 'utf8'));
const git = process.env.SOUL_GIT || 'git';
function gitOutput(args) {
  const result = spawnSync(git, ['-C', join(root, 'upstream'), ...args], { encoding: 'utf8', shell: false });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr.trim() || `git exited ${result.status}`);
  return result.stdout.trim();
}

try {
  const head = gitOutput(['rev-parse', 'HEAD']);
  if (head !== manifest.commit) throw new Error(`Pin mismatch: expected ${manifest.commit}, found ${head}`);
  if (gitOutput(['status', '--porcelain'])) throw new Error('Upstream worktree is modified');
  console.log(`PASS: unchanged upstream at ${head}`);

  const fixture = mkdtempSync(join(tmpdir(), 'soul-craft-probe-'));
  try {
    const builder = join(fixture, 'builder');
    mkdirSync(builder);
    const marker = join(fixture, 'private-inspection-marker.txt');
    writeFileSync(marker, 'synthetic private inspection evidence', { flag: 'wx' });
    const child = spawnSync(process.execPath, ['-e',
      'process.stdout.write(require("node:fs").readFileSync(process.argv[1], "utf8"))', marker],
    { cwd: builder, encoding: 'utf8', shell: false, timeout: 5000 });
    if (child.error) throw child.error;
    if (child.status !== 0 || child.stdout !== 'synthetic private inspection evidence') {
      throw new Error('Probe inconclusive: child could not read the synthetic marker');
    }
    console.log('CONFIRMED: a plain child in a fresh builder folder can read sibling private evidence');
    console.log('Scope: ordinary Node child process; agent sandboxes were not tested.');
  } finally {
    const rel = relative(resolve(tmpdir()), resolve(fixture));
    if (!rel || rel.startsWith('..') || isAbsolute(rel)) throw new Error('Unsafe temporary cleanup path');
    rmSync(fixture, { recursive: true, force: true });
  }
  console.log('BLOCKED: full bridge requires a verified isolated runner and approved-handoff gate.');
  // A successful diagnostic is distinct from successful integration compatibility.
  process.exitCode = 2;
} catch (error) {
  console.error(`ERROR: ${error.message}`);
  process.exitCode = 1;
}

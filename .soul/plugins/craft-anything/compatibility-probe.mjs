#!/usr/bin/env node
/**
 * Purpose: verify the pinned Craft Anything checkout and demonstrate why a fresh
 * working directory alone is insufficient for clean room file-access isolation.
 *
 * Run from the host repository root:
 *   node .soul/plugins/craft-anything/compatibility-probe.mjs
 * Or through the shared plugin checker:
 *   node .soul/tools/check-plugins.mjs --plugin craft-anything
 * Set SOUL_GIT to the Git executable's path if Git is not on PATH.
 *
 * Steps:
 * 1. Locate this adapter and load the expected upstream commit from plugin.json.
 * 2. Require upstream/ to match that commit and have no Git-reported changes.
 * 3. Create synthetic private evidence beside a temporary builder/ directory.
 * 4. Start an ordinary Node child in builder/ and try reading the evidence by
 *    absolute path. A successful read demonstrates that cwd is not isolation.
 * 5. Validate the temporary cleanup path and remove the fixture, even if the
 *    child-process experiment fails.
 * 6. Report that the full bridge remains blocked; this diagnostic does not
 *    establish a verified isolated runner or an approved-handoff gate.
 *
 * Exit 2: diagnostic completed, but full integration is still blocked.
 * Exit 1: dependency check failed, the probe was inconclusive, or an error occurred.
 * No exit-0 readiness result is implemented. The final BLOCKED message is fixed;
 * this script does not detect whether a bridge has subsequently been completed.
 *
 * No Craft Anything build, AI agent, provider, original application, container or
 * paid tool is run. Only Git inspection and a synthetic local Node process run.
 * Upstream files are not modified; temporary probe files are created and removed.
 */
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname, resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

// Step 1: resolve paths from this file, so the caller's cwd does not affect them.
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
  // Step 2: refuse a different upstream version or a modified checkout.
  const head = gitOutput(['rev-parse', 'HEAD']);
  if (head !== manifest.commit) throw new Error(`Pin mismatch: expected ${manifest.commit}, found ${head}`);
  if (gitOutput(['status', '--porcelain'])) throw new Error('Upstream worktree is modified');
  console.log(`PASS: unchanged upstream at ${head}`);

  // Step 3: use disposable synthetic data, never real investigation evidence.
  const fixture = mkdtempSync(join(tmpdir(), 'soul-craft-probe-'));
  try {
    const builder = join(fixture, 'builder');
    mkdirSync(builder);
    const marker = join(fixture, 'private-inspection-marker.txt');
    writeFileSync(marker, 'synthetic private inspection evidence', { flag: 'wx' });
    // Step 4: the child starts in builder/, but reads a file outside that folder.
    // Its five-second timeout bounds this experiment; no agent sandbox is involved.
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
    // Step 5: remove only the fixture we created within the system temp directory.
    const rel = relative(resolve(tmpdir()), resolve(fixture));
    if (!rel || rel.startsWith('..') || isAbsolute(rel)) throw new Error('Unsafe temporary cleanup path');
    rmSync(fixture, { recursive: true, force: true });
  }
  // Step 6: completing this demonstration does not make the integration ready.
  console.log('BLOCKED: full bridge requires a verified isolated runner and approved-handoff gate.');
  // A successful diagnostic is distinct from successful integration compatibility.
  process.exitCode = 2;
} catch (error) {
  console.error(`ERROR: ${error.message}`);
  process.exitCode = 1;
}

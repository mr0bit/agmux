#!/usr/bin/env node
/**
 * Purpose: check the pinned Craft Anything checkout without running its tools.
 * Run: node .soul/tools/check-plugins.mjs --plugin craft-anything
 * Set SOUL_GIT to the Git executable's path if Git is not on PATH.
 *
 * Steps:
 * 1. Locate this adapter and read the expected commit from plugin.json.
 * 2. Require upstream/ to match that commit and have no Git-reported changes.
 * 3. Report the unfinished integration as BLOCKED.
 *
 * Exit 2: checkout checks passed, but the integration is still unfinished.
 * Exit 1: a checkout check failed or another error occurred.
 * The BLOCKED result is fixed, not a test of whether the bridge is ready.
 * No build, provider, AI agent, container or file-access experiment runs.
 */
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

// Step 1: locate the adapter independently of the caller's working directory.
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

  // Step 3: checkout consistency does not establish that the integration works.
  console.log('BLOCKED: full bridge requires a verified isolated runner and approved-handoff gate.');
  process.exitCode = 2;
} catch (error) {
  console.error(`ERROR: ${error.message}`);
  process.exitCode = 1;
}

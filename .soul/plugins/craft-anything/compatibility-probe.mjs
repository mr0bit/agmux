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
 * Configuration errors are reported in plain text; each Git call has a five-second limit.
 * The BLOCKED result is fixed, not a test of whether the bridge is ready.
 * No build, provider, AI agent, container or file-access experiment runs.
 */
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

// Step 1: locate the adapter independently of the caller's working directory.
const root = dirname(fileURLToPath(import.meta.url));
const git = process.env.SOUL_GIT || 'git';
function readManifest() {
  let source;
  try {
    source = readFileSync(join(root, 'plugin.json'), 'utf8');
  } catch (error) {
    if (error.code === 'ENOENT') throw new Error('Missing plugin.json. Restore the adapter manifest before checking.');
    throw new Error(`Cannot read plugin.json: ${error.message}`);
  }
  let manifest;
  try { manifest = JSON.parse(source); } catch {
    throw new Error('Invalid plugin.json: expected valid JSON.');
  }
  if (typeof manifest?.commit !== 'string' || !/^[a-f0-9]{40}$/i.test(manifest.commit)) {
    throw new Error('Invalid plugin.json: commit must be a 40-character Git commit.');
  }
  return manifest;
}

function gitOutput(args) {
  const result = spawnSync(git, ['-C', join(root, 'upstream'), ...args], { encoding: 'utf8', shell: false, timeout: 5000 });
  if (result.error?.code === 'ETIMEDOUT') throw new Error(`Git ${args[0]} timed out after 5 seconds.`);
  if (result.error?.code === 'ENOENT') throw new Error('Git executable not found. Install Git or set SOUL_GIT to its path.');
  if (result.error) throw new Error(`Git ${args[0]} could not run: ${result.error.message}`);
  if (result.status !== 0) throw new Error(`Git ${args[0]} failed: ${result.stderr.trim() || `exit ${result.status}`}`);
  return result.stdout.trim();
}

try {
  const manifest = readManifest();
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

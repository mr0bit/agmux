#!/usr/bin/env node
// Purpose: sync recorded plugin submodule versions, then automatically check all adapters.
// Run: node .soul/tools/update-plugins.mjs
// No newer-version selection, forced checkout or automatic plugin enablement.
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { checkPlugins, resultExitCode } from './check-plugins.mjs';

/** Sync only indexed plugin submodules to their pins, then run all adapter checks. */
export function updatePlugins(root, git = process.env.SOUL_GIT || 'git') {
  let syncError;
  try {
    const call = args => {
      const child = spawnSync(git, ['-C', root, ...args], {
        encoding: 'utf8', shell: false, timeout: 60000, maxBuffer: 1024 * 1024,
      });
      if (child.error) throw child.error;
      if (child.status !== 0) throw new Error(child.stderr?.trim() || `Git exited ${child.status}`);
      return child.stdout;
    };
    const entries = call(['ls-files', '--stage', '-z', '--', '.soul/plugins/']).split('\0').filter(Boolean);
    // 1. Read only indexed plugin submodules; reject unresolved index conflicts.
    const submodules = entries.filter(entry => entry.startsWith('160000 ')).map(entry => {
      const tab = entry.indexOf('\t');
      if (!entry.slice(0, tab).endsWith(' 0')) throw new Error('Resolve plugin index conflicts before updating');
      return entry.slice(tab + 1);
    });
    // 2. Sync those exact pins; unrelated submodules remain untouched.
    if (submodules.length) call(['submodule', 'update', '--init', '--', ...submodules]);
  } catch (error) {
    syncError = error.message;
  }
  // 3. Check all adapters, even after sync failure or when none use submodules.
  const results = checkPlugins(join(root, '.soul/plugins'), root);
  // 4. Update failure wins; otherwise return the combined check result.
  return { syncError, results, exitCode: syncError ? 1 : resultExitCode(results) };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 2) {
    console.error('Usage: node .soul/tools/update-plugins.mjs');
    process.exitCode = 1;
  } else {
    const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
    const outcome = updatePlugins(root);
    if (outcome.syncError) console.error(`FAILED: plugin update — ${outcome.syncError}`);
    else console.log('Plugin submodules synchronized to indexed commits. Running compatibility checks:');
    if (!outcome.results.length) console.log('NO PLUGINS: no plugin checks were run.');
    for (const result of outcome.results) {
      console.log(`${result.status}: ${result.name} — ${result.detail}`);
      if (result.output) console.log(result.output);
    }
    process.exitCode = outcome.exitCode;
  }
}

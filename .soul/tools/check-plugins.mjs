#!/usr/bin/env node
// Purpose: run declared plugin checks; report status. No installs or enablement.
// Usage: node .soul/tools/check-plugins.mjs [--plugin <name>]
import { readdirSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const here = dirname(fileURLToPath(import.meta.url));
const pluginsRoot = resolve(here, '../plugins');
const hostRoot = resolve(here, '../..');
const inside = (parent, child) => {
  const rel = relative(parent, child);
  return rel !== '' && rel !== '..' && !rel.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`) && !isAbsolute(rel);
};

/** Run each adapter's declared Node diagnostic; return results without enabling plugins. */
export function checkPlugins(root, cwd, selected) {
  // 1. Find adapter folders only; select one if requested. Never scan upstream trees.
  let entries;
  try {
    entries = readdirSync(root, { withFileTypes: true })
      .filter(entry => entry.isDirectory() || entry.isSymbolicLink())
      .sort((a, b) => a.name.localeCompare(b.name));
  } catch (error) {
    if (error.code === 'ENOENT' && !selected) return [];
    return [{ name: selected || '*', status: 'UNAVAILABLE', detail: error.message }];
  }
  if (selected) {
    entries = entries.filter(entry => entry.name === selected);
    if (!entries.length) return [{ name: selected, status: 'UNAVAILABLE', detail: 'Plugin not found' }];
  }
  return entries.map(entry => {
    const name = entry.name;
    try {
      const pluginDir = realpathSync(join(root, name));
      if (!inside(realpathSync(root), pluginDir)) throw new Error('Plugin directory escapes plugins root');
      const manifest = JSON.parse(readFileSync(join(pluginDir, 'plugin.json'), 'utf8'));
      // 2. Require a supported manifest, check script and valid timeout.
      const check = manifest.compatibilityCheck;
      if (manifest.schemaVersion !== 1 || !check || typeof check.script !== 'string' || !check.script) {
        throw new Error('Expected schemaVersion 1 and compatibilityCheck.script');
      }
      if (isAbsolute(check.script)) throw new Error('Check script must be relative to its adapter');
      // 3. Keep the resolved check file inside its adapter, including symlink targets.
      const script = resolve(pluginDir, check.script);
      if (!inside(pluginDir, script)) throw new Error('Check script escapes adapter directory');
      const timeout = check.timeoutMs ?? 30000;
      if (!Number.isInteger(timeout) || timeout < 1 || timeout > 300000) throw new Error('timeoutMs must be 1..300000');
      let actual;
      try { actual = realpathSync(script); } catch (error) {
        if (error.code === 'ENOENT') return { name, status: 'UNAVAILABLE', detail: 'Check script is missing' };
        throw error;
      }
      if (!inside(pluginDir, actual) || !statSync(actual).isFile()) throw new Error('Check must be a file inside its adapter');
      const child = spawnSync(process.execPath, [actual], {
        // 4. Run trusted adapter code directly; bound runtime and captured output.
        cwd, encoding: 'utf8', shell: false, timeout, maxBuffer: 1024 * 1024,
      });
      const output = [child.stdout, child.stderr].filter(Boolean).join('\n').trim();
      if (child.error || child.signal) {
        return { name, status: 'FAILED', detail: child.error?.message || `Terminated: ${child.signal}`, output };
      }
      const status = ({ 0: 'PASSED', 1: 'FAILED', 2: 'BLOCKED', 3: 'UNAVAILABLE' })[child.status] || 'FAILED';
      // 5. Preserve each check's result and output; continue checking other adapters.
      return { name, status, detail: `Check exited ${child.status}`, output };
    } catch (error) {
      return { name, status: 'FAILED', detail: error.message };
    }
  });
}

/** Errors/unavailable checks take precedence over blocked checks in an aggregate result. */
export function resultExitCode(results) {
  // 6. Aggregate: failure/unavailable first, then blocked, otherwise success.
  if (results.some(result => ['FAILED', 'UNAVAILABLE'].includes(result.status))) return 1;
  return results.some(result => result.status === 'BLOCKED') ? 2 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length && (args.length !== 2 || args[0] !== '--plugin' || !/^[a-z0-9][a-z0-9-]*$/.test(args[1]))) {
    console.error('Usage: node .soul/tools/check-plugins.mjs [--plugin <name>]');
    process.exitCode = 1;
  } else {
    const results = checkPlugins(pluginsRoot, hostRoot, args[1]);
    if (!results.length) console.log('NO PLUGINS: no plugin checks were run.');
    for (const result of results) {
      console.log(`${result.status}: ${result.name} — ${result.detail}`);
      if (result.output) console.log(result.output);
    }
    process.exitCode = resultExitCode(results);
  }
}

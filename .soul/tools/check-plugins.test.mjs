import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, relative, isAbsolute } from 'node:path';
import { checkPlugins, resultExitCode } from './check-plugins.mjs';

function fixture(t) {
  const home = mkdtempSync(join(tmpdir(), 'soul-plugin-tests-'));
  const root = join(home, 'plugins');
  mkdirSync(root);
  t.after(() => {
    const rel = relative(resolve(tmpdir()), resolve(home));
    if (!rel || rel.startsWith('..') || isAbsolute(rel)) throw new Error('Unsafe cleanup path');
    rmSync(home, { recursive: true, force: true });
  });
  const plugin = (name, code = 'process.exit(0)', fields = {}) => {
    const dir = join(root, name);
    mkdirSync(dir);
    const manifest = { schemaVersion: 1, executionEnabled: false, compatibilityCheck: { script: 'check with spaces.mjs' }, ...fields };
    writeFileSync(join(dir, 'plugin.json'), JSON.stringify(manifest));
    writeFileSync(join(dir, 'check with spaces.mjs'), code);
    return dir;
  };
  return { home, root, plugin };
}

test('discovers adapters once, preserves all outcomes and never recurses into upstream', t => {
  const { home, root, plugin } = fixture(t);
  plugin('pass', 'console.log(process.cwd()); process.exit(0)');
  const blocked = plugin('blocked', 'console.log("unfinished bridge"); process.exit(2)');
  plugin('failed', 'process.exit(1)');
  plugin('unavailable', 'process.exit(3)');
  mkdirSync(join(blocked, 'upstream'));
  writeFileSync(join(blocked, 'upstream', 'plugin.json'), '{}');
  const before = readFileSync(join(blocked, 'plugin.json'), 'utf8');
  const results = checkPlugins(root, home);
  assert.deepEqual(results.map(r => [r.name, r.status]), [
    ['blocked', 'BLOCKED'], ['failed', 'FAILED'], ['pass', 'PASSED'], ['unavailable', 'UNAVAILABLE'],
  ]);
  assert.equal(results.find(r => r.name === 'pass').output, home);
  assert.match(results[0].output, /unfinished bridge/);
  assert.equal(readFileSync(join(blocked, 'plugin.json'), 'utf8'), before);
  assert.equal(resultExitCode(results), 1);
});

test('selection runs only the requested adapter and reports unknown adapters', t => {
  const { home, root, plugin } = fixture(t);
  plugin('selected', 'process.exit(2)');
  plugin('other', 'throw new Error("must not run")');
  const results = checkPlugins(root, home, 'selected');
  assert.deepEqual(results.map(r => r.status), ['BLOCKED']);
  assert.equal(resultExitCode(results), 2);
  assert.equal(checkPlugins(root, home, 'unknown')[0].status, 'UNAVAILABLE');
});

test('missing and malformed manifests fail without preventing remaining checks', t => {
  const { home, root, plugin } = fixture(t);
  mkdirSync(join(root, 'missing'));
  const malformed = plugin('malformed');
  writeFileSync(join(malformed, 'plugin.json'), '{');
  plugin('valid');
  assert.deepEqual(checkPlugins(root, home).map(r => r.status), ['FAILED', 'FAILED', 'PASSED']);
});

test('rejects escaped script paths and invalid timeouts; missing scripts are unavailable', t => {
  const { home, root, plugin } = fixture(t);
  plugin('escape', '', { compatibilityCheck: { script: '../outside.mjs' } });
  plugin('missing', '', { compatibilityCheck: { script: 'absent.mjs' } });
  plugin('timeout', '', { compatibilityCheck: { script: 'check with spaces.mjs', timeoutMs: 0 } });
  const results = checkPlugins(root, home);
  assert.deepEqual(results.map(r => r.status), ['FAILED', 'UNAVAILABLE', 'FAILED']);
  assert.match(results[0].detail, /escapes/);
});

test('timeout and unexpected exits fail rather than passing or blocking', t => {
  const { home, root, plugin } = fixture(t);
  plugin('slow', 'setTimeout(() => {}, 10000)', { compatibilityCheck: { script: 'check with spaces.mjs', timeoutMs: 200 } });
  plugin('unexpected', 'process.exit(42)');
  const results = checkPlugins(root, home);
  assert.deepEqual(results.map(r => r.status), ['FAILED', 'FAILED']);
  assert.match(results[0].detail, /timed out|ETIMEDOUT/i);
});

test('empty scans are distinct from unavailable selection; successful checks aggregate to zero', t => {
  const { home, root, plugin } = fixture(t);
  assert.deepEqual(checkPlugins(root, home), []);
  assert.deepEqual(checkPlugins(join(home, 'absent'), home), []);
  assert.equal(checkPlugins(join(home, 'absent'), home, 'wanted')[0].status, 'UNAVAILABLE');
  assert.equal(resultExitCode([]), 0);
  plugin('ready');
  assert.equal(resultExitCode(checkPlugins(root, home)), 0);
});

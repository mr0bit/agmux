import test from 'node:test';
import assert from 'node:assert/strict';
import { copyFileSync, existsSync, mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';

const git = process.env.SOUL_GIT || 'git';
function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'craft-check-errors-'));
  const probe = join(dir, 'compatibility-probe.mjs');
  copyFileSync(join(dirname(fileURLToPath(import.meta.url)), 'compatibility-probe.mjs'), probe);
  t.after(() => {
    const rel = relative(resolve(tmpdir()), resolve(dir));
    if (!rel || rel.startsWith('..') || isAbsolute(rel)) throw new Error('Unsafe cleanup path');
    rmSync(dir, { recursive: true, force: true, maxRetries: 3 });
  });
  const config = value => writeFileSync(join(dir, 'plugin.json'), value);
  const run = (gitExecutable = git) => spawnSync(process.execPath, [probe], {
    encoding: 'utf8', shell: false, timeout: 15000, env: { ...process.env, SOUL_GIT: gitExecutable },
  });
  return { dir, config, run };
}

test('missing manifest produces one actionable error without a stack trace', t => {
  const { run } = fixture(t);
  const result = run();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /^ERROR: Missing plugin\.json\./);
  assert.doesNotMatch(result.stderr, /\n\s+at /);
});

test('invalid JSON and missing commit produce clear configuration errors', t => {
  const { config, run } = fixture(t);
  config('{');
  assert.match(run().stderr, /^ERROR: Invalid plugin\.json: expected valid JSON\./);
  for (const value of ['null', '{}', '{"commit":"main"}']) {
    config(value);
    const result = run();
    assert.equal(result.status, 1);
    assert.match(result.stderr, /^ERROR: Invalid plugin\.json: commit must be a 40-character Git commit\./);
  }
});

test('a real Git status that waits for a filesystem monitor is bounded by the timeout', t => {
  const { dir, config, run } = fixture(t);
  const upstream = join(dir, 'upstream');
  const hooks = join(dir, 'empty-hooks');
  mkdirSync(upstream);
  mkdirSync(hooks);
  const command = args => {
    const result = spawnSync(git, ['-c', `core.hooksPath=${hooks}`, '-c', 'user.name=Fixture',
      '-c', 'user.email=fixture@example.invalid', '-c', 'commit.gpgsign=false', '-C', upstream, ...args],
    { encoding: 'utf8', shell: false, timeout: 10000 });
    assert.equal(result.status, 0, result.error?.message || result.stderr);
    return result.stdout.trim();
  };
  command(['init']);
  writeFileSync(join(upstream, 'note.txt'), 'fixture');
  command(['add', 'note.txt']);
  command(['commit', '-m', 'Fixture']);
  config(JSON.stringify({ commit: command(['rev-parse', 'HEAD']) }));
  const pidFile = join(dir, 'monitor.pid');
  const monitor = join(dir, 'monitor.cjs');
  writeFileSync(monitor, `require('node:fs').writeFileSync(${JSON.stringify(pidFile)}, String(process.pid)); setTimeout(() => {}, 12000);`);
  const quote = path => '"' + path.replaceAll('\\', '/').replace(/["$`]/g, '\\$&') + '"';
  command(['config', 'core.fsmonitor', `${quote(process.execPath)} ${quote(monitor)}`]);
  try {
    const result = run();
    assert.equal(result.status, 1, result.error?.message || result.stderr);
    assert.match(result.stderr, /^ERROR: Git status timed out after 5 seconds\./);
    assert.ok(existsSync(pidFile), 'The real Git monitor must have started');
    assert.doesNotMatch(result.stdout, /BLOCKED:/);
  } finally {
    if (existsSync(pidFile)) {
      const pid = Number(readFileSync(pidFile, 'utf8'));
      try { process.kill(pid); } catch (error) { if (error.code !== 'ESRCH') throw error; }
    }
  }
});

test('missing Git explains how to configure the executable', t => {
  const { dir, config, run } = fixture(t);
  config(JSON.stringify({ commit: '0'.repeat(40) }));
  const result = run(join(dir, 'git-not-installed'));
  assert.equal(result.status, 1);
  assert.match(result.stderr, /^ERROR: Git executable not found\. Install Git or set SOUL_GIT to its path\./);
});

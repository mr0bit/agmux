import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { join, relative, resolve, isAbsolute } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { updatePlugins } from './update-plugins.mjs';

const git = process.env.SOUL_GIT || 'git';
function fixture(t, withSubmodules = true) {
  const home = mkdtempSync(join(tmpdir(), 'soul-update-tests-'));
  const host = join(home, 'host');
  const hooks = join(home, 'empty-hooks');
  mkdirSync(host);
  mkdirSync(hooks);
  t.after(() => {
    const rel = relative(resolve(tmpdir()), resolve(home));
    if (!rel || rel.startsWith('..') || isAbsolute(rel)) throw new Error('Unsafe cleanup path');
    rmSync(home, { recursive: true, force: true, maxRetries: 3 });
  });
  const command = (cwd, args) => {
    const child = spawnSync(git, ['-c', 'protocol.file.allow=always', '-c', `core.hooksPath=${hooks}`,
      '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', '-c', 'commit.gpgsign=false', '-C', cwd, ...args],
    { encoding: 'utf8', shell: false, timeout: 20000 });
    assert.equal(child.status, 0, child.error?.message || child.stderr);
    return child.stdout.trim();
  };
  command(host, ['init']);
  const dir = join(host, '.soul/plugins/demo');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'plugin.json'), JSON.stringify({ schemaVersion: 1, compatibilityCheck: { script: 'check.mjs' } }));
  writeFileSync(join(dir, 'check.mjs'), 'console.log("CHECK-RAN"); process.exit(2)');
  if (!withSubmodules) return { host, dir, command };
  const source = join(home, 'source');
  mkdirSync(source);
  command(source, ['init']);
  writeFileSync(join(source, 'note.txt'), 'first');
  command(source, ['add', 'note.txt']);
  command(source, ['commit', '-m', 'First fixture revision']);
  const first = command(source, ['rev-parse', 'HEAD']);
  const path = '.soul/plugins/demo/upstream';
  command(host, ['submodule', 'add', source, path]);
  const outside = 'other-tools/external';
  command(host, ['submodule', 'add', source, outside]);
  writeFileSync(join(source, 'note.txt'), 'second');
  command(source, ['commit', '-am', 'Second fixture revision']);
  const second = command(source, ['rev-parse', 'HEAD']);
  for (const child of [join(host, path), join(host, outside)]) {
    command(child, ['fetch', 'origin']);
    command(child, ['checkout', '--detach', second]);
  }
  writeFileSync(join(dir, 'check.mjs'),
    'import {readFileSync} from "node:fs"; console.log("CHECK-RAN"); process.exit(readFileSync(".soul/plugins/demo/upstream/note.txt","utf8") === "first" ? 0 : 1)');
  return { host, dir, command, first, second, outside };
}

test('updates only plugin submodules to indexed pins and then checks their resulting content', t => {
  const { host, dir, command, first, second, outside } = fixture(t);
  const outcome = updatePlugins(host, git);
  assert.equal(outcome.syncError, undefined);
  assert.equal(outcome.exitCode, 0);
  assert.equal(outcome.results[0].status, 'PASSED');
  assert.match(outcome.results[0].output, /CHECK-RAN/);
  assert.equal(command(join(dir, 'upstream'), ['rev-parse', 'HEAD']), first);
  assert.equal(command(join(host, outside), ['rev-parse', 'HEAD']), second);
});

test('preserves conflicting local edits, reports update failure and still runs diagnostics', t => {
  const { host, dir } = fixture(t);
  const note = join(dir, 'upstream/note.txt');
  writeFileSync(note, 'local edits');
  const outcome = updatePlugins(host, git);
  assert.ok(outcome.syncError);
  assert.equal(outcome.exitCode, 1);
  assert.match(outcome.results[0].output, /CHECK-RAN/);
  assert.equal(readFileSync(note, 'utf8'), 'local edits');
});

test('plugins without submodules still run checks and retain blocked status', t => {
  const { host } = fixture(t, false);
  const outcome = updatePlugins(host, git);
  assert.equal(outcome.syncError, undefined);
  assert.equal(outcome.exitCode, 2);
  assert.equal(outcome.results[0].status, 'BLOCKED');
});

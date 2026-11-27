import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, dirname } from 'node:path';

const require = createRequire(import.meta.url);
const manifest = require.resolve('groma.md/package.json');
const { bin } = require(manifest);
const cli = join(dirname(manifest), bin.groma);
const { scanners } = JSON.parse(readFileSync(new URL('../groma/config.json', import.meta.url), 'utf8'));

function groma(...args) {
  const result = spawnSync(process.execPath, [cli, ...args], { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

// A generic plugin restore can upgrade missing npm scanners. Restore exact sources first.
for (const { id, source } of scanners) {
  groma('plugin', 'update', id, source);
}
groma('plugin', 'install');

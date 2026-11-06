# Plugin lifecycle

Applies to every adapter under `.soul/plugins/`. Host orchestrator owns updates and acceptance; plugin-specific checks stay in each adapter.

## Commands

After pulling accepted plugin changes, sync pinned submodules and check all plugins:

```sh
node .soul/tools/update-plugins.mjs
```

Check without updating:

```sh
node .soul/tools/check-plugins.mjs
node .soul/tools/check-plugins.mjs --plugin <name>
```

Run checks after installation, adapter/dependency changes, relevant `.soul` changes, and before accepting upgrades. Before use, check the selected plugin and its execution rules. Direct Git updates do not trigger checks automatically.

## Plugin contract

Each `plugin.json` declares a Node check, relative to its adapter:

```json
{
  "schemaVersion": 1,
  "compatibilityCheck": {
    "script": "compatibility-probe.mjs",
    "timeoutMs": 30000
  }
}
```

Checks verify version, unchanged files, prerequisites and adapter compatibility. Review changed scripts before running: trusted local code, not a sandbox. No implicit installs, paid calls or builds. Clean up temporary files and child processes.

| Check exit | Meaning |
| --- | --- |
| 0 | PASSED |
| 1 / unexpected | FAILED |
| 2 | BLOCKED: unfinished requirement |
| 3 | UNAVAILABLE: missing prerequisite |

Shared runner exits 1 for failure/unavailable, otherwise 2 for blocked, otherwise 0. Empty scan means no checks ran. Passing checks never enable a plugin. Blocked optional plugins may remain disabled; ordinary work can continue.

## Accepting updates

- Updater syncs only recorded plugin submodule commits; no `--remote`, new pins or automatic enablement. Update errors remain failures even if checks pass.
- For a new version: preserve accepted pin, review changes, test candidate in an isolated checkout with a temporary candidate manifest. Only after passing checks accept manifest and gitlink together.
- Record results, environment and revisions in the host project. Investigate failed/unavailable checks before accepting affected changes.

For timeout, output and path-validation details, see [checker](../tools/check-plugins.mjs) and [updater](../tools/update-plugins.mjs). Timeouts stop direct processes; adapters own child cleanup.

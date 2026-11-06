# Optional plugin lifecycle

Applies to every immediate plugin directory under `.soul/plugins/`. The host orchestrator owns this process. Each plugin keeps an unchanged pinned upstream dependency, its adapter, and a `plugin.json` manifest. The shared runner discovers adapter directories only; it does not recurse through upstream repositories.

## Compatibility contract

Every plugin must declare a Node check in its manifest:

```json
{
  "schemaVersion": 1,
  "compatibilityCheck": {
    "script": "compatibility-probe.mjs",
    "timeoutMs": 30000
  }
}
```

`script` is relative to the adapter directory. Checks run with the host repository as cwd and inherit its environment. They are trusted adapter code, not a sandbox; review a newly introduced or changed check before executing it. Keep checks diagnostic: no implicit installs, upgrades, provider authentication, paid calls or builder execution. Declare any additional live verification separately in the adapter. Clean up child processes and temporary artifacts on success, failure and cancellation.

Each check must verify the dependency version/cleanliness and relevant adapter interfaces, prerequisites and constraints. A plugin with no upstream dependency instead records and checks the versioned components it uses. Keep plugin-specific knowledge in the adapter; the shared runner only validates the manifest, invokes its check and aggregates results.

| Check exit | Status | Meaning |
| --- | --- | --- |
| 0 | PASSED | Declared compatibility checks passed; this alone does not enable execution. |
| 1 or unexpected exit | FAILED | Broken contract or failed check. |
| 2 | BLOCKED | A known prerequisite or required implementation is unfinished. |
| 3 | UNAVAILABLE | A required local executable/dependency is not installed. |

Missing manifests, invalid configuration, escaped script paths, timeouts and signals never count as passed. A missing script is unavailable. Output is bounded to 1 MiB per check; checks have a 30-second default timeout, configurable up to five minutes. The timeout terminates the direct diagnostic process; adapters remain responsible for any descendants they launch.

## Required triggers

Run this command after installing a plugin, after changing an adapter or pinned dependency, and after updating `.soul` when plugin contracts or shared workflow/tooling have changed:

```sh
node .soul/tools/check-plugins.mjs
```

Run all checks before accepting a plugin or relevant `.soul` upgrade. Record each result, environment and dependency revision in the host project's verification evidence. A blocked optional plugin may remain present and disabled; report that result instead of declaring all plugins compatible. Failed/unavailable checks require investigation before accepting affected changes. Ordinary engineering work may continue without optional plugins.

Before using one plugin, check just that adapter and read its execution readiness rules:

```sh
node .soul/tools/check-plugins.mjs --plugin <name>
```

Require a passed check and the adapter's execution controls before starting its workflow. A changed environment or dependency invalidates earlier results. Do not infer readiness from other plugins passing.

For an upgrade candidate, preserve the last accepted pin, review the candidate diff, and test it against a temporary candidate manifest in an isolated checkout. Only after checks pass update the accepted manifest and parent gitlink together. Keep failed candidate results without labeling that candidate supported. This avoids testing a new checkout against the old accepted pin.

## Invocation and aggregate outcome

The command can run from any cwd; it locates `.soul` relative to its own file. It executes all discovered checks, including disabled plugins, and never changes their enablement flags. With `--plugin`, an unknown plugin is unavailable. With no plugin directories, report that zero checks ran.

The runner exits 1 if any check failed or is unavailable; otherwise 2 if any is blocked; otherwise 0. Exit 0 with zero plugins is an empty scan, not proof of compatibility. Inspect per-plugin outcomes when accepting updates with disabled optional dependencies.

These are mandatory instructions for the orchestrating agent. The runner does not install a filesystem watcher, Git hook, CI job or `.soul` updater. An updater must invoke this command at the triggers above; running `git pull` alone does not execute it.

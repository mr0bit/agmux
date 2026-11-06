# Craft Anything integration experiment

Date: 2026-11-06. Upstream: `storytold/craft-anything`, commit
`785539196a1863ee1a49c8cb69639d02f385bbbe` (workspace version 0.4.0).

## Result

Dependency integration is present. Full Study -> Build -> Check integration is **blocked**, not verified or enabled. No provider, target application, native upstream builder or paid tool was run. No agent sandbox was tested. No upstream files were modified.

The user approved an unchanged, optionally installed submodule; an external bridge using existing delegation lanes; a reviewed specification before building; a fresh implementer restricted to permitted inputs; bounded refinement; and human acceptance after engineering gates. They explicitly required returning concrete conflicts before changing those controls or modifying upstream.

## Experiment and evidence

`node .soul/plugins/craft-anything/compatibility-probe.mjs` verifies the checked-out commit and clean upstream worktree. Set `SOUL_GIT` to the Git executable if Git is not on PATH. It performs no file-access experiment and runs no Craft Anything tools, providers or builders.

Observed output:

```text
PASS: unchanged upstream at 785539196a1863ee1a49c8cb69639d02f385bbbe
BLOCKED: full bridge requires a verified isolated runner and approved-handoff gate.
```

Exit 2 means the checkout checks passed but integration remains blocked; exit 1 means diagnostic failure. The blocked result records known unfinished work, not a runtime readiness test. An earlier file-access demonstration was removed following review because it did not test Craft Anything compatibility. No replacement experiment script was added.

Source inspection at the pinned commit found:

| Finding | Evidence in unchanged upstream |
| --- | --- |
| External builder extension exists | `crates/ai/src/cli_agents/adapters.rs`: public `AgentAdapter`, `AgentAdapters::register`; `rebuild/mod.rs`: `RunOptions.adapters` |
| A relay is not a drop-in executable replacement | `crates/ai/src/config.rs`: custom command/args; adapter prompt/event interfaces differ from host relay brief-file/result-artifact interfaces |
| Study transitions into build without a dedicated reviewed-specification gate in `RunOptions` | `crates/ai/src/rebuild/mod.rs`: `Worker::run`, `self.study`, prompt construction and `self.turn` |
| Builder prompts include the original target and expose investigation context | `crates/ai/src/rebuild/brief.rs`: `brief`, `steps`, target/capture descriptions |
| Native Antigravity builder is unsuitable for direct use under the agreed approval contract | `crates/ai/src/cli_agents/adapters/antigravity.rs`: builder adds `--dangerously-skip-permissions` |
| `rebuild_check` cannot simply replace a standalone comparison stage | `crates/ai/src/rebuild/tools.rs`: sends a check request to an active build |

The host Codex relay defaults to workspace-write; this is not a demonstrated read allowlist. The host Antigravity relay describes sandbox read restrictions verified on macOS, not this Windows environment. Neither was certified by this experiment. Shell discovery found Codex and WSL, but no Docker, Podman or Antigravity command on the current PATH; that is not a complete inventory of installed software.

## Remaining work before bridge execution

Update: the user approved pursuing a container-based runner and requested a clean room process evidence report. The reusable evidence collection process and report template are now linked from the adapter. The runner and report generator are not implemented; no target-specific report can yet be issued. Current shell discovery still finds no Docker/Podman CLI or Docker executable at the checked standard locations; a runnable container engine remains an external prerequisite.

Recommended: add a verified isolated execution runner (for example a container or VM) around the existing delegation bridge. Mount only the approved handoff and implementation output; keep original targets, private evidence, ambient MCP connections and credentials outside the builder's permitted access. Provision required authentication deliberately. Test negative reads, indirect tool/network access, fresh sessions, cancellation and child-process cleanup before enabling execution. A container is not itself proof: its mounts, network and tool access must be tested.

Alternative: retain workflow instructions and use procedural separation without claiming enforced read isolation. This changes the approved requirement and needs the user's explicit decision. A tools-only workflow or an upstream patch would also change the selected design; neither was substituted.

The next integration experiment must compile the external adapter against this pin and demonstrate a real Study -> approved immutable handoff -> fresh delegated Build -> Check/refine -> human acceptance sequence. Source review and checkout checks do not establish that full verification.

## Validation

- Dependency pin and clean checkout: passed.
- File-access demonstration: removed after review; it was not an integration check.
- Node syntax check: passed.
- Bundled skill validator: passed during PR preparation after providing PyYAML in a temporary validation directory outside the plugin. The initial attempt lacked PyYAML.
- Diagnostic guard checks: passed for the clean pinned checkout (expected exit 2), an intentionally mismatched pin (exit 1), and an untracked upstream modification (exit 1). Temporary mutations were restored.
- Approved review follow-up: four CLI regression tests passed for missing/invalid configuration, missing Git, and a real Git status stalled by a test filesystem monitor. Errors are now concise and each Git call has a five-second timeout. The test monitor is stopped during cleanup. Checkout checks still return exit 2 while the bridge is unfinished.
- Shared plugin lifecycle runner: six tests passed for discovery, selection, manifest errors, missing/escaped scripts, timeouts, exit aggregation and empty scans. The actual all-plugin run discovers Craft Anything and returns BLOCKED (exit 2). The manifest registers its diagnostic; the runner does not enable it or install update hooks.
- Combined plugin update command: three integration tests passed using real temporary Git repositories. It synchronizes indexed plugin pins, leaves unrelated submodules alone, preserves conflicting local edits, runs checks after synchronization attempts, and checks plugins without submodules. The actual update command synchronized Craft Anything and automatically returned BLOCKED from its registered check. Total shared tooling tests: nine passed.
- Existing V-cycle checker: passed, 63/63 pre-existing requirements; this does not establish plugin verification or add plugin requirements. A fresh Windows worktree with CRLF specification files triggers 567 pre-existing parser errors. Normalizing only the worktree specification line endings to LF makes the same checker pass; Git confirms no specification content changes. The checker and specifications are outside this PR's diff.
- Upstream build, live agent execution, end-user visual proof and full bridge: not performed; integration is blocked before that stage.
- Host Rust compiler/test/lint/format gates: not run for this dependency/documentation experiment; existing unrelated Rust edits were preserved.

The submodule operation stages `.gitmodules` and its gitlink automatically. The plugin manifest records execution as disabled. The adapter and index expose this state explicitly so future agents do not mistake dependency presence for a functioning bridge. The foundation is prepared on a separate PR branch from `origin/main`, excluding unrelated application changes. The live pilot is deferred.

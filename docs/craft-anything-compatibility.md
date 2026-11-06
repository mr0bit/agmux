# Craft Anything compatibility status

## Status

Checked 2026-11-06. Upstream: `storytold/craft-anything`, version 0.4.0, commit `785539196a1863ee1a49c8cb69639d02f385bbbe`.

**Foundation installed; execution disabled.** No working bridge, verified isolation or end-to-end compatibility yet. Upstream unchanged. Live pilot deferred.

## Upstream findings

Paths below refer to the pinned upstream checkout.

| Finding | Source |
| --- | --- |
| Custom builder adapters supported; host relays need a bridge | `crates/ai/src/cli_agents/adapters.rs`, `crates/ai/src/config.rs` |
| Study automatically enters build; reviewed handoff needs integration | `crates/ai/src/rebuild/mod.rs` |
| Builder receives original-target and capture context | `crates/ai/src/rebuild/brief.rs` |
| Native Antigravity builder bypasses permission prompts | `crates/ai/src/cli_agents/adapters/antigravity.rs` |
| `rebuild_check` needs an active build; not a standalone checker | `crates/ai/src/rebuild/tools.rs` |

## Validation

- Checkout pin/cleanliness, mismatch/dirty-file rejection: passed.
- Shared checker/updater: nine tests passed. Configuration/Git failure handling: four tests passed, including a real stalled Git command.
- Syntax, whitespace and skill validation: passed. V-cycle: 63/63 existing requirements, no new plugin requirement verified.
- Known V-cycle issue: fresh Windows CRLF checkout produces 567 parser errors; LF worktree passes without specification content changes. Parser unchanged.
- Upstream compilation, live agents, container isolation, real rebuild and user POV proof: **not tested**. Host Rust gates not run for this tooling/documentation change.

Registered check verifies only checkout consistency. Exit 2 means the known unfinished bridge remains BLOCKED, not that runtime readiness was tested. Git calls have five-second limits. The unrelated file-access demonstration was removed.

```sh
node .soul/tools/check-plugins.mjs --plugin craft-anything
```

Set `SOUL_GIT` if Git is not on PATH. At the last environment check, Docker/Podman were not found in the searched locations; not a complete software inventory.

## Remaining work

1. Verify container runner: permitted inputs accessible; private evidence, original target and prohibited network/tool access denied.
2. Connect existing delegation lanes; preserve approved handoff, fresh implementer, approval rules, limits and cancellation without modifying upstream.
3. Generate evidence reports; later run the agreed pilot through build, checks and human acceptance.

If an agreed control cannot be preserved, report the conflict before changing the design.

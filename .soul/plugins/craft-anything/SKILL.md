---
name: craft-anything
description: Investigate application reverse engineering or clean room recreation using the optional Craft Anything plugin, including setup, compatibility checks and pinned upgrades. Consult readiness before executing a study or build.
---

# Craft Anything adapter

Read `plugin.json` first. Execution is currently disabled: the compatibility experiment has not established a runner that restricts the implementer to approved inputs. Report that limitation before attempting any study/build/check automation. This skill currently supplies dependency setup and compatibility investigation, not an executable recreation bridge.

## Dependency setup and upgrades

Run commands from the host repository root. Initialize only this optional submodule:

```sh
git submodule update --init -- .soul/plugins/craft-anything/upstream
git -C .soul/plugins/craft-anything/upstream rev-parse HEAD
git -C .soul/plugins/craft-anything/upstream status --porcelain
```

Require HEAD to equal `plugin.json`'s commit and the upstream worktree to be clean. Keep adapter code beside `upstream/`; preserve upstream unchanged. Ordinary engineering workflows do not require this dependency or its build.

For an upgrade, inspect the current pin and working tree, fetch an explicitly chosen candidate, review the diff, and check out its exact commit in detached mode. Never track a moving branch automatically. Build and re-run compatibility experiments before updating the manifest and parent gitlink together. Record failed experiments without declaring the candidate supported. Keep execution disabled until the complete bridge is verified.

The initial diagnostic is:

```sh
node .soul/plugins/craft-anything/compatibility-probe.mjs
```

This checks dependency identity and demonstrates that a child process's working directory alone is not read isolation. It does not test an agent sandbox, authenticate a provider, or certify bridge compatibility.

## Agreed workflow contract for the future bridge

The host orchestrator owns task state, implementation lanes, approvals, verification and final acceptance. Support investigation-only and recreation entry points. Record the target version, platform, scope, important interactions, intended differences, selected inspection mode and iteration limit before study. Ask for an explicit decision to change Clean room/Deep mode.

Keep all target material in the host project's investigation area, outside `.soul`. Separate private inspection artifacts from the handoff. A clean room handoff contains the reviewed behavioural specification and an explicit inventory of permitted observable references: screens, measurements, interaction traces, timing, shortcuts, error states and edge cases. Record unknowns and estimates. Internal source, decompilation, inspection transcripts and recovered assets remain outside that package.

Obtain the user's approval of the precise handoff revision before implementation. Any changed handoff requires renewed approval. Start a fresh implementer with no inspection conversation or resumed session, in an execution environment whose readable inputs have been tested. A new working folder, prompt instruction or workspace-write setting alone is not proof of read isolation. Keep original targets and private evidence inaccessible to the builder, including through inherited MCP tools and network access.

Dispatch implementation through host delegation policy. Route comparisons through the orchestrator; pass back only reviewed observable differences. Bound refinement by the recorded limit, and pause on stalled progress. Upstream Ready status means ready for host verification. Completion requires host engineering gates and the user's acceptance of the running result or documented differences.

If an agreed control cannot be preserved without changing upstream, report the specific conflict and alternatives to the user before proceeding. Do not silently substitute a tool-only workflow or native upstream builders.

## Process evidence report

For every clean room recreation, follow [clean room process evidence](../../process/clean-room-evidence.md) and use its linked report template. Keep the completed report, approved input inventories, actual approval records, isolation probe results, execution records and final acceptance in the host project. Collect evidence throughout the run; generate the report from those records. Pending or failed controls remain visible. A template or builder self-report is not proof of an executed clean room process.

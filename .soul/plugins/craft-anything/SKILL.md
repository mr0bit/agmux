---
name: craft-anything
description: Set up, check or upgrade the optional Craft Anything plugin; consult its readiness and workflow contract for reverse engineering or clean room recreation.
---

# Craft Anything adapter

Read `plugin.json` first. Execution is disabled pending a verified isolated runner and delegation bridge. The current diagnostic checks the upstream pin and demonstrates that a working folder alone is not isolation; it does not certify readiness.

## Setup and upgrades

Follow the shared [plugin lifecycle](../../process/plugin-lifecycle.md), including candidate review and compatibility gates. Keep `upstream/` unchanged and adapter code beside it.

From the host repository root, synchronize accepted pins and automatically check all plugins:

```sh
node .soul/tools/update-plugins.mjs
```

## Workflow contract

- Support investigation-only or recreation. Record target/version, platform, scope, differences and iteration limit. Changing Clean room/Deep mode requires an explicit user decision.
- For recreation, follow [clean room process evidence](../../process/clean-room-evidence.md): user-approved handoff revisions, a fresh implementer with verified input isolation, and an evidence-backed report. Keep project artifacts outside `.soul`.
- The host orchestrator owns delegation, approvals and verification. Return only reviewed observable comparison findings to the builder; pause on stalled progress or the iteration limit.
- Treat upstream Ready as pending host gates and the user's acceptance of the running result.
- If upstream cannot preserve these controls, report the conflict and alternatives before changing the design.

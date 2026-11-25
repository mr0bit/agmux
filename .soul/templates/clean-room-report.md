# Clean room evidence report

> TEMPLATE. Not proof of an executed implementation. Fill from actual records; keep missing/failed results visible.

Run ID / report revision: Pending

Status: NOT EXECUTED / INCOMPLETE / FAILED / COMPLETE

| Record | Evidence / result |
| --- | --- |
| Context | Target/version, platform, scope, intended differences, UTC capture period: Pending |
| Participants | Observer, implementer, inspection mode/changes, private evidence references, handoff reviewer: Pending |
| Approved inputs | Every specification/reference/refinement revision, baseline and dependencies; inventory path, SHA-256, provenance review and actual human approval record: Pending |
| Environment | Image digest, effective configuration/hash, tools/models, delegation lane, mounts, network rules, credential scopes: Pending |
| Isolation | Approved reads/output writes work; private evidence/target reads denied; indirect paths/tools/network tested; fresh session confirmed. Logs and untested paths: Pending |
| Implementation | Each round's approved input digest, dispatch/session record, ordered host events, limits/usage, approvals, stalls/cancellations and comparison results: Pending |
| Audit coverage | Host observations versus builder claims; command capture complete/partial/unavailable: Pending |
| Verification | Engineering commands/results, required behaviour checks and authentic user POV evidence: Pending |
| Final output | File inventory/hash or commit; human acceptance/rejection tied to that revision: Pending |
| Exceptions | Failed controls, missing records, unresolved differences, remediation and evidence: Pending |
| Integrity | Evidence manifest/hash, final bundle inventory, externally retained digest, actual signatures/timestamps if any, verification instructions: Pending |
| Independent review | Reviewer, date and findings; or not reviewed: Pending |

COMPLETE requires mandatory records, passing isolation/engineering checks and final acceptance. Acceptance cannot override failed isolation. Hashes establish consistency, not truth or approval. No claim about prior model knowledge or unobserved activity.

Collection and retention rules: [clean room process evidence](../process/clean-room-evidence.md). Keep completed reports in the host project; exclude private material and secrets from distributable bundles.

# Clean room process evidence

Use this process when a user requests an auditable record of a clean room implementation. Produce a report from the [report template](../templates/clean-room-report.md) and a machine-readable inventory of its supporting artifacts. Keep completed reports and evidence in the host project, outside `.soul`.

## What the report establishes

Describe the controls used, the records collected, the checks performed and the limits of those checks. Call it a **clean room process evidence report**, not an unconditional certificate. File hashes detect changes relative to the recorded inventory; they do not establish authorship, truth, approval or when a record existed. A fresh agent does not establish absence of prior knowledge in its model. State those limitations explicitly.

An investigation may use internal inspection while the implementation consumes only an approved behavioural handoff. Record those as separate activities; do not describe the whole investigation as observation-only when Deep inspection occurred. Human review must establish that every handoff item is permitted observable information; a hash cannot establish that classification.

## Records collected before implementation

1. Assign a run ID. Record the target/version/platform, agreed recreation scope, intended differences, inspection mode, participants and roles, plugin commit, adapter revision, and planned iteration/time limits.
2. Inventory private investigation records separately. Record which observer saw internals and which fresh implementer will receive the handoff. Preserve the provenance of observable descriptions even when the observer also inspected internals.
3. Prepare the handoff with a behavioural specification and permitted reference files. Inventory every relative path, byte size, SHA-256 and provenance. Include UI states, measurements, interaction traces, error cases and known unknowns. Record the implementation baseline and permitted dependencies/toolchain as additional inputs. Reject links or mounts that expose private artifacts indirectly.
4. Serialize the handoff inventory as a concrete file and hash its exact bytes. Capture the user's approval of that digest and scope: retain an actual approval record with actor, capture time and source reference. An agent-written `approved: true` is not a user approval. Show the user the reportable identity and approval wording before recording them. If any approved input changes, create a new revision and obtain approval for the new digest.

## Records collected during execution

The host orchestrator retains the evidence store outside the builder's writable mounts. Before starting an implementer, record the image content digest, effective container configuration, adapter/relay versions, mounts, network policy, agent model/configuration and fresh session creation. Mount the handoff read-only and only the intended project/output paths writable. Keep the container engine socket, host home, source target and investigation store unmounted. Inventory any deliberately provisioned credentials; retain identifiers and scope rather than secret values.

Collect positive and negative isolation checks from the same runner configuration used for the build:

- Read an approved reference and write an implementation output successfully.
- Attempt to read known canaries in private evidence and excluded target locations; require access to fail. Include indirect paths, symlinks and inherited tools in the checks.
- Attempt prohibited network destinations and host/metadata services; record results. Provider access must use a tested restricted route, not unrestricted outbound networking. Specify what remains untested.
- Confirm a new session and absence of observer transcripts, ambient MCP connections and saved sessions in the exposed environment.

For each probe retain the command, exit code, output and tested configuration digest. An untested control remains **not tested**. Stop before implementation on failed mandatory isolation checks.

Record ordered lifecycle events outside the builder's control: UTC capture time, sequence number, event type, round ID and related artifact digests. Retain dispatched briefs, relay outcomes, command/approval events where observable, outputs and comparison results. Distinguish host-observed events from builder self-reports. Record any command-audit gaps rather than claiming every command was observed. Track actual rounds against the limit, cancellation and stalled-progress stops.

Refinement is another input: review observable comparison findings, add their provenance and hashes, and obtain approval of any revised handoff before dispatch. Keep internal inspection findings outside builder feedback.

## Close and seal the record

Record engineering gate commands and results, behavioural comparisons, authentic user POV evidence, unresolved differences, final output hashes/commit and the user's acceptance or rejection of that exact output revision. Record exceptions and failed checks even if later resolved.

Create a host-side `evidence-manifest.json` listing every distributable artifact by relative path, size and SHA-256. Keep private inspection material and credentials out of the distributable bundle; record private retention references separately. Generate the report from this evidence, identifying missing records explicitly. Avoid circular hashing: inventory evidence first, generate the report, then create a final bundle inventory covering the report and evidence manifest. Store its digest outside the bundle through a user-controlled record or optional signing process.

A report is **complete** only when the mandatory records, passing isolation checks, approved handoff revisions, engineering checks and final acceptance exist. Otherwise label it **incomplete**, **failed**, or **not executed** and list the gaps. Human acceptance cannot turn a failed isolation probe into a passing control.

Digital signing and independent timestamping can strengthen integrity and provenance when explicitly configured. Record the signer, verification method and trust assumptions; do not fabricate signatures or treat a timestamp field as independently verified time. Independent review should reproduce manifest hashes, verify actual signatures if present, and trace each report assertion to the retained record.

# Architecture: Live Repository Maps

A live architecture map complements the architecture specification and module
registry with source-backed evidence. It does not authorize new modules, change
ownership rules, or replace requirement traceability and verification.

## Inspect before changing code

When a repository configures Groma:

1. Restore its pinned tooling and scanner packages using the repository's setup
   commands. Read `groma agent-instructions` and the guide for the current job.
2. Scan supported application sources and inspect the owner of each affected
   file. Use the map to investigate callers, collaborators and runtime boundaries.
3. Compare those findings with the approved module registry and architecture
   decisions. Report missing coverage or disagreements; do not silently rewrite
   the approved architecture to match scanner guesses.

## Keep evidence and intent separate

- A first scan is provisional evidence, not a curated architecture. Directories,
  imports and scanner placement alone do not establish runtime boundaries.
- Curate only when requested, following the installed structure, description and
  relationship guides. Read source and specifications before asserting meaning.
- Maintain Groma-owned map records through its CLI. Keep installation settings,
  source coverage and repository-specific commands in the host project.
- Report unsupported syntax, unresolved semantics and scanner findings honestly.
  Findings are review questions, not permission to refactor application code.

## Verify with the change

Rescan after source changes and review map diffs in the same pull request. Verify
that ownership and any approved curation survive the scan, then run the normal
traceability and software checks. If CI checks map freshness, commit the refreshed
map with the source change. A successful scan does not prove the software builds
or behaves correctly.

Keep generated browser exports and scanner caches outside the tracked map.
Exports may include source code; publish them only within the user's authorized
sharing scope. Do not add an architecture tool as a runtime application dependency.

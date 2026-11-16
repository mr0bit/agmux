# Groma architecture workflow

[Groma](https://github.com/MrLesk/groma.md) is installed as repository-local
architecture tooling, integrated with the portable
[live-map process](../../.soul/architecture/live-architecture.md).
The [module registry](module-list.md), SAD, specifications and V-cycle remain
normative. The map in `groma/` is evidence about the implementation.

## Setup and use

Requires Node.js 22+ and npm, matching CI. From the repository root:

```sh
npm ci
npm run arch:setup
npm run arch:check
npm run arch:scan
npm run arch:view
```

`arch:view` opens the local interactive map. No account or AI provider is required.
Scanner setup downloads pinned official packages; subsequent scans run locally.
This tooling is not included in agmux.exe and does not change its release assets.
The frontend retains its separate package manifest and lockfile.

For an agent or a terminal-only session:

```sh
npm exec -- groma agent-instructions
npm run arch:summary
npm exec -- groma view crates/agmux-cli/src/companion_gui/live.rs
npm exec -- groma view crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
npm run arch:lint
```

Before implementation, inspect the affected file owners and compare them with
`module-list.md`. After source changes, run `arch:scan`, review the map diff and
commit it with the code. CI restores the pinned scanners, checks readiness,
rescans and rejects uncommitted changes to the tracked map.

## Pinned setup and coverage

- CLI: `groma.md@0.7.8`, pinned in the root package manifest and lockfile.
- Rust: `@groma/scanner-rust@0.2.3`.
- TypeScript: `@groma/scanner-typescript@0.2.4`.
- React: `@groma/scanner-react@0.2.3`.
- Bundled Backlog work source: enabled in `groma/config.json`; existing tasks and
  their workflow stay in `.backlog/`. Installation creates no new backlog tasks.

Rust covers workspace and application crate manifests and sources. The root
verification manifest redeclares application binary paths, so the Rust scanner
reports duplicate compilation contexts for two entry files. Those files keep
owners, but their declarations and calls are omitted; this is a known coverage
gap, not evidence that they have no dependencies. The frontend scanners cover
its source, package, index and TypeScript configuration. Build artifacts,
third-party dependencies, tests and agent tooling are outside this initial map.
Build scripts, generated Rust, trait dispatch and some React callback bindings
are not fully resolved by these scanners. Read scan diagnostics before assuming
coverage. The lint findings are advisory; installation does not change app code.

## Initial map status

The checked-in map is an initial deterministic scan, not a completed C4 curation.
Names, boundaries and relationships inferred by scanners need review against the
SAD and module registry before architectural decisions rely on them. In particular,
a crate or package is not automatically an independently deployed C4 container.
For a separate curation request, use `groma agent-instructions` and its inspect,
structure, describe and relationships guides. Edit map records through the CLI.

The portable process lives in `.soul`; these package versions, coverage choices
and commands belong here to preserve `.soul` portability. Do not publish map
exports casually: they can contain repository source code.

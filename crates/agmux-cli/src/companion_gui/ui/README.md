# Agmux browser companion

The frontend has two explicitly separate entry points:

- **Live:** `agmux gui` serves built assets and injects a browser-only bootstrap
  credential. The UI uses the Rust bridge's sessions, PTYs, files, metadata, and
  diffs. There are no simulated replies or permission grants in live mode.
- **Design preview:** Vite or the private hosted Site has no bootstrap credential
  and renders the isolated demo workspace. Its **Connect backend** dialog explains
  how to start and open the real local companion. It does not tunnel to localhost.

## Start

Requires current stable Rust, Node.js 22+, npm, and Git. From this directory:

```sh
npm ci
npm run live
```

Open the URL printed by `agmux gui`. Start a Shell session, or select an installed
agent CLI. Agent authentication stays in the CLI on the machine running Rust;
never paste provider keys or daemon authentication tokens into the frontend.

For separate build/run steps:

```sh
npm run build
cargo build -p agmux-cli -p agmux-daemon
cargo run -p agmux-cli -- gui
```

The bridge locates `dist/` in this checkout, or `ui/dist/` beside the executable.
Set `AGMUX_UI_DIST` to an explicit built directory when distributing binaries.
Set `AGMUX_GUI_PORT` to a chosen loopback port, or leave it unset for an ephemeral
port. Without built assets, the CLI retains its original basic companion page.

The existing daemon environment settings (`AGMUX_SOCK`, `AGMUX_AUTH_TOKEN_PATH`,
`AGMUX_DB_PATH`, `AGMUX_CONFIG_PATH`) can isolate a development workspace.

## Live functionality

- Start real installed CLIs or a custom executable; arguments are passed as an
  array, never concatenated into a shell command by the bridge.
- Switch independent sessions; reconnect event streams; retain actual output
  across browser reloads. A daemon crash does not keep agent processes running.
- Send an explicit prompt through Chat or raw keystrokes through xterm.js.
  Interrupt sends Ctrl C. End stops the process and retains a snapshot.
- Send to another session through the daemon message bus. The backend decides
  whether delivery is immediate or queued; a queued reply is not an approval.
- Browse the selected session's workspace, edit registered UTF-8 files in
  CodeMirror, and save to disk. Saving a stale loaded version is rejected. This
  preflight check is not an atomic transaction against unrelated external editors.
- Review real uncommitted/staged/last-commit Git diffs in unified or split view.
  Untracked files are available in Files but are not implied by an empty diff.
- Save titles and pins in backend metadata. Appearance preferences remain local.

Chat renders a real terminal transcript, rather than inferring assistant/tool
roles from a TUI. Structured messages and native permission decisions require
provider-specific adapters. Mode switching, attachments, handoff and automatic
permission approval are intentionally unavailable in live mode. Use Terminal for
native agent settings and approval prompts.

## Development and verification

```sh
npm run build
npm run test:state
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
npm run test:live
```

`test:live` starts an isolated real daemon and companion, creates a temporary Git
workspace, and exercises real browser/HTTP/IPC/PTY interactions. Build the Rust
binaries first. It uses system Chromium when installed, otherwise Playwright's
Chromium. The test fixture binds port 5174 and cleans up its processes/files.
The live screenshot is saved to the repository's `docs/assets/screenshots/`.

The separate `npm test` suite targets design-preview behavior on Vite port 5173;
it excludes live tests. `npm run dev` starts that demo server. Do not use demo
results as evidence of live provider compatibility.

The UI uses React Native Web primitives, React, Motion, xterm.js, CodeMirror and
Prism. Appearance/density, virtualized demo lists, focus view, and reduced-motion
behavior are documented in source and covered by state/design tests.

Read [the integration design](../../../docs/companion_ui_lld.md) for authenticated
transport details, scope boundaries and current limitations.

# agmux

A Rust agentic terminal multiplexer with a browser companion for real local
sessions, terminal input/output, workspace files, and Git review.

## Run the Windows application

Extract the Windows release ZIP and keep `agmux.exe` and `agmuxd.exe` together.
Double-click `agmux.exe` to open the real application in your default browser.
The UI is embedded in the executable; Node.js, a source checkout and a separate
frontend server are not needed. Keep the application process running while using
the UI. `agmux.exe gui` opens it explicitly; `agmux.exe tui` opens the terminal UI.

## Build from source

Install a current stable Rust toolchain, Node.js 22+, npm, and Git. From a clone
of this integration branch:

```sh
cd crates/agmux-cli/src/companion_gui/ui
npm ci
npm run live
```

The browser opens automatically; the `http://127.0.0.1:<port>/` URL is also printed.
The command builds the frontend and both Rust binaries, starts or connects to
`agmuxd`, and serves the frontend from the authenticated companion origin.

Start a **Shell** session to try it without any AI account. Installed agent CLIs
appear in the new-session picker; they reuse their own login and configuration.
A custom executable and a JSON argument array are also supported.

The live UI supports multiple sessions, direct prompt submission, a lossless
interactive terminal with resizing/reconnection, interrupt/end actions, persisted
session titles/pins, inter-agent message delivery, workspace file editing with
stale-save protection, and actual Git diffs. Closing the browser does not stop
sessions. Ending a session retains a screen snapshot.

Chat currently displays the real terminal transcript. Structured tool timelines,
model switching, permission buttons, uploads, and cross-provider handoff are not
implemented; permissions and native CLI settings are handled in Terminal.

The hosted private Site remains a design preview with connection instructions.
It cannot reach a daemon on your computer. The live app binds only to loopback;
remote relay/pairing is separate work. The executable hosts the UI in your browser.

See the [companion guide](crates/agmux-cli/src/companion_gui/ui/README.md) for
configuration, transport contracts, and verification commands.

## Architecture workflow

The repository uses pinned Groma tooling alongside `.soul`. From the root, run
`npm ci`, `npm run arch:setup`, then `npm run arch:view` to open the local map.
See [setup, coverage and architecture rules](docs/archi/groma.md). The checked-in
map is an initial scan; the module registry remains authoritative.

## Windows startup troubleshooting

If v0.1.0 reports `Daemon process exited prematurely ... exit code: 1`, the default
named-pipe lock path has a known bug. In PowerShell, use a regular lock file:

```powershell
$env:AGMUX_LOCK_PATH = Join-Path $env:LOCALAPPDATA "agmux\agmuxd.pid"
.\agmux.exe gui
```

This affects the current PowerShell session. It does not delete or reset data.
The corrected startup implementation stores default named-pipe locks in the local
data directory and includes backend stderr plus a startup-log path on failure.
For a different failure, `agmux doctor` or running `.\agmuxd.exe` directly from
the extracted folder can reveal the underlying diagnostic. Do not post token-file
contents when reporting an issue.

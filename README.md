# agmux

A Rust agentic terminal multiplexer with a browser companion for real local
sessions, terminal input/output, workspace files, and Git review.

## Run the live companion

Install a current stable Rust toolchain, Node.js 22+, npm, and Git. From a clone
of this integration branch:

```sh
cd crates/agmux-cli/src/companion_gui/ui
npm ci
npm run live
```

Open the `http://127.0.0.1:<port>/` URL printed by `agmux gui` in your browser.
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
remote relay/pairing and a packaged desktop application are separate work.

See the [companion guide](crates/agmux-cli/src/companion_gui/ui/README.md) for
configuration, transport contracts, and verification commands.

# Agent discovery and first-run configuration

`LLD-AGENT-DISCOVERY-001`: Discovery searches PATH in order for six supported executable names, checks executable permissions on Unix, and returns both found and missing entries. Configuration generation emits terminal defaults, keybindings and detected agent paths as TOML. Creation uses exclusive file creation so existing user settings are preserved. The CLI init command operates without starting the daemon.

## Gemini CLI on Windows

`LLD-PLUGIN-BUILTIN-ADAPTERS-003`: Gemini retains the compiled-in `gemini`
provider and its normal interactive flags. PATH discovery distinguishes native
Windows executables from npm `.cmd`/`.bat` launchers, excluding npm's extensionless
POSIX script. Unix executable permission checks are unchanged.

`agent_discovery::windows_gemini_launch` resolves a native executable directly or
reads the official npm package's `bin.gemini` entry. It canonicalizes that entry
and requires it to remain inside `node_modules/@google/gemini-cli`. It resolves
Node from the launcher directory or PATH. The PTY manager supplies the entry path
and caller arguments separately to `CommandBuilder`; it invokes no command shell.
The logical session command remains `gemini` for discovery, history and UI naming.
Typed resolution errors explain missing CLI, Node, metadata or executable files.
Other provider launch paths are unchanged.

The live new-session dialog supports Gemini setup, availability refresh and an
optional model passed as `--model`. Duplicate model flags in advanced arguments
are rejected. The CLI handles login, permission prompts and all account secrets
inside the real Terminal tab. No provider credentials are saved in browser state.

Verification includes temporary-installation tests for Windows shim discovery,
native executable preference, official entry resolution, missing Node and escaping
entry paths. A real `@google/gemini-cli@0.68.0` browser test launches `--version`
with model arguments in a PTY and verifies displayed output. Windows release CI
installs that package and repeats the version probe through the actual browser
API and ConPTY. These probes use isolated `GEMINI_CLI_HOME`, require no login, and
do not claim that authenticated Gemini model responses have been tested.

# Module List & Traceability Root-of-Trust — Agmux

## 1. Overview
This document serves as the canonical **root of trust** for all crates and modules across the codebase. Every module listed below corresponds to an atomic implementation boundary in `crates/`, and explicitly maps to the upstream requirements defined in [`docs/specs/`](../specs/).

---

## 2. Crate Architecture Breakdown

```
crates/
├── agmux-proto/     # Layer 2: Shared IPC enums, frame transport, bincode serialization, auth tokens
├── agmux-core/      # Layer 3 & 4: PTY, VT100, state classification, plugins, SQLite, Git, zstd
├── agmux-daemon/    # Layer 3: Background Tokio daemon (agmuxd), IPC listener, bus, locks, federation
├── agmux-tui/       # Layer 1: Ratatui TUI client, Command Palette, viewport widgets
└── agmux-cli/       # Layer 1: Thin IPC CLI commands, daemon spawner, doctor, bootstrapper
```

---

## 3. Component & Module Directory

### 3.1 `agmux-proto` (Framed Transport & Types)
* **`transport`**: Framed length-delimited stream abstraction over Unix sockets and Windows Named Pipes using Bincode 2.0. Owns the existing `ipc.rs` connection/listener helpers, including bounded pre-request recovery from transient Windows pipe contention.
* **`auth`**: 256-bit cookie authentication token generation, loading, and handshake validation.
* **`messages`**: Strongly-typed `ClientToDaemon` and `DaemonToClient` IPC request/response enums.

### 3.2 `agmux-core` (Multiplexer Core & Persistence)
* **`pty_manager`**: Cross-platform pseudo-terminal allocation via `portable-pty`.
* **`wsl_bridge`**: Windows ConPTY bridge mapping to WSL2 `wsl.exe` instances.
* **`vt_parser`**: In-memory VT100 10,000-line screen grid parser and ANSI snapshot generator.
* **`pty_resizer`**: Synchronous terminal window resizing and VT reflow (`TIOCSWINSZ`).
* **`subtab_spawner`**: Child shell sub-tab (`❯`) environment and port slot initialization.
* **`state_machine`**: 7-state event-driven Tokio state machine per session.
* **`child_process_monitor`**: OS child PID inspection via `sysinfo` to detect active compilations.
* **`tail_matcher`**: SIMD `aho-corasick` scanner for Braille spinners, limits, and prompts.
* **`debounce_timer`**: Configurable `--idle-after-ms` and `--blocked-settle-ms` timer coordinators.
* **`shell_classifier`**: Plain shell sub-tab process evaluator (`WORKING` vs `IDLE`).
* **`plugin_api`**: Definition of the zero-allocation `trait AgentPlugin: Send + Sync`.
* **`stuck_detector`**: Tool-specific prompt boundary and held-input evaluation (>10s).
* **`builtin_plugins`**: Compiled implementations for the 14 first-class CLI tools.
* **`declarative_plugin`**: Dynamic parser for user-defined regexes in `agents.toml`.
* **`session_tracker`**: Automatic session UUID capture (flag injection and store watchers).
* **`agent_discovery`**: PATH discovery for supported agent executables and safe Windows Gemini npm-entry resolution.
* **`config_manager`**: First-run configuration generation and creation.
* **`cli_detector`**: Missing CLI binary scanner and installation guidance provider.
* **`storage_engine`**: Bundled SQLite single-writer database wrapper in WAL mode.
* **`migration_manager`**: Embedded atomic schema migrator using `PRAGMA user_version`.
* **`fts_indexer`**: ANSI-strip batch reader flushing lines to trigram `terminal_logs_fts`.
* **`snapshot_compressor`**: Zstandard level 3 ANSI scrollback compressor for dead sessions.
* **`worktree_manager`**: Pure-Rust Git worktree creation, `.worktreeinclude` copier, and port injector.
* **`worktree_safety_gate`**: Dirty working copy validation and automated backup `git stash` creator.
* **`diff_calculator`**: 5-scope diff engine computing hunks between commits/branches/worktrees.
* **`prompt_flattener`**: Line-level review comment compiler producing numbered prompt rounds.
* **`comment_tracker`**: Dual comment status updater (CLI/MCP resolve and diff hunk modification tracker).
* **`scratchpad_notes`**: Persistent multi-page session scratchpad notes stored in SQLite and indexed for search.

### 3.3 `agmux-daemon` (Daemon Server & Bus)
* **`process_supervisor`**: Dedicated OS process group supervisor (`setpgid` / Job Objects).
* **`shutdown_coordinator`**: Clean SIGHUP/SIGTERM signal propagation handler on daemon exit.
* **`instance_lock`**: Cross-platform single-instance daemon lockfile coordinator.
* **`recovery_manager`**: Startup database scanner marking abandoned active sessions as `DEAD`.
* **`subscription_manager`**: Client viewport subscriber and atomic `ScreenSyncFrame` broadcaster.
* **`event_broadcaster`**: Real-time push notification and session delta dispatcher.
* **`hook_listener`**: IPC listener for zero-ambiguity `agmux internal-hook` events.
* **`config_watcher`**: Filesystem watcher (`notify`) dynamically hot-reloading `config.toml`.
* **`message_queue`**: Queued inter-agent message buffer enforcing the `WAITING` approval safety guard.
* **`lock_manager`**: FIFO named resource mutex queue with dead-session auto-eviction.
* **`file_reservation_table`**: Hierarchical root-relative path lock manager with prefix exclusion.
* **`journal_watcher`**: Monotonic 64-bit sequence event log supporting reconnection replays.
* **`kanban_manager`**: SQLite-backed multi-agent task and report coordination engine.
* **`ssh_federation`**: SSH stdio proxy bridge and Tailscale remote host subtree mounter.
* **`network_auth`**: Security cookie handshake validator for remote connections.
* **`teardown_scheduler`**: Self-teardown (`archive_self` / `kill_self`) scheduler triggered on turn completion.
* **`notification_dispatcher`**: Native desktop notification and custom shell notification hook runner with 15s timeout.

### 3.4 `agmux-tui` (Terminal User Interface)
* **`viewport_renderer`**: Ratatui 0.30 + Crossterm 0.29 layout and `tui-term` terminal renderer.
* **`command_palette`**: Centralized Command Palette modal (`Ctrl+Shift+P` / `F1` / `:`) with `nucleo`.
* **`quick_prompt`**: Floating prompt composer (`Space`) with inline modifier hotkeys.
* **`sidebar_filter`**: Attention filter (`w`) and turn acknowledgment (`.`) controller.
* **`split_viewport`**: Pin-and-compare dual viewport split view (`v` / `m`).
* **`onboarding_wizard`**: Zero-session interactive first-run onboarding modal.
* **`search_modal`**: Interactive `Ctrl+F` cross-terminal search modal with auto-scrolling preview and `/` quick filter.
* **`workspace_hierarchy`**: 3-tier Project/Group/Session tree, `⭐ Favorites`, density toggle, archive/restore, and conversation forking.

### 3.5 `agmux-cli` (Stateless CLI Client)
* **`daemon_spawner`**: Transparent background daemon auto-spawner on socket absence.
* **`ipc_client`**: Framed IPC client routing all CLI subcommands to `agmuxd`.
* **`remote_bootstrapper`**: Pinned GitHub release asset downloader with SHA-256 validation.
* **`doctor`**: Diagnostic analyzer verifying PTYs, file permissions, and active tunnels.
* **`agent_context`**: Progressive token-efficient CLI documentation generator (`agmux agent-context`).
* **`mcp_server`**: Built-in opt-in JSON-RPC 2.0 MCP server exposing orchestration tools (`agmux mcp` / `--enable-mcp`).
* **`companion_gui`**: Phase 3 desktop/web companion bridge, `panels open <url|file>`, and `agmux://` deep-link handler.
  * **`companion_gui::live`**: Authenticated loopback HTTP/SSE/WebSocket adapter over public daemon IPC; built UI hosting, process actions, registered files, Git, journal replay and raw PTY streaming.
  * **`companion_gui/ui`**: React Native Web frontend. `LiveApp` and `backend` use only the companion API; `LiveTerminal` renders native PTY frames. The separate design-preview App is isolated from backend state. UI code never accesses daemon internals or provider credentials.
* **`self_updater`**: In-place binary self-updater checking GitHub Releases, validating SHA-256 checksums, and atomically replacing executables (`agmux update` / `--update`).

---

## 4. Software Requirement to Component Mapping Table

| Requirement ID | Requirement Summary | Assigned Software Component / Module |
| :--- | :--- | :--- |
| `REQ-DAEMON-AUTOSPAWN-001` | Auto-spawn `agmuxd` detached on client connection | `crates/agmux-cli::daemon_spawner` |
| `REQ-DAEMON-PROCGROUP-002` | Session OS process group isolation (`setpgid`) | `crates/agmux-daemon::process_supervisor` |
| `REQ-DAEMON-SHUTDOWN-SIG-003` | SIGHUP/SIGTERM propagation on daemon exit | `crates/agmux-daemon::shutdown_coordinator` |
| `REQ-DAEMON-LOCKFILE-004` | Single-instance lockfile enforcement | `crates/agmux-daemon::instance_lock` |
| `REQ-DAEMON-RECOVERY-005` | Post-crash startup recovery marking sessions `DEAD` | `crates/agmux-daemon::recovery_manager` |
| `REQ-IPC-FRAMING-001` | Length-delimited Bincode 2.0 frames over sockets/pipes | `crates/agmux-proto::transport` |
| `REQ-IPC-TOKEN-AUTH-002` | 256-bit cookie token authentication (`daemon.token`) | `crates/agmux-proto::auth` |
| `REQ-IPC-SINGLE-WRITER-003` | CLI subcommands talk exclusively via framed IPC | `crates/agmux-cli::ipc_client` |
| `REQ-IPC-SYNC-FRAME-004` | Atomic `ScreenSyncFrame` streaming on viewport subscription | `crates/agmux-daemon::subscription_manager` |
| `REQ-IPC-EVENT-DISPATCH-005` | Asynchronous notification and delta push dispatch | `crates/agmux-daemon::event_broadcaster` |
| `REQ-PTY-CROSSPLAT-001` | Cross-platform pseudo-terminals via `portable-pty` | `crates/agmux-core::pty_manager` |
| `REQ-PTY-WSL2-BRIDGE-002` | Windows ConPTY to WSL2 execution bridge | `crates/agmux-core::wsl_bridge` |
| `REQ-PTY-VT100-GRID-003` | 10,000-line in-memory VT100 grid parser | `crates/agmux-core::vt_parser` |
| `REQ-PTY-RESIZE-REFLOW-004` | Window resize reflow and OS PTY resize (`TIOCSWINSZ`) | `crates/agmux-core::pty_resizer` |
| `REQ-PTY-SHELL-SUBTAB-005` | Plain shell sub-tabs (`❯`) with port/path injection | `crates/agmux-core::subtab_spawner` |
| `REQ-STATE-7STATE-MODEL-001` | Strictly typed 7-state event-driven state machine | `crates/agmux-core::state_machine` |
| `REQ-STATE-TIER1-HOOKS-002` | Zero-ambiguity agent lifecycle hook listener | `crates/agmux-daemon::hook_listener` |
| `REQ-STATE-TIER2-CHILDPID-003` | OS child PID tree inspection during output pauses | `crates/agmux-core::child_process_monitor` |
| `REQ-STATE-TIER3-TAILMATCH-004` | SIMD `aho-corasick` terminal tail matcher | `crates/agmux-core::tail_matcher` |
| `REQ-STATE-DEBOUNCE-TIMERS-005` | Configurable debounce settle timers (`--idle-after-ms`) | `crates/agmux-core::debounce_timer` |
| `REQ-STATE-SHELL-TRACKING-006` | Dynamic child process tracking for plain shell sub-tabs | `crates/agmux-core::shell_classifier` |
| `REQ-PLUGIN-TRAIT-001` | Zero-allocation `AgentPlugin` Rust trait interface | `crates/agmux-core::plugin_api` |
| `REQ-PLUGIN-STUCK-HEURISTIC-002` | Tool-specific `check_stuck` prompt heuristics (>10s) | `crates/agmux-core::stuck_detector` |
| `REQ-PLUGIN-BUILTIN-ADAPTERS-003` | Compiled Rust adapters for 14 first-class CLI tools | `crates/agmux-core::builtin_plugins` |
| `REQ-PLUGIN-DECLARATIVE-TOML-004` | User declarative adapter support via `agents.toml` | `crates/agmux-core::declarative_plugin` |
| `REQ-PLUGIN-UUID-CAPTURE-005` | Automatic session UUID capture (flag and store watchers) | `crates/agmux-core::session_tracker` |
| `REQ-PLUGIN-MISSING-DETECTOR-006` | Missing CLI binary detector and installation banner | `crates/agmux-core::cli_detector` |
| `REQ-STORAGE-SQLITE-WAL-001` | Single-writer SQLite WAL database (`state.db`) | `crates/agmux-core::storage_engine` |
| `REQ-STORAGE-ATOMIC-MIGRATION-002` | Embedded atomic migrations using `PRAGMA user_version` | `crates/agmux-core::migration_manager` |
| `REQ-STORAGE-FTS5-RETENTION-003` | Rolling trigram FTS5 index with configurable line cap | `crates/agmux-core::fts_indexer` |
| `REQ-STORAGE-ZSTD-SNAPSHOT-004` | Zstandard level 3 ANSI snapshot compression (0 MB RAM) | `crates/agmux-core::snapshot_compressor` |
| `REQ-STORAGE-CONFIG-HOTRELOAD-005` | Dynamic `config.toml` hot-reloading via `notify` | `crates/agmux-daemon::config_watcher` |
| `REQ-TUI-RATATUI-VIEWPORT-001` | Multi-pane Ratatui 0.30 + Crossterm 0.29 TUI client | `crates/agmux-tui::viewport_renderer` |
| `REQ-TUI-COMMAND-PALETTE-002` | Command Palette modal (`Ctrl+Shift+P` / `F1` / `:`) | `crates/agmux-tui::command_palette` |
| `REQ-TUI-QUICK-PROMPT-003` | Floating Quick Prompt bar (`Space`) with modifiers | `crates/agmux-tui::quick_prompt` |
| `REQ-TUI-ATTENTION-FILTER-004` | Attention filter (`w`) and turn acknowledge (`.`) | `crates/agmux-tui::sidebar_filter` |
| `REQ-TUI-SPLIT-COMPARE-005` | Side-by-side split comparison view (`v` / `m`) | `crates/agmux-tui::split_viewport` |
| `REQ-TUI-COLD-START-006` | Zero-session interactive first-run onboarding wizard | `crates/agmux-tui::onboarding_wizard` |
| `REQ-TUI-GLOBAL-SEARCH-007` | Interactive `Ctrl+F` search modal with preview & `/` filter | `crates/agmux-tui::search_modal` |
| `REQ-TUI-SCRATCHPAD-NOTES-008` | Multi-page session scratchpad notes in SQLite & TUI | `crates/agmux-core::scratchpad_notes` |
| `REQ-TUI-WORKSPACE-HIERARCHY-009` | 3-tier hierarchy, favorites, archive/restore & forking | `crates/agmux-tui::workspace_hierarchy` |
| `REQ-GIT-WORKTREE-PROVISION-001` | Worktree provisioning, `.worktreeinclude`, port inject | `crates/agmux-core::worktree_manager` |
| `REQ-GIT-SAFETY-GATE-002` | Worktree removal gate with dirty check and backup stash | `crates/agmux-core::worktree_safety_gate` |
| `REQ-GIT-REVIEW-DIFF-ENGINE-003` | Interactive code review diff engine (`Ctrl+R`, 5 scopes) | `crates/agmux-core::diff_calculator` |
| `REQ-GIT-REVIEW-DISPATCH-004` | Flatten review comments into numbered prompt round (`C`) | `crates/agmux-core::prompt_flattener` |
| `REQ-GIT-DUAL-RESOLUTION-005` | Dual comment resolution (CLI/MCP resolve & hunk diff) | `crates/agmux-core::comment_tracker` |
| `REQ-ORCH-QUEUED-MSG-GUARD-001` | Inter-agent message queuing with `WAITING` prompt guard | `crates/agmux-daemon::message_queue` |
| `REQ-ORCH-DEAD-LOCK-EVICT-002` | Automatic lock/reservation eviction on `DEAD`/`ERRORED` | `crates/agmux-daemon::lock_manager` |
| `REQ-ORCH-FILE-RESERVATION-003` | Repository-relative hierarchical file reservations | `crates/agmux-daemon::file_reservation_table` |
| `REQ-ORCH-EVENT-WATCH-004` | Monotonic SQLite event journal stream (`agmux watch`) | `crates/agmux-daemon::journal_watcher` |
| `REQ-ORCH-KANBAN-REPORT-005` | Shared Kanban task board and worker reports | `crates/agmux-daemon::kanban_manager` |
| `REQ-ORCH-AGENT-CONTEXT-006` | Progressive token-efficient CLI documentation (`agent-context`) | `crates/agmux-cli::agent_context` |
| `REQ-ORCH-MCP-SERVER-007` | Opt-in JSON-RPC 2.0 MCP server (`agmux mcp` / `--enable-mcp`) | `crates/agmux-cli::mcp_server` |
| `REQ-ORCH-SELF-TEARDOWN-008` | Self-teardown scheduling (`archive_self` / `kill_self`) | `crates/agmux-daemon::teardown_scheduler` |
| `REQ-REMOTE-FEDERATION-SSH-001` | Bidirectional SSH stdio proxy and Tailscale federation | `crates/agmux-daemon::ssh_federation` |
| `REQ-REMOTE-BOOTSTRAP-PINNED-002` | Remote binary bootstrapping from GitHub release assets | `crates/agmux-cli::remote_bootstrapper` |
| `REQ-REMOTE-AUTH-TOKEN-003` | Cookie token authentication across remote sockets | `crates/agmux-daemon::network_auth` |
| `REQ-REMOTE-DOCTOR-REPAIR-004` | Diagnostic verification (`doctor`) and repair command | `crates/agmux-cli::doctor` |
| `REQ-GUI-NOTIFICATION-HOOKS-001` | Rich desktop notifications and custom shell hooks | `crates/agmux-daemon::notification_dispatcher` |
| `REQ-GUI-COMPANION-PANELS-002` | Phase 3 companion GUI, `panels open`, and deep-links | `crates/agmux-cli::companion_gui` |
| `REQ-GUI-SELF-UPDATER-003` | Pinned GitHub Releases self-updater and SHA-256 verification | `crates/agmux-cli::self_updater` |
| `REQ-GUI-MOBILE-PUSH-004` | External mobile push relays (ntfy/webhooks) with action replies | `crates/agmux-daemon::notification_dispatcher` |
| `REQ-GUI-DESKTOP-APP-005` | Desktop companion GUI with SSE streaming and Monaco split panels | `crates/agmux-cli::companion_gui` |

The CLI build script (`crates/agmux-cli/build.rs`) packages `companion_gui` UI assets into the executable; it adds no runtime module or daemon dependency.

## Architecture evidence tooling

The root `package.json` / lockfile pin Groma as development tooling.
`groma/config.json` scopes and pins scanners; `groma/` contains their architecture
Markdown records. This is a visualization of the registered modules, not a new
runtime module or a replacement ownership registry. See [Groma workflow](groma.md).

Daemon startup diagnostics are owned by the existing CLI `daemon_spawner` module ([LLD](../../crates/agmux-cli/docs/daemon_spawner_lld.md)); named-pipe lock placement remains owned by daemon `instance_lock`.

## Issue #17 conversation ownership

`crates/agmux-daemon::gemini_chat` owns the approved S4/S6 Gemini ACP subprocess,
authoritative structured messages, model discovery and turn/permission state.
It communicates through typed companion IPC; the browser never receives provider
credentials. `companion_gui` renders those messages and retains Terminal as the
separate native CLI view. This module is introduced for the user-approved slices,
not a new ownership registry.

`crates/agmux-daemon::server` owns authenticated IPC dispatch and native prompt submission.

## Batch 2 shared-conversation contract
The existing `crates/agmux-daemon::gemini_chat` owns one managed provider context
for both Chat and Terminal presentations. `crates/agmux-core::storage_engine`
retains conversation snapshots and lifecycle metadata. Existing companion UI
modules render both views, proposed/applied review and keyboard navigation;
no browser-side provider process or synthetic assistant response is introduced.

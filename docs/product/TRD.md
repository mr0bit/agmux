# **Agmux — Technical Requirements Document (TRD)**

**Version:** 1.3 (Finalized — C1–C7 Locked & Q1–Q17 Grill Decisions Incorporated)  
**Date:** October 28, 2026  
Language & Edition: Rust (2024 Edition, MSRV 1.85+)  
**Target Triples:** x86\_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86\_64-apple-darwin, aarch64-apple-darwin, x86\_64-pc-windows-msvc, aarch64-pc-windows-msvc

# **1\. System Architecture Overview**

**Agmux** uses a **decoupled Client-Daemon architecture** written in pure Rust. Unlike YoanWai/agent-manager and izll/agent-session-manager (which shell out to external tmux or psmux processes and poll capture-pane), agmuxd owns the operating system pseudo-terminals directly via portable-pty and maintains an in-memory VT100/ANSI screen state machine per active session.

# **1.1 High-Level Component Topology**

| Architectural Layer | Component / Binary | Key Responsibilities & Crates |
| :---- | :---- | :---- |
| Layer 1: Stateless Frontend Clients | agmux-tui (Phase 0–2) | Ratatui 0.30 \+ Crossterm 0.29 terminal UI, tui-term Pseudoterminal widget renderer, Nucleo fuzzy search & Command Palette modal (`Ctrl+Shift+P` / `F1` / `:`). |
| Layer 1: Stateless Frontend Clients | agmux-gui (Phase 3\) | Tauri v2 \+ Svelte 5 \+ xterm.js desktop UI, embedded WebView browser & Monaco editor panels, system tray, and agmux:// URI handler. |
| Layer 2: Framed Transport & IPC | agmux-proto | Bi-directional framed IPC using interprocess \+ bincode 2.0 with token authentication (`daemon.token`). Uses Unix Domain Sockets (/run/user/\$UID/agmuxd.sock) on Linux/macOS/WSL, Named Pipes (\\\\.\\pipe\\agmuxd-USERNAME) on Windows, and SSH Stdio / Tailscale mTLS TCP for remote federation. |
| Layer 3: Core Daemon Subsystems | agmuxd (PTY & VT Multiplexer) | Owns OS pseudo-terminals via portable-pty (POSIX PTY & Win32 ConPTY), supervises process groups with clean signal propagation, and maintains an in-memory 10,000-line screen grid via vt100 / libghostty-vt with zero-flicker sync frames. |
| Layer 3: Core Daemon Subsystems | agmuxd (4-Signal State Engine) | Event-driven classifier combining lifecycle hooks, sysinfo child PID trees (including plain shell tracking), aho-corasick terminal tail matching, and modular `AgentPlugin` STUCK detection. |
| Layer 3: Core Daemon Subsystems | agmuxd (Git & Review Manager) | Pure-Rust gix / git2 worktree lifecycle engine with dirty safety gates and stash backups, 5-scope diff engine, and dual comment resolution. |
| Layer 4: Storage & Orchestration Bus | agmuxd (Persistence & Bus) | **Strict single-writer SQLite WAL database** with FTS5 trigram search index (configurable rolling line retention via hot-reloaded `config.toml`), embedded atomic migrations (`PRAGMA user_version`), zstd Level 3 ANSI snapshot compression, journaled event ring buffer (`agmux watch`), CLI IPC router, and opt-in 27-tool JSON-RPC MCP server. |

# **1.2 Locked Architectural Decisions (C1–C7 & Q1–Q17 Implementation Mapping)**

* **C1 (3-Tier Hierarchy \+ Sub-Tabs):** Modelled via projects \-\> self-referencing groups(parent\_group\_id) \-\> self-referencing sessions(parent\_session\_id) in SQLite. Sub-tabs share the parent session's worktree\_path by default.  
* **C2 (Configurable Worktrees):** Controlled by projects.auto\_worktree boolean or per-spawn CreateSessionReq::use\_worktree (Ctrl+T). Copies .worktreeinclude and .env files and injects PORT=\<base\_port \+ slot\_id\>.  
* **C3 (7-State Machine \+ Settle Timers):** Implemented as an event-driven Tokio state machine per session (WORKING, WAITING, STUCK, FINISHED, IDLE, ERRORED, DEAD) with configurable idle\_after\_ms and blocked\_settle\_ms timers.  
* **C4 (Dual Orchestration Bus):** agmuxd handles both binary IPC requests from the agmux CLI (agmux agent-context, agmux watch, agmux lock, agmux report) and JSON-RPC 2.0 MCP requests when a session is launched with \--enable-mcp.  
* **C5 (Context-Aware Keybindings):** In agmux-tui, pressing v inspects selected\_session.state: if State::Dead, dispatches IpcRequest::Revive; otherwise toggles SplitComparisonMode (with Ctrl+W v as an explicit alias). Pressing f opens a 1-key modal (s \-\> ForkTarget::NewSession, t \-\> ForkTarget::NewSubTab).  
* **C6 (Phased Panels):** agmux panels open {path\_or\_url} checks if an agmux-gui client is attached; in TUI mode, files open in \$EDITOR inside a new sub-tab (parent\_session\_id) and URLs open via the OS browser; in Phase 3 GUI mode, renders inside a split Tauri WebView/Monaco pane.  
* **C7 & Q7 (Remote Federation & Auth):** agmux-tui connects to remote hosts via `ssh {host} agmuxd --stdio-proxy`. All IPC/network streams authenticate via `~/.local/share/agmux/daemon.token`. If `agmuxd` is missing on the target, downloads the matching target-triple release binary from GitHub releases verified with SHA-256.  
* **Q1 (Daemon Auto-Spawn & Process Groups):** `agmux` CLI/TUI auto-spawns `agmuxd` detached if no socket exists. Daemon runs each session in its own OS process group and propagates clean SIGHUP/SIGTERM on daemon exit.  
* **Q2 (IPC-Exclusive SQLite Single-Writer):** SQLite `state.db` is opened and queried exclusively by `agmuxd`. All CLI commands are thin IPC clients to eliminate WAL contention and maintain cache coherence.  
* **Q3 & Q11 (FTS5 Rolling Retention & Config Hot-Reload):** Rolling retention limit (default 50,000 lines/session) and settle debounce windows live in `config.toml` (XDG/AppData), hot-reloaded via `notify`.  
* **Q4 & Q8 (Two-Tier CLI Adapter Plugins):** Core exposes `trait AgentPlugin: Send + Sync` in Rust with built-in implementations for 14 CLIs to handle tool-specific ANSI state parsing and STUCK heuristics. Custom agents use declarative `agents.toml` with regex fallbacks.  
* **Q5 (Queued Message WAITING Guard):** Inter-agent messages (`✉`) target-bound to busy sessions are held in queue while the target is in `WAITING` state, auto-delivering only when the session reaches `IDLE` or `FINISHED`.  
* **Q6 (Worktree Teardown Safety Gate):** Deleting/archiving a worktree session validates clean git status; refuses deletion if uncommitted/unpushed changes exist unless `--force` is provided, and automatically generates a backup `git stash`.  
* **Q9 (Universal Command Palette):** `agmux-tui` integrates a Command Palette modal triggered by `Ctrl+Shift+P` (with `F1` and `:` universal fallbacks) searching static commands and dynamic targets with `nucleo`.  
* **Q10 (Zero-Flicker Detached Reconnection):** `agmuxd` responds to client reconnection with a complete screen synchronization frame (cells, cursor position, scrollback offset).  
* **Q12 (Dual Review Comment Resolution):** Supports explicit agent CLI/MCP resolution (`agmux review resolve <id>`) and automated hunk diff modification detection via `line_hash`.  
* **Q13 (Plain Shell Sub-Tab Tracking & Atomic Teardown):** Sub-tabs running shells (`❯`) track child processes for `WORKING`/`IDLE` states; parent sessions and sub-tabs are reaped together atomically.  
* **Q14 (Dead-Session Lock Eviction):** Transitions to `DEAD` or `ERRORED` immediately release all held resource locks and file reservations, waking the next queued waiter.  
* **Q15 (Hierarchical Path Normalization):** File reservations canonicalize against the primary repository root and enforce directory/file prefix collision prevention across concurrent worktrees.  
* **Q16 (Cold Start Interactive Onboarding):** Zero-session first run renders an interactive New Session wizard with auto-detected CLI tools and local Git repository details.  
* **Q17 (Embedded Atomic Migrations):** SQLite schema evolution is managed via embedded incremental migrations using `PRAGMA user_version` inside an atomic startup transaction.

# **2\. Non-Functional & Performance Requirements (SLAs)**

| Metric | Target SLA | Verification Method |
| :---- | :---- | :---- |
| Daemon Idle RAM | Under 12 MB (with 0 active PTYs, 500 DEAD sessions in SQLite) | heaptrack / /proc/self/statm / Windows Process Explorer |
| TUI Client RAM | Under 10 MB | Valgrind Massif / OS Resident Set Size (RSS) |
| Pane Switch Latency | Under 0.5 ms (60–120 FPS render loop) | Criterion benchmark on vt100::Screen pointer swap \+ Ratatui diff draw |
| Global Search (Ctrl+F) Latency | Under 5 ms across 100,000 indexed terminal lines | Criterion benchmark on SQLite fts5(tokenize='trigram') \+ nucleo |
| Dead Session Snapshot Compression | 10,000-line colored scrollback compressed to under 60 KB in under 2 ms | zstd compression ratio & throughput benchmark |
| Idle CPU Usage | 0.0% CPU when all managed PTYs are silent (IDLE/WAITING/DEAD) | Event-driven epoll/kqueue/IOCP via tokio; zero background polling loops when quiet |
| Binary Size | Under 8 MB stripped static release binary (lto \= "thin", strip \= true) | CI build artifact inspection across all 6 target triples |

# **3\. Core Subsystem Technical Specifications**

# **3.1 Cross-Platform PTY & Terminal Emulation Subsystem**

* **PTY Abstraction (portable-pty):**  
  * **Linux & macOS:** Uses POSIX openpty/forkpty with non-blocking async file descriptors registered in Tokio's reactor.  
  * **Windows Native:** Uses the Win32 Pseudo Console API (**ConPTY** via CreatePseudoConsole). Automatically bridges WSL2 sessions when a session profile specifies wsl.exe \-d {distro} \-- {agent\_cmd}.  
  * **Process Group Supervision & Auto-Spawn:** `agmuxd` auto-spawns detached in the background when any client connects if not already running. Each spawned agent runs in an isolated OS process group (`setpgid`). On daemon shutdown, clean SIGHUP/SIGTERM signals propagate to all child process groups to prevent orphaned runaway agent processes.  
* **VT100 State Machine (vt100 / libghostty-vt):**  
  * Each active session maintains a dedicated reader task consuming 8 KB chunks from the PTY master.  
  * Raw bytes are fed into an in-memory vt100::Parser configured with a configurable scrollback ring buffer (default 10,000 lines).  
  * **Zero-Flicker Detached Reconnection:** When a client subscribes (`IpcRequest::SubscribeViewport`), the daemon immediately streams a full screen synchronization frame containing all visible cell attributes, cursor coordinates, and scrollback offset.  
  * **Resize Reflow (TIOCSWINSZ):** When the TUI client resizes or toggles the split-comparison view (v), the client sends IpcRequest::Resize { session\_id, cols, rows }, which resizes both the vt100::Parser grid and the underlying OS PTY.  
  * **Cold Start First-Run UX:** If launched with zero active sessions, `agmux-tui` immediately opens an interactive New Session wizard with auto-detected local Git repository details and discovered installed CLI agents.

# **3.2 Event-Driven 4-Signal State Classification Engine & Plugin System**

Instead of running an expensive 2-second tmux capture-pane polling loop, agmuxd evaluates state transitions **event-driven on PTY byte bursts and silence timers**:

1. **Tier 1 — Native Agent Lifecycle Hooks (Zero-Ambiguity Signal):**  
   * For agents supporting lifecycle hooks (e.g., Claude Code \--settings, Gemini/Codex hooks), agmuxd injects a tiny hook callback (agmux internal-hook \--session \$AGMUX\_SESSION\_ID \--event {start\|stop\|wait\|error}) that writes directly to the daemon's local IPC socket.  
2. **Tier 2 — OS Child Process Tree Signal (sysinfo):**  
   * When PTY output pauses or on a lightweight 1s tick only for active sessions, inspect the agent's PID via sysinfo::System::refresh\_processes\_specifics. If child processes (git, cargo, npm, pytest, bash) or background subagent lines (busy\_line) exist, lock state to WORKING.  
   * *Plain Shell Sub-Tabs:* For sub-tabs running interactive shells (`❯`), evaluate child process trees to transition between `WORKING` (actively running commands) and `IDLE` (waiting at the shell prompt).  
3. **Tier 3 — SIMD Terminal Tail Matcher (aho-corasick \+ regex):**  
   * On every PTY read batch, inspect the bottom 5 visible rows of the vt100::Screen:  
     * **Braille Spinner Detector:** Scans for Unicode Braille block (⠀..⣿) \-\> transitions to WORKING.  
     * **Rate-Limit / Quota Matcher (limit\_line):** Matches rate limit, quota exceeded, 429 Too Many Requests, overloaded \-\> transitions to ERRORED.  
     * **Blocked Prompt Matcher (after \--blocked-settle-ms, default 400ms of silence):** Matches (y/n), \[Y/n\], \[y/N\], Allow, Approve, Do you want to proceed, or trailing ? on prompt line \-\> transitions to WAITING and fires the notification pipeline.  
4. **Tier 4 — Modular Plugin-Driven Held-Input (STUCK) & Idle Settle Timer:**  
   * Core defines `trait AgentPlugin: Send + Sync` in `agmux-core`. Built-in Rust implementations for the 14 standard CLIs implement `check_stuck(&screen, last_input_time, pty_silence)` to evaluate tool-specific prompt boundaries and held unsubmitted text (>10s) without heap allocations.  
   * For user-defined custom agents, declarative regexes and timeout thresholds from `agents.toml` serve as fallback heuristics.  
   * If the prompt is empty and silent for \--idle-after-ms, transition from WORKING to FINISHED (and emit turn-complete notification). Pressing . or entering the pane transitions FINISHED \-\> IDLE.

# **3.3 Dual-Stream Ingestion, SQLite WAL FTS5 Search & zstd Snapshots**

As bytes arrive from each PTY master, the reader task tees the stream into two pipelines:

* **Pipeline A (Live Render):** Raw ANSI bytes \-\> vt100::Parser \-\> dirty cell delta broadcast to subscribed TUI/GUI clients.  
* **Pipeline B (Search & Persistence):** Strips ANSI escape codes via strip-ansi-escapes on newline boundaries and flushes batched lines every 250ms inside a single SQLite WAL transaction (journal\_mode \= WAL, synchronous \= NORMAL, temp\_store \= MEMORY).  
* **Strict Single-Writer Architecture:** SQLite (`state.db`) is opened and managed strictly by `agmuxd`. All CLI subcommands (`agmux task`, `agmux lock`, `agmux report`, `agmux watch`) communicate exclusively over framed IPC (`agmux-proto`), eliminating database contention and locking failures.  
* **Rolling FTS5 Retention & Daemon Hot-Reload:** Enforces a configurable rolling retention limit (default 50,000 lines/session) and periodic WAL vacuuming. Settings live in `$XDG_CONFIG_HOME/agmux/config.toml` (or `%APPDATA%\agmux\config.toml` on Windows) and are hot-reloaded dynamically by `agmuxd` via the `notify` crate.  
* **Embedded Atomic Migrations:** Database schema versions are tracked with `PRAGMA user_version`. At daemon startup, pending embedded incremental SQL migrations are applied in a single atomic transaction.

## **SQLite Database Schema (\~/.local/share/agmux/state.db or %LOCALAPPDATA%\\agmux\\state.db)**

| Table Name | Columns & Types | Purpose & Indexing |
| :---- | :---- | :---- |
| projects | id (TEXT PK), name (TEXT), root\_path (TEXT), auto\_worktree (INTEGER DEFAULT 0), created\_at (INTEGER) | Top-level project workspace registry and worktree automation policy. |
| groups | id (TEXT PK), project\_id (FK), parent\_group\_id (FK self-ref), name (TEXT), sort\_order (INTEGER), is\_archived (INTEGER) | Infinitely nested group/folder hierarchy within a project. |
| sessions | id (TEXT PK), project\_id (FK), group\_id (FK), parent\_session\_id (FK self-ref for sub-tabs), title (TEXT), agent\_tool (TEXT), cli\_session\_uuid (TEXT), model (TEXT), reasoning\_effort (TEXT), profile\_name (TEXT), enable\_mcp (INTEGER), cwd (TEXT), worktree\_path (TEXT), worktree\_branch (TEXT), base\_branch (TEXT), start\_git\_sha (TEXT), diff\_subfolder (TEXT), diff\_hidden\_globs (TEXT), state (TEXT), is\_favorite (INTEGER), is\_archived (INTEGER), scheduled\_teardown (TEXT), last\_user\_prompt (TEXT), last\_agent\_reply (TEXT), ansi\_snapshot (BLOB), created\_at (INTEGER), updated\_at (INTEGER) | Primary session state record, Git worktree metadata, Session Diff base SHA, and zstd-compressed ANSI scrollback blob. |
| session\_notes | id (TEXT PK), session\_id (FK), page\_index (INTEGER), content (TEXT), updated\_at (INTEGER) | Multi-page persistent scratchpad notes per session. |
| terminal\_logs\_fts | session\_id (UNINDEXED), line\_no (UNINDEXED), timestamp (UNINDEXED), plain\_text (FTS5 indexed) | SQLite FTS5 Virtual Table configured with tokenize \= 'trigram' for sub-5ms substring and regex search across all live and dead sessions with rolling prune caps. |
| review\_comments | id (TEXT PK), session\_id (FK), file\_path (TEXT), line\_start (INTEGER), line\_end (INTEGER), line\_hash (TEXT), body (TEXT), status (TEXT: draft, sent, resolved, outdated) | Line-level code review annotations and outdated-line hash tracking. |
| tasks | id (TEXT PK), project\_id (FK), title (TEXT), description (TEXT), status (TEXT: open, claimed, done), claimed\_by\_session\_id (FK), updated\_at (INTEGER) | Shared multi-agent Kanban task coordination board. |

* **Zero-RAM Dead Session Inspection (x \-\> DEAD):**  
  * When KillAndSnapshot is triggered, the daemon calls screen.contents\_formatted() on the vt100::Parser, compresses the byte vector with zstd::encode\_all(bytes, 3), writes it to sessions.ansi\_snapshot, drops the vt100::Parser struct, and closes the PTY master. Memory usage for that session drops to **0 bytes**.  
  * All sub-tabs attached to the session (`parent_session_id`) are reaped atomically.  
  * When the user highlights the DEAD row in the TUI, the daemon streams the decompressed ANSI snapshot to the client's read-only tui-term viewport.

# **3.4 Two-Tier Agent CLI Adapter & Auto-Resume Registry**

To support zero-flag resume (v), conversation forking (f), and modular state classification across heterogeneous CLIs:

1. **Tier 1 — Compiled Rust Plugins (`trait AgentPlugin` in `agmux-core`):** High-performance compiled adapters for the 14 first-class agents (`claude`, `agy`, `gemini`, `codex`, `opencode`, `aider`, `cursor-agent`, `grok`, `q`, `pi`, `omp`, `cmd`, `hermes`, `muse`).  
2. **Tier 2 — Declarative User Adapters (`agents.toml`):** Config-driven adapters for custom or emerging CLIs.

| Agent Profile | Binary | ID Capture Strategy | Spawn / Resume / Fork Arguments | Discovery & Hook Protocol |
| :---- | :---- | :---- | :---- | :---- |
| agents.claude | claude | session\_id\_flag (mints UUID at launch) | Spawn: \--session-id {uuid} / Resume: \--resume {uuid} / Fork: \--resume {uuid} \--fork-session | Hook: claude\_settings\_json |
| agents.agy | agy | session\_store (\~/.antigravity/sessions/\*.json) | Resume: \--resume {uuid} | Store watcher \+ terminal tail rules |
| agents.gemini | gemini | session\_store | Resume: \--resume {uuid} | Discovery: \--acp |
| agents.codex | codex | session\_store | Resume: resume {uuid} | Discovery: app-server |
| agents.opencode | opencode | session\_store | Spawn: \--standalone / Resume: \--session {uuid} | Discovery: opencode models |

# **3.5 Git Worktree & Interactive Code Review Engine (gix / git2)**

* **Worktree Provisioning & Safe Teardown:** Uses pure-Rust gix (gitoxide) / git2 to create worktrees at {repo\_root}/.agmux-worktrees/{branch-slug} (or user-configured path), parses .worktreeinclude (using the ignore crate) to copy untracked .env files, and injects PORT={base\_port \+ slot\_id} into the PTY environment.  
  * **Worktree Safety Gate:** Tearing down or deleting a worktree session checks for uncommitted changes or unpushed commits; refuses deletion unless `--force` is supplied, and automatically generates a safety backup `git stash` prior to removal.  
* **Diff & Line-Comment Prompt Flattener (Ctrl+R \-\> C) & Dual Resolution:**  
  * Computes hunks between sessions.start\_git\_sha (or base branch/index) and the working tree.  
  * When the user presses C to submit draft comments, the daemon formats a deterministic numbered prompt round (e.g., "1. \[ID: rev\_01\] src/pty/manager.rs:142-145: {comment}. After fixing each item, run agmux review resolve {ID}.") and injects the payload directly into the agent's PTY master, transitioning the session state to WORKING.  
  * **Dual Comment Resolution:** Supports explicit agent CLI/MCP resolution (`agmux review resolve <id>`) and automated hunk diff modification tracking (marking comments resolved if the target hunk matching `line_hash` was edited during the agent's turn).

# **3.6 Client-Daemon IPC Protocol & Orchestration Bus**

* **Local Framed Transport with Token Authentication:** interprocess::local\_socket::tokio wrapping length-delimited bincode 2.0 frames with authentication handshake using `~/.local/share/agmux/daemon.token`:  
  * **ClientToDaemon Messages:** SubscribeViewport { session\_id }, SendInput { session\_id, bytes }, QuickPrompt { target\_id, text, image\_chips }, SetState { session\_id, state }, SearchFts { query, limit }, KillSnapshot { session\_id }, Revive { session\_id }, Fork { session\_id, target }, CommandPaletteExec { action }.  
  * **DaemonToClient Messages:** ScreenSyncFrame { session\_id, cells, cursor, scrollback }, ScreenDelta { session\_id, dirty\_cells }, SessionListUpdate { sessions }, SearchResults { hits }, NotificationEvent { session\_id, kind, quote }.  
* **Command Palette Modal (`Ctrl+Shift+P` / `F1` / `:`):** `agmux-tui` provides a VS Code-style Command Palette modal indexing both static actions and dynamic entity navigation via `nucleo` with universal terminal fallbacks.  
* **In-Memory Orchestration Primitives:**  
  * **Hierarchical File Reservation Table:** DashMap\<PathBuf, SessionId\> enforcing optimistic file locks canonicalized relative to the root repository, preventing directory/file prefix collisions across concurrent worktrees.  
  * **Named Resource Lock Queue:** DashMap\<String, MutexQueue\> with Tokio Notify waiters so agmux lock acquire {board\_name} \--wait blocks the calling CLI until the previous holder releases the lock.  
  * **Automatic Dead-Session Lock Eviction:** When any holding session transitions to `DEAD` or `ERRORED`, all held named locks and file reservations are automatically released, waking the next queued waiter.  
  * **Queued Message WAITING Guard:** Queued inter-agent messages (`✉`) destined for busy agents are held in queue while the target session is in the `WAITING` state (e.g., blocked on `(y/n)` or approval prompts), auto-delivering only when the session reaches `IDLE` or `FINISHED`.  
  * **Journaled Event Ring Buffer:** Monotonic 64-bit sequence number log persisted in SQLite (event\_journal table) backing agmux watch \--as {cursor\_name} \--follow.  
  * **Remote SSH Bootstrapping:** When connecting to a remote host via SSH, `agmuxd` verifies or downloads the matching target triple binary from official GitHub release assets using SHA-256 validation.

# **4\. Phased Engineering Roadmap & Workspace Crate Layout**

# **4.1 Cargo Workspace Structure**

| Crate Path | Binary / Library Target | Responsibility |
| :---- | :---- | :---- |
| crates/agmux-proto | Library | Shared IPC message enums, 7-state definitions, bincode 2.0 serialization schemas. |
| crates/agmux-core | Library | PTY pool (portable-pty), vt100 parser, 4-signal state classifier, SQLite FTS5 & zstd snapshot store, Git worktree & diff engine. |
| crates/agmux-daemon | Binary (agmuxd) | Background Tokio daemon, local/remote socket listeners, opt-in MCP JSON-RPC server, watch journal, resource lock manager. |
| crates/agmux-tui | Binary (agmux) | Ratatui \+ Crossterm TUI client, tui-term viewport, Nucleo search overlay, interactive code review UI. |
| crates/agmux-cli | Library / Subcommands | Orchestrator CLI subcommands (agent-context, watch, report, lock, reserve\_files, task) merged into the agmux binary. |
| crates/agmux-gui | Binary (agmux-gui) | Phase 3 Tauri v2 \+ Svelte 5 \+ xterm.js desktop client with embedded WebView browser/editor panels. |

# **4.2 Phased Implementation Milestones**

* **Phase 0 (Weeks 1–3 — Core MVP):**  
  * Implement agmux-proto, agmux-daemon (portable-pty \+ vt100 \+ interprocess), and agmux-tui (Ratatui sidebar \+ tui-term viewport).  
  * Build the 4-signal state classifier (WORKING, WAITING, STUCK, FINISHED, IDLE, ERRORED, DEAD), SQLite WAL \+ fts5(tokenize='trigram') pipeline, Ctrl+F Nucleo search overlay, zstd dead-session snapshot inspector (x), auto-resume adapter registry (v), and Quick Prompt Bar (Space).  
* **Phase 1 (Weeks 4–6 — Git Worktrees, Split View & Interactive Review):**  
  * Add 3-tier hierarchy (Projects, Nested Groups, Sub-Tabs, Favorites \*, Multi-page Notes).  
  * Implement Split Comparison View (v on live session \+ m), automated Git worktree lifecycle (gix/git2, .worktreeinclude, port assignment), and Interactive Code Review mode (Ctrl+R, 5 diff scopes, line comments flattened into prompt rounds via C).  
* **Phase 2 (Weeks 7–9 — Multi-Agent Orchestration & Remote Federation):**  
  * Build the agmux orchestrator CLI (agent-context, watch, report, lock, reserve\_files, task) and opt-in 27-tool MCP server (--enable-mcp).  
  * Implement SSH & Tailscale remote daemon federation (C) with auto-bootstrap and agmux doctor / daemon repair.  
* **Phase 3 (Weeks 10–12 — Tauri v2 Desktop GUI & Mobile Push):**  
  * Build agmux-gui in Tauri v2 attaching to agmuxd, adding embedded WebView browser/editor panels (panels open), header Attention Counter badge, agmux:// deep links, and mobile push notification relays.


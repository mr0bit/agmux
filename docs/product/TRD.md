# **Agmux — Technical Requirements Document (TRD)**

**Version:** 1.2 (Finalized — C1–C7 Decisions Locked, Clean Formatting)  
**Date:** October 28, 2026  
Language & Edition: Rust (2024 Edition, MSRV 1.85+)  
**Target Triples:** x86\_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86\_64-apple-darwin, aarch64-apple-darwin, x86\_64-pc-windows-msvc, aarch64-pc-windows-msvc

# **1\. System Architecture Overview**

**Agmux** uses a **decoupled Client-Daemon architecture** written in pure Rust. Unlike YoanWai/agent-manager and izll/agent-session-manager (which shell out to external tmux or psmux processes and poll capture-pane), agmuxd owns the operating system pseudo-terminals directly via portable-pty and maintains an in-memory VT100/ANSI screen state machine per active session.

# **1.1 High-Level Component Topology**

| Architectural Layer | Component / Binary | Key Responsibilities & Crates |
| :---- | :---- | :---- |
| Layer 1: Stateless Frontend Clients | agmux-tui (Phase 0–2) | Ratatui 0.30 \+ Crossterm 0.29 terminal UI, tui-term Pseudoterminal widget renderer, Nucleo fuzzy search modal. |
| Layer 1: Stateless Frontend Clients | agmux-gui (Phase 3\) | Tauri v2 \+ Svelte 5 \+ xterm.js desktop UI, embedded WebView browser & Monaco editor panels, system tray, and agmux:// URI handler. |
| Layer 2: Framed Transport & IPC | agmux-proto | Bi-directional framed IPC using interprocess \+ bincode 2.0. Uses Unix Domain Sockets (/run/user/\$UID/agmuxd.sock) on Linux/macOS/WSL, Named Pipes (\\\\.\\pipe\\agmuxd-USERNAME) on Windows, and SSH Stdio / Tailscale mTLS TCP for remote federation. |
| Layer 3: Core Daemon Subsystems | agmuxd (PTY & VT Multiplexer) | Owns OS pseudo-terminals via portable-pty (POSIX PTY & Win32 ConPTY) and maintains a 10,000-line in-memory screen grid via vt100 / libghostty-vt. |
| Layer 3: Core Daemon Subsystems | agmuxd (4-Signal State Engine) | Event-driven classifier combining lifecycle hook sockets, sysinfo child PID trees, aho-corasick terminal tail matching, and held-input (STUCK) detection. |
| Layer 3: Core Daemon Subsystems | agmuxd (Git & Review Manager) | Pure-Rust gix / git2 worktree lifecycle engine, .worktreeinclude copier, 5-scope diff engine, and line-comment prompt flattener. |
| Layer 4: Storage & Orchestration Bus | agmuxd (Persistence & Bus) | Bundled SQLite WAL database with FTS5 trigram search index, zstd Level 3 ANSI snapshot compression (0 MB RAM dead sessions), journaled event ring buffer (agmux watch), CLI handler, and opt-in 27-tool JSON-RPC MCP server. |

# **1.2 Locked Architectural Decisions (C1–C7 Implementation Mapping)**

* **C1 (3-Tier Hierarchy \+ Sub-Tabs):** Modelled via projects \-\> self-referencing groups(parent\_group\_id) \-\> self-referencing sessions(parent\_session\_id) in SQLite. Sub-tabs share the parent session's worktree\_path by default.  
* **C2 (Configurable Worktrees):** Controlled by projects.auto\_worktree boolean or per-spawn CreateSessionReq::use\_worktree (Ctrl+T). Copies .worktreeinclude and .env files and injects PORT=\<base\_port \+ slot\_id\>.  
* **C3 (7-State Machine \+ Settle Timers):** Implemented as an event-driven Tokio state machine per session (WORKING, WAITING, STUCK, FINISHED, IDLE, ERRORED, DEAD) with configurable idle\_after\_ms and blocked\_settle\_ms timers.  
* **C4 (Dual Orchestration Bus):** agmuxd handles both binary IPC requests from the agmux CLI (agmux agent-context, agmux watch, agmux lock, agmux report) and JSON-RPC 2.0 MCP requests when a session is launched with \--enable-mcp.  
* **C5 (Context-Aware Keybindings):** In agmux-tui, pressing v inspects selected\_session.state: if State::Dead, dispatches IpcRequest::Revive; otherwise toggles SplitComparisonMode (with Ctrl+W v as an explicit alias). Pressing f opens a 1-key modal (s \-\> ForkTarget::NewSession, t \-\> ForkTarget::NewSubTab).  
* **C6 (Phased Panels):** agmux panels open {path\_or\_url} checks if an agmux-gui client is attached; in TUI mode, files open in \$EDITOR inside a new sub-tab (parent\_session\_id) and URLs open via the OS browser; in Phase 3 GUI mode, renders inside a split Tauri WebView/Monaco pane.  
* **C7 (Remote agmuxd Federation):** agmux-tui connects to remote hosts via ssh {host} agmuxd \--stdio-proxy. If agmuxd is not installed on the remote target, the local client streams the matching target-triple static binary over SCP/SSH to \~/.local/bin/agmuxd.

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
* **VT100 State Machine (vt100 / libghostty-vt):**  
  * Each active session maintains a dedicated reader task consuming 8 KB chunks from the PTY master.  
  * Raw bytes are fed into an in-memory vt100::Parser configured with a configurable scrollback ring buffer (default 10,000 lines).  
  * **Resize Reflow (TIOCSWINSZ):** When the TUI client resizes or toggles the split-comparison view (v), the client sends IpcRequest::Resize { session\_id, cols, rows }, which resizes both the vt100::Parser grid and the underlying OS PTY.

# **3.2 Event-Driven 4-Signal State Classification Engine**

Instead of running an expensive 2-second tmux capture-pane polling loop, agmuxd evaluates state transitions **event-driven on PTY byte bursts and silence timers**:

1. **Tier 1 — Native Agent Lifecycle Hooks (Zero-Ambiguity Signal):**  
   * For agents supporting lifecycle hooks (e.g., Claude Code \--settings, Gemini/Codex hooks), agmuxd injects a tiny hook callback (agmux internal-hook \--session \$AGMUX\_SESSION\_ID \--event {start\|stop\|wait\|error}) that writes directly to the daemon's local IPC socket.  
2. **Tier 2 — OS Child Process Tree Signal (sysinfo):**  
   * When PTY output pauses or on a lightweight 1s tick only for active sessions, inspect the agent's PID via sysinfo::System::refresh\_processes\_specifics. If child processes (git, cargo, npm, pytest, bash) or background subagent lines (busy\_line) exist, lock state to WORKING.  
3. **Tier 3 — SIMD Terminal Tail Matcher (aho-corasick \+ regex):**  
   * On every PTY read batch, inspect the bottom 5 visible rows of the vt100::Screen:  
     * **Braille Spinner Detector:** Scans for Unicode Braille block (⠀..⣿) \-\> transitions to WORKING.  
     * **Rate-Limit / Quota Matcher (limit\_line):** Matches rate limit, quota exceeded, 429 Too Many Requests, overloaded \-\> transitions to ERRORED.  
     * **Blocked Prompt Matcher (after \--blocked-settle-ms, default 400ms of silence):** Matches (y/n), \[Y/n\], \[y/N\], Allow, Approve, Do you want to proceed, or trailing ? on prompt line \-\> transitions to WAITING and fires the notification pipeline.  
4. **Tier 4 — Held-Input (STUCK) & Idle Settle Timer:**  
   * Inspects the active input composer region of the vt100::Screen. If non-empty user/injected text sits after the prompt prefix without a newline for \> 10,000ms while no child process is running, flag held\_input\_present \= true and transition to STUCK.  
   * If the prompt is empty and silent for \--idle-after-ms, transition from WORKING to FINISHED (and emit turn-complete notification). Pressing . or entering the pane transitions FINISHED \-\> IDLE.

# **3.3 Dual-Stream Ingestion, SQLite WAL FTS5 Search & zstd Snapshots**

As bytes arrive from each PTY master, the reader task tees the stream into two pipelines:

* **Pipeline A (Live Render):** Raw ANSI bytes \-\> vt100::Parser \-\> dirty cell delta broadcast to subscribed TUI/GUI clients.  
* **Pipeline B (Search & Persistence):** Strips ANSI escape codes via strip-ansi-escapes on newline boundaries and flushes batched lines every 250ms inside a single SQLite WAL transaction (journal\_mode \= WAL, synchronous \= NORMAL, temp\_store \= MEMORY).

## **SQLite Database Schema (\~/.local/share/agmux/state.db or %LOCALAPPDATA%\\agmux\\state.db)**

| Table Name | Columns & Types | Purpose & Indexing |
| :---- | :---- | :---- |
| projects | id (TEXT PK), name (TEXT), root\_path (TEXT), auto\_worktree (INTEGER DEFAULT 0), created\_at (INTEGER) | Top-level project workspace registry and worktree automation policy. |
| groups | id (TEXT PK), project\_id (FK), parent\_group\_id (FK self-ref), name (TEXT), sort\_order (INTEGER), is\_archived (INTEGER) | Infinitely nested group/folder hierarchy within a project. |
| sessions | id (TEXT PK), project\_id (FK), group\_id (FK), parent\_session\_id (FK self-ref for sub-tabs), title (TEXT), agent\_tool (TEXT), cli\_session\_uuid (TEXT), model (TEXT), reasoning\_effort (TEXT), profile\_name (TEXT), enable\_mcp (INTEGER), cwd (TEXT), worktree\_path (TEXT), worktree\_branch (TEXT), base\_branch (TEXT), start\_git\_sha (TEXT), diff\_subfolder (TEXT), diff\_hidden\_globs (TEXT), state (TEXT), is\_favorite (INTEGER), is\_archived (INTEGER), scheduled\_teardown (TEXT), last\_user\_prompt (TEXT), last\_agent\_reply (TEXT), ansi\_snapshot (BLOB), created\_at (INTEGER), updated\_at (INTEGER) | Primary session state record, Git worktree metadata, Session Diff base SHA, and zstd-compressed ANSI scrollback blob. |
| session\_notes | id (TEXT PK), session\_id (FK), page\_index (INTEGER), content (TEXT), updated\_at (INTEGER) | Multi-page persistent scratchpad notes per session. |
| terminal\_logs\_fts | session\_id (UNINDEXED), line\_no (UNINDEXED), timestamp (UNINDEXED), plain\_text (FTS5 indexed) | SQLite FTS5 Virtual Table configured with tokenize \= 'trigram' for sub-5ms substring and regex search across all live and dead sessions. |
| review\_comments | id (TEXT PK), session\_id (FK), file\_path (TEXT), line\_start (INTEGER), line\_end (INTEGER), line\_hash (TEXT), body (TEXT), status (TEXT: draft, sent, resolved, outdated) | Line-level code review annotations and outdated-line hash tracking. |
| tasks | id (TEXT PK), project\_id (FK), title (TEXT), description (TEXT), status (TEXT: open, claimed, done), claimed\_by\_session\_id (FK), updated\_at (INTEGER) | Shared multi-agent Kanban task coordination board. |

* **Zero-RAM Dead Session Inspection (x \-\> DEAD):**  
  * When KillAndSnapshot is triggered, the daemon calls screen.contents\_formatted() on the vt100::Parser, compresses the byte vector with zstd::encode\_all(bytes, 3), writes it to sessions.ansi\_snapshot, drops the vt100::Parser struct, and closes the PTY master. Memory usage for that session drops to **0 bytes**.  
  * When the user highlights the DEAD row in the TUI, the daemon streams the decompressed ANSI snapshot to the client's read-only tui-term viewport.

# **3.4 Agent CLI Adapter & Auto-Resume Registry (agents.toml)**

To support zero-flag resume (v) and conversation forking (f) across heterogeneous CLIs without hardcoding logic in Rust, Agmux loads adapter definitions from agents.toml:

| Agent Profile | Binary | ID Capture Strategy | Spawn / Resume / Fork Arguments | Discovery & Hook Protocol |
| :---- | :---- | :---- | :---- | :---- |
| agents.claude | claude | session\_id\_flag (mints UUID at launch) | Spawn: \--session-id {uuid} / Resume: \--resume {uuid} / Fork: \--resume {uuid} \--fork-session | Hook: claude\_settings\_json |
| agents.agy | agy | session\_store (\~/.antigravity/sessions/\*.json) | Resume: \--resume {uuid} | Store watcher \+ terminal tail rules |
| agents.gemini | gemini | session\_store | Resume: \--resume {uuid} | Discovery: \--acp |
| agents.codex | codex | session\_store | Resume: resume {uuid} | Discovery: app-server |
| agents.opencode | opencode | session\_store | Spawn: \--standalone / Resume: \--session {uuid} | Discovery: opencode models |

# **3.5 Git Worktree & Interactive Code Review Engine (gix / git2)**

* **Worktree Provisioning:** Uses pure-Rust gix (gitoxide) / git2 to create worktrees at {repo\_root}/.agmux-worktrees/{branch-slug} (or user-configured path), parses .worktreeinclude (using the ignore crate) to copy untracked .env files, and injects PORT={base\_port \+ slot\_id} into the PTY environment.  
* **Diff & Line-Comment Prompt Flattener (Ctrl+R \-\> C):**  
  * Computes hunks between sessions.start\_git\_sha (or base branch/index) and the working tree.  
  * When the user presses C to submit draft comments, the daemon formats a deterministic numbered prompt round (e.g., "1. \[ID: rev\_01\] src/pty/manager.rs:142-145: {comment}. After fixing each item, run agmux review resolve {ID}.") and injects the payload directly into the agent's PTY master, transitioning the session state to WORKING.

# **3.6 Client-Daemon IPC Protocol & Orchestration Bus**

* **Local Framed Transport:** interprocess::local\_socket::tokio wrapping length-delimited bincode 2.0 frames:  
  * **ClientToDaemon Messages:** SubscribeViewport { session\_id }, SendInput { session\_id, bytes }, QuickPrompt { target\_id, text, image\_chips }, SetState { session\_id, state }, SearchFts { query, limit }, KillSnapshot { session\_id }, Revive { session\_id }, Fork { session\_id, target }.  
  * **DaemonToClient Messages:** ScreenDelta { session\_id, dirty\_cells }, SessionListUpdate { sessions }, SearchResults { hits }, NotificationEvent { session\_id, kind, quote }.  
* **In-Memory Orchestration Primitives:**  
  * **File Reservation Table:** DashMap\<PathBuf, SessionId\> enforcing optimistic file locks across agents in the same repository.  
  * **Named Resource Lock Queue:** DashMap\<String, MutexQueue\> with Tokio Notify waiters so agmux lock acquire {board\_name} \--wait blocks the calling CLI until the previous holder releases the lock or its session dies (auto-release on DEAD/ERRORED).  
  * **Journaled Event Ring Buffer:** Monotonic 64-bit sequence number log persisted in SQLite (event\_journal table) backing agmux watch \--as {cursor\_name} \--follow.

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


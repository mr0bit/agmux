# **Agmux — Product Requirements Document (PRD)**

**Version:** 1.2 (Finalized — C1–C7 Locked & Q1–Q17 Grill Decisions Incorporated)  
**Date:** October 28, 2026  
Target Platforms: Linux, macOS, Windows (Native ConPTY \+ WSL2 Bridge)  
**Form Factor:** Cross-Platform TUI (`agmux`) \+ Background Daemon (`agmuxd`), with Phase 3 Desktop/Mobile GUI (`agmux-gui` in `Tauri v2`)

# **1\. Executive Summary & Product Vision**

**Agmux** is an ultra-fast, lightweight (\~10 MB RAM, \~5 MB single binary) cross-platform agentic terminal multiplexer and session orchestrator. It unifies the best capabilities of **YoanWai/agent-manager**, **izll/agent-session-manager (`asmgr`)**, and greenfield-inc/Pane (runpane) into a single cohesive tool without requiring tmux or psmux installed.

Engineers running parallel CLI coding agents (claude, agy, gemini, codex, opencode, aider, cursor-agent, etc.) suffer from terminal sprawl, missed permission prompts, lost conversation context across restarts, and high RAM overhead from idle Node/Python agent processes. Agmux solves this by providing:

1. **Zero-Latency Visual State Monitoring:** Instant visibility into whether every agent is `WORKING`, `WAITING` (blocked on approval/question), `STUCK` (unsubmitted input left in prompt), `FINISHED`, `IDLE`, `ERRORED` (rate-limited/crashed), or `DEAD` (killed to save RAM).  
2. **Instant Cross-Terminal Keyword Search (`Ctrl+F`):** Real-time full-text and fuzzy search across all live and closed agent transcripts and shell scrollbacks, with auto-scrolling read-only context previews.  
3. **Zero-Flag Auto-Resume & Zero-RAM Read-Only Snapshots:** Kill idle agents to free 100% of their RAM while inspecting their full colored history in read-only mode, and revive or fork them with a single keystroke without manually typing `--resume` flags.  
4. **Integrated Git Worktree & Code Review Loop:** Seamless worktree isolation, session-scoped diffs, and inline multi-line code review comments flattened into a single structured agent prompt round.  
5. **Autonomous Multi-Agent Orchestration:** Built-in token-efficient CLI (`agmux`) and opt-in MCP server providing inter-agent messaging, optimistic file reservations, shared Kanban tasks, and distributed resource locks.

# **2\. Target Personas & Core Use Cases**

* **The Parallel AI Architect:** Runs 5 to 20 concurrent CLI agents across microservices or embedded/application layers; needs instant visual alerts when any agent blocks on a `(y/n)` prompt and a 1-key Quick Prompt bar (`Space`) to reply without switching contexts.  
* **The Context Hunter:** Discussed an architectural constraint or register address with an agent 3 days ago; hits `Ctrl+F`, types the keyword, previews the exact highlighted line across 50 closed sessions in under 2ms, and hits `v` to revive that exact conversation.  
* **The Code Reviewer & Orchestrator:** Spawns a builder agent and a QA/review agent inside an isolated Git worktree, compares two competing agent implementations side-by-side (`v` \+ `m`), annotates diff lines (`Ctrl+R`), and dispatches all review comments in one batch (`C`).

# **3\. Finalized Architectural & Design Decisions (C1–C7 Locked & Q1–Q17 Settled)**

All seven original design collisions between the reference tools and seventeen architectural stress-test decisions (Q1–Q17) have been formally resolved and locked into the Agmux v1.2 specification:

### **3.1 Reference Tool Collision Resolutions (C1–C7 Locked)**

| ID | Decision Area | Locked Specification (Approved) |
| :---- | :---- | :---- |
| **C1** | **Workspace Hierarchy Model** | **Hybrid 3-Tier Tree:** Top-level `Project` switcher \-\> Infinitely nested `Groups/Folders` (`g`) \-\> `Session Row` with horizontal **Sub-Tabs** (allowing shared-worktree helper/QA agents and plain shells `❯` to live inside the same session). |
| **C2** | **Git Worktree Automation Policy** | **Configurable Opt-In:** Opt-in per session (`Ctrl+T` in Quick Prompt) with a per-project config flag (`auto_worktree = true/false`), plus automatic `.worktreeinclude` / `.env` copying and deterministic per-worktree `PORT` injection. |
| **C3** | **State Machine Granularity** | **Unified 7-State Machine:** `WORKING`, `WAITING` (needs user input), `STUCK` (held unsubmitted input \>10s), `FINISHED` (turn complete, unacknowledged), `IDLE` (acknowledged/ready), `ERRORED` (rate limit/crash), and `DEAD` (killed, snapshot only), paired with `--idle-after-ms` and `--blocked-settle-ms` debounce timers. |
| **C4** | **Inter-Agent Orchestration Protocol** | **Single Engine, Dual Interface:** `agmuxd` exposes the token-efficient `agmux` CLI by default (with `agmux agent-context` progressive lazy-loaded docs) AND an opt-in 27-tool MCP server flag (`--enable-mcp`) per agent profile. |
| **C5** | **Keybinding Resolution (`v` and `f`)** | **Context-Aware `v` \+ Prompted `f`:** On a `DEAD` session, `v` **Revives**; on a live session, `v` toggles **Split Comparison View** (with `Ctrl+W v` as an explicit alias). Pressing `f` opens a 1-key Fork menu (`s` \= new standalone session, `t` \= new sub-tab). |
| **C6** | **Embedded Browser & Editor Panels** | **Phased Native Delivery:** In **Phase 1–2 (TUI)**, `panels open <file>` launches `$EDITOR` in a sub-tab and `panels open <url>` launches the OS default browser. Full embedded WebView browser and Monaco/CodeMirror panels are delivered in **Phase 3 (`agmux-gui` Tauri v2)**. |
| **C7** | **Remote Host Federation (`C`)** | **Auto-Bootstrapped Remote `agmuxd`:** SSH and Tailscale federation connects the local TUI/GUI to a remote `agmuxd` binary over an SSH stdio tunnel or Tailscale socket, automatically bootstrapping/updating the static `agmuxd` binary on first connect and supporting stateful cursor replay. |

### **3.2 Architectural & Edge-Case Stress-Test Decisions (Q1–Q17 Locked)**

| ID | Topic | Locked Specification |
| :---- | :---- | :---- |
| **Q1** | **Daemon Lifecycle & Process Supervision** | `agmuxd` auto-spawns detached in the background on demand when any client (`agmux` TUI or CLI) connects. Child PTYs run in dedicated process groups and receive clean SIGHUP/SIGTERM on daemon exit to prevent token/RAM leakage. |
| **Q2** | **IPC-Exclusive SQLite Access** | Single-writer architecture: `state.db` is strictly owned by `agmuxd`. All CLI subcommands (`agmux task`, `agmux lock`, `agmux report`, etc.) communicate exclusively through framed IPC (`agmux-proto`), eliminating SQLite `SQLITE_BUSY` lock contention and maintaining cache coherence. |
| **Q3** | **FTS5 Rolling Retention Cap** | Terminal log FTS5 indexing enforces a configurable rolling retention cap (default 50,000 lines per session) and periodic background WAL compaction, configured via `config.toml`. |
| **Q4 & Q8** | **Two-Tier Plugin Architecture for CLI Adapters & STUCK Detection** | STUCK detection and ANSI state parsing are encapsulated per CLI tool. Tier 1: compiled Rust `AgentPlugin` trait implementations for the 14 standard agents (Claude, Codex, Aider, OpenCode, Gemini, etc.) for zero-allocation performance. Tier 2: declarative `agents.toml` configuration for user-defined custom agents. |
| **Q5** | **Queued Message Safety Guard** | Inter-agent messages (`✉`) destined for busy agents are held in queue while the target session is in the `WAITING` state (e.g., blocked on `(y/n)` or approval prompts), auto-delivering only when the session reaches `IDLE` or `FINISHED` to prevent corrupting security prompts. |
| **Q6** | **Worktree Removal Safety Gate** | Teardown, deletion, or archiving of a worktree session blocks if uncommitted changes or unpushed commits exist unless explicit `--force` confirmation is passed, and automatically creates a backup `git stash` before deleting any worktree directory. |
| **Q7** | **IPC Token Authentication & Pinned Remote Bootstrapping** | All local and remote IPC transports authenticate via a filesystem cookie/token (`~/.local/share/agmux/daemon.token`). Remote SSH bootstrapping downloads matching target triple binaries verified with SHA-256 from pinned GitHub release assets. |
| **Q9** | **VS Code-Style Command Palette (`Ctrl+Shift+P` / `F1` / `:`)** | Built-in Command Palette bound to `Ctrl+Shift+P` with `F1` and `:` universal terminal fallbacks, indexing both static actions and dynamic entity navigation via Nucleo fuzzy search. |
| **Q10** | **Zero-Flicker Detached Reconnection** | When reattaching after being detached, `agmuxd` immediately streams a full screen synchronization frame containing the complete `vt100` screen grid and cursor state for instant, zero-flicker reconnect. |
| **Q11** | **Standard Config File & Daemon Hot-Reloading** | Settings live in `$XDG_CONFIG_HOME/agmux/config.toml` (or `%APPDATA%\agmux\config.toml` on Windows). `agmuxd` watches and hot-reloads config dynamically without restarting or dropping sessions. |
| **Q12** | **Code Review Comment Resolution Lifecycle** | Dual resolution supporting both explicit agent commands (`agmux review resolve <id>` / MCP `resolve_review_comment`) and automated hunk-diff tracking based on `line_hash` modifications. |
| **Q13** | **Plain Shell Sub-Tab State & Atomic Teardown** | Plain shell sub-tabs (`❯`) dynamically classify state from child process trees (`WORKING` when running commands, `IDLE` at prompt). Parent sessions and their sub-tabs are reaped together as an atomic lifecycle unit. |
| **Q14** | **Dead-Session Resource Lock Eviction** | When a session transitions to `DEAD` or `ERRORED`, all held named resource locks (`agmux lock`) and optimistic file reservations (`reserve_files`) are automatically released, waking the next queued agent in FIFO order. |
| **Q15** | **Hierarchical Path Normalization for File Reservations** | File reservations canonicalize paths against the primary repository root and enforce directory/file prefix collision prevention across concurrent worktrees. |
| **Q16** | **First-Run Cold Start UX** | Launching `agmux` with zero existing sessions opens an interactive New Session wizard with auto-detected CLI tools and local Git repository details. |
| **Q17** | **Embedded Atomic SQLite Schema Migrations** | Schema evolution is handled via embedded incremental migrations using `PRAGMA user_version` inside an atomic transaction during daemon boot. |

# **4\. Comprehensive Feature Requirements by Phase**

## **Phase 0 (P0 — Core MVP: Multiplexer, Status Engine, Search & Lifecycle)**

### **4.1 Supported Agent Profiles & Dynamic Discovery**

* **FR-P0-01 (Two-Tier CLI Adapter Plugin Architecture):**  
  * **Tier 1 (Built-in Rust Plugins):** Compiled-in zero-allocation `AgentPlugin` implementations for 14 first-class CLI tools: Claude Code (`claude`), Antigravity (`agy`), Gemini CLI (`gemini`), Codex (`codex`), OpenCode (`opencode`), Aider (`aider`), Cursor CLI (`cursor-agent`), Grok (`grok`), Amazon Q (`q`), Pi (`pi`), Oh My Pi (`omp`), Command Code (`cmd`), Hermes (`hermes`), and Muse (`muse`). Encapsulates tool-specific ANSI state parsing, prompt extraction, and STUCK heuristics.  
  * **Tier 2 (Declarative User Plugins):** Custom user-defined CLI profiles registered via `~/.config/agmux/agents.toml` with declarative fields (`prompt_regex`, `stuck_timeout_ms`, `limit_regex`, `resume_cmd`) without requiring Rust recompilation.  
* **FR-P0-02 (Windows `cmd` Collision Guard):** Automatically disable or alias the Command Code (`cmd`) profile on native Windows so it never collides with `cmd.exe`.  
* **FR-P0-03 (Dynamic Model, Effort & Profile Querying):** Query CLIs at startup/spawn (via SDK init, ACP, or RPC flags) to populate filterable dropdowns for **Model**, **Reasoning Effort** (`low`, `medium`, `high`, `max`), and **Profile**. Persist these selections in the session metadata for future restarts, revives, and forks.  
* **FR-P0-04 (Missing-CLI Detector):** Detect if a chosen agent binary is missing from `PATH` and display an interactive install command / link banner.

### **4.2 Visual State Machine & Notification Engine**

* **FR-P0-05 (7-State Visual Indicator & Plugin-Driven Detection):** Display real-time state badges per session row:  
  * `WORKING` (Animated spinner: streaming output, Braille spinner detected, or active child processes / `busy_line` background subagents).  
  * `WAITING` (Highlighted amber/red badge: blocked on permission prompt, `(y/n)`, or direct question).  
  * `STUCK` (Warning icon: unsubmitted text sitting in the agent's input composer box for \>10s, evaluated via the tool's plugin API).  
  * `FINISHED` (Green badge: turn completed cleanly, awaiting user acknowledgment).  
  * `IDLE` (Dim dot: user acknowledged finished turn via `.` or visited pane).  
  * `ERRORED` (Red alert badge: non-zero exit, crash, or rate-limit/quota banner matched via `limit_line`).  
  * `DEAD` (Gray snapshot icon: OS process terminated to free RAM; read-only history available).  
  * *Plain Shell Sub-Tabs:* Sub-tabs running standard shells (`❯`) dynamically classify state from OS child process trees (`WORKING` when executing build/test commands, `IDLE` at prompt rest).  
* **FR-P0-06 (Configurable Settle & Debounce Windows):** Support `--idle-after-ms` (prevents false `FINISHED`/`IDLE` flips while parent agent waits on subagents) and `--blocked-settle-ms` (cancels notification if user answers inline immediately), hot-reloadable from `config.toml`.  
* **FR-P0-07 (Attention Filter & Acknowledge):** Press `w` to filter the sidebar solely to sessions requiring attention (`WAITING`, `FINISHED`, `ERRORED`, `STUCK`). Press `.` to mark a `FINISHED` session as `IDLE` without attaching.  
* **FR-P0-08 (Rich Notifications & Custom Shell Hooks):** Emit native OS desktop notifications on transitions to `WAITING`, `FINISHED`, or `ERRORED` including a 120-character quote of the question/output. Support custom shell notification hooks receiving `AGMUX_NOTIFY_KIND`, `AGMUX_SESSION_ID`, `AGMUX_SESSION_NAME`, `AGMUX_TOOL`, `AGMUX_CWD`, `AGMUX_BRANCH`, `AGMUX_TITLE`, `AGMUX_BODY` (with a 15s execution timeout and a security confirmation prompt whenever the hook command changes in config).

### **4.3 Global Search, Command Palette & Filtering**

* **FR-P0-09 (Global Cross-Terminal Keyword Search `Ctrl+F` & Rolling Retention):** Instant full-text search across all active and closed (`DEAD` / archived) agent transcripts and shell scrollbacks. Highlighting a result in the search modal renders the session's read-only preview pane on the right and auto-scrolls directly to the matched line. FTS5 indexing enforces a configurable rolling retention cap (default 50,000 lines per session) defined in `config.toml`.  
* **FR-P0-09a (VS Code-Style Command Palette `Ctrl+Shift+P` / `F1` / `:`):** Provide a centralized Command Palette modal indexing both static actions (*"Toggle Split View"*, *"Archive Dead Sessions"*, *"New Worktree Session"*) and dynamic entity navigation (*"Jump to Session: <name>"*, *"Switch Model: <model>"*) with Nucleo fuzzy matching. Universal terminal fallbacks (`F1` and `:`) ensure compatibility across all terminal emulators.  
* **FR-P0-10 (Quick Name & Notes Filter `/`):** Vim-style `/` filter bar to instantaneously filter the session tree by session title, branch name, or attached scratchpad notes.

### **4.4 Session Lifecycle, Read-Only Inspect & Auto-Resume**

* **FR-P0-11 (2-Strategy Session ID Capture):** Automatically track conversation UUIDs without user intervention via either (a) pre-minted UUID injection (`session_id_flag`) or (b) filesystem/stdout store watchers (`session_store` for `claude`, `agy`, `gemini`, `codex`, `opencode`, `cursor`).  
* **FR-P0-12 (Kill to 0 MB RAM, Atomic Sub-Tab Reaping & Read-Only Snapshot `x` / `X`):** Press `x` (single session) or `X` (entire group/filtered view) to terminate the underlying OS process and PTY, atomically tearing down all attached sub-tabs (helper shells, QA agents) and compressing the final colored ANSI scrollback buffer into storage. Selecting a `DEAD` session renders the full colored scrollback in a **Read-Only Inspector Viewport** (scroll, copy, search) using 0 MB of agent RAM.  
* **FR-P0-13 (Context-Aware Auto-Resume `v` / `V` & History Browser `r`):** Press `v` on any `DEAD` session (or `V` on a group) to respawn the PTY in its saved `cwd` and automatically pass the native `--resume <session_id>` command, restoring saved model/effort settings. Press `r` to open a chronological Resume Browser across past native CLI sessions with timestamps and last-message previews.  
* **FR-P0-14 (Fresh Context Restart `R` & Copy Last Reply `y`):** Press `R` to restart an agent with a clean context while keeping its name, group, directory, tool, model, and effort. Press `y` from the sidebar to copy the agent's latest structured reply directly to the system clipboard without entering the pane.  
* **FR-P0-14a (Daemon Auto-Spawn & Clean Supervision):** If `agmuxd` is not running, launching `agmux` (or any CLI subcommand) automatically spawns `agmuxd` detached in the background. The daemon isolates each session into its own process group and propagates clean SIGHUP/SIGTERM signals upon graceful shutdown to prevent runaway orphan processes.  
* **FR-P0-14b (First-Run Cold Start UX):** Launching `agmux` with zero existing sessions displays an interactive New Session onboarding wizard pre-populated with detected local Git repo details and detected installed CLI agents, allowing a new session to spawn with `[Enter]`.  
* **FR-P0-14c (Zero-Flicker Detached Reconnection):** When a detached `agmux` TUI reconnects to a running `agmuxd` daemon, `agmuxd` immediately streams a full screen synchronization frame (VT100 grid cells, cursor coordinates, scrollback offset) for instantaneous, zero-flicker reattachment.

### **4.5 Quick Prompt Bar (`Space`)**

* **FR-P0-15 (Inline Reply & Group Spawning):** Press `Space` on an existing session to open a floating Quick Prompt composer and send a reply without attaching; press `Space` on a group to spawn a new session inside that group.  
* **FR-P0-16 (Live Target Switching & Spawn Modifiers):** Use `↑`/`↓` while typing in the Quick Prompt bar to retarget which session/group receives the message. Use inline hotkeys to cycle CLI tool (`Tab`/`Shift+Tab`), model (`Ctrl+L`), reasoning effort (`Ctrl+X`), profile (`Ctrl+Y`), and worktree toggle (`Ctrl+T`).  
* **FR-P0-17 (Prompt Templates & Clipboard Image Chips):** Insert saved prompt templates (`Ctrl+O`) or paste images directly from the system clipboard (`Ctrl+V`) as atomic visual chips inside the composer (`Backspace` deletes the whole chip).

---

## **Phase 1 (P1 — Power Workflow: Hierarchy, Worktrees, Split Compare & Interactive Code Review)**

### **4.6 Workspace Organization, Sub-Tabs & Notes**

* **FR-P1-01 (3-Tier Organization & Favorites `*`):** Support Project Workspaces, infinite nested Group folders (`g`) with drag/keyboard reordering (`J`/`K`), and a pinned `⭐ Favorites` virtual group (`*`) at the top of the sidebar (auto-hidden when empty).  
* **FR-P1-02 (Compact vs. Comfortable Row Density):** Toggle between 1-line Compact rows and 3-line Comfortable rows (displaying Session Title \+ status, user's last prompt prefixed with `❯`, and the agent's last reply line).  
* **FR-P1-03 (Multi-Tab Sessions / Shared-Worktree Panels):** Allow each session row to spawn multiple horizontal sub-tabs (e.g., Builder Agent \+ Review Agent \+ Test Runner Shell `❯` \+ `$EDITOR` via `panels open <file>`), each tracking its own `cwd` or sharing the parent session's Git worktree, plus saved background shell commands.  
* **FR-P1-04 (Multi-Page Session Scratchpad Notes):** Attach persistent multi-page markdown/plain-text notes to any session (`Ctrl+PgUp` / `Ctrl+PgDn`), indexed by `/` and `Ctrl+F`.  
* **FR-P1-05 (Archive & Restore `a` / `u` / `t`):** Archive completed sessions or groups (`a`), hiding them from the active tree while keeping snapshots searchable; toggle archived view (`t`); restore (`u`) and automatically un-archive ancestor groups.  
* **FR-P1-06 (Dual-Target Conversation Forking `f`):** Pressing `f` opens a 1-key Fork menu to clone an active or dead session's conversation history into either a **New Standalone Session (`s`)** or a **New Sub-Tab (`t`)** within the current session.

### **4.7 Side-by-Side Split Comparison (`v` & `m`)**

* **FR-P1-07 (Pin-and-Compare Split Viewport):** Pressing `v` on any live session (or `Ctrl+W v`) splits the main viewport into two live panes; press `m` to pin/mark one agent session in the primary pane while cycling through competing agent sessions in the secondary pane using arrow keys.

### **4.8 Invisible Git Worktree Lifecycle**

* **FR-P1-08 (Automated Worktree Provisioning & `.worktreeinclude`):** When worktree mode is active (via `Ctrl+T` or project `auto_worktree = true`), automatically create an isolated Git worktree and task branch, copy untracked `.env` / `.worktreeinclude` files, and assign a deterministic per-worktree `PORT` environment variable to prevent dev-server port collisions.  
* **FR-P1-09 (Worktree Adoption, Rebase, Squash, Cleanup & Safety Gate):** Adopt externally created worktrees (`adopt`); provide 1-key shortcuts to Rebase worktree branch onto base and Squash commits; batch-archive merged PR worktrees (`archive --merged`) and recreate worktrees on session restore (`u`). **Safety Gate:** Session deletion or auto-teardown refuses to remove a worktree directory if uncommitted changes or unpushed commits exist unless explicit `--force` confirmation is passed, and automatically creates a backup `git stash` before deleting any worktree directory.

### **4.9 Interactive Code Review Mode (`Ctrl+R`)**

* **FR-P1-10 (Smart Monorepo Discovery & Sub-Folder Scoping):** Scan up to 3 directory levels deep when a session runs in a parent folder to auto-select the sub-repo with uncommitted changes or the newest commit; allow manual `diff_folder` scoping and non-destructive `diff_hidden` file exclusion.  
* **FR-P1-11 (5 Diff Scopes & Dual Layout):** Support `Session Diff` (`D` — changes since the session's initial `git HEAD` SHA), `Full Uncommitted` (`F`), `Versus Target Branch` (`B`), `Last Commit`, and `Staged`. Toggle between Unified and Side-by-Side diff views (`u`) while keeping the cursor locked on the exact source line, plus a code-only noise filter (`f`).  
* **FR-P1-12 (Line-Level Review Comments, Single-Round Dispatch `C` & Dual Resolution):** Mark files reviewed (`Space`); attach draft comments to specific diff lines (`c`/`d`). Pressing `C` flattens all pending draft comments into a single numbered prompt round with stable comment IDs and injects it into the agent's PTY while keeping the diff live. Automatically mark comments `outdated` if the target lines are deleted. **Dual Comment Resolution:** Supports both explicit agent commands (`agmux review resolve <id>` / MCP `resolve_review_comment`) and automated hunk-diff tracking based on `line_hash` modifications on subsequent turns.

---

## **Phase 2 (P2 — Autonomous Multi-Agent Orchestration, CLI & Remote Federation)**

### **4.10 Dual Orchestration Interface (`agmux` CLI \+ Opt-In MCP Server)**

* **FR-P2-01 (Single-Writer IPC CLI Architecture with Progressive Docs):** All `agmux` subcommands communicate exclusively with `agmuxd` via framed IPC (`agmux-proto`), ensuring zero SQLite lock contention. Provide `agmux agent-context` (prints a compact 200-token command overview) and `agmux agent-context --command <name>` so orchestrator agents can lazy-load CLI docs without bloating their context window.  
* **FR-P2-02 (Opt-In 27-Tool MCP Server `--enable-mcp`):** When enabled per profile or session, auto-register the built-in MCP server into spawned agents for native tool calling.  
* **FR-P2-03 (Programmatic Session Control, Queued Messaging & WAITING Guard):** Allow parent agents or scripts to `list`, `create`, `read`, `revive`, `kill`, `archive`, and `send` messages to sibling sessions. If the target agent is busy, queue the message (`✉` badge) and deliver automatically once idle, or park via `wait_for_session` without polling. **WAITING Guard:** Messages destined for busy agents are held in queue while the target session is in the `WAITING` state (e.g., blocked on `(y/n)` or approval prompts) and are only delivered once the human resolves or bypasses the prompt (`IDLE`/`FINISHED`).  
* **FR-P2-04 (Self-Teardown Scheduling):** Allow agents to call `archive_self` (`↓` badge) or `kill_self` (`■` badge) to automatically tear down once their current response turn finishes (cancellable by user via `c`).  
* **FR-P2-05 (Shared Kanban Task Board & Structured Worker Reports):** Built-in `task` board (`list`, `create`, `claim`, `finish`, `release`, `delete`) and structured `agmux report` command (`state`, `PR URL`, `head commit`, `summary`, `blocking question`) plus transcript-aware `last-message` extraction.  
* **FR-P2-06 (Hierarchical File Reservations & Dead-Session Lock Eviction):**  
  * `reserve_files` / `release_files`: Prevent two parallel agents in the same checkout from editing the same files simultaneously. Paths are canonicalized against the primary repository root with directory/file prefix collision prevention across concurrent worktrees.  
  * `agmux lock acquire <name> --wait` / `release`: Distributed mutex locks with FIFO wait queues for shared external resources.  
  * **Automatic Eviction:** When any session holding locks or reservations transitions to `DEAD` or `ERRORED`, `agmuxd` immediately releases all held locks and reservations and wakes the next queued waiter.  
* **FR-P2-07 (Journaled Event Stream `agmux watch`):** Stateful cursor-based event streaming (`agmux watch --as <cursor> --follow`) with at-least-once and at-most-once semantics, automatic reconnect replay (`replay: true`, `changedWhileAway: true`), and journal loss alerts.

### **4.11 Remote SSH & Tailscale Federation**

* **FR-P2-08 (Remote Daemon Federation `C`, Token Authentication & Pinned Bootstrap):** Connect to remote Linux/macOS/Windows hosts over SSH (`~/.ssh/config` aware) or Tailscale. All local IPC and remote connections are authenticated via a permission-restricted session token (`daemon.token`). If `agmuxd` is missing on the remote host, automatically bootstrap the target triple binary from official GitHub releases (verified with SHA-256) into `~/.local/bin/agmuxd`, mounting the remote host's groups and sessions as a foldable subtree in the local TUI.  
* **FR-P2-09 (Diagnostics & Self-Healing):** Provide `agmux doctor` and `agmux daemon repair` to verify PTY availability, CLI paths, SQLite integrity, and repair remote tunnels without losing session state.  
* **FR-P2-09 (Diagnostics & Self-Healing):** Provide `agmux doctor` and `agmux daemon repair` to verify PTY availability, CLI paths, SQLite integrity, and repair remote tunnels without losing session state.

---

## **Phase 3 (P3 — Desktop GUI, Embedded Panels & Mobile Push Ecosystem)**

* **FR-P3-01 (Tauri v2 Desktop Client `agmux-gui`):** Lightweight (\~45 MB RAM) native GUI attaching to the exact same `agmuxd` daemon, featuring a persistent header Attention Counter badge with inline reply popups and system tray integration.  
* **FR-P3-02 (Embedded Browser & Code Editor Split Panels):** Support `panels open <url|file>` inside the Tauri v2 client to dock live localhost web previews and Monaco/CodeMirror file editors directly beside agent terminals, plus `agmux://` URI deep-linking from GitHub PRs and issue trackers.  
* **FR-P3-03 (Mobile Push Notifications & Self-Updater):** Relay `WAITING`/`ERRORED` prompts (with 1-tap reply support) to mobile devices via WebPush / `ntfy` / Tailscale companion web UI, plus built-in binary self-updating (`agmux --update`).


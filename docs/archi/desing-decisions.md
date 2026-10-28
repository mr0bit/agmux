# Architectural Decision Records (ADRs) — Agmux

## 1. Overview
This document compiles all foundational architectural decisions, consolidating the resolution of reference tool collisions (C1–C7) and the comprehensive design tree stress tests (Q1–Q17).

---

## 2. Reference Tool Collision Decisions (C1–C7)

### ADR-C1: Hybrid 3-Tier Hierarchy with Session Sub-Tabs
* **Status:** Accepted (Locked)
* **Context:** YoanWai/agent-manager uses flat session lists; `asmgr` uses workspaces and projects; runpane groups by folders.
* **Decision:** Implement a 3-tier hierarchy: Top-level `Project` switcher -> nested `Groups/Folders` -> `Session Row` with horizontal **Sub-Tabs** (allowing builder agents, QA agents, and shell sub-tabs `❯` to share the same session context).
* **Consequences:** Provides deep organization without cluttering the top-level tree; allows multi-agent collaboration within a single worktree.

### ADR-C2: Configurable Opt-In Git Worktrees
* **Status:** Accepted (Locked)
* **Context:** Automatic worktrees can confuse developers working on small scripts, while manual worktrees introduce friction.
* **Decision:** Worktree isolation is opt-in per session (`Ctrl+T` in Quick Prompt) with a project-level default (`auto_worktree = true/false`). When enabled, automatically copy `.worktreeinclude` / `.env` and inject deterministic `PORT`.
* **Consequences:** Eliminates git branch switching conflicts while preserving zero-overhead for non-git workflows.

### ADR-C3: Unified 7-State Classification Machine
* **Status:** Accepted (Locked)
* **Context:** Reference tools either over-simplify states into binary busy/idle or suffer from false positives during background compilation.
* **Decision:** Implement a strictly-typed 7-state model (`WORKING`, `WAITING`, `STUCK`, `FINISHED`, `IDLE`, `ERRORED`, `DEAD`) backed by a 4-signal engine (Hooks, Child PID trees, SIMD Tail Matcher, and Settle Debounce Timers).
* **Consequences:** Zero-ambiguity visibility into agent status with zero battery-draining polling loops.

### ADR-C4: Dual Orchestration Interface (Token-Efficient CLI + Opt-In MCP)
* **Status:** Accepted (Locked)
* **Context:** Full MCP schemas consume 2,000+ prompt tokens per agent turn, bloating context windows and slowing responses.
* **Decision:** Default to a token-efficient CLI (`agmux agent-context` printing <200 tokens) with progressive lazy-loaded command docs, while providing an opt-in 27-tool MCP server (`--enable-mcp`) per profile.
* **Consequences:** Maximizes token efficiency for orchestrator agents while retaining native tool calling for agents optimized for MCP.

### ADR-C5: Context-Aware Keybindings (`v` and `f`)
* **Status:** Accepted (Locked)
* **Context:** Keystroke collisions between Revive, Split View, and Fork across reference tools.
* **Decision:** On a `DEAD` session, `v` revives; on a live session, `v` toggles Split Comparison View (`Ctrl+W v` as explicit alias). Pressing `f` opens a 1-key Fork menu (`s` = new session, `t` = new sub-tab).
* **Consequences:** Intuitive single-key workflows without keybinding ambiguity.

### ADR-C6: Phased Embedded Panels
* **Status:** Accepted (Locked)
* **Context:** Embedding full browser engines and code editors inside a lightweight terminal multiplexer blows the 10 MB RAM budget.
* **Decision:** In Phase 0–2 (TUI), `panels open <file>` launches `$EDITOR` in a sub-tab and URLs open in the OS browser. Embedded WebViews and Monaco editors are delivered in Phase 3 (`agmux-gui` in Tauri v2).
* **Consequences:** Keeps the daemon and TUI strictly within the 10 MB RAM SLA.

### ADR-C7: Remote Host Federation via Auto-Bootstrapped `agmuxd`
* **Status:** Accepted (Locked)
* **Context:** Managing remote agents over flaky SSH terminal sessions results in lost context and high latency.
* **Decision:** Connect local clients to a remote `agmuxd` binary over SSH stdio tunnels or Tailscale sockets, automatically bootstrapping the static binary from release assets if missing.
* **Consequences:** Seamless local rendering of remote agent sessions with offline resilience and stateful cursor replay.

---

## 3. Stress-Test Architectural Decisions (Q1–Q17)

### ADR-Q1: Daemon Auto-Spawn & Process Group Supervision
* **Status:** Accepted (Locked)
* **Decision:** `agmuxd` auto-spawns detached in the background on demand. Sessions run in distinct OS process groups (`setpgid`), receiving clean SIGHUP/SIGTERM signals on daemon exit to prevent orphan processes.

### ADR-Q2: Strict Single-Writer SQLite IPC Architecture
* **Status:** Accepted (Locked)
* **Decision:** `state.db` is strictly private to `agmuxd`. All CLI subcommands (`agmux task`, `agmux lock`, `agmux report`) communicate exclusively over framed IPC (`agmux-proto`), eliminating WAL lock contention.

### ADR-Q3 & ADR-Q11: Rolling FTS5 Retention & Daemon Hot-Reload
* **Status:** Accepted (Locked)
* **Decision:** Terminal log FTS5 indexing enforces a rolling retention cap (default 50,000 lines/session), configured via `config.toml`, which `agmuxd` watches and hot-reloads via `notify`.

### ADR-Q4 & ADR-Q8: Two-Tier CLI Adapter Plugin Architecture
* **Status:** Accepted (Locked)
* **Decision:** Core exposes `trait AgentPlugin: Send + Sync` implemented in compiled Rust for 14 standard tools (handling tool-specific STUCK heuristics and ANSI parsing), with declarative `agents.toml` fallback for custom user agents.

### ADR-Q5: Queued Message WAITING Safety Guard
* **Status:** Accepted (Locked)
* **Decision:** Inter-agent messages (`✉`) destined for busy agents are held in queue while the target session is in `WAITING` state, auto-delivering only after prompt resolution (`IDLE`/`FINISHED`) to prevent corrupting security prompts.

### ADR-Q6: Worktree Teardown Safety Gate
* **Status:** Accepted (Locked)
* **Decision:** Deleting or archiving a worktree session validates clean git status, refusing removal if uncommitted/unpushed changes exist unless `--force` is used, and automatically saving a backup `git stash`.

### ADR-Q7: IPC Token Authentication & SHA-256 Remote Bootstrap
* **Status:** Accepted (Locked)
* **Decision:** All local IPC and remote network connections authenticate via `~/.local/share/agmux/daemon.token`. Remote SSH bootstrapping verifies target binaries against official GitHub release SHA-256 checksums.

### ADR-Q9: Universal Command Palette (`Ctrl+Shift+P` / `F1` / `:`)
* **Status:** Accepted (Locked)
* **Decision:** Centralized Command Palette modal indexing both static actions and dynamic targets with `nucleo`, supporting `F1` and `:` fallbacks for terminal emulators without CSI-u support.

### ADR-Q10: Zero-Flicker Detached Reconnection
* **Status:** Accepted (Locked)
* **Decision:** `agmuxd` streams an atomic `ScreenSyncFrame` (full VT100 grid cells, cursor coordinates, scrollback offset) upon client reattachment for instant, flicker-free rendering.

### ADR-Q12: Dual Code Review Comment Resolution
* **Status:** Accepted (Locked)
* **Decision:** Supports both explicit agent commands (`agmux review resolve <id>` / MCP) and automated diff hunk modification tracking based on `line_hash`.

### ADR-Q13: Plain Shell Sub-Tab Tracking & Atomic Teardown
* **Status:** Accepted (Locked)
* **Decision:** Sub-tabs running shells (`❯`) track child processes for `WORKING`/`IDLE` states; parent sessions and sub-tabs are reaped together atomically.

### ADR-Q14: Automatic Dead-Session Lock Eviction
* **Status:** Accepted (Locked)
* **Decision:** Transitions to `DEAD` or `ERRORED` immediately release all held resource locks and file reservations, waking the next queued waiter in FIFO order.

### ADR-Q15: Hierarchical Path Normalization for File Reservations
* **Status:** Accepted (Locked)
* **Decision:** Reservations canonicalize against the primary repository root and enforce directory/file prefix collision prevention across concurrent worktrees.

### ADR-Q16: Zero-Session Cold Start Onboarding
* **Status:** Accepted (Locked)
* **Decision:** Launching `agmux` with zero existing sessions renders an interactive New Session wizard with auto-detected CLI tools and local Git repository details.

### ADR-Q17: Embedded Atomic SQLite Migrations
* **Status:** Accepted (Locked)
* **Decision:** Database schema evolution is managed via embedded incremental migrations using `PRAGMA user_version` inside an atomic transaction during daemon boot.

# Software Requirements Specification: State Machine & Classification Engine

## 1. Overview
This specification details the event-driven 4-signal agent state classification engine, settle debounce windows, and lifecycle transitions across the unified 7 states (`WORKING`, `WAITING`, `STUCK`, `FINISHED`, `IDLE`, `ERRORED`, `DEAD`).

---

## 2. Requirements

### REQ-STATE-7STATE-MODEL-001
* **Unique ID:** `REQ-STATE-7STATE-MODEL-001`
* **Text:** As a user monitoring concurrent agents, the system shall maintain a strictly typed 7-state machine per session: `WORKING` (actively computing/streaming), `WAITING` (blocked on user approval/prompt), `STUCK` (held unsubmitted text >10s), `FINISHED` (completed turn awaiting acknowledgment), `IDLE` (turn acknowledged or shell ready), `ERRORED` (crashed, non-zero exit, or rate-limited), and `DEAD` (process reaped, read-only snapshot stored).
* **Criticality:** High
* **SW Component:** `crates/agmux-core::state_machine`
* **Implementation Status:** Verified
* **Verification Criteria:** Drive an agent session through all 7 states; verify that every transition generates a valid typed `StateChangeEvent` broadcast over IPC with zero illegal state flips.
* **Downstream Req:** `LLD-STATE-7STATE-MODEL-001`
* **Downstream Test:** `TEST-VER-STATE-001`
* **Test Status:** Passed

---

### REQ-STATE-TIER1-HOOKS-002
* **Unique ID:** `REQ-STATE-TIER1-HOOKS-002`
* **Text:** As an agent CLI supporting lifecycle notifications (e.g. Claude Code settings hooks), the system shall accept IPC hook messages via `agmux internal-hook --session $AGMUX_SESSION_ID --event {start|stop|wait|error}` to immediately transition state with zero parsing ambiguity.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::hook_listener`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Send a hook event from within a running session; verify that `agmuxd` transitions the session state within 0.1ms without waiting for terminal silence or tail inspection.
* **Downstream Req:** `LLD-STATE-TIER1-HOOKS-002`
* **Downstream Test:** `TEST-VER-STATE-002`
* **Test Status:** Failed

---

### REQ-STATE-TIER2-CHILDPID-003
* **Unique ID:** `REQ-STATE-TIER2-CHILDPID-003`
* **Text:** As an agent running background build or test commands (e.g., `cargo test`, `npm build`, `pytest`), the system shall inspect the agent's child process tree using `sysinfo` on output pauses; if child processes are active, the state shall remain locked to `WORKING` regardless of terminal silence.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::child_process_monitor`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Simulate an agent launching a 5-second silent compilation command; verify that state remains `WORKING` throughout the silent compilation window and does not prematurely transition to `FINISHED` or `IDLE`.
* **Downstream Req:** `LLD-STATE-TIER2-CHILDPID-003`
* **Downstream Test:** `TEST-VER-STATE-003`
* **Test Status:** Failed

---

### REQ-STATE-TIER3-TAILMATCH-004
* **Unique ID:** `REQ-STATE-TIER3-TAILMATCH-004`
* **Text:** As PTY byte bursts arrive, the system shall inspect the bottom 5 visible rows of the VT100 screen using SIMD `aho-corasick` and regex to detect Unicode Braille spinners (`WORKING`), rate-limit/quota banners (`ERRORED`), and blocked prompts (`(y/n)`, `Allow`, `Approve`, `?`) transitioning to `WAITING`.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::tail_matcher`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Feed ANSI strings containing `[y/N]`, `Rate limit exceeded`, and Braille animation frames into the parser; verify that state transitions accurately match each token within 5ms.
* **Downstream Req:** `LLD-STATE-TIER3-TAILMATCH-004`
* **Downstream Test:** `TEST-VER-STATE-004`
* **Test Status:** Failed

---

### REQ-STATE-DEBOUNCE-TIMERS-005
* **Unique ID:** `REQ-STATE-DEBOUNCE-TIMERS-005`
* **Text:** As a user configuring state responsiveness, the system shall evaluate state transitions using `--idle-after-ms` (debounce window preventing false completion flips while waiting on subagents) and `--blocked-settle-ms` (silence window before firing blocked prompt notifications), loaded and hot-reloaded from `config.toml`.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::debounce_timer`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Rapidly stream output punctuated by 200ms pauses; verify that with `--idle-after-ms 1000`, the session remains `WORKING` continuously without fluttering between states.
* **Downstream Req:** `LLD-STATE-DEBOUNCE-TIMERS-005`
* **Downstream Test:** `TEST-VER-STATE-005`
* **Test Status:** Failed

---

### REQ-STATE-SHELL-TRACKING-006
* **Unique ID:** `REQ-STATE-SHELL-TRACKING-006`
* **Text:** As a user running plain shell sub-tabs (`❯`), the system shall dynamically track active child processes to mark the shell sub-tab as `WORKING` when executing commands and `IDLE` when returned to the command prompt.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::shell_classifier`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Run `sleep 2` inside a plain shell sub-tab; verify state is `WORKING` for 2 seconds, then immediately flips to `IDLE` upon returning to prompt.
* **Downstream Req:** `LLD-STATE-SHELL-TRACKING-006`
* **Downstream Test:** `TEST-VER-STATE-006`
* **Test Status:** Failed

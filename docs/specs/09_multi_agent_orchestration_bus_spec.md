# Software Requirements Specification: Multi-Agent Orchestration Bus & CLI

## 1. Overview
This specification details the autonomous multi-agent orchestration primitives, queued messaging with WAITING safety guards, automatic lock eviction, hierarchical file reservations, and the journaled event stream.

---

## 2. Requirements

### REQ-ORCH-QUEUED-MSG-GUARD-001
* **Unique ID:** `REQ-ORCH-QUEUED-MSG-GUARD-001`
* **Text:** As a parent or sibling agent sending an asynchronous message to a target session (`agmux send <target_id> <message>`), if the target session is in the `WAITING` state (blocked on permission prompt or `(y/n)`), the system shall hold the message in queue (`✉` badge) and auto-deliver it only after the human resolves or bypasses the prompt (`IDLE`/`FINISHED`), preventing corruption of security prompts.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::message_queue`
* **Implementation Status:** Verified
* **Verification Criteria:** Place a target session in `WAITING` on a `(y/n)` prompt; send an inter-agent message via `agmux send`. Verify that the message is queued without writing to PTY stdin. Resolve the prompt as the user; verify that the queued message is delivered cleanly once the session finishes the turn.
* **Downstream Req:** `LLD-ORCH-QUEUED-MSG-GUARD-001`
* **Downstream Test:** `TEST-VER-ORCH-001`
* **Test Status:** Passed

---

### REQ-ORCH-DEAD-LOCK-EVICT-002
* **Unique ID:** `REQ-ORCH-DEAD-LOCK-EVICT-002`
* **Text:** As an agent holding named resource locks (`agmux lock`) or file reservations (`reserve_files`) transitions to `DEAD` or `ERRORED`, `agmuxd` shall automatically release all of that session's held locks and reservations and wake the next queued agent in FIFO order.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::lock_manager`
* **Implementation Status:** Verified
* **Verification Criteria:** Agent 1 acquires lock `db_migrate`; Agent 2 calls `agmux lock acquire db_migrate --wait`; kill Agent 1 (`kill -9`). Verify that `agmuxd` evicts Agent 1's lock and Agent 2 immediately acquires the lock without manual intervention.
* **Downstream Req:** `LLD-ORCH-DEAD-LOCK-EVICT-002`
* **Downstream Test:** `TEST-VER-ORCH-002`
* **Test Status:** Passed

---

### REQ-ORCH-FILE-RESERVATION-003
* **Unique ID:** `REQ-ORCH-FILE-RESERVATION-003`
* **Text:** As parallel agents work in different worktrees of the same repository, the system shall canonicalize file reservation paths relative to the primary repository root and enforce directory/file prefix exclusion (e.g. reserving `src/` blocks reserving `src/lib.rs` and vice versa), preventing concurrent edits to identical files.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::file_reservation_table`
* **Implementation Status:** Verified
* **Verification Criteria:** Agent A in worktree 1 reserves `crates/core/src/`; Agent B in worktree 2 attempts to reserve `crates/core/src/state.rs`. Verify that Agent B's reservation is rejected with an active reservation conflict error.
* **Downstream Req:** `LLD-ORCH-FILE-RESERVATION-003`
* **Downstream Test:** `TEST-VER-ORCH-003`
* **Test Status:** Passed

---

### REQ-ORCH-EVENT-WATCH-004
* **Unique ID:** `REQ-ORCH-EVENT-WATCH-004`
* **Text:** As an orchestrator script monitoring workspace state (`agmux watch --as <cursor> --follow`), the system shall stream journaled events backed by SQLite monotonic sequence numbers, supporting reconnection replay (`replay: true`, `changedWhileAway: true`) without dropping events.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::journal_watcher`
* **Implementation Status:** Verified
* **Verification Criteria:** Start `agmux watch --as test_cursor`; disconnect client; trigger 5 session state transitions; reconnect `agmux watch --as test_cursor`. Verify that all 5 missed events are replayed in exact monotonic sequence order.
* **Downstream Req:** `LLD-ORCH-EVENT-WATCH-004`
* **Downstream Test:** `TEST-VER-ORCH-004`
* **Test Status:** Passed

---

### REQ-ORCH-KANBAN-REPORT-005
* **Unique ID:** `REQ-ORCH-KANBAN-REPORT-005`
* **Text:** As autonomous agents coordinate complex workflows, the system shall provide shared Kanban task management (`agmux task {list|create|claim|finish|release}`) and structured worker reporting (`agmux report`) persisted in SQLite and queryable via CLI and opt-in MCP.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::kanban_manager`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Agent 1 claims task `T-101`; posts a structured report with PR URL and head commit SHA; marks task finished. Verify that the task status and report are queryable via `agmux task list --json` and `agmux report view T-101`.
* **Downstream Req:** `LLD-ORCH-KANBAN-REPORT-005`
* **Downstream Test:** `TEST-VER-ORCH-005`
* **Test Status:** Failed

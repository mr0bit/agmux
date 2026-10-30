# LLD-ORCH-KANBAN-REPORT-005: Autonomous Kanban Board & Worker Reporting

* **Unique ID:** `LLD-ORCH-KANBAN-REPORT-005`
* **Upstream Requirement:** `REQ-ORCH-KANBAN-REPORT-005`
* **Downstream Verification Test:** `TEST-VER-ORCH-005` (`tests/e2e_kanban_report_test.rs`)
* **SW Component:** `crates/agmux-daemon::kanban_manager`

## 1. Overview
`KanbanManager` coordinates shared Kanban task lifecycles (`todo` ➔ `in_progress` ➔ `done`) and structured worker completion reports (`WorkerReport`) persisted in SQLite via `StorageEngine`, emitting monotonic journal events through `JournalWatcher`.

## 2. Persistence Schema (`StorageEngine`)
- `kanban_tasks`:
  - `id TEXT PRIMARY KEY`
  - `title TEXT NOT NULL`
  - `description TEXT NOT NULL`
  - `status TEXT NOT NULL` (`"todo"`, `"in_progress"`, `"done"`)
  - `claimed_by TEXT`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
- `worker_reports`:
  - `id INTEGER PRIMARY KEY AUTOINCREMENT`
  - `task_id TEXT NOT NULL`
  - `session_id TEXT NOT NULL`
  - `summary TEXT NOT NULL`
  - `pr_url TEXT`
  - `commit_sha TEXT`
  - `created_at INTEGER NOT NULL`

## 3. State Transitions & Claim Semantics
- `create_kanban_task(task_id, title, description)`: Inserts a task in `"todo"` state with `claimed_by = NULL`, or updates title/description if not already claimed.
- `claim_kanban_task(task_id, session_id)`:
  - If `task_id` does not exist in `kanban_tasks`, auto-creates it with `title = task_id`, `description = ""`, `status = "in_progress"`, and `claimed_by = Some(session_id)`.
  - If `task_id` exists in `"todo"` (or is already `"in_progress"` by the same `session_id`), transitions to `"in_progress"` with `claimed_by = Some(session_id)`.
  - If `task_id` is `"in_progress"` claimed by another session, returns a conflict error.
- `finish_kanban_task(task_id, session_id)`: Transitions task to `"done"`, preserving `claimed_by = Some(session_id)`.
- `release_kanban_task(task_id, session_id)`: Releases a task claimed by `session_id` back to `"todo"` with `claimed_by = NULL`.
- `post_worker_report(task_id, session_id, summary, pr_url, commit_sha)`: Appends a structured report row to `worker_reports`.
- `get_worker_reports(task_id)`: Queries all reports for `task_id` ordered by `id ASC`.

## 4. Journal Events
`KanbanManager` publishes monotonic journal events via `JournalWatcher`:
- `task.created`
- `task.claimed`
- `task.finished`
- `task.released`
- `report.posted`

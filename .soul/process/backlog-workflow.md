# Process: Task Workflow & Backlog.md Discipline

## 1. Overview
All work in this project is planned and executed through markdown-native tasks located in `backlog/` using the **Backlog.md** system (`npx backlog.md`).

---

## 2. CLI-First Task Operations
Agents must interact with tasks using the `backlog` CLI commands rather than manually editing task headers to prevent schema desynchronization:

```bash
# 1. Inspect the Kanban Board
npx backlog.md board
npx backlog.md tasks list --plain

# 2. View Task Details & Acceptance Criteria
npx backlog.md task view <taskId>

# 3. Claim Task & Transition Status
npx backlog.md task edit <taskId> -s "In Progress"

# 4. Mark Task Complete (Only after DoD is satisfied!)
npx backlog.md task edit <taskId> -s "Done"
```

---

## 3. Definition of Done (DoD)
An AI agent must **never** transition a task to `"Done"` unless all of the following conditions are met:

1. **Acceptance Criteria:** Every criteria item (`--ac`) listed in the task markdown file has been verified and checked off (`[x]`).
2. **Quality Gates:** 
   * `cargo check` passes with 0 errors.
   * `cargo test` passes 100% of unit and integration tests.
   * `cargo clippy -- -D warnings` reports clean code with zero warnings.
   * `cargo fmt --check` succeeds.
3. **Traceability Updated:** The upstream requirements in `docs/specs/` have their `Implementation Status` and `Test Status` attributes updated to reflect the new state.
4. **No Git Commits by Subagent:** Per delegation policy, delegated implementers do not run `git commit`; the orchestrator/human reviews diffs.

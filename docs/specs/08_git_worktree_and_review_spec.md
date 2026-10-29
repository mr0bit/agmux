# Software Requirements Specification: Git Worktree Lifecycle & Interactive Code Review

## 1. Overview
This specification details the Git worktree provisioning engine (`gix`/`git2`), `.worktreeinclude` copying, worktree removal safety gates with backup stashing, interactive code review mode (`Ctrl+R`), and dual comment resolution.

---

## 2. Requirements

### REQ-GIT-WORKTREE-PROVISION-001
* **Unique ID:** `REQ-GIT-WORKTREE-PROVISION-001`
* **Text:** As a user enabling worktree isolation for a session (`Ctrl+T` or project `auto_worktree = true`), the system shall create an isolated Git worktree at `{repo_root}/.agmux-worktrees/{branch-slug}` using pure-Rust `gix`/`git2`, copy untracked files matching `.worktreeinclude`, and inject a deterministic `PORT` environment variable.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::worktree_manager`
* **Implementation Status:** Verified
* **Verification Criteria:** Spawn a worktree session in a repository with `.env` listed in `.worktreeinclude`; verify that the worktree directory is created, branch is checked out, `.env` is copied, and `$PORT` is set in the session PTY.
* **Downstream Req:** `LLD-GIT-WORKTREE-PROVISION-001`
* **Downstream Test:** `TEST-VER-GIT-001`
* **Test Status:** Passed

---

### REQ-GIT-SAFETY-GATE-002
* **Unique ID:** `REQ-GIT-SAFETY-GATE-002`
* **Text:** As a user or autonomous agent attempting to delete, archive, or teardown a session with an active Git worktree, the system shall inspect git status; if uncommitted changes or unpushed commits exist, the operation shall be blocked unless explicit `--force` confirmation is passed, and a backup `git stash` shall be created automatically before deletion.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::worktree_safety_gate`
* **Implementation Status:** Verified
* **Verification Criteria:** Modify a file in a worktree without committing; trigger `agmux session delete`. Verify that the operation fails with a dirty-working-tree warning. Add `--force`; verify that an automatic backup `git stash` is created before the worktree directory is removed.
* **Downstream Req:** `LLD-GIT-SAFETY-GATE-002`
* **Downstream Test:** `TEST-VER-GIT-002`
* **Test Status:** Passed

---

### REQ-GIT-REVIEW-DIFF-ENGINE-003
* **Unique ID:** `REQ-GIT-REVIEW-DIFF-ENGINE-003`
* **Text:** As a user reviewing agent code changes (`Ctrl+R`), the system shall compute diffs across 5 scopes: `Session Diff` (changes since the session's initial `start_git_sha`), `Full Uncommitted`, `Versus Target Branch`, `Last Commit`, and `Staged`, rendering side-by-side or unified diff views.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::diff_calculator`
* **Implementation Status:** Verified
* **Verification Criteria:** Create changes in an agent session across 3 commits; enter review mode (`Ctrl+R`); switch between `Session Diff` and `Last Commit`. Verify that diff hunks correctly reflect the chosen comparison base without leaving the TUI.
* **Downstream Req:** `LLD-GIT-REVIEW-DIFF-ENGINE-003`
* **Downstream Test:** `TEST-VER-GIT-003`
* **Test Status:** Passed

---

### REQ-GIT-REVIEW-DISPATCH-004
* **Unique ID:** `REQ-GIT-REVIEW-DISPATCH-004`
* **Text:** As a user annotating diff lines in review mode, the system shall store line-level draft comments (`c`/`d`) and, upon pressing `C`, flatten all pending drafts into a single numbered structured prompt round with stable comment IDs and inject the payload into the agent's PTY master.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::prompt_flattener`
* **Implementation Status:** Verified
* **Verification Criteria:** Add 3 review comments on different files; press `C`. Verify that the agent's PTY receives a single structured prompt containing all 3 comments with IDs (`[ID: rev_01]`, etc.) and transitions the session state to `WORKING`.
* **Downstream Req:** `LLD-GIT-REVIEW-DISPATCH-004`
* **Downstream Test:** `TEST-VER-GIT-004`
* **Test Status:** Passed

---

### REQ-GIT-DUAL-RESOLUTION-005
* **Unique ID:** `REQ-GIT-DUAL-RESOLUTION-005`
* **Text:** As an agent resolving review comments, the system shall support dual resolution: (a) explicit agent command via `agmux review resolve <id>` or MCP `resolve_review_comment`, and (b) automated diff hunk tracking that marks comments `resolved` if the hunk matching `line_hash` was modified during the agent's turn.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::comment_tracker`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Dispatch a review comment on line 42; allow the agent to edit line 42 and conclude its turn. Verify that the comment status transitions from `draft`/`sent` to `resolved` automatically in SQLite.
* **Downstream Req:** `LLD-GIT-DUAL-RESOLUTION-005`
* **Downstream Test:** `TEST-VER-GIT-005`
* **Test Status:** Failed

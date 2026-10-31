# Software Requirements Specification: TUI Viewport, Command Palette & Navigation

## 1. Overview
This specification details the Ratatui terminal user interface (`agmux-tui`), sidebar navigation, attention filtering, the VS Code-style Command Palette, the floating Quick Prompt bar, and split comparison viewports.

---

## 2. Requirements

### REQ-TUI-RATATUI-VIEWPORT-001
* **Unique ID:** `REQ-TUI-RATATUI-VIEWPORT-001`
* **Text:** As a user interacting with Agmux in a terminal, the system shall render a multi-pane TUI using Ratatui 0.30 and Crossterm 0.29, displaying the 3-tier sidebar hierarchy on the left and an active `tui-term` terminal viewport on the right, operating under 10 MB RAM.
* **Criticality:** High
* **SW Component:** `crates/agmux-tui::viewport_renderer`
* **Implementation Status:** Verified
* **Verification Criteria:** Launch `agmux-tui` in a standard terminal; verify clean multi-pane rendering, correct color rendition, responsive keyboard input, and resident memory usage under 10 MB.
* **Downstream Req:** `LLD-TUI-RATATUI-VIEWPORT-001`
* **Downstream Test:** `TEST-VER-TUI-001`
* **Test Status:** Passed

---

### REQ-TUI-COMMAND-PALETTE-002
* **Unique ID:** `REQ-TUI-COMMAND-PALETTE-002`
* **Text:** As a user pressing `Ctrl+Shift+P` (or `F1` / `:` fallbacks), the system shall open an overlay Command Palette modal indexing both static actions (*"Toggle Split View"*, *"New Worktree Session"*, *"Archive Session"*) and dynamic targets (*"Jump to Session: <name>"*, *"Switch Model: <model>"*) using the Nucleo fuzzy matcher.
* **Criticality:** High
* **SW Component:** `crates/agmux-tui::command_palette`
* **Implementation Status:** Verified
* **Verification Criteria:** Press `F1` and `:` in terminal emulators without CSI-u support; verify that the Command Palette opens, fuzzy-matches typed queries across static and dynamic items, and executes the selected action on `[Enter]`.
* **Downstream Req:** `LLD-TUI-COMMAND-PALETTE-002`
* **Downstream Test:** `TEST-VER-TUI-002`
* **Test Status:** Passed

---

### REQ-TUI-QUICK-PROMPT-003
* **Unique ID:** `REQ-TUI-QUICK-PROMPT-003`
* **Text:** As a user pressing `Space` on any session row, the system shall open a floating Quick Prompt composer to send prompts without attaching into the terminal, supporting inline cycling of tool (`Tab`), model (`Ctrl+L`), reasoning effort (`Ctrl+X`), and worktree toggle (`Ctrl+T`).
* **Criticality:** High
* **SW Component:** `crates/agmux-tui::quick_prompt`
* **Implementation Status:** Verified
* **Verification Criteria:** Highlight an idle session; hit `Space`; type a prompt; press `Enter`. Verify that the prompt is delivered to the session's PTY and the session transitions to `WORKING` while remaining in the sidebar view.
* **Downstream Req:** `LLD-TUI-QUICK-PROMPT-003`
* **Downstream Test:** `TEST-VER-TUI-003`
* **Test Status:** Passed

---

### REQ-TUI-ATTENTION-FILTER-004
* **Unique ID:** `REQ-TUI-ATTENTION-FILTER-004`
* **Text:** As a user managing 10+ concurrent agents, pressing `w` shall toggle an Attention Filter that hides all `IDLE` and `DEAD` sessions, displaying only sessions in `WAITING`, `FINISHED`, `ERRORED`, or `STUCK` states; pressing `.` shall acknowledge a `FINISHED` session and mark it `IDLE`.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-tui::sidebar_filter`
* **Implementation Status:** Verified
* **Verification Criteria:** Spawn 5 `IDLE` sessions and 2 `WAITING` sessions; press `w`. Verify that only the 2 `WAITING` sessions are visible in the sidebar tree. Press `w` again to restore the full tree.
* **Downstream Req:** `LLD-TUI-ATTENTION-FILTER-004`
* **Downstream Test:** `TEST-VER-TUI-004`
* **Test Status:** Passed

---

### REQ-TUI-SPLIT-COMPARE-005
* **Unique ID:** `REQ-TUI-SPLIT-COMPARE-005`
* **Text:** As a user comparing two competing agent implementations, pressing `v` on a live session shall split the main viewport into side-by-side panes; pressing `m` shall pin the active session in the left pane while arrow keys cycle through target sessions in the right pane.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-tui::split_viewport`
* **Implementation Status:** Verified
* **Verification Criteria:** Activate split view via `v`; mark session A via `m`; navigate sidebar down to session B. Verify that session A remains pinned on the left while session B renders live on the right.
* **Downstream Req:** `LLD-TUI-SPLIT-COMPARE-005`
* **Downstream Test:** `TEST-VER-TUI-005`
* **Test Status:** Passed

---

### REQ-TUI-COLD-START-006
* **Unique ID:** `REQ-TUI-COLD-START-006`
* **Text:** As a first-time user launching `agmux` with zero existing sessions in the database, the system shall automatically display an interactive New Session onboarding wizard pre-populated with detected local Git repo details and installed CLI agents, allowing a session to launch on `[Enter]`.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-tui::onboarding_wizard`
* **Implementation Status:** Verified
* **Verification Criteria:** Start `agmux` with an empty database; verify that the wizard displays detected CLI agents (e.g. `claude`, `codex`, `agy`), populates the repository root, and spawns the chosen agent on `[Enter]`.
* **Downstream Req:** `LLD-TUI-COLD-START-006`
* **Downstream Test:** `TEST-VER-TUI-006`
* **Test Status:** Passed

---

### REQ-TUI-GLOBAL-SEARCH-007
* **Unique ID:** `REQ-TUI-GLOBAL-SEARCH-007`
* **Text:** As a user pressing `Ctrl+F` or `/` in `agmux-tui`, the system shall provide an interactive Global Search modal (`Ctrl+F`) with a right-hand read-only context preview pane auto-scrolled to the matched transcript line across live and `DEAD` sessions, alongside a `/` quick sidebar filter bar filtering sessions by name, branch, or notes.
* **Criticality:** High
* **SW Component:** `crates/agmux-tui::search_modal`
* **Implementation Status:** Verified
* **Verification Criteria:** Open the Global Search modal via `Ctrl+F`, enter a query, verify matching transcript hits and right-hand context preview rendering; activate `/` filter in the sidebar and verify live filtering of the session list.
* **Downstream Req:** `LLD-TUI-GLOBAL-SEARCH-007`
* **Downstream Test:** `TEST-VER-TUI-007`
* **Test Status:** Passed

---

### REQ-TUI-SCRATCHPAD-NOTES-008
* **Unique ID:** `REQ-TUI-SCRATCHPAD-NOTES-008`
* **Text:** As a user tracking context per agent session, the system shall support persistent multi-page scratchpad notes attached to any session (`Ctrl+PgUp` / `Ctrl+PgDn` in TUI and `agmux note` in CLI), persisted in SQLite and searchable via `/` and `Ctrl+F`.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::scratchpad_notes`
* **Implementation Status:** Verified
* **Verification Criteria:** Attach multi-page notes to a session via CLI/IPC and TUI (`Ctrl+PgDn`); verify persistence across daemon queries and verify that filtering via `/` or searching matches the attached note content.
* **Downstream Req:** `LLD-TUI-SCRATCHPAD-NOTES-008`
* **Downstream Test:** `TEST-VER-TUI-008`
* **Test Status:** Passed

---

### REQ-TUI-WORKSPACE-HIERARCHY-009
* **Unique ID:** `REQ-TUI-WORKSPACE-HIERARCHY-009`
* **Text:** As a power user organizing many concurrent agents, `agmux-tui` shall support the 3-tier hierarchy (Project -> Group `g` -> Session), `⭐ Favorites` pinning (`*`), Compact (1-line) vs. Comfortable (3-line) row density toggle (`D`), Archive/Restore (`a` / `u` / `t`), Conversation Forking (`f` -> standalone `s` or sub-tab `t`), and Quick Prompt inline modifiers (`Tab` tool, `Ctrl+L` model, `Ctrl+X` effort, `Ctrl+T` worktree).
* **Criticality:** High
* **SW Component:** `crates/agmux-tui::workspace_hierarchy`
* **Implementation Status:** Verified
* **Verification Criteria:** Pin a favorite session (`*`), toggle Comfortable 3-line density, archive (`a`) and toggle archived view (`t`) then restore (`u`), open the Fork modal (`f`) to fork into a standalone session or sub-tab, and cycle Quick Prompt modifiers (`Tab`, `Ctrl+L`, `Ctrl+X`, `Ctrl+T`).
* **Downstream Req:** `LLD-TUI-WORKSPACE-HIERARCHY-009`
* **Downstream Test:** `TEST-VER-TUI-009`
* **Test Status:** Passed

* **Evidence:** `tests/e2e_tui_workspace_hierarchy_test.rs`, `crates/agmux-tui/tests/workspace_hierarchy_test.rs`, and `tests/reports/tui_workspace_hierarchy_test_report.md`.

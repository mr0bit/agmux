# Session workflow acceptance — issue #17 batch 2

Approved 2026-11-18. One live provider conversation has Chat and Terminal presentations. This supersedes the independent-context decision from batch 1. Failed initially means not yet verified.

### REQ-UX-SHARED-CONTEXT-113
* **Unique ID:** `REQ-UX-SHARED-CONTEXT-113`
* **Text:** As a user of Agmux, One provider conversation shall power Chat and Terminal, so prompts from either view use the same context.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::gemini_chat`
* **Implementation Status:** Verified
* **Verification Criteria:** Enter a unique fact in Chat, continue in Terminal and back; assert one provider session/process and complete ordered request history.
* **Downstream Req:** `LLD-UX-SHARED-CONTEXT-113`
* **Downstream Test:** `TEST-VER-UX-113`
* **Test Status:** Passed

### REQ-UX-SHARED-STATE-114
* **Unique ID:** `REQ-UX-SHARED-STATE-114`
* **Text:** As a user of Agmux, Both views shall share messages, drafts, approvals and in-flight state, so switching cannot fork or duplicate a turn.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Switch while streaming/waiting; submit in either view, preserve draft and reject duplicate/concurrent requests.
* **Downstream Req:** `LLD-UX-SHARED-STATE-114`
* **Downstream Test:** `TEST-VER-UX-114`
* **Test Status:** Passed

### REQ-UX-LIVE-CHANGES-115
* **Unique ID:** `REQ-UX-LIVE-CHANGES-115`
* **Text:** As a user of Agmux, Applied file changes shall refresh automatically including new/untracked files, so users see actual project changes while agents work.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::diff_calculator`
* **Implementation Status:** Verified
* **Verification Criteria:** Modify/create/delete files in a real Git directory; the sidebar updates without Refresh and switching sessions cannot mix results.
* **Downstream Req:** `LLD-UX-LIVE-CHANGES-115`
* **Downstream Test:** `TEST-VER-UX-115`
* **Test Status:** Passed

### REQ-UX-PROPOSED-DIFF-116
* **Unique ID:** `REQ-UX-PROPOSED-DIFF-116`
* **Text:** As a user of Agmux, Provider edit proposals shall retain real old/new text and paths, so the user can review them before approval.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::gemini_chat`
* **Implementation Status:** Verified
* **Verification Criteria:** Emit an actual provider diff block in an approval request; inspect side-by-side preview before any disk write; no fabricated preview for unsupported content.
* **Downstream Req:** `LLD-UX-PROPOSED-DIFF-116`
* **Downstream Test:** `TEST-VER-UX-116`
* **Test Status:** Passed

### REQ-UX-EDIT-APPROVAL-117
* **Unique ID:** `REQ-UX-EDIT-APPROVAL-117`
* **Text:** As a user of Agmux, Proposed and Applied changes shall remain distinct through approval/rejection, so previews cannot masquerade as completed writes.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Reject leaves file unchanged; allow leads to actual on-disk diff in Applied; pending and applied groups show correct before/after content.
* **Downstream Req:** `LLD-UX-EDIT-APPROVAL-117`
* **Downstream Test:** `TEST-VER-UX-117`
* **Test Status:** Passed

### REQ-UX-INACTIVE-ARCHIVE-118
* **Unique ID:** `REQ-UX-INACTIVE-ARCHIVE-118`
* **Text:** As a user of Agmux, Ended sessions shall move to Inactive and support explicit persistent archiving, so retained sessions remain discoverable without cluttering active work.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** End, reload, archive and restore visibility via Archived; verify active sessions are unaffected and no data is deleted.
* **Downstream Req:** `LLD-UX-INACTIVE-ARCHIVE-118`
* **Downstream Test:** `TEST-VER-UX-118`
* **Test Status:** Passed

### REQ-UX-SESSION-VIEWS-119
* **Unique ID:** `REQ-UX-SESSION-VIEWS-119`
* **Text:** As a user of Agmux, Session navigation shall offer recent-activity and canonical project-path grouping, so users can find active agents across projects.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Use two projects with identical basenames and several sessions; verify distinct full-path groups, activity ordering, status badges and persisted view choice.
* **Downstream Req:** `LLD-UX-SESSION-VIEWS-119`
* **Downstream Test:** `TEST-VER-UX-119`
* **Test Status:** Passed

### REQ-UX-COMPACT-ROWS-120
* **Unique ID:** `REQ-UX-COMPACT-ROWS-120`
* **Text:** As a user of Agmux, Compact session rows shall use a single line with accessible title and status, so many sessions fit without broken virtualization.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Measure compact row height, test long labels, keyboard selection and scrolling a large list without overlaps.
* **Downstream Req:** `LLD-UX-COMPACT-ROWS-120`
* **Downstream Test:** `TEST-VER-UX-120`
* **Test Status:** Passed

### REQ-UX-AGENT-ONLY-121
* **Unique ID:** `REQ-UX-AGENT-ONLY-121`
* **Text:** As a user of Agmux, New-session UI shall offer agent providers without a Shell default or fallback, so session creation is agent focused.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Check installed and no-provider cases, select an actual available agent and confirm Shell is absent from the UI.
* **Downstream Req:** `LLD-UX-AGENT-ONLY-121`
* **Downstream Test:** `TEST-VER-UX-121`
* **Test Status:** Passed

### REQ-UX-KEYBOARD-WORKFLOW-122
* **Unique ID:** `REQ-UX-KEYBOARD-WORKFLOW-122`
* **Text:** As a user of Agmux, The workspace shall provide a familiar command palette, quick switcher and shortcut help, so keyboard workflows are discoverable and respect input focus.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Use Ctrl/Cmd+Shift+P, Ctrl/Cmd+P, sidebar/Terminal shortcuts and shortcut help; verify Escape, focus return, IME and editor/terminal input handling.
* **Downstream Req:** `LLD-UX-KEYBOARD-WORKFLOW-122`
* **Downstream Test:** `TEST-VER-UX-122`
* **Test Status:** Passed

### REQ-UX-BATCH2-ACCEPTANCE-123
* **Unique ID:** `REQ-UX-BATCH2-ACCEPTANCE-123`
* **Text:** As a user of Agmux, The packaged Windows app shall have recorded shared-session, file-review and navigation acceptance evidence, so mock protocol success is not mistaken for user confirmation.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Implemented
* **Verification Criteria:** Record source SHA, artifact checksum and Windows native regression; keep authenticated/user confirmation separate and attach screenshots.
* **Downstream Req:** `LLD-UX-BATCH2-ACCEPTANCE-123`
* **Downstream Test:** `TEST-VER-UX-123`
* **Test Status:** Failed

### REQ-UX-RETAINED-HISTORY-124
* **Unique ID:** `REQ-UX-RETAINED-HISTORY-124`
* **Text:** As a user of Agmux, Conversation history and session ownership shall be retained when a session ends or the daemon restarts, so inactive/archive views remain readable without silently starting another conversation.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::storage_engine`
* **Implementation Status:** Verified
* **Verification Criteria:** End and restart daemon, then read the saved transcript; no provider process or prompt is started for ended/archived records.
* **Downstream Req:** `LLD-UX-RETAINED-HISTORY-124`
* **Downstream Test:** `TEST-VER-UX-124`
* **Test Status:** Passed


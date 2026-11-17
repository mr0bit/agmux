# Companion user acceptance requirements — issue #17

These atomic requirements close gaps found during Windows QA. Existing lower-level
requirements remain in force; their prior verification does not imply these
interactive flows passed. IDs 101–112 are new acceptance contracts. Requirements were added as
Not implemented/Failed per the repository schema; **Failed here means
not yet verified**, not that a test has already been executed. LLD/test links now resolve to the artifacts listed in
`tests/reports/qa17_acceptance_report.md`; native acceptance remains separate.

Source: https://github.com/mr0bit/agmux/issues/17
User approved S1–S7 granularity and dependencies on 2026-11-18 (Africa/Cairo).

### REQ-UX-PROMPT-SUBMIT-101
* **Unique ID:** `REQ-UX-PROMPT-SUBMIT-101`
* **Text:** As a user sending a prompt, the system shall submit the complete prompt and exactly one provider-appropriate Enter action, so that consecutive prompts start separate turns.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::server`
* **Implementation Status:** Implemented
* **Verification Criteria:** Send two different prompts through the authenticated browser/IPC path into a native Windows interactive process; observe two distinct submissions and preserve multiline content.
* **Downstream Req:** `LLD-UX-PROMPT-SUBMIT-101`
* **Downstream Test:** `TEST-VER-UX-101`
* **Test Status:** Failed

### REQ-UX-NONGIT-DIFF-102
* **Unique ID:** `REQ-UX-NONGIT-DIFF-102`
* **Text:** As a user working in a plain folder, the system shall report that Git review is unavailable without invoking a failing diff or modifying the folder, so that a valid agent session does not require Git.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::diff_calculator`
* **Implementation Status:** Verified
* **Verification Criteria:** Start in a directory without .git and verify a typed non-repository result, no git init, and successful chat/terminal use; verify normal repository diffs still work.
* **Downstream Req:** `LLD-UX-NONGIT-DIFF-102`
* **Downstream Test:** `TEST-VER-UX-102`
* **Test Status:** Passed

### REQ-UX-BOUNDED-ERROR-103
* **Unique ID:** `REQ-UX-BOUNDED-ERROR-103`
* **Text:** As a user viewing a workspace error, the system shall show a concise summary with bounded expandable diagnostics, so that an error cannot replace or obscure the workspace.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Inject a very long diagnostic and verify the summary and scrollable details remain bounded while navigation and the composer remain usable.
* **Downstream Req:** `LLD-UX-BOUNDED-ERROR-103`
* **Downstream Test:** `TEST-VER-UX-103`
* **Test Status:** Passed

### REQ-UX-TERMINAL-CELLS-104
* **Unique ID:** `REQ-UX-TERMINAL-CELLS-104`
* **Text:** As a user resizing a Windows terminal, the system shall preserve measured monospace cell geometry, cursor placement and borders across font loading and resize, so that native CLI interfaces remain legible.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Implemented
* **Verification Criteria:** Display a deterministic border/cursor fixture, resize panels and viewport, and compare cell alignment at 100/125/150 percent display scaling.
* **Downstream Req:** `LLD-UX-TERMINAL-CELLS-104`
* **Downstream Test:** `TEST-VER-UX-104`
* **Test Status:** Failed

### REQ-UX-CHAT-MESSAGES-105
* **Unique ID:** `REQ-UX-CHAT-MESSAGES-105`
* **Text:** As a user choosing Gemini Chat, the system shall render actual provider user/assistant messages separately from the native terminal interface, so that terminal banners and prompt controls do not masquerade as assistant responses.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Implemented
* **Verification Criteria:** Run two real conversation turns, verify ordered user/assistant content, and verify raw TUI content is confined to Terminal.
* **Downstream Req:** `LLD-UX-CHAT-MESSAGES-105`
* **Downstream Test:** `TEST-VER-UX-105`
* **Test Status:** Failed

### REQ-UX-TURN-LIFECYCLE-106
* **Unique ID:** `REQ-UX-TURN-LIFECYCLE-106`
* **Text:** As a user awaiting a response, the system shall publish explicit submitting, working, waiting-for-permission, idle, cancelled and error states for Gemini chat turns, so that a live process alone cannot leave a turn labelled Working indefinitely.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::gemini_chat`
* **Implementation Status:** Verified
* **Verification Criteria:** Exercise successful streaming, provider rejection, permission waiting, cancellation and process exit; each operation reaches an observable terminal or blocked state.
* **Downstream Req:** `LLD-UX-TURN-LIFECYCLE-106`
* **Downstream Test:** `TEST-VER-UX-106`
* **Test Status:** Passed

### REQ-UX-ADVANCED-SETUP-107
* **Unique ID:** `REQ-UX-ADVANCED-SETUP-107`
* **Text:** As a user starting a session, the system shall show basic launch fields first and hide technical arguments behind an accessible collapsed disclosure, so that session creation does not require editing JSON.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Launch with basic fields only; keyboard-toggle advanced settings and verify explicit advanced options remain usable.
* **Downstream Req:** `LLD-UX-ADVANCED-SETUP-107`
* **Downstream Test:** `TEST-VER-UX-107`
* **Test Status:** Passed

### REQ-UX-PROVIDER-MODELS-108
* **Unique ID:** `REQ-UX-PROVIDER-MODELS-108`
* **Text:** As a user selecting a Gemini model, the system shall discover models and the current default from the installed provider protocol when available, so that model choices reflect actual advertised capabilities.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::gemini_chat`
* **Implementation Status:** Verified
* **Verification Criteria:** Verify provider-advertised models/default and timeout/authentication/unsupported cases without inventing an available model inventory.
* **Downstream Req:** `LLD-UX-PROVIDER-MODELS-108`
* **Downstream Test:** `TEST-VER-UX-108`
* **Test Status:** Passed

### REQ-UX-MODEL-DEFAULTS-109
* **Unique ID:** `REQ-UX-MODEL-DEFAULTS-109`
* **Text:** As a user configuring a model, the system shall select the discovered current/default model and present an honest fallback when discovery is unavailable, so that a valid session remains launchable without guessing model identifiers.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Switch providers and discovery outcomes; verify the selected default, no stale inventory, no duplicate model flags and a usable default fallback.
* **Downstream Req:** `LLD-UX-MODEL-DEFAULTS-109`
* **Downstream Test:** `TEST-VER-UX-109`
* **Test Status:** Passed

### REQ-UX-WINDOWS-ACCEPTANCE-110
* **Unique ID:** `REQ-UX-WINDOWS-ACCEPTANCE-110`
* **Text:** As a user downloading a Windows release, the system shall pass the interactive submit/respond, layout, plain-folder and startup acceptance flows on the packaged executable, so that a version-only probe cannot qualify a functional release.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Implemented
* **Verification Criteria:** Record exact source SHA/archive checksum and native Windows two-turn, Git/non-Git, resize and startup evidence; version-only checks do not satisfy this requirement.
* **Downstream Req:** `LLD-UX-WINDOWS-ACCEPTANCE-110`
* **Downstream Test:** `TEST-VER-UX-110`
* **Test Status:** Failed

### REQ-UX-CHAT-ISOLATION-111
* **Unique ID:** `REQ-UX-CHAT-ISOLATION-111`
* **Text:** As a user working with multiple chat sessions, the system shall keep message histories and request identifiers isolated per selected session and reject concurrent/duplicate turn submission safely, so that switching or reloading the UI cannot mix or duplicate turns.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::gemini_chat`
* **Implementation Status:** Verified
* **Verification Criteria:** Use two sessions and repeated request IDs; verify independent messages, no duplicate provider requests, and UI reload recovery of daemon-held conversation state.
* **Downstream Req:** `LLD-UX-CHAT-ISOLATION-111`
* **Downstream Test:** `TEST-VER-UX-111`
* **Test Status:** Passed

### REQ-UX-STARTUP-DIAGNOSTICS-112
* **Unique ID:** `REQ-UX-STARTUP-DIAGNOSTICS-112`
* **Text:** As a user starting the Windows app, the system shall use normal user-writable files for startup coordination and retain readable bounded backend diagnostics on failure, so that startup is usable and failures explain the cause.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::daemon_spawner`
* **Implementation Status:** Implemented
* **Verification Criteria:** Launch default packaged Windows app without environment workarounds; force an invalid database and verify understandable reason and retained diagnostic-log path.
* **Downstream Req:** `LLD-UX-STARTUP-DIAGNOSTICS-112`
* **Downstream Test:** `TEST-VER-UX-112`
* **Test Status:** Failed


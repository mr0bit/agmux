# Software Requirements Specification: CLI Adapter Plugin System

## 1. Overview
This specification details the two-tier CLI adapter architecture (`AgentPlugin` Rust trait and declarative `agents.toml`), tool-specific STUCK detection heuristics, session UUID capture, and dynamic model discovery.

---

## 2. Requirements

### REQ-PLUGIN-TRAIT-001
* **Unique ID:** `REQ-PLUGIN-TRAIT-001`
* **Text:** As an architect extending agent support, the system shall define a zero-overhead Rust trait `AgentPlugin: Send + Sync` in `agmux-core` providing hooks for command generation, session ID capture, dynamic flag queries, and ANSI state parsing.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::plugin_api`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Implement a mock `AgentPlugin`; verify that `agmux-core` successfully compiles, registers the plugin, and invokes its lifecycle callbacks with zero heap allocations in the hot read loop.
* **Downstream Req:** `LLD-PLUGIN-TRAIT-001`
* **Downstream Test:** `TEST-VER-PLUGIN-001`
* **Test Status:** Failed

---

### REQ-PLUGIN-STUCK-HEURISTIC-002
* **Unique ID:** `REQ-PLUGIN-STUCK-HEURISTIC-002`
* **Text:** As a user whose agent might leave unsubmitted text in the input box, each `AgentPlugin` implementation shall provide tool-specific `check_stuck(&screen, last_input_time, pty_silence)` logic to detect when text sits after the prompt marker for >10s without a newline, transitioning the session state to `STUCK`.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::stuck_detector`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Simulate injecting text into a simulated Claude Code and Aider prompt without pressing enter; verify that after 10 seconds of silence with no child process, both sessions accurately transition to `STUCK` and display the warning badge.
* **Downstream Req:** `LLD-PLUGIN-STUCK-HEURISTIC-002`
* **Downstream Test:** `TEST-VER-PLUGIN-002`
* **Test Status:** Failed

---

### REQ-PLUGIN-BUILTIN-ADAPTERS-003
* **Unique ID:** `REQ-PLUGIN-BUILTIN-ADAPTERS-003`
* **Text:** As a user running standard coding agents, the system shall provide compiled-in `AgentPlugin` implementations for 14 first-class CLI tools: Claude Code (`claude`), Antigravity (`agy`), Gemini CLI (`gemini`), Codex (`codex`), OpenCode (`opencode`), Aider (`aider`), Cursor CLI (`cursor-agent`), Grok (`grok`), Amazon Q (`q`), Pi (`pi`), Oh My Pi (`omp`), Command Code (`cmd`), Hermes (`hermes`), and Muse (`muse`).
* **Criticality:** High
* **SW Component:** `crates/agmux-core::builtin_plugins`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Spawn each of the 14 supported agents in a dry-run harness; verify that correct CLI spawn arguments, resume arguments, and hook parameters are generated according to tool specifications.
* **Downstream Req:** `LLD-PLUGIN-BUILTIN-ADAPTERS-003`
* **Downstream Test:** `TEST-VER-PLUGIN-003`
* **Test Status:** Failed

---

### REQ-PLUGIN-DECLARATIVE-TOML-004
* **Unique ID:** `REQ-PLUGIN-DECLARATIVE-TOML-004`
* **Text:** As a user using custom or in-house agent scripts, the system shall load user-defined CLI profiles from `~/.config/agmux/agents.toml` with configurable regexes (`prompt_regex`, `stuck_timeout_ms`, `limit_regex`, `resume_cmd`), allowing custom agents to be used without recompiling Agmux.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::declarative_plugin`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Define a custom agent profile in `agents.toml`; spawn the agent in `agmux-tui`; verify that custom prompt regexes, resume commands, and limits are correctly parsed and applied.
* **Downstream Req:** `LLD-PLUGIN-DECLARATIVE-TOML-004`
* **Downstream Test:** `TEST-VER-PLUGIN-004`
* **Test Status:** Failed

---

### REQ-PLUGIN-UUID-CAPTURE-005
* **Unique ID:** `REQ-PLUGIN-UUID-CAPTURE-005`
* **Text:** As a user restarting or reviving sessions, the system shall capture native conversation UUIDs without user intervention via either (a) pre-minted UUID injection (`session_id_flag`) or (b) filesystem/stdout store watchers (`session_store`), persisting the UUID in SQLite.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::session_tracker`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Spawn a Claude session with pre-minted UUID injection, and an Antigravity (`agy`) session with store watching; verify that both UUIDs are captured within 2 seconds of spawn and saved to `sessions.cli_session_uuid`.
* **Downstream Req:** `LLD-PLUGIN-UUID-CAPTURE-005`
* **Downstream Test:** `TEST-VER-PLUGIN-005`
* **Test Status:** Failed

---

### REQ-PLUGIN-MISSING-DETECTOR-006
* **Unique ID:** `REQ-PLUGIN-MISSING-DETECTOR-006`
* **Text:** As a user attempting to spawn an agent whose binary is missing from `PATH`, the system shall detect the missing binary before spawning and display an interactive installation banner and documentation link rather than crashing or showing a raw OS error.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::cli_detector`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Attempt to spawn a non-installed agent tool; verify that `agmux-tui` displays a clean modal with the official installation command and prevents invalid session creation.
* **Downstream Req:** `LLD-PLUGIN-MISSING-DETECTOR-006`
* **Downstream Test:** `TEST-VER-PLUGIN-006`
* **Test Status:** Failed

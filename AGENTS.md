# AGENTS.md — agmux

Guidelines and constraints for AI agents (Codex, Antigravity/AGY, and orchestrators) operating in the **agmux** repository.

---

## 1. Project Overview

**agmux** is an ultra-fast, lightweight (~10 MB RAM, ~5 MB single binary) cross-platform agentic terminal multiplexer and session orchestrator. It provides zero-latency state monitoring, cross-terminal search, zero-flag auto-resume, Git worktree integration, and multi-agent coordination without requiring tmux.

* **Primary Language:** Rust (latest stable edition)
* **Architecture Documents:**
  * PRD: [`docs/product/PRD.md`](docs/product/PRD.md)
  * TRD: [`docs/product/TRD.md`](docs/product/TRD.md)
  * Delegation Policy: [`.agents/rules/delegation.md`](.agents/rules/delegation.md)

---

## 2. Core Implementation Principles

1. **Performance & Footprint:**
   * Strict memory budget (~10 MB RAM idle daemon).
   * Zero unneeded heap allocations in hot loops (PTY parsing, search indexing, event dispatch).
   * Prefer streaming processing over buffering entire session histories into RAM.

2. **Cross-Platform Parity:**
   * Support Linux, macOS, and Windows (native ConPTY + WSL2 bridge).
   * Never hardcode Unix-only paths (`/tmp`) or shell assumptions; use platform-agnostic abstractions.

3. **Code Quality & Rust Conventions:**
   * Enforce `cargo fmt` and clean `cargo clippy -- -D warnings`.
   * Clear error handling with `thiserror` for library boundaries and `anyhow` for top-level CLI errors.
   * Write comprehensive unit and integration tests for critical subsystems (PTY, state machine, search).

---

## 3. Delegation & Execution Contract

When working as a delegated implementer (`codex-delegate` or `agy-delegate`):

1. **Do NOT commit changes:**
   * The human/orchestrator reviews the diff and lands commits. Do not run `git commit` or force git actions.
2. **Respect the Brief Scope:**
   * Modify only the files requested in the task brief. Do not perform speculative refactors outside scope.
3. **Run Gates Before Finishing:**
   * Always verify that code builds and passes relevant tests (`cargo check`, `cargo test`) before concluding the task.
4. **Report Concisely:**
   * Summarize modified files, verification results, and any architectural observations in your task report.

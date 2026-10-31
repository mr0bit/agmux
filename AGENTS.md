# AGENTS.md — agmux

Guidelines and constraints for AI agents (Codex, Antigravity/AGY, and orchestrators) operating in the **agmux** repository.

---

## 1. Project Overview & Quick Reference

**agmux** is an ultra-fast, lightweight (~10 MB RAM daemon, ~5 MB single binary) cross-platform agentic terminal multiplexer and session orchestrator written in modern Rust. It provides zero-latency state monitoring, cross-terminal search, zero-flag auto-resume, Git worktree integration, and multi-agent coordination without requiring tmux.

* **Primary Language:** Rust (2024 Edition, MSRV 1.85+)
* **Target Triples:** Linux, macOS, Windows (native ConPTY + WSL2 bridge)
* **Architecture Documents:**
  * PRD: [`docs/product/PRD.md`](docs/product/PRD.md)
  * TRD: [`docs/product/TRD.md`](docs/product/TRD.md)
  * Software Architecture (SAD): [`docs/archi/software-architecture.md`](docs/archi/software-architecture.md)
  * Module Root of Trust: [`docs/archi/module-list.md`](docs/archi/module-list.md)
  * Specifications: [`docs/specs/`](docs/specs/)
  * Backlog & Tasks: [`.backlog/`](.backlog/)
  * Fleet Delegation Policy: [`.agents/rules/delegation.md`](.agents/rules/delegation.md)

---

## 2. Core Agent Operating System (`.soul/`)

All engineering processes, behavioral heuristics, architectural invariants, and testing standards are governed by **The Soul**:

👉 **At the start of any session or task, read [`.soul/INDEX.md`](.soul/INDEX.md) to navigate and dynamically load the relevant operating system modules.**

---

## 3. Delegated Execution Rules

When working as a delegated implementer (`codex-delegate` or `agy-delegate`):

1. **Do NOT commit changes:** The human/orchestrator reviews the diff and lands commits. Do not run `git commit` or force git actions.
2. **Respect the Brief Scope:** Modify only the files requested in the task brief. Do not perform speculative refactors outside scope.
3. **Run Gates Before Finishing:** Always verify `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and run `node .soul/tools/v-cycle-checker.mjs` before concluding.
4. **Report Concisely:** Summarize modified files, verification test results, and spec status updates in your final report.

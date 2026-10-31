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

All software engineering processes, behavioral heuristics, requirement management rules, architectural invariants, and testing standards are governed by **The Soul** located in the [`.soul/`](.soul/) directory.

Before planning, modifying code, or running tests at the start of any session or task:

👉 **[`.soul/INDEX.md`](.soul/INDEX.md)** — The Master Agent Operating System Index

Agents must consult [`.soul/INDEX.md`](.soul/INDEX.md) to inspect the full operating system index and dynamically select which modules to read for their current task (with [`.soul/guidelines/karpathy-guidelines.md`](.soul/guidelines/karpathy-guidelines.md) serving as the universal behavioral baseline for thinking, simplicity, surgical changes, and goal-driven verification).

### Summary of Governing Modules in `.soul/`:
* **Behavioral Guidelines ([`.soul/guidelines/`](.soul/guidelines/)):**
  * [`karpathy-guidelines.md`](.soul/guidelines/karpathy-guidelines.md): Non-negotiable cognitive posture (Think Before Coding, Simplicity First, Surgical Changes, Goal-Driven Execution).
* **Self-Evolution & Firewall ([`.soul/evolution/`](.soul/evolution/)):**
  * [`portability-firewall.md`](.soul/evolution/portability-firewall.md): Defending `.soul` portability via the 4-Gate Test.
  * [`evolution-protocol.md`](.soul/evolution/evolution-protocol.md): The 5-phase learning loop for continuous self-evolution.
  * [`anti-patterns-catalog.md`](.soul/evolution/anti-patterns-catalog.md): Reusable failure modes to avoid across tasks.
  * [`learning-record.md`](.soul/evolution/learning-record.md): Permanent log of codified learnings and promoted rules.
* **Engineering Process ([`.soul/process/`](.soul/process/)):**
  * [`v-cycle.md`](.soul/process/v-cycle.md): 5-level V-Model and traceability chain.
  * [`backlog-workflow.md`](.soul/process/backlog-workflow.md): Task lifecycle via `npx backlog.md` and Definition of Done.
  * [`agent-execution-rules.md`](.soul/process/agent-execution-rules.md): The 3-step rhythm (Inspect ➔ Implement & Trace ➔ Verify Gates).
  * [`delegation-discipline.md`](.soul/process/delegation-discipline.md): Multi-agent fleet & setup prerequisite.
* **Requirements Management ([`.soul/requirements/`](.soul/requirements/)):**
  * [`attributes-schema.md`](.soul/requirements/attributes-schema.md): 9 mandatory attributes for atomic requirements.
  * [`state-transitions.md`](.soul/requirements/state-transitions.md): Exact protocol for updating requirement states (`Not implemented` ➔ `Implemented` ➔ `Verified`).
* **Architecture Discipline ([`.soul/architecture/`](.soul/architecture/)):**
  * [`module-root-of-trust.md`](.soul/architecture/module-root-of-trust.md): Strictly bounded module creation without speculative helpers.
  * [`deep-modules-and-seams.md`](.soul/architecture/deep-modules-and-seams.md): Deep module interfaces and minimal testing seams.
* **Testing Standards ([`.soul/testing/`](.soul/testing/)):**
  * [`testing-seams.md`](.soul/testing/testing-seams.md): Testing public seams (IPC boundary) without internal mocks.
  * [`unit-vs-verification.md`](.soul/testing/unit-vs-verification.md): SUT (`crates/*/tests/`) vs SVT (`tests/`).
  * [`test-reports.md`](.soul/testing/test-reports.md): Traceable markdown test report formatting.
* **Code Quality & Standards ([`.soul/code-quality/`](.soul/code-quality/) & [`.soul/languages/`](.soul/languages/)):**
  * [`code-conventions.md`](.soul/code-quality/code-conventions.md): Universal error boundaries, compiler gates, and async discipline.
  * [`performance-invariants.md`](.soul/code-quality/performance-invariants.md): Zero-allocation hot loops, memory budgets, and 0 MB dead-session RAM.
  * [`languages/rust.md`](.soul/languages/rust.md): Language-specific conventions for modern Rust (2024 Edition).

---

## 3. Delegated Execution Rules

When working as a delegated implementer (`codex-delegate` or `agy-delegate`):

1. **Do NOT commit changes:** The human/orchestrator reviews the diff and lands commits. Do not run `git commit` or force git actions.
2. **Respect the Brief Scope:** Modify only the files requested in the task brief. Do not perform speculative refactors outside scope.
3. **Run Gates Before Finishing:** Always verify `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and run `node .soul/tools/v-cycle-checker.mjs` before concluding.
4. **Report Concisely:** Summarize modified files, verification test results, and spec status updates in your final report.

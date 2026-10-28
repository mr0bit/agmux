# Software Architecture Document (SAD) — Agmux

## 1. Executive Summary & Architectural Scope
**Agmux** is an ultra-fast, lightweight (~10 MB RAM daemon, ~5 MB binary) cross-platform agentic terminal multiplexer and session orchestrator written in modern Rust. It eliminates the overhead and latency of legacy multiplexers (tmux/psmux) by owning OS pseudo-terminals directly via `portable-pty` and maintaining an in-memory VT100 screen grid per active session.

This master Software Architecture Document synthesizes the complete system architecture across the dedicated architectural domain documents listed below.

---

## 2. Architectural Subsystem Sections

The detailed architecture is partitioned into modular, focused specifications. Refer directly to each section document:

### Section 1: Static Architecture & Topology
* **Source:** [`docs/archi/static-architecture.md`](./static-architecture.md)
* **Contents:**
  * System Overview & High-Level Component Topology
  * Layered Architecture (Stateless Frontends, Framed Transport, Core Daemon, Storage)
  * Third-Party Crate Integration Matrix (`portable-pty`, `vt100`, `ratatui`, `gix`, `rusqlite`, `zstd`, `nucleo`)
  * Domain Struct & Class Diagrams

### Section 2: Component Breakdown & Module Root of Trust
* **Source:** [`docs/archi/module-list.md`](./module-list.md)
* **Contents:**
  * Cargo Workspace Crate Structure (`agmux-proto`, `agmux-core`, `agmux-daemon`, `agmux-tui`, `agmux-cli`)
  * Module Root of Trust across all 49 functional subsystems
  * Complete Software Requirement to Component Traceability Matrix (`REQ-*` -> `crates/*`)

### Section 3: API & Interface Specifications
* **Source:** [`docs/archi/api-interface-list-architecture.md`](./api-interface-list-architecture.md)
* **Contents:**
  * Framed IPC Protocol Specification (`ClientToDaemon`, `DaemonToClient` binary enums)
  * Authentication Handshake & `ScreenSyncFrame` Zero-Flicker Protocol
  * Daemon Configuration (`config.toml`) & CLI Adapter Schema (`agents.toml`)
  * Injected Environment Variables (`AGMUX_*`, `PORT`) & System Constants

### Section 4: Architectural Decision Records (ADRs)
* **Source:** [`docs/archi/desing-decisions.md`](./desing-decisions.md)
* **Contents:**
  * Reference Tool Collision Resolutions (C1–C7 Locked)
  * Stress-Test Architectural Decisions (Q1–Q17 Locked: Single-Writer SQLite, Process Groups, Two-Tier Plugins, Safety Gates, Command Palette, Dead-Session Lock Eviction)

### Section 5: Security Architecture & Controls
* **Source:** [`docs/archi/security-architecture.md`](./security-architecture.md)
* **Contents:**
  * 256-bit Cookie Authentication Token (`daemon.token`) Handshake & Permissions Matrix
  * In-Transit Encryption (SSH Stdio Tunnels, Tailscale mTLS) & At-Rest Storage Controls
  * Process Group Containment (`setpgid`), Array-based Execution, and Shell Injection Guards
  * Interactive `WAITING` Approval Prompt Interceptor & Worktree Deletion Safety Gate

### Section 6: Data Design & Storage Architecture
* **Source:** [`docs/archi/data-design-and-storage-architecture.md`](./data-design-and-storage-architecture.md)
* **Contents:**
  * Mermaid Entity-Relationship Diagram (ERD)
  * Detailed SQLite Database Schemas (Tables, Constraints, Indexes)
  * Trigram FTS5 Full-Text Search Virtual Table & Rolling FIFO Pruning Policy
  * Zero-RAM Dead Session Snapshot Architecture (Zstandard Level 3 ANSI Compression)

---

## 3. V-Cycle Traceability & Verification Anchor
In accordance with the project V-Cycle, this software architecture realizes the upstream [Software Requirements Specifications (SWRS)](../specs/README.md) and provides the architectural contracts for:
1. **Low-Level Design (LLD):** Detailed module specifications located inside crate documentation directories.
2. **Software Unit Tests (SUT):** Isolated algorithmic and parser verification inside crate `tests/`.
3. **Software Verification Tests (SVT):** End-to-end integration test suites executing across the IPC protocol boundary (`tests/`).

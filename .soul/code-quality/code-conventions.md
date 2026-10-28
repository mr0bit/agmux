# Code Quality: Universal Code Conventions & Error Boundaries

## 1. Overview
All code must adhere to strict compiler checks, zero warnings, explicit error boundaries, and defensive asynchronous execution.

---

## 2. Universal Invariants

1. **Explicit Error Handling Boundaries:**
   * **Internal / Library Packages:** Use strongly-typed, exhaustive error enumerations or domain error types. Never use generic untyped errors in reusable library packages.
   * **Application Entry Points (CLI / Daemons / Frontends):** Use contextual top-level error aggregators that wrap underlying domain errors with user-facing contextual messages.
   * Never silently swallow errors; propagate context or log descriptive warnings.
2. **Compiler Quality Gates:**
   * Build with zero compiler warnings and zero linter warnings enabled.
   * Code formatting must pass automated lint/format checks in CI.
3. **Asynchronous Execution Discipline:**
   * Keep async execution reactors unblocked.
   * Never execute long-running CPU-bound computation or synchronous file system I/O directly on the async event loop; offload to dedicated blocking worker pools.
4. **Documentation Integrity:**
   * All public modules, interfaces, structs, functions, and error types must maintain clear documentation comments explaining intent, failure modes, and invariants.

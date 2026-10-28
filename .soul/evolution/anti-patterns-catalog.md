# Evolution: Universal Anti-Patterns Catalog

This catalog documents recurring failure modes observed across AI-driven and human software engineering. All agents must proactively recognize and avoid these anti-patterns.

---

## 1. Process Anti-Patterns

### AP-PROC-01: The Untraced Code Dump
* **Symptom:** Adding implementation code or tests without an upstream requirement in `docs/specs/` or a task in the backlog.
* **Harm:** Creates unmaintained, orphaned code whose rationale and business value are unknown to future agents and humans.
* **Remedy:** Every feature or structural change must trace to an atomic requirement with the 9 mandatory attributes.

### AP-PROC-02: Premature Task Closure
* **Symptom:** Marking a task `Done` in the backlog while acceptance criteria remain unchecked or tests remain unexecuted.
* **Harm:** Gives a false sense of progress and introduces silent regressions.
* **Remedy:** Definition of Done is non-negotiable. All acceptance criteria and verification gates must pass before task completion.

---

## 2. Architectural Anti-Patterns

### AP-ARCH-01: The "Utils / Helpers" Garbage Can
* **Symptom:** Creating generic dumping grounds like `src/utils/`, `helpers/`, `misc/`, or `common/`.
* **Harm:** Violates module cohesion, hides dependencies, and leads to tightly coupled spaghetti architecture.
* **Remedy:** Enforce the Module Root of Trust. Every function belongs to a bounded, documented architectural module.

### AP-ARCH-02: Speculative Indirection (Shallow Modules)
* **Symptom:** Creating single-implementation interfaces, traits, or wrapper layers "just in case we need to swap it later."
* **Harm:** Adds cognitive load and maintenance burden with zero architectural benefit.
* **Remedy:** Follow the rule of two: one adapter means a hypothetical seam; two adapters means a real seam.

---

## 3. Testing Anti-Patterns

### AP-TEST-01: Testing Private Implementation Details
* **Symptom:** Writing tests that inspect private fields, internal methods, or private state machines.
* **Harm:** Tests break every time the code is refactored, creating fragile test suites that punish optimization.
* **Remedy:** The interface is the test surface. Test observable external behavior through public seams.

### AP-TEST-02: Mock Mania
* **Symptom:** Mocking internal domain classes, database layers, and file systems using heavy mocking libraries.
* **Harm:** Tests pass against artificial mocks while failing in production environments.
* **Remedy:** Use real, in-memory stand-ins (SQLite `:memory:`, temporary directories, loopback transports). Reserve mocks strictly for third-party network services.

---

## 4. Code Quality Anti-Patterns

### AP-CODE-01: Silent Error Swallowing
* **Symptom:** Catching errors and returning default values or ignoring results (`let _ = ...`, `.unwrap_or_default()`) without logging or propagating context.
* **Harm:** Transforms critical failures into silent, impossible-to-diagnose data corruptions.
* **Remedy:** Propagate typed errors using `thiserror` (libraries) or `anyhow` (entry points) with clear descriptive context.

### AP-CODE-02: Hidden Allocations in Hot Loops
* **Symptom:** Allocating strings, vectors, or cloning buffers inside stream readers, parser loops, or frame dispatchers.
* **Harm:** Degrades latency SLAs and triggers unnecessary garbage collection or memory fragmentation.
* **Remedy:** Pass borrowed slices (`&[u8]`, `&str`) and pre-allocate reusable buffers outside the hot loop.

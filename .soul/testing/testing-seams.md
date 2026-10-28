# Testing: Testing Seams & Interface Discipline

## 1. Overview
The **interface is the test surface**. Tests assert on observable outcomes through public seams, never on private internal state.

---

## 2. Invariants

1. **Test Observable External Behavior:** If an internal refactor changes your test, your test was coupled to implementation details rather than external behavior. Tests must survive deep internal optimizations.
2. **Primary System Seam:** For Agmux, the primary integration testing seam is the **Framed IPC Boundary** (`agmux-proto`). Integration tests instantiate the daemon with virtual/loopback adapters and assert on protocol request/response streams.
3. **No Mocking Frameworks for Domain Logic:** 
   * Avoid complex mocking libraries (`mockall`) for internal domain structs.
   * Prefer in-memory test stand-ins (e.g. SQLite `:memory:`, loopback PTYs, temp directories).
   * Reserve injected ports strictly for external third-party services that cannot run locally.

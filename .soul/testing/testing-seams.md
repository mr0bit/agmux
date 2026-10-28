# Testing: Testing Seams & Interface Discipline

## 1. Overview
The **interface is the test surface**. Tests assert on observable outcomes through public seams, never on private internal state.

---

## 2. Invariants

1. **Test Observable External Behavior:** If an internal refactor changes your test, your test was coupled to implementation details rather than external behavior. Tests must survive deep internal optimizations.
2. **Primary System Seam:** Every project architecture must designate its **primary external seam** (such as a framed IPC protocol, an API gateway, or a top-level facade). Integration tests instantiate the system with in-memory or loopback adapters and assert strictly on observable request/response and event streams.
3. **No Mocking Frameworks for Domain Logic:** 
   * Avoid complex mocking libraries for internal domain structs.
   * Prefer real, lightweight in-memory test stand-ins (e.g. SQLite `:memory:`, loopback transports, temporary directories).
   * Reserve injected mock ports strictly for third-party external services that cannot execute locally.

# Code Quality: Rust Conventions & Error Handling

## 1. Overview
All Rust code in Agmux must adhere to strict compiler checks, zero warnings, and clean error ergonomics.

---

## 2. Invariants

1. **Error Handling Boundaries:**
   * **Library Crates (`agmux-proto`, `agmux-core`):** Use [`thiserror`](https://docs.rs/thiserror) to define strongly-typed, exhaustive error enums. Never use `anyhow` inside library crates.
   * **Binary Crates (`agmux-daemon`, `agmux-tui`, `agmux-cli`):** Use [`anyhow`](https://docs.rs/anyhow) for flexible top-level error context and reporting.
2. **Compiler Quality Gates:**
   * Must compile completely clean under `cargo clippy -- -D warnings`.
   * Must pass `cargo fmt --check`.
3. **Async Runtime Discipline:**
   * Use [`tokio`](https://tokio.rs) for all asynchronous I/O.
   * Never perform blocking operations (e.g. synchronous disk I/O, heavy compression) inside async Tokio tasks; use `tokio::task::spawn_blocking`.
4. **Documentation Integrity:**
   * All public structs, enums, traits, and functions must have doc comments (`///`).

# Language Profile: Rust Conventions

This language profile defines idiomatic conventions for projects using modern Rust (2024 Edition, MSRV 1.85+).

---

## 1. Error Handling Discipline
* **Library Crates:** Use [`thiserror`](https://docs.rs/thiserror) to define strongly-typed, exhaustive error enums. Never expose `anyhow` in public library APIs.
* **Binary Crates (Daemon, CLI, TUI):** Use [`anyhow`](https://docs.rs/anyhow) for flexible top-level error context and reporting (`.context("Failed to bind socket")?`).

---

## 2. Compiler Gates & Lints
* Always enforce clean compilation under:
  ```bash
  cargo clippy -- -D warnings
  cargo fmt --check
  ```
* Proactively apply `clippy::pedantic` suggestions where appropriate; disallow `unwrap()` in production paths, preferring `expect("descriptive invariant failure message")` or returning `Result`.

---

## 3. Tokio & Async Runtime
* Use [`tokio`](https://tokio.rs) for async event reactors.
* Offload blocking synchronous I/O or CPU-heavy compression to `tokio::task::spawn_blocking`.
* Graceful cancellation via `tokio::select!` and `tokio_util::sync::CancellationToken`.

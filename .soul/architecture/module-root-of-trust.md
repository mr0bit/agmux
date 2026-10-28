# Architecture: Module Root of Trust

## 1. Overview
The project architecture defines a strict **Root of Trust** for all software components. AI agents must respect module boundaries and are strictly prohibited from inventing arbitrary new files, modules, or ad-hoc helper folders outside this registry.

---

## 2. Invariants

1. **The Registry is Law:** Every source file created under `crates/` or `src/` must map directly to an entry in `docs/archi/module-list.md`.
2. **No Speculative Abstractions:** Do not create `utils/`, `helpers/`, `common/`, or `misc/` folders. If a function does not belong to a documented architectural module, the architecture document must be formally amended first.
3. **Decoupled Crate Dependencies:** Dependencies between crates follow a strict one-way directed acyclic graph:
   ```
   agmux-tui / agmux-cli  -->  agmux-proto
             │                     │
             ▼                     ▼
        agmux-daemon   -------> agmux-core
   ```
   * `agmux-proto` depends on zero internal crates (pure schema & protocol).
   * `agmux-core` contains no UI or network listener code.
   * `agmux-daemon` owns server sockets and orchestrates `agmux-core`.
   * `agmux-tui` and `agmux-cli` communicate strictly over `agmux-proto`.

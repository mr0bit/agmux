# Architecture: Module Root of Trust

## 1. Overview
The project architecture defines a strict **Root of Trust** for all software components. AI agents must respect module boundaries and are strictly prohibited from inventing arbitrary new files, modules, or ad-hoc helper folders outside this registry.

---

## 2. Invariants

1. **The Registry is Law:** Every source file created in the project repository must map directly to an entry in the project's canonical architecture specification (`docs/archi/module-list.md`).
2. **No Speculative Abstractions:** Do not create `utils/`, `helpers/`, `common/`, or `misc/` folders. If a function or type does not naturally belong to a documented architectural module, the architecture document must be formally amended and approved first.
3. **Decoupled Architecture Boundaries:** Dependencies between modules/packages must form a strict one-way directed acyclic graph (DAG):
   * Protocol and schema layers must depend on zero internal domain logic.
   * Core domain engines must remain agnostic of user interfaces and network transports.
   * Server/Daemon layers own networking and orchestrate core engines.
   * Client interfaces (CLI, TUI, GUI) communicate with the system exclusively across documented public seams.

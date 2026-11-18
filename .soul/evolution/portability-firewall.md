# The Portability Firewall

## 1. The Core Law of .soul
**`.soul` is strictly project-agnostic and universally portable.**

Any rule, process, or standard codified inside `.soul/` must be capable of being dropped into any software engineering project (Rust, Go, TypeScript, C++, Python, embedded, web, or systems) without modification.

Project-specific configurations, names, paths, and business logic belong in the project's own files (`AGENTS.md`, `docs/`, `config.toml`), **never inside `.soul/`**.

Optional reusable tool dependencies may live under `.soul/plugins/` as pinned, unchanged upstream submodules with separately maintained adapters. Tool identity and dependency-relative paths belong to those adapters; this exception does not permit host-project names, target inspection evidence, application specifications or generated applications inside `.soul`. Keep plugins optional so the core process remains usable without installing them.

```
+-----------------------------------------------------------------------------------+
|                           THE PORTABILITY FIREWALL                                |
|                                                                                   |
|   UNIVERSAL / PORTABLE (.soul/)        |   PROJECT-SPECIFIC (agmux / other repo)  |
|   --------------------------------     |   -------------------------------------  |
|   - V-Cycle traceability model         |   - Product vision & personas (PRD)      |
|   - 9 mandatory requirement attributes |   - Technical architecture specs (TRD)   |
|   - Definition of Done (DoD)           |   - Target triples & platform matrix     |
|   - Seam & Deep Module discipline      |   - Specific crate / module names        |
|   - Anti-patterns catalog              |   - Specific socket paths & port numbers |
|   - Self-evolution protocol            |   - Local environment variables          |
+-----------------------------------------------------------------------------------+
```

---

## 2. The 4-Gate Portability Test
Before any agent or engineer proposes, modifies, or promotes a rule into `.soul/`, the candidate rule must pass all four gates:

### Gate 1: The Project-Blind Test
* *Question:* Can an engineer or agent fully understand and execute this rule without knowing what product is being built?
* *Check:* If the rule references a specific product feature, user persona, or business constraint, it **fails**. Move it to the project's `docs/` or `AGENTS.md`.

### Gate 2: The Multi-Stack Test
* *Question:* Does this engineering principle hold true across different tech stacks (compiled systems languages, garbage-collected runtimes, microservices, monolithic CLI tools)?
* *Check:* If the rule is specific to a single programming language, it must reside in `.soul/languages/<lang>.md` as an opt-in language profile, not in the core process or architecture modules.

### Gate 3: The Zero-Leak Test
* *Question:* Does the file contain any hardcoded project names, repository paths, binary names, or specific port numbers?
* *Check:* Search for project branding, repo-specific paths, or hardcoded constants. If found, abstract them into parameterized contracts (e.g., replace `agmuxd.sock` with `"the project's IPC socket defined in architecture"`).

### Gate 4: The Invariant Test
* *Question:* Does this rule enforce a fundamental software engineering invariant (correctness, safety, testability, traceability, performance, maintainability)?
* *Check:* Rules that are merely temporary workarounds or personal preferences must be rejected. Only durable, high-leverage principles are admitted.

---

## 3. Enforcement
Agents operating in any repository powered by `.soul` must actively defend this firewall:
* When instructed to update `.soul`, **strip all project context** and generalize the rule.
* If a proposed rule is inherently project-specific, write it into the project's local `AGENTS.md` instead.

# Process: Multi-Agent Delegation & Fleet Discipline

## 1. Overview
When an engineering project leverages multi-agent orchestration—delegating coding, testing, and refactoring tasks across diverse AI agent CLIs (e.g., Codex, Antigravity, Claude, Aider, Cline)—the fleet must operate under a standardized, reproducible delegation protocol based on the [delegate-skills](https://github.com/amElnagdy/delegate-skills) architecture.

---

## 2. First-Run Setup Prerequisite (`delegate-setup`)
Before any autonomous delegation relays (`*-delegate`) can be dispatched in a repository, the delegation fleet **must be explicitly configured**:

1. **Discovery & Setup Command:**
   The human operator or orchestrator must run the `delegate-setup` skill:
   ```bash
   # Run discovery and fleet lane configuration
   node .agents/skills/delegate-setup/scripts/discover.mjs
   ```
2. **Configuration Generation (`.delegate/config.json`):**
   The setup workflow discovers available local CLIs, asks the grounding allocation questions, and outputs the canonical fleet configuration file to `.delegate/config.json`.
3. **Configuration Gate:**
   Agents must never assume default models or invent imaginary model parameters in code. If `.delegate/config.json` is missing, the agent must halt and prompt the user to run `delegate-setup` before proceeding with delegated execution.

---

## 3. Fleet Architecture & Lane Mapping
Fleet delegation operates on the concept of **lanes** rather than ad-hoc flags:

* **Lane Definition:** A lane pairs an implementer CLI (e.g., `codex`, `agy`, `claude`) with specific operational dials (`model`, `effort`, `variant`).
* **Symmetric Alternation (Round-Robin):** When multiple providers are available, tasks in each category (`feature`, `tests`, `refactor`, `complex`) should be configured in paired lanes (e.g., `feature1` and `feature2`) to balance quota consumption.
* **Resilient Fallback:** If an implementer in Lane 1 encounters a rate limit, quota error, or transient failure, the orchestrator automatically re-routes the task brief to Lane 2.

---

## 4. Bounded Delegation Rules
Delegated implementer subagents must adhere to strict operational boundaries:

1. **No Direct Git Commits:** Delegated implementers make changes to the working tree but **must never run `git commit`**. The human operator or orchestrator reviews the diff and lands commits.
2. **Vertical Tracer Scope:** Task briefs dispatched to implementers must follow the `to-tickets` vertical slicing discipline—cutting across schema, API, logic, and tests within a single context window.
3. **Seam Verification Before Hand-off:** Before completing a delegation relay, the implementer must execute compiler and lint gates (`cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`).

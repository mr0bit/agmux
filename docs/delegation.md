# Fleet Delegation Policy

This repository uses a dual-implementer fleet configuration ([`.delegate/config.json`](../.delegate/config.json)) balancing work between **OpenAI Codex** (`codex`) and **Google Antigravity** (`agy`).

## Alternation Policy (Round-Robin)

For every task category, lanes are defined in alternating pairs:
* **Lane 1 (`*1`)**: Dispatched to **Codex** via `codex-delegate`
* **Lane 2 (`*2`)**: Dispatched to **Antigravity** via `agy-delegate`

The orchestrator must alternate between Lane 1 and Lane 2 across successive tasks in each category to balance quota burn equally:
1. First task in a category -> use `*1` (`codex`)
2. Next task in that category -> use `*2` (`agy`)
3. Subsequent tasks -> continue alternating round-robin

### Fallback Policy
If an implementer experiences a rate-limit error, quota exhaustion, or dispatch failure, the orchestrator must automatically re-dispatch the task brief to the matching alternate lane (`*2` if `*1` failed, or vice versa).

---

## Fleet Lane Reference

| Category | Lane 1 (`codex`) | Model | Effort | Lane 2 (`agy`) | Model | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| **Feature** | `feature1` | `gpt-6.1-terra` | `medium` | `feature2` | `gemini-3.8-flash-medium` | `medium` |
| **Tests** | `tests1` | `gpt-6.1-terra` | `medium` | `tests2` | `gemini-3.8-flash-medium` | `medium` |
| **Refactor** | `refactor1` | `gpt-6.1-terra` | `medium` | `refactor2` | `gemini-3.8-flash-medium` | `medium` |
| **Complex** | `complex1` | `gpt-6.1-sol` | `high` | `complex2` | `gemini-4-argon-high` | `high` |
| **Debate** | `debate1` | `gpt-6.1-sol` | `high` | `debate2` | `gemini-4-argon-high` | `high` |

---

## Dispatch Commands

* **Codex**:
  ```bash
  node .agents/skills/codex-delegate/scripts/relay.mjs --lane <lane1> --brief <path-to-brief>
  ```
* **Antigravity**:
  ```bash
  node .agents/skills/agy-delegate/scripts/relay.mjs --lane <lane2> --brief <path-to-brief>
  ```

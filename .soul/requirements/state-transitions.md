# Requirements: State Transition Lifecycle

## 1. Overview
Requirements must progress through a deterministic, irreversible-until-broken state machine. AI agents are responsible for updating these attributes in the specification markdown files as work advances.

---

## 2. State Transition Diagram

```mermaid
stateDiagram-v2
    [*] --> NotImplemented: Requirement Authored

    state NotImplemented {
        [*] --> Initial
        Initial: Implementation Status = Not implemented
        Initial: Test Status = Failed
    }

    NotImplemented --> Implemented: Code written + LLD documented + Unit tests pass
    
    state Implemented {
        CodeReady: Implementation Status = Implemented
        CodeReady: Downstream Req = LLD-XXX
        CodeReady: Test Status = Failed (Pending SVT)
    }

    Implemented --> Verified: Downstream Verification Test (SVT) passes
    
    state Verified {
        FullyTested: Implementation Status = Verified
        FullyTested: Test Status = Passed
    }

    Verified --> NotImplemented: Regression or Spec Modification
```

---

## 3. Transition Rules for AI Agents

1. **Rule 1 (Writing Implementation):** When an agent creates the code and writes the LLD document in `crates/<crate>/docs/<module>_lld.md`:
   * Change `Implementation Status: Not implemented` ➔ `Implemented`.
   * Fill in `Downstream Req: LLD-xxx-001`.
   * Keep `Test Status: Failed` until the verification test is executed.

2. **Rule 2 (Passing Verification Tests):** When the integration test suite in `tests/` passes:
   * Change `Implementation Status: Implemented` ➔ `Verified`.
   * Change `Test Status: Failed` ➔ `Passed`.

3. **Rule 3 (Atomic Commit Readiness):** A Pull Request or feature branch cannot merge if any requirement in scope remains in `Implemented` with `Test Status: Failed`.

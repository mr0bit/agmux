# Process: The 5-Level V-Model & Traceability Chain

## 1. Overview
This engineering process enforces a bidirectional V-Model where every line of implementation code is anchored upstream by an atomic software requirement and verified downstream by a dedicated test.

```
[ Level 1: Software Specs ] -----------------------------------------> [ Level 5: Verification Tests (SVT) ]
   (docs/specs/*.md)                                                          (tests/*_tests/)
          \                                                                        /
   [ Level 2: Architecture ]                                       [ Level 4: Unit Tests (SUT) ]
      (docs/archi/*.md)                                                 (crates/*/tests/)
             \                                                           /
      [ Level 3: Low-Level Design (LLD) ] ------> [ Implementation ] ---
         (crates/*/docs/*_lld.md)                    (crates/*/src/)
```

---

## 2. The 5 Levels of the V-Model

### Level 1: Software Requirements Specifications (SWRS)
* **Location:** `docs/specs/<feature>_spec.md`
* **Artifacts:** High-level atomic requirements identified by `REQ-<DOMAIN>-<NAME>-<NUM>`.
* **Nature:** User-story style, technology-agnostic, behavior-focused.
* **Traceability Links:** Maps downstream to Software Architecture modules and Verification Tests.

### Level 2: Software Architecture (SAD)
* **Location:** `docs/archi/`
* **Artifacts:** Component topology, interface contracts, data schemas, ADRs, and the canonical `module-list.md`.
* **Nature:** Deconstructs the system into decoupled crates/modules and serves as the architectural root of trust.

### Level 3: Low-Level Design (LLD)
* **Location:** Local crate documentation: `crates/<crate>/docs/<module>_lld.md` (or `src/<module>/docs/`)
* **Artifacts:** Detailed design specifications identified by `LLD-<DOMAIN>-<NAME>-<NUM>`.
* **Nature:** Details internal state machines, algorithmic complexity, struct layouts, and private invariants.

### Level 4: Software Unit Tests (SUT)
* **Location:** `crates/<crate>/tests/` and execution report `crates/<crate>/docs/<module>_ut_report.md`.
* **Artifacts:** Fast, isolated tests verifying internal module logic, edge cases, and parsing invariants.
* **Coverage Contract:** Unit tests directly cover and verify Level 3 LLD requirements.

### Level 5: Software Verification Tests (SVT)
* **Location:** `tests/<feature>_tests/` and execution report `tests/reports/<feature>_test_report.md`.
* **Artifacts:** Holistic integration/acceptance tests identified by `TEST-VER-<DOMAIN>-<NUM>`.
* **Coverage Contract:** Verification tests verify Level 1 software requirements across external public seams (e.g. IPC).

---

## 3. The Traceability Contract
1. No requirement is considered **Implemented** unless its corresponding **Level 3 LLD** is written and code exists in the assigned module.
2. No requirement is considered **Verified** unless its corresponding **Level 5 Verification Test** passes.

# Testing: Unit Tests (SUT) vs. Verification Tests (SVT)

## 1. Overview
The testing architecture separates fast, local unit tests from comprehensive, holistic verification tests.

---

## 2. Distinction & Structure

| Dimension | Software Unit Tests (SUT) | Software Verification Tests (SVT) |
| :--- | :--- | :--- |
| **Location** | `crates/<crate>/tests/` | `tests/<feature>_tests/` |
| **Target Level** | Level 3 Low-Level Design (LLD) | Level 1 Software Specs (SWRS) |
| **Execution Speed** | Sub-millisecond (in-memory) | Milliseconds to seconds (end-to-end) |
| **Scope** | Single module or struct | Full system across IPC socket |
| **Report Location** | `crates/<crate>/docs/<module>_ut_report.md` | `tests/reports/<feature>_test_report.md` |
| **Verification Gate** | Required for `Implementation Status = Implemented` | Required for `Implementation Status = Verified` |

---

## 3. Execution Rules
* **Unit tests** verify internal algorithms, state reducers, regex parsers, and edge cases.
* **Verification tests** verify user-visible features (e.g. creating a session, detecting a blocked prompt, executing a fuzzy command palette search, compression ratio).

# Testing: Traceable Test Report Standards

## 1. Overview
All verification and unit test suites generate or maintain structured markdown test reports providing verifiable evidence of requirement satisfaction.

---

## 2. Verification Test Report Template (`tests/reports/<feature>_test_report.md`)

```markdown
# Software Verification Test Report: <Feature Name>

## 1. Execution Summary
* **Date:** YYYY-MM-DD
* **Target Version:** <git SHA / version>
* **Test Suite:** `tests/<feature>_tests/`
* **Overall Result:** [PASSED / FAILED]

---

## 2. Requirement Verification Matrix

| Requirement ID | Test Case ID | Status | Execution Duration | Evidence / Assertion |
| :--- | :--- | :--- | :--- | :--- |
| `REQ-DAEMON-AUTOSPAWN-001` | `TEST-VER-DAEMON-001` | PASSED | 12ms | Verified daemon spawned detached and socket bound |
| `REQ-DAEMON-PROCGROUP-002` | `TEST-VER-DAEMON-002` | PASSED | 45ms | Process group killed; daemon remained unharmed |

---

## 3. Logs & Output Artifacts
\`\`\`text
<Relevant test logs or terminal captures>
\`\`\`
```

---

## 4. Pull Request Verification & Visual Evidence Standards

Every Pull Request generated in this repository must provide irrefutable, visual proof that each implemented task functions end-to-end. Reviewers must be able to visually inspect and verify the behavior directly within the PR body.

### Mandatory PR Body Structure

For every task included in the PR, the PR description must contain:

1. **Task Implementation Summary:**
   * A concise technical summary of *how* the task was implemented across architectural layers (data models, IPC messages, state machines, CLI/TUI routers).
   * Exact files modified and key design decisions.
2. **Visual Proof Screenshot:**
   * An authentic, high-resolution terminal or TUI capture showing the feature executed live by a real user session.
   * Stored in `docs/assets/screenshots/<task>_verification.png` and embedded directly in markdown (`![<Task Name>](docs/assets/screenshots/<task>_verification.png)`).

---

## 5. Requirement Traceability Matrix & PR Delta

Every PR body must include the V-Cycle Traceability Matrix generated from the current specifications, detailing:

1. **Global Traceability Metrics:**
   * Total atomic requirements in repository.
   * Global Implementation Rate, Verification Rate, and Test Pass Rate before and after the PR.
2. **PR Traceability Delta:**
   * Explicit list of atomic requirements transitioned to `Verified` / `Passed` as a direct result of this PR.
   * Traceability chain for each requirement: `Upstream SRS` $\rightarrow$ `Downstream LLD` $\rightarrow$ `Verification Test` $\rightarrow$ `Backlog Task`.

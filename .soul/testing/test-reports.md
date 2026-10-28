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

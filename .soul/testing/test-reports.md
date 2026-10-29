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

### 4.1 Mandatory PR Body Structure

For every task included in the PR, the PR description must contain:

1. **Task Implementation Summary:**
   * A concise technical summary of *how* the task was implemented across architectural layers (data models, IPC messages, state machines, CLI/TUI routers).
   * Exact files modified and key design decisions.
2. **Visual Proof Screenshot:**
   * An authentic, high-resolution terminal or TUI capture showing the feature executed live by a real user session.
   * Stored in `docs/assets/screenshots/<task>_verification.png` and embedded directly in markdown (`![<Task Name>](docs/assets/screenshots/<task>_verification.png)`).

### 4.2 Private Repository Screenshot Upload Protocol (`gh pr --attach` trick)

> [!IMPORTANT]
> In private GitHub repositories, images referenced via `https://raw.githubusercontent.com/...` fail to render in web browsers with a 404 broken image placeholder, because `raw.githubusercontent.com` lacks the session OAuth cookies required for private repos.

To ensure screenshots render natively inside GitHub pull requests, agents must use GitHub CLI's asset upload mechanism:

1. **Command Syntax:**
   When creating or updating a pull request via `gh pr create` or `gh pr edit`, supply `--attach` flags pointing to the local screenshot files:
   ```bash
   gh pr edit <PR_NUMBER> \
     --body-file <PR_BODY_FILE> \
     --attach docs/assets/screenshots/task_10_verification.png \
     --attach docs/assets/screenshots/task_11_verification.png ...
   ```
2. **Automatic Link Rewriting:**
   When the markdown body contains a relative image reference matching the attached file path (e.g. `![TASK-10 Verification](docs/assets/screenshots/task_10_verification.png)`), GitHub CLI automatically:
   * Uploads the asset directly to GitHub's user-attachment CDN (`https://github.com/user-attachments/assets/<uuid>`).
   * Rewrites the markdown image link in place to point to the secure, authenticated user-attachment URL.
3. **Redundant Repository Fallback Link:**
   In addition to the embedded image, always provide a direct repository blob link immediately below the image:
   ```markdown
   ![TASK-10 Verification](docs/assets/screenshots/task_10_verification.png)
   *(Direct link: [task_10_verification.png](https://github.com/<owner>/<repo>/blob/<branch>/docs/assets/screenshots/task_10_verification.png))*
   ```
   This gives the reviewer a 1-click fallback to inspect the committed asset file directly in GitHub's file viewer.

---

## 5. Requirement Traceability Matrix & PR Delta

Every PR body must include the V-Cycle Traceability Matrix generated from the current specifications, detailing:

1. **Requirement Traceability Contribution & Delta Table:**
   A clear, explicit before-and-after contribution breakdown:
   ```markdown
   | Phase / Scope | Prior Baseline | Added in This PR | Post-PR Status | Traceability Gain |
   | :--- | :--- | :--- | :--- | :--- |
   | **Phase 0: Core Foundation** | 14 / 14 (100%) | — | 14 / 14 (100%) | Baseline |
   | **Phase 1: Power Workflow** | 0 / 7 (0%) | +7 verified | 7 / 7 (100%) | **+13.5%** |
   | **Phase 2: Multi-Agent Bus** | 0 / 5 (0%) | +4 verified | 4 / 5 (80%) | **+7.7%** |
   | **Total Global Coverage** | **14 / 52 (26.9%)** | **+11 verified** | **25 / 52 (48.1%)** | **+21.2% Net Gain** |
   ```
2. **Global Traceability Metrics:**
   * Total atomic requirements in repository.
   * Global Implementation Rate, Verification Rate, and Test Pass Rate before and after the PR.
3. **PR Traceability Delta List:**
   * Explicit list of atomic requirements transitioned to `Verified` / `Passed` as a direct result of this PR.
   * Traceability chain for each requirement: `Upstream SRS` $\rightarrow$ `Downstream LLD` $\rightarrow$ `Verification Test` $\rightarrow$ `Backlog Task`.

# Process: The 3-Step Agent Execution Rhythm

## 1. Overview
Whenever an AI agent (Codex, Antigravity, or orchestrator) is tasked with implementing a feature, bugfix, or refactor, it must follow this 3-step rhythm.

For work originating in PR feedback, first follow the [PR discussion protocol](pr-discussion.md). Its agreement gate precedes implementation; existing explicit agreement to the current plan remains valid.

---

## 2. The 3 Steps

### Step 1: Inspect & Align (Look Before You Leap)
1. **Vertical Tracer Validation:** If creating new tasks, ensure they follow the `to-tickets` vertical tracer slicing discipline with explicit blocking edges and user quiz before publishing to `.backlog/`.
2. **Read the Task:** View the task in `.backlog/` (`npx backlog.md task view <taskId>`).
3. **Review Upstream Requirements:** Read the corresponding specification file in `docs/specs/` and architecture in `docs/archi/`.
4. **Verify Module Ownership:** Check `docs/archi/module-list.md` to confirm the exact crate and module responsible for this change.
5. **Transition Task:** Run `npx backlog.md task edit <taskId> -s "In Progress"`.

### Step 2: Implement & Trace (Code with Proof)
1. **Write LLD:** Document the low-level design in `crates/<crate>/docs/<module>_lld.md`.
2. **Write Unit Tests (TDD):** Implement tests in `crates/<crate>/tests/` before or alongside the implementation.
3. **Write Implementation Code:** Implement the minimal code satisfying the acceptance criteria in the assigned crate.
4. **Update Requirements:** Update the spec file in `docs/specs/` setting `Implementation Status: Implemented` and linking `Downstream Req: LLD-*`.

### Step 3: Verify Gates & Report (Sign-Off)
1. **Run Verification Suite:** Run the software verification tests (`cargo test --test <feature>_tests`).
2. **Verify Lints & Formatting:**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   ```
3. **Capture User's Point-of-View (POV) Visual Proof Artifact:** Execute the feature live as a real user would interact with it. Capture an authentic, high-resolution screenshot representing the user's authentic perspective (CLI terminal session output, interactive TUI panels, desktop companion GUI window, or mobile push notification cards). Unit test runner logs (e.g. `cargo test`) do NOT satisfy this requirement. Store under `docs/assets/screenshots/<task>_verification.png`.
4. **Update Test Status:** Update the spec file in `docs/specs/` setting `Implementation Status: Verified` and `Test Status: Passed`.
5. **Close Task in Backlog:** Run `npx backlog.md task edit <taskId> -s "Done"`.
6. **Report with Visual Proof (User's POV) & Traceability Matrix:**
   * In PR bodies, include a dedicated technical report for each task followed by its User POV visual proof screenshot and descriptive caption.
   * **Private Repo Asset Upload Trick:** Use `gh pr edit` / `gh pr create` with `--attach docs/assets/screenshots/<task>_verification.png` flags to automatically upload images to GitHub user-attachment storage and rewrite markdown references.
   * Include the **Traceability Contribution & Delta Table** showing the baseline, per-phase gains, net gain, and post-PR status.

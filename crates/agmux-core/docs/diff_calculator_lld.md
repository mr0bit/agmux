# LLD-GIT-REVIEW-DIFF-ENGINE-003

The diff calculator invokes Git in the session working directory and converts unified patches to ordered files, hunks and lines with old and new line numbers. Session and target comparisons include tracked index and worktree changes; last commit compares HEAD to its parent; staged compares the index to HEAD. Target defaults to `main`. Git failures and missing session baselines are returned to the caller. Untracked files are outside Git's diff output.

The daemon captures HEAD before starting the session, persists it in the migrated sessions table, and runs Git on the blocking pool. Authenticated IPC carries the scope and optional target and returns the structured result or an error. The CLI renders unified text or JSON.

## LLD-UX-NONGIT-DIFF-102
Probe `git rev-parse --is-inside-work-tree` before selecting revisions/running
diff. A non-repository is `DiffError::NotRepository`; I/O/other Git failures
remain errors. No implicit initialization is performed. The existing IPC error
string is mapped at the browser bridge to `{repository:false,files:[]}`. The
explicitly scoped sentinel avoids classifying arbitrary Git failures as empty
changes. `TEST-VER-UX-102` uses a real plain directory and the browser/daemon seam;
the existing five-scope test retains repository coverage.

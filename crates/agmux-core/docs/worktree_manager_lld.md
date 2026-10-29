# Worktree manager design

**LLD-GIT-WORKTREE-PROVISION-001**: The manager invokes Git to create a branch checkout under `.agmux-worktrees`, copies root-level files selected by `.worktreeinclude` without replacing checked-out files, and derives a stable port from the branch name. It removes a newly created checkout if copying fails. The daemon discovers the repository from the client working directory and passes the checkout and port to the PTY. Removal invokes `git worktree remove --force`.

This tracer uses the Git executable as specified in TASK-10; the upstream requirement's pure-Rust Git backend remains outstanding. Include patterns support root-level `*` and `?`, not recursive paths or the full Git glob syntax. Ports are deterministic but may collide.

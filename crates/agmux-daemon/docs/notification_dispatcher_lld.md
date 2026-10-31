# Notification dispatcher

`LLD-GUI-NOTIFICATION-HOOKS-001` implements `REQ-GUI-NOTIFICATION-HOOKS-001`. Canonical `WAITING_INPUT`, `COMPLETED`, and `FAILED` transitions map to `WAITING`, `FINISHED`, and `ERRORED`. The broadcaster obtains plain text from the live VT screen and retains its last 120 Unicode scalar values. The bounded SQLite history is available through companion IPC; clients also receive the structured event on the normal broadcaster.

A maximum of four jobs gather config and metadata and run hooks away from the PTY processing path. Set `notification_hook = "command"` in the daemon's `AGMUX_CONFIG_PATH` (default `agmux.toml`). The configured command is the shell program; session values are supplied solely as the eight documented `AGMUX_*` environment variables. Session project metadata supplies the name, falling back to ID, and the branch is queried from the session working directory. Unix hooks run in a separate process group and the group is killed at timeout; Windows attaches the hook to a kill-on-close Job Object. Timeout is 15 seconds. Excess work is dropped rather than accumulating an unbounded queue.

Verification: `notification_dispatcher_test` exercises Unicode, all variables and a real 15-second timeout including a descendant; `TEST-VER-GUI-001` exercises a Tier 1 notification through daemon IPC.

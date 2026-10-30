# Deferred teardown low-level design

## LLD-ORCH-SELF-TEARDOWN-008

Authenticated teardown requests serialize schedule, cancellation and completion through a daemon-owned async gate. Pending actions are stored in SQLite and included in session-list envelopes for client badges. Only live sessions can schedule or cancel. A completion hook, idle acknowledgment, already-idle scheduling, or process exit consumes the action. The daemon captures a compressed screen, atomically commits ARCHIVED or DEAD and removes the request, terminates the process group, releases locks, reservations and task claims, removes the active PTY and broadcasts the list. Ordinary late classifier transitions cannot overwrite terminal teardown states. Crash recovery clears abandoned pending actions for recovered sessions. Archived and dead snapshots remain inspectable. TUI `c` sends a cancellation request for the selected session.

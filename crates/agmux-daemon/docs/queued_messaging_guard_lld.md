# LLD-ORCH-QUEUED-MSG-GUARD-001: Queued messaging guard

The authenticated IPC request carries a target and a message. The daemon reads the persisted session state while holding the storage lock; WAITING and WAITING_INPUT messages are recorded in SQLite without writing to the PTY. Other states require an active PTY and write a newline-terminated message. An IDLE acknowledgment attempts pending messages in database ID order and records each successful write; unavailable PTYs and failed writes leave messages pending.

The existing classifier emits WAITING_INPUT, RUNNING and COMPLETED, but has no live turn-finished or IDLE signal. Consequently prompt resolution cannot yet trigger automatic delivery. Process exit is not a valid delivery point because the PTY child has already terminated. A live turn-completion signal must be introduced before this design satisfies the requirement.

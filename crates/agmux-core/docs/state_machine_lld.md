# State classifier tracer design

The session classifier starts in RUNNING, retains a bounded terminal tail for prompts split across PTY chunks, and tracks the last output or input time. Prompt patterns select WAITING_INPUT; delivered input or subsequent non-prompt output resumes RUNNING. A one-shot inactivity deadline changes RUNNING to STUCK; output resumes RUNNING. Process exit selects COMPLETED or FAILED. IDLE and FROZEN are reserved for future lifecycle signals.

The daemon owns one classifier per PTY, persists each transition with a SQLite update before broadcasting the IPC event, and configures the inactivity deadline with `AGMUX_STUCK_TIMEOUT_MS` (default 120000). Prompt recognition is heuristic and does not inspect the operating system's stdin wait state or a VT screen.

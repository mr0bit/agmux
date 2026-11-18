
## LLD-UX-PROMPT-SUBMIT-101
`ActivePtySession::submit_prompt` is the one native prompt seam. It preserves
message bytes, adds bracketed-paste framing when the child has enabled it, and
appends exactly one CR (keyboard Enter). All daemon prompt routes use it;
raw terminal keystrokes still use `send_input`. The browser supplies a message,
never a platform-specific terminator. Structured Gemini prompts use ACP instead.
`TEST-VER-UX-101` exercises two submissions and multiline framing through HTTP,
authenticated IPC, PTY and an external raw-input fixture. Native Windows
submission evidence remains required by the acceptance specification.


## Native Windows exit notification
Child-process exit and terminal output EOF are distinct. A ConPTY output pipe
can remain open while the connected viewport retains its master. After reaping
the child, Windows allows at most 250 ms for final output drain, then publishes
the actual exit code independently of EOF. The detached reader can finish when
the master is released. Unix keeps its existing complete EOF drain. The native
handoff still waits for the reaped-child exit event before starting another
writer. `windows_conpty_exit_is_published_while_viewport_retains_master` covers
natural exit and forced termination through the public PTY API; the packaged
Windows browser suite covers native exit and same-UUID Chat handoff.

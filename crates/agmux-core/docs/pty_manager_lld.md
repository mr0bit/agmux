
## LLD-UX-PROMPT-SUBMIT-101
`ActivePtySession::submit_prompt` is the one native prompt seam. It preserves
message bytes, adds bracketed-paste framing when the child has enabled it, and
appends exactly one CR (keyboard Enter). All daemon prompt routes use it;
raw terminal keystrokes still use `send_input`. The browser supplies a message,
never a platform-specific terminator. Structured Gemini prompts use ACP instead.
`TEST-VER-UX-101` exercises two submissions and multiline framing through HTTP,
authenticated IPC, PTY and an external raw-input fixture. Native Windows
submission evidence remains required by the acceptance specification.

# Palette and prompt tracer design

The TUI keeps one modal state alongside the selected session. The command palette
filters action labels case-insensitively and uses arrow keys to select a result.
Executing a session jump changes the selected viewport; freeze and new session
produce daemon requests. The attention filter affects sidebar visibility.

The quick prompt collects text until Enter or Escape. Enter creates a terminal
input frame addressed to the selected session, with a trailing newline. The TUI
event loop sends the pending frame over the existing authenticated connection.

This tracer does not yet implement all of REQ-TUI-COMMAND-PALETTE-002 or
REQ-TUI-QUICK-PROMPT-003: Nucleo matching, the specified additional actions,
and prompt modifier controls remain outstanding.

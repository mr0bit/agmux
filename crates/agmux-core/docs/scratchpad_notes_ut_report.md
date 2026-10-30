# Unit test report: scratchpad notes

- Date: 2026-10-30
- Design: `LLD-TUI-SCRATCHPAD-NOTES-008`
- Command: `cargo test -p agmux-core --test scratchpad_notes_test`
- Result: PASSED

The test writes and replaces independent pages, reopens the SQLite database,
checks page ordering and missing-page behavior, and verifies that the FTS index
reflects replacements and treats FTS operators as literal input.

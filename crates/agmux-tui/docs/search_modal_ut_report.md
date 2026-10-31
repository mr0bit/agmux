# Unit test report: search modal and notes UI

- Date: 2026-10-30
- Designs: `LLD-TUI-GLOBAL-SEARCH-007`, `LLD-TUI-SCRATCHPAD-NOTES-008`
- Command: `cargo test -p agmux-tui --test search_notes_test`
- Result: PASSED

The tests exercise Ctrl modifiers, correlated stale responses, sidebar selection
under filtering, page navigation and edits, failure to load a page without
allowing an overwrite, and rendering a preview scrolled to a distant match.

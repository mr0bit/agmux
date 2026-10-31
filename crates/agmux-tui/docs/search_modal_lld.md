# Global search and sidebar filter low-level design

## LLD-TUI-GLOBAL-SEARCH-007

The TUI's `search_modal` state handles Ctrl+F and `/` independently of the
viewport. Each nonempty search or filter edit sends a framed `SearchNotesRequest`
with its literal query. The daemon searches the trigram FTS5 transcript and note
indexes on a blocking worker; short strings use a literal substring fallback.
Search responses carry the original query, source session, optional note page,
matched line, and surrounding read-only lines. The TUI discards responses for a
query that is no longer displayed, selects results with up/down, and scrolls the
context pane to place the highlighted line in view. Empty input clears results.
Errors are displayed without terminating the client. Transcript-index events and
note changes cause an open search to refresh.

The filter daemon operation compares session ID, command, Git branch and note
content. Its correlated response supplies the set of matching session IDs. The
TUI intersects this set with the attention filter and maintains the selected
index in that visible set, including a no-selection state for zero matches.
Filter updates are refreshed after session-list changes.

## LLD-TUI-SCRATCHPAD-NOTES-008 (TUI)

`N` opens the selected session's page 1 editor; Ctrl+PageUp/PageDown opens it
or changes pages. Tab/Shift+Tab and F7/F8 provide navigation fallbacks. Page
loads are correlated by session and page and editing waits for the load. Each
edit sends an ordered upsert to the daemon, preserving newlines and allowing
Escape after the last edit. Pages are displayed as text, and note errors appear
in the status bar. The CLI exposes `note set SESSION PAGE CONTENT`, `get`, and
`list` through the same authenticated framed IPC request family.

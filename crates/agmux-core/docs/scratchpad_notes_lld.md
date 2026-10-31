# Scratchpad notes low-level design

## LLD-TUI-SCRATCHPAD-NOTES-008

The storage engine creates `session_notes` on database open. Its composite
primary key identifies a page by session ID and page number. `set_note` uses
an upsert, `get_note` distinguishes a missing page from an empty page, and
`list_notes` returns pages in numeric order. SQLite persists pages across daemon
restarts. An FTS5 trigram table is maintained by insert/update/delete triggers;
startup backfills missing rows. Literal search escapes FTS syntax and uses a
substring fallback for queries shorter than three characters. The daemon
serializes storage access on a blocking worker and validates a session before
writing. The same framed request family serves the CLI and TUI.

# Terminal history ingestion

`TerminalHistory` keeps bounded raw and stripped byte streams and carries escape parsing state across PTY reads. Complete lines and the final partial line are indexed by the daemon in per-read SQLite transactions. Lines are numbered from one per session. The FTS5 trigram table stores clean text with session and timestamp metadata. Search treats the query as a literal phrase, falls back to substring scanning for queries shorter than three characters, and optionally filters by session.

This tracer does not yet implement configurable retention or the 250 ms batching policy required by REQ-STORAGE-FTS5-RETENTION-003.

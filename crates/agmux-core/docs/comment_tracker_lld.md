# LLD-GIT-DUAL-RESOLUTION-005

Sent review comments are candidates for automatic resolution when their file appears on either side of a diff. File-level matching also handles deleted files and changes without textual hunks. The daemon performs this check on uncommitted or last-commit diff inspection, and persists matching IDs as resolved. Explicit resolution uses an authenticated IPC request and optionally restricts the ID to the supplied session. Unknown IDs return an error. The tracker is a pure function; Git and SQLite remain daemon responsibilities.

This tracer bullet compares the inspected diff as a whole; it does not capture a turn baseline or compare line hashes. An earlier change in that diff can therefore resolve a newly sent comment. MCP resolution is outside this implementation.

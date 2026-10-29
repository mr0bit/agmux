# Session storage design

`LLD-STORAGE-SQLITE-WAL-001`: The daemon owns a SQLite connection configured for WAL, normal synchronous writes and memory temporary storage. Repository operations insert, update, list and remove session metadata. Blocking database calls run on Tokio's blocking pool and are serialized by the daemon.

`LLD-STORAGE-ATOMIC-MIGRATION-002`: Opening a version-zero database creates the sessions table and updates `user_version` in one transaction before the IPC listener starts. Newer unknown versions are rejected.

Session creation persists the child PID, command, working directory and Unix creation time before reporting success. A separate exit watcher marks it DEAD even when its client disconnects. Listing returns non-DEAD and non-ERRORED records through authenticated IPC.

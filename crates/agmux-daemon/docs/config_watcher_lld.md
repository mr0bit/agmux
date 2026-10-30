# LLD-STORAGE-CONFIG-HOTRELOAD-005: Dynamic Configuration Hot-Reload Watcher

* **Unique ID:** `LLD-STORAGE-CONFIG-HOTRELOAD-005`
* **Upstream Requirement:** `REQ-STORAGE-CONFIG-HOTRELOAD-005`
* **Downstream Verification Test:** `TEST-VER-STORAGE-005` (`tests/e2e_daemon_resilience_test.rs`)
* **SW Component:** `crates/agmux-daemon::config_watcher`

## 1. Overview
`ConfigWatcher` monitors the configuration file (`agmux.toml` or `AGMUX_CONFIG_PATH`) for modifications and atomically updates the shared `RuntimeConfig` without restarting `agmuxd` or dropping active PTY sessions.

## 2. Hot-Reloaded Parameters
- `idle_after_ms: u64` (default `120_000`)
- `blocked_settle_ms: u64` (default `500`)
- `fts_retention_lines: usize` (default `10_000`)

## 3. Change Detection & Journal Notification
A background task polls the configuration file's modification timestamp and content digest at a short interval (100ms). When a change is parsed, `ConfigWatcher` updates the `Arc<RwLock<RuntimeConfig>>` and records a `config.reloaded` event in the SQLite `event_journal` with the updated settings payload.

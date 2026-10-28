# API & Interface Architecture — Agmux

## 1. IPC Protocol Specification (`agmux-proto`)

Communication between frontend clients (`agmux-tui`, `agmux-cli`, `agmux-gui`) and `agmuxd` uses length-delimited Bincode 2.0 binary frames.

### 1.1 Frame Layout
Each message frame transmitted over the socket or pipe consists of:
```
+-----------------------------------+-----------------------------------------+
| Frame Length: 32-bit unsigned LE  | Payload: Bincode 2.0 serialized Enum    |
| (4 bytes)                         | (N bytes)                               |
+-----------------------------------+-----------------------------------------+
```

### 1.2 Authentication Handshake
Immediately upon establishing a socket connection, the client must transmit an authentication handshake packet:
```rust
pub struct AuthHandshake {
    pub client_version: String,
    pub token: [u8; 32], // Read from ~/.local/share/agmux/daemon.token
}
```
If the token matches, `agmuxd` replies with `AuthResult::Success { daemon_version: String }`; otherwise, the daemon closes the connection.

### 1.3 `ClientToDaemon` Message Schema
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientToDaemon {
    // Session Management
    CreateSession {
        project_id: Option<String>,
        group_id: Option<String>,
        parent_session_id: Option<String>, // Non-null for sub-tabs
        title: String,
        agent_tool: String,
        model: Option<String>,
        reasoning_effort: Option<String>,
        profile: Option<String>,
        cwd: PathBuf,
        use_worktree: bool,
    },
    KillSnapshot { session_id: String },
    Revive { session_id: String },
    RestartFresh { session_id: String },
    Fork { session_id: String, target: ForkTarget }, // NewSession or NewSubTab
    ArchiveSession { session_id: String },
    DeleteSession { session_id: String, force: bool },

    // Terminal I/O & Viewport
    SubscribeViewport { session_id: String },
    UnsubscribeViewport { session_id: String },
    SendInput { session_id: String, bytes: Vec<u8> },
    Resize { session_id: String, cols: u16, rows: u16 },
    QuickPrompt { target_id: String, prompt: String, image_chips: Vec<PathBuf> },

    // Search & Command Palette
    SearchFts { query: String, limit: usize },
    CommandPaletteExec { action_id: String, target_id: Option<String> },

    // Git & Code Review
    CreateReviewComment { session_id: String, file_path: String, line_start: u32, line_end: u32, line_hash: String, body: String },
    SubmitReviewRound { session_id: String },
    ResolveReviewComment { comment_id: String },

    // Orchestration Bus
    AcquireLock { lock_name: String, session_id: String, wait: bool },
    ReleaseLock { lock_name: String, session_id: String },
    ReserveFiles { session_id: String, relative_paths: Vec<PathBuf> },
    ReleaseFiles { session_id: String, relative_paths: Vec<PathBuf> },
    CreateTask { project_id: String, title: String, description: String },
    ClaimTask { task_id: String, session_id: String },
    FinishTask { task_id: String, session_id: String },
    SubmitReport { session_id: String, report_json: String },
    WatchEvents { cursor: Option<String>, follow: bool },
}
```

### 1.4 `DaemonToClient` Message Schema
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonToClient {
    AuthResult { success: bool, error: Option<String> },
    ScreenSyncFrame {
        session_id: String,
        cols: u16,
        rows: u16,
        cursor_col: u16,
        cursor_row: u16,
        cells: Vec<CellData>, // UTF-8 char + foreground + background + attributes
        scrollback_offset: usize,
    },
    ScreenDelta {
        session_id: String,
        dirty_cells: Vec<(u16, u16, CellData)>,
    },
    SessionListUpdate {
        sessions: Vec<SessionSummary>,
    },
    SearchResults {
        query: String,
        hits: Vec<SearchHit>,
    },
    NotificationEvent {
        session_id: String,
        kind: NotificationKind, // Waiting, Finished, Errored, Stuck
        quote: String,
    },
    EventJournalBatch {
        events: Vec<JournaledEvent>,
    },
    OperationResult {
        request_id: u64,
        success: bool,
        error: Option<String>,
    },
}
```

---

## 2. Configuration Specifications

### 2.1 Daemon Configuration (`config.toml`)
Default location: `~/.config/agmux/config.toml` (Linux/macOS) or `%APPDATA%\agmux\config.toml` (Windows).

```toml
[daemon]
socket_path = "/run/user/1000/agmuxd.sock"
idle_after_ms = 1000            # Settle timer before transition to FINISHED
blocked_settle_ms = 400         # Settle timer before WAITING notification fires
stuck_timeout_ms = 10000        # Held input duration before transition to STUCK

[storage]
database_path = "~/.local/share/agmux/state.db"
fts_retention_lines = 50000     # Maximum rolling FTS5 indexed lines per session
wal_autocheckpoint_pages = 1000

[worktree]
base_dir = ".agmux-worktrees"
auto_worktree = false
base_port = 3000
port_stride = 1

[notifications]
enable_desktop = true
hook_command = "notify-send '{title}' '{body}'"
```

### 2.2 CLI Adapter Registry (`agents.toml`)
Default location: `~/.config/agmux/agents.toml`.

```toml
[agents.claude]
binary = "claude"
id_strategy = "session_id_flag"
spawn_args = ["--session-id", "{uuid}"]
resume_args = ["--resume", "{uuid}"]
fork_args = ["--resume", "{uuid}", "--fork-session"]
discovery = "static"

[agents.agy]
binary = "agy"
id_strategy = "session_store"
store_path = "~/.antigravity/sessions/*.json"
resume_args = ["--resume", "{uuid}"]
discovery = "static"

# Custom User Agent Example
[agents.custom_qa]
binary = "my-qa-agent"
id_strategy = "stdout_regex"
id_regex = "Session ID: ([a-f0-9-]+)"
prompt_regex = "Prompt: "
limit_regex = "Rate limited"
stuck_timeout_ms = 15000
spawn_args = ["--auto-confirm"]
resume_args = ["--continue", "{uuid}"]
```

---

## 3. Environment Variables & Constants

### 3.1 Injected Environment Variables
Every spawned PTY session receives the following environment variables:
* `AGMUX_SESSION_ID`: Canonical session UUID.
* `AGMUX_SESSION_NAME`: Display name of the session.
* `AGMUX_TOOL`: Active agent profile name (e.g. `claude`, `agy`).
* `AGMUX_CWD`: Active working directory.
* `AGMUX_BRANCH`: Active Git branch (if inside a git repository).
* `PORT`: Deterministic assigned port (`base_port + slot_id`).
* `AGMUX_SOCK`: Path to the active `agmuxd` IPC socket.

### 3.2 Notification Hook Variables
When executing a user-configured notification command, `agmuxd` injects:
* `AGMUX_NOTIFY_KIND`: `WAITING`, `FINISHED`, `ERRORED`, or `STUCK`.
* `AGMUX_TITLE`: Session title.
* `AGMUX_BODY`: 120-character preview of the blocking question or error.

### 3.3 System Constants
| Constant Name | Value | Purpose |
| :--- | :--- | :--- |
| `DEFAULT_SCROLLBACK_LINES` | `10000` | In-memory VT100 ring buffer size per active session |
| `DEFAULT_FTS_RETENTION` | `50000` | Maximum indexed terminal lines in SQLite FTS5 table |
| `DEFAULT_BLOCKED_SETTLE_MS` | `400` | Silence duration before firing `WAITING` notification |
| `DEFAULT_IDLE_AFTER_MS` | `1000` | Silence duration before marking turn `FINISHED` |
| `DEFAULT_STUCK_TIMEOUT_MS` | `10000` | Held input duration before transition to `STUCK` |
| `FTS_FLUSH_INTERVAL_MS` | `250` | Ingestion batch interval for SQLite WAL transactions |
| `ZSTD_COMPRESSION_LEVEL` | `3` | Compression level for dead session ANSI snapshots |

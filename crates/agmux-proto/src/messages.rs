//! Strongly-typed IPC message definitions for client-daemon communication.

use serde::{Deserialize, Serialize};

/// Handshake request sent by a client immediately upon connecting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeRequest {
    /// Semantic version of the connecting client.
    pub client_version: String,
    /// 32-byte authentication token (hex or string encoded).
    pub token: String,
}

/// Handshake response returned by the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeResponse {
    /// Whether the authentication handshake succeeded.
    pub success: bool,
    /// Semantic version of the running daemon.
    pub daemon_version: String,
    /// Optional error description if authentication failed.
    pub error: Option<String>,
}

/// Request to spawn a command inside a new PTY session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpawnSessionRequest {
    /// Unique identifier for the session.
    pub session_id: String,
    /// Executable command name or path.
    pub command: String,
    /// Command line arguments.
    pub args: Vec<String>,
    /// Optional working directory for the spawned process.
    pub cwd: Option<String>,
    /// Optional Git branch for an isolated worktree.
    #[serde(default)]
    pub worktree_branch: Option<String>,
    /// When true, session output is not forwarded over the spawning connection.
    #[serde(default)]
    pub detached: bool,
    /// Initial terminal columns, if specified by client.
    #[serde(default)]
    pub cols: Option<u16>,
    /// Initial terminal rows, if specified by client.
    #[serde(default)]
    pub rows: Option<u16>,
}

/// Daemon response indicating whether session spawning succeeded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpawnSessionResponse {
    /// Session identifier matching the request.
    pub session_id: String,
    /// Whether process spawn was successful.
    pub success: bool,
    /// Optional error message if spawning failed.
    pub error: Option<String>,
}

/// Terminal output frame forwarded from PTY master to subscribed clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalOutputFrame {
    /// Target session identifier.
    pub session_id: String,
    /// Raw terminal byte payload (ANSI escape sequences or text).
    pub data: Vec<u8>,
}

/// Terminal input frame forwarded from client to PTY master stdin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalInputFrame {
    /// Target session identifier.
    pub session_id: String,
    /// Raw keyboard or input byte stream.
    pub data: Vec<u8>,
}

/// Event notification emitted when a session's child process exits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionExitEvent {
    /// Session identifier.
    pub session_id: String,
    /// Process exit code if available.
    pub exit_code: Option<i32>,
}

/// Notification of a persisted session state transition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStateChangedEvent {
    pub session_id: String,
    pub old_state: String,
    pub new_state: String,
    pub reason: String,
}

/// Request the active sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSessionsRequest {}

/// Persisted session metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfo {
    pub session_id: String,
    pub pid: u32,
    pub cwd: String,
    pub command: String,
    /// Unix timestamp in seconds.
    pub created_at: u64,
    pub state: String,
}

/// Active sessions returned by the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionInfo>,
    #[serde(default)]
    pub teardowns: std::collections::HashMap<String, String>,
}

/// Search request, optionally restricted to one session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchLogsRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub limit: u32,
}
/// A matching terminal line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchMatch {
    pub session_id: String,
    pub line_no: u64,
    pub timestamp: u64,
    pub snippet: String,
}
/// Search results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchLogsResponse {
    pub matches: Vec<SearchMatch>,
}

/// Scratchpad and interactive search operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchNotesRequest {
    Set {
        session_id: String,
        page: u32,
        content: String,
    },
    Get {
        session_id: String,
        page: u32,
    },
    List {
        session_id: String,
    },
    Search {
        query: String,
    },
    Filter {
        query: String,
    },
}

/// A transcript or scratchpad match and surrounding read-only context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalSearchHit {
    pub session_id: String,
    pub page: Option<u32>,
    pub line_no: u64,
    pub snippet: String,
    pub context: Vec<String>,
    pub context_match: usize,
}

/// Correlated response for notes, search and filtering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchNotesResponse {
    pub request: SearchNotesRequest,
    pub pages: Vec<(u32, String)>,
    pub hits: Vec<GlobalSearchHit>,
    pub session_ids: Vec<String>,
    pub error: Option<String>,
}

/// Request to persist and stop an active session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreezeSessionRequest {
    pub session_id: String,
}

/// Result of freezing a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreezeSessionResponse {
    pub success: bool,
    pub compressed_size: usize,
}

/// Request to inspect a persisted session snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeSessionRequest {
    pub session_id: String,
}

/// Decompressed terminal data from a persisted session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeSessionResponse {
    pub success: bool,
    pub screen_data: Vec<u8>,
}

/// Request acknowledgment of a completed session, resetting it to IDLE.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcknowledgeSessionRequest {
    pub session_id: String,
}

/// Result of session acknowledgment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcknowledgeSessionResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Request removal of a persisted session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoveSessionRequest {
    pub session_id: String,
    pub force: bool,
}

/// Result of a session removal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoveSessionResponse {
    pub success: bool,
    pub stash_created: Option<String>,
    pub error: Option<String>,
}

/// Comparison selected for a review diff.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffScope {
    SessionDiff,
    FullUncommitted,
    VersusTarget,
    LastCommit,
    Staged,
}

/// One changed file and its ordered hunks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub hunks: Vec<DiffHunk>,
}
/// Unified hunk coordinates and lines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u64,
    pub old_count: u64,
    pub new_start: u64,
    pub new_count: u64,
    pub header: String,
    pub lines: Vec<DiffLine>,
}
/// A line and its position on each side of the comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineType,
    pub content: String,
    pub old_line: Option<u64>,
    pub new_line: Option<u64>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    NoNewline,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetDiffRequest {
    pub session_id: String,
    pub scope: DiffScope,
    pub target: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetDiffResponse {
    pub files: Vec<FileDiff>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewComment {
    pub id: String,
    pub session_id: String,
    pub file_path: String,
    pub line_number: u64,
    pub line_hash: String,
    pub content: String,
    pub status: String,
    pub created_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddReviewCommentRequest {
    pub session_id: String,
    pub file_path: String,
    pub line_number: u64,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddReviewCommentResponse {
    pub success: bool,
    pub comment_id: Option<String>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReviewCommentsRequest {
    pub session_id: String,
    pub status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReviewCommentsResponse {
    pub success: bool,
    pub comments: Vec<ReviewComment>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolveReviewCommentRequest {
    pub session_id: Option<String>,
    pub comment_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolveReviewCommentResponse {
    pub success: bool,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchReviewRoundRequest {
    pub session_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchReviewRoundResponse {
    pub success: bool,
    pub round_number: u32,
    pub count: usize,
    pub prompt: String,
    pub error: Option<String>,
}

/// Request asynchronous delivery to a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessageRequest {
    pub target_session_id: String,
    pub message: String,
}

/// Result of a message delivery request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessageResponse {
    pub success: bool,
    pub queued: bool,
    pub error: Option<String>,
}

/// Request to acquire a named resource lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcquireLockRequest {
    pub lock_name: String,
    pub session_id: String,
    pub wait: bool,
}

/// Result of acquiring a named lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcquireLockResponse {
    pub success: bool,
    pub acquired: bool,
    pub holder_session_id: Option<String>,
    pub error: Option<String>,
}

/// Request to release a named resource lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseLockRequest {
    pub lock_name: String,
    pub session_id: String,
}

/// Result of releasing a named lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseLockResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Request active locks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListLocksRequest {}

/// Information about an active lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockEntry {
    pub name: String,
    pub holder_session_id: String,
    pub acquired_at: u64,
}

/// List of active named locks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListLocksResponse {
    pub locks: Vec<LockEntry>,
    pub error: Option<String>,
}

/// Request to reserve paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReserveFilesRequest {
    pub session_id: String,
    pub paths: Vec<String>,
}

/// Result of reserving paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReserveFilesResponse {
    pub success: bool,
    pub conflicting_path: Option<String>,
    pub holder_session_id: Option<String>,
    pub error: Option<String>,
}

/// Request to release reserved paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseFilesRequest {
    pub session_id: String,
    pub paths: Vec<String>,
}

/// Result of releasing paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseFilesResponse {
    pub success: bool,
    pub released_count: usize,
    pub error: Option<String>,
}

/// Request active file reservations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReservationsRequest {}

/// Active file reservation entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileReservationEntry {
    pub path: String,
    pub holder_session_id: String,
    pub acquired_at: u64,
}

/// List of active file reservations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReservationsResponse {
    pub reservations: Vec<FileReservationEntry>,
    pub error: Option<String>,
}

/// Journaled workspace event with monotonic sequence number.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalEvent {
    pub seq: u64,
    pub event_type: String,
    pub session_id: Option<String>,
    pub payload: String,
    pub timestamp: u64,
    pub replay: bool,
}

/// Request streaming journaled workspace events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchEventsRequest {
    pub cursor_id: Option<String>,
    pub since_seq: Option<u64>,
    pub follow: bool,
}

/// Streamed event frame sent to watching clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchEventFrame {
    pub event: Option<JournalEvent>,
    pub changed_while_away: bool,
    #[serde(default)]
    pub is_done: bool,
}

/// Acknowledge a cursor sequence position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckCursorRequest {
    pub cursor_id: String,
    pub seq: u64,
}

/// Result of acknowledging cursor position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckCursorResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Shared Kanban board task entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KanbanTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub claimed_by: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Structured worker completion report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerReport {
    pub task_id: String,
    pub session_id: String,
    pub summary: String,
    pub pr_url: Option<String>,
    pub commit_sha: Option<String>,
    pub created_at: u64,
}

/// Request to create a Kanban task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateTaskRequest {
    pub task_id: String,
    pub title: String,
    pub description: String,
}

/// Result of creating a Kanban task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateTaskResponse {
    pub success: bool,
    pub task: Option<KanbanTask>,
    pub error: Option<String>,
}

/// Request to list Kanban tasks, optionally filtered by status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListTasksRequest {
    pub status: Option<String>,
}

/// List of Kanban tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListTasksResponse {
    pub tasks: Vec<KanbanTask>,
    pub error: Option<String>,
}

/// Request to claim a Kanban task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimTaskRequest {
    pub task_id: String,
    pub session_id: String,
}

/// Result of claiming a Kanban task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimTaskResponse {
    pub success: bool,
    pub task: Option<KanbanTask>,
    pub error: Option<String>,
}

/// Request to mark a Kanban task finished.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinishTaskRequest {
    pub task_id: String,
    pub session_id: String,
}

/// Result of marking a Kanban task finished.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinishTaskResponse {
    pub success: bool,
    pub task: Option<KanbanTask>,
    pub error: Option<String>,
}

/// Request to release a claimed Kanban task back to todo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseTaskRequest {
    pub task_id: String,
    pub session_id: String,
}

/// Result of releasing a Kanban task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseTaskResponse {
    pub success: bool,
    pub task: Option<KanbanTask>,
    pub error: Option<String>,
}

/// Request to post a structured worker report for a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostReportRequest {
    pub task_id: String,
    pub session_id: String,
    pub summary: String,
    pub pr_url: Option<String>,
    pub commit_sha: Option<String>,
}

/// Result of posting a worker report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostReportResponse {
    pub success: bool,
    pub report: Option<WorkerReport>,
    pub error: Option<String>,
}

/// Request to view structured worker reports for a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewReportRequest {
    pub task_id: String,
}

/// Result of viewing worker reports for a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewReportResponse {
    pub success: bool,
    pub reports: Vec<WorkerReport>,
    pub error: Option<String>,
}

/// Request graceful daemon shutdown with SIGHUP -> SIGTERM propagation to active child process groups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShutdownDaemonRequest {}

/// Result of graceful daemon shutdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShutdownDaemonResponse {
    pub success: bool,
    pub terminated_sessions: usize,
}

/// Atomic screen state snapshot sent immediately upon `SubscribeViewport`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenSyncFrame {
    pub session_id: String,
    pub rows: u16,
    pub cols: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub scrollback_offset: usize,
    pub formatted_screen: Vec<u8>,
    pub lines: Vec<String>,
}

/// Asynchronous notification event pushed to subscribed clients on state transitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationEvent {
    pub session_id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub timestamp: u64,
}

/// Asynchronous session list update pushed to subscribed clients on session lifecycle changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionListUpdate {
    pub sessions: Vec<SessionInfo>,
    #[serde(default)]
    pub teardowns: std::collections::HashMap<String, String>,
}

/// Request to resize a PTY session grid and propagate `TIOCSWINSZ`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResizeSessionRequest {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

/// Result of resizing a PTY session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResizeSessionResponse {
    pub success: bool,
    pub cols: u16,
    pub rows: u16,
    pub error: Option<String>,
}

/// Request to spawn an interactive shell sub-tab inheriting a parent session's CWD and PORT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpawnSubtabRequest {
    pub parent_session_id: String,
    pub subtab_id: String,
    pub shell: Option<String>,
}

/// Result of spawning an interactive shell sub-tab.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpawnSubtabResponse {
    pub success: bool,
    pub subtab_id: String,
    pub cwd: String,
    pub port: u16,
    pub error: Option<String>,
}

/// Request to emit a Tier 1 deterministic CLI hook event (`working`, `permission_request`, `stop`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalHookRequest {
    pub session_id: String,
    pub event: String,
}

/// Result of processing a Tier 1 deterministic CLI hook event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalHookResponse {
    pub success: bool,
    pub new_state: String,
    pub error: Option<String>,
}

/// Request to federate and mount a remote daemon over SSH or Tailscale (`REQ-REMOTE-FEDERATION-SSH-001`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectRemoteRequest {
    pub target: String,
    #[serde(default)]
    pub remote_socket: Option<String>,
    #[serde(default)]
    pub remote_token: Option<String>,
}

/// Result of connecting and mounting a remote daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectRemoteResponse {
    pub success: bool,
    pub host_label: String,
    pub mounted_sessions: Vec<SessionInfo>,
    pub error: Option<String>,
}

/// Request active federated remote host labels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListRemotesRequest {}

/// List of connected federated remote host labels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListRemotesResponse {
    pub remotes: Vec<String>,
}

/// Schedule or cancel a deferred session teardown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeardownRequest {
    pub session_id: String,
    /// One of `archive`, `kill`, or `cancel`.
    pub action: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeardownResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadSessionRequest {
    pub session_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadSessionResponse {
    pub success: bool,
    pub screen: String,
    pub error: Option<String>,
}

/// Persisted organization and visibility for a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMetadata {
    pub project: String,
    /// Slash-separated nested group path.
    pub group: String,
    pub favorite: bool,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkspaceRequest {
    List,
    Set {
        session_id: String,
        metadata: WorkspaceMetadata,
    },
    Fork {
        session_id: String,
        new_session_id: String,
        subtab: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceResponse {
    pub sessions: Vec<SessionInfo>,
    pub metadata: std::collections::HashMap<String, WorkspaceMetadata>,
    pub error: Option<String>,
}

/// A persistent panel registered in a session workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionPanel {
    pub id: i64,
    pub session_id: String,
    pub kind: String,
    pub target: String,
}

/// Authenticated companion operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompanionRequest {
    Open {
        session_id: String,
        target: String,
    },
    Select {
        session_id: String,
    },
    SelectPanel {
        session_id: String,
        panel_id: i64,
    },
    List,
    Reply {
        session_id: String,
        message: String,
    },
    Read {
        panel_id: i64,
    },
    Write {
        panel_id: i64,
        content: String,
    },
    Chat {
        session_id: String,
        operation: String,
        payload: String,
    },
    ChatStates,
    Models {
        cwd: String,
    },
    StartChat {
        session_id: String,
        cwd: String,
        model: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionResponse {
    pub notifications: Vec<NotificationEvent>,
    pub selected_panel: Option<i64>,
    pub panels: Vec<CompanionPanel>,
    pub selected: Option<String>,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// Unified framed IPC message envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum IpcMessage {
    CompanionRequest(CompanionRequest),
    CompanionResponse(CompanionResponse),
    WorkspaceRequest(WorkspaceRequest),
    WorkspaceResponse(WorkspaceResponse),
    ReadSessionRequest(ReadSessionRequest),
    ReadSessionResponse(ReadSessionResponse),
    TeardownRequest(TeardownRequest),
    TeardownResponse(TeardownResponse),
    ConnectRemoteRequest(ConnectRemoteRequest),
    ConnectRemoteResponse(ConnectRemoteResponse),
    ListRemotesRequest(ListRemotesRequest),
    ListRemotesResponse(ListRemotesResponse),
    ResizeSessionRequest(ResizeSessionRequest),
    ResizeSessionResponse(ResizeSessionResponse),
    SpawnSubtabRequest(SpawnSubtabRequest),
    SpawnSubtabResponse(SpawnSubtabResponse),
    InternalHookRequest(InternalHookRequest),
    InternalHookResponse(InternalHookResponse),
    ShutdownDaemonRequest(ShutdownDaemonRequest),
    ShutdownDaemonResponse(ShutdownDaemonResponse),
    ScreenSyncFrame(ScreenSyncFrame),
    NotificationEvent(NotificationEvent),
    SessionListUpdate(SessionListUpdate),
    CreateTaskRequest(CreateTaskRequest),
    CreateTaskResponse(CreateTaskResponse),
    ListTasksRequest(ListTasksRequest),
    ListTasksResponse(ListTasksResponse),
    ClaimTaskRequest(ClaimTaskRequest),
    ClaimTaskResponse(ClaimTaskResponse),
    FinishTaskRequest(FinishTaskRequest),
    FinishTaskResponse(FinishTaskResponse),
    ReleaseTaskRequest(ReleaseTaskRequest),
    ReleaseTaskResponse(ReleaseTaskResponse),
    PostReportRequest(PostReportRequest),
    PostReportResponse(PostReportResponse),
    ViewReportRequest(ViewReportRequest),
    ViewReportResponse(ViewReportResponse),
    WatchEventsRequest(WatchEventsRequest),
    WatchEventFrame(WatchEventFrame),
    AckCursorRequest(AckCursorRequest),
    AckCursorResponse(AckCursorResponse),
    ReserveFilesRequest(ReserveFilesRequest),
    ReserveFilesResponse(ReserveFilesResponse),
    ReleaseFilesRequest(ReleaseFilesRequest),
    ReleaseFilesResponse(ReleaseFilesResponse),
    ListReservationsRequest(ListReservationsRequest),
    ListReservationsResponse(ListReservationsResponse),
    AcquireLockRequest(AcquireLockRequest),
    AcquireLockResponse(AcquireLockResponse),
    ReleaseLockRequest(ReleaseLockRequest),
    ReleaseLockResponse(ReleaseLockResponse),
    ListLocksRequest(ListLocksRequest),
    ListLocksResponse(ListLocksResponse),
    SendMessageRequest(SendMessageRequest),
    SendMessageResponse(SendMessageResponse),
    ResolveReviewCommentRequest(ResolveReviewCommentRequest),
    ResolveReviewCommentResponse(ResolveReviewCommentResponse),
    AddReviewCommentRequest(AddReviewCommentRequest),
    AddReviewCommentResponse(AddReviewCommentResponse),
    ListReviewCommentsRequest(ListReviewCommentsRequest),
    ListReviewCommentsResponse(ListReviewCommentsResponse),
    DispatchReviewRoundRequest(DispatchReviewRoundRequest),
    DispatchReviewRoundResponse(DispatchReviewRoundResponse),
    GetDiffRequest(GetDiffRequest),
    GetDiffResponse(GetDiffResponse),
    RemoveSessionRequest(RemoveSessionRequest),
    RemoveSessionResponse(RemoveSessionResponse),
    FreezeSessionRequest(FreezeSessionRequest),
    FreezeSessionResponse(FreezeSessionResponse),
    ResumeSessionRequest(ResumeSessionRequest),
    ResumeSessionResponse(ResumeSessionResponse),
    SearchLogsRequest(SearchLogsRequest),
    SearchLogsResponse(SearchLogsResponse),
    /// Subscribe to one session viewport, replacing the previous subscription.
    SubscribeViewport {
        session_id: String,
    },
    ListSessionsRequest(ListSessionsRequest),
    ListSessionsResponse(ListSessionsResponse),
    /// Client initiates connection handshake.
    HandshakeRequest(HandshakeRequest),
    /// Daemon accepts or rejects connection handshake.
    HandshakeResponse(HandshakeResponse),
    /// Client requests spawning a command in a PTY.
    SpawnSessionRequest(SpawnSessionRequest),
    /// Daemon reports spawn result.
    SpawnSessionResponse(SpawnSessionResponse),
    /// Daemon streams child PTY stdout/stderr to client.
    TerminalOutputFrame(TerminalOutputFrame),
    /// Client forwards keyboard input to child PTY stdin.
    TerminalInputFrame(TerminalInputFrame),
    /// Daemon signals child process termination.
    SessionExitEvent(SessionExitEvent),
    SessionStateChangedEvent(SessionStateChangedEvent),
    AcknowledgeSessionRequest(AcknowledgeSessionRequest),
    AcknowledgeSessionResponse(AcknowledgeSessionResponse),
    /// Search and scratchpad request.
    SearchNotesRequest(SearchNotesRequest),
    /// Search and scratchpad response.
    SearchNotesResponse(SearchNotesResponse),
}

impl From<HandshakeRequest> for IpcMessage {
    fn from(req: HandshakeRequest) -> Self {
        Self::HandshakeRequest(req)
    }
}

impl From<HandshakeResponse> for IpcMessage {
    fn from(res: HandshakeResponse) -> Self {
        Self::HandshakeResponse(res)
    }
}

impl From<SpawnSessionRequest> for IpcMessage {
    fn from(req: SpawnSessionRequest) -> Self {
        Self::SpawnSessionRequest(req)
    }
}

impl From<SpawnSessionResponse> for IpcMessage {
    fn from(res: SpawnSessionResponse) -> Self {
        Self::SpawnSessionResponse(res)
    }
}

impl From<TerminalOutputFrame> for IpcMessage {
    fn from(frame: TerminalOutputFrame) -> Self {
        Self::TerminalOutputFrame(frame)
    }
}

impl From<TerminalInputFrame> for IpcMessage {
    fn from(frame: TerminalInputFrame) -> Self {
        Self::TerminalInputFrame(frame)
    }
}

impl From<SessionExitEvent> for IpcMessage {
    fn from(event: SessionExitEvent) -> Self {
        Self::SessionExitEvent(event)
    }
}

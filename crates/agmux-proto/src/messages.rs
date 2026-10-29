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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Unified framed IPC message envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum IpcMessage {
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

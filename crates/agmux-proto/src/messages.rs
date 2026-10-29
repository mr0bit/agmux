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

/// Unified framed IPC message envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum IpcMessage {
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

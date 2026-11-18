//! Shared protocol definitions and framed transport for agmux IPC.

/// Capability advertised in the existing Companion List response for safe UI upgrades.
pub const COMPANION_ACP_CAPABILITY: &str = "companion-workflow-v2";

pub mod auth;
pub mod ipc;
pub mod messages;
pub mod transport;

pub use auth::constant_time_eq;
pub use ipc::{IpcClientStream, IpcListener, IpcServerStream, connect_ipc};
pub use messages::{
    AckCursorRequest, AckCursorResponse, AcknowledgeSessionRequest, AcknowledgeSessionResponse,
    AcquireLockRequest, AcquireLockResponse, AddReviewCommentRequest, AddReviewCommentResponse,
    ClaimTaskRequest, ClaimTaskResponse, ConnectRemoteRequest, ConnectRemoteResponse,
    CreateTaskRequest, CreateTaskResponse, DiffHunk, DiffLine, DiffLineType, DiffScope,
    DispatchReviewRoundRequest, DispatchReviewRoundResponse, FileDiff, FileReservationEntry,
    FinishTaskRequest, FinishTaskResponse, FreezeSessionRequest, FreezeSessionResponse,
    GetDiffRequest, GetDiffResponse, HandshakeRequest, HandshakeResponse, InternalHookRequest,
    InternalHookResponse, IpcMessage, JournalEvent, KanbanTask, ListLocksRequest,
    ListLocksResponse, ListRemotesRequest, ListRemotesResponse, ListReservationsRequest,
    ListReservationsResponse, ListReviewCommentsRequest, ListReviewCommentsResponse,
    ListSessionsRequest, ListSessionsResponse, ListTasksRequest, ListTasksResponse, LockEntry,
    NotificationEvent, PostReportRequest, PostReportResponse, ReadSessionRequest,
    ReadSessionResponse, ReleaseFilesRequest, ReleaseFilesResponse, ReleaseLockRequest,
    ReleaseLockResponse, ReleaseTaskRequest, ReleaseTaskResponse, RemoveSessionRequest,
    RemoveSessionResponse, ReserveFilesRequest, ReserveFilesResponse, ResizeSessionRequest,
    ResizeSessionResponse, ResolveReviewCommentRequest, ResolveReviewCommentResponse,
    ResumeSessionRequest, ResumeSessionResponse, ReviewComment, ScreenSyncFrame, SearchLogsRequest,
    SearchLogsResponse, SearchMatch, SendMessageRequest, SendMessageResponse, SessionExitEvent,
    SessionInfo, SessionListUpdate, SessionStateChangedEvent, ShutdownDaemonRequest,
    ShutdownDaemonResponse, SpawnSessionRequest, SpawnSessionResponse, SpawnSubtabRequest,
    SpawnSubtabResponse, TeardownRequest, TeardownResponse, TerminalInputFrame,
    TerminalOutputFrame, ViewReportRequest, ViewReportResponse, WatchEventFrame,
    WatchEventsRequest, WorkerReport,
};
pub use messages::{GlobalSearchHit, SearchNotesRequest, SearchNotesResponse};
pub use transport::{FramedStream, IpcCodec, ProtoError, new_framed};

pub use messages::{WorkspaceMetadata, WorkspaceRequest, WorkspaceResponse};

pub use messages::{CompanionPanel, CompanionRequest, CompanionResponse};

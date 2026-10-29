//! Shared protocol definitions and framed transport for agmux IPC.

pub mod auth;
pub mod messages;
pub mod transport;

pub use auth::constant_time_eq;
pub use messages::{
    AcknowledgeSessionRequest, AcknowledgeSessionResponse, AcquireLockRequest, AcquireLockResponse,
    AddReviewCommentRequest, AddReviewCommentResponse, DiffHunk, DiffLine, DiffLineType, DiffScope,
    DispatchReviewRoundRequest, DispatchReviewRoundResponse, FileDiff, FileReservationEntry,
    FreezeSessionRequest, FreezeSessionResponse, GetDiffRequest, GetDiffResponse, HandshakeRequest,
    HandshakeResponse, IpcMessage, ListLocksRequest, ListLocksResponse, ListReservationsRequest,
    ListReservationsResponse, ListReviewCommentsRequest, ListReviewCommentsResponse,
    ListSessionsRequest, ListSessionsResponse, LockEntry, ReleaseFilesRequest,
    ReleaseFilesResponse, ReleaseLockRequest, ReleaseLockResponse, RemoveSessionRequest,
    RemoveSessionResponse, ReserveFilesRequest, ReserveFilesResponse, ResolveReviewCommentRequest,
    ResolveReviewCommentResponse, ResumeSessionRequest, ResumeSessionResponse, ReviewComment,
    SearchLogsRequest, SearchLogsResponse, SearchMatch, SendMessageRequest, SendMessageResponse,
    SessionExitEvent, SessionInfo, SessionStateChangedEvent, SpawnSessionRequest,
    SpawnSessionResponse, TerminalInputFrame, TerminalOutputFrame,
};
pub use transport::{FramedStream, IpcCodec, ProtoError, new_framed};

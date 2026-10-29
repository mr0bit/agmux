//! Shared protocol definitions and framed transport for agmux IPC.

pub mod auth;
pub mod messages;
pub mod transport;

pub use auth::constant_time_eq;
pub use messages::{
    AcknowledgeSessionRequest, AcknowledgeSessionResponse, AddReviewCommentRequest,
    AddReviewCommentResponse, DiffHunk, DiffLine, DiffLineType, DiffScope,
    DispatchReviewRoundRequest, DispatchReviewRoundResponse, FileDiff, FreezeSessionRequest,
    FreezeSessionResponse, GetDiffRequest, GetDiffResponse, HandshakeRequest, HandshakeResponse,
    IpcMessage, ListReviewCommentsRequest, ListReviewCommentsResponse, ListSessionsRequest,
    ListSessionsResponse, RemoveSessionRequest, RemoveSessionResponse, ResolveReviewCommentRequest,
    ResolveReviewCommentResponse, ResumeSessionRequest, ResumeSessionResponse, ReviewComment,
    SearchLogsRequest, SearchLogsResponse, SearchMatch, SessionExitEvent, SessionInfo,
    SessionStateChangedEvent, SpawnSessionRequest, SpawnSessionResponse, TerminalInputFrame,
    TerminalOutputFrame,
};
pub use transport::{FramedStream, IpcCodec, ProtoError, new_framed};

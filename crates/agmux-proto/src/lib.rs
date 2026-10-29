//! Shared protocol definitions and framed transport for agmux IPC.

pub mod auth;
pub mod messages;
pub mod transport;

pub use auth::constant_time_eq;
pub use messages::{
    DiffHunk, DiffLine, DiffLineType, DiffScope, FileDiff, FreezeSessionRequest,
    FreezeSessionResponse, GetDiffRequest, GetDiffResponse, HandshakeRequest, HandshakeResponse,
    IpcMessage, ListSessionsRequest, ListSessionsResponse, RemoveSessionRequest,
    RemoveSessionResponse, ResumeSessionRequest, ResumeSessionResponse, SearchLogsRequest,
    SearchLogsResponse, SearchMatch, SessionExitEvent, SessionInfo, SessionStateChangedEvent,
    SpawnSessionRequest, SpawnSessionResponse, TerminalInputFrame, TerminalOutputFrame,
};
pub use transport::{FramedStream, IpcCodec, ProtoError, new_framed};

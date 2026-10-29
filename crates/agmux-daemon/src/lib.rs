//! Daemon orchestration and server components.

pub mod process_supervisor;
pub mod server;

pub use server::{DaemonServer, default_socket_path};

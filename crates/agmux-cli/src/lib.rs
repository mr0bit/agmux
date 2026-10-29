//! Stateless CLI client libraries and subcommand routers.

pub mod daemon_spawner;
pub mod ipc_client;

pub use daemon_spawner::ensure_daemon_running;
pub use ipc_client::{freeze_session, resume_session, run_command};

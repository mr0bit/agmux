//! Stateless CLI client libraries and subcommand routers.

pub mod companion_gui;

pub mod agent_context;
pub mod daemon_spawner;
pub mod doctor;
pub mod ipc_client;
pub mod mcp_server;
pub mod remote_bootstrapper;

pub use daemon_spawner::ensure_daemon_running;
pub use ipc_client::{freeze_session, resume_session, run_command};

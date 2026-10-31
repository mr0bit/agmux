//! Daemon orchestration and server components.

pub mod config_watcher;
pub mod event_broadcaster;
pub mod file_reservation_table;
pub mod hook_listener;
pub mod instance_lock;
pub mod journal_watcher;
pub mod kanban_manager;
pub mod lock_manager;
pub mod network_auth;
pub mod notification_dispatcher;
pub mod process_supervisor;
pub mod recovery_manager;
pub mod server;
pub mod shutdown_coordinator;
pub mod ssh_federation;
pub mod subscription_manager;
pub mod teardown_scheduler;

pub use server::{DaemonServer, default_socket_path};

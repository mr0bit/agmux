//! Dynamic state tracking for plain interactive shell sub-tabs (`❯`).

use crate::child_process_monitor::ChildProcessMonitor;
use crate::state_machine::SessionState;

/// Tracks whether a plain shell sub-tab is actively executing a foreground child process (`RUNNING`)
/// or waiting at the interactive command prompt (`IDLE`).
#[derive(Debug, Clone)]
pub struct ShellClassifier {
    shell_pid: u32,
    state: SessionState,
}

impl ShellClassifier {
    /// Create a new `ShellClassifier` monitoring `shell_pid`.
    pub fn new(shell_pid: u32) -> Self {
        Self {
            shell_pid,
            state: SessionState::Idle,
        }
    }

    /// Current classified state of the shell sub-tab.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Poll the shell's child process tree and return `Some((old, new))` if the state changed.
    pub fn poll(&mut self) -> Option<(SessionState, SessionState)> {
        let next = if ChildProcessMonitor::has_active_children(self.shell_pid) {
            SessionState::Running
        } else {
            SessionState::Idle
        };
        if next != self.state {
            let old = self.state;
            self.state = next;
            Some((old, next))
        } else {
            None
        }
    }
}

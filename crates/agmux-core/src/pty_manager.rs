//! Cross-platform PTY allocation and child process lifecycle management.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, watch};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Capacity for output broadcast channel.
pub const OUTPUT_BROADCAST_CAPACITY: usize = 1024;

/// Errors arising during PTY management and process supervision.
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    /// Failed to allocate native PTY pair.
    #[error("Failed to allocate native PTY: {0}")]
    PtyAllocation(String),

    /// Failed to spawn child process inside PTY slave.
    #[error("Failed to spawn command '{command}': {details}")]
    SpawnFailed {
        /// Executable command name.
        command: String,
        /// Underlying error description.
        details: String,
    },

    /// Specified session was not found in active registry.
    #[error("Session '{0}' not found")]
    SessionNotFound(String),

    /// Session with this ID already exists.
    #[error("Session '{0}' already exists")]
    SessionAlreadyExists(String),

    /// Standard I/O write failure when delivering input to PTY.
    #[error("Failed to write to PTY stdin: {0}")]
    InputWrite(#[from] std::io::Error),
}

/// Represents an active PTY session with streaming I/O channels.
pub struct ActivePtySession {
    /// Session identifier.
    pub session_id: String,
    /// Child process identifier.
    pub pid: u32,
    /// Optional working directory for this session.
    pub cwd: Option<PathBuf>,
    /// Optional deterministic `PORT` allocated for this session.
    pub port: Option<u16>,
    /// Thread-safe handle to the PTY master for terminal resize operations.
    master: Arc<std::sync::Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    /// Thread-safe writer connected to PTY master input.
    writer: Arc<std::sync::Mutex<Box<dyn Write + Send>>>,
    /// Broadcast channel for child stdout/stderr output frames.
    output_tx: broadcast::Sender<Vec<u8>>,
    /// Watch channel for process exit status (`None` while running).
    exit_rx: watch::Receiver<Option<i32>>,
    input_tx: broadcast::Sender<()>,
    initial_output: std::sync::Mutex<Option<broadcast::Receiver<Vec<u8>>>>,
    screen_buffer: Arc<std::sync::Mutex<crate::snapshot_compressor::ScreenBuffer>>,
    #[cfg(windows)]
    killer: std::sync::Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
}

impl ActivePtySession {
    /// Resize the underlying PTY master (`TIOCSWINSZ`) and in-memory screen buffer.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        let master = self
            .master
            .lock()
            .map_err(|e| PtyError::PtyAllocation(e.to_string()))?;
        master
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::PtyAllocation(e.to_string()))?;
        drop(master);

        if let Ok(mut sb) = self.screen_buffer.lock() {
            sb.resize(rows.max(1), cols.max(1));
        }
        Ok(())
    }

    /// Send raw keyboard or byte input to the child process stdin.
    pub fn send_input(&self, data: &[u8]) -> Result<(), PtyError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| PtyError::InputWrite(std::io::Error::other(e.to_string())))?;
        writer.write_all(data)?;
        writer.flush()?;
        let _ = self.input_tx.send(());
        Ok(())
    }

    /// Subscribe to real-time output stream of this session.
    pub fn subscribe_output(&self) -> broadcast::Receiver<Vec<u8>> {
        self.initial_output
            .lock()
            .expect("output receiver lock")
            .take()
            .unwrap_or_else(|| self.output_tx.subscribe())
    }

    /// Subscribe to output and capture a screen at the same output boundary.
    pub fn subscribe_viewport(&self) -> (Vec<u8>, broadcast::Receiver<Vec<u8>>) {
        let screen = self.screen_buffer.lock().expect("screen buffer lock");
        let receiver = self.output_tx.subscribe();
        (screen.capture_formatted(), receiver)
    }

    /// Subscribe to output and capture an atomic `ScreenSyncFrame` at the same output boundary.
    pub fn subscribe_viewport_sync(
        &self,
    ) -> (agmux_proto::ScreenSyncFrame, broadcast::Receiver<Vec<u8>>) {
        let screen = self.screen_buffer.lock().expect("screen buffer lock");
        let receiver = self.output_tx.subscribe();
        (screen.capture_sync_frame(&self.session_id), receiver)
    }

    /// Subscribe to delivered input.
    pub fn subscribe_input(&self) -> broadcast::Receiver<()> {
        self.input_tx.subscribe()
    }

    /// Subscribe to the process exit event.
    pub fn subscribe_exit(&self) -> watch::Receiver<Option<i32>> {
        self.exit_rx.clone()
    }

    /// Capture current ANSI screen grid formatting.
    pub fn capture_screen(&self) -> Vec<u8> {
        self.screen_buffer
            .lock()
            .map(|sb| sb.capture_formatted())
            .unwrap_or_default()
    }

    /// Terminate the underlying child process group.
    pub fn kill(&self) {
        #[cfg(windows)]
        if let Ok(mut killer) = self.killer.lock() {
            let _ = killer.kill();
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(self.pid as libc::pid_t, libc::SIGTERM);
        }
    }
}

/// Manager overseeing PTY allocation, process groups, and session indexing.
#[derive(Clone, Default)]
pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, Arc<ActivePtySession>>>>,
}

impl PtyManager {
    /// Create a new `PtyManager`.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a child command inside a newly allocated pseudo-terminal.
    pub async fn spawn(
        &self,
        session_id: String,
        command: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
        port: Option<u16>,
    ) -> Result<Arc<ActivePtySession>, PtyError> {
        self.spawn_with_env(session_id, command, args, cwd, port, Vec::new())
            .await
    }

    /// Spawn a child command inside a newly allocated pseudo-terminal with custom environment variables.
    pub async fn spawn_with_env(
        &self,
        session_id: String,
        command: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
        port: Option<u16>,
        extra_env: Vec<(String, String)>,
    ) -> Result<Arc<ActivePtySession>, PtyError> {
        let mut sessions_lock = self.sessions.lock().await;
        if sessions_lock.contains_key(&session_id) {
            return Err(PtyError::SessionAlreadyExists(session_id));
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::PtyAllocation(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&command);
        cmd.args(&args);
        if let Some(ref dir) = cwd {
            cmd.cwd(dir);
        }

        if let Some(port) = port {
            cmd.env("PORT", port.to_string());
        }
        cmd.env("AGMUX_SESSION_ID", &session_id);
        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed {
                command: command.clone(),
                details: e.to_string(),
            })?;

        let pid = child
            .process_id()
            .ok_or_else(|| PtyError::PtyAllocation("child PID unavailable".into()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::PtyAllocation(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::PtyAllocation(e.to_string()))?;

        let master = Arc::new(std::sync::Mutex::new(pair.master));

        let (output_tx, output_rx) = broadcast::channel(OUTPUT_BROADCAST_CAPACITY);
        let (input_tx, _) = broadcast::channel(64);
        let (exit_tx, exit_rx) = watch::channel(None);

        let screen_buffer = Arc::new(std::sync::Mutex::new(
            crate::snapshot_compressor::ScreenBuffer::new(24, 80),
        ));
        let screen_buf_clone = screen_buffer.clone();

        let output_tx_clone = output_tx.clone();
        let session_id_clone = session_id.clone();

        // Background reader thread: captures raw output from PTY master
        let reader_task = tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF reached when slave closes
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();
                        if let Ok(mut sb) = screen_buf_clone.lock() {
                            sb.process(&chunk);
                            let _ = output_tx_clone.send(chunk);
                        }
                    }
                    Err(e) => {
                        tracing::debug!("PTY reader loop finished for {}: {e}", session_id_clone);
                        break;
                    }
                }
            }
        });

        #[cfg(windows)]
        let killer = std::sync::Mutex::new(child.clone_killer());

        // Background child wait thread: monitors exit code
        let session_id_for_exit = session_id.clone();
        tokio::spawn(async move {
            let exit_code = match tokio::task::spawn_blocking(move || child.wait()).await {
                Ok(Ok(status)) => {
                    let code = status.exit_code() as i32;
                    Some(code)
                }
                other => {
                    tracing::warn!("Failed waiting for child process: {other:?}");
                    None
                }
            };

            let _ = reader_task.await;
            let _ = exit_tx.send(Some(exit_code.unwrap_or(-1)));

            // Retain session in registry until explicitly reaped or let it remain for status inspection
            tracing::info!(
                "Session '{}' terminated with code {:?}",
                session_id_for_exit,
                exit_code
            );
        });

        let session = Arc::new(ActivePtySession {
            session_id: session_id.clone(),
            pid,
            cwd,
            port,
            master,
            writer: Arc::new(std::sync::Mutex::new(writer)),
            output_tx,
            input_tx,
            exit_rx,
            initial_output: std::sync::Mutex::new(Some(output_rx)),
            screen_buffer,
            #[cfg(windows)]
            killer,
        });

        sessions_lock.insert(session_id, session.clone());
        Ok(session)
    }

    /// Retrieve an existing session by ID.
    pub async fn get_session(&self, session_id: &str) -> Result<Arc<ActivePtySession>, PtyError> {
        let lock = self.sessions.lock().await;
        lock.get(session_id)
            .cloned()
            .ok_or_else(|| PtyError::SessionNotFound(session_id.to_string()))
    }

    /// Remove a session from active tracking and return it if present.
    pub async fn remove_session(&self, session_id: &str) -> Option<Arc<ActivePtySession>> {
        let mut lock = self.sessions.lock().await;
        lock.remove(session_id)
    }

    /// Snapshot all currently active `(session_id, pid)` pairs.
    pub async fn active_pids(&self) -> Vec<(String, u32)> {
        let lock = self.sessions.lock().await;
        lock.values()
            .filter(|s| s.subscribe_exit().borrow().is_none())
            .map(|s| (s.session_id.clone(), s.pid))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_spawn_and_output() {
        let manager = PtyManager::new();
        let session = manager
            .spawn(
                "test-1".to_string(),
                "echo".to_string(),
                vec!["HELLO_TEST_PTY".to_string()],
                None,
                None,
            )
            .await
            .expect("spawn session");

        let mut rx = session.subscribe_output();
        let mut exit_rx = session.subscribe_exit();

        let mut output = Vec::new();
        loop {
            tokio::select! {
                Ok(chunk) = rx.recv() => {
                    output.extend_from_slice(&chunk);
                    if output.windows(14).any(|w| w == b"HELLO_TEST_PTY") {
                        break;
                    }
                }
                _ = exit_rx.changed() => {
                    if let Some(_code) = *exit_rx.borrow() {
                        // Even if exited, check if output contains text
                        if output.windows(14).any(|w| w == b"HELLO_TEST_PTY") {
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(3)) => {
                    panic!("Timed out waiting for PTY output. Collected: {:?}", String::from_utf8_lossy(&output));
                }
            }
        }

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("HELLO_TEST_PTY"));
    }
}

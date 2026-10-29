//! Interactive terminal viewport for daemon sessions.

pub mod ui;

use std::{io, path::Path, time::Duration};

use agmux_core::{
    agent_discovery::{DiscoveredAgent, discover_installed_agents},
    config_manager::ensure_config_exists,
};
use agmux_proto::connect_ipc;
use agmux_proto::{
    FreezeSessionRequest, HandshakeRequest, IpcMessage, ListSessionsRequest, SessionInfo,
    SpawnSessionRequest, TerminalInputFrame, new_framed,
};
use anyhow::{Context, Result, bail};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use ratatui::{Terminal, backend::CrosstermBackend};

/// The currently displayed floating input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ModalState {
    #[default]
    None,
    CommandPalette {
        query: String,
        selected: usize,
    },
    OnboardingWizard {
        selected: usize,
    },
    QuickPrompt {
        input: String,
    },
    ReviewDiff {
        session_id: String,
        scope: agmux_proto::DiffScope,
        selected_hunk: usize,
        files: Vec<agmux_proto::FileDiff>,
    },
}

/// State displayed by the interactive viewport.
#[derive(Default)]
pub struct App {
    pub sessions: Vec<SessionInfo>,
    pub selected_index: usize,
    pub active_viewport_lines: Vec<String>,
    pub should_quit: bool,
    pub modal: ModalState,
    pub attention_filter: bool,
    pub split_view: bool,
    pub pinned_session_index: Option<usize>,
    pub pinned_viewport_lines: Vec<String>,
    pub discovered_agents: Vec<DiscoveredAgent>,
    pub config_path: std::path::PathBuf,
    pub onboarding_error: Option<String>,
    pub repository_root: Option<std::path::PathBuf>,
    /// Pending daemon request produced by an input action.
    pub pending_message: Option<IpcMessage>,
}

impl App {
    /// Open onboarding for a cold start with no sessions.
    pub fn open_onboarding(&mut self, config_path: std::path::PathBuf) {
        self.config_path = config_path;
        self.discovered_agents = discover_installed_agents();
        self.repository_root = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                std::path::PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_owned())
            });
        self.modal = ModalState::OnboardingWizard { selected: 0 };
    }

    /// Available launch commands, beginning with the user's shell.
    pub fn onboarding_commands(&self) -> Vec<String> {
        let mut commands = vec![
            std::env::var("SHELL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "bash".into()),
        ];
        commands.extend(
            self.discovered_agents
                .iter()
                .filter(|agent| agent.available)
                .map(|agent| agent.binary_name.clone()),
        );
        commands
    }

    /// Actions matching the palette query, in display order.
    pub fn palette_actions(&self, query: &str) -> Vec<String> {
        let mut actions = vec![
            "New Session".to_owned(),
            "Review Changes (Ctrl+R)".to_owned(),
            "Freeze Session".to_owned(),
            "Toggle Attention Filter".to_owned(),
            "Quit Agmux".to_owned(),
        ];
        actions.extend(
            self.sessions
                .iter()
                .filter(|s| s.state != "DEAD")
                .map(|s| format!("Jump to Session: {}", s.session_id)),
        );
        let query = query.to_lowercase();
        actions
            .into_iter()
            .filter(|action| action.to_lowercase().contains(&query))
            .collect()
    }

    /// Handle a key without modifiers.
    pub fn handle_key(&mut self, key: KeyCode) {
        self.handle_key_with_modifiers(key, KeyModifiers::NONE);
    }

    /// Handle a key and its terminal modifiers.
    pub fn handle_key_with_modifiers(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        let modal = std::mem::take(&mut self.modal);
        self.modal = match modal {
            ModalState::None => {
                match key {
                    KeyCode::F(1) | KeyCode::Char(':') => return self.open_palette(),
                    KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return self.open_palette();
                    }
                    KeyCode::Char('r') if modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(session) = self.sessions.get(self.selected_index) {
                            let session_id = session.session_id.clone();
                            self.pending_message =
                                Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                    session_id: session_id.clone(),
                                    scope: agmux_proto::DiffScope::SessionDiff,
                                    target: None,
                                }));
                            return self.modal = ModalState::ReviewDiff {
                                session_id,
                                scope: agmux_proto::DiffScope::SessionDiff,
                                selected_hunk: 0,
                                files: Vec::new(),
                            };
                        }
                    }
                    KeyCode::Char(' ') => {
                        return self.modal = ModalState::QuickPrompt {
                            input: String::new(),
                        };
                    }
                    KeyCode::Char('w') => {
                        self.attention_filter = !self.attention_filter;
                        if self.attention_filter
                            && self
                                .sessions
                                .get(self.selected_index)
                                .is_some_and(|s| matches!(s.state.as_str(), "IDLE" | "DEAD"))
                        {
                            if let Some(index) = self
                                .sessions
                                .iter()
                                .position(|s| !matches!(s.state.as_str(), "IDLE" | "DEAD"))
                            {
                                self.selected_index = index;
                            }
                        }
                    }
                    KeyCode::Char('.') => {
                        if let Some(session) = self.sessions.get_mut(self.selected_index) {
                            if session.state == "FINISHED" {
                                session.state = "IDLE".into();
                                self.pending_message = Some(IpcMessage::AcknowledgeSessionRequest(
                                    agmux_proto::AcknowledgeSessionRequest {
                                        session_id: session.session_id.clone(),
                                    },
                                ));
                            }
                        }
                    }
                    KeyCode::Char('v') => {
                        self.split_view = !self.split_view;
                        if self.split_view && self.pinned_session_index.is_none() {
                            self.pinned_session_index = Some(self.selected_index);
                            self.pinned_viewport_lines = self.active_viewport_lines.clone();
                        }
                    }
                    KeyCode::Char('m') => {
                        self.pinned_session_index = Some(self.selected_index);
                        self.pinned_viewport_lines = self.active_viewport_lines.clone();
                    }
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
                    KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
                    _ => {}
                }
                ModalState::None
            }
            ModalState::CommandPalette {
                mut query,
                mut selected,
            } => {
                match key {
                    KeyCode::Esc => return,
                    KeyCode::Enter => {
                        if let Some(action) = self.palette_actions(&query).get(selected).cloned() {
                            self.execute_action(&action);
                        }
                        return;
                    }
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        selected = selected
                            .saturating_add(1)
                            .min(self.palette_actions(&query).len().saturating_sub(1))
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        selected = 0;
                    }
                    KeyCode::Char(c)
                        if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        query.push(c);
                        selected = 0;
                    }
                    _ => {}
                }
                ModalState::CommandPalette { query, selected }
            }
            ModalState::OnboardingWizard { mut selected } => {
                match key {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        selected = (selected + 1).min(self.onboarding_commands().len() - 1)
                    }
                    KeyCode::Esc => {
                        self.should_quit = true;
                    }
                    KeyCode::Enter => match ensure_config_exists(&self.config_path) {
                        Ok(_) => {
                            let command = self.onboarding_commands()
                                [selected.min(self.onboarding_commands().len() - 1)]
                            .clone();
                            let id = format!(
                                "session-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                            );
                            self.pending_message =
                                Some(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
                                    session_id: id,
                                    command,
                                    args: vec![],
                                    cwd: self
                                        .repository_root
                                        .as_ref()
                                        .map(|p| p.to_string_lossy().into_owned()),
                                    worktree_branch: None,
                                }));
                            self.onboarding_error = None;
                            return;
                        }
                        Err(error) => self.onboarding_error = Some(error.to_string()),
                    },
                    _ => {}
                }
                ModalState::OnboardingWizard { selected }
            }
            ModalState::QuickPrompt { mut input } => {
                match key {
                    KeyCode::Esc => return,
                    KeyCode::Enter => {
                        if let Some(session) = self.sessions.get(self.selected_index) {
                            input.push('\n');
                            self.pending_message =
                                Some(IpcMessage::TerminalInputFrame(TerminalInputFrame {
                                    session_id: session.session_id.clone(),
                                    data: input.into_bytes(),
                                }));
                        }
                        return;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c)
                        if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        input.push(c)
                    }
                    _ => {}
                }
                ModalState::QuickPrompt { input }
            }
            ModalState::ReviewDiff {
                session_id,
                mut scope,
                mut selected_hunk,
                files,
            } => {
                match key {
                    KeyCode::Esc | KeyCode::Char('q') => return,
                    KeyCode::Tab => {
                        scope = match scope {
                            agmux_proto::DiffScope::SessionDiff => {
                                agmux_proto::DiffScope::FullUncommitted
                            }
                            agmux_proto::DiffScope::FullUncommitted => {
                                agmux_proto::DiffScope::VersusTarget
                            }
                            agmux_proto::DiffScope::VersusTarget => {
                                agmux_proto::DiffScope::LastCommit
                            }
                            agmux_proto::DiffScope::LastCommit => agmux_proto::DiffScope::Staged,
                            agmux_proto::DiffScope::Staged => agmux_proto::DiffScope::SessionDiff,
                        };
                        selected_hunk = 0;
                        self.pending_message =
                            Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                session_id: session_id.clone(),
                                scope,
                                target: None,
                            }));
                    }
                    KeyCode::Char('1') => {
                        scope = agmux_proto::DiffScope::SessionDiff;
                        selected_hunk = 0;
                        self.pending_message =
                            Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                session_id: session_id.clone(),
                                scope,
                                target: None,
                            }));
                    }
                    KeyCode::Char('2') => {
                        scope = agmux_proto::DiffScope::FullUncommitted;
                        selected_hunk = 0;
                        self.pending_message =
                            Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                session_id: session_id.clone(),
                                scope,
                                target: None,
                            }));
                    }
                    KeyCode::Char('3') => {
                        scope = agmux_proto::DiffScope::VersusTarget;
                        selected_hunk = 0;
                        self.pending_message =
                            Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                session_id: session_id.clone(),
                                scope,
                                target: None,
                            }));
                    }
                    KeyCode::Char('4') => {
                        scope = agmux_proto::DiffScope::LastCommit;
                        selected_hunk = 0;
                        self.pending_message =
                            Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                session_id: session_id.clone(),
                                scope,
                                target: None,
                            }));
                    }
                    KeyCode::Char('5') => {
                        scope = agmux_proto::DiffScope::Staged;
                        selected_hunk = 0;
                        self.pending_message =
                            Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                                session_id: session_id.clone(),
                                scope,
                                target: None,
                            }));
                    }
                    KeyCode::Char('C') => {
                        self.pending_message = Some(IpcMessage::DispatchReviewRoundRequest(
                            agmux_proto::DispatchReviewRoundRequest {
                                session_id: session_id.clone(),
                            },
                        ));
                    }
                    KeyCode::Char('c') => {
                        let mut current_idx = 0;
                        for file in &files {
                            let path = file
                                .new_path
                                .clone()
                                .or_else(|| file.old_path.clone())
                                .unwrap_or_else(|| "file".into());
                            for hunk in &file.hunks {
                                if current_idx == selected_hunk {
                                    let line_no = hunk.new_start;
                                    self.pending_message =
                                        Some(IpcMessage::AddReviewCommentRequest(
                                            agmux_proto::AddReviewCommentRequest {
                                                session_id: session_id.clone(),
                                                file_path: path.clone(),
                                                line_number: line_no,
                                                content: "Needs revision".into(),
                                            },
                                        ));
                                    break;
                                }
                                current_idx += 1;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let total_hunks: usize = files.iter().map(|f| f.hunks.len()).sum();
                        if total_hunks > 0 {
                            selected_hunk = (selected_hunk + 1).min(total_hunks - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected_hunk = selected_hunk.saturating_sub(1);
                    }
                    _ => {}
                }
                ModalState::ReviewDiff {
                    session_id,
                    scope,
                    selected_hunk,
                    files,
                }
            }
        };
    }

    fn open_palette(&mut self) {
        self.modal = ModalState::CommandPalette {
            query: String::new(),
            selected: 0,
        };
    }

    fn move_selection(&mut self, forward: bool) {
        let visible: Vec<_> = self
            .sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| !self.attention_filter || !matches!(s.state.as_str(), "IDLE" | "DEAD"))
            .map(|(i, _)| i)
            .collect();
        if visible.is_empty() {
            return;
        }
        let position = visible
            .iter()
            .position(|&i| i == self.selected_index)
            .unwrap_or(0);
        self.selected_index = visible[if forward {
            (position + 1).min(visible.len() - 1)
        } else {
            position.saturating_sub(1)
        }];
    }

    fn execute_action(&mut self, action: &str) {
        match action {
            "Quit Agmux" => self.should_quit = true,
            "Toggle Attention Filter" => {
                self.attention_filter = !self.attention_filter;
                if self.attention_filter
                    && self
                        .sessions
                        .get(self.selected_index)
                        .is_some_and(|s| matches!(s.state.as_str(), "IDLE" | "DEAD"))
                {
                    if let Some(index) = self
                        .sessions
                        .iter()
                        .position(|s| !matches!(s.state.as_str(), "IDLE" | "DEAD"))
                    {
                        self.selected_index = index;
                    }
                }
            }
            "Freeze Session" => {
                if let Some(session) = self.sessions.get(self.selected_index) {
                    self.pending_message =
                        Some(IpcMessage::FreezeSessionRequest(FreezeSessionRequest {
                            session_id: session.session_id.clone(),
                        }));
                }
            }
            "Review Changes (Ctrl+R)" => {
                if let Some(session) = self.sessions.get(self.selected_index) {
                    let session_id = session.session_id.clone();
                    self.pending_message =
                        Some(IpcMessage::GetDiffRequest(agmux_proto::GetDiffRequest {
                            session_id: session_id.clone(),
                            scope: agmux_proto::DiffScope::SessionDiff,
                            target: None,
                        }));
                    self.modal = ModalState::ReviewDiff {
                        session_id,
                        scope: agmux_proto::DiffScope::SessionDiff,
                        selected_hunk: 0,
                        files: Vec::new(),
                    };
                }
            }
            "New Session" => {
                let id = format!(
                    "session-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
                self.pending_message = Some(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
                    session_id: id,
                    command: std::env::var("SHELL").unwrap_or_else(|_| "sh".into()),
                    args: vec![],
                    cwd: None,
                    worktree_branch: None,
                }));
            }
            _ => {
                if let Some(id) = action.strip_prefix("Jump to Session: ") {
                    if let Some(index) = self.sessions.iter().position(|s| s.session_id == id) {
                        self.selected_index = index;
                    }
                }
            }
        }
    }
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Connect to the daemon and run the interactive viewport until quit.
pub async fn run_tui(socket_path: &Path, token: &str) -> Result<()> {
    let stream = connect_ipc(socket_path)
        .await
        .context("Failed to connect to daemon")?;
    let mut ipc = new_framed(stream);
    ipc.send(IpcMessage::HandshakeRequest(HandshakeRequest {
        client_version: env!("CARGO_PKG_VERSION").into(),
        token: token.into(),
    }))
    .await?;
    match ipc
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    ipc.send(IpcMessage::ListSessionsRequest(ListSessionsRequest {}))
        .await?;
    let mut app = loop {
        match ipc
            .next()
            .await
            .context("Connection closed during session list")??
        {
            IpcMessage::ListSessionsResponse(response) => {
                break App {
                    sessions: response.sessions,
                    ..App::default()
                };
            }
            IpcMessage::SessionStateChangedEvent(_) => {}
            other => bail!("Unexpected list response: {other:?}"),
        }
    };
    if app.sessions.is_empty() && !Path::new("agmux.toml").exists() {
        app.open_onboarding("agmux.toml".into());
    }
    if let Some(session) = app.sessions.first() {
        ipc.send(IpcMessage::SubscribeViewport {
            session_id: session.session_id.clone(),
        })
        .await?;
    }
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut parser = vt100::Parser::new(
        std::num::NonZeroU16::new(24).expect("nonzero rows"),
        std::num::NonZeroU16::new(80).expect("nonzero columns"),
        0,
    );
    let mut tick = tokio::time::interval(Duration::from_millis(16));
    loop {
        tokio::select! {
            _ = tick.tick() => {
                terminal.draw(|frame| ui::render(frame, &app))?;
                while event::poll(Duration::ZERO)? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind != KeyEventKind::Press { continue; }
                        let previous = app.selected_index;
                        app.handle_key_with_modifiers(key.code, key.modifiers);
                        if let Some(message) = app.pending_message.take() { ipc.send(message).await?; }
                        if app.should_quit { return Ok(()); }
                        if previous != app.selected_index {
                            parser = vt100::Parser::new(std::num::NonZeroU16::new(24).expect("nonzero rows"), std::num::NonZeroU16::new(80).expect("nonzero columns"), 0);
                            app.active_viewport_lines.clear();
                            if let Some(session) = app.sessions.get(app.selected_index) {
                                ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?;
                            }
                        }
                    }
                }
            }
            message = ipc.next() => match message.context("Daemon connection closed")?? {
                IpcMessage::TerminalOutputFrame(frame) if app.sessions.get(app.selected_index).is_some_and(|s| s.session_id == frame.session_id) => {
                    parser.process(&frame.data);
                    app.active_viewport_lines = parser.screen().contents().lines().map(str::to_owned).collect();
                }
                IpcMessage::SpawnSessionResponse(response) => {
                    if response.success { ipc.send(IpcMessage::SubscribeViewport { session_id: response.session_id }).await?; }
                    ipc.send(IpcMessage::ListSessionsRequest(ListSessionsRequest {})).await?;
                }
                IpcMessage::FreezeSessionResponse(_) => {
                    ipc.send(IpcMessage::ListSessionsRequest(ListSessionsRequest {})).await?;
                }
                IpcMessage::GetDiffResponse(response) => {
                    if let ModalState::ReviewDiff { files, .. } = &mut app.modal {
                        *files = response.files;
                    }
                }
                IpcMessage::ListSessionsResponse(response) => {
                    app.sessions = response.sessions;
                    app.selected_index = app.selected_index.min(app.sessions.len().saturating_sub(1));
                }
                IpcMessage::SessionStateChangedEvent(event) => {
                    if let Some(session) = app.sessions.iter_mut().find(|s| s.session_id == event.session_id) { session.state = event.new_state; }
                }
                _ => {}
            }
        }
    }
}

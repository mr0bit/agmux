//! Interactive terminal viewport for daemon sessions.

pub mod search_modal;
pub mod ui;
pub mod workspace_hierarchy;

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

/// Platform-aware default interactive shell (`powershell.exe` on Windows, `$SHELL` / `bash` on Unix).
pub fn default_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "bash".to_string())
    }
}

/// Alias for `default_shell()`.
pub fn default_shell_command() -> String {
    default_shell()
}

/// Encode a terminal `KeyCode` + `KeyModifiers` into raw PTY stdin bytes for Interactive Mode passthrough.
pub fn encode_key_to_pty_bytes(key: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    match key {
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
            let lower = c.to_ascii_lowercase();
            if lower.is_ascii_lowercase() {
                let ctrl_byte = (lower as u8) - b'a' + 1;
                Some(vec![ctrl_byte])
            } else {
                None
            }
        }
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
        }
        _ => None,
    }
}

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
    GroupEditor {
        input: String,
    },
    ForkMenu,
    ReviewDiff {
        session_id: String,
        scope: agmux_proto::DiffScope,
        selected_hunk: usize,
        files: Vec<agmux_proto::FileDiff>,
    },
    KanbanBoard {
        selected: usize,
    },
    GlobalSearch {
        query: String,
        selected: usize,
        hits: Vec<agmux_proto::GlobalSearchHit>,
        error: Option<String>,
    },
    Filter,
    Notes {
        session_id: String,
        page: u32,
        content: String,
        loaded: bool,
    },
}

/// State displayed by the interactive viewport.
#[derive(Default)]
pub struct App {
    pub sessions: Vec<SessionInfo>,
    pub workspace: std::collections::HashMap<String, agmux_proto::WorkspaceMetadata>,
    pub show_archived: bool,
    pub pending_selection: Option<String>,
    pub prompt_tool: usize,
    pub prompt_model: usize,
    pub prompt_effort: usize,
    pub prompt_worktree: bool,
    pub teardowns: std::collections::HashMap<String, String>,
    pub selected_index: usize,
    pub active_viewport_lines: Vec<String>,
    pub should_quit: bool,
    pub modal: ModalState,
    /// When `true`, keystrokes are forwarded directly to the active session's PTY (`Esc` returns to Normal Mode).
    pub input_mode: bool,
    pub attention_filter: bool,
    /// Display session rows with additional command and directory details.
    pub comfortable_rows: bool,
    pub split_view: bool,
    pub pinned_session_index: Option<usize>,
    pub pinned_viewport_lines: Vec<String>,
    pub subtabs: Vec<String>,
    pub active_subtab_idx: usize,
    pub kanban_tasks: Vec<agmux_proto::KanbanTask>,
    pub discovered_agents: Vec<DiscoveredAgent>,
    pub config_path: std::path::PathBuf,
    pub onboarding_error: Option<String>,
    pub repository_root: Option<std::path::PathBuf>,
    /// Pending daemon request produced by an input action.
    pub pending_message: Option<IpcMessage>,
    pub filter_query: String,
    pub filter_ids: Option<Vec<String>>,
    pub feature_error: Option<String>,
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
        let mut commands = vec![default_shell()];
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
                if key == KeyCode::Char('f') && modifiers.contains(KeyModifiers::CONTROL) {
                    self.input_mode = false;
                    return self.modal = ModalState::GlobalSearch {
                        query: String::new(),
                        selected: 0,
                        hits: Vec::new(),
                        error: None,
                    };
                }
                if (matches!(key, KeyCode::PageUp | KeyCode::PageDown)
                    && modifiers.contains(KeyModifiers::CONTROL))
                    || (!self.input_mode && key == KeyCode::Char('N'))
                {
                    if let Some(session) = self
                        .sessions
                        .get(self.selected_index)
                        .filter(|_| self.session_visible(self.selected_index))
                    {
                        let session_id = session.session_id.clone();
                        let page = 1;
                        self.pending_message = Some(IpcMessage::SearchNotesRequest(
                            agmux_proto::SearchNotesRequest::Get {
                                session_id: session_id.clone(),
                                page,
                            },
                        ));
                        self.input_mode = false;
                        return self.modal = ModalState::Notes {
                            session_id,
                            page,
                            content: String::new(),
                            loaded: false,
                        };
                    }
                }
                if self.input_mode {
                    if key == KeyCode::Esc {
                        self.input_mode = false;
                        return;
                    }
                    if let Some(bytes) = encode_key_to_pty_bytes(key, modifiers)
                        && let Some(session) = self.sessions.get(self.selected_index)
                    {
                        self.pending_message =
                            Some(IpcMessage::TerminalInputFrame(TerminalInputFrame {
                                session_id: session.session_id.clone(),
                                data: bytes,
                            }));
                    }
                    return;
                }

                match key {
                    KeyCode::Char('/') => return self.modal = ModalState::Filter,
                    KeyCode::F(1) | KeyCode::Char(':') => return self.open_palette(),
                    KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                        return self.open_palette();
                    }
                    KeyCode::Char('i') | KeyCode::Enter => {
                        self.input_mode = true;
                        return;
                    }
                    KeyCode::Char('n') | KeyCode::Char('s') => {
                        self.execute_action("New Session");
                        return;
                    }
                    KeyCode::Char('T') => {
                        if let Some(parent) = self.sessions.get(self.selected_index) {
                            let subtab_id =
                                format!("{}:shell-{}", parent.session_id, self.subtabs.len() + 1);
                            self.subtabs.push(subtab_id.clone());
                            self.active_subtab_idx = self.subtabs.len() - 1;
                            self.pending_message = Some(IpcMessage::SpawnSubtabRequest(
                                agmux_proto::SpawnSubtabRequest {
                                    parent_session_id: parent.session_id.clone(),
                                    subtab_id,
                                    shell: Some(default_shell()),
                                },
                            ));
                        }
                    }
                    KeyCode::Char('t') => {
                        self.show_archived = !self.show_archived;
                        self.align_selection();
                    }
                    KeyCode::Char('*') => {
                        let mut meta = self.metadata_for(self.selected_index);
                        meta.favorite = !meta.favorite;
                        self.update_workspace(meta);
                    }
                    KeyCode::Char('a') | KeyCode::Char('u') => {
                        let mut meta = self.metadata_for(self.selected_index);
                        meta.archived = key == KeyCode::Char('a');
                        self.update_workspace(meta);
                    }
                    KeyCode::Char('g') => {
                        let meta = self.metadata_for(self.selected_index);
                        return self.modal = ModalState::GroupEditor {
                            input: format!("{}/{}", meta.project.trim_end_matches('/'), meta.group),
                        };
                    }
                    KeyCode::Char('f') => return self.modal = ModalState::ForkMenu,
                    KeyCode::Char('[') => {
                        if !self.subtabs.is_empty() {
                            self.active_subtab_idx = self.active_subtab_idx.saturating_sub(1);
                            if let Some(i) = self
                                .sessions
                                .iter()
                                .position(|s| s.session_id == self.subtabs[self.active_subtab_idx])
                            {
                                self.selected_index = i;
                            }
                        }
                        return;
                    }
                    KeyCode::Char(']') => {
                        if !self.subtabs.is_empty() {
                            self.active_subtab_idx =
                                (self.active_subtab_idx + 1).min(self.subtabs.len() - 1);
                            if let Some(i) = self
                                .sessions
                                .iter()
                                .position(|s| s.session_id == self.subtabs[self.active_subtab_idx])
                            {
                                self.selected_index = i;
                            }
                        }
                        return;
                    }
                    KeyCode::Char('K') => {
                        self.pending_message = Some(IpcMessage::ListTasksRequest(
                            agmux_proto::ListTasksRequest { status: None },
                        ));
                        return self.modal = ModalState::KanbanBoard { selected: 0 };
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
                        self.align_selection();
                    }
                    KeyCode::Char('D') => self.comfortable_rows = !self.comfortable_rows,
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
                    KeyCode::Char('c') => {
                        if let Some(session) = self.sessions.get(self.selected_index) {
                            self.pending_message =
                                Some(IpcMessage::TeardownRequest(agmux_proto::TeardownRequest {
                                    session_id: session.session_id.clone(),
                                    action: "cancel".into(),
                                }));
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
            ModalState::GlobalSearch {
                mut query,
                mut selected,
                mut hits,
                mut error,
            } => {
                let mut changed = false;
                match key {
                    KeyCode::Esc => return,
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(hits.len().saturating_sub(1)),
                    KeyCode::Backspace => {
                        query.pop();
                        changed = true;
                    }
                    KeyCode::Char(c)
                        if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        query.push(c);
                        changed = true;
                    }
                    _ => {}
                }
                if changed {
                    selected = 0;
                    hits.clear();
                    error = None;
                    if !query.is_empty() {
                        self.pending_message = Some(IpcMessage::SearchNotesRequest(
                            agmux_proto::SearchNotesRequest::Search {
                                query: query.clone(),
                            },
                        ));
                    }
                }
                ModalState::GlobalSearch {
                    query,
                    selected,
                    hits,
                    error,
                }
            }
            ModalState::Filter => {
                match key {
                    KeyCode::Esc => {
                        self.filter_query.clear();
                        self.refresh_filter();
                        return;
                    }
                    KeyCode::Enter => return,
                    KeyCode::Backspace => {
                        self.filter_query.pop();
                        self.refresh_filter();
                    }
                    KeyCode::Char(c)
                        if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        self.filter_query.push(c);
                        self.refresh_filter();
                    }
                    _ => {}
                }
                ModalState::Filter
            }
            ModalState::Notes {
                session_id,
                mut page,
                mut content,
                mut loaded,
            } => {
                let mut changed = false;
                match key {
                    KeyCode::Esc => return,
                    KeyCode::PageUp if modifiers.contains(KeyModifiers::CONTROL) => {
                        page = page.saturating_sub(1).max(1);
                        loaded = false;
                    }
                    KeyCode::PageDown if modifiers.contains(KeyModifiers::CONTROL) => {
                        page = page.saturating_add(1);
                        loaded = false;
                    }
                    KeyCode::F(7) => {
                        page = page.saturating_sub(1).max(1);
                        loaded = false;
                    }
                    KeyCode::F(8) => {
                        page = page.saturating_add(1);
                        loaded = false;
                    }
                    KeyCode::BackTab => {
                        page = page.saturating_sub(1).max(1);
                        loaded = false;
                    }
                    KeyCode::Tab => {
                        page = page.saturating_add(1);
                        loaded = false;
                    }
                    KeyCode::Backspace if loaded => {
                        content.pop();
                        changed = true;
                    }
                    KeyCode::Enter if loaded => {
                        content.push('\n');
                        changed = true;
                    }
                    KeyCode::Char(c)
                        if loaded
                            && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        content.push(c);
                        changed = true;
                    }
                    _ => {}
                }
                if !loaded {
                    content.clear();
                    self.pending_message = Some(IpcMessage::SearchNotesRequest(
                        agmux_proto::SearchNotesRequest::Get {
                            session_id: session_id.clone(),
                            page,
                        },
                    ));
                } else if changed {
                    self.pending_message = Some(IpcMessage::SearchNotesRequest(
                        agmux_proto::SearchNotesRequest::Set {
                            session_id: session_id.clone(),
                            page,
                            content: content.clone(),
                        },
                    ));
                }
                ModalState::Notes {
                    session_id,
                    page,
                    content,
                    loaded,
                }
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
            ModalState::GroupEditor { mut input } => {
                match key {
                    KeyCode::Esc => return,
                    KeyCode::Enter => {
                        // The project is the current project root; the entered path names nested groups.
                        let mut meta = self.metadata_for(self.selected_index);
                        let prefix = format!("{}/", meta.project.trim_end_matches('/'));
                        meta.group = input
                            .strip_prefix(&prefix)
                            .unwrap_or(&input)
                            .trim_matches('/')
                            .to_owned();
                        self.update_workspace(meta);
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
                ModalState::GroupEditor { input }
            }
            ModalState::ForkMenu => {
                if matches!(key, KeyCode::Char('s') | KeyCode::Char('t')) {
                    if let Some(session) = self.sessions.get(self.selected_index) {
                        let subtab = key == KeyCode::Char('t');
                        let suffix = agmux_core::session_tracker::SessionTracker::mint_uuid_v4();
                        let new_session_id = if subtab {
                            format!("{}:fork-{suffix}", session.session_id)
                        } else {
                            format!("fork-{suffix}")
                        };
                        self.pending_selection = Some(new_session_id.clone());
                        self.pending_message = Some(IpcMessage::WorkspaceRequest(
                            agmux_proto::WorkspaceRequest::Fork {
                                session_id: session.session_id.clone(),
                                new_session_id,
                                subtab,
                            },
                        ));
                    }
                    return;
                }
                if key == KeyCode::Esc {
                    return;
                }
                ModalState::ForkMenu
            }
            ModalState::QuickPrompt { mut input } => {
                match key {
                    KeyCode::Esc => return,
                    KeyCode::Tab => {
                        self.prompt_tool = (self.prompt_tool + 1) % 3;
                        if self.prompt_tool == 0 {
                            self.prompt_model = 0;
                            self.prompt_effort = 0;
                            self.prompt_worktree = false;
                        }
                    }
                    KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                        self.prompt_model = (self.prompt_model + 1) % 3;
                        if self.prompt_tool == 0 {
                            self.prompt_tool = 1;
                        }
                    }
                    KeyCode::Char('x') if modifiers.contains(KeyModifiers::CONTROL) => {
                        self.prompt_effort = (self.prompt_effort + 1) % 4;
                        if self.prompt_tool == 0 {
                            self.prompt_tool = 1;
                        }
                    }
                    KeyCode::Char('t') if modifiers.contains(KeyModifiers::CONTROL) => {
                        self.prompt_worktree = !self.prompt_worktree;
                        if self.prompt_tool == 0 {
                            self.prompt_tool = 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(session) = self.sessions.get(self.selected_index) {
                            if self.prompt_tool == 0
                                && self.prompt_model == 0
                                && self.prompt_effort == 0
                                && !self.prompt_worktree
                            {
                                input.push('\n');
                                self.pending_message =
                                    Some(IpcMessage::TerminalInputFrame(TerminalInputFrame {
                                        session_id: session.session_id.clone(),
                                        data: input.into_bytes(),
                                    }));
                            } else {
                                let id = format!(
                                    "prompt-{}",
                                    agmux_core::session_tracker::SessionTracker::mint_uuid_v4()
                                );
                                let mut args = Vec::new();
                                if self.prompt_model != 0 {
                                    args.extend([
                                        "--model".into(),
                                        if self.prompt_tool == 2 {
                                            ["", "sonnet", "opus"][self.prompt_model]
                                        } else {
                                            ["", "gpt-5.4", "gpt-5.3-codex"][self.prompt_model]
                                        }
                                        .into(),
                                    ]);
                                }
                                if self.prompt_effort != 0 {
                                    if self.prompt_tool == 2 {
                                        args.extend([
                                            "--effort".into(),
                                            ["", "low", "medium", "high"][self.prompt_effort]
                                                .into(),
                                        ]);
                                    } else {
                                        args.extend([
                                            "-c".into(),
                                            format!(
                                                "model_reasoning_effort=\"{}\"",
                                                ["", "low", "medium", "high"][self.prompt_effort]
                                            ),
                                        ]);
                                    }
                                }
                                args.push(input);
                                self.pending_selection = Some(id.clone());
                                self.pending_message =
                                    Some(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
                                        session_id: id.clone(),
                                        command: if self.prompt_tool == 2 {
                                            "claude"
                                        } else {
                                            "codex"
                                        }
                                        .into(),
                                        args,
                                        cwd: Some(session.cwd.clone()),
                                        worktree_branch: self.prompt_worktree.then_some(id),
                                    }));
                            }
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
            ModalState::KanbanBoard { mut selected } => {
                match key {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('K') => return,
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.kanban_tasks.is_empty() {
                            selected = (selected + 1).min(self.kanban_tasks.len() - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = selected.saturating_sub(1);
                    }
                    _ => {}
                }
                ModalState::KanbanBoard { selected }
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
        let visible = self.workspace_order();
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
                self.align_selection();
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
                    command: default_shell(),
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
                    teardowns: response.teardowns,
                    ..App::default()
                };
            }
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => {}
            other => bail!("Unexpected list response: {other:?}"),
        }
    };
    ipc.send(IpcMessage::WorkspaceRequest(
        agmux_proto::WorkspaceRequest::List,
    ))
    .await?;
    loop {
        match ipc
            .next()
            .await
            .context("Connection closed during workspace list")??
        {
            IpcMessage::WorkspaceResponse(response) => {
                if let Some(error) = &response.error {
                    bail!("Workspace list failed: {error}");
                }
                app.apply_workspace(response);
                break;
            }
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => {}
            other => bail!("Unexpected workspace response: {other:?}"),
        }
    }
    if app.sessions.is_empty() && !Path::new("agmux.toml").exists() {
        app.open_onboarding("agmux.toml".into());
    }
    if let Some(session) = app.sessions.get(app.selected_index) {
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
                IpcMessage::ScreenSyncFrame(frame) if app.sessions.get(app.selected_index).is_some_and(|s| s.session_id == frame.session_id) => {
                    let rows = std::num::NonZeroU16::new(frame.rows.max(1)).expect("nonzero rows");
                    let cols = std::num::NonZeroU16::new(frame.cols.max(1)).expect("nonzero cols");
                    parser = vt100::Parser::new(rows, cols, 0);
                    parser.process(&frame.formatted_screen);
                    app.active_viewport_lines = frame.lines;
                }
                IpcMessage::TerminalOutputFrame(frame) if app.sessions.get(app.selected_index).is_some_and(|s| s.session_id == frame.session_id) => {
                    parser.process(&frame.data);
                    app.active_viewport_lines = parser.screen().contents().lines().map(str::to_owned).collect();
                }
                IpcMessage::SpawnSessionResponse(response) => {
                    if !response.success { app.feature_error = response.error; app.pending_selection = None; }
                    else if response.session_id.contains(":fork-") { if !app.subtabs.contains(&response.session_id) { app.subtabs.push(response.session_id.clone()); } app.active_subtab_idx = app.subtabs.iter().position(|id| id == &response.session_id).unwrap_or(0); }
                    ipc.send(IpcMessage::ListSessionsRequest(ListSessionsRequest {})).await?;
                }
                IpcMessage::SpawnSubtabResponse(response) => {
                    if !response.success { app.feature_error = response.error.clone(); }
                    if response.success {
                        if !app.subtabs.contains(&response.subtab_id) {
                            app.subtabs.push(response.subtab_id.clone());
                        }
                        ipc.send(IpcMessage::SubscribeViewport { session_id: response.subtab_id }).await?;
                    }
                    ipc.send(IpcMessage::ListSessionsRequest(ListSessionsRequest {})).await?;
                }
                IpcMessage::ListTasksResponse(response) => {
                    app.kanban_tasks = response.tasks;
                }
                IpcMessage::FreezeSessionResponse(_) => {
                    ipc.send(IpcMessage::ListSessionsRequest(ListSessionsRequest {})).await?;
                }
                IpcMessage::GetDiffResponse(response) => {
                    if let ModalState::ReviewDiff { files, .. } = &mut app.modal {
                        *files = response.files;
                    }
                }
                IpcMessage::SearchNotesResponse(response) => {
                    let previous = app.selected_index;
                    app.apply_search_notes_response(response);
                    if previous != app.selected_index {
                        app.active_viewport_lines.clear();
                        parser = vt100::Parser::new(std::num::NonZeroU16::new(24).expect("nonzero rows"), std::num::NonZeroU16::new(80).expect("nonzero columns"), 0);
                        if let Some(session) = app.sessions.get(app.selected_index) {
                            ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?;
                        }
                    }
                }
                IpcMessage::ListSessionsResponse(response) => {
                    app.teardowns = response.teardowns;
                    ipc.send(IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List)).await?;
                }
                IpcMessage::SessionListUpdate(update) => {
                    app.teardowns = update.teardowns;
                    ipc.send(IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List)).await?;
                    if let ModalState::GlobalSearch { query, .. } = &app.modal {
                        if !query.is_empty() { ipc.send(IpcMessage::SearchNotesRequest(agmux_proto::SearchNotesRequest::Search { query: query.clone() })).await?; }
                    }
                }
                IpcMessage::WorkspaceResponse(response) => {
                    let selected = app.sessions.get(app.selected_index).map(|s| s.session_id.clone());
                    if response.error.is_some() {
                        app.pending_selection = None;
                        ipc.send(IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List)).await?;
                    }
                    app.apply_workspace(response);
                    app.refresh_filter();
                    if let Some(message) = app.pending_message.take() { ipc.send(message).await?; }
                    if let Some(id) = app.pending_selection.as_ref() {
                        if let Some(index) = app.sessions.iter().position(|s| &s.session_id == id) {
                            if app.session_visible(index) { app.selected_index = index; }
                            app.pending_selection = None;
                        }
                    }
                    if selected.as_deref() != app.sessions.get(app.selected_index).map(|s| s.session_id.as_str()) {
                        app.active_viewport_lines.clear();
                        parser = vt100::Parser::new(std::num::NonZeroU16::new(24).expect("nonzero rows"), std::num::NonZeroU16::new(80).expect("nonzero columns"), 0);
                        if let Some(session) = app.sessions.get(app.selected_index) { ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?; }
                    }
                }
                IpcMessage::SessionStateChangedEvent(event) => {
                    if let Some(session) = app.sessions.iter_mut().find(|s| s.session_id == event.session_id) { session.state = event.new_state; }
                    let previous = app.selected_index;
                    app.align_selection();
                    if previous != app.selected_index {
                        app.active_viewport_lines.clear();
                        parser = vt100::Parser::new(std::num::NonZeroU16::new(24).expect("nonzero rows"), std::num::NonZeroU16::new(80).expect("nonzero columns"), 0);
                        if let Some(session) = app.sessions.get(app.selected_index) { ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?; }
                    }
                }
                IpcMessage::NotificationEvent(event) if event.kind == "search.index" => {
                    if let ModalState::GlobalSearch { query, .. } = &app.modal {
                        if !query.is_empty() { ipc.send(IpcMessage::SearchNotesRequest(agmux_proto::SearchNotesRequest::Search { query: query.clone() })).await?; }
                    }
                }
                _ => {}
            }
        }
    }
}

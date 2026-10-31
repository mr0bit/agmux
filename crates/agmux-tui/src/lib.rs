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
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::{Color as RatatuiColor, Modifier as RatatuiModifier, Style as RatatuiStyle},
    text::{Line as RatatuiLine, Span as RatatuiSpan},
};

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
    // 1. Check Alt / Meta combinations first
    if modifiers.contains(KeyModifiers::ALT) {
        match key {
            KeyCode::Char(c) => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf).as_bytes();
                let mut bytes = vec![0x1b];
                bytes.extend_from_slice(encoded);
                return Some(bytes);
            }
            KeyCode::Backspace => return Some(vec![0x1b, 0x7f]),
            KeyCode::Left => return Some(b"\x1b[1;3D".to_vec()),
            KeyCode::Right => return Some(b"\x1b[1;3C".to_vec()),
            KeyCode::Up => return Some(b"\x1b[1;3A".to_vec()),
            KeyCode::Down => return Some(b"\x1b[1;3B".to_vec()),
            _ => {}
        }
    }

    // 2. Check Control combinations with arrows or special characters
    if modifiers.contains(KeyModifiers::CONTROL) {
        match key {
            KeyCode::Left => return Some(b"\x1b[1;5D".to_vec()),
            KeyCode::Right => return Some(b"\x1b[1;5C".to_vec()),
            KeyCode::Up => return Some(b"\x1b[1;5A".to_vec()),
            KeyCode::Down => return Some(b"\x1b[1;5B".to_vec()),
            KeyCode::Char(c) => {
                let lower = c.to_ascii_lowercase();
                if lower.is_ascii_lowercase() {
                    let ctrl_byte = (lower as u8) - b'a' + 1;
                    return Some(vec![ctrl_byte]);
                }
                match lower {
                    '@' | ' ' => return Some(vec![0x00]),
                    '[' => return Some(vec![0x1b]),
                    '\\' => return Some(vec![0x1c]),
                    ']' => return Some(vec![0x1d]),
                    '^' => return Some(vec![0x1e]),
                    '_' => return Some(vec![0x1f]),
                    '?' => return Some(vec![0x7f]),
                    _ => return None,
                }
            }
            _ => {}
        }
    }

    // 3. Check Shift combinations with arrows or tab
    if modifiers.contains(KeyModifiers::SHIFT) {
        match key {
            KeyCode::Tab | KeyCode::BackTab => return Some(b"\x1b[Z".to_vec()),
            KeyCode::Left => return Some(b"\x1b[1;2D".to_vec()),
            KeyCode::Right => return Some(b"\x1b[1;2C".to_vec()),
            KeyCode::Up => return Some(b"\x1b[1;2A".to_vec()),
            KeyCode::Down => return Some(b"\x1b[1;2B".to_vec()),
            _ => {}
        }
    }

    // 4. Standard keys
    match key {
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::Insert => Some(b"\x1b[2~".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::F(1) => Some(b"\x1bOP".to_vec()),
        KeyCode::F(2) => Some(b"\x1bOQ".to_vec()),
        KeyCode::F(3) => Some(b"\x1bOR".to_vec()),
        KeyCode::F(4) => Some(b"\x1bOS".to_vec()),
        KeyCode::F(5) => Some(b"\x1b[15~".to_vec()),
        KeyCode::F(6) => Some(b"\x1b[17~".to_vec()),
        KeyCode::F(7) => Some(b"\x1b[18~".to_vec()),
        KeyCode::F(8) => Some(b"\x1b[19~".to_vec()),
        KeyCode::F(9) => Some(b"\x1b[20~".to_vec()),
        KeyCode::F(10) => Some(b"\x1b[21~".to_vec()),
        KeyCode::F(11) => Some(b"\x1b[23~".to_vec()),
        KeyCode::F(12) => Some(b"\x1b[24~".to_vec()),
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
        }
        _ => None,
    }
}

/// Convert a VT100 parsed screen grid into styled Ratatui Lines with 24-bit Truecolor and 256-color fidelity.
pub fn screen_to_ratatui_lines(screen: &vt100::Screen) -> Vec<RatatuiLine<'static>> {
    let (r, c) = screen.size();
    let (rows, cols) = (r.get(), c.get());
    let mut lines = Vec::with_capacity(rows as usize);

    for row in 0..rows {
        let mut spans: Vec<RatatuiSpan<'static>> = Vec::new();
        let mut current_style: Option<RatatuiStyle> = None;
        let mut current_text = String::new();

        for col in 0..cols {
            if let Some(cell) = screen.cell(row, col) {
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut style = RatatuiStyle::default();
                match cell.fgcolor() {
                    vt100::Color::Default => {}
                    vt100::Color::Idx(i) => style = style.fg(RatatuiColor::Indexed(i)),
                    vt100::Color::Rgb(r, g, b) => style = style.fg(RatatuiColor::Rgb(r, g, b)),
                }
                match cell.bgcolor() {
                    vt100::Color::Default => {}
                    vt100::Color::Idx(i) => style = style.bg(RatatuiColor::Indexed(i)),
                    vt100::Color::Rgb(r, g, b) => style = style.bg(RatatuiColor::Rgb(r, g, b)),
                }
                if cell.bold() {
                    style = style.add_modifier(RatatuiModifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(RatatuiModifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(RatatuiModifier::UNDERLINED);
                }
                if cell.inverse() {
                    style = style.add_modifier(RatatuiModifier::REVERSED);
                }

                let text = cell.contents();
                let text_to_append = if text.is_empty() { " " } else { text };

                if let Some(prev_style) = current_style {
                    if prev_style == style {
                        current_text.push_str(text_to_append);
                    } else {
                        if !current_text.is_empty() {
                            spans.push(RatatuiSpan::styled(current_text, prev_style));
                            current_text = String::new();
                        }
                        current_style = Some(style);
                        current_text.push_str(text_to_append);
                    }
                } else {
                    current_style = Some(style);
                    current_text.push_str(text_to_append);
                }
            } else {
                let style = RatatuiStyle::default();
                if let Some(prev_style) = current_style {
                    if prev_style == style {
                        current_text.push(' ');
                    } else {
                        if !current_text.is_empty() {
                            spans.push(RatatuiSpan::styled(current_text, prev_style));
                            current_text = String::new();
                        }
                        current_style = Some(style);
                        current_text.push(' ');
                    }
                } else {
                    current_style = Some(style);
                    current_text.push(' ');
                }
            }
        }

        if let Some(style) = current_style {
            if !current_text.is_empty() {
                spans.push(RatatuiSpan::styled(current_text, style));
            }
        }

        // Trim trailing default-styled whitespace
        while let Some(last_span) = spans.last_mut() {
            if last_span.style == RatatuiStyle::default() {
                let trimmed = last_span.content.trim_end_matches(' ');
                if trimmed.is_empty() {
                    spans.pop();
                } else {
                    if trimmed.len() != last_span.content.len() {
                        last_span.content = trimmed.to_string().into();
                    }
                    break;
                }
            } else {
                break;
            }
        }

        lines.push(RatatuiLine::from(spans));
    }

    lines
}

/// The currently displayed floating input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ModalState {
    #[default]
    None,
    Help,
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
    pub active_viewport_styled: Vec<RatatuiLine<'static>>,
    pub cursor_position: Option<(u16, u16)>,
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
            "Help / Cheatsheet (?)".to_owned(),
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

    /// Returns the session index corresponding to the given vertical offset inside the sidebar list content, if any.
    pub fn session_at_sidebar_row(&self, row_offset: usize) -> Option<usize> {
        let mut current_line = 0;
        let mut previous_favorite = false;
        let mut previous_project = String::new();
        let mut previous_group = String::new();

        for index in self.workspace_order() {
            let meta = self.metadata_for(index);
            if meta.favorite && !previous_favorite {
                current_line += 1;
            }
            if !meta.favorite {
                if previous_favorite || previous_project != meta.project {
                    current_line += 1;
                    previous_project = meta.project.clone();
                    previous_group.clear();
                }
                if previous_group != meta.group {
                    for _ in meta.group.split('/').filter(|s| !s.is_empty()) {
                        current_line += 1;
                    }
                    previous_group = meta.group.clone();
                }
            }
            previous_favorite = meta.favorite;

            let item_height = if self.comfortable_rows { 3 } else { 1 };
            if row_offset >= current_line && row_offset < current_line + item_height {
                return Some(index);
            }
            current_line += item_height;
        }
        None
    }

    /// Handle terminal mouse events (clicks, scrolls) across sidebar, viewport, and modals.
    pub fn handle_mouse(&mut self, mouse: MouseEvent, screen_width: u16, screen_height: u16) {
        let screen = ratatui::layout::Rect::new(0, 0, screen_width, screen_height);
        let (sidebar_area, viewport_area, status_area) = ui::layout_areas(screen);

        let modal = std::mem::take(&mut self.modal);
        self.modal = match modal {
            ModalState::Help => {
                if matches!(mouse.kind, MouseEventKind::Down(_)) {
                    ModalState::None
                } else {
                    ModalState::Help
                }
            }
            ModalState::OnboardingWizard { mut selected } => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        selected = selected.saturating_sub(1);
                    }
                    MouseEventKind::ScrollDown => {
                        selected =
                            (selected + 1).min(self.onboarding_commands().len().saturating_sub(1));
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let modal_area = ui::centered(screen, 70, 22);
                        if mouse.column < modal_area.x
                            || mouse.column >= modal_area.x + modal_area.width
                            || mouse.row < modal_area.y
                            || mouse.row >= modal_area.y + modal_area.height
                        {
                            return;
                        } else {
                            self.modal = ModalState::OnboardingWizard { selected };
                            self.handle_key(KeyCode::Enter);
                            return;
                        }
                    }
                    _ => {}
                }
                ModalState::OnboardingWizard { selected }
            }
            ModalState::CommandPalette {
                query,
                mut selected,
            } => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        selected = selected.saturating_sub(1);
                    }
                    MouseEventKind::ScrollDown => {
                        let actions_len = self.palette_actions(&query).len();
                        selected = (selected + 1).min(actions_len.saturating_sub(1));
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let modal_area = ui::centered(screen, 60, 12);
                        if mouse.column < modal_area.x
                            || mouse.column >= modal_area.x + modal_area.width
                            || mouse.row < modal_area.y
                            || mouse.row >= modal_area.y + modal_area.height
                        {
                            return;
                        } else if mouse.row > modal_area.y + 2 {
                            let click_offset = (mouse.row - (modal_area.y + 2)) as usize;
                            let actions = self.palette_actions(&query);
                            if let Some(action) = actions.get(click_offset).cloned() {
                                self.execute_action(&action);
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                ModalState::CommandPalette { query, selected }
            }
            ModalState::GlobalSearch {
                query,
                mut selected,
                hits,
                error,
            } => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        selected = selected.saturating_sub(1);
                    }
                    MouseEventKind::ScrollDown => {
                        selected = (selected + 1).min(hits.len().saturating_sub(1));
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let modal_area = ui::centered(screen, 90, 80);
                        if mouse.column < modal_area.x
                            || mouse.column >= modal_area.x + modal_area.width
                            || mouse.row < modal_area.y
                            || mouse.row >= modal_area.y + modal_area.height
                        {
                            return;
                        }
                    }
                    _ => {}
                }
                ModalState::GlobalSearch {
                    query,
                    selected,
                    hits,
                    error,
                }
            }
            ModalState::KanbanBoard { mut selected } => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        selected = selected.saturating_sub(1);
                    }
                    MouseEventKind::ScrollDown => {
                        selected = selected.saturating_add(1);
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let modal_area = ui::centered(screen, 90, 80);
                        if mouse.column < modal_area.x
                            || mouse.column >= modal_area.x + modal_area.width
                            || mouse.row < modal_area.y
                            || mouse.row >= modal_area.y + modal_area.height
                        {
                            return;
                        }
                    }
                    _ => {}
                }
                ModalState::KanbanBoard { selected }
            }
            ModalState::ReviewDiff {
                session_id,
                scope,
                mut selected_hunk,
                files,
            } => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        selected_hunk = selected_hunk.saturating_sub(1);
                    }
                    MouseEventKind::ScrollDown => {
                        let total_hunks: usize = files.iter().map(|f| f.hunks.len()).sum();
                        if total_hunks > 0 {
                            selected_hunk = (selected_hunk + 1).min(total_hunks - 1);
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let modal_area = ui::centered(screen, 90, 80);
                        if mouse.column < modal_area.x
                            || mouse.column >= modal_area.x + modal_area.width
                            || mouse.row < modal_area.y
                            || mouse.row >= modal_area.y + modal_area.height
                        {
                            return;
                        }
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
            ModalState::Notes { .. }
            | ModalState::ForkMenu
            | ModalState::GroupEditor { .. }
            | ModalState::QuickPrompt { .. }
            | ModalState::Filter => {
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    let is_outside = mouse.column >= viewport_area.x + viewport_area.width
                        || mouse.row >= status_area.y;
                    if is_outside { ModalState::None } else { modal }
                } else {
                    modal
                }
            }
            ModalState::None => {
                // Check status bar first
                if mouse.row == status_area.y {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        if self.input_mode {
                            self.input_mode = false;
                        } else if mouse.column < 12 {
                            return self.modal = ModalState::Help;
                        } else if mouse.column >= 12 && mouse.column < 32 {
                            self.execute_action("New Session");
                        } else if mouse.column >= screen_width.saturating_sub(12) {
                            self.should_quit = true;
                        }
                    }
                    return;
                }

                // Check sidebar
                if mouse.column >= sidebar_area.x
                    && mouse.column < sidebar_area.x + sidebar_area.width
                    && mouse.row >= sidebar_area.y
                    && mouse.row < sidebar_area.y + sidebar_area.height
                {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            self.move_selection(false);
                        }
                        MouseEventKind::ScrollDown => {
                            self.move_selection(true);
                        }
                        MouseEventKind::Down(MouseButton::Left)
                            if mouse.row > sidebar_area.y
                                && mouse.row < sidebar_area.y + sidebar_area.height - 1 =>
                        {
                            let row_offset = (mouse.row - (sidebar_area.y + 1)) as usize;
                            if let Some(target_index) = self.session_at_sidebar_row(row_offset) {
                                if self.selected_index == target_index {
                                    self.input_mode = true;
                                } else {
                                    self.selected_index = target_index;
                                    self.input_mode = false;
                                }
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                // Check viewport
                if mouse.column >= viewport_area.x
                    && mouse.column < viewport_area.x + viewport_area.width
                    && mouse.row >= viewport_area.y
                    && mouse.row < viewport_area.y + viewport_area.height
                {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            if !self.input_mode && !self.sessions.is_empty() {
                                self.input_mode = true;
                            }
                        }
                        MouseEventKind::ScrollUp if self.split_view => {
                            self.split_view = false;
                        }
                        _ => {}
                    }
                }
                ModalState::None
            }
        };
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
                    KeyCode::Char('?') => return self.modal = ModalState::Help,
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
            ModalState::Help => {
                match key {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                        return;
                    }
                    _ => {}
                }
                ModalState::Help
            }
            ModalState::OnboardingWizard { mut selected } => {
                match key {
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        selected = (selected + 1).min(self.onboarding_commands().len() - 1)
                    }
                    KeyCode::Esc => {
                        return;
                    }
                    KeyCode::Char('q') => {
                        self.should_quit = true;
                        return;
                    }
                    KeyCode::Char('?') => {
                        return self.modal = ModalState::Help;
                    }
                    KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('n') => {
                        match ensure_config_exists(&self.config_path) {
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
                        }
                    }
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
            "Help / Cheatsheet (?)" => {
                self.modal = ModalState::Help;
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
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = disable_raw_mode();
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
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let initial_size = terminal.size()?;
    let mut current_dims = ui::active_viewport_inner_size(
        ratatui::layout::Rect::new(0, 0, initial_size.width, initial_size.height),
        app.split_view,
    );
    let mut parser = vt100::Parser::new(
        std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows"),
        std::num::NonZeroU16::new(current_dims.0).expect("nonzero columns"),
        0,
    );
    if let Some(session) = app.sessions.get(app.selected_index) {
        ipc.send(IpcMessage::ResizeSessionRequest(
            agmux_proto::ResizeSessionRequest {
                session_id: session.session_id.clone(),
                cols: current_dims.0,
                rows: current_dims.1,
            },
        ))
        .await?;
        ipc.send(IpcMessage::SubscribeViewport {
            session_id: session.session_id.clone(),
        })
        .await?;
    }
    let mut tick = tokio::time::interval(Duration::from_millis(16));
    loop {
        tokio::select! {
            _ = tick.tick() => {
                let size = terminal.size()?;
                let new_dims = ui::active_viewport_inner_size(
                    ratatui::layout::Rect::new(0, 0, size.width, size.height),
                    app.split_view,
                );
                if new_dims != current_dims {
                    current_dims = new_dims;
                    let rows = std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows");
                    let cols = std::num::NonZeroU16::new(current_dims.0).expect("nonzero cols");
                    let formatted = parser.screen().contents_formatted();
                    parser = vt100::Parser::new(rows, cols, 0);
                    parser.process(formatted.as_bytes());
                    app.active_viewport_styled = screen_to_ratatui_lines(parser.screen());
                    app.active_viewport_lines = parser.screen().contents().lines().map(str::to_owned).collect();
                    let screen = parser.screen();
                    app.cursor_position = if !screen.hide_cursor() {
                        let (r, c) = screen.cursor_position();
                        Some((c, r))
                    } else {
                        None
                    };
                    if let Some(session) = app.sessions.get(app.selected_index) {
                        ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                            session_id: session.session_id.clone(),
                            cols: current_dims.0,
                            rows: current_dims.1,
                        })).await?;
                    }
                }
                terminal.draw(|frame| ui::render(frame, &app))?;
                while event::poll(Duration::ZERO)? {
                    match event::read()? {
                        Event::Key(key) => {
                            if key.kind != KeyEventKind::Press { continue; }
                            let previous = app.selected_index;
                            app.handle_key_with_modifiers(key.code, key.modifiers);
                            if let Some(message) = app.pending_message.take() { ipc.send(message).await?; }
                            if app.should_quit { return Ok(()); }
                            if previous != app.selected_index {
                                parser = vt100::Parser::new(std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows"), std::num::NonZeroU16::new(current_dims.0).expect("nonzero columns"), 0);
                                app.active_viewport_lines.clear();
                                app.active_viewport_styled.clear();
                                app.cursor_position = None;
                                if let Some(session) = app.sessions.get(app.selected_index) {
                                    ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                                        session_id: session.session_id.clone(),
                                        cols: current_dims.0,
                                        rows: current_dims.1,
                                    })).await?;
                                    ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?;
                                }
                            }
                        }
                        Event::Mouse(mouse) => {
                            let previous = app.selected_index;
                            let size = terminal.size()?;
                            app.handle_mouse(mouse, size.width, size.height);
                            if let Some(message) = app.pending_message.take() { ipc.send(message).await?; }
                            if app.should_quit { return Ok(()); }
                            if previous != app.selected_index {
                                parser = vt100::Parser::new(std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows"), std::num::NonZeroU16::new(current_dims.0).expect("nonzero columns"), 0);
                                app.active_viewport_lines.clear();
                                app.active_viewport_styled.clear();
                                app.cursor_position = None;
                                if let Some(session) = app.sessions.get(app.selected_index) {
                                    ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                                        session_id: session.session_id.clone(),
                                        cols: current_dims.0,
                                        rows: current_dims.1,
                                    })).await?;
                                    ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?;
                                }
                            }
                        }
                        _ => {}
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
                    app.active_viewport_styled = screen_to_ratatui_lines(parser.screen());
                    let screen = parser.screen();
                    app.cursor_position = if !screen.hide_cursor() {
                        let (r, c) = screen.cursor_position();
                        Some((c, r))
                    } else {
                        None
                    };
                }
                IpcMessage::TerminalOutputFrame(frame) if app.sessions.get(app.selected_index).is_some_and(|s| s.session_id == frame.session_id) => {
                    parser.process(&frame.data);
                    app.active_viewport_lines = parser.screen().contents().lines().map(str::to_owned).collect();
                    app.active_viewport_styled = screen_to_ratatui_lines(parser.screen());
                    let screen = parser.screen();
                    app.cursor_position = if !screen.hide_cursor() {
                        let (r, c) = screen.cursor_position();
                        Some((c, r))
                    } else {
                        None
                    };
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
                        ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                            session_id: response.subtab_id.clone(),
                            cols: current_dims.0,
                            rows: current_dims.1,
                        })).await?;
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
                        app.active_viewport_styled.clear();
                        app.cursor_position = None;
                        parser = vt100::Parser::new(std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows"), std::num::NonZeroU16::new(current_dims.0).expect("nonzero columns"), 0);
                        if let Some(session) = app.sessions.get(app.selected_index) {
                            ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                                session_id: session.session_id.clone(),
                                cols: current_dims.0,
                                rows: current_dims.1,
                            })).await?;
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
                        app.active_viewport_styled.clear();
                        app.cursor_position = None;
                        parser = vt100::Parser::new(std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows"), std::num::NonZeroU16::new(current_dims.0).expect("nonzero columns"), 0);
                        if let Some(session) = app.sessions.get(app.selected_index) {
                            ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                                session_id: session.session_id.clone(),
                                cols: current_dims.0,
                                rows: current_dims.1,
                            })).await?;
                            ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?;
                        }
                    }
                }
                IpcMessage::SessionStateChangedEvent(event) => {
                    if let Some(session) = app.sessions.iter_mut().find(|s| s.session_id == event.session_id) { session.state = event.new_state; }
                    let previous = app.selected_index;
                    app.align_selection();
                    if previous != app.selected_index {
                        app.active_viewport_lines.clear();
                        app.active_viewport_styled.clear();
                        app.cursor_position = None;
                        parser = vt100::Parser::new(std::num::NonZeroU16::new(current_dims.1).expect("nonzero rows"), std::num::NonZeroU16::new(current_dims.0).expect("nonzero columns"), 0);
                        if let Some(session) = app.sessions.get(app.selected_index) {
                            ipc.send(IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
                                session_id: session.session_id.clone(),
                                cols: current_dims.0,
                                rows: current_dims.1,
                            })).await?;
                            ipc.send(IpcMessage::SubscribeViewport { session_id: session.session_id.clone() }).await?;
                        }
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

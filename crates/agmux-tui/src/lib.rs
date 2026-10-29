//! Interactive terminal viewport for daemon sessions.

pub mod ui;

use std::{io, path::Path, time::Duration};

use agmux_proto::{HandshakeRequest, IpcMessage, ListSessionsRequest, SessionInfo, new_framed};
use anyhow::{Context, Result, bail};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::net::UnixStream;

/// State displayed by the interactive viewport.
#[derive(Default)]
pub struct App {
    pub sessions: Vec<SessionInfo>,
    pub selected_index: usize,
    pub active_viewport_lines: Vec<String>,
    pub should_quit: bool,
}

impl App {
    /// Handle a navigation key, keeping selection within the session list.
    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index = self
                    .selected_index
                    .saturating_add(1)
                    .min(self.sessions.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_index = self.selected_index.saturating_sub(1)
            }
            _ => {}
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
    let stream = UnixStream::connect(socket_path)
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
                        app.handle_key(key.code);
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
                IpcMessage::SessionStateChangedEvent(event) => {
                    if let Some(session) = app.sessions.iter_mut().find(|s| s.session_id == event.session_id) { session.state = event.new_state; }
                }
                _ => {}
            }
        }
    }
}

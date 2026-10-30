//! End-to-end verification tests for the Full Interactive TUI Multiplexer (`TASK-28`).
//!
//! Verifies:
//! 1. Interactive PTY Keyboard Passthrough Mode (`i` / `Enter` -> `input_mode`, `Esc` -> Normal Mode)
//! 2. Platform-Aware Default Shell & One-Key Session Spawn (`n`)
//! 3. Rich Empty-State Welcome & Agent Launchpad Dashboard
//! 4. Interactive Shell Sub-Tab Spawning (`t`) and Cycling (`[` / `]`)
//! 5. Shared Kanban Board Overlay Modal (`K`)

use agmux_proto::{IpcMessage, KanbanTask, SessionInfo};
use agmux_tui::{App, ModalState, default_shell, encode_key_to_pty_bytes, ui};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

fn render_to_string(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 28)).expect("test terminal");
    terminal
        .draw(|frame| ui::render(frame, app))
        .expect("draw frame");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_interactive_pty_keyboard_passthrough_and_esc_return() {
    let mut app = App {
        sessions: vec![SessionInfo {
            session_id: "agent-alpha".to_string(),
            pid: 1001,
            cwd: "/workspace".to_string(),
            command: "claude".to_string(),
            created_at: 1,
            state: "RUNNING".to_string(),
        }],
        active_viewport_lines: vec!["claude> ".to_string()],
        ..App::default()
    };

    assert!(!app.input_mode);

    // Press `i` in Normal Mode -> enters Interactive Mode
    app.handle_key(KeyCode::Char('i'));
    assert!(app.input_mode);
    let rendered = render_to_string(&app);
    assert!(rendered.contains("agent-alpha [INTERACTIVE - Esc to exit]"));
    assert!(rendered.contains("[INTERACTIVE]"));

    // Type 'a' and Enter -> forwarded as `TerminalInputFrame` bytes to `agent-alpha`
    app.handle_key(KeyCode::Char('a'));
    assert_eq!(
        app.pending_message.take(),
        Some(IpcMessage::TerminalInputFrame(
            agmux_proto::TerminalInputFrame {
                session_id: "agent-alpha".to_string(),
                data: b"a".to_vec(),
            }
        ))
    );

    app.handle_key(KeyCode::Enter);
    assert_eq!(
        app.pending_message.take(),
        Some(IpcMessage::TerminalInputFrame(
            agmux_proto::TerminalInputFrame {
                session_id: "agent-alpha".to_string(),
                data: b"\r".to_vec(),
            }
        ))
    );

    // Verify Ctrl+C encoding
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Some(vec![3])
    );

    // Press `Esc` -> exits Interactive Mode back to Normal Mode without quitting app
    app.handle_key(KeyCode::Esc);
    assert!(!app.input_mode);
    assert!(!app.should_quit);
}

#[test]
fn test_platform_aware_default_shell_and_one_key_spawn() {
    let shell = default_shell();
    assert!(!shell.trim().is_empty());

    let mut app = App::default();
    app.handle_key(KeyCode::Char('n'));
    match app.pending_message.take() {
        Some(IpcMessage::SpawnSessionRequest(req)) => {
            assert_eq!(req.command, shell);
            assert!(req.session_id.starts_with("session-"));
        }
        other => panic!("Expected SpawnSessionRequest on 'n' key, got {other:?}"),
    }
}

#[test]
fn test_empty_state_welcome_and_agent_launchpad_rendering() {
    let app = App::default();
    let rendered = render_to_string(&app);

    assert!(rendered.contains("Welcome to Agmux — Agentic Terminal Multiplexer & Orchestrator"));
    assert!(rendered.contains("Quick-Start Hotkeys:"));
    assert!(rendered.contains("14 Supported Agent Profiles:"));
    assert!(rendered.contains("claude, agy, gemini, codex"));
    // Guardrail: status bar must never contain literal "New Session"
    assert!(!rendered.contains("New Session"));
}

#[test]
fn test_subtab_spawning_and_cycling() {
    let mut app = App {
        sessions: vec![SessionInfo {
            session_id: "worker-1".to_string(),
            pid: 2002,
            cwd: "/workspace".to_string(),
            command: "codex".to_string(),
            created_at: 1,
            state: "RUNNING".to_string(),
        }],
        ..App::default()
    };

    // Press 't' -> spawns first shell sub-tab
    app.handle_key(KeyCode::Char('t'));
    assert_eq!(app.subtabs, vec!["worker-1:shell-1".to_string()]);
    assert_eq!(app.active_subtab_idx, 0);
    match app.pending_message.take() {
        Some(IpcMessage::SpawnSubtabRequest(req)) => {
            assert_eq!(req.parent_session_id, "worker-1");
            assert_eq!(req.subtab_id, "worker-1:shell-1");
        }
        other => panic!("Expected SpawnSubtabRequest, got {other:?}"),
    }

    // Press 't' again -> spawns second shell sub-tab
    app.handle_key(KeyCode::Char('t'));
    assert_eq!(app.subtabs.len(), 2);
    assert_eq!(app.active_subtab_idx, 1);

    // Cycle subtabs with '[' and ']'
    app.handle_key(KeyCode::Char('['));
    assert_eq!(app.active_subtab_idx, 0);
    app.handle_key(KeyCode::Char(']'));
    assert_eq!(app.active_subtab_idx, 1);

    let rendered = render_to_string(&app);
    assert!(rendered.contains("[❯ worker-1:shell-2]"));
}

#[test]
fn test_kanban_board_overlay_modal() {
    let mut app = App {
        kanban_tasks: vec![
            KanbanTask {
                id: "T-301".to_string(),
                title: "Implement PTY Passthrough".to_string(),
                description: String::new(),
                status: "in_progress".to_string(),
                claimed_by: Some("agent-1".to_string()),
                created_at: 1,
                updated_at: 2,
            },
            KanbanTask {
                id: "T-302".to_string(),
                title: "Verify SHA-256 Bootstrapper".to_string(),
                description: String::new(),
                status: "todo".to_string(),
                claimed_by: None,
                created_at: 1,
                updated_at: 1,
            },
        ],
        ..App::default()
    };

    // Press 'K' -> opens KanbanBoard modal and dispatches ListTasksRequest
    app.handle_key(KeyCode::Char('K'));
    assert_eq!(app.modal, ModalState::KanbanBoard { selected: 0 });
    assert_eq!(
        app.pending_message.take(),
        Some(IpcMessage::ListTasksRequest(
            agmux_proto::ListTasksRequest { status: None }
        ))
    );

    let rendered = render_to_string(&app);
    assert!(rendered.contains("Kanban Task Board (K)"));
    assert!(rendered.contains("T-301"));
    assert!(rendered.contains("Implement PTY Passthrough"));

    // Navigate down and close with Esc
    app.handle_key(KeyCode::Down);
    assert_eq!(app.modal, ModalState::KanbanBoard { selected: 1 });
    app.handle_key(KeyCode::Esc);
    assert_eq!(app.modal, ModalState::None);
}

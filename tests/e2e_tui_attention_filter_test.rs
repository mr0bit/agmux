use agmux_proto::{IpcMessage, SessionInfo};
use agmux_tui::{App, ui};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| ui::render(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn test_attention_filter_and_session_acknowledgment() {
    let mut app = App {
        sessions: vec![
            SessionInfo {
                session_id: "idle-worker".into(),
                pid: 101,
                cwd: "/".into(),
                command: "sh".into(),
                created_at: 0,
                state: "IDLE".into(),
            },
            SessionInfo {
                session_id: "waiting-agent".into(),
                pid: 102,
                cwd: "/".into(),
                command: "codex".into(),
                created_at: 0,
                state: "WAITING".into(),
            },
            SessionInfo {
                session_id: "finished-task".into(),
                pid: 103,
                cwd: "/".into(),
                command: "agy".into(),
                created_at: 0,
                state: "FINISHED".into(),
            },
            SessionInfo {
                session_id: "dead-proc".into(),
                pid: 104,
                cwd: "/".into(),
                command: "sh".into(),
                created_at: 0,
                state: "DEAD".into(),
            },
        ],
        selected_index: 0,
        ..App::default()
    };

    let full_tree = screen(&app);
    assert!(full_tree.contains("idle-worker"));
    assert!(full_tree.contains("waiting-agent"));
    assert!(full_tree.contains("finished-task"));
    assert!(full_tree.contains("dead-proc"));

    // Press 'w' to activate attention filter
    app.handle_key(KeyCode::Char('w'));
    assert!(app.attention_filter);

    let filtered = screen(&app);
    assert!(filtered.contains("ATTENTION FILTER"));
    assert!(filtered.contains("waiting-agent"));
    assert!(filtered.contains("finished-task"));
    assert!(!filtered.contains("idle-worker"));
    assert!(!filtered.contains("dead-proc"));

    // Navigate to 'finished-task' and press '.' to acknowledge
    app.handle_key(KeyCode::Down);
    assert_eq!(app.selected_index, 2);
    app.handle_key(KeyCode::Char('.'));

    assert_eq!(app.sessions[2].state, "IDLE");
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::AcknowledgeSessionRequest(req)) if req.session_id == "finished-task"
    ));

    // Press 'w' again to toggle off attention filter
    app.handle_key(KeyCode::Char('w'));
    assert!(!app.attention_filter);
    let restored = screen(&app);
    assert!(restored.contains("idle-worker"));
    assert!(!restored.contains("ATTENTION FILTER"));
}

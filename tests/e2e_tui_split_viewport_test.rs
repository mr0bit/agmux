use agmux_proto::SessionInfo;
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
fn test_split_viewport_pin_and_compare() {
    let mut app = App {
        sessions: vec![
            SessionInfo {
                session_id: "agent-alpha".into(),
                pid: 201,
                cwd: "/".into(),
                command: "codex".into(),
                created_at: 0,
                state: "RUNNING".into(),
            },
            SessionInfo {
                session_id: "agent-beta".into(),
                pid: 202,
                cwd: "/".into(),
                command: "agy".into(),
                created_at: 0,
                state: "RUNNING".into(),
            },
        ],
        selected_index: 0,
        active_viewport_lines: vec!["Output from Agent Alpha (baseline)".into()],
        ..App::default()
    };

    // Press 'v' to activate split viewport
    app.handle_key(KeyCode::Char('v'));
    assert!(app.split_view);

    // Press 'm' to pin Agent Alpha
    app.handle_key(KeyCode::Char('m'));
    assert_eq!(app.pinned_session_index, Some(0));
    assert_eq!(
        app.pinned_viewport_lines,
        vec!["Output from Agent Alpha (baseline)"]
    );

    // Navigate to Agent Beta in the sidebar and update active viewport lines
    app.handle_key(KeyCode::Down);
    assert_eq!(app.selected_index, 1);
    app.active_viewport_lines = vec!["Output from Agent Beta (candidate)".into()];

    let rendered = screen(&app);
    // Left pane has pinned Alpha
    assert!(rendered.contains("agent-alpha [PINNED]"));
    assert!(rendered.contains("Output from Agent Alpha (baseline)"));
    // Right pane has live Beta
    assert!(rendered.contains("agent-beta [LIVE]"));
    assert!(rendered.contains("Output from Agent Beta (candidate)"));

    // Press 'v' to close split viewport
    app.handle_key(KeyCode::Char('v'));
    assert!(!app.split_view);
    let single = screen(&app);
    assert!(!single.contains("[PINNED]"));
}

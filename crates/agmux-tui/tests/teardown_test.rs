use agmux_proto::{IpcMessage, SessionInfo};
use agmux_tui::App;
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn badge_and_cancel_use_selected_session() {
    let mut app = App::default();
    app.sessions.push(SessionInfo {
        session_id: "worker".into(),
        pid: 1,
        cwd: "/".into(),
        command: "agent".into(),
        created_at: 0,
        state: "RUNNING".into(),
    });
    let mut terminal = Terminal::new(TestBackend::new(100, 20)).unwrap();
    for (action, badge) in [("kill", "■"), ("archive", "↓")] {
        app.teardowns.insert("worker".into(), action.into());
        terminal
            .draw(|frame| agmux_tui::ui::render(frame, &app))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains(badge));
    }
    app.handle_key(KeyCode::Char('c'));
    assert!(
        matches!(app.pending_message, Some(IpcMessage::TeardownRequest(req)) if req.session_id == "worker" && req.action == "cancel")
    );
}

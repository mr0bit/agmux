use agmux_proto::{SessionInfo, WorkspaceMetadata};
use agmux_tui::{App, ui};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};
fn session(id: &str) -> SessionInfo {
    SessionInfo {
        session_id: id.into(),
        pid: 5,
        cwd: "/project".into(),
        command: "sh".into(),
        created_at: 0,
        state: "RUNNING".into(),
    }
}
fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    terminal.draw(|f| ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}
#[test]
fn favorites_nested_groups_density_and_selection_order() {
    let mut app = App {
        sessions: vec![session("z"), session("a"), session("b")],
        ..App::default()
    };
    app.workspace.insert(
        "z".into(),
        WorkspaceMetadata {
            project: "/project".into(),
            group: "team/api".into(),
            ..Default::default()
        },
    );
    app.workspace.insert(
        "b".into(),
        WorkspaceMetadata {
            project: "/project".into(),
            group: "team/ui".into(),
            favorite: true,
            archived: false,
        },
    );
    let text = screen(&app);
    assert!(text.contains("Favorites"));
    assert!(text.contains("team"));
    assert!(text.contains("api"));
    app.selected_index = 2;
    app.handle_key(KeyCode::Down);
    assert_eq!(app.sessions[app.selected_index].session_id, "a");
    app.handle_key(KeyCode::Down);
    assert_eq!(app.sessions[app.selected_index].session_id, "z");
    app.handle_key(KeyCode::Char('D'));
    let text = screen(&app);
    assert!(text.contains("/project"));
    assert!(text.contains("sh"));
    app.workspace.get_mut("b").unwrap().favorite = false;
    assert!(!screen(&app).contains("Favorites"));
}

use agmux_proto::SessionInfo;
use agmux_tui::{App, ui};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn renders_sessions_and_viewport_and_navigates() {
    let session = |id: &str, cmd: &str, state: &str| SessionInfo {
        session_id: id.into(),
        pid: 1420,
        cwd: "/home/user/project".into(),
        command: cmd.into(),
        created_at: 0,
        state: state.into(),
    };
    let mut app = App {
        sessions: vec![
            session("agent-alpha", "cargo test", "RUNNING"),
            session("agent-beta", "claude --model sonnet", "WAITING"),
            session("agent-gamma", "npm run build", "STUCK"),
            session("agent-delta", "git push origin main", "DEAD"),
        ],
        active_viewport_lines: vec![
            "   Compiling agmux-core v0.1.0 (/media/inegm/Data/p/agmux/crates/agmux-core)".into(),
            "   Compiling agmux-daemon v0.1.0 (/media/inegm/Data/p/agmux/crates/agmux-daemon)"
                .into(),
            "   Compiling agmux-tui v0.1.0 (/media/inegm/Data/p/agmux/crates/agmux-tui)".into(),
            "    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.18s".into(),
            "     Running tests/e2e_tui_render_test.rs".into(),
            "".into(),
            "running 1 test".into(),
            "test renders_sessions_and_viewport_and_navigates ... ok".into(),
            "".into(),
            "test result: ok. 1 passed; 0 failed; 0 ignored; finished in 0.01s".into(),
        ],
        ..App::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(120, 20)).unwrap();
    terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if std::env::var("PRINT_TUI").is_ok() {
        println!("{text}");
    }
    assert!(text.contains("agent-alpha"));
    assert!(text.contains("[RUNNING]"));
    assert!(text.contains("test result: ok"));
    app.handle_key(KeyCode::Down);
    assert_eq!(app.selected_index, 1);
}

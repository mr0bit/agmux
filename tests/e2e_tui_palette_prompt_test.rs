use agmux_proto::{IpcMessage, SessionInfo};
use agmux_tui::{App, ModalState, ui};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
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
fn palette_filters_and_executes_and_prompt_submits() {
    let mut app = App {
        sessions: vec![SessionInfo {
            session_id: "alpha".into(),
            pid: 1,
            cwd: "/".into(),
            command: "sh".into(),
            created_at: 0,
            state: "RUNNING".into(),
        }],
        ..App::default()
    };
    for key in [KeyCode::F(1), KeyCode::Char(':')] {
        app.handle_key(key);
        let rendered = screen(&app);
        assert!(rendered.contains("Command Palette"));
        assert!(rendered.contains("New Session"));
        assert!(rendered.contains("Jump to Session: alpha"));
        app.handle_key(KeyCode::Esc);
    }
    app.handle_key(KeyCode::F(1));
    for c in "freeze".chars() {
        app.handle_key(KeyCode::Char(c));
    }
    let rendered = screen(&app);
    assert!(rendered.contains("Freeze Session"));
    assert!(!rendered.contains("New Session"));
    app.handle_key(KeyCode::Enter);
    assert!(matches!(
        app.pending_message.take(),
        Some(IpcMessage::FreezeSessionRequest(_))
    ));
    app.handle_key(KeyCode::Char(' '));
    assert!(screen(&app).contains("Quick Prompt"));
    for c in "hello".chars() {
        app.handle_key(KeyCode::Char(c));
    }
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.modal, ModalState::None);
    match app.pending_message.take().unwrap() {
        IpcMessage::TerminalInputFrame(frame) => {
            assert_eq!(frame.session_id, "alpha");
            assert_eq!(frame.data, b"hello\n");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn test_render_command_palette_modal() {
    let mut app = App {
        sessions: vec![
            SessionInfo {
                session_id: "agent-builder".into(),
                pid: 1420,
                cwd: "/home/user/project".into(),
                command: "cargo check".into(),
                created_at: 0,
                state: "RUNNING".into(),
            },
            SessionInfo {
                session_id: "agent-reviewer".into(),
                pid: 1425,
                cwd: "/home/user/project".into(),
                command: "codex-delegate".into(),
                created_at: 0,
                state: "WAITING".into(),
            },
        ],
        active_viewport_lines: vec![
            "   Compiling agmux-tui v0.1.0".into(),
            "   Compiling agmux-cli v0.1.0".into(),
            "    Finished `dev` profile in 1.45s".into(),
        ],
        ..App::default()
    };
    app.handle_key(KeyCode::F(1));
    for c in "ses".chars() {
        app.handle_key(KeyCode::Char(c));
    }
    let mut terminal = Terminal::new(TestBackend::new(105, 20)).unwrap();
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
    if std::env::var("PRINT_PALETTE").is_ok() {
        println!("{text}");
    }
    assert!(text.contains("Command Palette"));
}

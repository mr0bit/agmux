use agmux_core::{
    agent_discovery::{DiscoveredAgent, discover_installed_agents},
    config_manager::generate_default_config,
};
use agmux_proto::IpcMessage;
use agmux_tui::{App, ModalState, ui};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::TestBackend};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn discovery_and_config_are_valid() {
    let agents = discover_installed_agents();
    assert_eq!(agents.len(), 6);
    assert!(agents.iter().all(|a| !a.name.is_empty()
        && !a.binary_name.is_empty()
        && a.available == a.path.is_some()));
    let config = generate_default_config(&agents);
    let mut parser = Command::new("python3")
        .args(["-c", "import sys,tomllib; tomllib.loads(sys.stdin.read())"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("python3 with tomllib required");
    parser
        .stdin
        .take()
        .unwrap()
        .write_all(config.as_bytes())
        .unwrap();
    assert!(parser.wait().unwrap().success());
}

#[test]
fn wizard_renders_and_launches_selected_agent() {
    let dir = std::env::temp_dir().join(format!("agmux-wizard-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("agmux.toml");
    let mut app = App::default();
    app.open_onboarding(path.clone());
    app.discovered_agents = vec![
        DiscoveredAgent {
            name: "Codex".into(),
            binary_name: "codex".into(),
            path: Some("/bin/codex".into()),
            available: true,
        },
        DiscoveredAgent {
            name: "Claude".into(),
            binary_name: "claude".into(),
            path: None,
            available: false,
        },
    ];
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    for expected in [
        "Welcome to Agmux",
        "Detected AI Agents in PATH:",
        "[INSTALLED] Codex",
        "[NOT FOUND] Claude",
        "Select Default Environment to Launch:",
        "[Enter] Launch",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    app.handle_key(KeyCode::Down);
    app.handle_key(KeyCode::Enter);
    assert!(path.is_file());
    assert_eq!(app.modal, ModalState::None);
    assert!(
        matches!(app.pending_message, Some(IpcMessage::SpawnSessionRequest(ref req)) if req.command == "codex")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

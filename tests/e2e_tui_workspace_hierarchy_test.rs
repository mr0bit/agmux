//! TEST-VER-TUI-009: workspace organization through daemon IPC and TUI controls.
#![cfg(unix)]
use agmux_proto::{
    IpcMessage, SessionInfo, WorkspaceMetadata, WorkspaceRequest, WorkspaceResponse,
};
use agmux_tui::{App, ModalState};
use crossterm::event::{KeyCode, KeyModifiers};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Daemon {
    root: PathBuf,
    child: Child,
}
impl Daemon {
    fn start(root: PathBuf) -> Self {
        let child = Command::new("target/debug/agmuxd")
            .env("AGMUX_SOCK", root.join("socket"))
            .env("AGMUX_AUTH_TOKEN_PATH", root.join("token"))
            .env("AGMUX_DB_PATH", root.join("state.db"))
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    root.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        for _ in 0..200 {
            if root.join("socket").exists() {
                return Self { root, child };
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("daemon did not start")
    }
    async fn request(&self, request: WorkspaceRequest) -> IpcMessage {
        let token = agmux_core::get_or_create_token(&self.root.join("token")).unwrap();
        agmux_cli::ipc_client::review_request(
            &self.root.join("socket"),
            &token,
            IpcMessage::WorkspaceRequest(request),
        )
        .await
        .unwrap()
    }
    async fn snapshot(&self) -> WorkspaceResponse {
        match self.request(WorkspaceRequest::List).await {
            IpcMessage::WorkspaceResponse(r) => r,
            x => panic!("{x:?}"),
        }
    }
}
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(self.root.join("socket"));
    }
}
fn session(id: &str, cwd: &str, command: &str) -> SessionInfo {
    SessionInfo {
        session_id: id.into(),
        pid: 0,
        cwd: cwd.into(),
        command: command.into(),
        created_at: 1,
        state: "DEAD".into(),
    }
}
#[tokio::test]
async fn hierarchy_persists_and_commands_cross_ipc() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-31-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "initial"
            ])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let root_str = root.to_str().unwrap();
    use std::os::unix::fs::PermissionsExt;
    let fake = root.join("claude");
    let output = root.join("fork-args");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\n",
            output.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::copy(&fake, root.join("codex")).unwrap();
    std::fs::set_permissions(root.join("codex"), std::fs::Permissions::from_mode(0o755)).unwrap();
    {
        let db = agmux_core::storage_engine::StorageEngine::open(&root.join("state.db")).unwrap();
        db.insert_session(&session("alpha", root_str, "sh"))
            .unwrap();
        db.insert_session(&session("beta", root_str, "sh")).unwrap();
        db.insert_session(&session("agent", root_str, fake.to_str().unwrap()))
            .unwrap();
        db.set_cli_session_uuid("agent", Some("native-uuid"))
            .unwrap();
    }
    let daemon = Daemon::start(root.clone());
    let mut app = App::default();
    app.apply_workspace(daemon.snapshot().await);
    app.selected_index = app
        .sessions
        .iter()
        .position(|s| s.session_id == "alpha")
        .unwrap();
    app.handle_key(KeyCode::Char('g'));
    // Group editor starts with the project prefix; append a nested path.
    for c in "team/api".chars() {
        app.handle_key(KeyCode::Char(c));
    }
    app.handle_key(KeyCode::Enter);
    let IpcMessage::WorkspaceRequest(req) = app.pending_message.take().unwrap() else {
        panic!()
    };
    let IpcMessage::WorkspaceResponse(resp) = daemon.request(req).await else {
        panic!()
    };
    app.apply_workspace(resp);
    assert_eq!(app.metadata_for(app.selected_index).group, "team/api");
    app.handle_key(KeyCode::Char('*'));
    let IpcMessage::WorkspaceRequest(req) = app.pending_message.take().unwrap() else {
        panic!()
    };
    let IpcMessage::WorkspaceResponse(resp) = daemon.request(req).await else {
        panic!()
    };
    app.apply_workspace(resp);
    assert!(app.metadata_for(app.selected_index).favorite);
    app.handle_key(KeyCode::Char('D'));
    assert!(app.comfortable_rows);
    app.handle_key(KeyCode::Char('a'));
    let IpcMessage::WorkspaceRequest(req) = app.pending_message.take().unwrap() else {
        panic!()
    };
    let IpcMessage::WorkspaceResponse(resp) = daemon.request(req).await else {
        panic!()
    };
    app.apply_workspace(resp);
    assert_ne!(app.sessions[app.selected_index].session_id, "alpha");
    app.handle_key(KeyCode::Char('t'));
    assert_eq!(app.sessions[app.selected_index].session_id, "alpha");
    drop(daemon);
    let daemon = Daemon::start(root.clone());
    let snapshot = daemon.snapshot().await;
    assert_eq!(
        snapshot.metadata["alpha"],
        WorkspaceMetadata {
            project: root_str.into(),
            group: "team/api".into(),
            favorite: true,
            archived: true
        }
    );
    app.apply_workspace(snapshot);
    app.handle_key(KeyCode::Char('u'));
    let IpcMessage::WorkspaceRequest(req) = app.pending_message.take().unwrap() else {
        panic!()
    };
    let IpcMessage::WorkspaceResponse(resp) = daemon.request(req).await else {
        panic!()
    };
    app.apply_workspace(resp);
    assert!(!daemon.snapshot().await.metadata["alpha"].archived);
    app.handle_key(KeyCode::Char('t'));
    app.selected_index = app
        .sessions
        .iter()
        .position(|s| s.session_id == "alpha")
        .unwrap();
    for mode in ['s', 't'] {
        app.handle_key(KeyCode::Char('f'));
        assert!(matches!(app.modal, ModalState::ForkMenu));
        app.handle_key(KeyCode::Char(mode));
        let IpcMessage::WorkspaceRequest(req) = app.pending_message.take().unwrap() else {
            panic!()
        };
        let IpcMessage::WorkspaceResponse(resp) = daemon.request(req).await else {
            panic!()
        };
        assert!(resp.error.unwrap().contains("does not support"));
    }
    for (mode, id) in [(false, "fork-standalone"), (true, "agent:fork-child")] {
        let response = daemon
            .request(WorkspaceRequest::Fork {
                session_id: "agent".into(),
                new_session_id: id.into(),
                subtab: mode,
            })
            .await;
        assert!(
            matches!(&response, IpcMessage::SpawnSessionResponse(r) if r.success),
            "fork spawn failed: {response:?}"
        );
    }
    for _ in 0..100 {
        if std::fs::read_to_string(&output)
            .unwrap_or_default()
            .matches("--fork-session")
            .count()
            == 2
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let args = std::fs::read_to_string(&output).unwrap();
    assert_eq!(args.matches("--resume").count(), 2);
    assert_eq!(args.matches("native-uuid").count(), 2);
    assert_eq!(args.matches("--fork-session").count(), 2);
    app.handle_key(KeyCode::Char(' '));
    app.handle_key(KeyCode::Tab);
    app.handle_key_with_modifiers(KeyCode::Char('l'), KeyModifiers::CONTROL);
    app.handle_key_with_modifiers(KeyCode::Char('x'), KeyModifiers::CONTROL);
    app.handle_key_with_modifiers(KeyCode::Char('t'), KeyModifiers::CONTROL);
    app.handle_key(KeyCode::Char('h'));
    app.handle_key(KeyCode::Enter);
    let Some(IpcMessage::SpawnSessionRequest(req)) = app.pending_message.take() else {
        panic!()
    };
    assert_eq!(req.command, "codex");
    assert!(req.args.contains(&"--model".into()));
    assert!(
        req.args
            .iter()
            .any(|a| a.contains("model_reasoning_effort"))
    );
    assert_eq!(req.args.last().unwrap(), "h");
    assert!(req.worktree_branch.is_some());
    let spawn_id = req.session_id.clone();
    let branch = req.worktree_branch.clone().unwrap();
    let token = agmux_core::get_or_create_token(&root.join("token")).unwrap();
    let response = agmux_cli::ipc_client::review_request(
        &root.join("socket"),
        &token,
        IpcMessage::SpawnSessionRequest(req),
    )
    .await
    .unwrap();
    assert!(matches!(response, IpcMessage::SpawnSessionResponse(r) if r.success));
    let snapshot = daemon.snapshot().await;
    let spawned = snapshot
        .sessions
        .iter()
        .find(|s| s.session_id == spawn_id)
        .unwrap();
    assert!(spawned.cwd.ends_with(&format!(".agmux-worktrees/{branch}")));
    for _ in 0..100 {
        if std::fs::read_to_string(&output)
            .unwrap_or_default()
            .contains("model_reasoning_effort")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let args = std::fs::read_to_string(&output).unwrap();
    assert!(args.contains("gpt-5.4"));
    assert!(args.contains("model_reasoning_effort=\"low\""));

    drop(daemon);
    std::fs::remove_dir_all(root).unwrap();
}

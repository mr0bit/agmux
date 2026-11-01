#![cfg(unix)]
use agmux_proto::{
    HandshakeRequest, IpcMessage, ListSessionsRequest, SpawnSessionRequest, new_framed,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixStream;

#[tokio::test]
async fn broadcasts_and_persists_inactivity_and_exit() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-classifier-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let socket = root.join("socket");
    let token = root.join("token");
    let mut daemon = Command::new("target/debug/agmuxd")
        .args([
            "--socket",
            socket.to_str().unwrap(),
            "--token-path",
            token.to_str().unwrap(),
        ])
        .env("AGMUX_DB_PATH", root.join("state.db"))
        .env("AGMUX_STUCK_TIMEOUT_MS", "150")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let secret = std::fs::read_to_string(&token).unwrap();
    let mut client = new_framed(UnixStream::connect(&socket).await.unwrap());
    client
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "test".into(),
            token: secret.trim().into(),
        }))
        .await
        .unwrap();
    assert!(
        matches!(client.next().await.unwrap().unwrap(), IpcMessage::HandshakeResponse(r) if r.success)
    );
    client
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "classifier".into(),
            command: "sleep".into(),
            args: vec!["1".into()],
            cwd: None,
            worktree_branch: None,
            ..Default::default()
        }))
        .await
        .unwrap();
    assert!(
        matches!(client.next().await.unwrap().unwrap(), IpcMessage::SpawnSessionResponse(r) if r.success)
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut stuck = false;
        let mut completed = false;
        let mut persisted = false;
        while !completed {
            let message = client.next().await.unwrap().unwrap();
            if let IpcMessage::SessionStateChangedEvent(e) = message {
                if e.new_state == "STUCK" {
                    assert_eq!(e.old_state, "RUNNING");
                    stuck = true;
                    client
                        .send(IpcMessage::ListSessionsRequest(ListSessionsRequest {}))
                        .await
                        .unwrap();
                }
                if e.new_state == "COMPLETED" {
                    assert!(stuck);
                    completed = true;
                }
            } else if let IpcMessage::ListSessionsResponse(r) = message {
                assert!(
                    r.sessions
                        .iter()
                        .any(|s| s.session_id == "classifier" && s.state == "STUCK")
                );
                persisted = true;
            }
        }
        assert!(persisted);
    })
    .await
    .unwrap();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

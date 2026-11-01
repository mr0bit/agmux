use std::path::PathBuf;
use std::time::Duration;

use agmux_daemon::DaemonServer;
use agmux_proto::{
    HandshakeRequest, IpcMessage, ResizeSessionRequest, SpawnSessionRequest, TerminalInputFrame,
    connect_ipc, new_framed,
};
use futures_util::{SinkExt, StreamExt};

struct TempDir(PathBuf);

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn start_test_daemon(dir: &TempDir) -> (PathBuf, String) {
    let sock_path = dir.path().join("agmux-pty-color-test.sock");
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

    let server = DaemonServer::new(sock_path.clone(), token.clone());
    tokio::spawn(async move {
        let _ = server.run().await;
    });

    for _ in 0..50 {
        if connect_ipc(&sock_path).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    (sock_path, token)
}

#[tokio::test]
async fn test_pty_environment_colorterm_and_xterm_256color() {
    let dir = TempDir::new("agmux-colorterm-env");
    let (sock_path, token) = start_test_daemon(&dir).await;

    let stream = connect_ipc(&sock_path).await.expect("connect");
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.clone(),
        }))
        .await
        .expect("handshake");
    let _ = framed.next().await;

    // Spawn session running shell command printing TERM and COLORTERM
    framed
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "env-test-sess".to_string(),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo CHECK_TERM=$TERM; echo CHECK_COLORTERM=$COLORTERM; sleep 1".to_string(),
            ],
            cwd: Some(dir.path().display().to_string()),
            worktree_branch: None,
            ..Default::default()
        }))
        .await
        .expect("spawn request");

    let spawn_resp = framed.next().await.expect("response").expect("ipc message");
    assert!(matches!(spawn_resp, IpcMessage::SpawnSessionResponse(ref r) if r.success));

    // Subscribe to viewport
    framed
        .send(IpcMessage::SubscribeViewport {
            session_id: "env-test-sess".to_string(),
        })
        .await
        .expect("subscribe viewport");

    let mut combined_output = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);

    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(300), framed.next()).await
        {
            match msg {
                IpcMessage::ScreenSyncFrame(sync) if sync.session_id == "env-test-sess" => {
                    combined_output.push_str(&String::from_utf8_lossy(&sync.formatted_screen));
                }
                IpcMessage::TerminalOutputFrame(frame) if frame.session_id == "env-test-sess" => {
                    combined_output.push_str(&String::from_utf8_lossy(&frame.data));
                }
                _ => {}
            }
            if combined_output.contains("CHECK_TERM=xterm-256color")
                && combined_output.contains("CHECK_COLORTERM=truecolor")
            {
                break;
            }
        }
    }

    assert!(
        combined_output.contains("CHECK_TERM=xterm-256color"),
        "PTY must export TERM=xterm-256color, got: {combined_output}"
    );
    assert!(
        combined_output.contains("CHECK_COLORTERM=truecolor"),
        "PTY must export COLORTERM=truecolor, got: {combined_output}"
    );
}

#[tokio::test]
async fn test_subscribe_viewport_emits_single_sync_frame_without_immediate_duplicate() {
    let dir = TempDir::new("agmux-no-duplicate-sync");
    let (sock_path, token) = start_test_daemon(&dir).await;

    let stream = connect_ipc(&sock_path).await.expect("connect");
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.clone(),
        }))
        .await
        .expect("handshake");
    let _ = framed.next().await;

    // Spawn a quiet session
    framed
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "quiet-sess".to_string(),
            command: "sleep".to_string(),
            args: vec!["5".to_string()],
            cwd: Some(dir.path().display().to_string()),
            worktree_branch: None,
            ..Default::default()
        }))
        .await
        .expect("spawn request");

    let _ = framed.next().await;

    // Subscribe to viewport
    framed
        .send(IpcMessage::SubscribeViewport {
            session_id: "quiet-sess".to_string(),
        })
        .await
        .expect("subscribe viewport");

    // First frame MUST be ScreenSyncFrame
    let first_msg = tokio::time::timeout(Duration::from_secs(1), framed.next())
        .await
        .expect("timeout waiting for sync frame")
        .expect("stream ended")
        .expect("ipc error");

    assert!(
        matches!(first_msg, IpcMessage::ScreenSyncFrame(_)),
        "Expected first message to be ScreenSyncFrame, got: {first_msg:?}"
    );

    // Second message should NOT immediately arrive with duplicate data (since session is sleeping)
    let second_msg = tokio::time::timeout(Duration::from_millis(200), framed.next()).await;
    assert!(
        second_msg.is_err(),
        "Did not expect immediate duplicate TerminalOutputFrame, but got: {:?}",
        second_msg
    );
}

#[tokio::test]
async fn test_resize_session_request_updates_dimensions() {
    let dir = TempDir::new("agmux-resize-sync");
    let (sock_path, token) = start_test_daemon(&dir).await;

    let stream = connect_ipc(&sock_path).await.expect("connect");
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.clone(),
        }))
        .await
        .expect("handshake");
    let _ = framed.next().await;

    framed
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "dyn-resize-sess".to_string(),
            command: "sleep".to_string(),
            args: vec!["5".to_string()],
            cwd: Some(dir.path().display().to_string()),
            worktree_branch: None,
            ..Default::default()
        }))
        .await
        .expect("spawn");

    let _ = framed.next().await;

    // Send ResizeSessionRequest with custom cols and rows (120x45)
    framed
        .send(IpcMessage::ResizeSessionRequest(ResizeSessionRequest {
            session_id: "dyn-resize-sess".to_string(),
            cols: 120,
            rows: 45,
        }))
        .await
        .expect("send resize");

    let resize_resp = tokio::time::timeout(Duration::from_secs(1), framed.next())
        .await
        .expect("timeout")
        .expect("stream")
        .expect("msg");

    match resize_resp {
        IpcMessage::ResizeSessionResponse(resp) => {
            assert!(resp.success);
            assert_eq!(resp.cols, 120);
            assert_eq!(resp.rows, 45);
        }
        other => panic!("Expected ResizeSessionResponse, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_interactive_input_does_not_duplicate_output_frames() {
    let dir = TempDir::new("pty-dedup-test");
    let (sock_path, token) = start_test_daemon(&dir).await;

    let stream = connect_ipc(&sock_path).await.expect("connect");
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.clone(),
        }))
        .await
        .expect("handshake");
    let _ = framed.next().await;

    let session_id = "test-dedup-sess".to_string();
    framed
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: session_id.clone(),
            command: "sh".to_string(),
            args: vec![],
            cwd: None,
            worktree_branch: None,
            detached: true,
            cols: Some(100),
            rows: Some(30),
        }))
        .await
        .expect("spawn");

    let spawn_resp = framed.next().await.unwrap().unwrap();
    assert!(matches!(spawn_resp, IpcMessage::SpawnSessionResponse(r) if r.success));

    // Now send SubscribeViewport on the same connection (like TUI does)
    framed
        .send(IpcMessage::SubscribeViewport {
            session_id: session_id.clone(),
        })
        .await
        .expect("subscribe");

    // Read until ScreenSyncFrame
    let mut saw_sync = false;
    for _ in 0..5 {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(500), framed.next()).await
        {
            if matches!(msg, IpcMessage::ScreenSyncFrame(_)) {
                saw_sync = true;
                break;
            }
        }
    }
    assert!(saw_sync, "Must see ScreenSyncFrame");

    // Send input: echo UNIQUE_DEDUP_MARKER
    framed
        .send(IpcMessage::TerminalInputFrame(TerminalInputFrame {
            session_id: session_id.clone(),
            data: b"echo UNIQUE_DEDUP_MARKER\n".to_vec(),
        }))
        .await
        .expect("send input");

    let mut marker_count = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(500), framed.next()).await
        {
            if let IpcMessage::TerminalOutputFrame(frame) = msg {
                let text = String::from_utf8_lossy(&frame.data);
                if text.contains("UNIQUE_DEDUP_MARKER") {
                    marker_count += 1;
                }
            }
        } else {
            break;
        }
    }

    assert_eq!(
        marker_count, 2,
        "Expected exactly 2 marker instances: echo from PTY + stdout from shell"
    );
}

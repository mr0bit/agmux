//! Verification Tests:
//! - `TEST-VER-IPC-003` (`REQ-IPC-SINGLE-WRITER-003`, `LLD-IPC-SINGLE-WRITER-003`)
//! - `TEST-VER-IPC-004` (`REQ-IPC-SYNC-FRAME-004`, `LLD-IPC-SYNC-FRAME-004`)
//! - `TEST-VER-IPC-005` (`REQ-IPC-EVENT-DISPATCH-005`, `LLD-IPC-EVENT-DISPATCH-005`)

#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use agmux_proto::{HandshakeRequest, IpcMessage, SpawnSessionRequest, new_framed};
use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;

fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agmux-{prefix}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `TEST-VER-IPC-003`: 50 concurrent IPC requests execute while `agmuxd` flushes FTS log batches
/// with zero `SQLITE_BUSY` errors.
#[tokio::test]
async fn test_ver_ipc_003_single_writer_50_concurrent_requests() {
    let root = unique_test_dir("ipc-single-writer");
    let socket = root.join("daemon.sock");
    let token_path = root.join("token");
    let db_path = root.join("state.db");

    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let mut daemon = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token_path)
        .env("AGMUX_DB_PATH", &db_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    let token = std::fs::read_to_string(&token_path)
        .unwrap()
        .trim()
        .to_string();

    // Spawn a session producing FTS-indexed terminal output while 50 concurrent IPC requests run
    let mut spawner = new_framed(UnixStream::connect(&socket).await.unwrap());
    spawner
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "test".into(),
            token: token.clone(),
        }))
        .await
        .unwrap();
    let _ = spawner.next().await.unwrap().unwrap();
    spawner
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "fts-producer".into(),
            command: "sh".into(),
            args: vec![
                "-c".into(),
                "for i in $(seq 1 30); do echo \"FTS_BATCH_LINE_$i\"; done".into(),
            ],
            cwd: None,
            worktree_branch: None,
        }))
        .await
        .unwrap();
    let _ = spawner.next().await.unwrap().unwrap();

    // Launch 50 concurrent IPC tasks performing state writes & reads (create_task, list_sessions, search_logs)
    let mut handles = Vec::with_capacity(50);
    for i in 0..50 {
        let sock = socket.clone();
        let tok = token.clone();
        handles.push(tokio::spawn(async move {
            let create = agmux_cli::ipc_client::create_task(
                &sock,
                &tok,
                format!("T-{i:03}"),
                format!("Concurrent Task {i}"),
                "Single-writer stress test".into(),
            )
            .await?;
            if !create.success {
                anyhow::bail!("create_task failed: {:?}", create.error);
            }
            let sessions = agmux_cli::ipc_client::list_sessions(&sock, &tok).await?;
            let _ = agmux_cli::ipc_client::search_logs(&sock, &tok, "FTS_BATCH".into(), None, 10)
                .await?;
            Ok::<usize, anyhow::Error>(sessions.len())
        }));
    }

    for h in handles {
        let res = h.await.expect("task join");
        assert!(res.is_ok(), "Concurrent IPC request failed: {res:?}");
    }

    let tasks = agmux_cli::ipc_client::list_tasks(&socket, &token, None)
        .await
        .unwrap();
    assert_eq!(tasks.tasks.len(), 50);

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = std::fs::remove_dir_all(&root);
}

/// `TEST-VER-IPC-004` & `TEST-VER-IPC-005`:
/// - `SubscribeViewport` immediately returns an atomic `ScreenSyncFrame` with dimensions, cursor, and lines.
/// - State transitions push `NotificationEvent` and `SessionListUpdate` to connected clients without polling.
#[tokio::test]
async fn test_ver_ipc_004_and_005_screen_sync_frame_and_event_broadcast() {
    let root = unique_test_dir("ipc-sync-bcast");
    let socket = root.join("daemon.sock");
    let token_path = root.join("token");
    let db_path = root.join("state.db");

    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let mut daemon = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token_path)
        .env("AGMUX_DB_PATH", &db_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    let token = std::fs::read_to_string(&token_path)
        .unwrap()
        .trim()
        .to_string();

    let mut client = new_framed(UnixStream::connect(&socket).await.unwrap());
    client
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "test".into(),
            token: token.clone(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        client.next().await.unwrap().unwrap(),
        IpcMessage::HandshakeResponse(r) if r.success
    ));

    // Spawn a session that prints a known banner and then prompts (y/n) to trigger a state transition
    client
        .send(IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "sync-session".into(),
            command: "sh".into(),
            args: vec![
                "-c".into(),
                "printf 'SYNC_BANNER_READY\\r\\nProceed? (y/n) '; sleep 1".into(),
            ],
            cwd: None,
            worktree_branch: None,
        }))
        .await
        .unwrap();
    assert!(matches!(
        client.next().await.unwrap().unwrap(),
        IpcMessage::SpawnSessionResponse(r) if r.success
    ));

    // Wait briefly for initial output and state transition
    let mut saw_notification = false;
    let mut saw_list_update = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline && (!saw_notification || !saw_list_update) {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(200), client.next()).await
        {
            match msg {
                IpcMessage::NotificationEvent(ev) if ev.session_id == "sync-session" => {
                    saw_notification = true;
                }
                IpcMessage::SessionListUpdate(upd)
                    if upd.sessions.iter().any(|s| s.session_id == "sync-session") =>
                {
                    saw_list_update = true;
                }
                _ => {}
            }
        }
    }
    assert!(
        saw_notification,
        "Client must receive asynchronous NotificationEvent on state transition"
    );
    assert!(
        saw_list_update,
        "Client must receive asynchronous SessionListUpdate on state transition"
    );

    // Connect a second client and send SubscribeViewport to verify immediate ScreenSyncFrame
    let mut sub_client = new_framed(UnixStream::connect(&socket).await.unwrap());
    sub_client
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "test".into(),
            token: token.clone(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        sub_client.next().await.unwrap().unwrap(),
        IpcMessage::HandshakeResponse(r) if r.success
    ));

    sub_client
        .send(IpcMessage::SubscribeViewport {
            session_id: "sync-session".into(),
        })
        .await
        .unwrap();

    let mut sync_frame = None;
    for _ in 0..10 {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(500), sub_client.next()).await
            && let IpcMessage::ScreenSyncFrame(frame) = msg
        {
            sync_frame = Some(frame);
            break;
        }
    }
    let frame = sync_frame.expect("SubscribeViewport must immediately emit ScreenSyncFrame");
    assert_eq!(frame.session_id, "sync-session");
    assert_eq!(frame.rows, 24);
    assert_eq!(frame.cols, 80);
    assert!(
        frame.lines.iter().any(|l| l.contains("SYNC_BANNER_READY")),
        "ScreenSyncFrame lines must contain captured VT100 screen contents, got: {:?}",
        frame.lines
    );

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = std::fs::remove_dir_all(&root);
}

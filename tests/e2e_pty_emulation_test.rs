//! End-to-end verification tests for PTY emulation, WSL2 bridge, VT100 grid, resize reflow, and shell sub-tab spawning.
//!
//! Verifies:
//! - REQ-PTY-WSL2-BRIDGE-002 (TEST-VER-PTY-002)
//! - REQ-PTY-VT100-GRID-003 (TEST-VER-PTY-003)
//! - REQ-PTY-RESIZE-REFLOW-004 (TEST-VER-PTY-004)
//! - REQ-PTY-SHELL-SUBTAB-005 (TEST-VER-PTY-005)

use std::path::PathBuf;
use std::time::Duration;

use agmux_cli::ipc_client::{resize_session, review_request, spawn_subtab};
use agmux_core::vt_parser::{DEFAULT_SCROLLBACK_LINES, VtParser};
use agmux_core::wsl_bridge::WslBridge;
use agmux_daemon::DaemonServer;
use agmux_proto::{HandshakeRequest, IpcMessage, SpawnSessionRequest, connect_ipc, new_framed};
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
    let sock_path = dir.path().join("agmux-pty-e2e.sock");
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

#[test]
fn test_ver_pty_002_wsl2_bridge_command_and_path_translation() {
    let linux_cwd = WslBridge::windows_to_wsl_path(r"C:\Users\dev\project");
    assert_eq!(linux_cwd, "/mnt/c/Users/dev/project");

    let (bin, args) = WslBridge::build_command(
        "Ubuntu-24.04",
        "claude",
        &["--resume".to_string(), "sess-42".to_string()],
        Some(r"C:\Users\dev\project"),
    );

    assert_eq!(bin, "wsl.exe");
    assert_eq!(
        args,
        vec![
            "-d",
            "Ubuntu-24.04",
            "--cd",
            "/mnt/c/Users/dev/project",
            "--",
            "claude",
            "--resume",
            "sess-42",
        ]
    );
}

#[test]
fn test_ver_pty_003_vt100_grid_and_100k_line_scrollback_cap() {
    let mut parser = VtParser::new(24, 80);

    // 1. Stream 24-bit TrueColor ANSI sequence + UTF-8 box drawing
    parser.feed_bytes(
        b"\x1b[38;2;12;34;56m\x1b[48;2;210;100;50m\xe2\x94\x8c\xe2\x94\x80TRUECOLOR_BOX\xe2\x94\x90\x1b[0m\r\n",
    );
    let visible = parser.visible_lines();
    assert!(visible.iter().any(|l| l.contains("┌─TRUECOLOR_BOX┐")));

    // 2. Stream 100,000 lines in 8 KB chunks and verify scrollback cap of 10,000 lines
    let mut chunk = Vec::with_capacity(8192);
    for i in 0..100_000 {
        chunk.extend_from_slice(format!("STREAM_LINE_{i:06}\r\n").as_bytes());
        if chunk.len() >= 8192 {
            parser.feed_bytes(&chunk);
            chunk.clear();
        }
    }
    if !chunk.is_empty() {
        parser.feed_bytes(&chunk);
    }

    assert_eq!(parser.scrollback_lines().len(), DEFAULT_SCROLLBACK_LINES);
    assert_eq!(
        parser.scrollback_lines().back().map(String::as_str),
        Some("STREAM_LINE_099999")
    );
}

#[tokio::test]
async fn test_ver_pty_004_resize_reflow_and_tiocswinsz_propagation() {
    let dir = TempDir::new("agmux-pty-004");
    let (sock_path, token) = start_test_daemon(&dir).await;

    // Spawn a shell session that prints `stty size` after resize
    let spawn_resp = review_request(
        &sock_path,
        &token,
        IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "resize-sess".to_string(),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "sleep 0.25; stty size; sleep 1".to_string(),
            ],
            cwd: Some(dir.path().display().to_string()),
            worktree_branch: None,
            ..Default::default()
        }),
    )
    .await
    .expect("spawn session");
    assert!(matches!(
        spawn_resp,
        IpcMessage::SpawnSessionResponse(ref r) if r.success
    ));

    // Resize session from 80x24 to 160x50 over IPC
    let resize_resp = resize_session(&sock_path, &token, "resize-sess".to_string(), 160, 50)
        .await
        .expect("resize session");
    assert!(resize_resp.success);
    assert_eq!(resize_resp.cols, 160);
    assert_eq!(resize_resp.rows, 50);

    // Subscribe to viewport and verify ScreenSyncFrame reflects 160x50 and `stty size` reports `50 160`
    let stream = connect_ipc(&sock_path).await.expect("connect");
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.clone(),
        }))
        .await
        .expect("send handshake");
    let _ = framed.next().await;

    framed
        .send(IpcMessage::SubscribeViewport {
            session_id: "resize-sess".to_string(),
        })
        .await
        .expect("subscribe viewport");

    let mut saw_sync_size = false;
    let mut saw_stty_output = false;
    let mut combined = String::new();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(400), framed.next()).await
        {
            match msg {
                IpcMessage::ScreenSyncFrame(sync) if sync.session_id == "resize-sess" => {
                    assert_eq!(sync.cols, 160);
                    assert_eq!(sync.rows, 50);
                    saw_sync_size = true;
                    let text = String::from_utf8_lossy(&sync.formatted_screen);
                    combined.push_str(&text);
                }
                IpcMessage::TerminalOutputFrame(out) if out.session_id == "resize-sess" => {
                    let text = String::from_utf8_lossy(&out.data);
                    combined.push_str(&text);
                }
                _ => {}
            }
            if combined.contains("50 160") {
                saw_stty_output = true;
                break;
            }
        }
    }

    assert!(saw_sync_size, "Expected ScreenSyncFrame with 160x50");
    assert!(
        saw_stty_output,
        "Expected stty size '50 160' in PTY output, got: {combined:?}"
    );
}

#[tokio::test]
async fn test_ver_pty_005_shell_subtab_inherits_cwd_and_port() {
    let dir = TempDir::new("agmux-pty-005");
    let work_subdir = dir.path().join("worktree-alpha");
    std::fs::create_dir_all(&work_subdir).expect("create subdir");
    let (sock_path, token) = start_test_daemon(&dir).await;

    // 1. Spawn parent session in `worktree-alpha`
    let spawn_resp = review_request(
        &sock_path,
        &token,
        IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "parent-agent".to_string(),
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: Some(work_subdir.display().to_string()),
            worktree_branch: None,
            ..Default::default()
        }),
    )
    .await
    .expect("spawn parent");
    assert!(matches!(
        spawn_resp,
        IpcMessage::SpawnSessionResponse(ref r) if r.success
    ));

    // 2. Spawn interactive shell sub-tab associated with `parent-agent`
    let subtab_resp = spawn_subtab(
        &sock_path,
        &token,
        "parent-agent".to_string(),
        "parent-agent:shell-1".to_string(),
        Some("/bin/sh".to_string()),
    )
    .await
    .expect("spawn subtab");

    assert!(
        subtab_resp.success,
        "Subtab spawn failed: {:?}",
        subtab_resp.error
    );
    assert_eq!(subtab_resp.subtab_id, "parent-agent:shell-1");
    assert_eq!(subtab_resp.cwd, work_subdir.display().to_string());
    assert!(
        (8000..=8999).contains(&subtab_resp.port),
        "Expected deterministic PORT in 8000..=8999, got {}",
        subtab_resp.port
    );
}

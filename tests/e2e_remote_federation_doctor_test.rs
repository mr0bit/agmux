//! End-to-end verification tests for Remote SSH/Tailscale Federation, Pinned Binary Bootstrapper,
//! Cookie Token Network Auth, and Doctor Repair (`TASK-27`).
//!
//! Verifies:
//! - REQ-REMOTE-FEDERATION-SSH-001 (TEST-VER-REMOTE-001)
//! - REQ-REMOTE-BOOTSTRAP-PINNED-002 (TEST-VER-REMOTE-002)
//! - REQ-REMOTE-AUTH-TOKEN-003 (TEST-VER-REMOTE-003)
//! - REQ-REMOTE-DOCTOR-REPAIR-004 (TEST-VER-REMOTE-004)

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use agmux_cli::doctor::{repair_daemon_env, run_doctor};
use agmux_cli::ipc_client::{connect_remote, list_remotes, list_sessions, review_request};
use agmux_cli::remote_bootstrapper::{BootstrapError, RemoteBootstrapper};
use agmux_daemon::DaemonServer;
use agmux_daemon::network_auth::NetworkAuthValidator;
use agmux_proto::{
    HandshakeRequest, IpcMessage, SpawnSessionRequest, TerminalInputFrame, connect_ipc, new_framed,
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

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn start_daemon(sock_path: PathBuf, token: String) {
    let server = DaemonServer::new(sock_path.clone(), token);
    tokio::spawn(async move {
        let _ = server.run().await;
    });

    for _ in 0..50 {
        if connect_ipc(&sock_path).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn test_ver_remote_001_ssh_federation_dual_daemon_bridge_and_subtree_mounting() {
    let dir = TempDir::new("agmux-remote-001");
    let local_sock = dir.path().join("local.sock");
    let remote_sock = dir.path().join("remote.sock");
    let local_token =
        "1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let remote_token =
        "2222222222222222222222222222222222222222222222222222222222222222".to_string();

    start_daemon(local_sock.clone(), local_token.clone()).await;
    start_daemon(remote_sock.clone(), remote_token.clone()).await;

    // 1. Spawn a session on the remote daemon (`remote_sock`) that echoes stdin
    let remote_spawn = review_request(
        &remote_sock,
        &remote_token,
        IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
            session_id: "gpu-worker-1".to_string(),
            command: "cat".to_string(),
            args: Vec::new(),
            cwd: Some(dir.path().display().to_string()),
            worktree_branch: None,
        }),
    )
    .await
    .expect("spawn remote session");
    assert!(matches!(
        remote_spawn,
        IpcMessage::SpawnSessionResponse(ref r) if r.success
    ));

    // 2. Connect local daemon to remote daemon (`ssh://dev@gpu-box`)
    let t0 = Instant::now();
    let conn_resp = connect_remote(
        &local_sock,
        &local_token,
        "ssh://dev@gpu-box".to_string(),
        Some(remote_sock.display().to_string()),
        Some(remote_token.clone()),
    )
    .await
    .expect("connect remote");
    assert!(conn_resp.success);
    assert_eq!(conn_resp.host_label, "gpu-box");
    assert_eq!(conn_resp.mounted_sessions.len(), 1);
    assert_eq!(
        conn_resp.mounted_sessions[0].session_id,
        "[gpu-box] gpu-worker-1"
    );
    assert!(
        t0.elapsed() < Duration::from_millis(250),
        "Federation took {:?}",
        t0.elapsed()
    );

    // 3. Verify `list_sessions` on local daemon includes `[gpu-box] gpu-worker-1`
    let all_sessions = list_sessions(&local_sock, &local_token)
        .await
        .expect("list local+remote sessions");
    assert!(
        all_sessions
            .iter()
            .any(|s| s.session_id == "[gpu-box] gpu-worker-1")
    );

    let remotes = list_remotes(&local_sock, &local_token)
        .await
        .expect("list remotes");
    assert_eq!(remotes.remotes, vec!["gpu-box".to_string()]);

    // 4. Forward terminal input through local daemon to `[gpu-box] gpu-worker-1` and verify on remote
    let local_stream = connect_ipc(&local_sock).await.expect("connect local");
    let mut local_framed = new_framed(local_stream);
    local_framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: local_token.clone(),
        }))
        .await
        .expect("local handshake");
    let _ = local_framed.next().await;

    local_framed
        .send(IpcMessage::TerminalInputFrame(TerminalInputFrame {
            session_id: "[gpu-box] gpu-worker-1".to_string(),
            data: b"FEDERATED_PING_OK\n".to_vec(),
        }))
        .await
        .expect("send federated input");

    // Subscribe on remote daemon viewport to verify `FEDERATED_PING_OK` arrived
    let rem_stream = connect_ipc(&remote_sock).await.expect("connect remote");
    let mut rem_framed = new_framed(rem_stream);
    rem_framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: remote_token.clone(),
        }))
        .await
        .expect("remote handshake");
    let _ = rem_framed.next().await;
    rem_framed
        .send(IpcMessage::SubscribeViewport {
            session_id: "gpu-worker-1".to_string(),
        })
        .await
        .expect("subscribe remote");

    let mut received = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_millis(200), rem_framed.next()).await
        {
            match msg {
                IpcMessage::ScreenSyncFrame(sync) => {
                    received.push_str(&String::from_utf8_lossy(&sync.formatted_screen));
                }
                IpcMessage::TerminalOutputFrame(out) => {
                    received.push_str(&String::from_utf8_lossy(&out.data));
                }
                _ => {}
            }
            if received.contains("FEDERATED_PING_OK") {
                break;
            }
        }
    }
    assert!(
        received.contains("FEDERATED_PING_OK"),
        "Expected federated input on remote PTY, got: {received:?}"
    );
}

#[test]
fn test_ver_remote_002_pinned_binary_bootstrapper_and_sha256_verification() {
    assert_eq!(
        RemoteBootstrapper::resolve_target_triple("Linux aarch64").unwrap(),
        "aarch64-unknown-linux-musl"
    );
    assert_eq!(
        RemoteBootstrapper::resolve_target_triple("Linux x86_64").unwrap(),
        "x86_64-unknown-linux-musl"
    );
    assert_eq!(
        RemoteBootstrapper::resolve_target_triple("Darwin arm64").unwrap(),
        "aarch64-apple-darwin"
    );

    let url = RemoteBootstrapper::release_asset_url("0.1.0", "aarch64-unknown-linux-musl");
    assert_eq!(
        url,
        "https://github.com/agmux/agmux/releases/download/v0.1.0/agmuxd-v0.1.0-aarch64-unknown-linux-musl"
    );

    // Verify NIST SHA-256 test vector ("abc")
    assert_eq!(
        RemoteBootstrapper::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let dir = TempDir::new("agmux-bootstrap-002");
    let fake_binary = b"#!/bin/sh\necho agmuxd-aarch64\n";
    let expected_sha = RemoteBootstrapper::sha256_hex(fake_binary);

    // Tampered checksum must fail before writing binary
    let bad_res = RemoteBootstrapper::verify_and_install(
        fake_binary,
        "0000000000000000000000000000000000000000000000000000000000000000",
        dir.path(),
    );
    assert!(matches!(
        bad_res,
        Err(BootstrapError::ChecksumMismatch { .. })
    ));

    // Matching checksum installs to `~/.local/bin/agmuxd` with executable permissions
    let installed_path =
        RemoteBootstrapper::verify_and_install(fake_binary, &expected_sha, dir.path())
            .expect("verify and install");
    assert_eq!(installed_path, dir.path().join(".local/bin/agmuxd"));
    assert_eq!(std::fs::read(&installed_path).unwrap(), fake_binary);
}

#[tokio::test]
async fn test_ver_remote_003_network_cookie_token_authentication_rejects_unauthorized() {
    let dir = TempDir::new("agmux-remote-003");
    let sock_path = dir.path().join("auth-guard.sock");
    let valid_token =
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string();

    start_daemon(sock_path.clone(), valid_token.clone()).await;

    // 1. Direct unit check on NetworkAuthValidator
    let validator = NetworkAuthValidator::new(valid_token.clone());
    let bad_resp = validator.verify_handshake(&HandshakeRequest {
        client_version: "0.1.0".to_string(),
        token: "wrong-token".to_string(),
    });
    assert!(!bad_resp.success);
    assert_eq!(validator.rejected_count(), 1);

    // 2. Live IPC connection with invalid token is immediately rejected and disconnected
    let stream = connect_ipc(&sock_path)
        .await
        .expect("connect unauthenticated");
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "0.1.0".to_string(),
            token: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        }))
        .await
        .expect("send bad handshake");

    let first = framed
        .next()
        .await
        .expect("response frame")
        .expect("decode");
    match first {
        IpcMessage::HandshakeResponse(resp) => {
            assert!(!resp.success);
            assert!(resp.error.unwrap_or_default().contains("Unauthorized"));
        }
        other => panic!("Expected HandshakeResponse, got {other:?}"),
    }

    // Subsequent frame must see closed stream (`None`)
    assert!(framed.next().await.is_none());
}

#[test]
fn test_ver_remote_004_doctor_diagnostics_and_daemon_repair_restores_0600() {
    let dir = TempDir::new("agmux-doctor-004");
    let token_path = dir.path().join("daemon.token");
    let db_path = dir.path().join("state.db");
    let sock_path = dir.path().join("daemon.sock");

    let _ = agmux_core::get_or_create_token(&token_path).expect("create token");

    // 1. Healthy environment -> `report.healthy == true`
    let healthy_report = run_doctor(&token_path, &db_path, vec!["gpu-box".to_string()]);
    assert!(
        healthy_report.healthy,
        "Expected healthy report, got issues: {:?}",
        healthy_report.issues
    );
    assert!(healthy_report.pty_available);
    assert!(healthy_report.token_permissions_ok);
    assert!(healthy_report.db_accessible);

    // 2. Corrupt token permissions to `0644` and plant a stale lockfile -> `doctor` flags failure
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token_path, std::fs::Permissions::from_mode(0o644))
            .expect("corrupt permissions to 0644");
    }
    let stale_lock = sock_path.with_extension("lock");
    std::fs::write(&stale_lock, "99999999\n").expect("write stale lock");

    let broken_report = run_doctor(&token_path, &db_path, Vec::new());
    #[cfg(unix)]
    {
        assert!(!broken_report.healthy);
        assert!(!broken_report.token_permissions_ok);
        assert_eq!(broken_report.token_mode, 0o644);
    }

    // 3. Run `repair_daemon_env` (`agmux daemon repair`) -> restores `0600` and removes stale lockfile
    let actions = repair_daemon_env(&token_path, &sock_path).expect("repair daemon env");
    assert!(!actions.is_empty());
    assert!(!stale_lock.exists(), "Expected stale lockfile removed");

    let repaired_report = run_doctor(&token_path, &db_path, Vec::new());
    assert!(repaired_report.healthy);
    assert!(repaired_report.token_permissions_ok);
    assert_eq!(repaired_report.token_mode, 0o600);
}
